//! Guest manifest: compiled curated skill list (metadata only).

use serde::{Deserialize, Serialize};
use skill_cookbook_relay::auth::AuthManager;
use skill_cookbook_relay::rss_client::RssClient;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestSkillEntry {
    pub skill_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuestManifest {
    pub skills: Vec<GuestSkillEntry>,
}

const MANIFEST_FILENAME: &str = "manifest.json";

/// Read manifest from guest dir.
#[allow(dead_code)] // used by Tauri/relay when reading guest list
pub fn read_manifest(
    guest_dir: &std::path::Path,
) -> Result<GuestManifest, Box<dyn std::error::Error + Send + Sync>> {
    let path = guest_dir.join(MANIFEST_FILENAME);
    if !path.exists() {
        return Ok(GuestManifest::default());
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("read manifest: {}", e))?;
    let manifest: GuestManifest = serde_json::from_str(&data)
        .map_err(|e| format!("parse manifest: {}", e))?;
    Ok(manifest)
}

/// Write manifest to guest dir.
pub fn write_manifest(
    guest_dir: &std::path::Path,
    manifest: &GuestManifest,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::fs::create_dir_all(guest_dir).map_err(|e| format!("create guest dir: {}", e))?;
    let path = guest_dir.join(MANIFEST_FILENAME);
    let data = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("serialize manifest: {}", e))?;
    std::fs::write(&path, data).map_err(|e| format!("write manifest: {}", e))?;
    Ok(())
}

/// Fetch curated skills from server (libraries + skills) and write manifest to guest dir.
pub fn sync_guest_from_server(
    config: &skill_cookbook_relay::config::Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config.clone();
    let guest_dir = config.guest_dir.clone();
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let auth = Arc::new(
            AuthManager::new(config.clone()).map_err(|e| e.to_string())?,
        );
        auth.initialize().await.map_err(|e| e.to_string())?;
        if !auth.is_authenticated() {
            println!("Not authenticated. Run login first. Guest manifest not updated.");
            return Ok(());
        }
        let rss = RssClient::new(config.rss_api_url.clone(), auth);
        let libraries = rss.list_libraries().await.map_err(|e| e.to_string())?;
        let mut skills = Vec::new();
        for lib in &libraries {
            let lib_skills = rss.list_skills(&lib.library_id).await.map_err(|e| e.to_string())?;
            for s in lib_skills {
                skills.push(GuestSkillEntry {
                    skill_id: s.skill_id,
                    name: s.name,
                    description: s.description,
                    library_id: Some(lib.library_id.clone()),
                });
            }
        }
        let manifest = GuestManifest { skills };
        write_manifest(&guest_dir, &manifest)?;
        println!("Guest manifest updated: {} skills", manifest.skills.len());
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = GuestManifest {
            skills: vec![
                GuestSkillEntry {
                    skill_id: "s1".to_string(),
                    name: "Skill One".to_string(),
                    description: Some("First".to_string()),
                    library_id: Some("lib1".to_string()),
                },
                GuestSkillEntry {
                    skill_id: "s2".to_string(),
                    name: "Skill Two".to_string(),
                    description: None,
                    library_id: None,
                },
            ],
        };
        write_manifest(dir.path(), &manifest).unwrap();
        let read = read_manifest(dir.path()).unwrap();
        assert_eq!(read.skills.len(), 2);
        assert_eq!(read.skills[0].skill_id, "s1");
        assert_eq!(read.skills[1].name, "Skill Two");
    }

    #[test]
    fn manifest_missing_file_is_default() {
        let dir = tempfile::tempdir().unwrap();
        let read = read_manifest(dir.path()).unwrap();
        assert!(read.skills.is_empty());
    }
}
