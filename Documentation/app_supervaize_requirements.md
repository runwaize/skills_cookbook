# app.supervaize.com Backend Requirements for Skills Delivery Pipeline

**Status:** Implemented (Feb 2026)

This document describes backend changes required for app.supervaize.com (Skills Studio) to support end-to-end skills delivery pipeline testing with the Skills Cookbook Relay.

---

## Summary of Implemented Changes

The following have been implemented in the studio backend:

### 1. Artifact Signature Endpoint

**Requirement:** Relay needs a dedicated `/artifacts/{id}/signature` endpoint to fetch Ed25519 signature for verification.

**Implementation:**
- `ArtifactSignatureView` in `apps/skills_studio/views/relay_api_views.py`
- URL: `GET /api/rss/v1/artifacts/<id>/signature/`
- Response: `{ "signature": "", "algorithm": "Ed25519", "key_id": "", "issued_at": "", "expires_at": null }`
- Auth: Bearer token required

### 2. Updates Feed Response Format

**Requirement:** Relay expects `updates` key (not `changes`).

**Implementation:**
- `UpdatesSinceView` returns `{"cursor": "...", "updates": []}`
- Alias: Relay types also accept `changes` via serde for backward compatibility

### 3. Future: Delta Feed Implementation

**Requirement:** `GET /updates/since?cursor=...` should return delta of artifact changes since cursor.

**Current:** Returns empty `updates` list.

**Future work:**
- Use `AuditEvent` model or `updated_at` ranges to compute delta
- Return `ArtifactUpdate` objects: `{ "artifact_id": "", "update_type": "new_version"|"revoked"|"metadata_changed", "timestamp": "" }`
- Relay will process `NewVersion` and `Revoked` to update cache

### 4. Response Field Aliases (Relay-Side)

Relay uses serde aliases to accept:
- `id` → `library_id` (libraries, skills)
- `scope` → `visibility` (libraries)
- `latest_approved_release_id` → `latest_approved_release`
- `changes` → `updates` (updates feed)

No backend changes needed for these; relay handles both shapes.

---

## API Contract Summary

| Endpoint | Method | Auth | Notes |
|----------|--------|------|-------|
| `/api/rss/v1/libraries/` | GET | Bearer | Returns `[{ "id", "name", "description", "scope" }]` |
| `/api/rss/v1/libraries/{id}/` | GET | Bearer | Returns `{ "id", "name", "latest_approved_release_id" }` |
| `/api/rss/v1/skills/?library_id=` | GET | Bearer | Returns `[{ "id", "name", "slug", "status" }]` |
| `/api/rss/v1/skills/{id}/latest-approved/` | GET | Bearer | Returns `{ "artifact_id", "skill_id", "skill_version_id", "content_hash", "artifact_type" }` |
| `/api/rss/v1/artifacts/{id}/metadata/` | GET | Bearer | Metadata only; no payload |
| `/api/rss/v1/artifacts/{id}/payload/` | GET | Bearer | JSON payload |
| `/api/rss/v1/artifacts/{id}/signature/` | GET | Bearer | **New** — signature for verification |
| `/api/rss/v1/signing/jwks/` | GET | None | Public keys |
| `/api/rss/v1/updates/since/?cursor=` | GET | Bearer | Returns `{ "cursor", "updates": [] }` |

---

## Device Flow Auth

- `POST /api/rss/v1/auth/device/` — Initiate (requires auth)
- `POST /api/rss/v1/auth/device/claim/` — Claim user code (requires auth)
- `POST /api/rss/v1/auth/device/token/` — Exchange for tokens (no auth)
- `POST /api/rss/v1/auth/token/refresh/` — Refresh access token

---

## Test Fixtures

E2E tests use:
- `skill`, `skill_version`, `artifact`, `collection`, `collection_item`, `library_release`, `device_code`, `relay_token`, `signing_key`

Run: `just test-skills-delivery` or `pytest apps/skills_studio/tests/test_e2e_delivery_pipeline.py apps/skills_studio/tests/test_relay_api.py -v`
