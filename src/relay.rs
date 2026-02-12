use crate::auth::AuthManager;
use crate::cache::ArtifactCache;
use crate::config::Config;
use crate::crypto;
use crate::error::{RelayError, Result};
use crate::rss_client::RssClient;
use crate::types::*;
use crate::variables::VariableResolver;
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct RelayState {
    pub config: Config,
    auth_manager: Arc<AuthManager>,
    rss_client: Arc<RssClient>,
    cache: Arc<ArtifactCache>,
    variable_resolver: Arc<RwLock<VariableResolver>>,
    signing_key: Arc<RwLock<Option<String>>>,
    last_sync: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    sync_cursor: Arc<RwLock<Option<String>>>,
}

impl RelayState {
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize auth manager
        let auth_manager = Arc::new(AuthManager::new(config.clone())?);
        auth_manager.initialize().await?;

        // Initialize RSS client
        let rss_client = Arc::new(RssClient::new(
            config.rss_api_url.clone(),
            auth_manager.clone(),
        ));

        // Initialize cache
        let cache = Arc::new(ArtifactCache::new(
            config.cache_dir.clone(),
            config.cache_ttl_hours,
            config.cache_max_size_mb,
        )?);

        // Initialize variable resolver
        let variable_resolver = Arc::new(RwLock::new(VariableResolver::new(None, None)));

        let state = Self {
            config,
            auth_manager,
            rss_client,
            cache,
            variable_resolver,
            signing_key: Arc::new(RwLock::new(None)),
            last_sync: Arc::new(RwLock::new(None)),
            sync_cursor: Arc::new(RwLock::new(None)),
        };

        // Start background sync task
        state.spawn_sync_task();

