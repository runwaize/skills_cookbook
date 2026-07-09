import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Search, Download, Monitor, Sparkles, Github, ShieldAlert, Loader2 } from 'lucide-react';
import {
  commands,
  type DiscoveredClient,
  type ScannedSkillInfo,
  type AdoptCandidate,
  type SecurityReview,
} from '@/lib/tauri';
import { useRole } from '@/contexts/RoleContext';
import { managedAndDetected } from '@/lib/clients';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';

export function SearchDiscover() {
  const { config } = useRole();
  const [agents, setAgents] = useState<DiscoveredClient[]>([]);
  const [skills, setSkills] = useState<ScannedSkillInfo[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [scanning, setScanning] = useState(false);
  const [importing, setImporting] = useState(false);
  const [customPath, setCustomPath] = useState('');
  const [loadingAgents, setLoadingAgents] = useState(true);
  const [adoptClient, setAdoptClient] = useState<DiscoveredClient | null>(null);

  useEffect(() => {
    commands
      .discoverAgents()
      .then(setAgents)
      .catch(() => {})
      .finally(() => setLoadingAgents(false));
  }, []);

  const scan = async (path?: string) => {
    setScanning(true);
    try {
      const found = await commands.scanForSkills(path);
      setSkills(found);
      // Pre-select skills not already in chef
      const preselect = new Set<string>();
      for (const s of found) {
        if (!s.already_in_chef) preselect.add(s.path);
      }
      setSelected(preselect);
    } catch (e) {
      console.error('Scan failed:', e);
    } finally {
      setScanning(false);
    }
  };

  const toggleSkill = (path: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const importSelected = async () => {
    setImporting(true);
    try {
      const paths = Array.from(selected);
      const names = paths.map((p) => skills.find((s) => s.path === p)?.name ?? '');
      await commands.importSkills(paths, names);
      setSkills((prev) =>
        prev.map((s) =>
          selected.has(s.path) ? { ...s, already_in_chef: true } : s,
        ),
      );
      setSelected(new Set());
    } catch (e) {
      console.error('Import failed:', e);
    } finally {
      setImporting(false);
    }
  };

  const detectedAgents = managedAndDetected(agents, config?.managed_client_ids ?? []);

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">
          Import from a folder
        </h2>
        <p className="text-xs text-muted-foreground mb-3">
          Recursively scan a folder for SKILL.md files anywhere inside it and copy selected ones into your cookbook.
          The originals are left untouched.
        </p>

        {/* Detected Agents */}
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Monitor className="h-4 w-4" />
              Detected AI Agents
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loadingAgents ? (
              <div className="space-y-2">
                {[1, 2, 3].map((i) => (
                  <Skeleton key={i} className="h-8 w-full" />
                ))}
              </div>
            ) : (
              <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
                {agents.map((agent) => (
                  <div
                    key={agent.id}
                    className={`flex items-center gap-2 p-2 rounded border text-sm ${
                      agent.detected ? 'border-primary/30' : 'border-border opacity-50'
                    }`}
                  >
                    <div
                      className={`h-2 w-2 rounded-full ${
                        agent.detected ? 'bg-primary' : 'bg-muted-foreground'
                      }`}
                    />
                    {agent.name}
                  </div>
                ))}
              </div>
            )}
            <div className="flex gap-2 mt-4">
              <Button size="sm" onClick={() => scan()} disabled={scanning}>
                <Search className="h-4 w-4 mr-1" />
                {scanning ? 'Scanning...' : 'Scan Agent Folders'}
              </Button>
            </div>
          </CardContent>
        </Card>

        {/* Custom Path Scan */}
        <Card className="mt-3">
          <CardHeader>
            <CardTitle className="text-base">Custom Path</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex gap-2">
              <Input
                placeholder="/path/to/scan"
                value={customPath}
                onChange={(e) => setCustomPath(e.target.value)}
              />
              <Button
                size="sm"
                onClick={() => scan(customPath)}
                disabled={scanning || !customPath}
              >
                Scan
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>

      {detectedAgents.length > 0 && (
        <div>
          <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Adopt from an installed client
          </h2>
          <p className="text-xs text-muted-foreground mb-3">
            Find skills already living directly inside a client's own skills folder (not yet in your cookbook),
            move them in, and leave a symlink behind so the client keeps working.
          </p>
          <Card>
            <CardContent className="p-4 space-y-2">
              <div className="flex items-center gap-2">
                <Sparkles className="h-3.5 w-3.5 text-primary" />
                <p className="text-sm font-medium">Scan a client's skills folder</p>
              </div>
              <div className="flex flex-wrap gap-2 pt-1">
                {detectedAgents.map((client) => (
                  <Button key={client.id} variant="outline" size="sm" onClick={() => setAdoptClient(client)}>
                    Scan {client.name}
                  </Button>
                ))}
              </div>
            </CardContent>
            {adoptClient && (
              <AdoptDialog client={adoptClient} onClose={() => setAdoptClient(null)} />
            )}
          </Card>
        </div>
      )}

      <div>
        <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">
          Import from GitHub
        </h2>
        <p className="text-xs text-muted-foreground mb-3">
          Fetches a public GitHub repo into a temporary local sandbox (never installed anywhere) to scan it for
          SKILL.md files. The security review reads that same sandbox copy directly — no second download — and
          nothing is added to your cookbook until you explicitly click Install.
        </p>
        <GithubImportSection reviewCli={config?.default_review_cli ?? null} />
      </div>

      {/* Results */}
      {skills.length > 0 && (
        <Card>
          <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle className="text-base">
              Found {skills.length} skill(s)
            </CardTitle>
            <Button
              size="sm"
              disabled={selected.size === 0 || importing}
              onClick={importSelected}
            >
              <Download className="h-4 w-4 mr-1" />
              {importing ? 'Importing...' : `Import ${selected.size} Selected`}
            </Button>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {skills.map((skill) => (
                <label
                  key={skill.path}
                  className="flex items-center gap-3 p-2 rounded hover:bg-muted/50 cursor-pointer"
                >
                  <input
                    type="checkbox"
                    className="checkbox checkbox-sm checkbox-primary"
                    checked={selected.has(skill.path)}
                    onChange={() => toggleSkill(skill.path)}
                    disabled={skill.already_in_chef}
                  />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">{skill.name}</p>
                    <p className="text-xs text-muted-foreground truncate">{skill.path}</p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge variant="outline" className="text-[10px]">
                      {skill.source_agent}
                    </Badge>
                    {skill.already_in_chef && (
                      <Badge variant="secondary" className="text-[10px]">
                        in chef
                      </Badge>
                    )}
                  </div>
                </label>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

function AdoptDialog({
  client,
  onClose,
}: {
  client: DiscoveredClient;
  onClose: () => void;
}) {
  const [candidates, setCandidates] = useState<AdoptCandidate[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [applying, setApplying] = useState(false);
  const [adopted, setAdopted] = useState<string[] | null>(null);

  useEffect(() => {
    setLoading(true);
    commands
      .adoptScan(client.id)
      .then((res) => {
        setCandidates(res);
        setSelected(new Set(res.map((c) => c.name)));
      })
      .catch(() => setCandidates([]))
      .finally(() => setLoading(false));
  }, [client.id]);

  const toggle = (name: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  const handleAdopt = async () => {
    setApplying(true);
    try {
      const names = Array.from(selected);
      await commands.adoptApply(client.id, names);
      setAdopted(names);
    } catch (e) {
      console.error('Failed to adopt skills:', e);
    } finally {
      setApplying(false);
    }
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Adopt skills from {client.name}</DialogTitle>
        </DialogHeader>
        {adopted ? (
          <p className="text-sm text-muted-foreground">
            Adopted {adopted.length} skill(s) — find them in Chef &gt; Local Skills.
          </p>
        ) : loading ? (
          <div className="space-y-2">
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-full" />
          </div>
        ) : candidates.length === 0 ? (
          <p className="text-sm text-muted-foreground">No un-adopted skills found.</p>
        ) : (
          <div className="space-y-1.5 max-h-64 overflow-y-auto">
            {candidates.map((candidate) => (
              <label
                key={candidate.name}
                className="flex items-center gap-2 text-sm py-1 select-none"
              >
                <input
                  type="checkbox"
                  className="h-3.5 w-3.5 rounded border-input accent-primary"
                  checked={selected.has(candidate.name)}
                  onChange={() => toggle(candidate.name)}
                />
                <div>
                  <p className="font-medium">{candidate.name}</p>
                  <p className="text-xs text-muted-foreground">{candidate.path}</p>
                </div>
              </label>
            ))}
          </div>
        )}
        <DialogFooter>
          {adopted ? (
            <Button size="sm" onClick={onClose}>
              Done
            </Button>
          ) : (
            <>
              <Button variant="outline" size="sm" onClick={onClose}>
                Cancel
              </Button>
              <Button
                size="sm"
                disabled={applying || selected.size === 0}
                onClick={handleAdopt}
              >
                Adopt selected
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface ReviewState {
  status: 'idle' | 'reviewing' | 'reviewed' | 'error';
  review?: SecurityReview;
  error?: string;
  showRaw?: boolean;
  installed?: boolean;
  installing?: boolean;
}

function verdictBadge(verdict: string) {
  switch (verdict) {
    case 'safe':
      return <Badge variant="success">Safe</Badge>;
    case 'concerns':
      return <Badge variant="warning">Concerns</Badge>;
    case 'unsafe':
      return <Badge variant="destructive">Unsafe</Badge>;
    default:
      return <Badge variant="secondary">Inconclusive</Badge>;
  }
}

function GithubImportSection({ reviewCli }: { reviewCli: string | null }) {
  const [url, setUrl] = useState('');
  const [scanning, setScanning] = useState(false);
  const [scanned, setScanned] = useState<ScannedSkillInfo[]>([]);
  const [repoSlug, setRepoSlug] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [reviews, setReviews] = useState<Record<string, ReviewState>>({});
  const [reviewingAll, setReviewingAll] = useState(false);
  const [scanError, setScanError] = useState('');

  const scan = async () => {
    if (!url) return;
    setScanning(true);
    setScanError('');
    setScanned([]);
    setReviews({});
    setSelected(new Set());
    try {
      const [found, slug] = await Promise.all([
        commands.scanGithubRepo(url),
        commands.githubRepoSlug(url).catch(() => null),
      ]);
      setScanned(found);
      setRepoSlug(slug);
      const preselect = new Set<string>();
      for (const s of found) {
        if (!s.already_in_chef) preselect.add(s.path);
      }
      setSelected(preselect);
    } catch (e) {
      setScanError(String(e));
    } finally {
      setScanning(false);
    }
  };

  const toggle = (path: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const reviewSelected = async () => {
    if (!reviewCli) return;
    setReviewingAll(true);
    const targets = scanned.filter((s) => selected.has(s.path));
    for (const skill of targets) {
      setReviews((prev) => ({ ...prev, [skill.path]: { status: 'reviewing' } }));
      try {
        const review = await commands.runSkillSecurityReview(skill.path, reviewCli);
        setReviews((prev) => ({ ...prev, [skill.path]: { status: 'reviewed', review } }));
      } catch (e) {
        setReviews((prev) => ({ ...prev, [skill.path]: { status: 'error', error: String(e) } }));
      }
    }
    setReviewingAll(false);
  };

  const toggleRaw = (path: string) => {
    setReviews((prev) => ({
      ...prev,
      [path]: { ...prev[path], showRaw: !prev[path]?.showRaw },
    }));
  };

  const install = async (skill: ScannedSkillInfo) => {
    const state = reviews[skill.path];
    if (state?.review?.verdict === 'unsafe') {
      const proceed = window.confirm(
        `The security review flagged "${skill.name}" as UNSAFE:\n\n${state.review.summary}\n\n` +
          'Install anyway? Only proceed if you understand and accept this risk.',
      );
      if (!proceed) return;
    }
    setReviews((prev) => ({ ...prev, [skill.path]: { ...prev[skill.path], installing: true } }));
    try {
      const source = repoSlug ? `github:${repoSlug}` : undefined;
      await commands.importSkills([skill.path], [skill.name], source);
      setReviews((prev) => ({
        ...prev,
        [skill.path]: { ...prev[skill.path], installing: false, installed: true },
      }));
      setScanned((prev) =>
        prev.map((s) => (s.path === skill.path ? { ...s, already_in_chef: true } : s)),
      );
    } catch (e) {
      setReviews((prev) => ({
        ...prev,
        [skill.path]: { ...prev[skill.path], installing: false, error: String(e) },
      }));
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base flex items-center gap-2">
          <Github className="h-4 w-4" />
          Scan a GitHub repository
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex gap-2">
          <Input
            placeholder="https://github.com/owner/repo"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
          />
          <Button size="sm" onClick={scan} disabled={scanning || !url}>
            <Search className="h-4 w-4 mr-1" />
            {scanning ? 'Scanning...' : 'Scan'}
          </Button>
        </div>
        {scanError && <p className="text-xs text-red-600">{scanError}</p>}

        {!reviewCli && (
          <div className="flex items-center justify-between gap-3 rounded border border-amber-300 bg-amber-50 dark:border-amber-900 dark:bg-amber-950/40 px-3 py-2 text-xs text-amber-800 dark:text-amber-200">
            <span className="flex items-center gap-1.5">
              <ShieldAlert className="h-3.5 w-3.5 shrink-0" />
              Set a default review CLI in Settings before importing skills from GitHub.
            </span>
            <Button asChild size="sm" variant="outline">
              <Link to="/settings">Go to Settings</Link>
            </Button>
          </div>
        )}

        {scanned.length > 0 && (
          <>
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Found {scanned.length} skill(s)</p>
              <Button
                size="sm"
                variant="outline"
                disabled={!reviewCli || selected.size === 0 || reviewingAll}
                onClick={reviewSelected}
              >
                {reviewingAll ? 'Reviewing...' : 'Review selected'}
              </Button>
            </div>
            <div className="space-y-2">
              {scanned.map((skill) => {
                const state = reviews[skill.path];
                return (
                  <div key={skill.path} className="rounded border p-2 space-y-2">
                    <label className="flex items-center gap-3 cursor-pointer">
                      <input
                        type="checkbox"
                        className="checkbox checkbox-sm checkbox-primary"
                        checked={selected.has(skill.path)}
                        onChange={() => toggle(skill.path)}
                        disabled={skill.already_in_chef || reviewingAll}
                      />
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium truncate">{skill.name}</p>
                        <p className="text-xs text-muted-foreground truncate">
                          {skill.display_path ?? skill.path}
                        </p>
                      </div>
                      <div className="flex items-center gap-2">
                        <Badge
                          variant="outline"
                          className="text-[10px]"
                          title="Fetched to a temporary local copy for scanning/review only — nothing is installed until you click Install."
                        >
                          temporary sandbox
                        </Badge>
                        {skill.already_in_chef && (
                          <Badge variant="secondary" className="text-[10px]">
                            in chef
                          </Badge>
                        )}
                      </div>
                    </label>

                    {state?.status === 'reviewing' && (
                      <div className="flex items-center gap-2 text-xs text-muted-foreground pl-7">
                        <Loader2 className="h-3.5 w-3.5 animate-spin" />
                        Running security review...
                      </div>
                    )}

                    {state?.status === 'error' && (
                      <p className="text-xs text-red-600 pl-7">Review failed: {state.error}</p>
                    )}

                    {state?.status === 'reviewed' && state.review && (
                      <div className="pl-7 space-y-1.5">
                        <div className="flex items-center gap-2">
                          {verdictBadge(state.review.verdict)}
                          <p className="text-xs text-muted-foreground flex-1">{state.review.summary}</p>
                        </div>
                        <button
                          type="button"
                          className="text-[11px] text-muted-foreground underline underline-offset-2"
                          onClick={() => toggleRaw(skill.path)}
                        >
                          {state.showRaw ? 'Hide full output' : 'Show full output'}
                        </button>
                        {state.showRaw && (
                          <pre className="text-[10px] whitespace-pre-wrap bg-muted/50 rounded p-2 max-h-48 overflow-y-auto">
                            {state.review.raw_output}
                          </pre>
                        )}
                        <div>
                          {state.installed ? (
                            <Badge variant="secondary" className="text-[10px]">
                              Installed
                            </Badge>
                          ) : (
                            <Button
                              size="sm"
                              variant="outline"
                              disabled={state.installing}
                              onClick={() => install(skill)}
                            >
                              <Download className="h-3.5 w-3.5 mr-1" />
                              {state.installing ? 'Installing...' : 'Install'}
                            </Button>
                          )}
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </>
        )}

        <p className="text-[11px] text-muted-foreground pt-1">
          AI security review is a heuristic check, not a guarantee — use judgment.
        </p>
      </CardContent>
    </Card>
  );
}
