//! Shared skill lifecycle operations: activate/deactivate/delete, multi-target
//! deploy (symlinking into IDE/LLM client skills dirs), and adoption of skills
//! that already live inside a client's skills dir.
//!
//! Skills live in `<chef_dir>/skills/<name>` when active and
//! `<chef_dir>/skills-inactive/<name>` when inactive. "Deploying" a skill to a
//! client means symlinking its folder into that client's skills directory;
//! deactivating a skill removes all such symlinks first.

use crate::error::{RelayError, Result};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

/// Known client ids that can receive deployed skill symlinks.
pub const TARGET_CLIENT_IDS: &[&str] = &[
    "claude_code",
    "cursor",
    "codex",
    "codeium",
    "windsurf",
    "aider",
    "zed",
];

/// Resolve the skills directory a given client id should have skills symlinked into.
///
/// Convention, not universally confirmed for most clients: each client gets a
/// `<home>/.<client>/skills` folder. Not all clients may actually read a "skills"
/// subfolder today — this is our best-guess placement, not a per-client detection.
///
/// Codex is the one exception with a confirmed answer: per official Codex docs
/// (developers.openai.com/codex/skills, verified directly, not convention-guessed),
/// Codex's user-level global skill scope is `$HOME/.agents/skills`, NOT
/// `~/.codex/skills`. The `CODEX_HOME`-set branch below is kept as a defensive
/// fallback (it's unconfirmed whether Codex actually honors `CODEX_HOME/skills` for
/// this, but changing that path isn't confirmed necessary and isn't worth the risk).
pub fn target_skills_dir(client_id: &str) -> Option<PathBuf> {
    if client_id == "codex" {
        if let Ok(codex_home) = std::env::var("CODEX_HOME") {
            if !codex_home.is_empty() {
                return Some(PathBuf::from(codex_home).join("skills"));
            }
        }
    }
    let home = dirs::home_dir()?;
    let sub = match client_id {
        "claude_code" => ".claude",
        "cursor" => ".cursor",
        "windsurf" => ".windsurf",
        "codeium" => ".codeium",
        "aider" => ".aider",
        "zed" => ".zed",
        // Confirmed via official docs: Codex's global scope is ~/.agents/skills.
        "codex" => ".agents",
        _ => return None,
    };
    Some(home.join(sub).join("skills"))
}

/// Unique link name: relative path from chef_skills_dir with path separators replaced by "-".
pub fn link_name_for_skill_dir(skill_dir: &Path, chef_skills_dir: &Path) -> String {
    let relative = skill_dir.strip_prefix(chef_skills_dir).unwrap_or(skill_dir);
    let s = relative.to_string_lossy();
    let sep = std::path::MAIN_SEPARATOR;
    s.replace(sep, "-")
}

/// Validate a candidate skill name (e.g. from SKILL.md frontmatter, which may come
/// from an untrusted GitHub repo) for safe use as a single directory component under
/// `chef_dir/skills`. Rejects anything containing a path separator or `..`, empty
/// strings, and names that are just `.`/`..`, so a malicious `name:` field can't
/// escape the skills directory.
pub fn sanitize_skill_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return None;
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return None;
    }
    Some(trimmed.to_string())
}

#[cfg(unix)]
pub(crate) fn symlink_dir(src: &Path, dst: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
pub(crate) fn symlink_dir(src: &Path, dst: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

/// Remove a path that may be a file, symlink (including dangling), or directory.
fn remove_any(path: &Path) -> io::Result<()> {
    std::fs::remove_file(path).or_else(|e| {
        if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            Err(e)
        }
    })
}

/// Which of TARGET_CLIENT_IDS currently have a deploy symlink (or dangling symlink)
/// for this skill dir. Uses symlink_metadata so broken symlinks still count.
pub fn deployed_targets(skill_dir: &Path, chef_skills_dir: &Path) -> Vec<String> {
    let link_name = link_name_for_skill_dir(skill_dir, chef_skills_dir);
    let mut result = Vec::new();
    for &client_id in TARGET_CLIENT_IDS {
        let Some(target_dir) = target_skills_dir(client_id) else {
            continue;
        };
        if !target_dir.exists() {
            continue;
        }
        let link_path = target_dir.join(&link_name);
        if std::fs::symlink_metadata(&link_path).is_ok() {
            result.push(client_id.to_string());
        }
    }
    result
}

/// Most recent modification time across every file in `dir`, recursively (an
/// iterative walk, not recursive calls, so it can't stack-overflow on a deep tree).
/// This is the ground truth for "last updated" -- unlike a DB-tracked timestamp, it
/// can't go stale: it reflects ANY edit to ANY file in the skill folder, including
/// ones made outside this app (a text editor, `git pull`, another tool). Returns
/// `None` if the directory can't be read or contains no files -- callers should fall
/// back to their own tracked timestamp in that case.
pub fn folder_last_modified(dir: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    let mut latest: Option<std::time::SystemTime> = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(modified) = entry.metadata().and_then(|m| m.modified()) {
                latest = Some(match latest {
                    Some(cur) if cur >= modified => cur,
                    _ => modified,
                });
            }
        }
    }
    latest.map(chrono::DateTime::<chrono::Utc>::from)
}

