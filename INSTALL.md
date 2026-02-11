# Installation Guide - Skills Cookbook Relay

Complete installation instructions for all supported platforms.

## Table of Contents

- [macOS Installation](#macos-installation)
- [Linux Installation](#linux-installation)
- [Windows Installation](#windows-installation)
- [Building from Source](#building-from-source)
- [Post-Installation Setup](#post-installation-setup)
- [Uninstallation](#uninstallation)

---

## macOS Installation

### System Requirements

- macOS 10.15 (Catalina) or later
- 50 MB free disk space
- Internet connection for authentication

### Method 1: DMG Installer (Recommended)

1. **Download the installer**
   - Visit [Releases](https://github.com/SUPERVAIZE/skillsstudiorelay/releases)
   - Download `Skills-Cookbook-Relay-{version}.dmg`

2. **Install the application**
   ```bash
   # Open the DMG
   open Skills-Cookbook-Relay-*.dmg

   # Drag to Applications folder
   # Or use command line:
   cp -r "/Volumes/Skills Cookbook Relay/Skills Cookbook Relay.app" /Applications/
   ```

3. **First launch**
   - Open from Applications folder or Spotlight
   - macOS may show a security warning for unsigned apps
   - Go to **System Preferences → Security & Privacy → General**
   - Click "Open Anyway" if prompted

4. **Set auto-start (optional)**
   - Open **System Preferences → Users & Groups → Login Items**
   - Click **+** and add Skills Cookbook Relay

### Method 2: Homebrew

```bash
# Add the tap
brew tap supervaize/skills-cookbook

# Install
brew install skills-cookbook-relay

# Run
skills-cookbook-relay
```

---

## Linux Installation

### System Requirements

- Ubuntu 20.04+, Debian 11+, Fedora 35+, or equivalent
- 50 MB free disk space
- `libwebkit2gtk-4.0` and `libgtk-3-0` libraries

### Ubuntu/Debian

```bash
# Install dependencies
sudo apt update
sudo apt install libwebkit2gtk-4.0-dev \
                 libgtk-3-dev \
                 libayatana-appindicator3-dev \
                 librsvg2-dev

# Download .deb package
wget https://github.com/SUPERVAIZE/skillsstudiorelay/releases/download/v0.1.0/skills-cookbook-relay_0.1.0_amd64.deb

# Install
sudo dpkg -i skills-cookbook-relay_0.1.0_amd64.deb

# Fix dependencies if needed
sudo apt-get install -f

# Run
skills-cookbook-relay
```

### Fedora/RHEL

```bash
# Install dependencies
sudo dnf install webkit2gtk3-devel \
                 gtk3-devel \
                 librsvg2-devel

# Download .rpm package
wget https://github.com/SUPERVAIZE/skillsstudiorelay/releases/download/v0.1.0/skills-cookbook-relay-0.1.0.x86_64.rpm

# Install
sudo rpm -i skills-cookbook-relay-0.1.0.x86_64.rpm

# Run
skills-cookbook-relay
```

### Auto-start on Linux

Create a systemd user service:

```bash
# Create service file
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/skills-cookbook-relay.service <<EOF
[Unit]
Description=Skills Cookbook Relay
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/skills-cookbook-relay
Restart=on-failure

[Install]
WantedBy=default.target
EOF

# Enable and start
systemctl --user enable skills-cookbook-relay
systemctl --user start skills-cookbook-relay

# Check status
systemctl --user status skills-cookbook-relay
```

---

## Windows Installation

### System Requirements

- Windows 10 (1809+) or Windows 11
- 50 MB free disk space
- Internet connection for authentication

### Method 1: MSI Installer (Recommended)

1. **Download the installer**
   - Visit [Releases](https://github.com/SUPERVAIZE/skillsstudiorelay/releases)
   - Download `Skills-Cookbook-Relay-{version}.msi`

2. **Run the installer**
   - Double-click the MSI file
   - Follow the installation wizard
   - Choose installation directory (default: `C:\Program Files\Skills Cookbook Relay`)

3. **First launch**
   - Find in Start Menu: "Skills Cookbook Relay"
   - Or run from command line: `skills-cookbook-relay.exe`

4. **Set auto-start (optional)**
   - Press `Win+R` and type `shell:startup`
   - Create shortcut to Skills Cookbook Relay in this folder

### Method 2: Portable ZIP

```powershell
# Download portable version
Invoke-WebRequest -Uri "https://github.com/SUPERVAIZE/skillsstudiorelay/releases/download/v0.1.0/skills-cookbook-relay-portable.zip" -OutFile "skills-cookbook-relay.zip"

# Extract
Expand-Archive -Path "skills-cookbook-relay.zip" -DestinationPath "C:\Skills-Cookbook-Relay"

# Run
C:\Skills-Cookbook-Relay\skills-cookbook-relay.exe
```

---

## Building from Source

### Prerequisites

Install Rust toolchain:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Clone and Build

```bash
# Clone repository
git clone https://github.com/SUPERVAIZE/skillsstudiorelay.git
cd skillsstudiorelay

# Build release version
cargo build --release

# The binary will be in target/release/
./target/release/skills-cookbook-relay
```

### Platform-Specific Build Steps

#### macOS

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Build
cargo build --release

# Create DMG (optional)
cargo tauri build
```

#### Linux

```bash
# Install build dependencies
sudo apt install libwebkit2gtk-4.0-dev \
                 build-essential \
                 curl \
                 wget \
                 libssl-dev \
                 libgtk-3-dev \
                 libayatana-appindicator3-dev \
                 librsvg2-dev

# Build
cargo build --release
```

#### Windows

```powershell
# Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/downloads/

# Build
cargo build --release
```

---

## Post-Installation Setup

### 1. Verify Installation

```bash
# Check if relay is running
curl http://localhost:9876

# Expected: Connection refused (if not authenticated yet)
# or JSON-RPC response (if running)
```

### 2. Configure Your AI Agent

#### Claude Desktop

Edit: `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS)
Or: `%APPDATA%\Claude\claude_desktop_config.json` (Windows)

```json
{
  "mcpServers": {
    "skills-cookbook": {
      "url": "http://localhost:9876"
    }
  }
}
```

#### OpenClaw

Edit your OpenClaw config:

```yaml
mcp_servers:
  skills-cookbook:
    url: http://localhost:9876
    enabled: true
```

### 3. Authenticate

1. Open Skills Cookbook Relay dashboard
2. Click "Connect Account"
3. Follow the OAuth flow in your browser

### 4. Test the Connection

```bash
# Use curl to test MCP endpoint
curl -X POST http://localhost:9876 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "skills.status",
    "params": {}
  }'
```

---

## Uninstallation

### macOS

```bash
# Remove application
rm -rf /Applications/Skills\ Cookbook\ Relay.app

# Remove configuration and cache
rm -rf ~/Library/Application\ Support/skills-cookbook-relay
rm -rf ~/Library/Caches/skills-cookbook-relay
rm -rf ~/Library/Logs/skills-cookbook-relay

# Remove keychain entries (optional)
security delete-generic-password -s "com.supervaize.skills-cookbook-relay"
```

### Linux

```bash
# Ubuntu/Debian
sudo apt remove skills-cookbook-relay

# Fedora
sudo dnf remove skills-cookbook-relay

# Remove user data
rm -rf ~/.config/skills-cookbook-relay
rm -rf ~/.cache/skills-cookbook-relay
```

### Windows

```powershell
# Use Windows Settings → Apps → Uninstall
# Or use command line:
msiexec /x "Skills-Cookbook-Relay-0.1.0.msi"

# Remove user data
Remove-Item -Recurse "$env:APPDATA\skills-cookbook-relay"
Remove-Item -Recurse "$env:LOCALAPPDATA\skills-cookbook-relay"
```

---

## Troubleshooting Installation

### Port Already in Use

If port 9876 is taken:

```toml
# Edit config.toml
mcp_server_port = 9877  # Use different port
```

### Permission Denied (macOS)

```bash
# Reset app quarantine
xattr -cr /Applications/Skills\ Cookbook\ Relay.app
```

### Missing Dependencies (Linux)

```bash
# Check what's missing
ldd $(which skills-cookbook-relay)

# Install common missing libs
sudo apt install libssl1.1 libwebkit2gtk-4.0-37
```

### Firewall Issues

Make sure localhost connections are allowed:

```bash
# macOS
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --add /Applications/Skills\ Cookbook\ Relay.app

# Linux (UFW)
sudo ufw allow 9876/tcp
```

---

## Next Steps

After installation:

1. Read the [README.md](README.md) for usage guide
2. Check [DEVELOPER.md](DEVELOPER.md) for advanced configuration
3. Join our [Discord](https://discord.gg/supervaize) for support

## Support

Having installation issues?

- 📧 Email: support@supervaize.com
- 🐛 Report: [GitHub Issues](https://github.com/SUPERVAIZE/skillsstudiorelay/issues)
- 💬 Chat: [Discord Community](https://discord.gg/supervaize)
