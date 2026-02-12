// Skill Cookbook Relay - Local MCP server for secure skill delivery
// Prevents instantiation of Windows on macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
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
    let relay_state = Arc::new(RelayState::new(config).await?);
    tracing::info!("Relay state initialized");

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