/// Reconcile actual symlinks across every target dir to match `desired` client ids:
/// create missing ones, remove any not in `desired`.
pub fn set_skill_targets(
    skill_dir: &Path,
    chef_skills_dir: &Path,
    desired: &[String],
) -> Result<()> {
    let link_name = link_name_for_skill_dir(skill_dir, chef_skills_dir);
    for &client_id in TARGET_CLIENT_IDS {
        let Some(target_dir) = target_skills_dir(client_id) else {
            continue;
        };
        let want = desired.iter().any(|d| d == client_id);
        let link_path = target_dir.join(&link_name);
        let exists = std::fs::symlink_metadata(&link_path).is_ok();

        if want {
            if exists {
                continue;
            }
            std::fs::create_dir_all(&target_dir).map_err(|e| {
                RelayError::Internal(format!("create {}: {}", target_dir.display(), e))
            })?;
            symlink_dir(skill_dir, &link_path).map_err(|e| {
                RelayError::Internal(format!(
                    "symlink {} -> {}: {}",
                    skill_dir.display(),
                    link_path.display(),
                    e
                ))
            })?;
        } else if exists {
            remove_any(&link_path).map_err(|e| {
                RelayError::Internal(format!("remove {}: {}", link_path.display(), e))
            })?;
        }
    }
    Ok(())
}

/// Move `<chef_dir>/skills-inactive/<name>` to `<chef_dir>/skills/<name>` and commit.
pub fn activate_skill(chef_dir: &Path, name: &str) -> Result<PathBuf> {
    let src = chef_dir.join("skills-inactive").join(name);
    let dest = chef_dir.join("skills").join(name);
    if !src.exists() {
        return Err(RelayError::Config(format!(
            "Skill '{}' not found in skills-inactive",
            name
        )));
    }
    if dest.exists() {
        return Err(RelayError::Config(format!(
            "Destination already exists: {}",
            dest.display()
        )));
    }
    std::fs::create_dir_all(dest.parent().unwrap_or(chef_dir))
        .map_err(|e| RelayError::Internal(format!("create skills dir: {}", e)))?;
    std::fs::rename(&src, &dest).map_err(|e| {
        RelayError::Internal(format!(
            "move {} to {}: {}",
            src.display(),
            dest.display(),
            e
        ))
    })?;
    git_add_commit(chef_dir, &format!("Activate skill: {}", name))?;
    Ok(dest)
}

/// Strip all deployed symlinks, then move `<chef_dir>/skills/<name>` to `skills-inactive` and commit.
pub fn deactivate_skill(chef_dir: &Path, name: &str) -> Result<PathBuf> {
    let active_dir = chef_dir.join("skills").join(name);
    if !active_dir.exists() {
        return Err(RelayError::Config(format!(
            "Skill '{}' not found in skills",
            name
        )));
    }
    let chef_skills_dir = chef_dir.join("skills");
    set_skill_targets(&active_dir, &chef_skills_dir, &[])?;

    let dest = chef_dir.join("skills-inactive").join(name);
    if dest.exists() {
        return Err(RelayError::Config(format!(
            "Destination already exists: {}",
            dest.display()
        )));
    }
    std::fs::create_dir_all(dest.parent().unwrap_or(chef_dir))
        .map_err(|e| RelayError::Internal(format!("create skills-inactive dir: {}", e)))?;
    std::fs::rename(&active_dir, &dest).map_err(|e| {
        RelayError::Internal(format!(
            "move {} to {}: {}",
            active_dir.display(),
            dest.display(),
            e
        ))
    })?;
    git_add_commit(chef_dir, &format!("Deactivate skill: {}", name))?;
    Ok(dest)
}

