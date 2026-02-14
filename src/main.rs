// Skill Cookbook Relay - Local MCP server for secure skill delivery
// Prevents instantiation of Windows on macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod bridge;
mod cache;
mod config;
mod crypto;
mod discovery;
mod error;
mod mcp;
mod relay;
mod rss_client;
mod studio_client;
mod types;
mod variables;

use anyhow::Result;
use relay::RelayState;
use std::sync::Arc;
use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Tracks whether the app is using the live or local dev website.
#[derive(Clone)]
struct WebSource {
    label: String,
    url: String,
}

impl WebSource {
    fn resolve(config: &config::Config) -> Self {
        match std::env::var("TAURI_DEV_WEB") {
            Ok(url) if !url.is_empty() => Self {
                label: "Local Dev".to_string(),
                url,
            },
            _ => Self {
                label: "Live".to_string(),
                url: config.skills_web_url.clone(),
            },
        }
    }
}

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

    // Resolve web source (live vs local dev)
    let web_source = WebSource::resolve(&config);
    tracing::info!("Web source: {} ({})", web_source.label, web_source.url);

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
        .manage(relay_state.clone())
        .invoke_handler(tauri::generate_handler![
            get_relay_status,
            login_to_rss,
            logout_from_rss,
            refresh_skills,
            list_libraries,
            clear_cache,
            wipe_all_data,
        ])
        .setup(move |app| {
            let bridge_origin = Some(web_source.url.clone());

            #[cfg(feature = "custom-protocol")]
            {
                let bridge_relay_state = relay_state.clone();
                let bridge_token = session_token.clone();
                let bridge_port = config.bridge_port;
                let bridge_origin = bridge_origin.clone();
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
                let bridge_origin = bridge_origin.clone();
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

            // Navigate window to resolved web source with tauri_version param
            let base_url = &web_source.url;
            let initial_path = relay_state
                .is_authenticated()
                .then(|| "/inbox".to_string())
                .unwrap_or_else(|| "/account/login".to_string());
            let web_url = format!(
                "{}{}?tauri_version={}",
                base_url.trim_end_matches('/'),
                initial_path,
                env!("CARGO_PKG_VERSION"),
            );
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.navigate(tauri::Url::parse(&web_url).unwrap_or_else(|_| {
                    tauri::Url::parse("https://skills.runwaize.com").unwrap()
                })) {
                    tracing::warn!("Failed to navigate window: {}", e);
                }
            }

            // Set up system tray with About menu
            #[cfg(desktop)]
            {
                use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
                use tauri::tray::TrayIconBuilder;

                let about_label = format!(
                    "Skill Cookbook Relay v{}\nWeb: {} ({})",
                    env!("CARGO_PKG_VERSION"),
                    web_source.label,
                    web_source.url,
                );

                let about_item =
                    MenuItemBuilder::with_id("about", "About Skill Cookbook Relay").build(app)?;
                let show_item = MenuItemBuilder::with_id("show", "Show Window").build(app)?;
                let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

                let menu = MenuBuilder::new(app)
                    .item(&about_item)
                    .item(&PredefinedMenuItem::separator(app)?)
                    .item(&show_item)
                    .item(&quit_item)
                    .build()?;

                let _tray = TrayIconBuilder::new()
                    .tooltip("Skill Cookbook Relay")
                    .menu(&menu)
                    .show_menu_on_left_click(true)
                    .on_menu_event(move |app_handle, event| {
                        match event.id().as_ref() {
                            "about" => {
                                let msg = about_label.clone();
                                let handle = app_handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    use tauri_plugin_dialog::DialogExt;
                                    if let Some(window) = handle.get_webview_window("main") {
                                        window
                                            .dialog()
                                            .message(msg)
                                            .title("About Skill Cookbook Relay")
                                            .show(|_| {});
                                    }
                                });
                            }
                            "show" => {
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                            "quit" => {
                                app_handle.exit(0);
                            }
                            _ => {}
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
