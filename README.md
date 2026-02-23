# Skill Cookbook Relay

Local relay for Runwaize Skills Cookbook - A secure skill delivery system via MCP (Model Context Protocol).

## Overview

Skill Cookbook Relay is a lightweight desktop application that acts as a secure bridge between your AI agents and the Runwaize Skills Cookbook. It ensures you always have access to the latest approved skills without storing sensitive credentials in your agent runtime.

### Key Features

- 🔐 **Secure Authentication** - OAuth device flow with OS keychain storage
- ✅ **Cryptographic Verification** - All skills are signed and verified before use
- 🔄 **Always Up-to-Date** - Automatic background sync for latest approved versions
- 🔑 **Secret Management** - Resolves credentials from keychain, environment, or 1Password
- 🚀 **MCP Server** - Exposes skills via standardized Model Context Protocol
- 💾 **Smart Caching** - Local artifact cache with integrity verification
- 🎨 **Native UI** - System tray app with context menu and about dialog
- 🛠️ **Local Dev Mode** - Run against a local web frontend via `TAURI_DEV_WEB` env var

## Architecture

```
┌─────────────────┐
│   AI Agent      │ (Claude, OpenClaw, etc.)
│   (MCP Client)  │
└────────┬────────┘
         │ MCP Protocol (localhost:9876)
         │
┌────────▼────────────────────────────────┐
│  Skill Cookbook Relay (Tauri + Rust)   │
│  ┌──────────────────────────────────┐   │
│  │  MCP Server                      │   │
│  │  ├─ skills.list, skills.get      │   │
│  │  └─ libraries.list, status       │   │
│  ├──────────────────────────────────┤   │
│  │  Artifact Cache & Verification   │   │
│  │  ├─ Signature verification       │   │
│  │  ├─ Hash checking                │   │
│  │  └─ Revocation handling          │   │
│  ├──────────────────────────────────┤   │
│  │  Variable Resolver               │   │
│  │  ├─ OS Keychain                  │   │
│  │  ├─ Environment variables        │   │
│  │  └─ 1Password CLI (optional)     │   │
│  └──────────────────────────────────┘   │
└────────┬────────────────────────────────┘
         │ HTTPS + OAuth
         │
┌────────▼────────────────────────────────┐
│  Runwaize Skills Cookbook (RSS API)     │
│  ├─ Signed skill artifacts              │
│  ├─ Library management                  │
│  └─ Version control & approval          │
└─────────────────────────────────────────┘
```

### Bridge Detection

The web app at `skills.runwaize.com` only attempts to communicate with the local bridge (`127.0.0.1:9123`) when a `tauri_version` query parameter is present on the initial page load. The Tauri app appends `?tauri_version=X.Y.Z` automatically. When the page is opened in a regular browser (no parameter), no bridge requests are made.

## Installation

### Prerequisites

- macOS 10.15+ (Windows/Linux support coming soon)
- Rust 1.70+ (for building from source)

### Option 1: Install from Release (Recommended)

