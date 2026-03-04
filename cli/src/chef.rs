//! Chef commands: add skill to local repo, sync with server, list skills, edit.

use skill_cookbook_relay::config::Config;
use skill_cookbook_relay::discovery::{build_skill_zip, resolve_skill_md_path, scan_path_impl};
use skill_cookbook_relay::{auth::AuthManager, rss_client::RssClient, studio_client::StudioClient};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(clap::Args, Debug)]
pub struct AddArgs {
    /// Path to SKILL.md or directory containing SKILL.md.
    pub path: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct ListArgs {
    /// Fetch skills from server instead of listing local chef dir.
    #[arg(long)]
    pub remote: bool,
}

#[derive(clap::Args, Debug)]
pub struct EditArgs {
    /// Skill name to edit. If omitted, shows an interactive picker.
    pub name: Option<String>,
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

/// List local skills, or delegate to server list with --remote.
pub fn run_list_local(args: &ListArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if args.remote {
        return run_list_remote();
    }
    let config = Config::load().map_err(|e| e.to_string())?;
    let skills_dir = config.chef_dir.join("skills");
    let skills = list_local_skills(&skills_dir)?;
    if skills.is_empty() {
        println!("No skills in chef dir ({}).", skills_dir.display());
        return Ok(());
    }
    for (name, has_skill_md) in &skills {
        if *has_skill_md {
            println!("  {}", name);
        } else {
            println!("  {} [missing SKILL.md]", name);
        }
    }
    Ok(())
}

/// Open a skill's SKILL.md in an editor.
pub fn run_edit(args: &EditArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let skills_dir = config.chef_dir.join("skills");
    let skills = list_local_skills(&skills_dir)?;
    if skills.is_empty() {
        return Err("No skills in chef dir.".into());
    }

    let name = match &args.name {
        Some(n) => n.clone(),
        None => {
            let labels: Vec<&str> = skills.iter().map(|(n, _)| n.as_str()).collect();
            let selection = dialoguer::Select::new()
                .with_prompt("Select skill to edit")
                .items(&labels)
                .interact()
                .map_err(|e| format!("prompt error: {}", e))?;
            labels[selection].to_string()
        }
    };

    let skill_md = skills_dir.join(&name).join("SKILL.md");
    if !skill_md.exists() {
        return Err(format!("SKILL.md not found at {}", skill_md.display()).into());
    }
    open_in_editor(&skill_md)
}

/// Structured list of local skills (for Tauri UI).
pub fn list_local_skills_structured(
    config: &Config,
) -> Result<Vec<skill_cookbook_relay::types::LocalSkill>, Box<dyn std::error::Error + Send + Sync>> {
    let skills_dir = config.chef_dir.join("skills");
    let entries = list_local_skills(&skills_dir)?;
    Ok(entries
        .into_iter()
        .map(|(name, has_skill_md)| {
            let path = skills_dir.join(&name).to_string_lossy().to_string();
            skill_cookbook_relay::types::LocalSkill {
                name,
                has_skill_md,
                path,
            }
        })
        .collect())
}

/// Structured list of remote skills (for Tauri UI).
pub fn list_remote_skills_structured(
    config: &Config,
) -> Result<Vec<skill_cookbook_relay::types::RemoteSkillInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let auth = Arc::new(AuthManager::new(config.clone()).map_err(|e| e.to_string())?);
        auth.initialize().await.map_err(|e| e.to_string())?;
        if !auth.is_authenticated() {
            return Err("Not authenticated. Run login first.".into());
        }
        let rss = RssClient::new(config.rss_api_url.clone(), auth);
        let libraries = rss.list_libraries().await.map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for lib in &libraries {
            let skills = rss.list_skills(&lib.library_id).await.map_err(|e| e.to_string())?;
            for s in skills {
                result.push(skill_cookbook_relay::types::RemoteSkillInfo {
                    library_name: lib.name.clone(),
                    library_id: lib.library_id.clone(),
                    skill_name: s.name,
                    skill_id: s.skill_id,
                    description: s.description,
                });
            }
        }
        Ok(result)
    })
}

/// Add a skill by path (for Tauri UI, non-interactive).
pub fn add_skill_to_chef(
    config: &Config,
    path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let skill_md_path = skill_cookbook_relay::discovery::resolve_skill_md_path(path).map_err(|e| e.to_string())?;
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
    Ok(())
}

