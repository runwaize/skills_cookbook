Tauri Client — Bridge Detection via Query Parameter

Context

The Skills Cookbook web app (skills.runwaize.com) now gates all local relay communication (/bridge/ping, /bridge/handshake, etc.) behind a tauri_version
query parameter. If the parameter is absent, the app assumes it's running in a regular browser and never attempts to reach 127.0.0.1:9123.

Requirements

1. Append tauri_version query parameter on app launch

When the Tauri WebView loads the Skills Cookbook URL, append ?tauri_version=<version> to the URL:

https://skills.runwaize.com/?tauri_version=0.1.0

- The version value should come from the Tauri app's own version (e.g. from tauri.conf.json > version).
- If the URL already has query parameters, append with &tauri_version=... instead.

2. Preserve the parameter on initial load only

- The parameter only needs to be present on the initial page load. The web app persists it in sessionStorage for the lifetime of the browser tab.
- You do not need to inject it into subsequent navigations or deep links within the SPA.

3. Deep links / external redirects

- If the app navigates away (e.g. OAuth flow to app.supervaize.com) and returns, the sessionStorage value survives as long as the tab isn't closed.
- If the Tauri app opens a new WebView window, that window needs ?tauri_version= appended again.

4. No other changes required

- The local relay (127.0.0.1:9123) behavior is unchanged — it still needs to serve /bridge/ping, /bridge/handshake, /bridge/deploy as before.
- No changes to CORS, headers, or bridge token logic.

How the web app uses it

┌──────────────────────────────┬─────────────────────┬───────────────────────┬─────────────────────────┐
│ Scenario │ tauri_version param │ Bridge ping attempted │ Desktop UI shown │
├──────────────────────────────┼─────────────────────┼───────────────────────┼─────────────────────────┤
│ Tauri app loads the URL │ Present │ Yes │ Yes (if relay responds) │
├──────────────────────────────┼─────────────────────┼───────────────────────┼─────────────────────────┤
│ Regular browser │ Absent │ No │ No │
├──────────────────────────────┼─────────────────────┼───────────────────────┼─────────────────────────┤
│ Tauri app, relay not running │ Present │ Yes (times out) │ No │
└──────────────────────────────┴─────────────────────┴───────────────────────┴─────────────────────────┘

Acceptance criteria

- Tauri WebView URL includes ?tauri_version=X.Y.Z on startup
- Version value matches the app version from tauri.conf.json
- The "SETUP" button, "Deploy locally" buttons, and Desktop Panel appear when running from Tauri
- Opening skills.runwaize.com in Chrome/Safari shows no bridge-related network requests in DevTools
