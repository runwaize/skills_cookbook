//! Workspace list/select (stub until backend exposes workspace API).

use skill_cookbook_relay::config::Config;

#[derive(clap::Args, Debug)]
pub struct WorkspaceSelectArgs {
    /// Workspace ID to set as default (optional; without it, lists workspaces).
    pub id: Option<String>,
}

/// List workspaces (stub: returns default) or select one and save to config.
pub fn run_workspace_select(
    args: &WorkspaceSelectArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = Config::load().map_err(|e| e.to_string())?;

    match &args.id {
        Some(id) => {
            config.workspace_id = Some(id.clone());
            config.save().map_err(|e| e.to_string())?;
            println!("Workspace set to: {}", id);
        }
        None => {
            if let Some(ref w) = config.workspace_id {
                println!("Current workspace: {}", w);
            } else {
                println!("Current workspace: (not set)");
            }
            println!("(Workspace list from server not yet available; use select <id> to set.)");
        }
    }
    Ok(())
}
