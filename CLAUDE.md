# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Skill Cookbook Relay** is a Tauri 2 + Rust desktop application that acts as a secure local relay between AI agents and the Runwaize Skills Cookbook (RSS API). It exposes skills via MCP (Model Context Protocol) on `localhost:9876` and runs a bridge HTTP server on `localhost:9123` for the web UI at `skills.runwaize.com`.

## Build & Development Commands

All commands use `just` (justfile) or `cargo` directly:

```bash
just dev              # Run Tauri dev server (cargo tauri dev)
just dev-local        # Run against local web app (TAURI_DEV_WEB=http://localhost:5175)
just check            # Fast error check (cargo check --all-targets)
just build-release    # Build optimized binary (cargo build --release)
just build-app        # Build Tauri app bundle (cargo tauri build)
```

### Skills Cookbook CLI

The repo includes a companion CLI (`skills_cookbook`) for chef/guest workflows. The desktop app uses the CLI **library** for local filesystem operations; the binary can be used standalone.

```bash
just cli              # Build release CLI → target/release/skills_cookbook
just cli-run          # Run CLI (default: usage = help)
just cli-run init     # e.g. init, doctor, login, list, sync
just cli-test         # Run CLI tests only
```

Or with cargo:

```bash
cargo build -p runwaize_skills_cookbook_cli --release
./target/release/skills_cookbook usage
./target/release/skills_cookbook init
./target/release/skills_cookbook doctor
```

## Testing

```bash
just test                    # Run all tests (cargo test)
just test-one <TEST>         # Run specific test with output (cargo test <TEST> -- --nocapture)
just test-verbose            # All tests with output
just test-delivery           # Full pipeline: fmt-check → lint → test
just cli-test                # CLI tests only (cargo test -p runwaize_skills_cookbook_cli)
```

**Relay tests** live alongside source as `src/*_tests.rs` files (e.g., `crypto_tests.rs`, `config_tests.rs`, `error_tests.rs`, `types_tests.rs`). They are conditionally compiled via `#[cfg(test)]` in `src/lib.rs`. Some tests use `serial_test::serial` for sequential execution (config file conflicts) and `tempfile::tempdir()` for isolated dirs.

**CLI tests** are in `cli/src/cli_tests.rs`, use `CONFIG_DIR` env var to sandbox config.

## Runwaize Coordination

- Weekly session reviews are coordinated from the root RUNWAIZE repo via `../docs/weekly-session-review.md` and `../scripts/weekly_session_review.py`; include Skills Cookbook-specific findings when reviewing this repo.

## Code Quality

```bash
just fmt              # Format code (cargo fmt --all)
just fmt-check        # Check formatting
just lint             # Clippy with -D warnings
just quality          # fmt-check + lint + test
just ci               # Full CI: fmt-check + lint + test + build-release
just precommit        # Quick: fmt + check + test
```

## Architecture

### Workspace Structure

Two crates in one workspace:

- **Root (`skill-cookbook-relay`)** — Tauri desktop app + relay library. Binary: `skill-cookbook-relay`.
- **`cli/` (`runwaize_skills_cookbook_cli`)** — CLI crate. Binary: `skills_cookbook`. Depends on the relay crate (`skill-cookbook-relay = { path = ".." }`) for shared `Config`, `AuthManager`, `RssClient`, and types.

### Two HTTP Servers

The app runs **two separate axum HTTP servers** concurrently (spawned as tokio tasks in `main.rs`):

1. **MCP Server** (`src/mcp/`) — Port 9876. JSON-RPC endpoint for AI agents. Methods: `skills.list`, `skills.get`, `libraries.list`, `libraries.get`, `skills.status`, `skills.refresh`. Single POST route dispatched by method name in `mcp/server.rs` → `mcp/handlers.rs`.

2. **Bridge Server** (`src/bridge/`) — Port 9123. REST API for the web frontend (`skills.runwaize.com`). Routes split into public (`/bridge/ping`, `/bridge/handshake`) and authenticated (`/bridge/status`, `/bridge/refresh`, `/bridge/cache/clear`, `/bridge/expose`, `/bridge/update/*`, `/bridge/logs/export`). Includes discovery routes (`/bridge/discover-apps`, `/bridge/discovery/scan`, `/bridge/discovery/import`). Auth middleware validates session tokens and origin.

