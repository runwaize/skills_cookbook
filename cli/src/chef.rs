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
    let src_dir = skill_md_path
        .parent()
        .ok_or_else(|| format!("Invalid skill path: {}", skill_md_path.display()))?;
    skill_cookbook_relay::skill_ops::copy_dir_all(src_dir, &dest_dir).map_err(|e| e.to_string())?;

    git_add_commit(&config.chef_dir, &format!("Add skill: {}", skill_name))?;
    println!("Added skill '{}' at {}", skill_name, dest_dir.display());
    Ok(())
}

#[derive(clap::Args, Debug)]
pub struct SyncArgs {
    /// Skip pushing local changes to server (only commit and pull).
    #[arg(long)]
    pub no_push: bool,
}

#[derive(clap::Args, Debug)]
pub struct DeployArgs {
    /// Target client id: claude_code, cursor, codex, codeium, windsurf, aider, zed.
    pub target: String,
}

#[derive(clap::Args, Debug)]
pub struct DeactivateArgs {
    /// Skill name (folder name or deploy link name, e.g. "foo" or "group-foo").
    pub skill_name: String,
}

#[derive(clap::Args, Debug)]
pub struct ActivateArgs {
    /// Skill name (folder name under skills-inactive/).
    pub name: String,
}

#[derive(clap::Args, Debug)]
pub struct DeleteArgs {
    /// Skill name (folder name under skills/ or skills-inactive/).
    pub name: String,
}

#[derive(clap::Args, Debug)]
pub struct AdoptArgs {
    /// Client id to scan: claude_code, cursor, codex, codeium, windsurf, aider, zed.
    pub target: String,
    /// Actually move+relink candidates instead of just listing them.
    #[arg(long)]
    pub apply: bool,
}

#[derive(clap::Args, Debug)]
pub struct PublishArgs {
    /// Skill name (folder name or deploy link name, e.g. "foo" or "group-foo").
    pub name: String,
    /// Path to the target project's repo (or any dir inside it).
    pub project_dir: PathBuf,
    /// Publish to <project_dir>/.claude/skills/<name>.
    #[arg(long = "claude-code")]
    pub claude_code: bool,
    /// Publish to <project_dir>/.agents/skills/<name>.
    #[arg(long)]
    pub codex: bool,
}

/// Deploy every active chef skill to a target client (symlink into its skills dir),
/// preserving any targets it's already deployed to.
pub fn run_deploy(args: &DeployArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let chef_skills_dir = config.chef_dir.join("skills");
    if !chef_skills_dir.exists() {
        return Err(format!(
            "Chef skills dir does not exist: {}",
            chef_skills_dir.display()
        )
        .into());
    }
    if !skill_cookbook_relay::skill_ops::TARGET_CLIENT_IDS.contains(&args.target.as_str()) {
        return Err(format!(
            "Unknown target '{}'. Valid targets: {}",
            args.target,
            skill_cookbook_relay::skill_ops::TARGET_CLIENT_IDS.join(", ")
        )
        .into());
    }

    let skills = scan_path_impl(&chef_skills_dir, "chef", 0).map_err(|e| e.to_string())?;
    if skills.is_empty() {
        println!(
            "No skills found in {}. Nothing to deploy.",
            chef_skills_dir.display()
        );
        return Ok(());
    }

    let mut linked = 0;
    for s in &skills {
        let skill_md_path = Path::new(&s.path);
        let skill_dir = skill_md_path
            .parent()
            .ok_or_else(|| format!("Invalid skill path: {}", s.path))?;
        let mut current =
            skill_cookbook_relay::skill_ops::deployed_targets(skill_dir, &chef_skills_dir);
        if !current.iter().any(|t| t == &args.target) {
            current.push(args.target.clone());
        }
        skill_cookbook_relay::skill_ops::set_skill_targets(skill_dir, &chef_skills_dir, &current)
            .map_err(|e| e.to_string())?;
        println!("  {} -> {}", s.name, args.target);
        linked += 1;
    }
    println!("Deployed {} skill(s) to {}.", linked, args.target);
    Ok(())
}

