//! Scans local AI-coding-tool session transcripts to report which chef
//! skills have actually been used, and on which calendar days.
//!
//! Two sources are scanned:
//! - Claude Code: `~/.claude/projects/**/*.jsonl`
//! - Codex: `~/.codex/archived_sessions/*.jsonl` and `~/.codex/sessions/**/*.jsonl`
//!
//! Transcripts are JSONL: one JSON object per line. A skill is considered
//! "used" on a given line if any string value contains the path fragment
//! `/skills/<name>/` (a file inside the skill directory) or ends with
//! `/skills/<name>` (a reference to the directory itself) — but ONLY when
//! that string appears inside content that represents an actual tool
//! action, not a system/developer/contextual message dump. This catches
//! both direct `Skill` tool invocations (Claude Code) and indirect signals
//! such as a subagent reading a file that lives inside the skill's
//! directory (e.g. `.../skills/morning-routine/tasks/foo.md`).
//!
//! Both Claude Code and Codex inject a full catalog of every available
//! skill (name, description, and literal `SKILL.md` path) into session
//! context at the start of a session, as plain informational content — not
//! evidence of use. Naively matching path fragments anywhere in a
//! transcript line falsely counts every skill in the catalog as "used" once
//! per session. To avoid this:
//!
//! - **Claude Code**: path-fragment matching only descends into JSON
//!   objects whose own `"type"` is `"tool_use"` or `"tool_result"` — the
//!   real action/result blocks. It does not recurse into `"attachment"`
//!   objects, plain message `"text"` blocks, or any other wrapper.
//! - **Codex**: path-fragment matching is skipped entirely for lines whose
//!   `payload.type` is `"message"` or `"session_meta"` (the two known
//!   catalog-dump/metadata shapes). Every other `payload.type` (e.g.
//!   `function_call`, `function_call_output`, `local_shell_call`,
//!   `local_shell_call_output`) is searched as before.
//!
//! For Claude Code files specifically, the explicit
//! `"name":"Skill","input":{"skill":"<name>"}` tool_use pattern is also
//! recognized as a hit (matching either the raw name or the substring after
//! the last `:`, for plugin-namespaced skills). This pattern is structurally
//! specific enough that a catalog dump can't accidentally match it, so it is
//! searched for across the whole line as before.
//!
//! Output is one [`UsageEvent`] per unique (skill, file) pair found while
//! scanning: each transcript FILE contributes at most one event per skill,
//! carrying a `count` of how many times that skill was really invoked in
//! that file. Aggregating across files/dates for a skill's total is the
//! CALLER's job (the cache layer sums via SQL, the frontend sums across
//! returned rows) — this module never sums across files itself.
//!
//! Within a single file, the count is computed per skill as follows:
//! - **Claude Code**: if the file contains one or more explicit
//!   `"name":"Skill","input":{"skill":"<name>"}` tool_use blocks for that
//!   skill, `count` is the exact number of such blocks (this pattern is
//!   precise — Claude Code logs one tool_use block per actual invocation,
//!   no fan-out ambiguity).
//! - Otherwise, if the path-fragment heuristic (matching `/skills/<name>/`
//!   or a `/skills/<name>` suffix inside genuine tool-action content) hit
//!   anywhere in the file, `count` is capped at `1` — this heuristic is
//!   fan-out-prone (a single logical use, e.g. a subagent-driven skill like
//!   `morning-routine`, can spawn many matching lines/files), so it is only
//!   ever trusted for a binary "used at least once in this file" signal.
//! - If neither signal fires for a skill in this file, the skill has no
//!   contribution from this file at all (no event emitted for it).
//!
//! This module is read-only: it never writes to any transcripts directory.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Bump this whenever a change to the extraction/matching logic in this
/// module can invalidate previously-cached `usage_events` rows (i.e. old
/// cached data may now be wrong and must be rebuilt from scratch). Stored in
/// `usage_file_state` under [`USAGE_CACHE_VERSION_SENTINEL_PATH`]; checked
/// and enforced at the start of every sync in [`sync_roots`].
///
/// Bumped to "3" for the move from "one deduplicated event per
/// (skill,date,client)" to "one event per (skill,file) carrying a real
/// per-file invocation `count`" — the on-disk schema itself changed shape
/// (new `file_path`/`count` columns, new primary key), so every previously
/// cached row is stale and must be rebuilt from scratch.
const USAGE_CACHE_VERSION: &str = "4";

/// Reserved sentinel path in `usage_file_state` used to stash the cache
/// schema version (in the `mtime` column, repurposed to hold a plain
/// version string instead of an actual file mtime). Not a real transcript
/// path, so it can never collide with one.
pub(crate) const USAGE_CACHE_VERSION_SENTINEL_PATH: &str = "__usage_cache_version__";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UsageEvent {
    pub skill: String,
    pub date: String,
    pub client: String,
    /// The working directory (project folder) the transcript session ran in,
    /// if one could be determined for the file this event came from. Empty
    /// string if no cwd was found.
    pub context: String,
    /// How many times this skill was really invoked in the ONE transcript
    /// file this event came from. This is the exact explicit
    /// `"name":"Skill"` tool_use count when that precise signal fired one or
    /// more times; otherwise it is `1` (a capped "used at least once in this
    /// file" signal from the fan-out-prone path-fragment heuristic). Never
    /// summed across files here — see module doc comment.
    pub count: usize,
}

/// Scan both Claude Code and Codex transcript trees for usage of any skill
/// in `chef_skill_names`, keeping only events on or after `since`. Returns a
/// deduplicated, sorted `Vec<UsageEvent>` — at most one entry per unique
/// (skill, date, client) triple.
#[allow(dead_code)]
pub fn scan_events(chef_skill_names: &[String], since: DateTime<Utc>) -> Vec<UsageEvent> {
    let mut events: HashSet<UsageEvent> = HashSet::new();

    if let Some(home) = dirs::home_dir() {
        let claude_projects = home.join(".claude").join("projects");
        scan_root(
            &claude_projects,
            "claude_code",
            chef_skill_names,
            since,
            &mut events,
        );

        let codex_archived = home.join(".codex").join("archived_sessions");
        scan_root(
            &codex_archived,
            "codex",
            chef_skill_names,
            since,
            &mut events,
        );

        let codex_sessions = home.join(".codex").join("sessions");
        if codex_sessions.is_dir() {
            scan_root(
                &codex_sessions,
                "codex",
                chef_skill_names,
                since,
                &mut events,
            );
        }
    }

    let mut result: Vec<UsageEvent> = events.into_iter().collect();
    result.sort();
    result
}

