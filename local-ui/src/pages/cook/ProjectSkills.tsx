import { useEffect, useState } from 'react';
import { FolderGit2, FolderOpen, Trash2, UploadCloud } from 'lucide-react';
import { commands, type LocalSkill, type ProjectSkillEntry } from '@/lib/tauri';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';

const PROJECT_TARGETS: { id: string; label: string }[] = [
  { id: 'claude_code_project', label: 'Claude Code' },
  { id: 'codex_project', label: 'Codex' },
];

export function ProjectSkills() {
  // Not persisted, not shared with any other page — a fresh, page-local concern.
  const [projectDir, setProjectDir] = useState<string | null>(null);
  const [published, setPublished] = useState<ProjectSkillEntry[]>([]);
  const [loadingPublished, setLoadingPublished] = useState(false);
  const [removing, setRemoving] = useState<string | null>(null);

  const [cookbookSkills, setCookbookSkills] = useState<LocalSkill[]>([]);
  const [loadingCookbook, setLoadingCookbook] = useState(true);
  const [selectedSkills, setSelectedSkills] = useState<Set<string>>(new Set());
  const [selectedTargets, setSelectedTargets] = useState<Record<string, Set<string>>>({});
  const [publishing, setPublishing] = useState(false);

  useEffect(() => {
    commands
      .listLocalSkills()
      .then(setCookbookSkills)
      .catch(() => setCookbookSkills([]))
      .finally(() => setLoadingCookbook(false));
  }, []);

  const refreshPublished = (dir: string) => {
    setLoadingPublished(true);
    commands
      .listProjectSkills(dir)
      .then(setPublished)
      .catch(() => setPublished([]))
      .finally(() => setLoadingPublished(false));
  };

  const pickProjectFolder = async () => {
    try {
      const path = await commands.pickFolder('Select project repository folder');
      if (!path) return;
      setProjectDir(path);
      refreshPublished(path);
    } catch (e) {
      console.error('Failed to pick project folder:', e);
    }
  };

  const handleRemove = async (entry: ProjectSkillEntry) => {
    if (!projectDir) return;
    if (!window.confirm(`Remove "${entry.name}" from this project? This cannot be undone.`)) return;
    setRemoving(entry.name);
    try {
      await commands.unpublishSkillFromProject(projectDir, entry.name, entry.targets);
      refreshPublished(projectDir);
    } catch (e) {
      console.error('Failed to remove skill from project:', e);
    } finally {
      setRemoving(null);
    }
  };

  const toggleSkillSelected = (name: string) => {
    setSelectedSkills((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
    setSelectedTargets((prev) =>
      prev[name] ? prev : { ...prev, [name]: new Set(['claude_code_project', 'codex_project']) }
    );
  };

  const toggleSkillTarget = (name: string, targetId: string) => {
    setSelectedTargets((prev) => {
      const current = prev[name] ?? new Set(['claude_code_project', 'codex_project']);
      const next = new Set(current);
      if (next.has(targetId)) next.delete(targetId);
      else next.add(targetId);
      return { ...prev, [name]: next };
    });
  };

  const handlePublishSelected = async () => {
    if (!projectDir || selectedSkills.size === 0) return;
    setPublishing(true);
    try {
      for (const name of selectedSkills) {
        const targets = Array.from(selectedTargets[name] ?? ['claude_code_project', 'codex_project']);
        if (targets.length === 0) continue;
        await commands.publishSkillToProject(name, projectDir, targets);
      }
      setSelectedSkills(new Set());
      refreshPublished(projectDir);
    } catch (e) {
      console.error('Failed to publish skills to project:', e);
    } finally {
      setPublishing(false);
    }
  };

  const publishedByName = new Map(published.map((p) => [p.name, p]));
  const fullyPublished = (name: string) => {
    const entry = publishedByName.get(name);
    return !!entry && entry.targets.length >= PROJECT_TARGETS.length;
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <h1 className="text-lg font-semibold">Project Skills</h1>
        <Button size="sm" variant="outline" onClick={pickProjectFolder}>
          <FolderOpen className="h-4 w-4 mr-1" />
          Choose project folder...
        </Button>
      </div>

      {projectDir && (
        <p className="text-xs text-muted-foreground truncate" title={projectDir}>
          {projectDir}
        </p>
      )}

      {!projectDir ? (
        <div className="text-center py-12">
          <FolderGit2 className="h-10 w-10 text-muted-foreground mx-auto mb-3" />
          <h2 className="text-base font-medium">No project selected</h2>
          <p className="text-sm text-muted-foreground mt-1">
            Choose a project folder to manage its published skills.
          </p>
        </div>
      ) : (
        <>
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Currently published</CardTitle>
            </CardHeader>
            <CardContent>
              {loadingPublished ? (
                <div className="space-y-2">
                  {[1, 2].map((i) => (
                    <Skeleton key={i} className="h-10 w-full" />
                  ))}
                </div>
              ) : published.length === 0 ? (
                <p className="text-sm text-muted-foreground py-4 text-center">
                  Nothing published to this project yet.
                </p>
              ) : (
                <div className="space-y-1.5">
                  {published.map((entry) => (
                    <div
                      key={entry.name}
                      className="flex items-center justify-between gap-3 rounded border px-3 py-2"
                    >
                      <div className="flex items-center gap-2 min-w-0">
                        <span className="text-sm font-medium truncate">{entry.name}</span>
                        {PROJECT_TARGETS.map(
                          (t) =>
                            entry.targets.includes(t.id) && (
                              <Badge key={t.id} variant="outline" className="text-[10px] px-1.5 py-0">
                                {t.label}
                              </Badge>
                            )
                        )}
                      </div>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        disabled={removing === entry.name}
                        onClick={() => handleRemove(entry)}
                        title="Remove from project"
                      >
                        <Trash2 className="h-3.5 w-3.5 text-destructive" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle className="text-base">Add skills to this project</CardTitle>
              <Button
                size="sm"
                disabled={selectedSkills.size === 0 || publishing}
                onClick={handlePublishSelected}
              >
                <UploadCloud className="h-4 w-4 mr-1" />
                {publishing ? 'Publishing...' : `Publish ${selectedSkills.size || ''}`.trim()}
              </Button>
            </CardHeader>
            <CardContent>
              {loadingCookbook ? (
                <div className="space-y-2">
                  {[1, 2, 3].map((i) => (
                    <Skeleton key={i} className="h-10 w-full" />
                  ))}
                </div>
              ) : cookbookSkills.length === 0 ? (
                <p className="text-sm text-muted-foreground py-4 text-center">
                  No skills in your cookbook yet — add some from Chef first.
                </p>
              ) : (
                <div className="space-y-1.5">
                  {cookbookSkills.map((skill) => {
                    const targets = selectedTargets[skill.name] ?? new Set(['claude_code_project', 'codex_project']);
                    const already = fullyPublished(skill.name);
                    return (
                      <div
                        key={skill.name}
                        className={`flex items-center gap-3 rounded border px-3 py-2 ${
                          already ? 'opacity-60' : ''
                        }`}
                      >
                        <input
                          type="checkbox"
                          className="h-3.5 w-3.5 rounded border-input accent-primary shrink-0"
                          checked={selectedSkills.has(skill.name)}
                          onChange={() => toggleSkillSelected(skill.name)}
                        />
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-1.5">
                            <span className="text-sm font-medium truncate">{skill.name}</span>
                            {already && (
                              <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                                already published
                              </Badge>
                            )}
                          </div>
                          {skill.description && (
                            <p className="text-xs text-muted-foreground truncate">{skill.description}</p>
                          )}
                        </div>
                        <div className="flex items-center gap-3 shrink-0">
                          {PROJECT_TARGETS.map((t) => (
                            <label key={t.id} className="flex items-center gap-1.5 text-xs select-none">
                              <input
                                type="checkbox"
                                className="h-3.5 w-3.5 rounded border-input accent-primary"
                                checked={targets.has(t.id)}
                                onChange={() => toggleSkillTarget(skill.name, t.id)}
                              />
                              {t.label}
                            </label>
                          ))}
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