/// Move skill to chef/skills-inactive, stripping all deployed symlinks first.
pub fn run_deactivate(
    args: &DeactivateArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let chef_skills_dir = config.chef_dir.join("skills");
    if !chef_skills_dir.exists() {
        return Err(format!(
            "Chef skills dir does not exist: {}",
            chef_skills_dir.display()
        )
        .into());
    }

    let skill_dir =
        find_skill_dir_by_name(&chef_skills_dir, &args.skill_name).ok_or_else(|| {
            format!(
                "Skill '{}' not found in {}",
                args.skill_name,
                chef_skills_dir.display()
            )
        })?;
    let resolved_name = skill_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("Invalid skill dir: {}", skill_dir.display()))?
        .to_string();

    let dest = skill_cookbook_relay::skill_ops::deactivate_skill(&config.chef_dir, &resolved_name)
        .map_err(|e| e.to_string())?;
    println!("Moved {} -> {}", skill_dir.display(), dest.display());
    Ok(())
}

/// Activate a skill: move it from skills-inactive to skills.
pub fn run_activate(args: &ActivateArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let dest = skill_cookbook_relay::skill_ops::activate_skill(&config.chef_dir, &args.name)
        .map_err(|e| e.to_string())?;
    println!("Activated '{}' at {}", args.name, dest.display());
    Ok(())
}

/// Delete a skill (from skills/ or skills-inactive/), stripping deployed symlinks first.
pub fn run_delete(args: &DeleteArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;
    skill_cookbook_relay::skill_ops::delete_skill(&config.chef_dir, &args.name)
        .map_err(|e| e.to_string())?;
    println!("Deleted skill '{}'.", args.name);
    Ok(())
}

/// Scan (and optionally adopt) skills that already live inside a client's skills dir.
pub fn run_adopt(args: &AdoptArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;
    if !skill_cookbook_relay::skill_ops::TARGET_CLIENT_IDS.contains(&args.target.as_str()) {
        return Err(format!(
            "Unknown target '{}'. Valid targets: {}",
            args.target,
            skill_cookbook_relay::skill_ops::TARGET_CLIENT_IDS.join(", ")
        )
        .into());
    }

    let candidates =
        skill_cookbook_relay::skill_ops::adopt_scan(&args.target).map_err(|e| e.to_string())?;
    if candidates.is_empty() {
        println!("No adoptable skills found for '{}'.", args.target);
        return Ok(());
    }

    if !args.apply {
        println!(
            "Candidates in {} (dry run, use --apply to adopt):",
            args.target
        );
        for c in &candidates {
            println!("  {} ({})", c.name, c.path);
        }
        return Ok(());
    }

    let names: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
    let adopted =
        skill_cookbook_relay::skill_ops::adopt_apply(&config.chef_dir, &args.target, &names)
            .map_err(|e| e.to_string())?;
    if adopted.is_empty() {
        println!("Nothing adopted (all candidates already exist in chef).");
    } else {
        println!("Adopted {} skill(s) from {}:", adopted.len(), args.target);
        for name in &adopted {
            println!("  {}", name);
        }
    }
    Ok(())
}

/// Publish a skill as a real (non-symlink) copy into a target project repo's
/// `.claude/skills/` and/or `.agents/skills/`, so cloud/web-hosted Claude Code and
/// Codex sessions (which only see what's checked into the repo they clone) can use it.
/// If neither --claude-code nor --codex is given, publishes to both (the feature's
/// whole point is "make available everywhere").
pub fn run_publish(args: &PublishArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let chef_skills_dir = config.chef_dir.join("skills");

    let skill_dir = find_skill_dir_by_name(&chef_skills_dir, &args.name)
        .or_else(|| {
            let inactive_dir = config.chef_dir.join("skills-inactive");
            find_skill_dir_by_name(&inactive_dir, &args.name)
        })
        .ok_or_else(|| {
            format!(
                "Skill '{}' not found in skills or skills-inactive",
                args.name
            )
        })?;

    let publish_ids = skill_cookbook_relay::skill_ops::PROJECT_PUBLISH_TARGET_IDS;
    let mut targets: Vec<String> = Vec::new();
    if args.claude_code {
        targets.push("claude_code_project".to_string());
    }
    if args.codex {
        targets.push("codex_project".to_string());
    }
    if targets.is_empty() {
        targets = publish_ids.iter().map(|s| s.to_string()).collect();
    }

    let written = skill_cookbook_relay::skill_ops::publish_skill_to_project(
        &skill_dir,
        &args.project_dir,
        &targets,
    )
    .map_err(|e| e.to_string())?;

    if written.is_empty() {
        println!("Nothing published (no recognized targets).");
    } else {
        println!("Published '{}' to:", args.name);
        for t in &written {
            match t.as_str() {
                "claude_code_project" => println!(
                    "  {}",
                    args.project_dir
                        .join(".claude")
                        .join("skills")
                        .join(&args.name)
                        .display()
                ),
                "codex_project" => println!(
                    "  {}",
                    args.project_dir
                        .join(".agents")
                        .join("skills")
                        .join(&args.name)
                        .display()
                ),
                other => println!("  ({})", other),
            }
        }
    }
    Ok(())
}