### Core Components

- **`relay.rs`** — `RelayState` is the central orchestrator. Holds `Config`, `AuthManager`, `RssClient`, `ArtifactCache`, `VariableResolver`, and signing key. Managed as `Arc<RelayState>` shared across both servers and Tauri commands. Also reads guest manifest from `guest_dir` and spawns background sync tasks.
- **`auth.rs`** — OAuth device flow authentication. Tokens stored in OS keychain (`com.supervaize.skill-cookbook-relay`) via `keyring` crate.
- **`rss_client.rs`** — HTTP client for the RSS API (skills.cookbook backend).
- **`studio_client.rs`** — HTTP client for the Skills Studio ingest API (`app.supervaize.com`). Handles bearer auth multipart skill uploads.
- **`cache.rs`** — Local artifact cache using `sled` embedded database with integrity verification and TTL-based expiry.
- **`crypto.rs`** — Artifact signature verification (RSA/ED25519) and hash checking (SHA-256).
- **`variables.rs`** — Secret/variable resolution with scope priority: project → workspace → personal → defaults. Sources: keychain, env vars, 1Password CLI.
- **`config.rs`** — TOML config loaded from `~/{config_dir}/skill-cookbook-relay/config.toml`. Supports `save()` for persistence. Includes `chef_dir`, `guest_dir`, `workspace_id` for CLI. Overridable via `CONFIG_DIR` env var (used in tests).
- **`discovery/`** — Scan local filesystem for SKILL.md files, detect installed AI clients (Cursor, Claude Code, Codex, Codeium, Windsurf, Aider, Zed), build skill zips for upload.
- **`guest_manifest.rs`** — Reads `manifest.json` from `guest_dir` for curated skill list.
- **`types.rs`** — Shared type definitions for MCP requests/responses, artifacts, libraries, etc.
- **`error.rs`** — `RelayError` enum (thiserror) with `Result<T>` type alias.

### Guest Manifest: Dual Implementation

The guest manifest types (`GuestSkillEntry`, `GuestManifest`) exist in **both** `src/guest_manifest.rs` (relay) and `cli/src/guest.rs` (CLI). The CLI writes `manifest.json` to `guest_dir` via `sync_guest_from_server()`. The relay reads it via `RelayState::get_guest_manifest()`. This avoids a circular dependency since the relay binary doesn't depend on the CLI.

### CLI Commands

The CLI (`cli/src/lib.rs`) uses clap subcommands: `usage`, `init`, `doctor`, `login`, `logout`, `workspace select`, `list`, `sync`, `add`, `skill status`, `library {add,remove,add-skill,remove-skill}`, `install`, `remove`, `update`. Each command is in its own module under `cli/src/`.

### Feature Flags

- `custom-protocol` (default, enabled) — Tauri production mode. Enables bridge with `AppHandle` for native dialogs (confirmation prompts before cache clear, expose changes, updates). Without this flag, bridge runs without Tauri app context (used in dev/testing).

### Tauri Commands

Defined in `main.rs` as `#[tauri::command]` functions: `get_relay_status`, `login_to_rss`, `logout_from_rss`, `refresh_skills`, `list_libraries`, `clear_cache`, `wipe_all_data`. These delegate to `RelayState` methods.

### Frontend

The UI is a remote web app at `skills.runwaize.com` loaded in the Tauri webview (not a local SPA). The `ui/` directory contains a minimal offline fallback page. The window navigates to `/inbox` if authenticated or `/account/login` otherwise. Appends `?tauri_version=X.Y.Z` so the web app detects the Tauri context and enables bridge communication.

### Bridge Detection

The web app only communicates with the local bridge when `tauri_version` query param is present. In a regular browser (no param), no bridge requests are made.

## API Compatibility Notes

