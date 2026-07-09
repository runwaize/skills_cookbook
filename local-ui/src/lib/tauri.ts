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
  managed_client_ids: string[];
  /** Which CLI to use for AI-powered security review of imported skills ("claude_code" or "codex"); null until chosen in Settings. */
  default_review_cli: string | null;
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
  active: boolean;
  targets: string[];
  /** Provenance, e.g. "created", "downloaded", "adapted", "imported", "existing". Free-text. */
  source: string;
  /** RFC3339 timestamp — when this skill was first tracked (installed date). */
  created_at: string;
  /** RFC3339 timestamp — last metadata change. */
  updated_at: string;
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
  source: string;
  created_at: string;
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
  /** Repo-relative path for display (GitHub scans only) — never show `path` (an
   *  absolute local tempdir path) directly to the user when this is present. */
  display_path?: string | null;
}

export interface SecurityReview {
  /** One of "safe", "concerns", "unsafe", "inconclusive". */
  verdict: string;
  summary: string;
  raw_output: string;
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

export interface AdoptCandidate {
  name: string;
  path: string;
}

export interface ProjectSkillEntry {
  name: string;
  targets: string[];
}

export interface UsageEvent {
  skill: string;
  date: string;
  client: string;
  /** Working directory / project folder the transcript session ran in. Empty string if unknown. */
  context: string;
  /** How many real invocations this one event (transcript file/session) represents. */
  count: number;
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
  managed_client_ids: ['claude_code', 'cursor', 'windsurf'],
  default_review_cli: null,
};

// ---- Mock data for browser dev mode ----

const daysAgo = (n: number) => new Date(Date.now() - n * 24 * 60 * 60 * 1000).toISOString();

const mockSkills: LocalSkill[] = [
  { name: 'example-skill-1', has_skill_md: true, path: `${DEFAULT_SETTINGS_DIR}/chef/skills/example-skill-1`, status: 'draft', version: '0.1.0', tags: ['demo'], description: 'An example skill that demonstrates the basic structure', active: true, targets: ['claude_code'], source: 'adapted', created_at: daysAgo(12), updated_at: daysAgo(2) },
  { name: 'example-skill-2', has_skill_md: false, path: `${DEFAULT_SETTINGS_DIR}/chef/skills/example-skill-2`, status: 'published', version: '1.0.0', tags: [], description: null, active: false, targets: [], source: 'downloaded', created_at: daysAgo(30), updated_at: daysAgo(30) },
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
  source: 'created',
  created_at: daysAgo(12),
  updated_at: daysAgo(2),
  synced_at: null,
};

const mockManifest: GuestManifest = { skills: [] };

const mockDiscoveredClients: DiscoveredClient[] = [
  { id: 'claude_code', name: 'Claude Code', detected: true, path: '~/.claude' },
  { id: 'codex', name: 'Codex', detected: true, path: '~/.codex' },
  { id: 'cursor', name: 'Cursor', detected: false, path: null },
];

const dateNDaysAgo = (n: number) =>
  new Date(Date.now() - n * 24 * 60 * 60 * 1000).toISOString().slice(0, 10);

const REPO_A = '/Users/alp/Documents/GitRepo/RUNWAIZE/skills_cookbook';
const REPO_B = '/Users/alp/Documents/GitRepo/RUNWAIZE/studio';
const REPO_C = '/Users/alp/Documents/GitRepo/RUNWAIZE/www_runwaize';
const REPO_D = '/Users/alp/Documents/GitRepo/RUNWAIZE/supervaizer';

const mockUsageEvents: UsageEvent[] = [
  { skill: 'example-skill-1', date: dateNDaysAgo(0), client: 'claude_code', context: REPO_A, count: 3 },
  { skill: 'example-skill-1', date: dateNDaysAgo(0), client: 'codex', context: REPO_A, count: 1 },
  { skill: 'example-skill-1', date: dateNDaysAgo(1), client: 'claude_code', context: REPO_B, count: 1 },
  { skill: 'example-skill-1', date: dateNDaysAgo(2), client: 'claude_code', context: REPO_A, count: 1 },
  { skill: 'example-skill-1', date: dateNDaysAgo(4), client: 'codex', context: REPO_B, count: 1 },
  { skill: 'example-skill-1', date: dateNDaysAgo(7), client: 'claude_code', context: '', count: 1 },
  { skill: 'example-skill-2', date: dateNDaysAgo(1), client: 'codex', context: REPO_C, count: 1 },
  { skill: 'example-skill-2', date: dateNDaysAgo(3), client: 'claude_code', context: REPO_C, count: 1 },
  { skill: 'example-skill-2', date: dateNDaysAgo(5), client: 'codex', context: REPO_A, count: 1 },
  { skill: 'deploy_www', date: dateNDaysAgo(2), client: 'claude_code', context: REPO_C, count: 1 },
  { skill: 'deploy_www', date: dateNDaysAgo(2), client: 'codex', context: REPO_C, count: 1 },
  { skill: 'deploy_www', date: dateNDaysAgo(6), client: 'claude_code', context: REPO_C, count: 1 },
  { skill: 'morning-routine', date: dateNDaysAgo(0), client: 'claude_code', context: REPO_D, count: 1 },
  { skill: 'morning-routine', date: dateNDaysAgo(9), client: 'claude_code', context: REPO_A, count: 1 },
  { skill: 'morning-routine', date: dateNDaysAgo(12), client: 'claude_code', context: REPO_D, count: 1 },
  { skill: 'ponytail', date: dateNDaysAgo(3), client: 'codex', context: REPO_A, count: 1 },
];

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
  setManagedClients: (nextIds: string[]) => safeInvoke<void>('set_managed_clients', { ids: nextIds }),
  setDefaultReviewCli: (cli: string | null) => safeInvoke<void>('set_default_review_cli', { cli }),

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
  activateSkill: (name: string) => safeInvoke<void>('activate_skill', { name }),
  deactivateSkill: (name: string) => safeInvoke<void>('deactivate_skill', { name }),
  deleteSkill: (name: string) => safeInvoke<void>('delete_skill', { name }),
  setSkillTargets: (name: string, targets: string[]) =>
    safeInvoke<void>('set_skill_targets', { name, targets }),
  /**
   * Publish a real copy of a skill into a chosen project repo's `.claude/skills/`
   * and/or `.agents/skills/` so cloud/web Claude Code and Codex sessions can find it.
   * Distinct from `setSkillTargets` (machine-global symlink deploy) — never merge these.
   */
  publishSkillToProject: (name: string, projectDir: string, targets: string[]) =>
    safeInvoke<string[]>('publish_skill_to_project', { name, projectDir, targets }),
  /** List every skill currently published into a project repo, with which target(s) each is in. */
  listProjectSkills: (projectDir: string) =>
    safeInvoke<ProjectSkillEntry[]>('list_project_skills', { projectDir }),
  /** Remove a previously-published skill from a project repo for the given target(s). */
  unpublishSkillFromProject: (projectDir: string, name: string, targets: string[]) =>
    safeInvoke<string[]>('unpublish_skill_from_project', { projectDir, name, targets }),
  adoptScan: (clientId: string) => safeInvoke<AdoptCandidate[]>('adopt_scan', { clientId }),
  adoptApply: (clientId: string, names: string[]) =>
    safeInvoke<number>('adopt_apply', { clientId, names }),
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
  updateSkillMeta: (name: string, status: string, version: string, tags: string[], description: string | null, source: string) =>
    safeInvoke<SkillMeta>('update_skill_meta', { name, status, version, tags, description, source }),

