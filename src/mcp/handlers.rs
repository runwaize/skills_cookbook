use crate::error::Result;
use crate::relay::RelayState;
use serde_json::Value;
use std::sync::Arc;

/// Handle skills.list MCP method
pub async fn list_skills(
    relay_state: Arc<RelayState>,
    params: Option<Value>,
) -> Result<Value> {
    let library_id = params
        .as_ref()
        .and_then(|p| p.get("library_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let skills = relay_state.list_skills(library_id.as_deref()).await?;

    Ok(serde_json::to_value(skills)?)
}

/// Handle skills.get MCP method
pub async fn get_skill(
    relay_state: Arc<RelayState>,
    params: Option<Value>,
) -> Result<Value> {
    let params = params.ok_or_else(|| {
        crate::error::RelayError::Mcp("Missing parameters for skills.get".to_string())
    })?;

    let skill_id = params
        .get("skill_id")
        .or_else(|| params.get("slug"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            crate::error::RelayError::Mcp("Missing skill_id or slug parameter".to_string())
        })?;

    let version = params
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("latest_approved");

    let skill = relay_state.get_skill(skill_id, version).await?;

    Ok(serde_json::to_value(skill)?)
}

/// Handle libraries.list MCP method
pub async fn list_libraries(
    relay_state: Arc<RelayState>,
    _params: Option<Value>,
) -> Result<Value> {
    let libraries = relay_state.list_libraries().await?;

    Ok(serde_json::to_value(libraries)?)
}

/// Handle libraries.get MCP method
pub async fn get_library(
    relay_state: Arc<RelayState>,
    params: Option<Value>,
) -> Result<Value> {
    let params = params.ok_or_else(|| {
        crate::error::RelayError::Mcp("Missing parameters for libraries.get".to_string())
    })?;

    let library_id = params
        .get("library_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            crate::error::RelayError::Mcp("Missing library_id parameter".to_string())
        })?;

    let release = params
        .get("release")
        .and_then(|v| v.as_str())
        .unwrap_or("latest_approved");

    let library = relay_state.get_library(library_id, release).await?;

    Ok(serde_json::to_value(library)?)
}

/// Handle skills.status MCP method
pub async fn get_status(
    relay_state: Arc<RelayState>,
    _params: Option<Value>,
) -> Result<Value> {
    let status = relay_state.get_status().await?;

    Ok(serde_json::to_value(status)?)
}

/// Handle skills.refresh MCP method
pub async fn refresh_skills(
    relay_state: Arc<RelayState>,
    _params: Option<Value>,
) -> Result<Value> {
    let result = relay_state.refresh_skills().await?;

    Ok(serde_json::to_value(result)?)
}