/// Walk `root` recursively for `*.jsonl` files and record usage events into
/// `out`. Separated from [`scan_events`] so tests can point it at a tempdir
/// fixture instead of the user's real home directory.
#[allow(dead_code)]
fn scan_root(
    root: &Path,
    client: &str,
    chef_skill_names: &[String],
    since: DateTime<Utc>,
    out: &mut HashSet<UsageEvent>,
) {
    if !root.is_dir() {
        return;
    }

    let mut jsonl_files = Vec::new();
    collect_jsonl_files(root, &mut jsonl_files);

    let since_system: SystemTime = since.into();

    for file in jsonl_files {
        // Performance pre-filter: transcripts are append-only, so if the
        // file hasn't been touched since before the window, none of its
        // lines can be in-window either. Skip reading it entirely.
        match std::fs::metadata(&file).and_then(|m| m.modified()) {
            Ok(modified) if modified < since_system => continue,
            _ => {}
        }

        let since_date_str = since.format("%Y-%m-%d").to_string();
        let events = extract_events_from_file(&file, client, chef_skill_names);
        for event in events {
            if event.date < since_date_str {
                continue;
            }
            out.insert(event);
        }
    }
}

/// Extract usage events found in a single transcript `file` — at most one
/// [`UsageEvent`] per skill that appears in this file at all, each carrying
/// its own per-file `count` (see module doc comment for how `count` is
/// computed). No `since` filtering is applied here; that happens at the
/// caller (either immediately for `scan_root`'s existing behavior, or later
/// at cache-query time for the incremental sync path).
///
/// Deliberately returns one event per skill for the WHOLE file, not per
/// line: a session can span multiple lines/timestamps, but a file's
/// contribution to a given skill's count must be a single number, so all
/// lines are scanned first and skill contributions are aggregated before any
/// `UsageEvent`s are built.
fn extract_events_from_file(
    file: &Path,
    client: &str,
    chef_skill_names: &[String],
) -> HashSet<UsageEvent> {
    let mut out: HashSet<UsageEvent> = HashSet::new();

    let Ok(content) = std::fs::read_to_string(file) else {
        return out;
    };

    // cwd is constant per transcript file (one working directory per
    // session), so find it once from the first line that reveals it rather
    // than re-deriving it per line.
    let mut context = String::new();
    let mut context_found = false;

    // Precise signal: exact explicit-tool-use call count per skill,
    // accumulated across every line in the file.
    let mut explicit_calls: HashMap<&str, usize> = HashMap::new();
    // Imprecise signal: whether the path-fragment heuristic matched a skill
    // ANYWHERE in the file. Fan-out-prone, so only ever used as a binary
    // "present in this file" flag, never counted.
    let mut path_hit: HashSet<&str> = HashSet::new();
    // Last timestamp seen for each skill that had a hit on that line, used
    // as the event's date. For a file spanning multiple calendar dates this
    // picks the most recent contributing line's date — a deliberate, simple
    // choice; in practice a single transcript file's lines share a date.
    let mut last_date: HashMap<&str, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        if !context_found {
            if let Some(cwd) = find_cwd(&value, client) {
                context = cwd;
                context_found = true;
            }
        }

        let mut line_path_hits: HashSet<&str> = HashSet::new();

        // Path-fragment heuristic, restricted to genuine tool-action content
        // (see module doc comment for why).
        match client {
            "claude_code" => {
                collect_path_matches_claude(&value, chef_skill_names, &mut line_path_hits)
            }
            "codex" => collect_path_matches_codex(&value, chef_skill_names, &mut line_path_hits),
            _ => {}
        }

        // Explicit Skill tool_use pattern: Claude Code only. Counts every
        // occurrence on this line (a line's content array can hold more
        // than one tool_use block).
        let mut line_explicit_hits: Vec<&str> = Vec::new();
        if client == "claude_code" {
            let mut tool_names = Vec::new();
            collect_skill_tool_invocations(&value, &mut tool_names);
            for raw_name in tool_names {
                if let Some(name) = chef_skill_names
                    .iter()
                    .find(|n| n.as_str() == raw_name.as_str())
                {
                    line_explicit_hits.push(name.as_str());
                } else if let Some(after_colon) = raw_name.rsplit(':').next() {
                    if let Some(name) = chef_skill_names.iter().find(|n| n.as_str() == after_colon)
                    {
                        line_explicit_hits.push(name.as_str());
                    }
                }
            }
        }

        if line_path_hits.is_empty() && line_explicit_hits.is_empty() {
            continue;
        }

        let Some(date) = find_timestamp_date(&value) else {
            continue;
        };
        let date_str = date.format("%Y-%m-%d").to_string();

        for name in &line_path_hits {
            path_hit.insert(name);
            last_date.insert(name, date_str.clone());
        }
        for name in &line_explicit_hits {
            *explicit_calls.entry(name).or_insert(0) += 1;
            last_date.insert(name, date_str.clone());
        }
    }

    // Union of every skill that had any hit at all in this file.
    let mut all_skills: HashSet<&str> = HashSet::new();
    all_skills.extend(explicit_calls.keys().copied());
    all_skills.extend(path_hit.iter().copied());

    for name in all_skills {
        let count = match explicit_calls.get(name) {
            Some(&n) if n > 0 => n,
            _ => 1, // path-fragment-only hit, capped at 1 per file.
        };
        let Some(date_str) = last_date.get(name) else {
            continue;
        };

        out.insert(UsageEvent {
            skill: name.to_string(),
            date: date_str.clone(),
            client: client.to_string(),
            context: context.clone(),
            count,
        });
    }

    out
}

/// Recursively find all `*.jsonl` files under `dir`. Tolerant of unreadable
/// subdirectories — skips them rather than propagating an error.
fn collect_jsonl_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            out.push(path);
        }
    }
}

/// Check whether a string value contains `/skills/<name>/` or ends with
/// `/skills/<name>`, for each candidate name in `chef_skill_names`. Collects
/// matched names into `out`.
fn check_path_fragment<'a>(s: &str, chef_skill_names: &'a [String], out: &mut HashSet<&'a str>) {
    for name in chef_skill_names {
        let inner = format!("/skills/{name}/");
        let suffix = format!("/skills/{name}");
        if s.contains(&inner) || s.ends_with(&suffix) {
            out.insert(name.as_str());
        }
    }
}

/// Recursively apply [`check_path_fragment`] to every string value in a
/// JSON subtree. Used once we've already established the subtree is inside
/// a trustworthy tool-action object.
fn collect_path_fragments_in_subtree<'a>(
    value: &Value,
    chef_skill_names: &'a [String],
    out: &mut HashSet<&'a str>,
) {
    match value {
        Value::String(s) => check_path_fragment(s, chef_skill_names, out),
        Value::Object(map) => {
            for v in map.values() {
                collect_path_fragments_in_subtree(v, chef_skill_names, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_path_fragments_in_subtree(v, chef_skill_names, out);
            }
        }
        _ => {}
    }
}

/// A real, single skill invocation only ever references its OWN skill's
/// path. Any tool action whose output happens to mention MORE THAN ONE
/// distinct skill's path is a listing of some kind (a directory listing, a
/// git status/diff, a "find" result, a manifest dump, etc.) rather than
/// evidence that any specific one of them was actually used — confirmed
/// against real data twice now: once as a Codex `payload.type:"message"`
/// skill catalog, and once as a real `function_call_output` containing a
/// git-status-style listing of ~20 unrelated `.claude/skills/<name>`
/// entries that were merely "Added" in a commit, not invoked. Collect
/// matches for one action block/output into a scratch set first; only
/// merge into the caller's running result if it names 0 or 1 skills.
fn collect_path_fragments_if_single_skill<'a>(
    value: &Value,
    chef_skill_names: &'a [String],
    out: &mut HashSet<&'a str>,
) {
    let mut scratch: HashSet<&str> = HashSet::new();
    collect_path_fragments_in_subtree(value, chef_skill_names, &mut scratch);
    if scratch.len() <= 1 {
        out.extend(scratch);
    }
}

