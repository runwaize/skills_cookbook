//! Chef commands: add skill to local repo, sync with server, list skills.

use skill_cookbook_relay::config::Config;
use skill_cookbook_relay::discovery::{build_skill_zip, resolve_skill_md_path, scan_path_impl};
use skill_cookbook_relay::{auth::AuthManager, rss_client::RssClient, studio_client::StudioClient};
use std::path::Path;
use std::sync::Arc;

#[derive(clap::Args, Debug)]
pub struct AddArgs {
    /// Path to SKILL.md or directory containing SKILL.md.
    pub path: std::path::PathBuf,
}

/// Copy skill at path into chef dir and commit.
pub fn run_add(args: &AddArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let path_str = args.path.to_string_lossy();
    let skill_md_path = resolve_skill_md_path(&path_str).map_err(|e| e.to_string())?;

    let skill_name = skill_md_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("skill")
        .to_string();
    let dest_dir = config.chef_dir.join("skills").join(&skill_name);
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("create dest dir: {}", e))?;
    let dest_path = dest_dir.join("SKILL.md");
    let content = std::fs::read_to_string(&skill_md_path).map_err(|e| e.to_string())?;
    std::fs::write(&dest_path, content).map_err(|e| format!("write: {}", e))?;

    git_add_commit(&config.chef_dir, &format!("Add skill: {}", skill_name))?;
    println!("Added skill '{}' at {}", skill_name, dest_path.display());
    Ok(())
}

#[derive(clap::Args, Debug)]
pub struct SyncArgs {
    /// Skip pushing local changes to server (only commit and pull).
    #[arg(long)]
    pub no_push: bool,
}

/// Sync chef: commit local, optionally push to server.
pub fn run_sync_chef(
    args: &SyncArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;

    git_add_commit(&config.chef_dir, "Sync chef")?;

    if !args.no_push {
        push_local_skills_to_server(&config)?;
    }
    Ok(())
}

fn push_local_skills_to_server(config: &Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let skills_dir = config.chef_dir.join("skills");
    if !skills_dir.exists() {
        return Ok(());
    }
    let skills = scan_path_impl(&skills_dir, "chef", 2).map_err(|e| e.to_string())?;
    if skills.is_empty() {
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let auth = Arc::new(AuthManager::new(config.clone()).map_err(|e| e.to_string())?);
        auth.initialize().await.map_err(|e| e.to_string())?;
        if !auth.is_authenticated() {
            println!("Not authenticated; run login first. Skipping push.");
            return Ok(());
        }
        let token = auth.get_access_token().await.map_err(|e| e.to_string())?;
        let studio = StudioClient::new(config.studio_api_url.clone());
        for s in &skills {
            let path = std::path::Path::new(&s.path);
            match build_skill_zip(path) {
                Ok(zip_bytes) => {
                    match studio.upload_skill(&token, zip_bytes).await {
                        Ok(skill_id) => println!("Pushed {} -> {}", s.name, skill_id),
                        Err(e) => eprintln!("Failed to push {}: {}", s.name, e),
                    }
                }
                Err(e) => eprintln!("Failed to build zip for {}: {}", s.path, e),
            }
        }
        Ok(())
    })
}

/// List skills from server (libraries + skills).
pub fn run_list() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let auth = Arc::new(AuthManager::new(config.clone()).map_err(|e| e.to_string())?);
        auth.initialize().await.map_err(|e| e.to_string())?;
        if !auth.is_authenticated() {
            println!("Not authenticated. Run login first.");
            return Ok(());
        }
        let rss = RssClient::new(config.rss_api_url.clone(), auth);
        let libraries = rss.list_libraries().await.map_err(|e| e.to_string())?;
        for lib in &libraries {
            println!("Library: {} ({})", lib.name, lib.library_id);
            let skills = rss.list_skills(&lib.library_id).await.map_err(|e| e.to_string())?;
            for s in skills {
                println!("  - {} ({})", s.name, s.skill_id);
            }
        }
        Ok(())
    })
}

pub(crate) fn git_add_commit(repo_root: &Path, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo_root)
        .status()
        .map_err(|e| format!("git add: {}", e))?;
    let status = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo_root)
        .status()
        .map_err(|e| format!("git commit: {}", e))?;
    if status.code() == Some(0) {
        println!("Committed: {}", message);
    }
    Ok(())
}
