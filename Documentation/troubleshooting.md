# Troubleshooting

## CSRF verification failed (dev-local)

**Symptom**: Login works in Chrome at `http://localhost:5175`, but fails with "CSRF verification failed" when using `just dev-local` (Tauri app loading the same local frontend). The failing request goes to `/_allauth/browser/v1/auth/login`.

**Cause**: Two factors:
1. WKWebView (Tauri on macOS) sends `Origin: null` for requests from the embedded webview; Chrome sends `http://localhost:5175`.
2. When `isTauriSession()` is false (e.g. `tauri_version` not in URL or sessionStorage cleared), the frontend uses the BROWSER client (`/_allauth/browser/`) instead of the APP client (`/_allauth/app/`). Only `/_allauth/app/` was exempt from CSRF.

**Fix** (in Skills Studio Django backend):
1. Add `"null"` to `CSRF_TRUSTED_ORIGINS` in `studio/supervaize/settings.py` (`_DEV_CSRF_ORIGINS`).
2. `AllauthAppCsrfExemptMiddleware` in `studio/sv_core/middleware.py` exempts `/_allauth/browser/` when `Origin: null` (Tauri webview fallback).

**Verification**: After the fix, `just dev-local` with the local frontend and Django backend running should allow login without CSRF errors.

**Note**: The relay app itself does not perform CSRF checks; the error comes from the Django backend when the webview makes auth/API requests.
