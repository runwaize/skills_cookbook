#[cfg(test)]
mod tests {
    use crate::config::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();

        assert_eq!(config.rss_api_url, "https://api.skills.cookbook/v1");
        assert_eq!(config.oauth_client_id, "skills-relay-client");
        assert_eq!(config.mcp_server_host, "127.0.0.1");
        assert_eq!(config.mcp_server_port, 9876);
        assert_eq!(config.bridge_port, 9123);
        assert_eq!(config.skills_web_url, "https://skills.runwaize.com");
        assert!(config.verification_required);
        assert!(config.fail_closed_on_verification_error);
    }

    #[test]
    fn test_update_mode_serialization() {
        let mode = UpdateMode::LatestApproved;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"latest_approved\"");

        let mode = UpdateMode::Pinned;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"pinned\"");

        let mode = UpdateMode::Manual;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"manual\"");
    }

    #[test]
    fn test_update_mode_deserialization() {
        let mode: UpdateMode = serde_json::from_str("\"latest_approved\"").unwrap();
        assert!(matches!(mode, UpdateMode::LatestApproved));

        let mode: UpdateMode = serde_json::from_str("\"pinned\"").unwrap();
        assert!(matches!(mode, UpdateMode::Pinned));

        let mode: UpdateMode = serde_json::from_str("\"manual\"").unwrap();
        assert!(matches!(mode, UpdateMode::Manual));
    }

    #[test]
    #[serial]
    fn test_config_load_save_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().to_path_buf();
        env::set_var("CONFIG_DIR", config_dir.as_os_str());

        let config1 = Config::default();
        config1.save().unwrap();
        let config2 = Config::load().unwrap();
        assert_eq!(config1.rss_api_url, config2.rss_api_url);
        assert_eq!(config1.bridge_port, config2.bridge_port);
        assert_eq!(config1.exposed_libraries, config2.exposed_libraries);
    }

    #[test]
    #[serial]
    fn test_config_missing_file_creates_default() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().to_path_buf();
        env::set_var("CONFIG_DIR", config_dir.as_os_str());
        let config = Config::load().unwrap();
        assert_eq!(config.rss_api_url, "https://api.skills.cookbook/v1");
        assert!(config_dir.join("skill-cookbook-relay").join("config.toml").exists());
    }

    #[test]
    #[serial]
    fn test_config_malformed_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().to_path_buf();
        let skill_relay_dir = config_dir.join("skill-cookbook-relay");
        std::fs::create_dir_all(&skill_relay_dir).unwrap();
        std::fs::write(skill_relay_dir.join("config.toml"), "invalid toml {").unwrap();
        env::set_var("CONFIG_DIR", config_dir.as_os_str());
        let result = Config::load();
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_device_id_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().to_path_buf();
        env::set_var("CONFIG_DIR", config_dir.as_os_str());
        let id1 = Config::device_id().unwrap();
        assert!(!id1.is_empty());
        let id2 = Config::device_id().unwrap();
        assert_eq!(id1, id2);
    }
}
