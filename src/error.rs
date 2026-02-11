use thiserror::Error;

#[derive(Error, Debug)]
pub enum RelayError {
    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Verification error: {0}")]
    Verification(String),

    #[error("Variable resolution error: {0}")]
    VariableResolution(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Keychain error: {0}")]
    Keychain(String),

    #[error("Artifact not found: {0}")]
    ArtifactNotFound(String),

    #[error("Artifact revoked: {0}")]
    ArtifactRevoked(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, RelayError>;