/// Resolve SKILLNAME to a skill dir under chef_skills_dir: try direct child first, then match by deploy link name.
fn find_skill_dir_by_name(chef_skills_dir: &Path, skill_name: &str) -> Option<PathBuf> {
    let direct = chef_skills_dir.join(skill_name);
    if direct.is_dir() && direct.join("SKILL.md").exists() {
        return Some(direct);
    }
    let skills = scan_path_impl(chef_skills_dir, "chef", 0).ok()?;
    for s in skills {
        let skill_md_path = Path::new(&s.path);
        let skill_dir = skill_md_path.parent()?;
        if skill_cookbook_relay::skill_ops::link_name_for_skill_dir(skill_dir, chef_skills_dir)
            == skill_name
        {
            return Some(skill_dir.to_path_buf());
        }
    }
    None
}

/// Sync chef: commit local, optionally push to server.
pub fn run_sync_chef(args: &SyncArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;

    git_add_commit(&config.chef_dir, "Sync chef")?;

    if !args.no_push {
        push_local_skills_to_server(&config)?;
    }
    Ok(())
}

fn push_local_skills_to_server(
    config: &Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                Ok(zip_bytes) => match studio.upload_skill(&token, zip_bytes).await {
                    Ok(skill_id) => println!("Pushed {} -> {}", s.name, skill_id),
                    Err(e) => eprintln!("Failed to push {}: {}", s.name, e),
                },
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
) -> Result<Vec<skill_cookbook_relay::types::LocalSkill>, Box<dyn std::error::Error + Send + Sync>>
{
    let skills_dir = config.chef_dir.join("skills");
    let entries = list_local_skills(&skills_dir)?;
    Ok(entries
        .into_iter()
        .map(|(name, has_skill_md)| {
            let skill_path = skills_dir.join(&name);
            let targets =
                skill_cookbook_relay::skill_ops::deployed_targets(&skill_path, &skills_dir);
            let path = skill_path.to_string_lossy().to_string();
            skill_cookbook_relay::types::LocalSkill {
                name,
                has_skill_md,
                path,
                status: "draft".to_string(),
                version: "0.1.0".to_string(),
                tags: vec![],
                description: None,
                active: true,
                targets,
                source: String::new(),
                created_at: String::new(),
                updated_at: String::new(),
            }
        })
        .collect())
}

/// Structured list of remote skills (for Tauri UI).
pub fn list_remote_skills_structured(
    config: &Config,
) -> Result<
    Vec<skill_cookbook_relay::types::RemoteSkillInfo>,
    Box<dyn std::error::Error + Send + Sync>,
> {
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
            let skills = rss
                .list_skills(&lib.library_id)
                .await
                .map_err(|e| e.to_string())?;
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
    let skill_md_path =
        skill_cookbook_relay::discovery::resolve_skill_md_path(path).map_err(|e| e.to_string())?;
    let skill_name = skill_md_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("skill")
        .to_string();
    let dest_dir = config.chef_dir.join("skills").join(&skill_name);
    let src_dir = skill_md_path
        .parent()
        .ok_or_else(|| format!("Invalid skill path: {}", skill_md_path.display()))?;
    skill_cookbook_relay::skill_ops::copy_dir_all(src_dir, &dest_dir).map_err(|e| e.to_string())?;
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

fn push_local_skills_count(
    config: &Config,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
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
            let skills = rss
                .list_skills(&lib.library_id)
                .await
                .map_err(|e| e.to_string())?;
            for s in skills {
                println!("  - {} ({})", s.name, s.skill_id);
            }
        }
        Ok(())
    })
}

pub(crate) fn git_add_commit(
    repo_root: &Path,
    message: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
