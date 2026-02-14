//! HTTP handlers for discovery endpoints.

use super::{
    build_skill_zip, discover_apps_impl, resolve_client_path, resolve_skill_md_path, scan_path_impl,
    ImportRequest, ImportResponse, ImportResult, ScanRequest, ScanResponse,
};
use crate::bridge::handlers::BridgeState;
use crate::studio_client::StudioClient;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::path::Path;
use std::sync::Arc;

pub async fn discover_apps() -> impl IntoResponse {
    let response = discover_apps_impl();
    Json(response)
}

pub async fn discovery_scan(
    State(_state): State<Arc<BridgeState>>,
    Json(req): Json<ScanRequest>,
) -> impl IntoResponse {
    if let Some(ref path_str) = req.path {
        let path = Path::new(path_str);
        if !path.exists() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Path does not exist: {}", path_str)})),
            )
                .into_response();
        }
        if path.is_file() {
            if path.file_name().map(|n| n == "SKILL.md").unwrap_or(false) {
                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"error": e.to_string()})),
                        )
                            .into_response()
                    }
                };
                let (meta, _) = super::extract_frontmatter(&content);
                let name = super::derive_skill_name(&meta, path);
                let identifier = path.to_string_lossy().to_string();
                let client_id = req.client_id.as_deref().unwrap_or("unknown");
                return Json(ScanResponse {
                    skills: vec![super::ScannedSkill {
                        path: path.to_string_lossy().to_string(),
                        name,
                        identifier,
                        client_id: client_id.to_string(),
                    }],
                })
                .into_response();
            }
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Path is a file but not SKILL.md"})),
            )
                .into_response();
        }
        match scan_path_impl(path, req.client_id.as_deref().unwrap_or("unknown"), 0) {
            Ok(skills) => Json(ScanResponse { skills }).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response(),
        }
    } else if let Some(ref client_id) = req.client_id {
        match resolve_client_path(client_id) {
            Some(base_path) => match scan_path_impl(&base_path, client_id, 0) {
                Ok(skills) => Json(ScanResponse { skills }).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response(),
            },
            None => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Client '{}' not detected", client_id)})),
            )
                .into_response(),
        }
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Either 'path' or 'client_id' is required"})),
        )
            .into_response()
    }
}

pub async fn discovery_import(
    State(state): State<Arc<BridgeState>>,
    Json(req): Json<ImportRequest>,
) -> impl IntoResponse {
    if req.paths.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "paths cannot be empty"})),
        )
            .into_response();
    }
    if req.studio_token.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "studio_token is required"})),
        )
            .into_response();
    }
    let config = state.relay_state.config();
    let client = StudioClient::new(config.studio_api_url.clone());
    let mut results = Vec::with_capacity(req.paths.len());
    for path_str in &req.paths {
        match resolve_skill_md_path(path_str) {
            Ok(skill_md_path) => match build_skill_zip(&skill_md_path) {
                Ok(zip_bytes) => match client.upload_skill(&req.studio_token, zip_bytes).await {
                    Ok(skill_id) => results.push(ImportResult {
                        path: path_str.clone(),
                        success: true,
                        skill_id: Some(skill_id),
                        error: None,
                    }),
                    Err(e) => results.push(ImportResult {
                        path: path_str.clone(),
                        success: false,
                        skill_id: None,
                        error: Some(e.to_string()),
                    }),
                },
                Err(e) => results.push(ImportResult {
                    path: path_str.clone(),
                    success: false,
                    skill_id: None,
                    error: Some(e.to_string()),
                }),
            },
            Err(e) => results.push(ImportResult {
                path: path_str.clone(),
                success: false,
                skill_id: None,
                error: Some(e.to_string()),
            }),
        }
    }
    Json(ImportResponse { results }).into_response()
}
