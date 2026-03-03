//! Search command: scan directories for SKILL.md files and add selected ones to chef.

use crate::chef::git_add_commit;
use skill_cookbook_relay::config::Config;
use skill_cookbook_relay::discovery::{discover_apps_impl, scan_path_impl, ScannedSkill};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(clap::Args, Debug)]
pub struct SearchArgs {
    /// Directory to scan. If omitted, scans detected AI agent folders.
    pub path: Option<PathBuf>,

    /// Select all found skills without prompting.
    #[arg(long)]
    pub all: bool,
}

pub fn run_search(args: &SearchArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load().map_err(|e| e.to_string())?;

    let skills = if let Some(ref path) = args.path {
        if !path.is_dir() {
            return Err(format!("Not a directory: {}", path.display()).into());
        }
        println!("Scanning {}...", path.display());
        scan_path_impl(path, "search", 0).map_err(|e| e.to_string())?
    } else {
        scan_agent_folders()?
    };

    if skills.is_empty() {
        println!("No SKILL.md files found.");
        return Ok(());
    }

    // Dedup by skill name — keep first occurrence when multiple agents have the same skill.
    let skills = dedup_by_name(skills);

    let chef_skills_dir = config.chef_dir.join("skills");
    let existing = existing_skill_names(&chef_skills_dir);

    let labels: Vec<String> = skills
        .iter()
        .map(|s| {
            let rel = shorten_path(&s.path);
            if existing.contains(&s.name) {
                format!("{} ({}) [already in chef]", s.name, rel)
            } else {
                format!("{} ({})", s.name, rel)
            }
        })
        .collect();

    let selected_indices = if args.all {
        // With --all, skip skills that already exist in chef.
        let indices: Vec<_> = (0..skills.len())
            .filter(|&i| !existing.contains(&skills[i].name))
            .collect();
        if indices.len() < skills.len() {
            let skipped = skills.len() - indices.len();
            println!("Skipping {} skill(s) already in chef.", skipped);
        }
        indices
    } else {
        prompt_skill_selection(&labels)?
    };

    if selected_indices.is_empty() {
        println!("No skills selected.");
        return Ok(());
    }

    let mut added = Vec::new();
    for &i in &selected_indices {
        let skill = &skills[i];
        let skill_md = Path::new(&skill.path);
        let src_dir = match skill_md.parent() {
            Some(d) => d,
            None => continue,
        };
        let dest_dir = chef_skills_dir.join(&skill.name);
        copy_dir_recursive(src_dir, &dest_dir)?;
        added.push(skill.name.clone());
        println!("  Copied: {}", skill.name);
    }

    if !added.is_empty() {
        let msg = build_commit_message(&added);
        git_add_commit(&config.chef_dir, &msg)?;
    }

    println!("Done. {} skill(s) added to chef.", added.len());
    Ok(())
}

fn build_commit_message(added: &[String]) -> String {
    match added.len() {
        1 => format!("Add skill: {}", added[0]),
        n if n <= 5 => format!("Add {} skills: {}", n, added.join(", ")),
        n => {
            let shown: Vec<_> = added.iter().take(5).map(|s| s.as_str()).collect();
            format!("Add {} skills: {}, ...and {} more", n, shown.join(", "), n - 5)
        }
    }
}

fn dedup_by_name(skills: Vec<ScannedSkill>) -> Vec<ScannedSkill> {
    let mut seen = HashSet::new();
    skills
        .into_iter()
        .filter(|s| seen.insert(s.name.clone()))
        .collect()
}

fn scan_agent_folders() -> Result<Vec<ScannedSkill>, Box<dyn std::error::Error + Send + Sync>> {
    let apps = discover_apps_impl();
    let detected: Vec<_> = apps.clients.iter().filter(|c| c.detected).collect();

    if detected.is_empty() {
        println!("No AI agent folders detected.");
        return Ok(vec![]);
    }

    println!("Detected agents:");
    for c in &detected {
        println!(
            "  {} ({})",
            c.name,
            c.path.as_deref().unwrap_or("unknown")
        );
    }
    println!();

    let mut all_skills = Vec::new();
    for c in &detected {
        if let Some(ref p) = c.path {
            println!("Scanning {}...", c.name);
            let path = Path::new(p.as_str());
            match scan_path_impl(path, &c.id, 0) {
                Ok(skills) => {
                    println!("  Found {} skill(s).", skills.len());
                    all_skills.extend(skills);
                }
                Err(e) => eprintln!("  Warning: failed to scan {}: {}", p, e),
            }
        }
    }
    Ok(all_skills)
}

