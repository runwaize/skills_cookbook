use crate::error::{RelayError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rss_api_url: String,
    pub oauth_client_id: String,
    pub oauth_auth_url: String,
    pub oauth_token_url: String,

    pub mcp_server_host: String,
    pub mcp_server_port: u16,
    pub mcp_enable_stdio: bool,

    pub cache_dir: PathBuf,
    pub cache_max_size_mb: usize,
    pub cache_ttl_hours: u64,

    pub sync_interval_minutes: u64,
    pub verification_required: bool,
    pub fail_closed_on_verification_error: bool,

    pub log_level: String,
    pub debug_mode: bool,

    pub default_libraries: Vec<String>,
    pub update_mode: UpdateMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateMode {
    LatestApproved,
    Pinned,
    Manual,
}

impl Default for Config {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("skills-cookbook-relay");

        Self {
            rss_api_url: "https://api.skills.cookbook/v1".to_string(),
            oauth_client_id: "skills-relay-client".to_string(),
            oauth_auth_url: "https://auth.skills.cookbook/oauth/authorize".to_string(),
            oauth_token_url: "https://auth.skills.cookbook/oauth/token".to_string(),

            mcp_server_host: "127.0.0.1".to_string(),
            mcp_server_port: 9876,
            mcp_enable_stdio: true,

            cache_dir,
            cache_max_size_mb: 500,
            cache_ttl_hours: 24,

            sync_interval_minutes: 15,
            verification_required: true,
            fail_closed_on_verification_error: true,

            log_level: "info".to_string(),
            debug_mode: false,

            default_libraries: vec![],
            update_mode: UpdateMode::LatestApproved,
        }
    }
}

impl Config {
    /// Load configuration from file or create default
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| RelayError::Config(format!("Failed to read config: {}", e)))?;

            toml::from_str(&content)
                .map_err(|e| RelayError::Config(format!("Failed to parse config: {}", e)))
        } else {
            // Create default config
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RelayError::Config(format!("Failed to create config dir: {}", e)))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| RelayError::Config(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(&config_path, content)
            .map_err(|e| RelayError::Config(format!("Failed to write config: {}", e)))?;

        Ok(())
    }

    /// Get the path to the config file
    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| RelayError::Config("Cannot determine config directory".to_string()))?;

        Ok(config_dir.join("skills-cookbook-relay").join("config.toml"))
    }

    /// Get the device ID for this relay instance
    pub fn device_id() -> Result<String> {
        let device_id_path = Self::device_id_path()?;

        if device_id_path.exists() {
            std::fs::read_to_string(&device_id_path)
                .map_err(|e| RelayError::Config(format!("Failed to read device ID: {}", e)))
        } else {
            // Generate new device ID
            let device_id = uuid::Uuid::new_v4().to_string();

            // Ensure parent directory exists
            if let Some(parent) = device_id_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| RelayError::Config(format!("Failed to create device ID dir: {}", e)))?;
            }

            std::fs::write(&device_id_path, &device_id)
                .map_err(|e| RelayError::Config(format!("Failed to write device ID: {}", e)))?;

            Ok(device_id)
        }
    }

    /// Get the path to the device ID file
    fn device_id_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| RelayError::Config("Cannot determine config directory".to_string()))?;

        Ok(config_dir.join("skills-cookbook-relay").join("device_id"))
    }
}