1. Download the latest `.dmg` from [Releases](https://github.com/SUPERVAIZE/skill_cookbook/releases)
2. Open the DMG and drag Skill Cookbook Relay to Applications
3. Launch the app from Applications or Spotlight

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/SUPERVAIZE/skill_cookbook.git
cd skill_cookbook

# Install dependencies
cargo build --release

# Run the application
cargo run --release
```

## Quick Start

### 1. Launch the Relay

Open Skill Cookbook Relay from your Applications folder. The app will:

- Start the MCP server on `localhost:9876`
- Start the bridge server on `localhost:9123`
- Show a system tray icon with a context menu (About, Show Window, Quit)
- Load the dashboard at `skills.runwaize.com` with `?tauri_version=X.Y.Z` so the web app enables bridge communication

### 2. Connect Your Account

1. Click "Connect Account" in the dashboard
2. Visit the provided URL in your browser
3. Enter the displayed code
4. Authorize the relay to access your Skills Cookbook account

### 3. Configure Your Agent

Add the relay to your AI agent's MCP configuration:

#### For Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "skill-cookbook": {
      "url": "http://localhost:9876",
      "type": "http"
    }
  }
}
```

#### For OpenClaw

Edit your OpenClaw configuration:

```yaml
mcp_servers:
  - name: skill-cookbook
    url: http://localhost:9876
    enabled: true
```

### 4. Use Skills in Your Agent

Once configured, your agent can access skills:

```
User: "List available skills from my cookbook"
Agent: [Uses skills.list MCP method]

User: "Run the data-analysis skill on my CSV"
Agent: [Uses skills.get to fetch verified skill]
```

## Configuration

Configuration file location:

- **macOS**: `~/Library/Application Support/skill-cookbook-relay/config.toml`
- **Linux**: `~/.config/skill-cookbook-relay/config.toml`
- **Windows**: `%APPDATA%\skill-cookbook-relay\config.toml`

### Example Configuration

```toml
rss_api_url = "https://api.skills.cookbook/v1"
oauth_client_id = "skills-relay-client"
oauth_auth_url = "https://auth.skills.cookbook/oauth/authorize"
oauth_token_url = "https://auth.skills.cookbook/oauth/token"

mcp_server_host = "127.0.0.1"
mcp_server_port = 9876
mcp_enable_stdio = true

cache_dir = "~/.cache/skill-cookbook-relay"
cache_max_size_mb = 500
cache_ttl_hours = 24

sync_interval_minutes = 15
verification_required = true
fail_closed_on_verification_error = true

log_level = "info"
debug_mode = false

default_libraries = []
update_mode = "latest_approved"
```

## Secret Management

The relay resolves variables and secrets in the following order:

1. **Project scope** (highest priority)
2. **Workspace scope**
3. **Personal scope**
4. **Default values** (from skill schema)

### Storing Secrets

```bash
# The relay stores secrets in OS keychain automatically
# You can also set them via environment variables:
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."

# Or use 1Password CLI references:
# op://vault/item/field
```

## MCP Methods

The relay exposes these MCP methods:

### `skills.list`

List available skills, optionally filtered by library.

**Parameters:**

- `library_id` (optional): Filter by library ID

**Returns:** Array of skill summaries

### `skills.get`

Get a specific skill with resolved variables.

**Parameters:**

- `skill_id` or `slug`: Skill identifier
- `version` (optional): Version to fetch (default: "latest_approved")

**Returns:** Complete artifact with metadata and payload

### `libraries.list`

List all accessible libraries.

**Returns:** Array of libraries with metadata

### `libraries.get`

Get a specific library.

**Parameters:**

- `library_id`: Library identifier
- `release` (optional): Release version (default: "latest_approved")

### `skills.status`

Get relay status information.

**Returns:** Status object with cache, sync, and connection info

### `skills.refresh`

Force refresh skills from Skills Cookbook.

**Returns:** Refresh result with update counts

## Security

### Cryptographic Verification

Every skill artifact is:

1. **Signed** by Skills Cookbook using RSA or ED25519
2. **Hash-verified** using SHA-256
3. **Approval-checked** to ensure it's in published state
4. **Revocation-checked** before serving

If verification fails, the relay **fails closed** and refuses to serve the artifact.

### Token Storage

- Access tokens are stored in OS keychain (macOS Keychain, Windows Credential Manager)
- Refresh tokens are encrypted at rest
- Tokens are never written to logs or exposed to agents

### Network Security

- All communication with Skills Cookbook uses HTTPS with TLS 1.3
- MCP server binds only to `127.0.0.1` (localhost)
- No inbound connections from external networks

## Troubleshooting

### Relay won't start

Check the logs:

```bash
tail -f ~/Library/Logs/skill-cookbook-relay/relay.log
```

### Authentication fails

1. Check your internet connection
2. Clear tokens: Click "Wipe All Data" in dashboard
3. Try logging in again

### Skills not updating

1. Check "Last Sync" in dashboard
2. Click "Force Refresh"
3. Verify you're authenticated

### MCP connection issues

1. Verify the relay is running (check system tray)
2. Confirm MCP server port: `lsof -i :9876`
3. Check agent MCP configuration

## Development

### Running in Development

```bash
# Run against live skills.runwaize.com
just dev

# Run against a local web frontend (e.g. localhost:5175)
just dev-local
# Or manually:
TAURI_DEV_WEB=http://localhost:5175 cargo tauri dev
```

The `TAURI_DEV_WEB` environment variable overrides the web frontend URL. When set, the system tray About dialog will show "Local Dev" instead of "Live" to indicate the active web source.

**CSRF in dev-local**: If login fails with "CSRF verification failed" in the Tauri app (but works in Chrome), the Django backend must include `"null"` in `CSRF_TRUSTED_ORIGINS`—WKWebView sends `Origin: null`. See [Documentation/troubleshooting.md](Documentation/troubleshooting.md).

### Running Tests

```bash
# Run all tests
cargo test
# Or via justfile:
just test

# Run full delivery pipeline test cycle (format + lint + test)
just test-delivery

# Run specific test module
cargo test crypto_tests

# Run with output
cargo test -- --nocapture
just test-verbose
```

### Skills Cookbook CLI

A companion CLI (`skills_cookbook`) supports chef (creator) and guest (consumer) workflows. The desktop app uses the CLI **library** for local filesystem operations; you can also run the binary standalone.

```bash
just cli              # Build release binary → target/release/skills_cookbook
just cli-run          # Run CLI (default: usage)
just cli-run init     # Create chef/guest dirs, init git in chef
just cli-run doctor   # Diagnose config, dirs, auth
just cli-test         # Run CLI tests
```

See [CLAUDE.md](CLAUDE.md) for full CLI usage.

### Debug Mode

Enable debug logging:

```toml
# In config.toml
debug_mode = true
log_level = "debug"
```

Or via environment:

```bash
RUST_LOG=debug cargo run
```

### Project Structure

```
skill_cookbook/
├── cli/                     # Skills Cookbook CLI (skills_cookbook binary)
│   ├── src/
│   │   ├── lib.rs           # Library API for Tauri
│   │   ├── main.rs          # CLI entrypoint
│   │   ├── init.rs          # init command (chef/guest dirs)
│   │   ├── doctor.rs        # doctor command
│   │   └── guest.rs         # Guest manifest (read/write)
│   └── Cargo.toml
├── src/
│   ├── main.rs           # Tauri application entry
│   ├── lib.rs            # Library exports
│   ├── auth.rs           # OAuth & token management
│   ├── cache.rs          # Artifact caching
│   ├── config.rs         # Configuration
│   ├── crypto.rs         # Signature verification
│   ├── error.rs          # Error types
│   ├── relay.rs          # Core orchestration
│   ├── rss_client.rs     # API client
│   ├── types.rs          # Type definitions
│   ├── variables.rs      # Secret resolution
│   ├── bridge/
│   │   ├── mod.rs
│   │   └── handlers.rs   # Bridge REST API handlers
│   └── mcp/
│       ├── mod.rs
│       ├── server.rs     # MCP HTTP server
│       └── handlers.rs   # MCP method handlers
├── ui/
│   ├── index.html        # Offline fallback page
│   ├── styles.css
│   └── app.js
├── Cargo.toml
├── tauri.conf.json
├── justfile              # Task runner (just)
└── README.md
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Copyright © 2026 SUPERVAIZE Team. All rights reserved.

## Support

- 📧 Email: support@supervaize.com
- 💬 Discord: [SUPERVAIZE Community](https://discord.gg/supervaize)
- 🐛 Issues: [GitHub Issues](https://github.com/SUPERVAIZE/skill_cookbook/issues)