/// Claude Code path-fragment matcher: walks the tree looking for JSON
/// objects whose own `"type"` is `"tool_use"` or `"tool_result"` — the real
/// action/result blocks — and only applies the path-fragment check to
/// string values found WITHIN those objects' subtrees. Does not match
/// against `"attachment"` objects, plain message `"text"` blocks, or any
/// other contextual wrapper, since those can contain a full skill catalog
/// dump that isn't evidence of actual use.
fn collect_path_matches_claude<'a>(
    value: &Value,
    chef_skill_names: &'a [String],
    out: &mut HashSet<&'a str>,
) {
    match value {
        Value::Object(map) => {
            let is_action_block = matches!(
                map.get("type"),
                Some(Value::String(t)) if t == "tool_use" || t == "tool_result"
            );
            if is_action_block {
                collect_path_fragments_if_single_skill(value, chef_skill_names, out);
            } else {
                for v in map.values() {
                    collect_path_matches_claude(v, chef_skill_names, out);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_path_matches_claude(v, chef_skill_names, out);
            }
        }
        _ => {}
    }
}

/// Codex path-fragment matcher: skips path-fragment matching entirely for
/// lines whose `payload.type` is `"message"` (skill-catalog dumps injected
/// as developer/system context) or `"session_meta"` (session metadata, no
/// tool action). Every other `payload.type` — the real tool-call/output
/// shapes such as `function_call`, `function_call_output`,
/// `local_shell_call`, `local_shell_call_output` — is searched as before.
/// Lines with no `payload.type` at all are searched as before too.
fn collect_path_matches_codex<'a>(
    value: &Value,
    chef_skill_names: &'a [String],
    out: &mut HashSet<&'a str>,
) {
    if let Value::Object(map) = value {
        if let Some(payload) = map.get("payload") {
            if let Some(Value::String(payload_type)) = payload.get("type") {
                if payload_type == "message" || payload_type == "session_meta" {
                    return;
                }
            }
        }
    }
    collect_path_fragments_if_single_skill(value, chef_skill_names, out);
}

/// Find the working directory for a transcript file from a single parsed
/// line, if this particular line reveals it. cwd is constant per file, so
/// callers should call this once per line until it returns `Some`.
///
/// - Codex: `payload.type == "session_meta"` lines carry it at
///   `payload.payload.cwd` (double-nested: the outer `payload` is the Codex
///   envelope, the inner `payload` is the session_meta body).
/// - Claude Code: any line with a top-level `cwd` string field.
fn find_cwd(value: &Value, client: &str) -> Option<String> {
    match client {
        "codex" => {
            let payload = value.get("payload")?;
            if let Some(Value::String(t)) = payload.get("type") {
                if t == "session_meta" {
                    if let Some(Value::String(cwd)) =
                        payload.get("payload").and_then(|p| p.get("cwd"))
                    {
                        return Some(cwd.clone());
                    }
                }
            }
            None
        }
        "claude_code" => {
            if let Some(Value::String(cwd)) = value.get("cwd") {
                return Some(cwd.clone());
            }
            None
        }
        _ => None,
    }
}

/// Walk a parsed transcript-line JSON value looking for `tool_use` blocks
/// where `name == "Skill"` and `input.skill` is a string, collecting every
/// match (a single line's content array can hold multiple tool_use blocks).
fn collect_skill_tool_invocations(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let (Some(Value::String(name)), Some(input)) = (map.get("name"), map.get("input")) {
                if name == "Skill" {
                    if let Some(Value::String(skill)) = input.get("skill") {
                        out.push(skill.clone());
                    }
                }
            }
            for v in map.values() {
                collect_skill_tool_invocations(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_skill_tool_invocations(v, out);
            }
        }
        _ => {}
    }
}

