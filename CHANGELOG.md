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

- 🛣️ **Unified config directory**: All config, data, and workspace files consolidated under `~/.runwaize_skills_cookbook/`:
  - `config.toml` — app configuration
  - `device_id` — device identity
  - `webview/` — webview persistent storage
  - `chef/` — creator workspace (git repo with `skills/`)
  - `cook/` — consumer manifest (`manifest.json`)
  - Configurable via Settings page; moving the settings folder copies existing files and writes a redirect pointer at the default location.
- ✨ **Editable Settings page**: All settings are now editable inline with a Save button:
  - **Directories**: Settings folder (with move support), Chef directory, Cook directory — each with a native folder picker button (Tauri only).
  - **Ports**: MCP Server port, Bridge port.
  - Settings persist immediately to `config.toml` on save.
- ✨ **3 new Tauri commands**: `pick_folder` (native folder dialog via `tauri_plugin_dialog`), `update_config` (partial config update + save), `move_settings_dir` (copy files + write redirect).
- 🧑‍🎨 **Combined Dashboard**: Unified dashboard at `/` showing Chef skill count, Cook skill count, library count, MCP server status, and quick-nav cards to both Chef and Cook local skills.
- 🧑‍🎨 **Both roles visible**: Chef and Cook sidebar sections are always visible (no exclusive role selection). Users are both chef and cook simultaneously.
- 🧑‍🎨 **Cook Sync button**: ManifestView empty state now shows a "Sync from Server" button; populated state shows a Sync header button.
- 🦀 **Browser dev mode**: `isTauri` detection in `tauri.ts` with `safeInvoke()` wrapper. When running in browser (`npm run dev`), returns mock data for `getConfig`, `listLocalSkills`, `getGuestManifest`, `getRelayStatus`. File picker disabled in browser mode.
- 🧑‍🎨 **Local React UI**: New `local-ui/` React 19 + Vite 6 + Tailwind 4 + DaisyUI 5 application as the default Tauri frontend, replacing the remote `skills.runwaize.com` webview.
  - **Chef pages**: Dashboard, Local Skills (list/edit/open local SKILL.md files), Remote Skills (server libraries grouped by library), Skill Editor (inline SKILL.md editor with save), Discover (detect AI agents, scan for skills, checkbox import), Sync (git commit + push).
  - **Cook pages**: Local Skills (browse guest manifest with sync), Sync (fetch latest manifest from server).
  - **Shared pages**: Relay Status (MCP port, cache, auth, library count), Doctor (diagnostic checks with OK/Warning/Error), Settings (editable directories, ports).
  - **Design system**: Full Runwaize brand palette copied from Skills Studio (green #41e33b primary, dark theme, Outfit font, shadcn/ui new-york style).
  - **13 shadcn/ui components**: button, badge, card, dialog, input, label, separator, sheet, sidebar, skeleton, tabs, textarea, tooltip — copied from studio frontend.
- 🛣️ **Config**: `user_role` (chef/cook) and `ui_mode` (local/online) fields on `Config`, persisted in `config.toml`. Defaults: `cook` role, `local` UI mode.
- 🦀 **New types**: `LocalSkill`, `RemoteSkillInfo`, `SyncResult`, `DiagnosticCheck`, `DiagnosticStatus`, `ScannedSkillInfo`, `AppConfig` in `src/types.rs` for frontend communication.
- ✨ **19 Tauri commands**: `get_config`, `update_config`, `pick_folder`, `move_settings_dir`, `set_user_role`, `switch_ui_mode`, `list_local_skills`, `list_remote_skills`, `add_skill`, `read_skill_content`, `write_skill_content`, `open_skill_in_editor`, `sync_chef`, `discover_agents`, `scan_for_skills`, `import_skills`, `get_guest_manifest`, `sync_guest`, `run_doctor`, `init_cookbook`.
- 🧑‍🎨 **Tray menu**: Chef Mode / Cook Mode items (switch role, emit event to frontend), Local UI / Online UI items (navigate webview), separators between groups.
- ✨ **CLI structured APIs**: New public functions for Tauri consumption — `list_local_skills_structured()`, `list_remote_skills_structured()`, `add_skill_to_chef()`, `sync_chef_structured()`, `run_doctor_structured()`, `sync_guest_count()`, `scan_for_skills_structured()`, `import_skills_to_chef()`.
- 🦀 **Guest manifest write**: `write_guest_manifest()` in `src/guest_manifest.rs` for the `sync_guest` Tauri command.
- 🏗️ **Build integration**: `just ui-install`, `just ui-dev`, `just ui-build` recipes. Tauri config updated with `beforeDevCommand`, `devUrl`, `beforeBuildCommand`, `frontendDist` for local-ui.
- 🔒 **CSP update**: Extended Content Security Policy for local UI (`tauri:`, `ipc:`, fonts, `data:`, `blob:`).

### Changed

- 🛣️ **Config path**: Moved from `~/Library/Application Support/skill-cookbook-relay/config.toml` to `~/.runwaize_skills_cookbook/config.toml`. Supports redirect file (`base_dir_redirect`) for custom locations.
- 🛣️ **Default directories**: Chef defaults to `~/.runwaize_skills_cookbook/chef`, Cook (formerly "Guest") defaults to `~/.runwaize_skills_cookbook/cook`.
- 🧑‍🎨 **Renamed "Guest" to "Cook"**: All UI labels, field names (`guest_dir` → `cook_dir` in AppConfig), and page titles updated.
- 🧑‍🎨 **Default UI mode**: App now starts with the local React UI by default instead of loading `skills.runwaize.com`. Users can switch to online mode via tray menu, header button, or settings page.
- 🧑‍🎨 **Navigation**: Dashboard is a standalone top-level item. Chef and Cook each have their own sidebar section with "Local Skills" as the primary entry.
- 🛣️ **Window title**: Changed from "Skill Cookbook Relay" to "Skill Cookbook".
- 🏗️ **tauri.conf.json**: `frontendDist` changed from `ui` to `local-ui/dist`; uses `npm --prefix local-ui` for before commands.
- ✨ **CLI modules**: `chef`, `doctor`, `guest`, `init`, `search` modules made `pub` for library API access.
- ✨ **CLI defaults**: `init` command now uses `~/.runwaize_skills_cookbook/chef` and `~/.runwaize_skills_cookbook/cook` as default directories.

### Tests

| Status                  | Count | ⏲️     |
| ----------------------- | ----- | ------ |
| ✅ Passed CLI tests     | 38    | 0.06s  |
| ✅ Passed relay tests   | 12    | 0.09s  |

---

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
