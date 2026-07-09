//! Local discovery: agent detection and skill scanning.
//!
//! Discovers installed skill-capable clients (Cursor, Codex, etc.) by checking
//! known config paths, and scans directories for SKILL.md files.

mod handlers;

pub use handlers::{discover_apps, discovery_import, discovery_scan};

use crate::error::{RelayError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_SCAN_DEPTH: u32 = 10;

/// Client with supports_auto_discovery from studio client_registry.
#[derive(Clone, Debug)]
pub struct ClientDef {
    pub id: &'static str,
    pub name: &'static str,
}

const DISCOVERABLE_CLIENTS: &[ClientDef] = &[
    ClientDef {
        id: "claude_code",
        name: "Claude Code",
    },
    ClientDef {
        id: "cursor",
        name: "Cursor",
    },
    ClientDef {
        id: "codex",
        name: "Codex",
    },
    ClientDef {
        id: "codeium",
        name: "Codeium",
    },
    ClientDef {
        id: "windsurf",
        name: "Windsurf",
    },
    ClientDef {
        id: "aider",
        name: "Aider",
    },
    ClientDef {
        id: "zed",
        name: "Zed",
    },
];

#[derive(Serialize)]
pub struct DiscoveredClient {
    pub id: String,
    pub name: String,
    pub detected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Serialize)]
pub struct DiscoverAppsResponse {
    pub clients: Vec<DiscoveredClient>,
}

#[derive(Serialize)]
pub struct ScannedSkill {
    pub path: String,
    pub name: String,
    pub identifier: String,
    pub client_id: String,
}

#[derive(Serialize)]
pub struct ScanResponse {
    pub skills: Vec<ScannedSkill>,
}