/// Find and remove a skill from either skills/ or skills-inactive/, stripping deployed symlinks first.
pub fn delete_skill(chef_dir: &Path, name: &str) -> Result<()> {
    let active_dir = chef_dir.join("skills").join(name);
    let inactive_dir = chef_dir.join("skills-inactive").join(name);

    let target = if active_dir.exists() {
        active_dir
    } else if inactive_dir.exists() {
        inactive_dir
    } else {
        return Err(RelayError::Config(format!(
            "Skill '{}' not found in skills or skills-inactive",
            name
        )));
    };

    let chef_skills_dir = chef_dir.join("skills");
    set_skill_targets(&target, &chef_skills_dir, &[])?;

    std::fs::remove_dir_all(&target)
        .map_err(|e| RelayError::Internal(format!("remove {}: {}", target.display(), e)))?;
    git_add_commit(chef_dir, &format!("Delete skill: {}", name))?;
    Ok(())
}

/// Plain recursive directory copy (no symlink handling — copies whatever std::fs::read_dir reports).
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)
        .map_err(|e| RelayError::Internal(format!("create {}: {}", dst.display(), e)))?;
    for entry in std::fs::read_dir(src)
        .map_err(|e| RelayError::Internal(format!("read {}: {}", src.display(), e)))?
    {
        let entry = entry.map_err(|e| RelayError::Internal(e.to_string()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                RelayError::Internal(format!(
                    "copy {} -> {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                ))
            })?;
        }
    }
    Ok(())
}

/// Target ids for `publish_skill_to_project`. Deliberately distinct from
/// `TARGET_CLIENT_IDS`: this is a different mechanism (real copies committed into a
/// third-party git repo for cloud/portable access) and must never be conflated with
/// the machine-global symlink deploy/deactivate/adopt logic above.
pub const PROJECT_PUBLISH_TARGET_IDS: &[&str] = &["claude_code_project", "codex_project"];

/// Copy a skill folder into a target project repo's convention-defined skills path so
/// cloud/web-hosted agent sessions (which only see what's checked into the repo they
/// clone) can discover it. This is always a REAL copy, never a symlink — a symlink
/// pointing back outside the target repo would be dangling once the repo is cloned
/// elsewhere or run in a remote sandbox.
///
/// `targets` is a subset of `PROJECT_PUBLISH_TARGET_IDS`. Unknown ids are skipped.
/// Destinations:
///   - "claude_code_project" -> `<project_root>/.claude/skills/<skill_name>`
///     (per code.claude.com/docs/en/skills: project skills load from `.claude/skills/`
///     at the repo root, and this applies across terminal, IDE, desktop app, web, iOS,
///     and Slack).
///   - "codex_project" -> `<project_root>/.agents/skills/<skill_name>`
///     (per developers.openai.com/codex/skills: repository scope, scanned from cwd up
///     to the repo root. Confirmed for local Codex; Codex cloud tasks clone the repo
///     and check out the branch before running, and use the same walk-up-to-repo-root
///     discovery mechanism, so it is very likely — though not spelled out in a single
///     explicit doc sentence for the cloud case specifically — that cloud sessions see
///     `.agents/skills` the same way local Codex does, since it is purely a function of
///     what's checked into the cloned repo).
///
/// If a destination already exists it is removed and recopied fresh, so republishing an
/// updated skill actually updates the copy (stale files never linger alongside new
/// ones). Returns the target ids that were successfully written. Does NOT touch git in
/// the target project — the files are left in the working tree for the user's own
/// commit workflow.
pub fn publish_skill_to_project(
    skill_dir: &Path,
    project_root: &Path,
    targets: &[String],
) -> Result<Vec<String>> {
    let skill_name = skill_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            RelayError::Internal(format!("invalid skill dir: {}", skill_dir.display()))
        })?;

    let mut written = Vec::new();
    for target in targets {
        if !PROJECT_PUBLISH_TARGET_IDS.contains(&target.as_str()) {
            continue;
        }
        let sub = match target.as_str() {
            "claude_code_project" => ".claude",
            "codex_project" => ".agents",
            _ => continue,
        };
        let dest = project_root.join(sub).join("skills").join(skill_name);

        if dest.exists() {
            std::fs::remove_dir_all(&dest)
                .map_err(|e| RelayError::Internal(format!("remove {}: {}", dest.display(), e)))?;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RelayError::Internal(format!("create {}: {}", parent.display(), e)))?;
        }
        copy_dir_all(skill_dir, &dest)?;
        written.push(target.clone());
    }
    Ok(written)
}

