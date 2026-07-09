use super::auth::BridgeAuth;
use crate::relay::RelayState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "custom-protocol")]
use tauri::{AppHandle, Manager};

#[cfg(feature = "custom-protocol")]
use tauri_plugin_updater::UpdaterExt;

#[cfg(feature = "custom-protocol")]
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

const BRIDGE_CAPABILITIES: &[&str] = &[
    "status",
    "refresh",
    "cache_clear",
    "variables_check",
    "expose",
    "logs_export",
    "update_status",
    "update_apply",
    "discover_apps",
    "discovery_scan",
    "discovery_import",
];

fn get_capabilities() -> Vec<String> {
    BRIDGE_CAPABILITIES.iter().map(|s| s.to_string()).collect()
}

#[cfg(feature = "custom-protocol")]
async fn show_confirmation_dialog(
    app_handle: &AppHandle,
    title: &str,
    message: &str,
    kind: MessageDialogKind,
) -> bool {
    let title = title.to_string();
    let message = message.to_string();
    let app_handle = app_handle.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let app_for_closure = app_handle.clone();
    let _ = app_handle.run_on_main_thread(move || {
        if let Some(window) = app_for_closure.get_webview_window("main") {
            window
                .dialog()
                .message(&message)
                .title(&title)
                .kind(kind)
                .buttons(MessageDialogButtons::OkCancel)
                .show(move |result| {
                    let _ = tx.send(result);
                });
        } else {
            let _ = tx.send(false);
        }
    });
    rx.recv().unwrap_or(false)
}

#[derive(Clone)]
pub struct BridgeState {
    pub relay_state: Arc<RelayState>,
    pub auth: BridgeAuth,
    pub version: String,
    #[cfg(feature = "custom-protocol")]
    pub app_handle: Option<AppHandle>,
}

#[derive(Serialize)]
pub struct PingResponse {
    pub version: String,
    pub capabilities: Vec<String>,
}

#[derive(Serialize)]
pub struct HandshakeResponse {
    pub token: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

#[derive(Deserialize)]
pub struct ExposePayload {
    pub library_ids: Option<Vec<String>>,
}

pub async fn ping(State(state): State<Arc<BridgeState>>) -> impl IntoResponse {
    Json(PingResponse {
        version: state.version.clone(),
        capabilities: get_capabilities(),
    })
}

pub async fn handshake(State(state): State<Arc<BridgeState>>) -> impl IntoResponse {
    Json(HandshakeResponse {
        token: state.auth.session_token.clone(),
        version: state.version.clone(),
        capabilities: get_capabilities(),
    })
}

fn handle_relay_error(error: crate::error::RelayError) -> axum::response::Response {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
}

pub async fn status(State(state): State<Arc<BridgeState>>) -> impl IntoResponse {
    match state.relay_state.get_status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => handle_relay_error(e),
    }
}

pub async fn refresh(State(state): State<Arc<BridgeState>>) -> impl IntoResponse {
    match state.relay_state.refresh_skills().await {
        Ok(result) => Json(result).into_response(),
        Err(e) => handle_relay_error(e),
    }
}

pub async fn cache_clear(State(state): State<Arc<BridgeState>>) -> impl IntoResponse {
    #[cfg(feature = "custom-protocol")]
    if let Some(app) = &state.app_handle {
        let relay = state.relay_state.clone();
        let app_handle = app.clone();
        tokio::spawn(async move {
            let confirmed = show_confirmation_dialog(
                &app_handle,
                "Clear Cache",
                "Are you sure you want to clear the cache? Skills will need to be re-downloaded.",
                MessageDialogKind::Warning,
            )
            .await;
            if confirmed {
                if let Err(e) = relay.clear_cache().await {
                    tracing::error!("Failed to clear cache: {}", e);
                }
            }
        });
        return StatusCode::ACCEPTED.into_response();
    }
    match state.relay_state.clear_cache().await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => handle_relay_error(e),
    }
}

