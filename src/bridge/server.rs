use super::auth::BridgeAuth;
use super::handlers;
use crate::discovery;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tauri::AppHandle;

use crate::bridge::handlers::BridgeState;
use crate::relay::RelayState;

const BRIDGE_HOST: [u8; 4] = [127, 0, 0, 1];
const PUBLIC_PATHS: &[&str] = &["/bridge/ping", "/bridge/handshake"];

#[cfg(not(feature = "custom-protocol"))]
pub async fn start_bridge(
    relay_state: Arc<RelayState>,
    session_token: String,
    port: u16,
    allowed_origin: Option<String>,
) -> crate::error::Result<()> {
    start_bridge_with_app(relay_state, session_token, port, allowed_origin, None).await
}

#[cfg_attr(not(feature = "custom-protocol"), allow(unused_variables))]
pub async fn start_bridge_with_app(
    relay_state: Arc<RelayState>,
    session_token: String,
    port: u16,
    allowed_origin: Option<String>,
    app_handle: Option<AppHandle>,
) -> crate::error::Result<()> {
    let auth = BridgeAuth::new(session_token, allowed_origin);
    let version = env!("CARGO_PKG_VERSION").to_string();
    let state = Arc::new(BridgeState {
        relay_state,
        auth: auth.clone(),
        version,
        #[cfg(feature = "custom-protocol")]
        app_handle,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let public = Router::new()
        .route("/bridge/ping", get(handlers::ping))
        .route("/bridge/handshake", get(handlers::handshake));

    let private = Router::new()
        .route("/bridge/status", get(handlers::status))
        .route("/bridge/refresh", post(handlers::refresh))
        .route("/bridge/cache/clear", post(handlers::cache_clear))
        .route("/bridge/expose", post(handlers::expose))
        .route("/bridge/logs/export", get(handlers::logs_export))
        .route("/bridge/update/status", get(handlers::update_status))
        .route("/bridge/update/apply", post(handlers::update_apply))
        .route("/bridge/discover-apps", post(discovery::discover_apps))
        .route("/bridge/discovery/scan", post(discovery::discovery_scan))
        .route("/bridge/discovery/import", post(discovery::discovery_import))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_bridge_auth_conditional,
        ));

    let app = public
        .merge(private)
        .layer(cors)
        .with_state(state);

    let addr = std::net::SocketAddr::from((BRIDGE_HOST, port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Bridge server listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn require_bridge_auth_conditional(
    axum::extract::State(state): axum::extract::State<Arc<BridgeState>>,
    request: Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let path = request.uri().path();
    let is_public_path = PUBLIC_PATHS.contains(&path);
    
    if is_public_path {
        if path == "/bridge/handshake" {
            state.auth.validate_origin_only(&request)?;
        }
        return Ok(next.run(request).await);
    }
    
    state.auth.validate_request(&request)?;
    Ok(next.run(request).await)
}
