# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Skill Cookbook Relay** is a Tauri 2 + Rust desktop application that acts as a secure local relay between AI agents and the Runwaize Skills Cookbook (RSS API). It exposes skills via MCP (Model Context Protocol) on `localhost:9876` and runs a bridge HTTP server on `localhost:9123` for the web UI at `skills.runwaize.com`.

## Build & Development Commands

All commands use `just` (justfile) or `cargo` directly:

```bash
just dev              # Run Tauri dev server (cargo tauri dev)
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
```

Test modules live alongside source as `src/*_tests.rs` files (e.g., `crypto_tests.rs`, `config_tests.rs`, `error_tests.rs`, `types_tests.rs`). They are conditionally compiled via `#[cfg(test)]` in `src/lib.rs`.

## Code Quality

```bash
just fmt              # Format code (cargo fmt --all)
just fmt-check        # Check formatting
just lint             # Clippy with -D warnings
just quality          # fmt-check + lint + test
just ci               # Full CI: fmt-check + lint + test + build-release
```

## Architecture

### Two HTTP Servers

The app runs **two separate axum HTTP servers** concurrently (spawned as tokio tasks in `main.rs`):

1. **MCP Server** (`src/mcp/`) — Port 9876. JSON-RPC endpoint for AI agents. Methods: `skills.list`, `skills.get`, `libraries.list`, `libraries.get`, `skills.status`, `skills.refresh`. Single POST route dispatched by method name in `mcp/server.rs` → `mcp/handlers.rs`.

2. **Bridge Server** (`src/bridge/`) — Port 9123. REST API for the web frontend (`skills.runwaize.com`). Routes split into public (`/bridge/ping`, `/bridge/handshake`) and authenticated (`/bridge/status`, `/bridge/refresh`, `/bridge/cache/clear`, `/bridge/expose`, `/bridge/update/*`, `/bridge/logs/export`). Auth middleware validates session tokens and origin.

### Core Components

- **`relay.rs`** — `RelayState` is the central orchestrator. Holds `Config`, `AuthManager`, `RssClient`, `ArtifactCache`, `VariableResolver`, and signing key. Managed as `Arc<RelayState>` shared across both servers and Tauri commands.
- **`auth.rs`** — OAuth device flow authentication. Tokens stored in OS keychain via `keyring` crate.
- **`rss_client.rs`** — HTTP client for the RSS API (skills.cookbook backend).
- **`cache.rs`** — Local artifact cache using `sled` embedded database with integrity verification.
- **`crypto.rs`** — Artifact signature verification (RSA/ED25519) and hash checking (SHA-256).
- **`variables.rs`** — Secret/variable resolution with scope priority: project → workspace → personal → defaults.
- **`config.rs`** — TOML config loaded from `~/{config_dir}/skill-cookbook-relay/config.toml`. Supports `save()` for persistence. Includes `chef_dir`, `guest_dir`, `workspace_id` for CLI.
- **`types.rs`** — Shared type definitions for MCP requests/responses, artifacts, libraries, etc.
- **`error.rs`** — `RelayError` enum (thiserror) with `Result<T>` type alias.

### Feature Flags

- `custom-protocol` (default, enabled) — Tauri production mode. Enables bridge with `AppHandle` for native dialogs (confirmation prompts before cache clear, expose changes, updates). Without this flag, bridge runs without Tauri app context (used in dev/testing).

### Tauri Commands

Defined in `main.rs` as `#[tauri::command]` functions: `get_relay_status`, `login_to_rss`, `logout_from_rss`, `refresh_skills`, `list_libraries`, `clear_cache`, `wipe_all_data`. These delegate to `RelayState` methods.

### Frontend

The UI is a remote web app at `skills.runwaize.com` loaded in the Tauri webview (not a local SPA). The `ui/` directory contains a minimal offline fallback page. The window navigates to `/inbox` if authenticated or `/account/login` otherwise.

## API Compatibility Notes

The relay types use serde aliases for RSS API field compatibility:
- `Library`: `library_id` (alias: `id`), `visibility` (alias: `scope`)
- `SkillSummary`: `skill_id` (alias: `id`)
- `UpdatesSinceResponse`: `updates` (alias: `changes`)

## Key Conventions

- Concurrency: `parking_lot::RwLock` for synchronous state, `Arc` for shared ownership across async tasks
- Error handling: All modules use `crate::error::{RelayError, Result}`
- Logging: `tracing` crate with `RUST_LOG` env filter (default: `skill_cookbook_relay=info,tower_http=debug`)
- Binary name: `skill-cookbook-relay` (crate name: `skill-cookbook-relay`)
- **CLI**: Crate `runwaize_skills_cookbook_cli`, binary `skills_cookbook`. Lives in `cli/`. Shared config/auth with relay.
