//! Shallow-clone a GitHub (or any git-hosted) repo into a tempdir and scan it for
//! SKILL.md files, reusing the same discovery logic as the existing local-folder scan.

use crate::discovery;
use crate::error::{RelayError, Result};
use crate::types::ScannedSkillInfo;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Validate that `url` looks like a real git remote URL before it's ever handed to
/// `std::process::Command`. Rejects anything that isn't an `https://`, `http://`, or
/// `git@` URL -- in particular anything starting with `-`, which `git clone` could
/// otherwise interpret as a flag (flag-injection via a crafted "url").
pub fn validate_git_url(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return Err(RelayError::Discovery(format!(
            "Invalid repository URL: {}",
            url
        )));
    }
    let looks_like_git_url = trimmed.starts_with("https://")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("git@");
    if !looks_like_git_url {
        return Err(RelayError::Discovery(format!(
            "Invalid repository URL, must start with https://, http://, or git@: {}",
            url
        )));
    }
    Ok(())
}

/// Best-effort `owner/repo` slug extraction from a GitHub-style URL, for provenance
/// tagging (`github:owner/repo`). Returns `None` if the URL doesn't parse into at
/// least two path segments.
pub fn repo_slug_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);

    // git@github.com:owner/repo
    if let Some(rest) = trimmed.strip_prefix("git@") {
        let (_, path) = rest.split_once(':')?;
        return normalize_slug(path);
    }
    // https://github.com/owner/repo or http://host/owner/repo
    for prefix in ["https://", "http://"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let mut parts = rest.splitn(2, '/');
            parts.next()?; // host
            let path = parts.next()?;
            return normalize_slug(path);
        }
    }
    None
}

fn normalize_slug(path: &str) -> Option<String> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
        return None;
    }
    Some(format!("{}/{}", segments[0], segments[1]))
}

/// Names of subdirectories already present under `chef_skills_dir` (mirrors
/// `existing_skill_names` in `main.rs`'s local-folder scan, kept local here to avoid
/// a cross-module dependency for such a small helper).
fn existing_skill_names(chef_skills_dir: &Path) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Ok(entries) = std::fs::read_dir(chef_skills_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    names.insert(name.to_string());
                }
            }
        }
    }
    names
}

/// Shallow-clone `url` into a fresh tempdir and scan it for SKILL.md files, reusing
/// `discovery::scan_path_impl` (do not reimplement SKILL.md discovery).
///
/// Deliberately does NOT delete the tempdir before returning: each returned
/// `ScannedSkillInfo.path` points inside it, and those paths need to stay valid until
/// the user reviews/imports the results (`import_skills` takes plain paths, same as
/// the existing local-folder scan). Cleanup is left to the OS temp directory
/// lifecycle -- an accepted simplification matching this project's existing tests,
/// which already rely on the same non-cleanup convention for tempdirs.
pub fn clone_and_scan_repo(url: &str, chef_skills_dir: &Path) -> Result<Vec<ScannedSkillInfo>> {
    validate_git_url(url)?;

    let tmp = tempfile::tempdir()
        .map_err(|e| RelayError::Internal(format!("Failed to create temp dir: {}", e)))?;
    let tmp_path: PathBuf = tmp.path().to_path_buf();

    // Args as a Vec passed to Command (never a shell string) -- no injection risk from
    // special characters in `url`. The `--` separator plus the leading-dash rejection
    // in `validate_git_url` both defend against flag-injection via a crafted url.
    let output = Command::new("git")
        .args(["clone", "--depth", "1", "--", url])
        .arg(&tmp_path)
        .output()
        .map_err(|e| RelayError::Discovery(format!("Failed to run git clone: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RelayError::Discovery(format!(
            "git clone failed for {}: {}",
            url,
            stderr.trim()
        )));
    }

    let existing = existing_skill_names(chef_skills_dir);
    let scanned = discovery::scan_path_impl(&tmp_path, "github", 0)?;
    let result = finalize_scanned_skills(scanned, &tmp_path, url, &existing);

    // Leak the TempDir handle so its Drop impl never runs and deletes the clone --
    // the returned paths must stay valid after this function returns.
    let _ = tmp.keep();

    Ok(result)
}

