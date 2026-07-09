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
mod github_import;
mod guest_manifest;
mod mcp;
mod relay;
mod rss_client;
mod security_review;
mod skill_db;
mod skill_ops;
mod studio_client;
mod types;
mod usage;
mod variables;

use anyhow::Result;
use config::{UiMode, UserRole};
use relay::RelayState;
use std::path::Path;
use std::sync::Arc;
use tauri::{Manager, WebviewUrl};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Fixed 16-byte identifier for WKWebView data store (macOS 14+). Derived from app identifier
/// so the same localStorage/cookies are used across app restarts.
const WEBVIEW_DATA_STORE_ID: [u8; 16] = [
    45, 145, 110, 134, 59, 140, 197, 89, 18, 10, 155, 53, 78, 219, 254, 84,
];

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
                url: url.replace("127.0.0.1", "localhost"),
            },
            _ => Self {
                label: "Live".to_string(),
                url: config.skills_web_url.clone(),
            },
        }
    }
}

// ===== Existing commands =====

#[tauri::command]
async fn pick_folder(
    app: tauri::AppHandle,
    title: Option<String>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(title.as_deref().unwrap_or("Select folder"))
        .pick_folder(move |folder| {
            let _ = tx.send(folder.map(|p| p.to_string()));
        });
    rx.await.map_err(|e| format!("dialog error: {}", e))
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

// ===== New commands for Local UI =====

#[tauri::command]
async fn get_config(state: tauri::State<'_, Arc<RelayState>>) -> Result<types::AppConfig, String> {
    let cfg = state.config.read();
    let settings_dir = config::Config::base_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(types::AppConfig {
        user_role: cfg.user_role.clone(),
        ui_mode: cfg.ui_mode.clone(),
        settings_dir,
        chef_dir: cfg.chef_dir.to_string_lossy().to_string(),
        cook_dir: cfg.guest_dir.to_string_lossy().to_string(),
        workspace_id: cfg.workspace_id.clone(),
        mcp_server_port: cfg.mcp_server_port,
        bridge_port: cfg.bridge_port,
        managed_client_ids: cfg.managed_client_ids.clone(),
        default_review_cli: cfg.default_review_cli.clone(),
    })
}

/// List chef skill names from `chef_dir/skills` (directory names, skipping
/// dotfiles). Shared by `get_skill_usage` and `resync_skill_usage` so the
/// listing logic isn't duplicated between the two commands.
fn list_chef_skill_names(state: &RelayState) -> Result<Vec<String>, String> {
    let cfg = state.config.read();
    let chef_skills_dir = cfg.chef_dir.join("skills");
    drop(cfg);

    let mut names = Vec::new();
    if chef_skills_dir.exists() {
        let entries = std::fs::read_dir(&chef_skills_dir).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            names.push(name);
        }
    }
    Ok(names)
}

#[tauri::command]
async fn get_skill_usage(
    state: tauri::State<'_, Arc<RelayState>>,
    days: u32,
) -> Result<Vec<usage::UsageEvent>, String> {
    let names = list_chef_skill_names(&state)?;
    let since = chrono::Utc::now() - chrono::Duration::days(days as i64);
    usage::synced_events(&state.skill_db, &names, since).map_err(|e| e.to_string())
}

