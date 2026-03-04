//! Doctor command: diagnose config, dirs, auth, git, network.

use skill_cookbook_relay::config::Config;
use std::path::Path;

#[derive(clap::Args, Debug)]
pub struct DoctorArgs {
    /// Only check config and dirs (no network).
    #[arg(long)]
    pub offline: bool,
}

/// Run doctor and return structured diagnostics (for Tauri UI).
pub fn run_doctor_structured() -> Result<Vec<skill_cookbook_relay::types::DiagnosticCheck>, Box<dyn std::error::Error + Send + Sync>> {
    use skill_cookbook_relay::types::{DiagnosticCheck, DiagnosticStatus};
    let mut checks = Vec::new();

    match Config::load() {
        Ok(config) => {
            checks.push(DiagnosticCheck {
                name: "Config".into(),
                status: DiagnosticStatus::Ok,
                message: format!("Loaded from {}", config.chef_dir.display()),
            });

            // Chef dir
            if config.chef_dir.exists() && config.chef_dir.is_dir() {
                checks.push(DiagnosticCheck {
                    name: "Chef directory".into(),
                    status: DiagnosticStatus::Ok,
                    message: config.chef_dir.to_string_lossy().into(),
                });
            } else {
                checks.push(DiagnosticCheck {
                    name: "Chef directory".into(),
                    status: DiagnosticStatus::Error,
                    message: format!("Missing: {}", config.chef_dir.display()),
                });
            }

            // Guest dir
            if config.guest_dir.exists() {
                checks.push(DiagnosticCheck {
                    name: "Guest directory".into(),
                    status: DiagnosticStatus::Ok,
                    message: config.guest_dir.to_string_lossy().into(),
                });
            } else {
                checks.push(DiagnosticCheck {
                    name: "Guest directory".into(),
                    status: DiagnosticStatus::Warning,
                    message: format!("Missing: {}", config.guest_dir.display()),
                });
            }

            // Git repo
            if config.chef_dir.join(".git").exists() {
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
                status: if config.workspace_id.is_some() {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Warning
                },
                message: config.workspace_id.as_deref().unwrap_or("Not set").into(),
            });
        }
        Err(e) => {
            checks.push(DiagnosticCheck {
                name: "Config".into(),
                status: DiagnosticStatus::Error,
                message: format!("Failed to load: {}", e),
            });
        }
    }

    Ok(checks)
}

pub fn run_doctor(args: &DoctorArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut ok = true;

    match Config::load() {
        Ok(config) => {
            println!("config: ok ({})", config.chef_dir.display());
            check_dir("chef_dir", &config.chef_dir, true, &mut ok);
            check_dir("guest_dir", &config.guest_dir, false, &mut ok);
            check_git_repo(&config.chef_dir, &mut ok);
            if let Some(w) = &config.workspace_id {
                println!("workspace_id: {}", w);
            } else {
                println!("workspace_id: (not set)");
            }
        }
        Err(e) => {
            println!("config: error — {}", e);
            ok = false;
        }
    }

    if !args.offline {
        println!("network: (not checked)");
    }

    if ok {
        println!("doctor: all checks passed");
        Ok(())
    } else {
        Err("some checks failed".into())
    }
}

fn check_dir(name: &str, path: &Path, require_writable: bool, ok: &mut bool) {
    if !path.exists() {
        println!("{}: missing ({})", name, path.display());
        *ok = false;
        return;
    }
    if !path.is_dir() {
        println!("{}: not a directory", name);
        *ok = false;
        return;
    }
    if require_writable && !check_writable(path, name, ok) {
        return;
    }
    println!("{}: ok ({})", name, path.display());
}

fn check_writable(path: &Path, name: &str, ok: &mut bool) -> bool {
    let test_file = path.join(".doctor_write_test");
    if std::fs::write(&test_file, b"").is_err() {
        println!("{}: not writable", name);
        *ok = false;
        false
    } else {
        let _ = std::fs::remove_file(test_file);
        true
    }
}

fn check_git_repo(chef_dir: &Path, ok: &mut bool) {
    let git_dir = chef_dir.join(".git");
    if !git_dir.exists() {
        println!("chef git: not a repo (run init)");
        *ok = false;
    } else {
        println!("chef git: ok");
    }
}