#[derive(Deserialize)]
pub struct ScanRequest {
    pub path: Option<String>,
    pub client_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ImportRequest {
    pub paths: Vec<String>,
    pub studio_token: String,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub path: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ImportResponse {
    pub results: Vec<ImportResult>,
}

/// Paths to check per client (Unix/macOS). First existing wins.
fn client_paths(client_id: &str, home: &Path) -> Vec<PathBuf> {
    match client_id {
        "cursor" => vec![
            home.join(".cursor").join("skills"),
            home.join(".cursor"),
            home.join("Library")
                .join("Application Support")
                .join("Cursor"),
        ],
        "codex" => {
            let codex_home = std::env::var("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".codex"));
            vec![
                codex_home.join("skills"),
                home.join(".codex").join("skills"),
            ]
        }
        "claude_code" => vec![home.join(".claude").join("plugins"), home.join(".claude")],
        "codeium" => vec![home.join(".codeium")],
        "windsurf" => vec![
            home.join(".windsurf"),
            home.join("Library")
                .join("Application Support")
                .join("Windsurf"),
        ],
        "aider" => vec![home.join(".aider")],
        "zed" => vec![home.join(".zed")],
        _ => vec![],
    }
}

/// Resolve the first existing path for a client.
pub fn resolve_client_path(client_id: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    for p in client_paths(client_id, &home) {
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Discover installed clients.
pub fn discover_apps_impl() -> DiscoverAppsResponse {
    let clients: Vec<DiscoveredClient> = DISCOVERABLE_CLIENTS
        .iter()
        .map(|c| {
            let path = resolve_client_path(c.id);
            DiscoveredClient {
                id: c.id.to_string(),
                name: c.name.to_string(),
                detected: path.is_some(),
                path: path.map(|p| p.to_string_lossy().to_string()),
            }
        })
        .collect();
    DiscoverAppsResponse { clients }
}

/// Extract frontmatter (name, description) from SKILL.md content.
pub fn extract_frontmatter(content: &str) -> (std::collections::HashMap<String, String>, &str) {
    const DELIM: &str = "---";
    let s = content.trim_start();
    if !s.starts_with(DELIM) {
        return (std::collections::HashMap::new(), content.trim());
    }
    let after_first = s[DELIM.len()..].trim_start();
    let end = match after_first.find('\n') {
        Some(i) => i,
        None => return (std::collections::HashMap::new(), content.trim()),
    };
    let rest = after_first[end + 1..].trim_start();
    if !rest.starts_with(DELIM) {
        return (std::collections::HashMap::new(), content.trim());
    }
    let meta_str = &after_first[..end];
    let body = rest[DELIM.len()..].trim_start();
    let mut meta = std::collections::HashMap::new();
    for line in meta_str.lines() {
        if let Some((k, v)) = line.split_once(':') {
            meta.insert(
                k.trim().to_lowercase(),
                v.trim().trim_matches(|c| c == '"' || c == '\'').to_string(),
            );
        }
    }
    (meta, body)
}

fn derive_skill_name(meta: &std::collections::HashMap<String, String>, path: &Path) -> String {
    meta.get("name")
        .or_else(|| meta.get("title"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Untitled Skill")
                .to_string()
        })
}

fn derive_identifier(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

/// Scan a directory recursively for SKILL.md files.
pub fn scan_path_impl(path: &Path, client_id: &str, depth: u32) -> Result<Vec<ScannedSkill>> {
    if depth > MAX_SCAN_DEPTH {
        return Ok(vec![]);
    }
    let mut skills = Vec::new();
    let entries = std::fs::read_dir(path).map_err(|e| {
        RelayError::Discovery(format!(
            "Failed to read directory {}: {}",
            path.display(),
            e
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| RelayError::Discovery(e.to_string()))?;
        let p = entry.path();
        if p.is_dir() {
            skills.extend(scan_path_impl(&p, client_id, depth + 1)?);
        } else if p.file_name().map(|n| n == "SKILL.md").unwrap_or(false) {
            let content = std::fs::read_to_string(&p).map_err(|e| {
                RelayError::Discovery(format!("Failed to read {}: {}", p.display(), e))
            })?;
            let (meta, _) = extract_frontmatter(&content);
            let name = derive_skill_name(&meta, &p);
            let identifier = derive_identifier(&p);
            skills.push(ScannedSkill {
                path: p.to_string_lossy().to_string(),
                name,
                identifier,
                client_id: client_id.to_string(),
            });
        }
    }
    Ok(skills)
}

/// Resolve path to SKILL.md: if path is a file, use it; if dir, look for SKILL.md inside.
pub fn resolve_skill_md_path(path: &str) -> Result<PathBuf> {
    let p = Path::new(path);
    if p.is_file() {
        if p.file_name().map(|n| n == "SKILL.md").unwrap_or(false) {
            return Ok(p.to_path_buf());
        }
        return Err(RelayError::Discovery(format!(
            "Path is a file but not SKILL.md: {}",
            path
        )));
    }
    if p.is_dir() {
        let skill_md = p.join("SKILL.md");
        if skill_md.exists() {
            return Ok(skill_md);
        }
        return Err(RelayError::Discovery(format!(
            "Directory does not contain SKILL.md: {}",
            path
        )));
    }
    Err(RelayError::Discovery(format!(
        "Path does not exist: {}",
        path
    )))
}

/// Build a zip containing SKILL.md for studio ingestion.
pub fn build_skill_zip(skill_md_path: &Path) -> Result<Vec<u8>> {
    use std::io::Write;
    let content = std::fs::read_to_string(skill_md_path)
        .map_err(|e| RelayError::Discovery(format!("Failed to read SKILL.md: {}", e)))?;
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options: zip::write::FileOptions<'_, zip::write::ExtendedFileOptions> =
            zip::write::FileOptions::default()
                .unix_permissions(0o644)
                .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("SKILL.md", options)
            .map_err(|e| RelayError::Discovery(format!("Zip error: {}", e)))?;
        zip.write_all(content.as_bytes())
            .map_err(|e| RelayError::Discovery(format!("Zip write error: {}", e)))?;
        zip.finish()
            .map_err(|e| RelayError::Discovery(format!("Zip finish error: {}", e)))?;
    }
    Ok(buf)
}