fn prompt_skill_selection(
    labels: &[String],
) -> Result<Vec<usize>, Box<dyn std::error::Error + Send + Sync>> {
    let selection = dialoguer::MultiSelect::new()
        .with_prompt("Select skills to add (space to toggle, enter to confirm)")
        .items(labels)
        .interact()
        .map_err(|e| format!("prompt error: {}", e))?;
    Ok(selection)
}

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

fn copy_dir_recursive(
    src: &Path,
    dst: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::fs::create_dir_all(dst).map_err(|e| format!("create dir {}: {}", dst.display(), e))?;
    for entry in
        std::fs::read_dir(src).map_err(|e| format!("read dir {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let ft = entry
            .file_type()
            .map_err(|e| format!("file_type {}: {}", entry.path().display(), e))?;
        // Skip symlinks to avoid infinite recursion and path traversal.
        if ft.is_symlink() {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "copy {} -> {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

fn shorten_path(path: &str) -> String {
    let home = dirs::home_dir().map(|h| h.to_string_lossy().to_string());
    match home {
        Some(h) if path.starts_with(&h) => format!("~{}", &path[h.len()..]),
        _ => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn existing_skill_names_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let names = existing_skill_names(tmp.path());
        assert!(names.is_empty());
    }

    #[test]
    fn existing_skill_names_with_skills() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("my-skill")).unwrap();
        fs::create_dir(tmp.path().join("other-skill")).unwrap();
        // A file should be ignored (not a directory).
        fs::write(tmp.path().join("not-a-skill.txt"), "").unwrap();

        let names = existing_skill_names(tmp.path());
        assert_eq!(names.len(), 2);
        assert!(names.contains("my-skill"));
        assert!(names.contains("other-skill"));
    }

    #[test]
    fn existing_skill_names_missing_dir() {
        let names = existing_skill_names(Path::new("/nonexistent/path"));
        assert!(names.is_empty());
    }

    #[test]
    fn copy_dir_recursive_copies_files_and_subdirs() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        // Build a source tree: src/a.txt, src/sub/b.txt
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("SKILL.md"), "# test skill").unwrap();
        fs::write(src.join("sub").join("helper.md"), "helper").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("SKILL.md").exists());
        assert_eq!(fs::read_to_string(dst.join("SKILL.md")).unwrap(), "# test skill");
        assert!(dst.join("sub").join("helper.md").exists());
        assert_eq!(
            fs::read_to_string(dst.join("sub").join("helper.md")).unwrap(),
            "helper"
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_dir_recursive_skips_symlinks() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("real.txt"), "content").unwrap();
        // Symlink pointing back to src (would cause infinite recursion if followed).
        std::os::unix::fs::symlink(&src, src.join("loop")).unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("real.txt").exists());
        // The symlink should have been skipped.
        assert!(!dst.join("loop").exists());
    }

    #[test]
    fn shorten_path_replaces_home() {
        if let Some(home) = dirs::home_dir() {
            let full = format!("{}/some/path", home.display());
            let short = shorten_path(&full);
            assert_eq!(short, "~/some/path");
        }
    }

    #[test]
    fn shorten_path_no_home_prefix() {
        let path = "/tmp/other/path";
        assert_eq!(shorten_path(path), path);
    }

    #[test]
    fn dedup_by_name_keeps_first() {
        let skills = vec![
            ScannedSkill {
                path: "/a/SKILL.md".into(),
                name: "dup".into(),
                identifier: "id1".into(),
                client_id: "cursor".into(),
            },
            ScannedSkill {
                path: "/b/SKILL.md".into(),
                name: "dup".into(),
                identifier: "id2".into(),
                client_id: "claude_code".into(),
            },
            ScannedSkill {
                path: "/c/SKILL.md".into(),
                name: "unique".into(),
                identifier: "id3".into(),
                client_id: "cursor".into(),
            },
        ];
        let result = dedup_by_name(skills);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, "/a/SKILL.md");
        assert_eq!(result[1].name, "unique");
    }

    #[test]
    fn build_commit_message_single() {
        assert_eq!(
            build_commit_message(&["foo".into()]),
            "Add skill: foo"
        );
    }

    #[test]
    fn build_commit_message_few() {
        let names: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(build_commit_message(&names), "Add 3 skills: a, b, c");
    }

    #[test]
    fn build_commit_message_truncates() {
        let names: Vec<String> = (0..8).map(|i| format!("skill-{}", i)).collect();
        let msg = build_commit_message(&names);
        assert!(msg.contains("...and 3 more"));
        assert!(msg.starts_with("Add 8 skills:"));
    }
}
