#[cfg(test)]
mod tests {
    use crate::types::*;
    use chrono::Utc;

    #[test]
    fn test_artifact_type_serialization() {
        let types = vec![
            ArtifactType::Prompt,
            ArtifactType::McpTool,
            ArtifactType::OpenclawSkill,
            ArtifactType::Generic,
        ];
        for artifact_type in types {
            let json = serde_json::to_string(&artifact_type).unwrap();
            let deserialized: ArtifactType = serde_json::from_str(&json).unwrap();
            assert_eq!(
                format!("{:?}", artifact_type),
                format!("{:?}", deserialized)
            );
        }
    }

    #[test]
    fn test_approval_state_serialization() {
        let states = vec![
            ApprovalState::Draft,
            ApprovalState::Pending,
            ApprovalState::Approved,
            ApprovalState::Published,
            ApprovalState::Rejected,
        ];
        for state in states {
            let json = serde_json::to_string(&state).unwrap();
            let deserialized: ApprovalState = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", state), format!("{:?}", deserialized));
        }
    }

    #[test]
    fn test_variable_scope_serialization() {
        let scope = VariableScope::Personal;
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, "\"personal\"");

        let scope = VariableScope::Workspace;
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, "\"workspace\"");

        let scope = VariableScope::Project;
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, "\"project\"");
    }

    #[test]
    fn test_variable_source_serialization() {
        let sources = vec![
            VariableSource::Environment,
            VariableSource::Keychain,
            VariableSource::Config,
            VariableSource::OnePassword,
            VariableSource::Vault,
            VariableSource::RssDefault,
        ];
        for source in sources {
            let json = serde_json::to_string(&source).unwrap();
            let deserialized: VariableSource = serde_json::from_str(&json).unwrap();
            assert_eq!(source, deserialized);
        }
    }

    #[test]
    fn test_token_pair_serialization() {
        let token_pair = TokenPair {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            expires_at: Utc::now(),
            token_type: "Bearer".to_string(),
        };

        let json = serde_json::to_string(&token_pair).unwrap();
        assert!(json.contains("test_access_token"));
        assert!(json.contains("test_refresh_token"));
        assert!(json.contains("Bearer"));
    }

    #[test]
    fn test_mcp_request_serialization() {
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::json!(1),
            method: "skills.list".to_string(),
            params: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"skills.list\""));
    }

    #[test]
    fn test_mcp_response_success() {
        let response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: serde_json::json!(1),
            result: Some(serde_json::json!({"status": "ok"})),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_mcp_response_error() {
        let response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: serde_json::json!(1),
            result: None,
            error: Some(McpError {
                code: -32603,
                message: "Internal error".to_string(),
                data: None,
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("-32603"));
        assert!(json.contains("Internal error"));
    }

    #[test]
    fn test_update_type_serialization() {
        let updates = vec![
            UpdateType::NewVersion,
            UpdateType::Revoked,
            UpdateType::MetadataChanged,
        ];
        for update in updates {
            let json = serde_json::to_string(&update).unwrap();
            let deserialized: UpdateType = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", update), format!("{:?}", deserialized));
        }
    }

    #[test]
    fn test_visibility_serialization() {
        let visibilities = vec![
            Visibility::Personal,
            Visibility::Workspace,
            Visibility::Project,
            Visibility::Public,
        ];
        for vis in visibilities {
            let json = serde_json::to_string(&vis).unwrap();
            let deserialized: Visibility = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", vis), format!("{:?}", deserialized));
        }
    }
}