The relay types use serde aliases for RSS API field compatibility:
- `Library`: `library_id` (alias: `id`), `visibility` (alias: `scope`)
- `SkillSummary`: `skill_id` (alias: `id`)
- `UpdatesSinceResponse`: `updates` (alias: `changes`)

## Key Conventions

- **Concurrency**: `parking_lot::RwLock` for synchronous state, `Arc` for shared ownership across async tasks
- **Error handling**: All modules use `crate::error::{RelayError, Result}`
- **Logging**: `tracing` crate with `RUST_LOG` env filter (default: `skill_cookbook_relay=info,tower_http=debug`)
- **Binary names**: Relay binary is `skill-cookbook-relay`; CLI binary is `skills_cookbook` (different from crate name `runwaize_skills_cookbook_cli`)
- **Local dev**: `TAURI_DEV_WEB` env var overrides frontend URL. System tray shows "Local Dev" vs "Live"
- **CSRF note**: When using `dev-local`, Django backend must include `"null"` in `CSRF_TRUSTED_ORIGINS` because WKWebView sends `Origin: null`

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **skills_cookbook** (11787 symbols, 14121 relationships, 286 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## When Debugging

1. `gitnexus_query({query: "<error or symptom>"})` — find execution flows related to the issue
2. `gitnexus_context({name: "<suspect function>"})` — see all callers, callees, and process participation
3. `READ gitnexus://repo/skills_cookbook/process/{processName}` — trace the full execution flow step by step
4. For regressions: `gitnexus_detect_changes({scope: "compare", base_ref: "main"})` — see what your branch changed

## When Refactoring

- **Renaming**: MUST use `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` first. Review the preview — graph edits are safe, text_search edits need manual review. Then run with `dry_run: false`.
- **Extracting/Splitting**: MUST run `gitnexus_context({name: "target"})` to see all incoming/outgoing refs, then `gitnexus_impact({target: "target", direction: "upstream"})` to find all external callers before moving code.
- After any refactor: run `gitnexus_detect_changes({scope: "all"})` to verify only expected files changed.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Tools Quick Reference

| Tool | When to use | Command |
|------|-------------|---------|
| `query` | Find code by concept | `gitnexus_query({query: "auth validation"})` |
| `context` | 360-degree view of one symbol | `gitnexus_context({name: "validateUser"})` |
| `impact` | Blast radius before editing | `gitnexus_impact({target: "X", direction: "upstream"})` |
| `detect_changes` | Pre-commit scope check | `gitnexus_detect_changes({scope: "staged"})` |
| `rename` | Safe multi-file rename | `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` |
| `cypher` | Custom graph queries | `gitnexus_cypher({query: "MATCH ..."})` |

## Impact Risk Levels

| Depth | Meaning | Action |
|-------|---------|--------|
| d=1 | WILL BREAK — direct callers/importers | MUST update these |
| d=2 | LIKELY AFFECTED — indirect deps | Should test |
| d=3 | MAY NEED TESTING — transitive | Test if critical path |

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/skills_cookbook/context` | Codebase overview, check index freshness |
| `gitnexus://repo/skills_cookbook/clusters` | All functional areas |
| `gitnexus://repo/skills_cookbook/processes` | All execution flows |
| `gitnexus://repo/skills_cookbook/process/{name}` | Step-by-step execution trace |

## Self-Check Before Finishing

Before completing any code modification task, verify:
1. `gitnexus_impact` was run for all modified symbols
2. No HIGH/CRITICAL risk warnings were ignored
3. `gitnexus_detect_changes()` confirms changes match expected scope
4. All d=1 (WILL BREAK) dependents were updated

## Keeping the Index Fresh

After committing code changes, the GitNexus index becomes stale. Re-run analyze to update it:

```bash
npx gitnexus analyze
```

If the index previously included embeddings, preserve them by adding `--embeddings`:

```bash
npx gitnexus analyze --embeddings
```

To check whether embeddings exist, inspect `.gitnexus/meta.json` — the `stats.embeddings` field shows the count (0 means no embeddings). **Running analyze without `--embeddings` will delete any previously generated embeddings.**

> Claude Code users: A PostToolUse hook handles this automatically after `git commit` and `git merge`.

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
