use crate::error::Result;
use crate::relay::RelayState;
use crate::types::{McpError, McpRequest, McpResponse};
use axum::{extract::State, routing::post, Json, Router};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub async fn start_mcp_server(relay_state: Arc<RelayState>) -> Result<()> {
    let host = relay_state.config.read().mcp_server_host.clone();
    let port = relay_state.config.read().mcp_server_port;

    // Build router
    let app = Router::new()
        .route("/", post(handle_mcp_request))
        .layer(CorsLayer::permissive())
        .with_state(relay_state);

    let addr = format!("{}:{}", host, port);
    tracing::info!("Starting MCP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_mcp_request(
    State(relay_state): State<Arc<RelayState>>,
    Json(request): Json<McpRequest>,
) -> Json<McpResponse> {
    tracing::debug!(
        "MCP request: method={}, id={:?}",
        request.method,
        request.id
    );

    let result = match request.method.as_str() {
        "skills.list" => super::handlers::list_skills(relay_state, request.params).await,
        "skills.get" => super::handlers::get_skill(relay_state, request.params).await,
        "libraries.list" => super::handlers::list_libraries(relay_state, request.params).await,
        "libraries.get" => super::handlers::get_library(relay_state, request.params).await,
        "skills.status" => super::handlers::get_status(relay_state, request.params).await,
        "skills.refresh" => super::handlers::refresh_skills(relay_state, request.params).await,
        _ => Err(crate::error::RelayError::Mcp(format!(
            "Unknown method: {}",
            request.method
        ))),
    };

    Json(match result {
        Ok(value) => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(value),
            error: None,
        },
        Err(e) => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(McpError {
                code: -32603,
                message: e.to_string(),
                data: None,
            }),
        },
    })
}