/// One skill discovered inside a project repo's published-skills locations, and which
/// of `PROJECT_PUBLISH_TARGET_IDS` it's currently present in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSkillEntry {
    pub name: String,
    pub targets: Vec<String>,
}

/// Map a `PROJECT_PUBLISH_TARGET_IDS` entry to the project-relative skills dir it uses.
/// Mirrors the destination logic in `publish_skill_to_project`.
fn project_target_skills_dir(project_root: &Path, target: &str) -> Option<PathBuf> {
    let sub = match target {
        "claude_code_project" => ".claude",
        "codex_project" => ".agents",
        _ => return None,
    };
    Some(project_root.join(sub).join("skills"))
}

/// List every skill published into `project_root` via `publish_skill_to_project`, across
/// both `.claude/skills/` and `.agents/skills/`, reporting which target(s) each name is
/// present in. Only top-level subdirectories containing a `SKILL.md` file count — this
/// mirrors the flat convention `publish_skill_to_project` writes with, so no recursion.
/// A missing skills dir (project has neither, or just one) is treated as "no skills
/// there", not an error. Results are sorted by name.
pub fn list_project_skills(project_root: &Path) -> Result<Vec<ProjectSkillEntry>> {
    use std::collections::BTreeMap;

    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for &target in PROJECT_PUBLISH_TARGET_IDS {
        let Some(dir) = project_target_skills_dir(project_root, target) else {
            continue;
        };
        if !dir.exists() {
            continue;
        }
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| RelayError::Internal(format!("read {}: {}", dir.display(), e)))?;
        for entry in entries {
            let entry = entry.map_err(|e| RelayError::Internal(e.to_string()))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if !path.join("SKILL.md").exists() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            by_name.entry(name).or_default().push(target.to_string());
        }
    }

    Ok(by_name
        .into_iter()
        .map(|(name, targets)| ProjectSkillEntry { name, targets })
        .collect())
}

/// Remove a skill previously published into `project_root` via `publish_skill_to_project`,
/// for each requested target id. Only removes files from the working tree -- like
/// `publish_skill_to_project`, this does NOT touch git in the target project, leaving any
/// commit/discard decision to the user. If a target's directory is already absent, that
/// target is skipped (a no-op, not an error) and is NOT included in the returned list --
/// the return value reflects only targets whose directory actually existed and was
/// deleted by this call. Unrecognized target ids are skipped too.
pub fn unpublish_skill_from_project(
    project_root: &Path,
    name: &str,
    targets: &[String],
) -> Result<Vec<String>> {
    let mut removed = Vec::new();
    for target in targets {
        let Some(dir) = project_target_skills_dir(project_root, target) else {
            continue;
        };
        let dest = dir.join(name);
        if dest.exists() {
            std::fs::remove_dir_all(&dest)
                .map_err(|e| RelayError::Internal(format!("remove {}: {}", dest.display(), e)))?;
            removed.push(target.clone());
        }
    }
    Ok(removed)
}

/// A skill folder found sitting directly inside a client's skills dir, not yet adopted into chef.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoptCandidate {
    pub name: String,
    pub path: String,
}

/// List real (non-symlink) subdirectories of a client's skills dir that contain SKILL.md.
pub fn adopt_scan(client_id: &str) -> Result<Vec<AdoptCandidate>> {
    let Some(target_dir) = target_skills_dir(client_id) else {
        return Ok(vec![]);
    };
    if !target_dir.exists() {
        return Ok(vec![]);
    }
    let mut result = Vec::new();
    for entry in std::fs::read_dir(&target_dir)
        .map_err(|e| RelayError::Internal(format!("read {}: {}", target_dir.display(), e)))?
    {
        let entry = entry.map_err(|e| RelayError::Internal(e.to_string()))?;
        // entry.file_type() is symlink_metadata-based (does not follow links), so a
        // dangling symlink reports is_symlink() = true without erroring — unlike
        // entry.metadata(), which follows the link and would fail on a dangling one.
        let file_type = entry
            .file_type()
            .map_err(|e| RelayError::Internal(e.to_string()))?;
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let path = entry.path();
        if path.join("SKILL.md").exists() {
            result.push(AdoptCandidate {
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
            });
        }
    }
    Ok(result)
}