#[tauri::command]
async fn resync_skill_usage(
    state: tauri::State<'_, Arc<RelayState>>,
    days: u32,
) -> Result<Vec<usage::UsageEvent>, String> {
    let names = list_chef_skill_names(&state)?;
    let since = chrono::Utc::now() - chrono::Duration::days(days as i64);
    usage::force_resync(&state.skill_db, &names, since).map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_config(
    state: tauri::State<'_, Arc<RelayState>>,
    chef_dir: Option<String>,
    cook_dir: Option<String>,
    workspace_id: Option<String>,
    mcp_server_port: Option<u16>,
    bridge_port: Option<u16>,
) -> Result<types::AppConfig, String> {
    let mut cfg = state.config.write();
    if let Some(ref cd) = chef_dir {
        cfg.chef_dir = std::path::PathBuf::from(cd);
    }
    if let Some(ref gd) = cook_dir {
        cfg.guest_dir = std::path::PathBuf::from(gd);
    }
    if let Some(wid) = workspace_id {
        cfg.workspace_id = if wid.is_empty() { None } else { Some(wid) };
    }
    if let Some(port) = mcp_server_port {
        cfg.mcp_server_port = port;
    }
    if let Some(port) = bridge_port {
        cfg.bridge_port = port;
    }
    cfg.save().map_err(|e| e.to_string())?;
    let settings_dir = config::Config::base_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let result = types::AppConfig {
        user_role: cfg.user_role.clone(),
        ui_mode: cfg.ui_mode.clone(),
        settings_dir,
        chef_dir: cfg.chef_dir.to_string_lossy().to_string(),
        cook_dir: cfg.guest_dir.to_string_lossy().to_string(),
        workspace_id: cfg.workspace_id.clone(),
        mcp_server_port: cfg.mcp_server_port,
        bridge_port: cfg.bridge_port,
        managed_client_ids: cfg.managed_client_ids.clone(),
        default_review_cli: cfg.default_review_cli.clone(),
    };
    Ok(result)
}

#[tauri::command]
async fn move_settings_dir(
    state: tauri::State<'_, Arc<RelayState>>,
    new_dir: String,
) -> Result<types::AppConfig, String> {
    let old_dir = config::Config::base_dir().map_err(|e| e.to_string())?;
    let new_path = std::path::PathBuf::from(&new_dir);

    if old_dir == new_path {
        // No change needed — return current config
        let cfg = state.config.read();
        let settings_dir = old_dir.to_string_lossy().to_string();
        return Ok(types::AppConfig {
            user_role: cfg.user_role.clone(),
            ui_mode: cfg.ui_mode.clone(),
            settings_dir,
            chef_dir: cfg.chef_dir.to_string_lossy().to_string(),
            cook_dir: cfg.guest_dir.to_string_lossy().to_string(),
            workspace_id: cfg.workspace_id.clone(),
            mcp_server_port: cfg.mcp_server_port,
            bridge_port: cfg.bridge_port,
            managed_client_ids: cfg.managed_client_ids.clone(),
            default_review_cli: cfg.default_review_cli.clone(),
        });
    }

    // Create new dir and copy contents
    std::fs::create_dir_all(&new_path).map_err(|e| format!("create new settings dir: {}", e))?;
    copy_dir_recursive(&old_dir, &new_path)?;

    // If chef/cook dirs were inside the old settings dir, update them to new location
    {
        let mut cfg = state.config.write();
        if cfg.chef_dir.starts_with(&old_dir) {
            if let Ok(rel) = cfg.chef_dir.strip_prefix(&old_dir) {
                cfg.chef_dir = new_path.join(rel);
            }
        }
        if cfg.guest_dir.starts_with(&old_dir) {
            if let Ok(rel) = cfg.guest_dir.strip_prefix(&old_dir) {
                cfg.guest_dir = new_path.join(rel);
            }
        }
        // Save config to the NEW location (it was already copied, now overwrite with updated paths)
        let config_path = new_path.join("config.toml");
        let content =
            toml::to_string_pretty(&*cfg).map_err(|e| format!("serialize config: {}", e))?;
        std::fs::write(&config_path, content)
            .map_err(|e| format!("write config to new dir: {}", e))?;
    }

    // Write redirect file at default location so next startup finds the new dir
    let default_dir = config::Config::default_base_dir();
    std::fs::create_dir_all(&default_dir)
        .map_err(|e| format!("create default dir for redirect: {}", e))?;
    std::fs::write(default_dir.join("base_dir_redirect"), new_dir.as_bytes())
        .map_err(|e| format!("write redirect: {}", e))?;

    let cfg = state.config.read();
    Ok(types::AppConfig {
        user_role: cfg.user_role.clone(),
        ui_mode: cfg.ui_mode.clone(),
        settings_dir: new_dir,
        chef_dir: cfg.chef_dir.to_string_lossy().to_string(),
        cook_dir: cfg.guest_dir.to_string_lossy().to_string(),
        workspace_id: cfg.workspace_id.clone(),
        mcp_server_port: cfg.mcp_server_port,
        bridge_port: cfg.bridge_port,
        managed_client_ids: cfg.managed_client_ids.clone(),
        default_review_cli: cfg.default_review_cli.clone(),
    })
}

#[tauri::command]
async fn set_user_role(
    state: tauri::State<'_, Arc<RelayState>>,
    role: UserRole,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        cfg.user_role = role;
        cfg.save().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn set_managed_clients(
    state: tauri::State<'_, Arc<RelayState>>,
    ids: Vec<String>,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        cfg.managed_client_ids = ids;
        cfg.save().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn set_default_review_cli(
    state: tauri::State<'_, Arc<RelayState>>,
    cli: Option<String>,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        cfg.default_review_cli = cli;
        cfg.save().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn switch_ui_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<RelayState>>,
    mode: UiMode,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        cfg.ui_mode = mode.clone();
        cfg.save().map_err(|e| e.to_string())?;
    }

    if let Some(window) = app.get_webview_window("main") {
        match mode {
            UiMode::Online => {
                let cfg = state.config.read();
                let url = format!(
                    "{}?tauri_version={}",
                    cfg.skills_web_url,
                    env!("CARGO_PKG_VERSION")
                );
                let parsed = tauri::Url::parse(&url).map_err(|e| e.to_string())?;
                window.navigate(parsed).map_err(|e| e.to_string())?;
            }
            UiMode::Local => {
                // Navigate back to local UI
                let parsed = tauri::Url::parse("tauri://localhost").map_err(|e| e.to_string())?;
                window.navigate(parsed).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn list_local_skills(
    state: tauri::State<'_, Arc<RelayState>>,
) -> Result<Vec<types::LocalSkill>, String> {
    let cfg = state.config.read();
    let chef_dir = cfg.chef_dir.clone();
    drop(cfg);
    list_local_skills_impl(&chef_dir, &state.skill_db).map_err(|e| e.to_string())
}

fn list_local_skills_impl(
    chef_dir: &Path,
    skill_db: &skill_db::SkillDb,
) -> std::result::Result<Vec<types::LocalSkill>, String> {
    let chef_skills_dir = chef_dir.join("skills");
    let mut result = Vec::new();
    for (dir, active) in [
        (chef_skills_dir.clone(), true),
        (chef_dir.join("skills-inactive"), false),
    ] {
        if !dir.exists() {
            continue;
        }
        let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let entry_path = entry.path();
            let has_skill_md = entry_path.join("SKILL.md").exists();
            let path = entry_path.to_string_lossy().to_string();
            let targets = skill_ops::deployed_targets(&entry_path, &chef_skills_dir);

            // Join with SQLite metadata (auto-create if missing)
            let mut meta = skill_db.ensure_exists(&name).map_err(|e| e.to_string())?;

            // Self-healing backfill: skills added before description-on-import existed
            // (or added via the CLI/adopt flow, which also never set it) would otherwise
            // show "No description yet." forever. Read it from SKILL.md once and persist
            // it, same as a fresh import would have.
            if meta.description.as_deref().unwrap_or("").is_empty() && has_skill_md {
                if let Some(desc) = skill_description_from_md(&entry_path.join("SKILL.md")) {
                    skill_db
                        .set_description(&name, &desc)
                        .map_err(|e| e.to_string())?;
                    meta.description = Some(desc);
                }
            }

            // "Updated" is sourced from the folder's real filesystem mtime, not the
            // DB-tracked timestamp: the DB only gets bumped on specific app actions
            // (metadata edits), so it goes stale the moment a skill's files are
            // touched any other way (the built-in file editor, an external editor,
            // git). The folder's actual mtime can't go stale -- it's ground truth.
            let updated_at = skill_ops::folder_last_modified(&entry_path)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| meta.updated_at.clone());

            result.push(types::LocalSkill {
                name,
                has_skill_md,
                path,
                status: meta.status,
                version: meta.version,
                tags: meta.tags,
                description: meta.description,
                active,
                targets,
                source: meta.source,
                created_at: meta.created_at,
                updated_at,
            });
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

#[tauri::command]
async fn list_remote_skills(
    state: tauri::State<'_, Arc<RelayState>>,
) -> Result<Vec<types::RemoteSkillInfo>, String> {
    let libraries = state.list_libraries().await.map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for lib in &libraries {
        let skills = state
            .list_skills(Some(&lib.library_id))
            .await
            .unwrap_or_default();
        for s in skills {
            result.push(types::RemoteSkillInfo {
                library_name: lib.name.clone(),
                library_id: lib.library_id.clone(),
                skill_name: s.name,
                skill_id: s.skill_id,
                description: s.description,
            });
        }
    }
    Ok(result)
}

/// Read the `description:`/`title:` frontmatter field straight out of a just-copied
/// SKILL.md so newly imported skills aren't left with a blank description until the
/// user manually fills one in via the metadata editor.
fn skill_description_from_md(skill_md_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(skill_md_path).ok()?;
    let (meta, _) = discovery::extract_frontmatter(&content);
    meta.get("description")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[tauri::command]
async fn add_skill(state: tauri::State<'_, Arc<RelayState>>, path: String) -> Result<(), String> {
    let cfg = state.config.read();
    let skill_md_path = discovery::resolve_skill_md_path(&path).map_err(|e| e.to_string())?;
    let skill_name = skill_md_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("skill")
        .to_string();
    let dest_dir = cfg.chef_dir.join("skills").join(&skill_name);
    let src_dir = skill_md_path
        .parent()
        .ok_or_else(|| format!("Invalid skill path: {}", skill_md_path.display()))?;
    skill_ops::copy_dir_all(src_dir, &dest_dir).map_err(|e| e.to_string())?;
    state
        .skill_db
        .set_source(&skill_name, "adapted")
        .map_err(|e| e.to_string())?;
    if let Some(desc) = skill_description_from_md(&dest_dir.join("SKILL.md")) {
        state
            .skill_db
            .set_description(&skill_name, &desc)
            .map_err(|e| e.to_string())?;
    }
    git_add_commit_impl(&cfg.chef_dir, &format!("Add skill: {}", skill_name))?;
    Ok(())
}

#[tauri::command]
async fn activate_skill(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
) -> Result<(), String> {
    let chef_dir = state.config.read().chef_dir.clone();
    skill_ops::activate_skill(&chef_dir, &name)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn deactivate_skill(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
) -> Result<(), String> {
    let chef_dir = state.config.read().chef_dir.clone();
    skill_ops::deactivate_skill(&chef_dir, &name)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_skill(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
) -> Result<(), String> {
    let chef_dir = state.config.read().chef_dir.clone();
    skill_ops::delete_skill(&chef_dir, &name).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_skill_targets(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
    targets: Vec<String>,
) -> Result<(), String> {
    let chef_dir = state.config.read().chef_dir.clone();
    let chef_skills_dir = chef_dir.join("skills");
    let active_dir = chef_skills_dir.join(&name);
    let skill_dir = if active_dir.exists() {
        active_dir
    } else {
        let inactive_dir = chef_dir.join("skills-inactive").join(&name);
        if inactive_dir.exists() {
            inactive_dir
        } else {
            return Err(format!("Skill '{}' not found", name));
        }
    };
    skill_ops::set_skill_targets(&skill_dir, &chef_skills_dir, &targets).map_err(|e| e.to_string())
}

/// Copy a skill into a target project repo (`.claude/skills/` and/or `.agents/skills/`)
/// as a real, portable file tree so cloud-hosted Claude Code / Codex sessions (which
/// only ever see what's checked into the repo they clone) can discover it.
#[tauri::command]
async fn publish_skill_to_project(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
    project_dir: String,
    targets: Vec<String>,
) -> Result<Vec<String>, String> {
    let chef_dir = state.config.read().chef_dir.clone();
    let active_dir = chef_dir.join("skills").join(&name);
    let skill_dir = if active_dir.exists() {
        active_dir
    } else {
        let inactive_dir = chef_dir.join("skills-inactive").join(&name);
        if inactive_dir.exists() {
            inactive_dir
        } else {
            return Err(format!("Skill '{}' not found", name));
        }
    };
    skill_ops::publish_skill_to_project(&skill_dir, Path::new(&project_dir), &targets)
        .map_err(|e| e.to_string())
}

/// List every skill currently published into a project repo (`.claude/skills/` and/or
/// `.agents/skills/`), with which target(s) each is present in.
#[tauri::command]
async fn list_project_skills(
    project_dir: String,
) -> Result<Vec<skill_ops::ProjectSkillEntry>, String> {
    skill_ops::list_project_skills(Path::new(&project_dir)).map_err(|e| e.to_string())
}

/// Remove a skill previously published into a project repo for the given target(s).
#[tauri::command]
async fn unpublish_skill_from_project(
    project_dir: String,
    name: String,
    targets: Vec<String>,
) -> Result<Vec<String>, String> {
    skill_ops::unpublish_skill_from_project(Path::new(&project_dir), &name, &targets)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn adopt_scan(client_id: String) -> Result<Vec<skill_ops::AdoptCandidate>, String> {
    skill_ops::adopt_scan(&client_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn adopt_apply(
    state: tauri::State<'_, Arc<RelayState>>,
    client_id: String,
    names: Vec<String>,
) -> Result<usize, String> {
    let chef_dir = state.config.read().chef_dir.clone();
    let adopted =
        skill_ops::adopt_apply(&chef_dir, &client_id, &names).map_err(|e| e.to_string())?;
    for name in &adopted {
        state
            .skill_db
            .set_source(name, "downloaded")
            .map_err(|e| e.to_string())?;
    }
    Ok(adopted.len())
}

#[tauri::command]
async fn read_skill_content(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
) -> Result<String, String> {
    let cfg = state.config.read();
    let path = cfg.chef_dir.join("skills").join(&name).join("SKILL.md");
    std::fs::read_to_string(&path).map_err(|e| format!("read {}: {}", path.display(), e))
}

#[tauri::command]
async fn write_skill_content(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
    content: String,
) -> Result<(), String> {
    let cfg = state.config.read();
    let path = cfg.chef_dir.join("skills").join(&name).join("SKILL.md");
    std::fs::write(&path, content).map_err(|e| format!("write {}: {}", path.display(), e))
}

#[tauri::command]
async fn open_skill_in_editor(
    name: String,
    state: tauri::State<'_, Arc<RelayState>>,
) -> Result<(), String> {
    let cfg = state.config.read();
    let path = cfg.chef_dir.join("skills").join(&name).join("SKILL.md");
    drop(cfg);

    let path_str = path.to_string_lossy().to_string();
    if let Ok(editor) = std::env::var("EDITOR") {
        std::process::Command::new(&editor)
            .arg(&path)
            .spawn()
            .map_err(|e| format!("failed to launch {}: {}", editor, e))?;
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .args(["-t", &path_str])
            .spawn()
            .map_err(|e| format!("open: {}", e))?;
    } else if cfg!(target_os = "linux") {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("xdg-open: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
async fn sync_chef(
    state: tauri::State<'_, Arc<RelayState>>,
    no_push: bool,
) -> Result<types::SyncResult, String> {
    let cfg = state.config.read().clone();
    let committed = git_has_changes_impl(&cfg.chef_dir)?;
    if committed {
        git_add_commit_impl(&cfg.chef_dir, "Sync chef")?;
    }

    let skills_pushed = 0;
    let pushed = !no_push;

    Ok(types::SyncResult {
        committed,
        pushed,
        skills_pushed,
        guest_skills_updated: 0,
    })
}

#[tauri::command]
async fn discover_agents() -> Result<Vec<discovery::DiscoveredClient>, String> {
    let resp = discovery::discover_apps_impl();
    Ok(resp.clients)
}

#[tauri::command]
async fn scan_for_skills(
    state: tauri::State<'_, Arc<RelayState>>,
    path: Option<String>,
) -> Result<Vec<types::ScannedSkillInfo>, String> {
    let cfg = state.config.read();
    let existing = existing_skill_names(&cfg.chef_dir.join("skills"));

    let skills = if let Some(ref p) = path {
        let dir = Path::new(p);
        if !dir.is_dir() {
            return Err(format!("Not a directory: {}", p));
        }
        discovery::scan_path_impl(dir, "custom", 0).map_err(|e| e.to_string())?
    } else {
        // Scan all detected agent folders
        let apps = discovery::discover_apps_impl();
        let mut all = Vec::new();
        for client in apps.clients.iter().filter(|c| c.detected) {
            if let Some(ref p) = client.path {
                if let Ok(skills) = discovery::scan_path_impl(Path::new(p), &client.id, 0) {
                    all.extend(skills);
                }
            }
        }
        all
    };

    // Dedup by name
    let mut seen = std::collections::HashSet::new();
    let skills: Vec<_> = skills
        .into_iter()
        .filter(|s| seen.insert(s.name.clone()))
        .collect();

    Ok(skills
        .into_iter()
        .map(|s| types::ScannedSkillInfo {
            name: s.name.clone(),
            path: s.path.clone(),
            already_in_chef: existing.contains(&s.name),
            source_agent: s.client_id,
            display_path: None,
        })
        .collect())
}

/// Import scanned skills (from either the local-folder Discover flow or a GitHub
/// repo scan) into `chef_dir/skills`, tagging each with a provenance string in
/// `skill_db`. `source` is a fully-formed tag (e.g. `"github:owner/repo"`); when not
/// given, imports fall back to the generic `"discovered"` tag so they're still
/// distinguishable in the skill list from skills added other ways (`"adapted"` via
/// `add_skill`, `"downloaded"` via `adopt_apply`, etc.) rather than being left
/// untagged.
#[tauri::command]
async fn import_skills(
    state: tauri::State<'_, Arc<RelayState>>,
    paths: Vec<String>,
    names: Option<Vec<String>>,
    source: Option<String>,
) -> Result<usize, String> {
    let cfg = state.config.read();
    let chef_skills_dir = cfg.chef_dir.join("skills");
    let mut added = Vec::new();
    for (i, path_str) in paths.iter().enumerate() {
        let skill_md = Path::new(path_str);
        let src_dir = match skill_md.parent() {
            Some(d) => d,
            None => continue,
        };
        // Prefer the caller-supplied name (already derived from SKILL.md frontmatter
        // during scan, e.g. via derive_skill_name) over the source directory's
        // basename -- for a repo-root SKILL.md cloned into a tempdir, src_dir IS the
        // tempdir, so its basename is a random string, not the skill's real name.
        // Still sanitized here since the name may originate from an untrusted repo.
        let fallback_name = src_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("skill")
            .to_string();
        let name = names
            .as_ref()
            .and_then(|n| n.get(i))
            .and_then(|s| skill_ops::sanitize_skill_name(s))
            .unwrap_or(fallback_name);
        let dest_dir = chef_skills_dir.join(&name);
        copy_dir_recursive(src_dir, &dest_dir).map_err(|e| e.to_string())?;
        added.push(name);
    }
    if !added.is_empty() {
        let msg = match added.len() {
            1 => format!("Add skill: {}", added[0]),
            n => format!("Add {} skills", n),
        };
        git_add_commit_impl(&cfg.chef_dir, &msg)?;
        let tag = source.as_deref().unwrap_or("discovered");
        for name in &added {
            state
                .skill_db
                .set_source(name, tag)
                .map_err(|e| e.to_string())?;
            if let Some(desc) =
                skill_description_from_md(&chef_skills_dir.join(name).join("SKILL.md"))
            {
                state
                    .skill_db
                    .set_description(name, &desc)
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(added.len())
}

/// Derive the `owner/repo` slug from a GitHub-style URL (same URL passed to
/// `scan_github_repo`), for building the `"github:owner/repo"` provenance tag passed
/// to `import_skills`'s `source` param. Returns `None` if the URL doesn't parse.
#[tauri::command]
async fn github_repo_slug(url: String) -> Result<Option<String>, String> {
    Ok(github_import::repo_slug_from_url(&url))
}

/// Shallow-clone a GitHub (or other git-hosted) repo URL and scan it for SKILL.md
/// files, reusing the same discovery logic as the local-folder scan. Returned paths
/// point into a tempdir that is intentionally left in place (see
/// `github_import::clone_and_scan_repo`) so they stay valid for a following
/// `import_skills` call -- use `github_repo_slug(url)` to build the
/// `source: "github:owner/repo"` tag for that call.
#[tauri::command]
async fn scan_github_repo(
    state: tauri::State<'_, Arc<RelayState>>,
    url: String,
) -> Result<Vec<types::ScannedSkillInfo>, String> {
    let chef_skills_dir = state.config.read().chef_dir.join("skills");
    tokio::task::spawn_blocking(move || github_import::clone_and_scan_repo(&url, &chef_skills_dir))
        .await
        .map_err(|e| format!("scan_github_repo task failed: {}", e))?
        .map_err(|e| e.to_string())
}

/// Run a non-interactive AI security review of a skill using the named CLI
/// (`"claude_code"` or `"codex"`) before it's imported/trusted. See
/// `security_review::run_security_review` for the exact prompt and CLI invocation.
///
/// `skill_path` accepts either a skill's SKILL.md file path (what `scan_for_skills` /
/// `scan_github_repo` produce, same as `import_skills` accepts) or the skill
/// directory itself -- `run_security_review` resolves it via
/// `security_review::resolve_skill_dir`, matching how `import_skills` derives the
/// skill dir from the same scanned path.
#[tauri::command]
async fn run_skill_security_review(
    skill_path: String,
    cli: String,
) -> Result<security_review::SecurityReview, String> {
    tokio::task::spawn_blocking(move || {
        security_review::run_security_review(&cli, Path::new(&skill_path))
    })
    .await
    .map_err(|e| format!("run_skill_security_review task failed: {}", e))?
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_guest_manifest(
    state: tauri::State<'_, Arc<RelayState>>,
) -> Result<guest_manifest::GuestManifest, String> {
    let cfg = state.config.read();
    guest_manifest::read_guest_manifest(&cfg.guest_dir).map_err(|e| e.to_string())
}

#[tauri::command]
async fn sync_guest(state: tauri::State<'_, Arc<RelayState>>) -> Result<usize, String> {
    // Sync guest manifest from server
    let cfg = state.config.read().clone();
    let libraries = state.list_libraries().await.map_err(|e| e.to_string())?;
    let mut skills = Vec::new();
    for lib in &libraries {
        let lib_skills = state
            .list_skills(Some(&lib.library_id))
            .await
            .unwrap_or_default();
        for s in lib_skills {
            skills.push(guest_manifest::GuestSkillEntry {
                skill_id: s.skill_id,
                name: s.name,
                description: s.description,
                library_id: Some(lib.library_id.clone()),
            });
        }
    }
    let count = skills.len();
    let manifest = guest_manifest::GuestManifest { skills };
    guest_manifest::write_guest_manifest(&cfg.guest_dir, &manifest).map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
async fn run_doctor(
    state: tauri::State<'_, Arc<RelayState>>,
) -> Result<Vec<types::DiagnosticCheck>, String> {
    use types::{DiagnosticCheck, DiagnosticStatus};
    let cfg = state.config.read();
    let mut checks = Vec::new();

    // Config
    checks.push(DiagnosticCheck {
        name: "Config".into(),
        status: DiagnosticStatus::Ok,
        message: "Loaded successfully".into(),
    });

    // Chef dir
    if cfg.chef_dir.exists() && cfg.chef_dir.is_dir() {
        checks.push(DiagnosticCheck {
            name: "Chef directory".into(),
            status: DiagnosticStatus::Ok,
            message: cfg.chef_dir.to_string_lossy().into(),
        });
    } else {
        checks.push(DiagnosticCheck {
            name: "Chef directory".into(),
            status: DiagnosticStatus::Error,
            message: format!("Missing: {}", cfg.chef_dir.display()),
        });
    }

    // Guest dir
    if cfg.guest_dir.exists() {
        checks.push(DiagnosticCheck {
            name: "Guest directory".into(),
            status: DiagnosticStatus::Ok,
            message: cfg.guest_dir.to_string_lossy().into(),
        });
    } else {
        checks.push(DiagnosticCheck {
            name: "Guest directory".into(),
            status: DiagnosticStatus::Warning,
            message: format!("Missing: {}", cfg.guest_dir.display()),
        });
    }

    // Git repo
    if cfg.chef_dir.join(".git").exists() {
        checks.push(DiagnosticCheck {
            name: "Chef git repo".into(),
            status: DiagnosticStatus::Ok,
            message: "Initialized".into(),
        });
    } else {
        checks.push(DiagnosticCheck {
            name: "Chef git repo".into(),
            status: DiagnosticStatus::Error,
            message: "Not a git repo (run init)".into(),
        });
    }

    // Workspace
    checks.push(DiagnosticCheck {
        name: "Workspace ID".into(),
        status: if cfg.workspace_id.is_some() {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Warning
        },
        message: cfg.workspace_id.as_deref().unwrap_or("Not set").into(),
    });

    // Auth
    let authenticated = state.is_authenticated();
    checks.push(DiagnosticCheck {
        name: "Authentication".into(),
        status: if authenticated {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Warning
        },
        message: if authenticated {
            "Logged in".into()
        } else {
            "Not authenticated".into()
        },
    });

    // MCP
    checks.push(DiagnosticCheck {
        name: "MCP Server".into(),
        status: DiagnosticStatus::Ok,
        message: format!("Port {}", cfg.mcp_server_port),
    });

    Ok(checks)
}

#[tauri::command]
async fn init_cookbook(
    state: tauri::State<'_, Arc<RelayState>>,
    chef_dir: Option<String>,
    guest_dir: Option<String>,
) -> Result<(), String> {
    let mut cfg = state.config.write();

    if let Some(ref cd) = chef_dir {
        cfg.chef_dir = std::path::PathBuf::from(cd);
    }
    if let Some(ref gd) = guest_dir {
        cfg.guest_dir = std::path::PathBuf::from(gd);
    }

    std::fs::create_dir_all(&cfg.chef_dir).map_err(|e| format!("create chef dir: {}", e))?;
    std::fs::create_dir_all(&cfg.guest_dir).map_err(|e| format!("create guest dir: {}", e))?;

    // Init git in chef dir if needed
    if !cfg.chef_dir.join(".git").exists() {
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&cfg.chef_dir)
            .status()
            .map_err(|e| format!("git init: {}", e))?;
    }

    cfg.save().map_err(|e| e.to_string())?;
    Ok(())
}

// ===== Skill file tree & metadata commands =====

fn validate_relative_path(rel: &str) -> std::result::Result<(), String> {
    if rel.contains("..") {
        return Err("Path must not contain '..'".to_string());
    }
    if rel.starts_with('/') || rel.starts_with('\\') {
        return Err("Path must be relative".to_string());
    }
    Ok(())
}

#[tauri::command]
async fn list_skill_files(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
) -> Result<Vec<types::SkillFileEntry>, String> {
    let cfg = state.config.read();
    let skill_dir = cfg.chef_dir.join("skills").join(&name);
    drop(cfg);

    if !skill_dir.is_dir() {
        return Err(format!("Skill directory not found: {}", name));
    }

    fn walk(base: &Path, prefix: &str) -> std::result::Result<Vec<types::SkillFileEntry>, String> {
        let mut entries = Vec::new();
        let read = std::fs::read_dir(base).map_err(|e| e.to_string())?;
        for entry in read {
            let entry = entry.map_err(|e| e.to_string())?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with('.') {
                continue;
            }
            let rel = if prefix.is_empty() {
                file_name.clone()
            } else {
                format!("{}/{}", prefix, file_name)
            };
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            let is_dir = meta.is_dir();
            let size = if is_dir { 0 } else { meta.len() };
            let extension = if is_dir {
                None
            } else {
                Path::new(&file_name)
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
            };
            entries.push(types::SkillFileEntry {
                relative_path: rel.clone(),
                name: file_name,
                is_dir,
                size,
                extension,
            });
            if is_dir {
                entries.extend(walk(&entry.path(), &rel)?);
            }
        }
        Ok(entries)
    }

    walk(&skill_dir, "")
}

#[tauri::command]
async fn read_skill_file(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
    relative_path: String,
) -> Result<String, String> {
    validate_relative_path(&relative_path)?;
    let cfg = state.config.read();
    let path = cfg.chef_dir.join("skills").join(&name).join(&relative_path);
    std::fs::read_to_string(&path).map_err(|e| format!("read {}: {}", path.display(), e))
}

#[tauri::command]
async fn write_skill_file(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
    relative_path: String,
    content: String,
) -> Result<(), String> {
    validate_relative_path(&relative_path)?;
    let cfg = state.config.read();
    let path = cfg.chef_dir.join("skills").join(&name).join(&relative_path);
    std::fs::write(&path, content).map_err(|e| format!("write {}: {}", path.display(), e))
}

#[tauri::command]
async fn create_skill_file(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
    relative_path: String,
    content: String,
) -> Result<(), String> {
    validate_relative_path(&relative_path)?;
    let cfg = state.config.read();
    let path = cfg.chef_dir.join("skills").join(&name).join(&relative_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create dirs: {}", e))?;
    }
    std::fs::write(&path, content).map_err(|e| format!("create {}: {}", path.display(), e))
}

#[tauri::command]
async fn create_skill_folder(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
    relative_path: String,
) -> Result<(), String> {
    validate_relative_path(&relative_path)?;
    let cfg = state.config.read();
    let path = cfg.chef_dir.join("skills").join(&name).join(&relative_path);
    std::fs::create_dir_all(&path).map_err(|e| format!("create folder: {}", e))
}

#[tauri::command]
async fn delete_skill_file(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
    relative_path: String,
) -> Result<(), String> {
    validate_relative_path(&relative_path)?;
    let cfg = state.config.read();
    let path = cfg.chef_dir.join("skills").join(&name).join(&relative_path);
    if path.is_dir() {
        std::fs::remove_dir(&path).map_err(|e| format!("delete folder: {}", e))
    } else {
        std::fs::remove_file(&path).map_err(|e| format!("delete file: {}", e))
    }
}

#[tauri::command]
async fn rename_skill_file(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    validate_relative_path(&old_path)?;
    validate_relative_path(&new_path)?;
    let cfg = state.config.read();
    let base = cfg.chef_dir.join("skills").join(&name);
    let src = base.join(&old_path);
    let dst = base.join(&new_path);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create dirs: {}", e))?;
    }
    std::fs::rename(&src, &dst).map_err(|e| format!("rename: {}", e))
}

#[tauri::command]
async fn get_skill_meta(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
) -> Result<skill_db::SkillMeta, String> {
    state
        .skill_db
        .ensure_exists(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_skill_meta(
    state: tauri::State<'_, Arc<RelayState>>,
    name: String,
    status: String,
    version: String,
    tags: Vec<String>,
    description: Option<String>,
    source: String,
) -> Result<skill_db::SkillMeta, String> {
    state
        .skill_db
        .upsert(
            &name,
            &status,
            &version,
            &tags,
            description.as_deref(),
            &source,
        )
        .map_err(|e| e.to_string())?;
    state
        .skill_db
        .get(&name)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "skill not found after upsert".to_string())
}

// ===== Helper functions =====

fn git_add_commit_impl(repo_root: &Path, message: &str) -> std::result::Result<(), String> {
    let _ = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo_root)
        .status()
        .map_err(|e| format!("git add: {}", e))?;
    std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo_root)
        .status()
        .map_err(|e| format!("git commit: {}", e))?;
    Ok(())
}

fn git_has_changes_impl(repo_root: &Path) -> std::result::Result<bool, String> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("git status: {}", e))?;
    Ok(!output.stdout.is_empty())
}

fn existing_skill_names(chef_skills_dir: &Path) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    if let Ok(entries) = std::fs::read_dir(chef_skills_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    names.insert(name.to_string());
                }
            }
        }
    }
    names
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::result::Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("create dir {}: {}", dst.display(), e))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("read dir {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        if ft.is_symlink() {
            continue;
        }
        if entry.file_name() == ".git" {
            // GitHub-imported skills are cloned into a tempdir; don't nest that
            // clone's own git repo inside the chef dir's git repo.
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "copy {} -> {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

// ===== Main =====

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
            // Existing
            pick_folder,
            get_relay_status,
            login_to_rss,
            logout_from_rss,
            refresh_skills,
            list_libraries,
            clear_cache,
            wipe_all_data,
            // New for local UI
            get_config,
            update_config,
            move_settings_dir,
            set_user_role,
            set_managed_clients,
            set_default_review_cli,
            get_skill_usage,
            resync_skill_usage,
            switch_ui_mode,
            list_local_skills,
            list_remote_skills,
            add_skill,
            activate_skill,
            deactivate_skill,
            delete_skill,
            set_skill_targets,
            publish_skill_to_project,
            list_project_skills,
            unpublish_skill_from_project,
            adopt_scan,
            adopt_apply,
            read_skill_content,
            write_skill_content,
            open_skill_in_editor,
            sync_chef,
            discover_agents,
            scan_for_skills,
            import_skills,
            github_repo_slug,
            scan_github_repo,
            run_skill_security_review,
            get_guest_manifest,
            sync_guest,
            run_doctor,
            init_cookbook,
            // Skill file tree & metadata
            list_skill_files,
            read_skill_file,
            write_skill_file,
            create_skill_file,
            create_skill_folder,
            delete_skill_file,
            rename_skill_file,
            get_skill_meta,
            update_skill_meta,
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

            // Determine initial URL based on ui_mode. Online mode is only honored at
            // startup if already authenticated — this app is standalone-first, so a
            // saved ui_mode=Online preference (e.g. from a prior explicit switch)
            // must never surface a cold login wall on launch. Users who want the
            // online experience switch to it explicitly, in-session, from Settings.
            let webview_url = if matches!(config.ui_mode, UiMode::Online)
                && relay_state.is_authenticated()
            {
                let base_url = web_source.url.trim_end_matches('/');
                let web_url = format!(
                    "{}/inbox?tauri_version={}",
                    base_url,
                    env!("CARGO_PKG_VERSION"),
                );
                let url = tauri::Url::parse(&web_url)
                    .unwrap_or_else(|_| tauri::Url::parse("https://skills.runwaize.com").unwrap());
                WebviewUrl::External(url)
            } else {
                // In dev, the Vite dev server is running at devUrl; in production,
                // the local-ui/dist is served via the custom protocol.
                WebviewUrl::App("index.html".into())
            };

            let mut builder = tauri::WebviewWindowBuilder::new(app, "main", webview_url)
                .title("Skill Cookbook")
                .inner_size(1200.0, 800.0)
                .resizable(true)
                .decorations(true)
                .visible(true);

            #[cfg(any(target_os = "windows", target_os = "linux"))]
            {
                if let Ok(dir) = config::Config::webview_data_dir() {
                    builder = builder.data_directory(dir);
                }
            }
            #[cfg(target_os = "macos")]
            {
                builder = builder.data_store_identifier(WEBVIEW_DATA_STORE_ID);
            }

            builder.build()?;

            // Set up system tray
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
                    MenuItemBuilder::with_id("about", "About Skill Cookbook").build(app)?;
                let show_item = MenuItemBuilder::with_id("show", "Show Window").build(app)?;
                let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
                let local_ui_item = MenuItemBuilder::with_id("ui_local", "Local UI").build(app)?;
                let online_ui_item =
                    MenuItemBuilder::with_id("ui_online", "Online UI").build(app)?;
                let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

                let menu = MenuBuilder::new(app)
                    .item(&about_item)
                    .item(&PredefinedMenuItem::separator(app)?)
                    .item(&show_item)
                    .item(&settings_item)
                    .item(&PredefinedMenuItem::separator(app)?)
                    .item(&local_ui_item)
                    .item(&online_ui_item)
                    .item(&PredefinedMenuItem::separator(app)?)
                    .item(&quit_item)
                    .build()?;

                let relay_for_tray = relay_state.clone();
                let _tray = TrayIconBuilder::new()
                    .tooltip("Skill Cookbook")
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
                                            .title("About Skill Cookbook")
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
                            "settings" => {
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    // Navigate to settings in local UI
                                    if let Ok(parsed) =
                                        tauri::Url::parse("tauri://localhost/settings")
                                    {
                                        let _ = window.navigate(parsed);
                                    }
                                }
                            }
                            "ui_local" | "ui_online" => {
                                let mode = if event.id().as_ref() == "ui_local" {
                                    UiMode::Local
                                } else {
                                    UiMode::Online
                                };
                                let relay = relay_for_tray.clone();
                                let handle = app_handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    let mut cfg = relay.config.write();
                                    cfg.ui_mode = mode.clone();
                                    let _ = cfg.save();
                                    drop(cfg);

                                    if let Some(window) = handle.get_webview_window("main") {
                                        match mode {
                                            UiMode::Online => {
                                                let cfg = relay.config.read();
                                                let url = format!(
                                                    "{}?tauri_version={}",
                                                    cfg.skills_web_url,
                                                    env!("CARGO_PKG_VERSION")
                                                );
                                                if let Ok(parsed) = tauri::Url::parse(&url) {
                                                    let _ = window.navigate(parsed);
                                                }
                                            }
                                            UiMode::Local => {
                                                if let Ok(parsed) =
                                                    tauri::Url::parse("tauri://localhost")
                                                {
                                                    let _ = window.navigate(parsed);
                                                }
                                            }
                                        }
                                    }
                                });
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
