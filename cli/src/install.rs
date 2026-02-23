//! Install, remove, update CLI binary.

use std::path::PathBuf;

/// Copy binary to a dir on PATH (e.g. ~/.local/bin) or print instructions.
pub fn run_install() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bin_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let dest = default_install_dir()?;
    std::fs::create_dir_all(&dest).map_err(|e| format!("create dir: {}", e))?;
    let dest_bin = dest.join("skills_cookbook");
    std::fs::copy(&bin_path, &dest_bin).map_err(|e| format!("copy: {}", e))?;
    println!("Installed to {}", dest_bin.display());
    println!("Ensure {} is on your PATH.", dest.display());
    Ok(())
}

/// Remove binary from install location if it was installed by us.
pub fn run_remove() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dest = default_install_dir()?;
    let dest_bin = dest.join("skills_cookbook");
    if dest_bin.exists() {
        std::fs::remove_file(&dest_bin).map_err(|e| e.to_string())?;
        println!("Removed {}", dest_bin.display());
    } else {
        println!("No installed binary at {}", dest_bin.display());
    }
    Ok(())
}

/// Check for new release and prompt to update (stub).
pub fn run_update() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("update: self-update not yet implemented (check releases for new binary)");
    Ok(())
}

fn default_install_dir() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    dirs::home_dir()
        .ok_or_else(|| "cannot determine home dir".into())
        .map(|h| h.join(".local").join("bin"))
}
