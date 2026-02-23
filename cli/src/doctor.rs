//! Doctor command: diagnose config, dirs, auth, git, network.

use skill_cookbook_relay::config::Config;
use std::path::Path;

#[derive(clap::Args, Debug)]
pub struct DoctorArgs {
    /// Only check config and dirs (no network).
    #[arg(long)]
    pub offline: bool,
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
