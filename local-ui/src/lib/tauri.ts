// ---- Types matching Rust structs ----

export type UserRole = 'chef' | 'cook';
export type UiMode = 'local' | 'online';

export interface AppConfig {
  user_role: UserRole;
  ui_mode: UiMode;
  settings_dir: string;
  chef_dir: string;
  cook_dir: string;
  workspace_id: string | null;
  mcp_server_port: number;
  bridge_port: number;
}

export interface RelayStatus {
  connected: boolean;
  authenticated: boolean;
  user_email: string | null;
  workspace_id: string | null;
  last_sync: string | null;
  cache_size_bytes: number;
  artifact_count: number;
  library_count: number;
  guest_skill_count: number;
  mcp_server_running: boolean;
  mcp_server_port: number | null;
}

export interface LocalSkill {
  name: string;
  has_skill_md: boolean;
  path: string;
}

export interface RemoteSkillInfo {
  library_name: string;
  library_id: string;
  skill_name: string;
  skill_id: string;
  description: string | null;
}

export interface SyncResult {
  committed: boolean;
  pushed: boolean;
  skills_pushed: number;
  guest_skills_updated: number;
}

export interface DiagnosticCheck {
  name: string;
  status: 'ok' | 'warning' | 'error';
  message: string;
}

export interface ScannedSkillInfo {
  name: string;
  path: string;
  already_in_chef: boolean;
  source_agent: string;
}

export interface DiscoveredClient {
  id: string;
  name: string;
  detected: boolean;
  path: string | null;
}

export interface GuestSkillEntry {
  skill_id: string;
  name: string;
  description: string | null;
  library_id: string | null;
}

export interface GuestManifest {
  skills: GuestSkillEntry[];
}

export interface AuthStatus {
  authenticated: boolean;
  user_email: string | null;
  workspace_id: string | null;
  expires_at: string | null;
}

// ---- Tauri detection ----

/** True when running inside the Tauri webview (not a regular browser). */
export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * Safe invoke: calls the real Tauri invoke when available, throws a clear
 * error in a browser so callers can catch it gracefully.
 */
async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    throw new Error(`[tauri] not available in browser — command: ${cmd}`);
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

// ---- Mock defaults for browser dev mode ----

const DEFAULT_SETTINGS_DIR = '~/.runwaize_skills_cookbook';

const mockConfig: AppConfig = {
  user_role: 'cook',
  ui_mode: 'local',
  settings_dir: DEFAULT_SETTINGS_DIR,
  chef_dir: `${DEFAULT_SETTINGS_DIR}/chef`,
  cook_dir: `${DEFAULT_SETTINGS_DIR}/cook`,
  workspace_id: null,
  mcp_server_port: 9876,
  bridge_port: 9123,
};

// ---- Mock data for browser dev mode ----

const mockSkills: LocalSkill[] = [
  { name: 'example-skill-1', has_skill_md: true, path: `${DEFAULT_SETTINGS_DIR}/chef/skills/example-skill-1` },
  { name: 'example-skill-2', has_skill_md: false, path: `${DEFAULT_SETTINGS_DIR}/chef/skills/example-skill-2` },
];

const mockManifest: GuestManifest = { skills: [] };

const mockRelayStatus: RelayStatus = {
  connected: false,
  authenticated: false,
  user_email: null,
  workspace_id: null,
  last_sync: null,
  cache_size_bytes: 0,
  artifact_count: 0,
  library_count: 0,
  guest_skill_count: 0,
  mcp_server_running: false,
  mcp_server_port: 9876,
};

// ---- Tauri command wrappers ----

export const commands = {
  pickFolder: (title?: string) => safeInvoke<string | null>('pick_folder', { title }),
  getConfig: () =>
    isTauri
      ? safeInvoke<AppConfig>('get_config')
      : Promise.resolve(mockConfig),
  updateConfig: (opts: {
    chefDir?: string;
    cookDir?: string;
    mcpServerPort?: number;
    bridgePort?: number;
  }) => safeInvoke<AppConfig>('update_config', {
    chefDir: opts.chefDir,
    cookDir: opts.cookDir,
    mcpServerPort: opts.mcpServerPort,
    bridgePort: opts.bridgePort,
  }),
  moveSettingsDir: (newDir: string) => safeInvoke<AppConfig>('move_settings_dir', { newDir }),
  setUserRole: (role: UserRole) => safeInvoke<void>('set_user_role', { role }),
  switchUiMode: (mode: UiMode) => safeInvoke<void>('switch_ui_mode', { mode }),

  // Auth
  getRelayStatus: () =>
    isTauri
      ? safeInvoke<RelayStatus>('get_relay_status')
      : Promise.resolve(mockRelayStatus),
  login: () => safeInvoke<AuthStatus>('login_to_rss'),
  logout: () => safeInvoke<void>('logout_from_rss'),

  // Chef
  listLocalSkills: () =>
    isTauri
      ? safeInvoke<LocalSkill[]>('list_local_skills')
      : Promise.resolve(mockSkills),
  listRemoteSkills: () => safeInvoke<RemoteSkillInfo[]>('list_remote_skills'),
  addSkill: (path: string) => safeInvoke<void>('add_skill', { path }),
  readSkillContent: (name: string) => safeInvoke<string>('read_skill_content', { name }),
  writeSkillContent: (name: string, content: string) =>
    safeInvoke<void>('write_skill_content', { name, content }),
  openSkillInEditor: (name: string) => safeInvoke<void>('open_skill_in_editor', { name }),
  syncChef: (noPush: boolean) => safeInvoke<SyncResult>('sync_chef', { noPush }),

  // Discovery
  discoverAgents: () => safeInvoke<DiscoveredClient[]>('discover_agents'),
  scanForSkills: (path?: string) => safeInvoke<ScannedSkillInfo[]>('scan_for_skills', { path }),
  importSkills: (paths: string[]) => safeInvoke<number>('import_skills', { paths }),

  // Cook
  getGuestManifest: () =>
    isTauri
      ? safeInvoke<GuestManifest>('get_guest_manifest')
      : Promise.resolve(mockManifest),
  syncGuest: () => safeInvoke<number>('sync_guest'),

  // Shared
  runDoctor: () => safeInvoke<DiagnosticCheck[]>('run_doctor'),
  initCookbook: (chefDir?: string, guestDir?: string) =>
    safeInvoke<void>('init_cookbook', { chefDir, guestDir }),

  // Existing
  refreshSkills: () => safeInvoke<void>('refresh_skills'),
  clearCache: () => safeInvoke<void>('clear_cache'),
  listLibraries: () => safeInvoke<unknown[]>('list_libraries'),
};
