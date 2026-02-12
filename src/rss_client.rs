use crate::auth::AuthManager;
use crate::error::{RelayError, Result};
use crate::types::*;
use reqwest::Client;
use std::sync::Arc;

pub struct RssClient {
    base_url: String,
    client: Client,
    auth_manager: Arc<AuthManager>,
}

impl RssClient {
    pub fn new(base_url: String, auth_manager: Arc<AuthManager>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        Self {
            base_url,
            client,
            auth_manager,
        }
    }

    /// Get list of libraries accessible to the user
    pub async fn list_libraries(&self) -> Result<Vec<Library>> {
        let url = format!("{}/libraries", self.base_url);
        self.get(&url).await
    }

    /// Get a specific library by ID
    pub async fn get_library(&self, library_id: &str) -> Result<Library> {
        let url = format!("{}/libraries/{}", self.base_url, library_id);
        self.get(&url).await
    }

    /// Get latest approved release for a library
    #[allow(dead_code)]
    pub async fn get_latest_approved_library_release(
        &self,
        library_id: &str,
    ) -> Result<LatestApprovedResponse> {
        let url = format!(
            "{}/libraries/{}/releases/latest-approved",
            self.base_url, library_id
        );
        self.get(&url).await
    }

    /// Get skills for a library
    pub async fn list_skills(&self, library_id: &str) -> Result<Vec<SkillSummary>> {
        let url = format!("{}/skills?library_id={}", self.base_url, library_id);
        self.get(&url).await
    }

    /// Get latest approved version of a skill
    pub async fn get_latest_approved_skill(
        &self,
        skill_id: &str,
    ) -> Result<LatestApprovedResponse> {
        let url = format!("{}/skills/{}/latest-approved", self.base_url, skill_id);
        self.get(&url).await
    }

    /// Get artifact metadata
    pub async fn get_artifact_metadata(&self, artifact_id: &str) -> Result<ArtifactMetadata> {
        let url = format!("{}/artifacts/{}/metadata", self.base_url, artifact_id);
        self.get(&url).await
    }

    /// Get artifact payload
    pub async fn get_artifact_payload(&self, artifact_id: &str) -> Result<Vec<u8>> {
        let url = format!("{}/artifacts/{}/payload", self.base_url, artifact_id);
        let token = self.auth_manager.get_access_token().await?;

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(RelayError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RelayError::Internal(format!(
                "Failed to fetch artifact payload (status {}): {}",
                status, error_text
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(RelayError::Network)?
            .to_vec();

        Ok(bytes)
    }

    /// Get complete artifact (metadata + payload + signature)
    pub async fn get_artifact(&self, artifact_id: &str) -> Result<Artifact> {
        // Fetch metadata
        let metadata = self.get_artifact_metadata(artifact_id).await?;

        // Fetch payload
        let payload = self.get_artifact_payload(artifact_id).await?;

        // Compute content hash
        let content_hash = crate::crypto::compute_content_hash(&payload);

        // Get signature info from metadata endpoint
        let signature_url = format!("{}/artifacts/{}/signature", self.base_url, artifact_id);
        let signature: ArtifactSignature = self.get(&signature_url).await?;

        // Construct artifact
        let artifact = Artifact {
            artifact_id: artifact_id.to_string(),
            skill_id: metadata.name.clone(), // This should come from actual API
            skill_version_id: "unknown".to_string(), // This should come from actual API
            semver: None,
            content_hash,
            created_at: chrono::Utc::now(), // This should come from actual API
            artifact_type: ArtifactType::Generic, // This should come from actual API
            payload,
            metadata,
            signature,
        };

        Ok(artifact)
    }

    /// Get updates since a cursor
    pub async fn get_updates_since(&self, cursor: Option<&str>) -> Result<UpdatesSinceResponse> {
        let url = if let Some(c) = cursor {
            format!("{}/updates/since?cursor={}", self.base_url, c)
        } else {
            format!("{}/updates/since", self.base_url)
        };

        self.get(&url).await
    }

    /// Get signing public key (JWKS or PEM)
    pub async fn get_signing_key(&self) -> Result<String> {
        let url = format!("{}/signing/jwks", self.base_url);

        // For now, return as JSON string - in production, parse JWKS and extract key
        let response: serde_json::Value = self.get(&url).await?;

        // Extract first key's PEM if available
        if let Some(keys) = response.get("keys").and_then(|k| k.as_array()) {
            if let Some(first_key) = keys.first() {
                if let Some(pem) = first_key.get("pem").and_then(|p| p.as_str()) {
                    return Ok(pem.to_string());
                }
            }
        }

        Err(RelayError::Verification(
            "No valid signing key found".to_string(),
        ))
    }

    /// Generic GET request with authentication
    async fn get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let token = self.auth_manager.get_access_token().await?;

        let response = self
            .client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(RelayError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status.as_u16() == 401 {
                return Err(RelayError::Auth(
                    "Unauthorized - token may be expired".to_string(),
                ));
            }

            return Err(RelayError::Internal(format!(
                "API request failed (status {}): {}",
                status, error_text
            )));
        }

        response.json().await.map_err(RelayError::Network)
    }
}