/// Move requested skill folders from a client's skills dir into chef, then symlink back.
/// Skips (silently) any name that isn't a real dir with SKILL.md, or would clobber an
/// existing chef skill. Returns the names actually adopted.
pub fn adopt_apply(chef_dir: &Path, client_id: &str, names: &[String]) -> Result<Vec<String>> {
    let Some(target_dir) = target_skills_dir(client_id) else {
        return Ok(vec![]);
    };
    let chef_skills_dir = chef_dir.join("skills");
    let mut adopted = Vec::new();

    for name in names {
        let source = target_dir.join(name);
        let file_type = match std::fs::symlink_metadata(&source) {
            Ok(m) => m.file_type(),
            Err(_) => continue,
        };
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        if !source.join("SKILL.md").exists() {
            continue;
        }
        let dest = chef_skills_dir.join(name);
        if dest.exists() {
            continue;
        }

        std::fs::create_dir_all(&chef_skills_dir).map_err(|e| {
            RelayError::Internal(format!("create {}: {}", chef_skills_dir.display(), e))
        })?;
        std::fs::rename(&source, &dest).map_err(|e| {
            RelayError::Internal(format!(
                "move {} to {}: {}",
                source.display(),
                dest.display(),
                e
            ))
        })?;
        symlink_dir(&dest, &source).map_err(|e| {
            RelayError::Internal(format!(
                "symlink {} -> {}: {}",
                dest.display(),
                source.display(),
                e
            ))
        })?;
        adopted.push(name.clone());
    }

    if !adopted.is_empty() {
        git_add_commit(
            chef_dir,
            &format!("Adopt {} skill(s) from {}", adopted.len(), client_id),
        )?;
    }
    Ok(adopted)
}

