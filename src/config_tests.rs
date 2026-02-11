#[cfg(test)]
mod tests {
    use crate::config::*;
    use std::env;

    #[test]
    fn test_config_default() {
        let config = Config::default();

        assert_eq!(config.rss_api_url, "https://api.skills.cookbook/v1");
        assert_eq!(config.oauth_client_id, "skills-relay-client");
        assert_eq!(config.mcp_server_host, "127.0.0.1");
        assert_eq!(config.mcp_server_port, 9876);
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
    fn test_device_id_persistence() {
        // Get device ID
        let id1 = Config::device_id().unwrap();
        assert!(!id1.is_empty());

        // Should get same ID on second call
        let id2 = Config::device_id().unwrap();
        assert_eq!(id1, id2);
    }
}
