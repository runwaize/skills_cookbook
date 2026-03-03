//! Runwaize Skills Cookbook CLI — library API for Tauri and binary.

mod chef;
mod doctor;
mod guest;
mod init;
mod install;
mod library;
mod search;
mod workspace;

#[cfg(test)]
mod cli_tests;

use clap::Parser;
use std::process::ExitCode;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use skill_cookbook_relay::auth::AuthManager;

/// Runwaize Skills Cookbook CLI — chef/guest skills, sync, install, doctor.
#[derive(Parser, Debug)]
#[command(name = "skills_cookbook")]
#[command(about = "Runwaize Skills Cookbook CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    Usage,
    Init(init::InitArgs),
    Doctor(doctor::DoctorArgs),
    Login,
    Logout,
    /// List workspaces or select one.
    Workspace {
        #[command(subcommand)]
        sub: WorkspaceSub,
    },
    /// Chef skill management (list, edit, add, sync, search).
    #[command(visible_alias = "cook")]
    Chef {
        #[command(subcommand)]
        sub: ChefSub,
    },
    /// Skill lifecycle (status).
    Skill {
        #[command(subcommand)]
        sub: SkillSub,
    },
    /// Library management.
    Library {
        #[command(subcommand)]
        sub: LibrarySub,
    },
    Install,
    Remove,
    Update,
}

#[derive(clap::Subcommand, Debug)]
pub enum ChefSub {
    /// List local skills in chef dir. Use --remote to fetch from server.
    List(chef::ListArgs),
    /// Open a skill's SKILL.md in your editor.
    Edit(chef::EditArgs),
    /// Add skill at path to chef dir.
    Add(chef::AddArgs),
    /// Sync chef (commit, push) and guest (replace manifest).
    Sync(chef::SyncArgs),
    /// Search for SKILL.md files and add selected ones to chef.
    Search(search::SearchArgs),
}

#[derive(clap::Subcommand, Debug)]
pub enum WorkspaceSub {
    /// List workspaces or set current (stub).
    Select(workspace::WorkspaceSelectArgs),
}

#[derive(clap::Subcommand, Debug)]
pub enum SkillSub {
    /// Set skill status for release (API TBD).
    Status(library::SkillStatusArgs),
}

#[derive(clap::Subcommand, Debug)]
pub enum LibrarySub {
    Add(library::LibraryAddArgs),
    Remove(library::LibraryRemoveArgs),
    AddSkill(library::LibraryAddSkillArgs),
    RemoveSkill(library::LibraryRemoveSkillArgs),
}

/// Run the CLI; returns exit code.
pub fn run() -> ExitCode {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "runwaize_skills_cookbook_cli=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let cli = Cli::parse();

    match run_command(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run_command(cli: &Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match &cli.command {
        Command::Usage => {
            print_help();
            Ok(())
        }
        Command::Init(args) => init::run_init(args),
        Command::Doctor(args) => doctor::run_doctor(args),
        Command::Login => run_login(),
        Command::Logout => run_logout(),
        Command::Workspace { sub } => match sub {
            WorkspaceSub::Select(args) => workspace::run_workspace_select(args),
        },
        Command::Chef { sub } => match sub {
            ChefSub::List(args) => chef::run_list_local(args),
            ChefSub::Edit(args) => chef::run_edit(args),
            ChefSub::Add(args) => chef::run_add(args),
            ChefSub::Sync(args) => run_sync(args),
            ChefSub::Search(args) => search::run_search(args),
        },
        Command::Skill { sub } => match sub {
            SkillSub::Status(args) => library::run_skill_status(args),
        },
        Command::Library { sub } => match sub {
            LibrarySub::Add(args) => library::run_library_add(args),
            LibrarySub::Remove(args) => library::run_library_remove(args),
            LibrarySub::AddSkill(args) => library::run_library_add_skill(args),
            LibrarySub::RemoveSkill(args) => library::run_library_remove_skill(args),
        },
        Command::Install => install::run_install(),
        Command::Remove => install::run_remove(),
        Command::Update => install::run_update(),
    }
}

fn run_sync(args: &chef::SyncArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = skill_cookbook_relay::config::Config::load().map_err(|e| e.to_string())?;
    chef::run_sync_chef(args)?;
    guest::sync_guest_from_server(&config)?;
    Ok(())
}

fn print_help() {
    println!("Runwaize Skills Cookbook CLI");
    println!();
    println!("Usage: skills_cookbook <COMMAND>");
    println!();
    println!("Commands:");
    println!("  usage     Show this help");
    println!("  init      Create chef and guest dirs, init chef git repo");
    println!("  doctor    Diagnose config, dirs, auth, git, network");
    println!("  login     Login to Supervaize (device flow)");
    println!("  logout    Logout and clear tokens");
    println!("  workspace select [id]  List or set workspace");
    println!();
    println!("  chef (cook)  Chef skill management:");
    println!("    chef list [--remote]   List local skills (or server skills with --remote)");
    println!("    chef edit [name]       Open SKILL.md in editor (interactive picker if no name)");
    println!("    chef add <path>        Add skill to chef dir");
    println!("    chef sync [--no-push]  Sync chef and guest");
    println!("    chef search [path]     Scan for skills and add selected to chef");
    println!();
    println!("  skill status <id> <status>  Set skill status (API TBD)");
    println!("  library add|remove|add-skill|remove-skill  Manage libraries (API TBD)");
    println!("  install   Install CLI to PATH");
    println!("  remove    Uninstall CLI from PATH");
    println!("  update    Self-update (stub)");
}

fn run_login() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = skill_cookbook_relay::config::Config::load().map_err(|e| e.to_string())?;
    let auth = std::sync::Arc::new(
        AuthManager::new(config).map_err(|e| e.to_string())?,
    );
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let dev = auth.start_device_flow().await.map_err(|e| e.to_string())?;
        println!("Visit: {}", dev.verification_uri);
        println!("Code: {}", dev.user_code);
        if let Some(ref u) = dev.verification_uri_complete {
            println!("Or open: {}", u);
        }
        let _ = auth
            .poll_device_flow(dev.device_code, dev.interval)
            .await
            .map_err(|e| e.to_string())?;
        println!("Logged in.");
        Ok(())
    })
}

fn run_logout() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = skill_cookbook_relay::config::Config::load().map_err(|e| e.to_string())?;
    let auth = AuthManager::new(config).map_err(|e| e.to_string())?;
    auth.logout().map_err(|e| e.to_string())?;
    println!("Logged out.");
    Ok(())
}