/// Sync chef and return structured result (for Tauri UI).
pub fn sync_chef_structured(
    config: &Config,
    no_push: bool,
) -> Result<skill_cookbook_relay::types::SyncResult, Box<dyn std::error::Error + Send + Sync>> {
    // Commit
    let committed = git_has_changes(&config.chef_dir)?;
    if committed {
        git_add_commit(&config.chef_dir, "Sync chef")?;
    }

    let mut skills_pushed = 0;
    let pushed = !no_push;
    if pushed {
        skills_pushed = push_local_skills_count(config)?;
    }

    Ok(skill_cookbook_relay::types::SyncResult {
        committed,
        pushed,
        skills_pushed,
        guest_skills_updated: 0,
    })
}

fn git_has_changes(repo_root: &Path) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("git status: {}", e))?;
    Ok(!output.stdout.is_empty())
}

fn push_local_skills_count(config: &Config) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let skills_dir = config.chef_dir.join("skills");
    if !skills_dir.exists() {
        return Ok(0);
    }
    let skills = scan_path_impl(&skills_dir, "chef", 2).map_err(|e| e.to_string())?;
    if skills.is_empty() {
        return Ok(0);
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let auth = Arc::new(AuthManager::new(config.clone()).map_err(|e| e.to_string())?);
        auth.initialize().await.map_err(|e| e.to_string())?;
        if !auth.is_authenticated() {
            return Ok(0usize);
        }
        let token = auth.get_access_token().await.map_err(|e| e.to_string())?;
        let studio = StudioClient::new(config.studio_api_url.clone());
        let mut pushed = 0usize;
        for s in &skills {
            let path = std::path::Path::new(&s.path);
            if let Ok(zip_bytes) = build_skill_zip(path) {
                if studio.upload_skill(&token, zip_bytes).await.is_ok() {
                    pushed += 1;
                }
            }
        }
        Ok(pushed)
    })
}

/// Scan skills_dir for subdirs, returning (name, has_skill_md) sorted alphabetically.
/// Skips hidden directories (names starting with '.').
fn list_local_skills(
    skills_dir: &Path,
) -> Result<Vec<(String, bool)>, Box<dyn std::error::Error + Send + Sync>> {
    if !skills_dir.exists() {
        return Ok(vec![]);
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(skills_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let has_skill_md = entry.path().join("SKILL.md").exists();
        entries.push((name, has_skill_md));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

/// Open a file in the user's preferred editor.
fn open_in_editor(path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path_str = path.to_string_lossy();
    if let Ok(editor) = std::env::var("EDITOR") {
        let status = std::process::Command::new(&editor)
            .arg(path)
            .status()
            .map_err(|e| format!("failed to launch {}: {}", editor, e))?;
        if !status.success() {
            return Err(format!("{} exited with {}", editor, status).into());
        }
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .args(["-t", &path_str])
            .status()
            .map_err(|e| format!("open -t: {}", e))?;
    } else if cfg!(target_os = "linux") {
        std::process::Command::new("xdg-open")
            .arg(path)
            .status()
            .map_err(|e| format!("xdg-open: {}", e))?;
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path_str])
            .status()
            .map_err(|e| format!("cmd start: {}", e))?;
    } else {
        return Err("No $EDITOR set and unsupported platform.".into());
    }
    Ok(())
}

/// List skills from server (libraries + skills).
fn run_list_remote() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn list_local_skills_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let result = list_local_skills(tmp.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_local_skills_populated() {
        let tmp = tempfile::tempdir().unwrap();
        let skills = tmp.path();

        // Skill with SKILL.md
        fs::create_dir(skills.join("alpha")).unwrap();
        fs::write(skills.join("alpha").join("SKILL.md"), "# Alpha").unwrap();

        // Skill without SKILL.md
        fs::create_dir(skills.join("beta")).unwrap();

        // A regular file (should be ignored)
        fs::write(skills.join("not-a-skill.txt"), "").unwrap();

        let result = list_local_skills(skills).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ("alpha".to_string(), true));
        assert_eq!(result[1], ("beta".to_string(), false));
    }

    #[test]
    fn list_local_skills_skips_hidden() {
        let tmp = tempfile::tempdir().unwrap();
        let skills = tmp.path();

        fs::create_dir(skills.join(".git")).unwrap();
        fs::create_dir(skills.join(".hidden")).unwrap();
        fs::create_dir(skills.join("visible")).unwrap();
        fs::write(skills.join("visible").join("SKILL.md"), "").unwrap();

        let result = list_local_skills(skills).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "visible");
    }

    #[test]
    fn list_local_skills_missing_dir() {
        let result = list_local_skills(Path::new("/nonexistent/skills")).unwrap();
        assert!(result.is_empty());
    }
}
