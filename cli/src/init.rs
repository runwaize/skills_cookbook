//! Init command: create chef and guest dirs, init chef git repo.

use skill_cookbook_relay::config::Config;
use std::path::Path;

#[derive(clap::Args, Debug)]
pub struct InitArgs {
    /// Use custom chef dir (default: ~/.runwaize_skills_cookbook/chef).
    #[arg(long)]
    pub chef_dir: Option<std::path::PathBuf>,
    /// Use custom guest dir (default: ~/.runwaize_skills_cookbook/cook).
    #[arg(long)]
    pub guest_dir: Option<std::path::PathBuf>,
}

/// Create chef and guest dirs, init git in chef, save config.
pub fn run_init(args: &InitArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = Config::load().unwrap_or_else(|_| Config::default());

    let chef_dir = args.chef_dir.clone().unwrap_or_else(|| default_chef_dir());
    let guest_dir = args
        .guest_dir
        .clone()
        .unwrap_or_else(|| default_guest_dir());

    std::fs::create_dir_all(&chef_dir).map_err(|e| format!("create chef dir: {}", e))?;
    std::fs::create_dir_all(&guest_dir).map_err(|e| format!("create guest dir: {}", e))?;

    init_git_repo(&chef_dir)?;

    config.chef_dir = chef_dir.clone();
    config.guest_dir = guest_dir.clone();
    config.save().map_err(|e| e.to_string())?;

    println!("Chef dir: {}", chef_dir.display());
    println!("Guest dir: {}", guest_dir.display());
    println!("Config saved.");
    Ok(())
}

fn default_chef_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".runwaize_skills_cookbook")
        .join("chef")
}

fn default_guest_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".runwaize_skills_cookbook")
        .join("cook")
}

fn init_git_repo(dir: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let git_dir = dir.join(".git");
    if git_dir.exists() {
        return Ok(());
    }
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .status()
        .map_err(|e| format!("git init: {}", e))?;
    Ok(())
}
