# Skill Cookbook ChangeLog

All notable changes to this project will be documented in this file.

> The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
>
> | Emoji Legend |                        |               |                |
> | ------------ | ---------------------- | ------------- | -------------- |
> | 🦀 Core      | 🔌 MCP Server          | 🔐 Auth       | 🛣️ Config      |
> | 🐛 Bug       | 🏗️ Build/Infra         | 📖 Docs       | 🧪 Tests       |
> | 🧑‍🎨 UI        | 🥇 Performance         | 🔒 Security   | ✨ Feature     |

## [TODO]

## [Unreleased]

### Added

- ✨ **Local dev mode**: New `just dev-local` command and `TAURI_DEV_WEB` env var to run the app against a local web frontend (e.g. `localhost:5175`) instead of the live `skills.runwaize.com`.
- 🧑‍🎨 **System tray context menu**: Replaced the single left-click tray handler with a full context menu containing "About Skill Cookbook Relay", "Show Window", and "Quit" items.
- 🧑‍🎨 **About dialog**: Tray menu "About" item shows app version and which web source (Live/Local Dev) is active.

### Changed

- 🦀 **Web source resolution**: Extracted URL resolution into a `WebSource` struct, centralizing the live-vs-local-dev decision early in startup instead of resolving it inline in multiple places.
- 🏗️ **Justfile cleanup**: Removed duplicate `start-dev` recipe; clarified `dev` recipe description.

### Fixed

- 🐛 **Compiler warnings**: Added `#[allow(unused_variables)]` to `logs_export` and `update_apply` bridge handlers to suppress warnings when building without the `custom-protocol` feature flag.
- 🐛 **Trailing newline**: Fixed missing trailing newline in `src/mcp/mod.rs`.

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