        Ok(state)
    }

    /// Get current relay status
    pub async fn get_status(&self) -> Result<RelayStatus> {
        let cache_stats = self.cache.stats();
        let authenticated = self.auth_manager.is_authenticated();

        let library_count = if authenticated {
            self.rss_client
                .list_libraries()
                .await
                .ok()
                .map(|l| l.len())
                .unwrap_or(0)
        } else {
            0
        };

        Ok(RelayStatus {
            connected: authenticated,
            authenticated,
            user_email: None,   // TODO: Get from token claims
            workspace_id: None, // TODO: Get from token claims
            last_sync: *self.last_sync.read(),
            cache_size_bytes: cache_stats.size_bytes,
            artifact_count: cache_stats.artifact_count,
            library_count,
            mcp_server_running: true, // Assuming it's running
            mcp_server_port: Some(self.config.mcp_server_port),
        })
    }

    /// Login to RSS using device flow
    pub async fn login(&self) -> Result<AuthStatus> {
        // Start device flow
        let device_response = self.auth_manager.start_device_flow().await?;

        // Display user code and URL
        tracing::info!(
            "Please visit {} and enter code: {}",
            device_response.verification_uri,
            device_response.user_code
        );

        // Poll for completion
        let token_pair = self
            .auth_manager
            .poll_device_flow(device_response.device_code, device_response.interval)
            .await?;

        // Fetch signing key after authentication
        self.fetch_signing_key().await?;

        Ok(AuthStatus {
            authenticated: true,
            user_email: None,   // TODO: Extract from token
            workspace_id: None, // TODO: Extract from token
            expires_at: Some(token_pair.expires_at),
        })
    }

    /// Logout from RSS
    pub async fn logout(&self) -> Result<()> {
        self.auth_manager.logout()?;
        Ok(())
    }

    /// List all accessible libraries
    pub async fn list_libraries(&self) -> Result<Vec<Library>> {
        self.rss_client.list_libraries().await
    }

    /// Get a specific library
    pub async fn get_library(&self, library_id: &str, _release: &str) -> Result<Library> {
        self.rss_client.get_library(library_id).await
    }

    /// List skills, optionally filtered by library
    pub async fn list_skills(&self, library_id: Option<&str>) -> Result<Vec<SkillSummary>> {
        if let Some(lib_id) = library_id {
            self.rss_client.list_skills(lib_id).await
        } else {
            // List all skills from all libraries
            let libraries = self.rss_client.list_libraries().await?;
            let mut all_skills = Vec::new();

            for library in libraries {
                if let Ok(skills) = self.rss_client.list_skills(&library.library_id).await {
                    all_skills.extend(skills);
                }
            }

            Ok(all_skills)
        }
    }

    /// Get a specific skill with verification and variable resolution
    pub async fn get_skill(&self, skill_id: &str, version: &str) -> Result<Artifact> {
        // Get latest approved artifact ID
        let latest = if version == "latest_approved" {
            self.rss_client.get_latest_approved_skill(skill_id).await?
        } else {
            return Err(RelayError::Internal(format!(
                "Version '{}' not supported yet",
                version
            )));
        };

        // Check cache first
        if let Some(cached_artifact) = self.cache.get(&latest.artifact_id)? {
            // Verify artifact
            if self.config.verification_required {
                self.verify_artifact(&cached_artifact).await?;
            }

            // Check if revoked
            if cached_artifact.metadata.revoked {
                return Err(RelayError::ArtifactRevoked(format!(
                    "Artifact {} has been revoked: {}",
                    latest.artifact_id,
                    cached_artifact
                        .metadata
                        .revoked_reason
                        .as_deref()
                        .unwrap_or("No reason provided")
                )));
            }

            tracing::debug!("Serving artifact {} from cache", latest.artifact_id);
            return Ok(cached_artifact);
        }

        // Fetch from RSS
        tracing::info!("Fetching artifact {} from RSS", latest.artifact_id);
        let artifact = self.rss_client.get_artifact(&latest.artifact_id).await?;

        // Verify artifact
        if self.config.verification_required {
            self.verify_artifact(&artifact).await?;
        }

        // Check if revoked
        if artifact.metadata.revoked {
            return Err(RelayError::ArtifactRevoked(format!(
                "Artifact {} has been revoked: {}",
                latest.artifact_id,
                artifact
                    .metadata
                    .revoked_reason
                    .as_deref()
                    .unwrap_or("No reason provided")
            )));
        }

        // Check approval state
        match artifact.metadata.approval_state {
            ApprovalState::Approved | ApprovalState::Published => {
                // Cache the artifact
                self.cache.put(artifact.clone())?;
                Ok(artifact)
            }
            _ => Err(RelayError::Verification(format!(
                "Artifact {} is not in approved state (current: {:?})",
                latest.artifact_id, artifact.metadata.approval_state
            ))),
        }
    }

    /// Refresh skills from RSS
    pub async fn refresh_skills(&self) -> Result<RefreshResult> {
        tracing::info!("Starting skill refresh...");

        let cursor = self.sync_cursor.read().clone();
        let updates = self.rss_client.get_updates_since(cursor.as_deref()).await?;

        let mut updated_artifacts = 0;
        let mut revoked_artifacts = 0;

        for update in &updates.updates {
            match update.update_type {
                UpdateType::NewVersion => {
                    // Fetch and cache new version
                    if let Ok(artifact) = self.rss_client.get_artifact(&update.artifact_id).await {
                        if self.verify_artifact(&artifact).await.is_ok() {
                            self.cache.put(artifact)?;
                            updated_artifacts += 1;
                        }
                    }
                }
                UpdateType::Revoked => {
                    // Remove from cache
                    self.cache.remove(&update.artifact_id)?;
                    revoked_artifacts += 1;
                }
                UpdateType::MetadataChanged => {
                    // Re-fetch artifact
                    if let Ok(artifact) = self.rss_client.get_artifact(&update.artifact_id).await {
                        if self.verify_artifact(&artifact).await.is_ok() {
                            self.cache.put(artifact)?;
                            updated_artifacts += 1;
                        }
                    }
                }
            }
        }

        // Update cursor
        *self.sync_cursor.write() = Some(updates.cursor);
        *self.last_sync.write() = Some(Utc::now());

        tracing::info!(
            "Refresh complete: {} updated, {} revoked",
            updated_artifacts,
            revoked_artifacts
        );

        Ok(RefreshResult {
            updated_artifacts,
            revoked_artifacts,
            new_libraries: 0, // TODO: Track library changes
            sync_timestamp: Utc::now(),
        })
    }

    /// Clear cache
    pub async fn clear_cache(&self) -> Result<()> {
        self.cache.clear()?;
        tracing::info!("Cache cleared");
        Ok(())
    }

    /// Wipe all data (cache + tokens)
    pub async fn wipe_all_data(&self) -> Result<()> {
        self.cache.clear()?;
        self.auth_manager.logout()?;
        *self.signing_key.write() = None;
        *self.last_sync.write() = None;
        *self.sync_cursor.write() = None;
        tracing::warn!("All data wiped");
        Ok(())
    }

    /// Verify artifact signature and integrity
    async fn verify_artifact(&self, artifact: &Artifact) -> Result<()> {
        // Get signing key
        let signing_key = {
            let key_opt = self.signing_key.read().clone();
            if key_opt.is_none() {
                self.fetch_signing_key().await?;
                self.signing_key.read().clone()
            } else {
                key_opt
            }
        };

        let key = signing_key
            .ok_or_else(|| RelayError::Verification("Signing key not available".to_string()))?;

        crypto::verify_artifact(artifact, &key)?;

        // Update verification timestamp in cache
        self.cache.update_verification_time(&artifact.artifact_id)?;

        Ok(())
    }

    /// Fetch and cache the signing public key from RSS
    async fn fetch_signing_key(&self) -> Result<()> {
        let key = self.rss_client.get_signing_key().await?;
        *self.signing_key.write() = Some(key);
        tracing::info!("Signing key fetched and cached");
        Ok(())
    }

    /// Spawn background sync task
    fn spawn_sync_task(&self) {
        let interval_minutes = self.config.sync_interval_minutes;
        let state = Arc::new(self.clone_for_sync());

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(interval_minutes * 60));

            loop {
                interval.tick().await;

                if state.auth_manager.is_authenticated() {
                    tracing::debug!("Running background sync...");
                    if let Err(e) = state.refresh_skills().await {
                        tracing::error!("Background sync failed: {}", e);
                    }
                }
            }
        });
    }

    /// Create a lightweight clone for background tasks
    fn clone_for_sync(&self) -> Self {
        Self {
            config: self.config.clone(),
            auth_manager: self.auth_manager.clone(),
            rss_client: self.rss_client.clone(),
            cache: self.cache.clone(),
            variable_resolver: self.variable_resolver.clone(),
            signing_key: self.signing_key.clone(),
            last_sync: self.last_sync.clone(),
            sync_cursor: self.sync_cursor.clone(),
        }
    }
}
