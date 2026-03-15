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
  status: string;
  version: string;
  tags: string[];
  description: string | null;
}

export interface SkillFileEntry {
  relative_path: string;
  name: string;
  is_dir: boolean;
  size: number;
  extension: string | null;
}

export interface SkillMeta {
  name: string;
  status: string;
  version: string;
  tags: string[];
  description: string | null;
  updated_at: string;
  synced_at: string | null;
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
  { name: 'example-skill-1', has_skill_md: true, path: `${DEFAULT_SETTINGS_DIR}/chef/skills/example-skill-1`, status: 'draft', version: '0.1.0', tags: ['demo'], description: 'An example skill' },
  { name: 'example-skill-2', has_skill_md: false, path: `${DEFAULT_SETTINGS_DIR}/chef/skills/example-skill-2`, status: 'published', version: '1.0.0', tags: [], description: null },
];

const mockFileTree: SkillFileEntry[] = [
  { relative_path: 'SKILL.md', name: 'SKILL.md', is_dir: false, size: 1024, extension: 'md' },
  { relative_path: 'references', name: 'references', is_dir: true, size: 0, extension: null },
  { relative_path: 'references/guide.md', name: 'guide.md', is_dir: false, size: 512, extension: 'md' },
  { relative_path: 'examples', name: 'examples', is_dir: true, size: 0, extension: null },
  { relative_path: 'examples/basic.py', name: 'basic.py', is_dir: false, size: 256, extension: 'py' },
];

const mockMeta: SkillMeta = {
  name: 'example-skill-1',
  status: 'draft',
  version: '0.1.0',
  tags: ['demo'],
  description: 'An example skill',
  updated_at: new Date().toISOString(),
  synced_at: null,
};

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

const TEXT_EXTENSIONS = new Set([
  'md', 'txt', 'json', 'yaml', 'yml', 'toml', 'env',
  'py', 'js', 'ts', 'rs', 'sh', 'css', 'html', 'xml', 'csv',
]);

export function isTextFile(ext: string | null): boolean {
  return ext !== null && TEXT_EXTENSIONS.has(ext.toLowerCase());
}

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

  // Skill file tree & metadata
  listSkillFiles: (name: string) =>
    isTauri
      ? safeInvoke<SkillFileEntry[]>('list_skill_files', { name })
      : Promise.resolve(mockFileTree),
  readSkillFile: (name: string, relativePath: string) =>
    isTauri
      ? safeInvoke<string>('read_skill_file', { name, relativePath })
      : Promise.resolve(`# Mock content for ${relativePath}`),
  writeSkillFile: (name: string, relativePath: string, content: string) =>
    safeInvoke<void>('write_skill_file', { name, relativePath, content }),
  createSkillFile: (name: string, relativePath: string, content: string) =>
    safeInvoke<void>('create_skill_file', { name, relativePath, content }),
  createSkillFolder: (name: string, relativePath: string) =>
    safeInvoke<void>('create_skill_folder', { name, relativePath }),
  deleteSkillFile: (name: string, relativePath: string) =>
    safeInvoke<void>('delete_skill_file', { name, relativePath }),
  renameSkillFile: (name: string, oldPath: string, newPath: string) =>
    safeInvoke<void>('rename_skill_file', { name, oldPath, newPath }),
  getSkillMeta: (name: string) =>
    isTauri
      ? safeInvoke<SkillMeta>('get_skill_meta', { name })
      : Promise.resolve({ ...mockMeta, name }),
  updateSkillMeta: (name: string, status: string, version: string, tags: string[], description: string | null) =>
    safeInvoke<SkillMeta>('update_skill_meta', { name, status, version, tags, description }),

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
