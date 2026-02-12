# Skills Delivery Pipeline Testing

End-to-end testing for the skills delivery flow: upload → curate → package → retrieve.

## Overview

Tests cover two components:

1. **Skills Cookbook (Relay)** — Rust unit/integration tests
2. **Studio Backend** — Python pytest (relay API, E2E delivery pipeline)

## Running Tests

### Relay (Skills Cookbook)

```bash
cd skills_cookbook

# Quick test
just test

# Full cycle (format, lint, test)
just test-delivery

# Verbose output
just test-verbose
```

### Studio Backend (app.supervaize.com)

```bash
cd studio

# Skills delivery pipeline E2E + relay API tests
just test-skills-delivery

# All skills studio tests
just test-skills-studio

# Full test suite (excluding e2e)
just test
```

## E2E Test Scenarios

| Scenario | Description |
|----------|-------------|
| Library discovery | GET /libraries with Bearer token returns collections |
| Skills list | GET /skills?library_id= returns skills in library |
| Latest approved | GET /skills/{id}/latest-approved returns artifact pointer |
| Artifact metadata | GET /artifacts/{id}/metadata returns metadata (no payload) |
| Artifact signature | GET /artifacts/{id}/signature returns Ed25519 signature |
| Artifact payload | GET /artifacts/{id}/payload returns skill content |
| Updates feed | GET /updates/since returns cursor and updates list |
| Unauthorized | All relay endpoints return 401 without token |

## API Compatibility

The relay expects these response shapes from the RSS API:

- `Library`: `library_id` (alias: `id`), `visibility` (alias: `scope`)
- `SkillSummary`: `skill_id` (alias: `id`)
- `UpdatesSinceResponse`: `updates` (alias: `changes`)

See [src/types.rs](../src/types.rs) for relay type definitions.

## Fixtures

Studio conftest provides: `skill`, `skill_version`, `artifact`, `collection`, `collection_item`, `library_release`, `device_code`, `relay_token`, `signing_key`.