#[allow(dead_code)]
async fn variables_check_impl(state: Arc<BridgeState>, skill_id: String) -> impl IntoResponse {
    let skill_id = skill_id.trim();
    if skill_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "missing or empty skill_id"})),
        )
            .into_response();
    }
    match state.relay_state.check_missing_variables(skill_id).await {
        Ok(missing) => Json(serde_json::json!({ "missing_variables": missing })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[allow(dead_code)]
pub async fn variables_check(
    State(state): State<Arc<BridgeState>>,
    Path(skill_id): Path<String>,
) -> impl IntoResponse {
    variables_check_impl(state, skill_id).await
}

pub async fn expose(
    State(state): State<Arc<BridgeState>>,
    Json(payload): Json<ExposePayload>,
) -> impl IntoResponse {
    let library_ids = payload.library_ids.unwrap_or_default();
    #[cfg(feature = "custom-protocol")]
    if let Some(app) = &state.app_handle {
        let relay = state.relay_state.clone();
        let app_handle = app.clone();
        let libs = library_ids.clone();
        tokio::spawn(async move {
            let confirmed = show_confirmation_dialog(
                &app_handle,
                "Change Exposed Libraries",
                "Are you sure you want to change which libraries are exposed to agents?",
                MessageDialogKind::Warning,
            )
            .await;
            if confirmed {
                if let Err(e) = relay.set_exposed_libraries(libs) {
                    tracing::error!("Failed to update exposed libraries: {}", e);
                }
            }
        });
        return StatusCode::ACCEPTED.into_response();
    }
    match state.relay_state.set_exposed_libraries(library_ids) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => handle_relay_error(e),
    }
}

#[allow(unused_variables)]
pub async fn logs_export(State(state): State<Arc<BridgeState>>) -> impl IntoResponse {
    #[cfg(feature = "custom-protocol")]
    if let Some(app) = &state.app_handle {
        let confirmed = show_confirmation_dialog(
            app,
            "Export Logs",
            "Export diagnostic logs? This will include system information but no secrets.",
            MessageDialogKind::Info,
        )
        .await;
        if !confirmed {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    const REDACTED_MESSAGE: &[u8] = b"[REDACTED] No logs collected in this build.\n";
    let redacted = REDACTED_MESSAGE.to_vec();
    (
        [
            ("content-type", "application/zip"),
            (
                "content-disposition",
                &format!(
                    "attachment; filename=\"relay-logs-{}.zip\"",
                    chrono::Utc::now().format("%Y%m%d-%H%M%S")
                ),
            ),
        ],
        redacted,
    )
        .into_response()
}

#[derive(Serialize)]
pub struct UpdateStatusResponse {
    pub current_version: String,
    pub update_available: bool,
    pub latest_version: Option<String>,
}

#[cfg(feature = "custom-protocol")]
async fn check_for_update(app: &AppHandle) -> Option<String> {
    let updater = app.updater_builder().build().ok()?;
    let update = updater.check().await.ok()??;
    Some(update.version)
}

pub async fn update_status(State(state): State<Arc<BridgeState>>) -> impl IntoResponse {
    #[cfg(feature = "custom-protocol")]
    let (update_available, latest_version) = if let Some(app) = &state.app_handle {
        if let Some(version) = check_for_update(app).await {
            return Json(UpdateStatusResponse {
                current_version: state.version.clone(),
                update_available: true,
                latest_version: Some(version),
            });
        }
        (false, None)
    } else {
        (false, None)
    };
    #[cfg(not(feature = "custom-protocol"))]
    let (update_available, latest_version) = (false, None);

    Json(UpdateStatusResponse {
        current_version: state.version.clone(),
        update_available,
        latest_version,
    })
}

#[cfg(feature = "custom-protocol")]
async fn apply_update_internal(app: &AppHandle) -> Result<(), String> {
    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| format!("Failed to build updater: {}", e))?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))?
        .ok_or_else(|| "No update available".to_string())?;

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("Update download/install failed: {}", e))?;

    tauri::process::restart(&app.env());
}

#[allow(unused_variables)]
pub async fn update_apply(State(state): State<Arc<BridgeState>>) -> impl IntoResponse {
    #[cfg(feature = "custom-protocol")]
    if let Some(app) = &state.app_handle {
        let confirmed = show_confirmation_dialog(
            app,
            "Apply Update",
            "Apply the update now? The application will restart.",
            MessageDialogKind::Warning,
        )
        .await;
        if !confirmed {
            return StatusCode::FORBIDDEN.into_response();
        }
        match apply_update_internal(app).await {
            Ok(()) => StatusCode::ACCEPTED.into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            "Update apply requires Tauri app context",
        )
            .into_response()
    }
    #[cfg(not(feature = "custom-protocol"))]
    {
        (
            StatusCode::NOT_IMPLEMENTED,
            "Update apply requires Tauri app context",
        )
            .into_response()
    }
}