  // Discovery
  discoverAgents: () =>
    isTauri
      ? safeInvoke<DiscoveredClient[]>('discover_agents')
      : Promise.resolve(mockDiscoveredClients),
  getSkillUsage: (days: number) =>
    isTauri
      ? safeInvoke<UsageEvent[]>('get_skill_usage', { days })
      : Promise.resolve(
          mockUsageEvents.filter((ev) => ev.date >= dateNDaysAgo(days - 1))
        ),
  resyncSkillUsage: (days: number) =>
    isTauri
      ? safeInvoke<UsageEvent[]>('resync_skill_usage', { days })
      : Promise.resolve(
          mockUsageEvents.filter((ev) => ev.date >= dateNDaysAgo(days - 1))
        ),
  scanForSkills: (path?: string) => safeInvoke<ScannedSkillInfo[]>('scan_for_skills', { path }),
  importSkills: (paths: string[], names?: string[], source?: string) =>
    safeInvoke<number>('import_skills', { paths, names, source }),
  scanGithubRepo: (url: string) => safeInvoke<ScannedSkillInfo[]>('scan_github_repo', { url }),
  githubRepoSlug: (url: string) => safeInvoke<string | null>('github_repo_slug', { url }),
  runSkillSecurityReview: (skillPath: string, cli: string) =>
    safeInvoke<SecurityReview>('run_skill_security_review', { skillPath, cli }),

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