/// Walk a parsed transcript-line JSON value looking for a key literally
/// named `timestamp` whose value parses as an RFC3339/ISO8601 datetime.
/// Returns the first one found (transcript lines only ever have one
/// meaningful timestamp, at the top level or nested in `message`/etc).
fn find_timestamp_date(value: &Value) -> Option<DateTime<Utc>> {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(ts)) = map.get("timestamp") {
                if let Ok(parsed) = DateTime::parse_from_rfc3339(ts) {
                    return Some(parsed.with_timezone(&Utc));
                }
            }
            for v in map.values() {
                if let Some(found) = find_timestamp_date(v) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => {
            for v in arr {
                if let Some(found) = find_timestamp_date(v) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

/// Return cached usage events for `chef_skill_names` on or after `since`,
/// syncing the on-disk cache against any new/changed transcript files first.
/// Only files that are new or whose mtime has changed since the last sync
/// are re-read; unchanged files are served entirely from the cache.
pub fn synced_events(
    skill_db: &crate::skill_db::SkillDb,
    chef_skill_names: &[String],
    since: DateTime<Utc>,
) -> crate::error::Result<Vec<UsageEvent>> {
    sync_cache(skill_db, chef_skill_names)?;

    let since_date_str = since.format("%Y-%m-%d").to_string();
    let mut result = skill_db.query_usage_events(&since_date_str)?;

    // Filter out any cached events for skills no longer in the cookbook
    // (e.g. a skill was removed since the event was cached).
    result.retain(|e| chef_skill_names.iter().any(|n| n == &e.skill));

    result.sort();
    Ok(result)
}

/// Wipe the entire usage cache and rebuild it from scratch, treating every
/// transcript file as new. Used by the manual "Re-sync" escape hatch.
pub fn force_resync(
    skill_db: &crate::skill_db::SkillDb,
    chef_skill_names: &[String],
    since: DateTime<Utc>,
) -> crate::error::Result<Vec<UsageEvent>> {
    skill_db.clear_usage_cache()?;
    synced_events(skill_db, chef_skill_names, since)
}

/// Incrementally sync the on-disk cache against the current state of the
/// Claude Code and Codex transcript trees. For each `*.jsonl` file found,
/// compares its current mtime against the cached mtime: if unchanged, skips
/// the file entirely (nothing new to extract); otherwise (new file, or
/// changed mtime) re-extracts events from it, persists them, and updates the
/// cached mtime. Tolerant of per-file IO/metadata errors — skips just that
/// file rather than aborting the whole sync.
///
/// Known accepted limitation: if a brand-new skill is added to the cookbook
/// and an already-cached (mtime-unchanged) transcript file happens to
/// reference that skill's name, it won't be picked up until that file
/// changes again or the user triggers a manual resync. A newly-added skill
/// has no meaningful "history before it was added" anyway, so this is fine.
fn sync_cache(
    skill_db: &crate::skill_db::SkillDb,
    chef_skill_names: &[String],
) -> crate::error::Result<()> {
    let Some(home) = dirs::home_dir() else {
        return Ok(());
    };

    let roots: [(PathBuf, &str); 3] = [
        (home.join(".claude").join("projects"), "claude_code"),
        (home.join(".codex").join("archived_sessions"), "codex"),
        (home.join(".codex").join("sessions"), "codex"),
    ];

    sync_roots(skill_db, chef_skill_names, &roots)
}

/// Path-parameterized core of [`sync_cache`], separated so tests can point
/// it at tempdir fixtures instead of the user's real home directory (mirrors
/// the existing `scan_events`/`scan_root` split in this module).
fn sync_roots(
    skill_db: &crate::skill_db::SkillDb,
    chef_skill_names: &[String],
    roots: &[(PathBuf, &str)],
) -> crate::error::Result<()> {
    // One-time cache invalidation on upgrade: the matching-logic fix in
    // this module can turn previously-cached events into garbage (a
    // catalog-dump false-positive baked in before the fix shipped), but
    // those files' mtimes haven't changed, so the normal incremental sync
    // below would never re-read them. Detect a stale/missing version
    // marker, wipe the cache once, and record the new version — every file
    // then looks "new" and gets correctly re-extracted below.
    let cached_version = skill_db.usage_file_mtime(USAGE_CACHE_VERSION_SENTINEL_PATH)?;
    if cached_version.as_deref() != Some(USAGE_CACHE_VERSION) {
        skill_db.clear_usage_cache()?;
        skill_db.set_usage_cache_version(USAGE_CACHE_VERSION)?;
    }

    // Collect all (path, mtime, events) updates in memory first — the
    // per-file mtime lookups below are cheap read-only SELECTs, but writing
    // is not: with thousands of transcript files, one auto-committed write
    // per file (or per event) means thousands of separate fsyncs, which is
    // unusably slow. Persist everything in a single transaction at the end
    // via `sync_usage_batch` instead.
    let mut updates: Vec<(String, String, HashSet<UsageEvent>)> = Vec::new();

    for (root, client) in roots {
        if !root.is_dir() {
            continue;
        }

        let mut jsonl_files = Vec::new();
        collect_jsonl_files(root, &mut jsonl_files);

        for file in jsonl_files {
            let Ok(metadata) = std::fs::metadata(&file) else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            let Ok(duration) = modified.duration_since(UNIX_EPOCH) else {
                continue;
            };
            let current_mtime = duration.as_secs().to_string();
            let path_string = file.to_string_lossy().to_string();

            let cached_mtime = skill_db.usage_file_mtime(&path_string)?;
            if cached_mtime.as_deref() == Some(current_mtime.as_str()) {
                // Unchanged since last sync — nothing new to extract.
                continue;
            }

            let events = extract_events_from_file(&file, client, chef_skill_names);
            updates.push((path_string, current_mtime, events));
        }
    }

    if !updates.is_empty() {
        skill_db.sync_usage_batch(&updates)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::fs;

    fn long_ago() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap()
    }

    /// Regression test for the bug that motivated this redesign: a
    /// subagent reads a file inside the skill's directory, with NO
    /// `"name":"Skill"` tool_use block anywhere in the transcript. The old
    /// `collect_skill_invocations`-only approach would find zero hits here.
    #[test]
    fn detects_usage_from_subagent_file_path_with_no_skill_tool_use() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let project_dir = root.join("-Users-alp-project");
        fs::create_dir_all(&project_dir).unwrap();

        let file = project_dir.join("session1.jsonl");
        fs::write(
            &file,
            r#"{"timestamp":"2026-01-01T10:00:00.000Z","message":{"content":[{"type":"tool_result","content":"Reading /Users/alp/.claude/skills/morning-routine/tasks/bookmarks.md"}]}}
"#,
        )
        .unwrap();

        let chef_skill_names = vec!["morning-routine".to_string()];

        // Sanity check: prove the OLD tool_use-only detector would miss this.
        let raw = fs::read_to_string(&file).unwrap();
        let value: Value = serde_json::from_str(raw.lines().next().unwrap()).unwrap();
        let mut old_style_hits = Vec::new();
        collect_skill_tool_invocations(&value, &mut old_style_hits);
        assert!(
            old_style_hits.is_empty(),
            "fixture must contain zero Skill tool_use blocks to be a valid regression test"
        );

        let mut events = HashSet::new();
        scan_root(
            root,
            "claude_code",
            &chef_skill_names,
            long_ago(),
            &mut events,
        );

        assert_eq!(events.len(), 1, "expected exactly one usage event");
        let event = events.iter().next().unwrap();
        assert_eq!(event.skill, "morning-routine");
        assert_eq!(event.date, "2026-01-01");
        assert_eq!(event.client, "claude_code");
        assert_eq!(
            event.count, 1,
            "path-fragment-only hit must be capped at 1 regardless of matches"
        );
    }

    #[test]
    fn detects_usage_from_explicit_skill_tool_use_pattern() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let project_dir = root.join("-Users-alp-project");
        fs::create_dir_all(&project_dir).unwrap();

        let file = project_dir.join("session1.jsonl");
        fs::write(
            &file,
            r#"{"timestamp":"2026-01-05T08:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}
"#,
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string()];
        let mut events = HashSet::new();
        scan_root(
            root,
            "claude_code",
            &chef_skill_names,
            long_ago(),
            &mut events,
        );

        assert_eq!(events.len(), 1);
        let event = events.iter().next().unwrap();
        assert_eq!(event.skill, "ponytail");
        assert_eq!(event.date, "2026-01-05");
        assert_eq!(event.client, "claude_code");
        assert_eq!(event.count, 1, "single explicit tool_use block => count 1");
    }

    /// Proves claim (a) from the design: 3 distinct explicit
    /// `"name":"Skill"` tool_use blocks for the same skill in ONE file must
    /// yield `count == 3` for that event — the precise signal's exact count
    /// is trusted, not capped at 1 like the path-fragment heuristic.
    #[test]
    fn explicit_skill_tool_use_repeated_three_times_yields_count_three() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let project_dir = root.join("-Users-alp-project");
        fs::create_dir_all(&project_dir).unwrap();

        let file = project_dir.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n{}\n{}\n",
                r#"{"timestamp":"2026-01-05T08:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"#,
                r#"{"timestamp":"2026-01-05T09:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"#,
                r#"{"timestamp":"2026-01-05T10:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string()];
        let events = extract_events_from_file(&file, "claude_code", &chef_skill_names);

        assert_eq!(events.len(), 1, "one event per (skill, file)");
        let event = events.iter().next().unwrap();
        assert_eq!(event.skill, "ponytail");
        assert_eq!(
            event.count, 3,
            "3 distinct explicit Skill tool_use blocks must yield count 3"
        );
    }

    /// Proves claim (b) from the design: a path-fragment-only file (no
    /// explicit tool_use at all, e.g. the morning-routine-style subagent
    /// fixture) yields `count == 1` regardless of how many separate matching
    /// lines/paths exist in that file.
    #[test]
    fn path_fragment_only_file_with_many_matching_lines_yields_count_one() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let project_dir = root.join("-Users-alp-project");
        fs::create_dir_all(&project_dir).unwrap();

        let file = project_dir.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n{}\n{}\n{}\n",
                r#"{"timestamp":"2026-01-01T10:00:00.000Z","message":{"content":[{"type":"tool_result","content":"Reading /Users/alp/.claude/skills/morning-routine/tasks/bookmarks.md"}]}}"#,
                r#"{"timestamp":"2026-01-01T10:05:00.000Z","message":{"content":[{"type":"tool_result","content":"Reading /Users/alp/.claude/skills/morning-routine/tasks/logbook.md"}]}}"#,
                r#"{"timestamp":"2026-01-01T10:10:00.000Z","message":{"content":[{"type":"tool_result","content":"Reading /Users/alp/.claude/skills/morning-routine/tasks/backup.md"}]}}"#,
                r#"{"timestamp":"2026-01-01T10:15:00.000Z","message":{"content":[{"type":"tool_result","content":"Reading /Users/alp/.claude/skills/morning-routine/tasks/security.md"}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["morning-routine".to_string()];
        let events = extract_events_from_file(&file, "claude_code", &chef_skill_names);

        assert_eq!(events.len(), 1, "one event per (skill, file)");
        let event = events.iter().next().unwrap();
        assert_eq!(event.skill, "morning-routine");
        assert_eq!(
            event.count, 1,
            "4 matching lines from subagent fan-out must still cap at count 1"
        );
    }

    /// The property that makes the whole design safe: when a single file
    /// has BOTH the precise explicit-tool-use signal AND the imprecise
    /// path-fragment signal for the same skill, the explicit count wins
    /// outright — the path-fragment hits contribute nothing extra (no
    /// "count + 1" double counting), since a real `Skill` invocation
    /// virtually always also leaves matching path-fragment traces (e.g. the
    /// tool_result of the invocation itself).
    #[test]
    fn explicit_calls_take_precedence_over_path_fragment_hits_in_same_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let project_dir = root.join("-Users-alp-project");
        fs::create_dir_all(&project_dir).unwrap();

        let file = project_dir.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n{}\n{}\n",
                r#"{"timestamp":"2026-01-05T08:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"#,
                r#"{"timestamp":"2026-01-05T08:05:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/SKILL.md"}]}}"#,
                r#"{"timestamp":"2026-01-05T09:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string()];
        let events = extract_events_from_file(&file, "claude_code", &chef_skill_names);

        assert_eq!(events.len(), 1, "one event per (skill, file)");
        let event = events.iter().next().unwrap();
        assert_eq!(
            event.count, 2,
            "explicit count (2) must win outright over the path-fragment \
             hit also present in this file — no extra +1 for the path hit"
        );
    }

    #[test]
    fn detects_usage_from_codex_style_transcript() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root).unwrap();

        let file = root.join("rollout-2026-02-01.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n{}\n",
                r#"{"timestamp":"2026-02-01T00:00:00.000Z","type":"session_meta","payload":{{}}}"#,
                r#"{"timestamp":"2026-02-01T09:30:00.000Z","type":"function_call_output","payload":{"content":"cat /home/user/.codex/skills/deploy_www/SKILL.md"}}"#
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["deploy_www".to_string()];
        let mut events = HashSet::new();
        scan_root(root, "codex", &chef_skill_names, long_ago(), &mut events);

        assert_eq!(events.len(), 1);
        let event = events.iter().next().unwrap();
        assert_eq!(event.skill, "deploy_www");
        assert_eq!(event.date, "2026-02-01");
        assert_eq!(event.client, "codex");
        assert_eq!(event.count, 1, "path-fragment hit is capped at 1");
    }

    /// Under the new per-file design, `extract_events_from_file` is called
    /// once per file and each file contributes its OWN event — multiple
    /// path-fragment references within a single file still collapse to one
    /// `count == 1` event for that file (the anti-fan-out cap), but each
    /// DISTINCT file is its own independent contribution. This is verified
    /// directly against `extract_events_from_file` (the real per-file unit),
    /// rather than through `scan_root`'s cross-file `HashSet`, since two
    /// structurally-identical events from different files coincidentally
    /// collapsing in a `HashSet` would prove nothing about file-awareness
    /// (see `sync_usage_batch`/DB-layer tests below for the real proof that
    /// per-file identity is respected end-to-end).
    #[test]
    fn multiple_path_fragment_references_within_one_file_still_cap_at_count_one() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let project_dir = root.join("-Users-alp-project");
        fs::create_dir_all(&project_dir).unwrap();

        let file1 = project_dir.join("session1.jsonl");
        fs::write(
            &file1,
            format!(
                "{}\n{}\n",
                r#"{"timestamp":"2026-03-01T10:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/SKILL.md"}]}}"#,
                r#"{"timestamp":"2026-03-01T11:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/notes.md"}]}}"#,
            ),
        )
        .unwrap();

        let nested_dir = project_dir.join("subagents");
        fs::create_dir_all(&nested_dir).unwrap();
        let file2 = nested_dir.join("session2.jsonl");
        fs::write(
            &file2,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-03-01T12:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/again.md"}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string()];

        let events1 = extract_events_from_file(&file1, "claude_code", &chef_skill_names);
        assert_eq!(events1.len(), 1, "one event for file1");
        let event1 = events1.iter().next().unwrap();
        assert_eq!(
            event1.count, 1,
            "2 path-fragment references within file1 must still cap at count 1"
        );

        let events2 = extract_events_from_file(&file2, "claude_code", &chef_skill_names);
        assert_eq!(events2.len(), 1, "one event for file2");
        let event2 = events2.iter().next().unwrap();
        assert_eq!(
            event2.count, 1,
            "the single path-fragment reference within file2 caps at count 1"
        );
    }

    #[test]
    fn does_not_match_prefix_overlapping_skill_names() {
        // "morning" must not match "morning-routine"'s path because the
        // matcher requires a trailing '/' or end-of-string boundary.
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root).unwrap();

        let file = root.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-04-01T00:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/morning-routine/tasks/x.md"}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["morning".to_string()];
        let mut events = HashSet::new();
        scan_root(
            root,
            "claude_code",
            &chef_skill_names,
            long_ago(),
            &mut events,
        );

        assert!(
            events.is_empty(),
            "prefix-overlapping skill name must not falsely match"
        );
    }

    #[test]
    fn missing_root_yields_no_events_not_error() {
        let missing = PathBuf::from("/definitely/does/not/exist/anywhere");
        let chef_skill_names = vec!["foo".to_string()];
        let mut events = HashSet::new();
        scan_root(
            &missing,
            "claude_code",
            &chef_skill_names,
            long_ago(),
            &mut events,
        );
        assert!(events.is_empty());
    }

    #[test]
    fn events_before_since_are_excluded() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root).unwrap();

        let file = root.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2020-01-01T00:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/SKILL.md"}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string()];
        let mut events = HashSet::new();
        // "since" is in the future relative to the fixture's own timestamp,
        // but recent enough not to be excluded by the mtime pre-filter.
        let since = Utc::now() - chrono::Duration::days(1);
        scan_root(root, "claude_code", &chef_skill_names, since, &mut events);

        assert!(
            events.is_empty(),
            "event dated before `since` must be excluded"
        );
    }

    // ===== Caching layer tests =====

    fn open_test_db(tmp: &tempfile::TempDir) -> crate::skill_db::SkillDb {
        crate::skill_db::SkillDb::open(tmp.path()).expect("open test skill db")
    }

    #[test]
    fn synced_events_is_stable_across_repeated_calls_with_no_fs_changes() {
        let db_tmp = tempfile::tempdir().expect("db tempdir");
        let db = open_test_db(&db_tmp);

        let transcripts_tmp = tempfile::tempdir().expect("transcripts tempdir");
        let root = transcripts_tmp.path().to_path_buf();
        fs::create_dir_all(&root).unwrap();

        let file = root.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-05-01T00:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/SKILL.md"}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string()];
        let roots = [(root.clone(), "claude_code")];

        sync_roots(&db, &chef_skill_names, &roots).expect("first sync");
        let since = long_ago();
        let first = db
            .query_usage_events(&since.format("%Y-%m-%d").to_string())
            .expect("query 1");

        sync_roots(&db, &chef_skill_names, &roots).expect("second sync");
        let second = db
            .query_usage_events(&since.format("%Y-%m-%d").to_string())
            .expect("query 2");

        assert_eq!(first, second, "no fs changes must yield identical result");
        assert_eq!(first.len(), 1);

        let cached_mtime = db
            .usage_file_mtime(&file.to_string_lossy())
            .expect("mtime lookup");
        assert!(
            cached_mtime.is_some(),
            "file state row must exist after sync"
        );

        // Real proof-of-caching: rewrite the file's *content* to reference a
        // different skill, but restore its exact original mtime before
        // syncing again. If the skip-on-unchanged-mtime logic works, the new
        // skill must NOT appear — the file was never re-read.
        let original_mtime = std::fs::metadata(&file).unwrap().modified().unwrap();
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-05-01T00:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/deploy_www/SKILL.md"}]}}"#,
            ),
        )
        .unwrap();
        std::fs::File::open(&file)
            .unwrap()
            .set_modified(original_mtime)
            .expect("restore original mtime");

        let chef_skill_names_extended = vec!["ponytail".to_string(), "deploy_www".to_string()];
        sync_roots(&db, &chef_skill_names_extended, &roots).expect("third sync, same mtime");
        let third = db
            .query_usage_events(&since.format("%Y-%m-%d").to_string())
            .expect("query 3");

        assert_eq!(
            third.len(),
            1,
            "unchanged mtime must skip re-reading, so the new skill in the \
             rewritten content must not be picked up"
        );
        assert!(
            !third.iter().any(|e| e.skill == "deploy_www"),
            "file was skipped due to unchanged mtime, so deploy_www must be absent"
        );
    }

    #[test]
    fn force_resync_finds_everything_after_clearing_a_populated_cache() {
        let db_tmp = tempfile::tempdir().expect("db tempdir");
        let db = open_test_db(&db_tmp);

        let transcripts_tmp = tempfile::tempdir().expect("transcripts tempdir");
        let root = transcripts_tmp.path().to_path_buf();
        fs::create_dir_all(&root).unwrap();

        let file = root.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-05-02T00:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/SKILL.md"}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string()];
        let roots = [(root.clone(), "claude_code")];

        sync_roots(&db, &chef_skill_names, &roots).expect("initial sync");
        db.clear_usage_cache().expect("clear cache");

        // After clearing, both tables should be empty until we sync again.
        let empty = db.query_usage_events("2000-01-01").expect("query empty");
        assert!(empty.is_empty(), "cache must be empty right after clearing");

        sync_roots(&db, &chef_skill_names, &roots).expect("resync after clear");
        let rebuilt = db.query_usage_events("2000-01-01").expect("query rebuilt");

        assert_eq!(rebuilt.len(), 1, "resync must rebuild the cache fully");
        assert_eq!(rebuilt[0].skill, "ponytail");
        assert_eq!(rebuilt[0].date, "2026-05-02");
    }

    #[test]
    fn modified_file_between_syncs_picks_up_new_events_on_second_sync() {
        let db_tmp = tempfile::tempdir().expect("db tempdir");
        let db = open_test_db(&db_tmp);

        let transcripts_tmp = tempfile::tempdir().expect("transcripts tempdir");
        let root = transcripts_tmp.path().to_path_buf();
        fs::create_dir_all(&root).unwrap();

        let file = root.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-05-03T00:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/SKILL.md"}]}}"#,
            ),
        )
        .unwrap();
        // Force an explicit, early mtime so the second write's mtime is
        // guaranteed to differ (avoids second-granularity mtime flakiness).
        let f = std::fs::File::open(&file).unwrap();
        f.set_modified(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000))
            .expect("set initial mtime");

        let chef_skill_names = vec!["ponytail".to_string(), "deploy_www".to_string()];
        let roots = [(root.clone(), "claude_code")];

        sync_roots(&db, &chef_skill_names, &roots).expect("first sync");
        let after_first = db.query_usage_events("2000-01-01").expect("query 1");
        assert_eq!(after_first.len(), 1);

        // Modify the file's content and bump its mtime forward explicitly.
        fs::write(
            &file,
            format!(
                "{}\n{}\n",
                r#"{"timestamp":"2026-05-03T00:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/SKILL.md"}]}}"#,
                r#"{"timestamp":"2026-05-04T00:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/deploy_www/SKILL.md"}]}}"#,
            ),
        )
        .unwrap();
        let f = std::fs::File::open(&file).unwrap();
        f.set_modified(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_100))
            .expect("set updated mtime");

        sync_roots(&db, &chef_skill_names, &roots).expect("second sync");
        let after_second = db.query_usage_events("2000-01-01").expect("query 2");

        assert_eq!(
            after_second.len(),
            2,
            "modified file must contribute its new event on the second sync"
        );
        assert!(after_second
            .iter()
            .any(|e| e.skill == "deploy_www" && e.date == "2026-05-04"));
        assert!(after_second
            .iter()
            .any(|e| e.skill == "ponytail" && e.date == "2026-05-03"));
    }

    /// Proves claim (c) from the design — the critical correctness point:
    /// re-scanning a file whose content changed between two syncs must
    /// REPLACE its old contribution, not add to it. Constructs a file with 2
    /// explicit `"name":"Skill"` tool_use blocks for `ponytail` (count 2),
    /// syncs, then rewrites the SAME file with 5 explicit blocks (count 5)
    /// and bumps its mtime, and syncs again. If `sync_usage_batch` correctly
    /// deletes-then-reinserts by `file_path` (rather than ever summing old +
    /// new), the final queried count must be exactly 5 — never 7
    /// (2 old + 5 new) and never left stuck at 2 (old, unreplaced).
    #[test]
    fn rescanning_a_changed_file_replaces_its_old_contribution_not_adds_to_it() {
        let db_tmp = tempfile::tempdir().expect("db tempdir");
        let db = open_test_db(&db_tmp);

        let transcripts_tmp = tempfile::tempdir().expect("transcripts tempdir");
        let root = transcripts_tmp.path().to_path_buf();
        fs::create_dir_all(&root).unwrap();

        let file = root.join("session1.jsonl");
        let two_calls = format!(
            "{}\n{}\n",
            r#"{"timestamp":"2026-05-05T00:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"#,
            r#"{"timestamp":"2026-05-05T01:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"#,
        );
        fs::write(&file, &two_calls).unwrap();
        let f = std::fs::File::open(&file).unwrap();
        f.set_modified(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000))
            .expect("set initial mtime");

        let chef_skill_names = vec!["ponytail".to_string()];
        let roots = [(root.clone(), "claude_code")];

        sync_roots(&db, &chef_skill_names, &roots).expect("first sync");
        let after_first = db.query_usage_events("2000-01-01").expect("query 1");
        assert_eq!(
            after_first.len(),
            1,
            "exactly one row for this file's contribution"
        );
        assert_eq!(
            after_first[0].count, 2,
            "first sync must record the 2 explicit calls in the original content"
        );

        // Rewrite the SAME file (same path) with different, larger content:
        // 5 explicit Skill tool_use blocks for ponytail instead of 2. Bump
        // mtime forward so the sync doesn't skip it as unchanged.
        let five_calls = format!(
            "{}{}{}{}{}",
            r#"{"timestamp":"2026-05-06T00:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"# .to_string() + "\n",
            r#"{"timestamp":"2026-05-06T01:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"# .to_string() + "\n",
            r#"{"timestamp":"2026-05-06T02:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"# .to_string() + "\n",
            r#"{"timestamp":"2026-05-06T03:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"# .to_string() + "\n",
            r#"{"timestamp":"2026-05-06T04:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"ponytail"}}]}}"# .to_string() + "\n",
        );
        fs::write(&file, &five_calls).unwrap();
        let f = std::fs::File::open(&file).unwrap();
        f.set_modified(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_100))
            .expect("set updated mtime");

        sync_roots(&db, &chef_skill_names, &roots).expect("second sync, content changed");
        let after_second = db.query_usage_events("2000-01-01").expect("query 2");

        assert_eq!(
            after_second.len(),
            1,
            "still exactly one row for this one file — old row must be \
             replaced, not left alongside the new one"
        );
        assert_eq!(
            after_second[0].count, 5,
            "count must reflect ONLY the new content's 5 calls — not 2+5=7 \
             (would mean old+new summed) and not 2 (would mean the file was \
             never actually replaced)"
        );
        assert_eq!(after_second[0].date, "2026-05-06");
    }

    // ===== False-positive catalog-dump regression tests =====

    /// Real-data regression test: a Codex `payload.type:"message"` line
    /// (a developer/system message dumping the full skill catalog into
    /// session context) must produce ZERO usage events, even though it
    /// contains several different skills' literal `SKILL.md` paths.
    #[test]
    fn codex_message_type_catalog_dump_produces_zero_events() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root).unwrap();

        let file = root.join("rollout-2026-06-01.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-06-01T00:00:00.000Z","type":"response_item","payload":{"type":"message","role":"developer","content":"Available skills:\n- google-cloud-waf-security: .../skills/google-cloud-waf-security/SKILL.md\n- deploy_www: .../skills/deploy_www/SKILL.md"}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec![
            "google-cloud-waf-security".to_string(),
            "deploy_www".to_string(),
        ];
        let mut events = HashSet::new();
        scan_root(root, "codex", &chef_skill_names, long_ago(), &mut events);

        assert!(
            events.is_empty(),
            "a payload.type:\"message\" catalog dump must not produce any usage events"
        );
    }

    /// Real-data regression test: a Claude Code `"type":"attachment"` blob
    /// (a contextual dump, not a tool action) must produce ZERO usage
    /// events, even though it contains several different skills' `SKILL.md`
    /// paths in one blob.
    #[test]
    fn claude_code_attachment_type_catalog_dump_produces_zero_events() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let project_dir = root.join("-Users-alp-project");
        fs::create_dir_all(&project_dir).unwrap();

        let file = project_dir.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-06-02T00:00:00.000Z","type":"attachment","content":"Available skills:\n- ponytail: /Users/alp/.claude/skills/ponytail/SKILL.md\n- deploy_www: /Users/alp/.claude/skills/deploy_www/SKILL.md"}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string(), "deploy_www".to_string()];
        let mut events = HashSet::new();
        scan_root(
            root,
            "claude_code",
            &chef_skill_names,
            long_ago(),
            &mut events,
        );

        assert!(
            events.is_empty(),
            "a \"type\":\"attachment\" catalog dump must not produce any usage events"
        );
    }

    /// Real-data regression test: a genuine Codex `function_call_output`
    /// (a real tool action — NOT excluded by the message/session_meta type
    /// check) whose content is a git-status-style listing mentioning many
    /// unrelated skills' paths (e.g. "A .claude/skills/<name>" for a batch
    /// of newly-added files) must produce ZERO usage events for any of
    /// them — a real single-skill invocation only ever references its own
    /// skill, so a block naming several different skills at once is a
    /// listing, not evidence of using any specific one.
    #[test]
    fn codex_function_call_output_listing_many_skills_produces_zero_events() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root).unwrap();

        let file = root.join("rollout-2026-07-01.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-07-01T00:00:00.000Z","type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":"status:\nA .claude/skills/pulumi-esc\nA .claude/skills/pulumi-neo-handoff\nA .claude/skills/package-usage\n"}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec![
            "pulumi-esc".to_string(),
            "pulumi-neo-handoff".to_string(),
            "package-usage".to_string(),
        ];
        let mut events = HashSet::new();
        scan_root(root, "codex", &chef_skill_names, long_ago(), &mut events);

        assert!(
            events.is_empty(),
            "a function_call_output listing several unrelated skills' paths at once \
             must not produce any usage events, even though it's a genuine tool action"
        );
    }

    /// A genuine single-skill reference inside a real `function_call_output`
    /// (e.g. actually reading that one skill's SKILL.md) must still be
    /// detected — the multi-skill-listing filter must not swallow real,
    /// precise single-skill signals.
    #[test]
    fn codex_function_call_output_single_skill_still_detected() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root).unwrap();

        let file = root.join("rollout-2026-07-02.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-07-02T00:00:00.000Z","type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":"cat /home/user/.codex/skills/deploy_www/SKILL.md"}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["deploy_www".to_string()];
        let mut events = HashSet::new();
        scan_root(root, "codex", &chef_skill_names, long_ago(), &mut events);

        assert_eq!(
            events.len(),
            1,
            "a genuine single-skill reference must still count"
        );
        let event = events.iter().next().unwrap();
        assert_eq!(event.skill, "deploy_www");
    }

    // ===== Context (cwd) extraction tests =====

    #[test]
    fn claude_code_events_carry_cwd_from_any_line_with_top_level_cwd_field() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let project_dir = root.join("-Users-alp-project");
        fs::create_dir_all(&project_dir).unwrap();

        let file = project_dir.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n{}\n",
                r#"{"timestamp":"2026-06-03T00:00:00.000Z","type":"attachment","cwd":"/Users/alp/project","content":"unrelated context"}"#,
                r#"{"timestamp":"2026-06-03T01:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/SKILL.md"}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string()];
        let mut events = HashSet::new();
        scan_root(
            root,
            "claude_code",
            &chef_skill_names,
            long_ago(),
            &mut events,
        );

        assert_eq!(events.len(), 1);
        let event = events.iter().next().unwrap();
        assert_eq!(event.context, "/Users/alp/project");
    }

    #[test]
    fn codex_events_carry_cwd_from_session_meta_double_nested_payload() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root).unwrap();

        let file = root.join("rollout-2026-06-04.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n{}\n",
                r#"{"timestamp":"2026-06-04T00:00:00.000Z","type":"response_item","payload":{"type":"session_meta","payload":{"cwd":"/home/user/project"}}}"#,
                r#"{"timestamp":"2026-06-04T01:00:00.000Z","type":"function_call_output","payload":{"content":"cat /home/user/.codex/skills/deploy_www/SKILL.md"}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["deploy_www".to_string()];
        let mut events = HashSet::new();
        scan_root(root, "codex", &chef_skill_names, long_ago(), &mut events);

        assert_eq!(events.len(), 1);
        let event = events.iter().next().unwrap();
        assert_eq!(event.context, "/home/user/project");
    }

    #[test]
    fn missing_cwd_yields_empty_string_context_not_failure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root).unwrap();

        let file = root.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-06-05T00:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/SKILL.md"}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string()];
        let mut events = HashSet::new();
        scan_root(
            root,
            "claude_code",
            &chef_skill_names,
            long_ago(),
            &mut events,
        );

        assert_eq!(events.len(), 1);
        let event = events.iter().next().unwrap();
        assert_eq!(event.context, "");
    }

    /// The `context` field must not break the existing HashSet-based dedup:
    /// two events differing ONLY in context still collapse to one if their
    /// (skill, date, client) triple matches (dedup key is the whole struct
    /// via derived Hash/Eq, but in practice, since a file has one cwd, a
    /// given triple only ever gets one context value — proven here by
    /// showing that inserting two logically-identical events (same skill,
    /// date, client, AND context, since that's what a real single-cwd file
    /// produces) still yields one event, and that the batch DB layer's
    /// `INSERT OR IGNORE` primary key of (skill, date, client) also
    /// collapses regardless of context.
    #[test]
    fn context_field_does_not_break_dedup() {
        let mut set: HashSet<UsageEvent> = HashSet::new();
        set.insert(UsageEvent {
            skill: "ponytail".to_string(),
            date: "2026-06-06".to_string(),
            client: "claude_code".to_string(),
            context: "/Users/alp/project".to_string(),
            count: 1,
        });
        set.insert(UsageEvent {
            skill: "ponytail".to_string(),
            date: "2026-06-06".to_string(),
            client: "claude_code".to_string(),
            context: "/Users/alp/project".to_string(),
            count: 1,
        });
        assert_eq!(
            set.len(),
            1,
            "identical events (including context) must collapse"
        );

        // Two events with the SAME (skill, date, client) but DIFFERENT
        // context are, by design, distinct HashSet members (context is part
        // of Hash/Eq). In the DB layer, per-file identity (`file_path`) is
        // what actually determines the persisted primary key
        // (skill, file_path, client), so this in-memory HashSet divergence
        // is orthogonal to (doesn't need to match) real persisted rows.
        set.insert(UsageEvent {
            skill: "ponytail".to_string(),
            date: "2026-06-06".to_string(),
            client: "claude_code".to_string(),
            context: "/Users/alp/other-project".to_string(),
            count: 1,
        });
        assert_eq!(
            set.len(),
            2,
            "differing context makes two otherwise-identical events distinct \
             HashSet members; real per-row identity in the persisted cache \
             is enforced by the (skill, file_path, client) primary key, not \
             by this in-memory struct equality"
        );
    }

    // ===== Cache-version auto-invalidation tests =====

    #[test]
    fn stale_or_missing_cache_version_triggers_full_rebuild_and_clears_poison() {
        let db_tmp = tempfile::tempdir().expect("db tempdir");
        let db = open_test_db(&db_tmp);

        // Simulate pre-fix poisoned cache data: a usage_events row for a
        // skill that was never really used (a false-positive catalog-dump
        // hit), inserted directly with no corresponding real transcript
        // file, and NO version sentinel row (simulating a DB created before
        // this feature shipped).
        let poison_path = "/Users/alp/.claude/projects/poisoned/session.jsonl".to_string();
        let mut poison_events = HashSet::new();
        poison_events.insert(UsageEvent {
            skill: "google-cloud-waf-security".to_string(),
            date: "2026-01-01".to_string(),
            client: "codex".to_string(),
            context: String::new(),
            count: 1,
        });
        db.sync_usage_batch(&[(poison_path.clone(), "123456".to_string(), poison_events)])
            .expect("seed poisoned cache");

        let poisoned = db.query_usage_events("2000-01-01").expect("query poisoned");
        assert_eq!(
            poisoned.len(),
            1,
            "poisoned row must exist before the fix runs"
        );

        assert!(
            db.usage_file_mtime(USAGE_CACHE_VERSION_SENTINEL_PATH)
                .unwrap()
                .is_none(),
            "no version sentinel must exist yet, simulating a pre-upgrade DB"
        );

        // Real transcript fixture that should survive/rebuild correctly.
        let transcripts_tmp = tempfile::tempdir().expect("transcripts tempdir");
        let root = transcripts_tmp.path().to_path_buf();
        fs::create_dir_all(&root).unwrap();
        let file = root.join("session1.jsonl");
        fs::write(
            &file,
            format!(
                "{}\n",
                r#"{"timestamp":"2026-06-07T00:00:00.000Z","message":{"content":[{"type":"tool_result","content":"/Users/alp/.claude/skills/ponytail/SKILL.md"}]}}"#,
            ),
        )
        .unwrap();

        let chef_skill_names = vec!["ponytail".to_string()];
        let roots = [(root.clone(), "claude_code")];

        sync_roots(&db, &chef_skill_names, &roots).expect("sync with stale/missing version");

        let after = db
            .query_usage_events("2000-01-01")
            .expect("query after rebuild");
        assert!(
            !after.iter().any(|e| e.skill == "google-cloud-waf-security"),
            "poisoned event must be gone after auto-invalidation"
        );
        assert!(
            after
                .iter()
                .any(|e| e.skill == "ponytail" && e.date == "2026-06-07"),
            "real transcript fixture must be correctly rebuilt after invalidation"
        );

        let version_after = db
            .usage_file_mtime(USAGE_CACHE_VERSION_SENTINEL_PATH)
            .expect("version lookup");
        assert_eq!(
            version_after.as_deref(),
            Some(USAGE_CACHE_VERSION),
            "version marker must be written after invalidation runs"
        );

        // A second sync with the marker now in place must NOT wipe the
        // rebuilt data again.
        sync_roots(&db, &chef_skill_names, &roots).expect("second sync, version now current");
        let after_second = db
            .query_usage_events("2000-01-01")
            .expect("query after second sync");
        assert!(
            after_second
                .iter()
                .any(|e| e.skill == "ponytail" && e.date == "2026-06-07"),
            "data must persist once the version marker is current"
        );
    }
}
