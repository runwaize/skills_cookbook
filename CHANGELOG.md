# Skill Cookbook Relay ChangeLog

All notable changes to this project will be documented in this file.

> The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
>
> | Emoji Legend |                |             |            |
> | ------------ | -------------- | ----------- | ---------- |
> | 🦀 Core      | 🔌 MCP Server  | 🔐 Auth     | 🛣️ Config  |
> | 🐛 Bug       | 🏗️ Build/Infra | 📖 Docs     | 🧪 Tests   |
> | 🧑‍🎨 UI        | 🥇 Performance | 🔒 Security | ✨ Feature |

## [TODO]

## [Unreleased]

### Added

- ✨ **CLI**: `search [path]` — recursively scan a directory (or auto-detected AI agent folders) for SKILL.md files, interactively select which to add to chef dir, and commit. Supports `--all` flag to skip the prompt.
- ✨ **CLI (runwaize_skills_cookbook_cli)**:
  - **Workspace**: `workspace select [id]` — read/save `workspace_id` in relay config; list stub.
  - **Chef**: `add <path>` (resolve SKILL.md, copy to chef dir, git add/commit), `sync [--no-push]` (git commit + zip upload via StudioClient), `list` (RSS libraries + skills).
  - **Guest**: `sync` replaces local manifest with server-curated list (RSS libraries + skills → `manifest.json` in guest dir).
  - **Library/skill**: `library add|remove|add-skill|remove-skill`, `skill status` — stubs (API TBD).
  - **Install**: `install` (copy exe to `~/.local/bin/skills_cookbook`), `remove`, `update` (stub).
- 🦀 **Guest manifest (relay)**: New `src/guest_manifest.rs` — `GuestManifest` / `GuestSkillEntry`, `read_guest_manifest(guest_dir)`. Relay reads the same `manifest.json` the CLI writes; no CLI dependency (avoids circular dep).
- 🦀 **Relay status**: `RelayStatus.guest_skill_count` and `RelayState::get_guest_manifest()` for Tauri/bridge to use curated skill list; binary crate `main.rs` includes `mod guest_manifest`.
- ✨ **Local discovery (bridge)**: New endpoints for agent discovery and local skill import.
  - `POST /bridge/discover-apps`: Detect installed skill-capable clients (Cursor, Codex, Claude Code, Codeium, Windsurf, Aider, Zed) by checking known config paths under the user directory.
  - `POST /bridge/discovery/scan`: Scan a filesystem path or client id for `SKILL.md` files; returns list of skills with path, name (from frontmatter), identifier, and client_id.
  - `POST /bridge/discovery/import`: Batch import local skills: given paths and a studio Bearer token, builds zip per skill and POSTs to the Skills Studio ingest API.
- 🛣️ **Config**: `studio_api_url` (default `https://app.supervaize.com/api/skills-studio/v1`) for the ingest/upload base URL.
- 🦀 **Studio client**: `StudioClient` in `src/studio_client.rs` for uploading skill zips to the Skills Studio ingest API with Bearer auth.
- 🦀 **Error variants**: `RelayError::Discovery` and `RelayError::Studio` for discovery and studio API failures.
- 🧪 **Unit tests**: Discovery (path resolution, scan, frontmatter, zip building), studio client, and error display tests in `discovery_tests.rs` and `studio_client_tests.rs`.
- ✨ **Local dev mode**: New `just dev-local` command and `TAURI_DEV_WEB` env var to run the app against a local web frontend (e.g. `localhost:5175`) instead of the live `skills.runwaize.com`.
- 🧑‍🎨 **System tray context menu**: Replaced the single left-click tray handler with a full context menu containing "About Skill Cookbook Relay", "Show Window", and "Quit" items.
- 🧑‍🎨 **About dialog**: Tray menu "About" item shows app version and which web source (Live/Local Dev) is active.
- 🦀 **`tauri_version` query parameter**: WebView URL now includes `?tauri_version=X.Y.Z` on startup so the web app can detect it's running inside Tauri and enable bridge communication.

### Changed

- ✨ **CLI**: Grouped `list`, `add`, `sync`, `search` under `chef` subcommand (`cook` alias). Added `chef list` (local skills, `--remote` for server) and `chef edit [name]` (interactive SKILL.md editor).
- 🦀 **Web source resolution**: Extracted URL resolution into a `WebSource` struct, centralizing the live-vs-local-dev decision early in startup instead of resolving it inline in multiple places.
- 🏗️ **Justfile cleanup**: Removed duplicate `start-dev` recipe; clarified `dev` recipe description.

### Fixed

- 🐛 **Compiler warnings**: Added `#[allow(unused_variables)]` to `logs_export` and `update_apply` bridge handlers to suppress warnings when building without the `custom-protocol` feature flag.
- 🐛 **Trailing newline**: Fixed missing trailing newline in `src/mcp/mod.rs`.

### Tests

| Status                  | Count | ⏲️     |
| ----------------------- | ----- | ------ |
| ✅ Passed src/lib.rs    | 37    | 12.72s |
| s ☑️ Passed src/main.rs | 12    | 0.11s  |

## [0.1.0] - 2026-02-12

### Added

- 🦀 **Skill Cookbook Relay**: Initial Tauri + Rust desktop application for secure skill delivery via MCP.
- 🔐 **OAuth device flow**: Authentication with Runwaize Skill Cookbook via device code flow; tokens stored in OS keychain (macOS Keychain).
- 🔌 **MCP server**: HTTP server on localhost:9876 exposing `skills.list`, `skills.get`, `libraries.list`, `libraries.get`, `skills.status`, `skills.refresh`.
- 🔒 **Cryptographic verification**: Ed25519/RSA signature verification, SHA-256 hash checking, revocation handling for skill artifacts.
- 💾 **Local artifact cache**: Sled-based cache with integrity verification, configurable TTL and max size.
- 🔑 **Variable resolver**: Secret resolution from OS keychain, environment variables, 1Password CLI.
- 🧑‍🎨 **Native UI**: System tray app with dashboard for monitoring connection status, cache, and sync.
- 🛣️ **Configuration**: TOML config at `~/Library/Application Support/skill-cookbook-relay/config.toml` (macOS).
- 📖 **Documentation**: README with architecture, quick start, MCP methods, security model, troubleshooting.
