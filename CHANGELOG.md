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
