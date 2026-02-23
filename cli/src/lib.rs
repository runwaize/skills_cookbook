//! Runwaize Skills Cookbook CLI — library API for Tauri and binary.

mod doctor;
mod init;
mod guest;

#[cfg(test)]
mod cli_tests;

use clap::Parser;
use std::process::ExitCode;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    /// Show help and subcommands.
    Help,
    /// Create chef and guest dirs, init chef git repo.
    Init(init::InitArgs),
    /// Diagnose config, dirs, auth, git, network.
    Doctor(doctor::DoctorArgs),
    /// Login to Supervaize (device flow).
    Login,
    /// Logout and clear tokens.
    Logout,
    /// List skills from server.
    List,
    /// Sync chef (commit, pull, push) and guest (replace manifest).
    Sync,
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
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Init(args) => init::run_init(args),
        Command::Doctor(args) => doctor::run_doctor(args),
        Command::Login => run_login(),
        Command::Logout => run_logout(),
        Command::List => run_list(),
        Command::Sync => run_sync(),
    }
}

fn print_help() {
    println!("Runwaize Skills Cookbook CLI");
    println!();
    println!("Usage: skills_cookbook <COMMAND>");
    println!();
    println!("Commands:");
    println!("  help    Show this help");
    println!("  init    Create chef and guest dirs, init chef git repo");
    println!("  doctor  Diagnose config, dirs, auth, git, network");
    println!("  login   Login to Supervaize (device flow)");
    println!("  logout  Logout and clear tokens");
    println!("  list    List skills from server");
    println!("  sync    Sync chef and guest");
}

fn run_login() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("login: not yet implemented (use desktop app for device flow)");
    Ok(())
}

fn run_logout() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("logout: not yet implemented");
    Ok(())
}

fn run_list() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("list: not yet implemented");
    Ok(())
}

fn run_sync() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = skill_cookbook_relay::config::Config::load()
        .map_err(|e| e.to_string())?;
    let manifest = guest::read_manifest(&config.guest_dir)?;
    println!("sync: not yet implemented (guest manifest has {} skills)", manifest.skills.len());
    Ok(())
}