/// Turn raw `scan_path_impl` results into user-facing `ScannedSkillInfo`s: dedup by
/// name, clean up the display name/path, and mark what's already in the cookbook.
/// Pure/no I/O so it's directly testable without a real git clone.
///
/// `derive_skill_name` (used by `scan_path_impl`) falls back to the SKILL.md's
/// parent DIRECTORY name when the frontmatter has no `name:`/`title:` field. For a
/// local folder scan that fallback is meaningful (real folder names). For a GitHub
/// repo whose SKILL.md sits at the repo root -- so its "parent directory" is just
/// the freshly created tempdir -- that fallback produces the tempdir's own random
/// name (e.g. ".tmpXXXXXX"), which is meaningless to a user. Detect that exact case
/// and use the repo name instead. Also computes a repo-relative `display_path` so
/// the UI never has to show the raw absolute tempdir path.
fn finalize_scanned_skills(
    scanned: Vec<discovery::ScannedSkill>,
    tmp_path: &Path,
    url: &str,
    existing: &HashSet<String>,
) -> Vec<ScannedSkillInfo> {
    let tmp_basename = tmp_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let repo_name_fallback = repo_slug_from_url(url)
        .and_then(|slug| slug.split('/').next_back().map(|s| s.to_string()))
        .unwrap_or_else(|| "Imported Skill".to_string());

    let mut seen = HashSet::new();
    scanned
        .into_iter()
        .filter(|s| seen.insert(s.name.clone()))
        .map(|s| {
            let display_path = Path::new(&s.path)
                .strip_prefix(tmp_path)
                .ok()
                .map(|rel| rel.to_string_lossy().to_string());
            let name = if s.name == tmp_basename {
                repo_name_fallback.clone()
            } else {
                s.name
            };
            ScannedSkillInfo {
                already_in_chef: existing.contains(&name),
                name,
                path: s.path,
                source_agent: s.client_id,
                display_path,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_git_url_accepts_https_http_git_at() {
        assert!(validate_git_url("https://github.com/owner/repo").is_ok());
        assert!(validate_git_url("http://github.com/owner/repo").is_ok());
        assert!(validate_git_url("git@github.com:owner/repo.git").is_ok());
    }

    /// Regression test: a repo-root SKILL.md with no `name:`/`title:` frontmatter
    /// scans with `derive_skill_name`'s fallback == the tempdir's own random name
    /// (this is exactly what a fresh `tempfile::tempdir()` handle looks like). The
    /// display name/path shown to the user must never be that raw tempdir artifact.
    #[test]
    fn finalize_scanned_skills_replaces_tempdir_name_fallback_with_repo_name() {
        let tmp_path = PathBuf::from("/tmp/.tmpABC123");
        let skill_md_path = tmp_path.join("SKILL.md");
        let scanned = vec![discovery::ScannedSkill {
            path: skill_md_path.to_string_lossy().to_string(),
            name: ".tmpABC123".to_string(), // the fallback-to-parent-dir-name case
            identifier: skill_md_path.to_string_lossy().to_string(),
            client_id: "github".to_string(),
        }];

        let result = finalize_scanned_skills(
            scanned,
            &tmp_path,
            "https://github.com/alice/cool-skill",
            &HashSet::new(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].name, "cool-skill",
            "must fall back to the repo name, not the tempdir's random basename"
        );
        assert_eq!(
            result[0].display_path.as_deref(),
            Some("SKILL.md"),
            "display_path must be repo-relative, not the raw absolute tempdir path"
        );
    }

    /// A skill with a real, meaningful name (from frontmatter, or a real subfolder in
    /// the repo) must be left untouched -- the tempdir-name fallback must not
    /// over-trigger.
    #[test]
    fn finalize_scanned_skills_leaves_real_names_untouched() {
        let tmp_path = PathBuf::from("/tmp/.tmpXYZ789");
        let skill_md_path = tmp_path.join("skills").join("my-real-skill").join("SKILL.md");
        let scanned = vec![discovery::ScannedSkill {
            path: skill_md_path.to_string_lossy().to_string(),
            name: "my-real-skill".to_string(),
            identifier: skill_md_path.to_string_lossy().to_string(),
            client_id: "github".to_string(),
        }];

        let result = finalize_scanned_skills(
            scanned,
            &tmp_path,
            "https://github.com/alice/some-repo",
            &HashSet::new(),
        );

        assert_eq!(result[0].name, "my-real-skill");
        assert_eq!(
            result[0].display_path.as_deref(),
            Some("skills/my-real-skill/SKILL.md")
        );
    }

    #[test]
    fn validate_git_url_rejects_non_url_schemes() {
        assert!(validate_git_url("ftp://example.com/repo").is_err());
        assert!(validate_git_url("file:///etc/passwd").is_err());
        assert!(validate_git_url("just-some-text").is_err());
        assert!(validate_git_url("").is_err());
    }

    #[test]
    fn validate_git_url_rejects_leading_dash_flag_injection() {
        assert!(validate_git_url("--upload-pack=touch /tmp/pwned;").is_err());
        assert!(validate_git_url("-oProxyCommand=evil").is_err());
    }

    #[test]
    fn repo_slug_from_url_parses_https_and_ssh_forms() {
        assert_eq!(
            repo_slug_from_url("https://github.com/owner/repo"),
            Some("owner/repo".to_string())
        );
        assert_eq!(
            repo_slug_from_url("https://github.com/owner/repo.git"),
            Some("owner/repo".to_string())
        );
        assert_eq!(
            repo_slug_from_url("https://github.com/owner/repo/"),
            Some("owner/repo".to_string())
        );
        assert_eq!(
            repo_slug_from_url("git@github.com:owner/repo.git"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn repo_slug_from_url_returns_none_for_unparseable_input() {
        assert_eq!(repo_slug_from_url("not-a-url"), None);
        assert_eq!(repo_slug_from_url("https://github.com/onlyowner"), None);
    }
}
