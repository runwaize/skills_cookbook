Specs

This requirement extends the Skills Cookbook Relay from a background delivery component (MCP server + signed skill sync + local secret resolution) into a full desktop experience where users can also manage their skills and libraries directly inside the app. The Relay application will embed the existing skills.runwaize.com web UI (so we reuse the same product surface and avoid duplicating frontend work), while adding a secure “desktop-only” capability layer for features the browser cannot safely provide—such as local cache control, MCP exposure settings, keychain-backed credential resolution, diagnostics export, and the desktop app update flow. The goal is a seamless, single place to curate skills, connect agents, and operate the local runtime safely, with clear separation between web features and privileged desktop functionality.

The skills.runwaize.com source code is maintained here: /Volumes/SSDext1TB/Documents/GitRepo/SUPERVAIZE/studio/frontend_skills_studio

1. Desktop App Architecture

1.1 Components
• Tauri UI Shell
• Webview rendering skills.runwaize.com
• Tray/menu + native dialogs
• Relay Core (Rust)
• MCP server (localhost/stdio)
• Auth session management
• Signed artifact cache + verification
• Secret resolution
• Desktop Bridge (Local API)
• A localhost RPC surface exposed by the relay core to the embedded web UI
• Allows the website to call “desktop-only” features safely

1.2 Principle: “Web UI can request, Relay decides”
• The embedded site can ask for actions.
• The relay enforces policy and executes locally.

⸻

2. skills.runwaize.com changes (Desktop-aware website)

2.1 Desktop detection and capability handshake

Add a desktop integration layer:
• On page load, website checks if running inside the desktop shell:
• window.**RUNWAIZE_DESKTOP** injected by Tauri or
• a ping to http://127.0.0.1:<port>/bridge/ping
• If available, show “Desktop features” UI (see 2.2).

2.2 Desktop-specific UI features (examples)

When in desktop mode, enable:
• Agent Connectivity
• Relay status: running/stopped
• MCP endpoint info + “copy config snippet”
• Local cache management
• cache size, last sync, “refresh now”, “clear cache”
• Credentials / variables
• show which variables are resolved locally (no secret values)
• “open keychain helper” / “test resolution”
• Install helpers
• “Expose library to agent” toggle (configures relay allowlist)
• “Pin library” vs “Latest approved”
• Diagnostics
• “export logs”
• “connectivity test” to RSS
• App Update
• show current version + channel + update availability + “update now”

2.3 Desktop-mode routing

Create a specific route group:
• /desktop/\* pages (optional)
• Or feature-flag desktop components based on handshake

2.4 Security constraints for the website
• Website must never receive:
• refresh tokens
• secret values
• raw keychain contents
• Website can only see:
• status, non-sensitive metadata, and explicit user-approved actions

⸻

3. Desktop Bridge (local API) — required

3.1 Transport

Pick one:
• Tauri invoke commands (preferred when UI is bundled content)
• OR localhost HTTP/WS bridge (preferred when UI is remote skills.runwaize.com)

Because you’re embedding a remote site, you’ll likely need:
• localhost HTTPS (or HTTP bound to loopback) + strict origin checks.

3.2 Bridge authentication (critical)

The bridge must not be callable by random local web pages.
Requirements:
• Only accept requests from:
• the Tauri Webview origin that loaded skills.runwaize.com
• Use a handshake token:
• Relay generates desktop_session_token
• Inject token into webview as a JS global at load time
• Bridge requires token on every call
• Additionally verify Origin header == https://skills.runwaize.com

3.3 Bridge API surface (minimal v1)
• GET /bridge/ping → version + capabilities
• GET /bridge/status → relay/mcp state, last sync, cache size
• POST /bridge/refresh → trigger sync
• POST /bridge/cache/clear
• GET /bridge/variables/check/:skill_id → missing vars list (no values)
• POST /bridge/expose → set allowlist for libraries/skills
• GET /bridge/logs/export → downloadable bundle (redacted)
• GET /bridge/update/status → current app version, update available
• POST /bridge/update/apply → start update flow

3.4 Permission prompts

For any sensitive local action, require explicit confirmation via native dialog:
• Clear cache
• Export logs
• Change exposed libraries
• Apply update

⸻

4. Authentication / session model (website + relay)

4.1 Two sessions, two purposes
• Web session: user logged into skills.runwaize.com normally (cookies)
• Relay session: relay authenticated via device flow (tokens in keychain)

They are not interchangeable.

4.2 Linking them (UX)

Add a “Connect Desktop App” flow:
• Website shows a one-click button: “Connect Relay”
• Desktop app opens and completes device flow
• Website polls backend for “device connected” status (via device id)

Result: user sees the relay as a device under their account.

⸻

5. App update flow (Tauri updater)

5.1 Update channels

Support channels:
• stable (default)
• beta (opt-in)
• nightly (internal)

5.2 Update mechanics
• Use Tauri updater:
• signed update manifests
• downloadable installers
• Requirements:
• show update available in:
• desktop shell UI
• embedded website (via bridge update status)
• “Update now” triggers:
• download
• verification
• restart/apply

5.3 Backend requirements for updates
• Host update artifacts + manifest on Runwaize-controlled infra
• Per-platform artifacts:
• macOS: .app or .dmg/.pkg + signatures/notarization later
• Windows: .msi/.exe
• Linux: AppImage (v1), deb/rpm later
• Maintain release notes URL + version metadata

⸻

6. Deployment + operational concerns

6.1 CORS / CSP

Because you’re embedding a remote site:
• Configure CSP to allow calls to http://127.0.0.1:<port>
• Configure CORS on the bridge to allow only https://skills.runwaize.com
• Lock down connect-src to only required endpoints

6.2 Offline mode
• Website may be unreachable: app still runs relay
• Embedded webview should show:
• “Offline / limited mode” page
• Local status from bridge still visible

6.3 Telemetry (privacy-safe)
• Relay emits:
• counts (skills served, cache hits)
• failures (verification failed)
• No secrets, no payloads.

⸻

7. Feature flags (additions)
   • desktop.enabled
   • desktop.bridge_enabled
   • desktop.update_enabled
   • desktop.desktop_pages_enabled
   • desktop.device_linking_enabled

⸻

8. Acceptance criteria (v1)
   • Desktop app opens with embedded skills.runwaize.com and user can manage libraries normally
   • Website detects desktop mode and shows desktop-only UI panels
   • Desktop-only actions work via bridge:
   • show relay status
   • expose a library to agent
   • refresh cache
   • check missing variables
   • export logs (redacted)
   • Update flow works end-to-end:
   • website shows “update available”
   • clicking update triggers Tauri updater and applies update safely

⸻
