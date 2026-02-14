#[cfg(test)]
mod tests {
    use crate::error::RelayError;

    #[test]
    fn test_error_display() {
        let err = RelayError::Auth("test auth error".to_string());
        assert!(err.to_string().contains("Authentication error"));
        assert!(err.to_string().contains("test auth error"));
    }

    #[test]
    fn test_error_variants() {
        let variants: Vec<RelayError> = vec![
            RelayError::Auth("a".to_string()),
            RelayError::Cache("c".to_string()),
            RelayError::Verification("v".to_string()),
            RelayError::VariableResolution("vr".to_string()),
            RelayError::Config("cfg".to_string()),
            RelayError::Keychain("kc".to_string()),
            RelayError::ArtifactNotFound("anf".to_string()),
            RelayError::ArtifactRevoked("ar".to_string()),
            RelayError::InvalidSignature("is".to_string()),
            RelayError::Mcp("mcp".to_string()),
            RelayError::Internal("int".to_string()),
            RelayError::Discovery("d".to_string()),
            RelayError::Studio("s".to_string()),
        ];
        for err in variants {
            let msg = err.to_string();
            assert!(!msg.is_empty());
            assert!(
                std::any::type_name_of_val(&err).contains("RelayError"),
                "Error should be RelayError type"
            );
        }
    }

    #[test]
    fn test_error_from_io() {
        use std::io;
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let relay_err: RelayError = io_err.into();
        assert!(matches!(relay_err, RelayError::Io(_)));
    }
}
