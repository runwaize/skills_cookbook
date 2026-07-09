//! Guest manifest (curated skill list). Read by relay; written by CLI.
//! Same format as runwaize_skills_cookbook_cli::guest.

use serde::{Deserialize, Serialize};
use std::path::Path;

const MANIFEST_FILENAME: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestSkillEntry {
    pub skill_id: String,
    pub name: String,
    pub description: Option<String>,
    pub library_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuestManifest {
    pub skills: Vec<GuestSkillEntry>,
}

/// Write manifest to guest dir.
pub fn write_guest_manifest(
    guest_dir: &Path,
    manifest: &GuestManifest,
) -> crate::error::Result<()> {
    std::fs::create_dir_all(guest_dir)
        .map_err(|e| crate::error::RelayError::Config(format!("create guest dir: {}", e)))?;
    let path = guest_dir.join(MANIFEST_FILENAME);
    let data = serde_json::to_string_pretty(manifest)
        .map_err(|e| crate::error::RelayError::Config(format!("serialize manifest: {}", e)))?;
    std::fs::write(&path, data)
        .map_err(|e| crate::error::RelayError::Config(format!("write manifest: {}", e)))?;
    Ok(())
}

/// Read manifest from guest dir. Returns default if file missing.
pub fn read_guest_manifest(guest_dir: &Path) -> crate::error::Result<GuestManifest> {
    let path = guest_dir.join(MANIFEST_FILENAME);
    if !path.exists() {
        return Ok(GuestManifest::default());
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| crate::error::RelayError::Config(format!("read guest manifest: {}", e)))?;
    let manifest: GuestManifest = serde_json::from_str(&data)
        .map_err(|e| crate::error::RelayError::Config(format!("parse guest manifest: {}", e)))?;
    Ok(manifest)
}