/// Canonical `git add -A && git commit -m <message>` helper. Only errors if the git
/// process fails to spawn; a non-zero exit (e.g. nothing to commit, or chef_dir isn't
/// a git repo) is not treated as an error.
pub fn git_add_commit(repo_root: &Path, message: &str) -> Result<()> {
    let _ = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo_root)
        .status()
        .map_err(|e| RelayError::Internal(format!("git add: {}", e)))?;
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo_root)
        .status()
        .map_err(|e| RelayError::Internal(format!("git commit: {}", e)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::fs;

    #[test]
    fn sanitize_skill_name_accepts_plain_names() {
        assert_eq!(
            sanitize_skill_name("fable-orchestration"),
            Some("fable-orchestration".to_string())
        );
        assert_eq!(
            sanitize_skill_name("  spaced-name  "),
            Some("spaced-name".to_string())
        );
    }

    #[test]
    fn sanitize_skill_name_rejects_path_traversal_and_empty() {
        assert_eq!(sanitize_skill_name("../../etc/passwd"), None);
        assert_eq!(sanitize_skill_name("foo/bar"), None);
        assert_eq!(sanitize_skill_name("foo\\bar"), None);
        assert_eq!(sanitize_skill_name(".."), None);
        assert_eq!(sanitize_skill_name("."), None);
        assert_eq!(sanitize_skill_name(""), None);
        assert_eq!(sanitize_skill_name("   "), None);
    }

    #[test]
    fn folder_last_modified_finds_the_max_mtime_across_nested_files() {
        use std::time::{Duration, SystemTime};

        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path();
        fs::create_dir_all(skill_dir.join("scripts")).unwrap();

        let older = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let newer = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);

        let skill_md = skill_dir.join("SKILL.md");
        fs::write(&skill_md, "# hi").unwrap();
        fs::File::options()
            .write(true)
            .open(&skill_md)
            .unwrap()
            .set_modified(older)
            .unwrap();

        // A nested file, modified LATER -- must win even though it's deeper in the
        // tree and alphabetically after SKILL.md.
        let nested = skill_dir.join("scripts").join("run.py");
        fs::write(&nested, "print('hi')").unwrap();
        fs::File::options()
            .write(true)
            .open(&nested)
            .unwrap()
            .set_modified(newer)
            .unwrap();

        let result = folder_last_modified(skill_dir).expect("must find a timestamp");
        let expected: chrono::DateTime<chrono::Utc> = newer.into();
        assert_eq!(result, expected);
    }

    #[test]
    fn folder_last_modified_returns_none_for_empty_or_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        // Empty dir: no files at all.
        assert!(folder_last_modified(tmp.path()).is_none());
        // Missing dir entirely.
        assert!(folder_last_modified(&tmp.path().join("does-not-exist")).is_none());
    }

    /// Point dirs::home_dir() (which reads $HOME on unix) at a tempdir, and make sure
    /// CODEX_HOME doesn't leak in from the real environment.
    fn with_fake_home() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        env::set_var("HOME", tmp.path());
        env::remove_var("CODEX_HOME");
        tmp
    }

    fn init_git_repo(dir: &Path) {
        std::process::Command::new("git")
            .arg("init")
            .current_dir(dir)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .status()
            .unwrap();
    }

    #[test]
    #[serial]
    fn activate_deactivate_round_trip() {
        let _home = with_fake_home();
        let chef = tempfile::tempdir().unwrap();
        init_git_repo(chef.path());

        let inactive = chef.path().join("skills-inactive").join("foo");
        fs::create_dir_all(&inactive).unwrap();
        fs::write(inactive.join("SKILL.md"), "# Foo").unwrap();
        git_add_commit(chef.path(), "seed").unwrap();

        let active_path = activate_skill(chef.path(), "foo").unwrap();
        assert!(active_path.exists());
        assert!(!inactive.exists());
        assert_eq!(active_path, chef.path().join("skills").join("foo"));

        let inactive_path = deactivate_skill(chef.path(), "foo").unwrap();
        assert!(inactive_path.exists());
        assert!(!active_path.exists());
    }

    #[test]
    #[serial]
    fn set_skill_targets_creates_then_removes_symlink() {
        let _home = with_fake_home();
        let chef = tempfile::tempdir().unwrap();
        let chef_skills_dir = chef.path().join("skills");
        let skill_dir = chef_skills_dir.join("foo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Foo").unwrap();

        set_skill_targets(&skill_dir, &chef_skills_dir, &["claude_code".to_string()]).unwrap();
        let link_name = link_name_for_skill_dir(&skill_dir, &chef_skills_dir);
        let link_path = target_skills_dir("claude_code").unwrap().join(&link_name);
        assert!(std::fs::symlink_metadata(&link_path).is_ok());
        assert_eq!(
            deployed_targets(&skill_dir, &chef_skills_dir),
            vec!["claude_code".to_string()]
        );

        set_skill_targets(&skill_dir, &chef_skills_dir, &[]).unwrap();
        assert!(std::fs::symlink_metadata(&link_path).is_err());
        assert!(deployed_targets(&skill_dir, &chef_skills_dir).is_empty());
    }

    #[test]
    #[serial]
    fn delete_skill_removes_dir() {
        let _home = with_fake_home();
        let chef = tempfile::tempdir().unwrap();
        init_git_repo(chef.path());
        let skill_dir = chef.path().join("skills").join("foo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Foo").unwrap();
        git_add_commit(chef.path(), "seed").unwrap();

        delete_skill(chef.path(), "foo").unwrap();
        assert!(!skill_dir.exists());
    }

    #[test]
    fn copy_dir_all_copies_nested_files() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        fs::write(src.path().join("SKILL.md"), "# Foo").unwrap();
        fs::create_dir_all(src.path().join("references")).unwrap();
        fs::write(src.path().join("references").join("notes.md"), "notes").unwrap();
        fs::create_dir_all(src.path().join("scripts").join("nested")).unwrap();
        fs::write(
            src.path().join("scripts").join("nested").join("run.sh"),
            "echo hi",
        )
        .unwrap();

        let dest_dir = dst.path().join("foo");
        copy_dir_all(src.path(), &dest_dir).unwrap();

        assert!(dest_dir.join("SKILL.md").exists());
        assert!(dest_dir.join("references").join("notes.md").exists());
        assert!(dest_dir
            .join("scripts")
            .join("nested")
            .join("run.sh")
            .exists());
    }

    #[test]
    #[serial]
    fn adopt_scan_finds_real_dir_skips_symlink() {
        let _home = with_fake_home();
        let target_dir = target_skills_dir("claude_code").unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        let real = target_dir.join("real-skill");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("SKILL.md"), "# Real").unwrap();

        // A directory without SKILL.md should be ignored.
        fs::create_dir_all(target_dir.join("not-a-skill")).unwrap();

        // A symlinked skill (simulating an already-deployed one) should be skipped.
        let other = tempfile::tempdir().unwrap();
        fs::create_dir_all(other.path().join("linked-skill")).unwrap();
        fs::write(
            other.path().join("linked-skill").join("SKILL.md"),
            "# Linked",
        )
        .unwrap();
        symlink_dir(
            &other.path().join("linked-skill"),
            &target_dir.join("linked-skill"),
        )
        .unwrap();

        // A dangling symlink (target removed after deploy-then-delete) must be skipped
        // without erroring the whole scan.
        let ghost_target = other.path().join("ghost-skill");
        fs::create_dir_all(&ghost_target).unwrap();
        symlink_dir(&ghost_target, &target_dir.join("ghost-skill")).unwrap();
        fs::remove_dir_all(&ghost_target).unwrap();

        let candidates = adopt_scan("claude_code").unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "real-skill");
    }

    #[test]
    fn publish_skill_to_project_writes_both_targets() {
        let skill = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        // publish_skill_to_project uses skill_dir.file_name() as the skill name, so
        // the source dir must itself be named "foo".
        let skill_dir = skill.path().join("foo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Foo").unwrap();
        fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        fs::write(skill_dir.join("scripts").join("run.sh"), "echo hi").unwrap();

        let written = publish_skill_to_project(
            &skill_dir,
            project.path(),
            &[
                "claude_code_project".to_string(),
                "codex_project".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(
            written,
            vec![
                "claude_code_project".to_string(),
                "codex_project".to_string()
            ]
        );

        let claude_dest = project.path().join(".claude").join("skills").join("foo");
        let codex_dest = project.path().join(".agents").join("skills").join("foo");
        assert_eq!(
            fs::read_to_string(claude_dest.join("SKILL.md")).unwrap(),
            "# Foo"
        );
        assert!(claude_dest.join("scripts").join("run.sh").exists());
        assert_eq!(
            fs::read_to_string(codex_dest.join("SKILL.md")).unwrap(),
            "# Foo"
        );
        assert!(codex_dest.join("scripts").join("run.sh").exists());

        // Neither destination should be a symlink.
        assert!(!std::fs::symlink_metadata(&claude_dest)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(!std::fs::symlink_metadata(&codex_dest)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn publish_skill_to_project_republish_overwrites_stale_content() {
        let project = tempfile::tempdir().unwrap();
        let skill_dir = project.path().join("_src").join("foo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# v1").unwrap();
        fs::write(skill_dir.join("old-file.md"), "stale").unwrap();

        publish_skill_to_project(
            &skill_dir,
            project.path(),
            &["claude_code_project".to_string()],
        )
        .unwrap();

        // Edit the source: drop old-file.md, update SKILL.md.
        fs::remove_file(skill_dir.join("old-file.md")).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# v2").unwrap();

        publish_skill_to_project(
            &skill_dir,
            project.path(),
            &["claude_code_project".to_string()],
        )
        .unwrap();

        let dest = project.path().join(".claude").join("skills").join("foo");
        assert_eq!(fs::read_to_string(dest.join("SKILL.md")).unwrap(), "# v2");
        assert!(
            !dest.join("old-file.md").exists(),
            "stale file from first publish must not survive republish"
        );
    }

    #[test]
    fn list_project_skills_reports_per_skill_targets() {
        let project = tempfile::tempdir().unwrap();

        // "both" published to both claude and codex.
        for sub in [".claude", ".agents"] {
            let dir = project.path().join(sub).join("skills").join("both");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("SKILL.md"), "# Both").unwrap();
        }
        // "claude-only" published to claude only.
        let claude_only_dir = project
            .path()
            .join(".claude")
            .join("skills")
            .join("claude-only");
        fs::create_dir_all(&claude_only_dir).unwrap();
        fs::write(claude_only_dir.join("SKILL.md"), "# Claude only").unwrap();
        // "codex-only" published to codex only.
        let codex_only_dir = project
            .path()
            .join(".agents")
            .join("skills")
            .join("codex-only");
        fs::create_dir_all(&codex_only_dir).unwrap();
        fs::write(codex_only_dir.join("SKILL.md"), "# Codex only").unwrap();
        // A directory without SKILL.md must be ignored entirely.
        fs::create_dir_all(
            project
                .path()
                .join(".claude")
                .join("skills")
                .join("not-a-skill"),
        )
        .unwrap();

        let entries = list_project_skills(project.path()).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["both", "claude-only", "codex-only"]);

        let both = entries.iter().find(|e| e.name == "both").unwrap();
        assert_eq!(
            both.targets,
            vec![
                "claude_code_project".to_string(),
                "codex_project".to_string()
            ]
        );
        let claude_only = entries.iter().find(|e| e.name == "claude-only").unwrap();
        assert_eq!(claude_only.targets, vec!["claude_code_project".to_string()]);
        let codex_only = entries.iter().find(|e| e.name == "codex-only").unwrap();
        assert_eq!(codex_only.targets, vec!["codex_project".to_string()]);
    }

    #[test]
    fn list_project_skills_empty_for_missing_project_dirs() {
        let project = tempfile::tempdir().unwrap();
        // Neither .claude/skills nor .agents/skills exists.
        let entries = list_project_skills(project.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn unpublish_skill_from_project_removes_dir_and_is_idempotent() {
        let skill = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let skill_dir = skill.path().join("foo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Foo").unwrap();

        publish_skill_to_project(
            &skill_dir,
            project.path(),
            &[
                "claude_code_project".to_string(),
                "codex_project".to_string(),
            ],
        )
        .unwrap();

        let claude_dest = project.path().join(".claude").join("skills").join("foo");
        let codex_dest = project.path().join(".agents").join("skills").join("foo");
        assert!(claude_dest.exists());
        assert!(codex_dest.exists());

        let removed = unpublish_skill_from_project(
            project.path(),
            "foo",
            &["claude_code_project".to_string()],
        )
        .unwrap();
        assert_eq!(removed, vec!["claude_code_project".to_string()]);
        assert!(!claude_dest.exists());
        assert!(codex_dest.exists(), "codex target must be untouched");

        // Calling again on the already-removed target must be a safe no-op, not an
        // error -- and since nothing was actually removed this time, the returned list
        // must be empty (not re-report a target whose dir was already gone).
        let removed_again = unpublish_skill_from_project(
            project.path(),
            "foo",
            &["claude_code_project".to_string()],
        )
        .unwrap();
        assert!(removed_again.is_empty());
        assert!(!claude_dest.exists());

        // Clean up the remaining codex target too.
        let removed_codex =
            unpublish_skill_from_project(project.path(), "foo", &["codex_project".to_string()])
                .unwrap();
        assert_eq!(removed_codex, vec!["codex_project".to_string()]);
        assert!(!codex_dest.exists());
    }

    #[test]
    #[serial]
    fn target_skills_dir_codex_defaults_to_dot_agents() {
        let _home = with_fake_home();
        let dir = target_skills_dir("codex").unwrap();
        assert_eq!(
            dir,
            dirs::home_dir().unwrap().join(".agents").join("skills")
        );
        assert!(dir.ends_with(".agents/skills"));
    }

    #[test]
    #[serial]
    fn adopt_apply_moves_and_relinks() {
        let _home = with_fake_home();
        let chef = tempfile::tempdir().unwrap();
        init_git_repo(chef.path());

        let target_dir = target_skills_dir("claude_code").unwrap();
        fs::create_dir_all(&target_dir).unwrap();
        let source = target_dir.join("adopt-me");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Adopt me").unwrap();

        let adopted = adopt_apply(chef.path(), "claude_code", &["adopt-me".to_string()]).unwrap();
        assert_eq!(adopted, vec!["adopt-me".to_string()]);

        let new_home = chef.path().join("skills").join("adopt-me");
        assert!(new_home.join("SKILL.md").exists());
        assert!(std::fs::symlink_metadata(&source).is_ok());
        assert_eq!(std::fs::read_link(&source).unwrap(), new_home);
    }
}
