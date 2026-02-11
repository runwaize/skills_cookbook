use crate::error::{RelayError, Result};
use crate::types::{Artifact, CachedArtifact};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ArtifactCache {
    cache_dir: PathBuf,
    cache_ttl: Duration,
    max_size_bytes: u64,
    db: sled::Db,
    in_memory: Arc<RwLock<HashMap<String, CachedArtifact>>>,
}

impl ArtifactCache {
    pub fn new(cache_dir: PathBuf, cache_ttl_hours: u64, max_size_mb: usize) -> Result<Self> {
        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| RelayError::Cache(format!("Failed to create cache dir: {}", e)))?;

        // Open sled database
        let db_path = cache_dir.join("artifacts.db");
        let db = sled::open(&db_path)
            .map_err(|e| RelayError::Cache(format!("Failed to open cache database: {}", e)))?;

        Ok(Self {
            cache_dir,
            cache_ttl: Duration::hours(cache_ttl_hours as i64),
            max_size_bytes: (max_size_mb as u64) * 1024 * 1024,
            db,
            in_memory: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get artifact from cache by artifact_id
    pub fn get(&self, artifact_id: &str) -> Result<Option<Artifact>> {
        // Check in-memory cache first
        {
            let memory_cache = self.in_memory.read();
            if let Some(cached) = memory_cache.get(artifact_id) {
                // Check if still valid
                if Utc::now() - cached.cached_at < self.cache_ttl {
                    tracing::debug!("Cache hit (memory): {}", artifact_id);
                    return Ok(Some(cached.artifact.clone()));
                }
            }
        }

        // Check disk cache
        let key = format!("artifact:{}", artifact_id);
        match self.db.get(&key) {
            Ok(Some(data)) => {
                let cached: CachedArtifact = bincode::deserialize(&data)
                    .map_err(|e| RelayError::Cache(format!("Failed to deserialize: {}", e)))?;

                // Check if still valid
                if Utc::now() - cached.cached_at < self.cache_ttl {
                    tracing::debug!("Cache hit (disk): {}", artifact_id);

                    // Update in-memory cache
                    self.in_memory.write().insert(artifact_id.to_string(), cached.clone());

                    Ok(Some(cached.artifact))
                } else {
                    tracing::debug!("Cache expired: {}", artifact_id);
                    Ok(None)
                }
            }
            Ok(None) => {
                tracing::debug!("Cache miss: {}", artifact_id);
                Ok(None)
            }
            Err(e) => Err(RelayError::Cache(format!("Cache read error: {}", e))),
        }
    }

    /// Store artifact in cache
    pub fn put(&self, artifact: Artifact) -> Result<()> {
        let cached = CachedArtifact {
            artifact: artifact.clone(),
            cached_at: Utc::now(),
            last_verified_at: Utc::now(),
        };

        // Store in memory
        self.in_memory.write().insert(artifact.artifact_id.clone(), cached.clone());

        // Store on disk
        let key = format!("artifact:{}", artifact.artifact_id);
        let data = bincode::serialize(&cached)
            .map_err(|e| RelayError::Cache(format!("Failed to serialize: {}", e)))?;

        self.db
            .insert(&key, data)
            .map_err(|e| RelayError::Cache(format!("Failed to write to cache: {}", e)))?;

        self.db
            .flush()
            .map_err(|e| RelayError::Cache(format!("Failed to flush cache: {}", e)))?;

        tracing::debug!("Cached artifact: {}", artifact.artifact_id);

        // Check cache size and evict if needed
        self.evict_if_needed()?;

        Ok(())
    }

    /// Remove artifact from cache
    pub fn remove(&self, artifact_id: &str) -> Result<()> {
        // Remove from memory
        self.in_memory.write().remove(artifact_id);

        // Remove from disk
        let key = format!("artifact:{}", artifact_id);
        self.db
            .remove(&key)
            .map_err(|e| RelayError::Cache(format!("Failed to remove from cache: {}", e)))?;

        tracing::debug!("Removed from cache: {}", artifact_id);

        Ok(())
    }

    /// Clear all cached artifacts
    pub fn clear(&self) -> Result<()> {
        // Clear in-memory cache
        self.in_memory.write().clear();

        // Clear disk cache
        self.db
            .clear()
            .map_err(|e| RelayError::Cache(format!("Failed to clear cache: {}", e)))?;

        tracing::info!("Cache cleared");

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let size_bytes = self.db.size_on_disk().unwrap_or(0);
        let count = self.in_memory.read().len();

        CacheStats {
            size_bytes,
            artifact_count: count,
            max_size_bytes: self.max_size_bytes,
        }
    }

    /// Evict oldest entries if cache exceeds max size
    fn evict_if_needed(&self) -> Result<()> {
        let current_size = self.db.size_on_disk().unwrap_or(0);

        if current_size > self.max_size_bytes {
            tracing::warn!(
                "Cache size ({} bytes) exceeds limit ({} bytes), evicting oldest entries",
                current_size,
                self.max_size_bytes
            );

            // Collect all cached items with timestamps
            let mut items: Vec<(String, DateTime<Utc>)> = Vec::new();

            for item in self.db.iter() {
                if let Ok((key, value)) = item {
                    if let Ok(key_str) = std::str::from_utf8(&key) {
                        if key_str.starts_with("artifact:") {
                            if let Ok(cached) = bincode::deserialize::<CachedArtifact>(&value) {
                                items.push((key_str.to_string(), cached.cached_at));
                            }
                        }
                    }
                }
            }

            // Sort by timestamp (oldest first)
            items.sort_by_key(|(_, timestamp)| *timestamp);

            // Remove oldest 25% of items
            let to_remove = items.len() / 4;
            for (key, _) in items.iter().take(to_remove) {
                let artifact_id = key.strip_prefix("artifact:").unwrap_or(key);
                self.remove(artifact_id)?;
            }

            tracing::info!("Evicted {} oldest cache entries", to_remove);
        }

        Ok(())
    }

    /// Update last verified timestamp for an artifact
    pub fn update_verification_time(&self, artifact_id: &str) -> Result<()> {
        // Update in memory
        if let Some(cached) = self.in_memory.write().get_mut(artifact_id) {
            cached.last_verified_at = Utc::now();
        }

        // Update on disk
        let key = format!("artifact:{}", artifact_id);
        if let Some(data) = self.db.get(&key)
            .map_err(|e| RelayError::Cache(format!("Cache read error: {}", e)))?
        {
            let mut cached: CachedArtifact = bincode::deserialize(&data)
                .map_err(|e| RelayError::Cache(format!("Failed to deserialize: {}", e)))?;

            cached.last_verified_at = Utc::now();

            let updated_data = bincode::serialize(&cached)
                .map_err(|e| RelayError::Cache(format!("Failed to serialize: {}", e)))?;

            self.db.insert(&key, updated_data)
                .map_err(|e| RelayError::Cache(format!("Failed to update cache: {}", e)))?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size_bytes: u64,
    pub artifact_count: usize,
    pub max_size_bytes: u64,
}

// CachedArtifact already implements Serialize/Deserialize in types.rs
