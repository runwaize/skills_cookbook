// Skill Cookbook Relay - Local MCP server for secure skill delivery
// Prevents instantiation of Windows on macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod bridge;
mod cache;
mod config;
mod crypto;
mod error;
mod mcp;
mod relay;
mod rss_client;
mod types;
mod variables;

use anyhow::Result;
use relay::RelayState;
use std::sync::Arc;
use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tauri::command]
async fn get_relay_status(
    state: tauri::State<'_, Arc<RelayState>>,
) -> Result<types::RelayStatus, String> {
    state.get_status().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn login_to_rss(
    state: tauri::State<'_, Arc<RelayState>>,
) -> Result<types::AuthStatus, String> {
    state.login().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn logout_from_rss(state: tauri::State<'_, Arc<RelayState>>) -> Result<(), String> {
    state.logout().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn refresh_skills(
    state: tauri::State<'_, Arc<RelayState>>,
) -> Result<types::RefreshResult, String> {
    state.refresh_skills().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_libraries(
    state: tauri::State<'_, Arc<RelayState>>,
) -> Result<Vec<types::Library>, String> {
    state.list_libraries().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_cache(state: tauri::State<'_, Arc<RelayState>>) -> Result<(), String> {
    state.clear_cache().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn wipe_all_data(state: tauri::State<'_, Arc<RelayState>>) -> Result<(), String> {
    state.wipe_all_data().await.map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "skill_cookbook_relay=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    // Load configuration
    let config = config::Config::load()?;
    tracing::info!("Configuration loaded");

    // Initialize relay state
    let relay_state = Arc::new(RelayState::new(config.clone()).await?);
    tracing::info!("Relay state initialized");

    // Generate session token for bridge (will be used in setup)
    let session_token = uuid::Uuid::new_v4().to_string();

    // Spawn MCP server task
    let mcp_relay_state = relay_state.clone();
    tokio::spawn(async move {
        if let Err(e) = mcp::server::start_mcp_server(mcp_relay_state).await {
            tracing::error!("MCP server error: {}", e);
        }
    });

    // Build Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(relay_state)
        .invoke_handler(tauri::generate_handler![
            get_relay_status,
            login_to_rss,
            logout_from_rss,
            refresh_skills,
            list_libraries,
            clear_cache,
            wipe_all_data,
        ])
        .setup(|app| {
            // Spawn bridge server task (with AppHandle for updater)
            let bridge_relay_state = relay_state.clone();
            let bridge_token = session_token.clone();
            let bridge_port = config.bridge_port;
            let bridge_origin = Some(config.skills_web_url.clone());
            let app_handle = app.handle().clone();
            #[cfg(feature = "custom-protocol")]
            {
                let bridge_relay_state = relay_state.clone();
                let bridge_token = session_token.clone();
                let bridge_port = config.bridge_port;
                let bridge_origin = Some(config.skills_web_url.clone());
                let app_handle = app.handle().clone();
                tokio::spawn(async move {
                    if let Err(e) = bridge::start_bridge_with_app(
                        bridge_relay_state,
                        bridge_token,
                        bridge_port,
                        bridge_origin,
                        Some(app_handle),
                    )
                    .await
                    {
                        tracing::error!("Bridge server error: {}", e);
                    }
                });
            }
            #[cfg(not(feature = "custom-protocol"))]
            {
                let bridge_relay_state = relay_state.clone();
                let bridge_token = session_token.clone();
                let bridge_port = config.bridge_port;
                let bridge_origin = Some(config.skills_web_url.clone());
                tokio::spawn(async move {
                    if let Err(e) = bridge::start_bridge(
                        bridge_relay_state,
                        bridge_token,
                        bridge_port,
                        bridge_origin,
                    )
                    .await
                    {
                        tracing::error!("Bridge server error: {}", e);
                    }
                });
            }

            // Set window URL from config (or env var for dev)
            let web_url = std::env::var("TAURI_DEV_WEB")
                .unwrap_or_else(|_| {
                    config::Config::load()
                        .map(|c| c.skills_web_url)
                        .unwrap_or_else(|_| "https://skills.runwaize.com".to_string())
                });
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.navigate(tauri::Url::parse(&web_url).unwrap_or_else(|_| {
                    tauri::Url::parse("https://skills.runwaize.com").unwrap()
                })) {
                    tracing::warn!("Failed to navigate window: {}", e);
                }
            }

            // Set up system tray
            #[cfg(desktop)]
            {
                use tauri::tray::{MouseButton, TrayIconBuilder};

                let _tray = TrayIconBuilder::new()
                    .tooltip("Skill Cookbook Relay")
                    .on_tray_icon_event(|tray, event| {
                        if let tauri::tray::TrayIconEvent::Click {
                            button: MouseButton::Left,
                            ..
                        } = event
                        {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
