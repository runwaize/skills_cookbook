#[cfg(test)]
mod tests {
    use crate::types::*;
    use chrono::Utc;

    #[test]
    fn test_artifact_type_serialization() {
        let artifact_type = ArtifactType::Prompt;
        let json = serde_json::to_string(&artifact_type).unwrap();
        assert_eq!(json, "\"prompt\"");

        let artifact_type = ArtifactType::McpTool;
        let json = serde_json::to_string(&artifact_type).unwrap();
        assert_eq!(json, "\"mcp_tool\"");
    }

    #[test]
    fn test_approval_state_serialization() {
        let state = ApprovalState::Approved;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"approved\"");

        let state = ApprovalState::Draft;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"draft\"");
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
        let source = VariableSource::Environment;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"environment\"");

        let source = VariableSource::Keychain;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"keychain\"");
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
        let update = UpdateType::NewVersion;
        let json = serde_json::to_string(&update).unwrap();
        assert_eq!(json, "\"new_version\"");

        let update = UpdateType::Revoked;
        let json = serde_json::to_string(&update).unwrap();
        assert_eq!(json, "\"revoked\"");
    }

    #[test]
    fn test_visibility_serialization() {
        let vis = Visibility::Public;
        let json = serde_json::to_string(&vis).unwrap();
        assert_eq!(json, "\"public\"");

        let vis = Visibility::Personal;
        let json = serde_json::to_string(&vis).unwrap();
        assert_eq!(json, "\"personal\"");
    }
}
