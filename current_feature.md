# Current Feature: Unified Config + Editable Settings + Dual-Role UI

**Branch**: `feature/cookbook-cli`
**Date**: 2026-03-05

## What Changed

### 1. Unified Config Directory

All app data is now under a single directory: `~/.runwaize_skills_cookbook/`

```
~/.runwaize_skills_cookbook/
├── config.toml      # App settings (ports, directories, role, UI mode)
├── device_id        # Unique device identifier
├── webview/         # Webview persistent storage
├── chef/            # Creator workspace
│   └── skills/      # Skill folders (each with SKILL.md)
└── cook/            # Consumer data
    └── manifest.json  # Synced from server
```

**Previous**: Config was at `~/Library/Application Support/skill-cookbook-relay/`, chef at `~/.runwaize_skills_cookbook_chef/`, guest at `~/.runwaize_skills_cookbook_guest/`.

### 2. Editable Settings Page

The Settings page (`/settings`) now has editable fields:

- **Settings folder** — where all config lives. Changing it copies existing files to the new location and writes a redirect pointer at the default path.
- **Chef directory** — path to the creator workspace.
- **Cook directory** — path to the consumer manifest folder.
- **MCP Server port** — default 9876.
- **Bridge port** — default 9123.
- Each path field has a **folder picker** button (native dialog, Tauri only).
- **Save Settings** button persists changes to `config.toml`.

### 3. Dual-Role Navigation

Chef and Cook sidebar sections are **always visible** — no more exclusive role selection. The sidebar layout:

- **Dashboard** (top-level, outside groups)
- **Chef**: Local Skills, Remote Skills, Discover, Sync
- **Cook**: Local Skills, Sync
- **System**: Relay Status, Doctor, Settings

### 4. Combined Dashboard

The Dashboard at `/` shows:
- Chef skills count (from `chef_dir/skills/`)
- Cook skills count (from manifest)
- Library count (from server)
- MCP Server status
- Quick-nav cards to Chef Local Skills and Cook Local Skills

### 5. "Guest" Renamed to "Cook"

All UI labels and TypeScript types use "Cook" instead of "Guest". The Rust `Config` still uses `guest_dir` internally for backward compatibility.

### 6. Browser Dev Mode

When running `npm run dev` in a browser (without Tauri), the app shows mock data instead of crashing. The `isTauri` flag detects the runtime environment.

## How to Test

### Prerequisites

```bash
cd /Volumes/SSDext1TB/Documents/GitRepo/SUPERVAIZE/skills_cookbook

# Install local-ui dependencies (if not done)
just ui-install
```

### Option A: Full Tauri App (recommended)

```bash
just dev
```

This opens the Tauri webview with full access to the filesystem. All Tauri commands work — you'll see real skill data.

If `just dev` fails due to path issues (symlinked dirs), use two terminals:

**Terminal 1** — start the UI dev server:
```bash
cd local-ui && npm run dev
```

**Terminal 2** — start the Rust backend:
```bash
cargo run --no-default-features
```

### Option B: Browser-only (UI layout testing)

```bash
cd local-ui && npm run dev
# Open http://localhost:5173
```

You'll see mock data (2 example skills). Tauri commands (save settings, file picker, sync) will show errors — this is expected.

### What to Verify

#### Dashboard (`/`)
- [ ] Shows Chef skill count (should be 33 if `~/.runwaize_skills_cookbook/chef/skills/` is populated)
- [ ] Shows Cook skill count (0 until synced)
- [ ] Shows library count and MCP status
- [ ] Quick-nav cards navigate to `/chef/skills` and `/cook/skills`

#### Chef > Local Skills (`/chef/skills`)
- [ ] Lists all skill folders from `~/.runwaize_skills_cookbook/chef/skills/`
- [ ] Each skill shows name, path, and a "Missing SKILL.md" badge if applicable
- [ ] Edit button navigates to the inline editor
- [ ] External link button opens in system editor

#### Cook > Local Skills (`/cook/skills`)
- [ ] Shows "No skills in manifest" if cook dir is empty
- [ ] "Sync from Server" button triggers manifest sync (requires auth)
- [ ] After sync, skills appear with name, description, library badge

#### Settings (`/settings`)
- [ ] All fields pre-populated with current values
- [ ] Settings folder shows `~/.runwaize_skills_cookbook` by default
- [ ] Folder picker buttons open native dialogs (Tauri only)
- [ ] Changing a value and clicking Save persists to `config.toml`
- [ ] Moving the settings folder copies files to new location

#### Sidebar Navigation
- [ ] Dashboard is at the top, outside Chef/Cook/System groups
- [ ] Both Chef and Cook sections are always visible
- [ ] Active page is highlighted in the sidebar

### Automated Checks

```bash
# Rust tests (relay + CLI)
just test
# Expected: all pass

# CLI tests only
just cli-test
# Expected: all pass

# TypeScript type check
cd local-ui && npx tsc --noEmit
# Expected: no errors
```

## Files Modified (16 files)

| File | Change |
|------|--------|
| `src/config.rs` | Unified base dir (`~/.runwaize_skills_cookbook/`), public `base_dir()`, redirect file support |
| `src/config_tests.rs` | Updated path assertions for new base dir |
| `src/main.rs` | Added `pick_folder`, `update_config`, `move_settings_dir` commands |
| `src/types.rs` | Added `settings_dir`, renamed `guest_dir` → `cook_dir` in `AppConfig` |
| `cli/src/init.rs` | Updated default chef/cook paths |
| `cli/src/cli_tests.rs` | Removed old `skill-cookbook-relay` subdir creation |
| `local-ui/src/lib/tauri.ts` | `isTauri` detection, `safeInvoke`, mock data, new command wrappers |
| `local-ui/src/App.tsx` | Updated routes (dashboard at `/`, cook skills) |
| `local-ui/src/components/AppSidebar.tsx` | Both roles always visible, dashboard top-level |
| `local-ui/src/components/AppHeader.tsx` | Updated page titles, removed role badge |
| `local-ui/src/contexts/RoleContext.tsx` | Simplified (removed role selection logic) |
| `local-ui/src/pages/RoleSelectPage.tsx` | Redirects to `/` (no longer needed) |
| `local-ui/src/pages/shared/DashboardPage.tsx` | New combined dashboard |
| `local-ui/src/pages/shared/SettingsPage.tsx` | Editable form with folder pickers |
| `local-ui/src/pages/cook/ManifestView.tsx` | Added Sync button in empty state and header |
| `tauri.conf.json` | Changed `beforeDevCommand` to use `npm --prefix` |
