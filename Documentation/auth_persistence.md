# Tauri Skills Relay — Auth Persistence Requirements (Option A)

| Document | Skills Studio / Tauri Relay                                                  |
| -------- | ---------------------------------------------------------------------------- |
| Project  | RUNWAIZE                                                                     |
| Date     | 2026-02                                                                      |
| Status   | Requirements for Tauri app                                                   |
| Approach | Option A — Persistent webview storage                                        |
| Related  | [tauri_skills_relay_auto_discovery.md](tauri_skills_relay_auto_discovery.md) |

This document specifies what the Tauri desktop app (Skills Cookbook Relay) must implement so that user authentication and workspace selection persist across app restarts. We use **Option A**: persistent webview storage.

---

## 1. Problem

When the Skills Cookbook runs inside the Tauri desktop app:

- **Auth is lost on restart**: Users must re-login every time they restart the app.
- **Workspace is not assigned**: After login from desktop, the workspace shows "No workspace" (browser login works correctly).

The root cause is that session cookies and/or webview storage do not persist across app restarts in the current Tauri configuration.

---

## 2. Solution: Persistent Webview Storage (Option A)

The Skills Studio frontend uses **session tokens** (django-allauth app client) when running in Tauri. Session tokens are stored in **localStorage** and sent via the `X-Session-Token` header. For this to work, the Tauri webview must use **persistent storage** so `localStorage` survives app restarts.

---

## 3. Tauri Requirements

### 3.1. Persistent User Data Directory

**Requirement**: Configure the webview to use a **persistent user data directory** tied to the app installation, not a temporary or in-memory profile.

**Tauri 2.x (Rust)**:

- Use `WebviewWindow::with_webview()` or equivalent to set a custom `user_data_dir` / `data_directory` for the webview.
- Ensure the directory is in a persistent location (e.g. `$HOME/.config/skills-cookbook-relay/` or `%APPDATA%/SkillsCookbookRelay/`).

**Tauri 1.x**:

- Check `tauri.conf.json` for `webview` / `dataPath` or similar.
- Ensure the webview is not configured to use a temporary or ephemeral data directory.

**Verification**: After logging in, close the app completely and reopen. The user should remain logged in without re-entering credentials.

---

### 3.2. Loading URL with `tauri_version` Parameter

**Requirement (already expected):** When loading the Skills Cookbook SPA, the Tauri app must append `?tauri_version=x.y.z` to the URL so the frontend can detect it is running inside Tauri.

Example: `https://skills.runwaize.com/?tauri_version=1.0.0`

This is used by `isTauriSession()` in `desktopBridge.ts` to switch to session-token auth and localStorage.

---

### 3.3. CORS / Origin

No Tauri changes required. The session token is sent in the `X-Session-Token` header. The Skills Studio backend accepts this header for authentication. CORS allows the header from the Tauri webview origin.

**CSRF in dev-local**: WKWebView sends `Origin: null` for embedded webview requests. The Django backend must include `"null"` in `CSRF_TRUSTED_ORIGINS` or login will fail with "CSRF verification failed". See [troubleshooting.md](troubleshooting.md).

---

## 4. Summary

| Requirement                             | Status       |
| --------------------------------------- | ------------ |
| Persistent webview storage              | **Required** |
| `tauri_version` query param on load URL | **Required** |

No bridge endpoints for session token storage are needed. The frontend uses `localStorage` directly; persistence is ensured by the webview configuration.

---

## 5. Dependencies

- **Skills Studio frontend** (main.tsx, allauth.js, skillsStudio.ts): Use app client + session tokens when `isTauriSession()`.
- **Skills Studio backend**: `XSessionTokenAuthentication` in DRF, `x-session-token` in CORS allowed headers.

These are implemented in the Studio repository; this document covers only the Tauri app requirements.
