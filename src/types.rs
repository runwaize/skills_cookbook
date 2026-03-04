use serde::{Deserialize, Serialize};

// ===== Core Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: String,
    pub skill_id: String,
    pub skill_version_id: String,
    pub semver: Option<String>,
    pub content_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub artifact_type: ArtifactType,
    pub payload: Vec<u8>,
    pub metadata: ArtifactMetadata,
    pub signature: ArtifactSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Prompt,
    McpTool,
    OpenclawSkill,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub name: String,
    pub description: Option<String>,
    pub inputs: Vec<InputSchema>,
    pub outputs: Vec<OutputSchema>,
    pub compatibility: Vec<String>,
    pub risk_tags: Vec<String>,
    pub required_permissions: Vec<String>,
    pub variables: Vec<VariableSchema>,
    pub approval_state: ApprovalState,
    pub revoked: bool,
    pub revoked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Draft,
    Pending,
    Approved,
    Published,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSchema {
    pub name: String,
    #[serde(rename = "type")]
    pub schema_type: String,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSchema {
    pub name: String,
    #[serde(rename = "type")]
    pub schema_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableSchema {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: String,
    pub scope: VariableScope,
    pub is_secret: bool,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableScope {
    Personal,
    Workspace,
    Project,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSignature {
    pub signature: String,
    pub algorithm: String,
    pub key_id: String,
    pub issued_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ===== Library Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    #[serde(alias = "id")]
    pub library_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(alias = "scope")]
    pub visibility: Visibility,
    #[serde(alias = "latest_approved_release_id")]
    pub latest_approved_release: Option<String>,
    #[serde(default)]
    pub skills: Vec<SkillSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Personal,
    Workspace,
    Project,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSummary {
    #[serde(alias = "id")]
    pub skill_id: String,
    pub name: String,
    pub description: Option<String>,
    pub latest_approved_version: Option<String>,
}

// ===== Relay Status Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayStatus {
    pub connected: bool,
    pub authenticated: bool,
    pub user_email: Option<String>,
    pub workspace_id: Option<String>,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub cache_size_bytes: u64,
    pub artifact_count: usize,
    pub library_count: usize,
    pub guest_skill_count: usize,
    pub mcp_server_running: bool,
    pub mcp_server_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub user_email: Option<String>,
    pub workspace_id: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResult {
    pub updated_artifacts: usize,
    pub revoked_artifacts: usize,
    pub new_libraries: usize,
    pub sync_timestamp: chrono::DateTime<chrono::Utc>,
}

// ===== MCP Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// ===== Variable Resolution Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ResolvedVariable {
    pub name: String,
    pub value: serde_json::Value,
    pub is_secret: bool,
    pub source: VariableSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum VariableSource {
    Environment,
    Keychain,
    Config,
    OnePassword,
    Vault,
    RssDefault,
}

// ===== Cache Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedArtifact {
    pub artifact: Artifact,
    pub cached_at: chrono::DateTime<chrono::Utc>,
    pub last_verified_at: chrono::DateTime<chrono::Utc>,
}

// ===== RSS API Response Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestApprovedResponse {
    pub artifact_id: String,
    pub skill_id: String,
    pub skill_version_id: String,
    pub semver: Option<String>,
    pub content_hash: String,
    pub metadata_url: String,
    pub payload_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatesSinceResponse {
    pub cursor: String,
    #[serde(alias = "changes")]
    pub updates: Vec<ArtifactUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactUpdate {
    pub artifact_id: String,
    pub update_type: UpdateType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateType {
    NewVersion,
    Revoked,
    MetadataChanged,
}

// ===== Local UI Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSkill {
    pub name: String,
    pub has_skill_md: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSkillInfo {
    pub library_name: String,
    pub library_id: String,
    pub skill_name: String,
    pub skill_id: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub committed: bool,
    pub pushed: bool,
    pub skills_pushed: usize,
    pub guest_skills_updated: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    pub name: String,
    pub status: DiagnosticStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStatus {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedSkillInfo {
    pub name: String,
    pub path: String,
    pub already_in_chef: bool,
    pub source_agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub user_role: crate::config::UserRole,
    pub ui_mode: crate::config::UiMode,
    pub chef_dir: String,
    pub guest_dir: String,
    pub workspace_id: Option<String>,
    pub mcp_server_port: u16,
    pub bridge_port: u16,
}

// ===== Token Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub token_type: String,
}
