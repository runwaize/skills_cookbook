import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FileText, Trash2, Plus, ChevronRight, ChevronDown, X, Sparkles } from 'lucide-react';
import { commands, type LocalSkill, type DiscoveredClient, type SkillMeta } from '@/lib/tauri';
import { useRole } from '@/contexts/RoleContext';
import { managedAndDetected } from '@/lib/clients';
import { relativeTime, localeDate } from '@/lib/format';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import { MetadataPanel, statusBadgeClass } from '@/components/MetadataPanel';

type ActiveFilter = 'all' | 'active' | 'inactive';
type SortBy = 'name' | 'updated' | 'installed';

function skillToMeta(skill: LocalSkill): SkillMeta {
  return {
    name: skill.name,
    status: skill.status,
    version: skill.version,
    tags: skill.tags,
    description: skill.description,
    source: skill.source,
    created_at: skill.created_at,
    updated_at: skill.updated_at,
    synced_at: null,
  };
}

export function SkillsList() {
  const navigate = useNavigate();
  const { config } = useRole();
  const [skills, setSkills] = useState<LocalSkill[]>([]);
  const [loading, setLoading] = useState(true);
  const [clients, setClients] = useState<DiscoveredClient[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState('');
  const [activeFilter, setActiveFilter] = useState<ActiveFilter>('all');
  const [tagFilter, setTagFilter] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<SortBy>('name');

  const refresh = () => {
    setLoading(true);
    commands.listLocalSkills().then(setSkills).catch(() => setSkills([])).finally(() => setLoading(false));
  };

  // Silent refresh: re-fetches the list without flipping `loading`, so expanded
  // rows and scroll position survive actions taken from within a row.
  const softRefresh = () => {
    commands.listLocalSkills().then(setSkills).catch(() => {});
  };

  useEffect(refresh, []);

  useEffect(() => {
    commands.discoverAgents().then(setClients).catch(() => setClients([]));
  }, []);

  const detectedClients = managedAndDetected(clients, config?.managed_client_ids ?? []);

  const toggleExpanded = (name: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  const openEditor = (name: string) => {
    navigate(`/chef/edit/${encodeURIComponent(name)}`);
  };

  const toggleActive = async (skill: LocalSkill) => {
    setBusy(skill.name);
    try {
      if (skill.active) {
        await commands.deactivateSkill(skill.name);
      } else {
        await commands.activateSkill(skill.name);
      }
      softRefresh();
    } catch (e) {
      console.error('Failed to toggle active state:', e);
    } finally {
      setBusy(null);
    }
  };

  const toggleTarget = async (skill: LocalSkill, clientId: string) => {
    const next = skill.targets.includes(clientId)
      ? skill.targets.filter((t) => t !== clientId)
      : [...skill.targets, clientId];
    setBusy(skill.name);
    try {
      await commands.setSkillTargets(skill.name, next);
      softRefresh();
    } catch (e) {
      console.error('Failed to update targets:', e);
    } finally {
      setBusy(null);
    }
  };

  const handleDelete = async (skill: LocalSkill) => {
    if (!window.confirm(`Delete skill "${skill.name}"? This cannot be undone.`)) return;
    setBusy(skill.name);
    try {
      await commands.deleteSkill(skill.name);
      refresh();
    } catch (e) {
      console.error('Failed to delete skill:', e);
    } finally {
      setBusy(null);
    }
  };

  const handleAddSkill = async () => {
    try {
      const path = await commands.pickFolder('Select skill folder');
      if (!path) return;
      await commands.addSkill(path);
      refresh();
    } catch (e) {
      console.error('Failed to add skill:', e);
    }
  };

  const handleMetaUpdated = (updated: SkillMeta) => {
    setSkills((prev) =>
      prev.map((s) =>
        s.name === updated.name
          ? {
              ...s,
              status: updated.status,
              version: updated.version,
              tags: updated.tags,
              description: updated.description,
              source: updated.source,
              updated_at: updated.updated_at,
            }
          : s
      )
    );
  };

  const filteredSkills = useMemo(() => {
    const q = search.trim().toLowerCase();
    const matched = skills.filter((skill) => {
      if (activeFilter === 'active' && !skill.active) return false;
      if (activeFilter === 'inactive' && skill.active) return false;
      if (tagFilter && !skill.tags.includes(tagFilter)) return false;
      if (!q) return true;
      const haystack = [skill.name, skill.description ?? '', ...skill.tags].join(' ').toLowerCase();
      return haystack.includes(q);
    });
    // created_at/updated_at are RFC3339 strings, which sort lexicographically in
    // the same order as chronologically — no need to parse into Date objects.
    return matched.sort((a, b) => {
      if (sortBy === 'updated') return b.updated_at.localeCompare(a.updated_at);
      if (sortBy === 'installed') return b.created_at.localeCompare(a.created_at);
      return a.name.localeCompare(b.name);
    });
  }, [skills, search, activeFilter, tagFilter, sortBy]);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <h1 className="text-lg font-semibold">Skills</h1>
        <div className="flex items-center gap-2">
          <Input
            className="h-8 w-56 text-sm"
            placeholder="Search skills..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <Button size="sm" onClick={handleAddSkill}>
            <Plus className="h-4 w-4 mr-1" />
            Add Skill
          </Button>
        </div>
      </div>

      <div className="flex items-center gap-2 flex-wrap">
        <div className="inline-flex rounded-md border overflow-hidden">
          {(['all', 'active', 'inactive'] as ActiveFilter[]).map((f) => (
            <button
              key={f}
              onClick={() => setActiveFilter(f)}
              className={`px-2.5 py-1 text-xs capitalize ${
                activeFilter === f ? 'bg-primary text-primary-foreground' : 'bg-transparent hover:bg-muted'
              }`}
            >
              {f}
            </button>
          ))}
        </div>
        {tagFilter && (
          <Badge variant="outline" className="text-xs gap-1 py-0.5">
            tag: {tagFilter}
            <button onClick={() => setTagFilter(null)}>
              <X className="h-3 w-3" />
            </button>
          </Badge>
        )}
        <label className="flex items-center gap-1.5 text-xs text-muted-foreground">
          Sort:
          <select
            className="h-6 text-xs rounded border bg-transparent px-1"
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as SortBy)}
          >
            <option value="name">Name</option>
            <option value="updated">Recently updated</option>
            <option value="installed">Recently installed</option>
          </select>
        </label>
        <span className="text-xs text-muted-foreground ml-auto">
          {filteredSkills.length} of {skills.length} skills
        </span>
        <Button variant="ghost" size="sm" className="text-xs" onClick={() => navigate('/chef/discover')}>
          <Sparkles className="h-3.5 w-3.5 mr-1" />
          Find skills to import
        </Button>
      </div>

      {loading ? (
        <div className="space-y-1.5">
          {[1, 2, 3, 4, 5].map((i) => (
            <Skeleton key={i} className="h-10 w-full" />
          ))}
        </div>
      ) : skills.length === 0 ? (
        <div className="text-center py-10">
          <FileText className="h-10 w-10 text-muted-foreground mx-auto mb-3" />
          <h2 className="text-base font-medium">No skills yet</h2>
          <p className="text-sm text-muted-foreground mt-1">
            Add skills from the Discover page or use <code>chef add</code> in the CLI.
          </p>
          <Button className="mt-4" onClick={() => navigate('/chef/discover')}>
            <Search className="h-4 w-4 mr-2" />
            Discover Skills
          </Button>
        </div>
      ) : filteredSkills.length === 0 ? (
        <p className="text-sm text-muted-foreground text-center py-8">
          No skills match your search or filters.
        </p>
      ) : (
        <div className="border rounded-md divide-y">
          {filteredSkills.map((skill) => (
            <SkillRow
              key={skill.name}
              skill={skill}
              busy={busy === skill.name}
              expanded={expanded.has(skill.name)}
              onToggleExpanded={() => toggleExpanded(skill.name)}
              detectedClients={detectedClients}
              onToggleActive={() => toggleActive(skill)}
              onToggleTarget={(clientId) => toggleTarget(skill, clientId)}
              onEdit={() => openEditor(skill.name)}
              onDelete={() => handleDelete(skill)}
              onTagClick={(tag) => setTagFilter(tag)}
              onMetaUpdated={handleMetaUpdated}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function SkillRow({
  skill,
  busy,
  expanded,
  onToggleExpanded,
  detectedClients,
  onToggleActive,
  onToggleTarget,
  onEdit,
  onDelete,
  onTagClick,
  onMetaUpdated,
}: {
  skill: LocalSkill;
  busy: boolean;
  expanded: boolean;
  onToggleExpanded: () => void;
  detectedClients: DiscoveredClient[];
  onToggleActive: () => void;
  onToggleTarget: (clientId: string) => void;
  onEdit: () => void;
  onDelete: () => void;
  onTagClick: (tag: string) => void;
  onMetaUpdated: (meta: SkillMeta) => void;
}) {
  const [descExpanded, setDescExpanded] = useState(false);
  const description = skill.description ?? '';
  const isLongDescription = description.length > 240;
  const shownDescription = isLongDescription && !descExpanded
    ? `${description.slice(0, 240)}…`
    : description;

  return (
    <div>
      <div className="flex items-center gap-2 px-2 py-1.5 hover:bg-muted/40 min-h-[40px]">
        <button
          onClick={onToggleExpanded}
          className="shrink-0 text-muted-foreground hover:text-foreground"
          title="Show details"
        >
          {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
        </button>
        <FileText className="h-3.5 w-3.5 text-primary shrink-0" />

        <div className="flex items-center gap-1.5 min-w-0 flex-1">
          <button
            onClick={onEdit}
            className="font-medium text-sm truncate shrink-0 max-w-[220px] text-left hover:underline"
            title="Edit"
          >
            {skill.name}
          </button>
          <span className="text-[10px] text-muted-foreground shrink-0">v{skill.version}</span>
          {skill.tags.slice(0, 2).map((tag) => (
            <button key={tag} onClick={() => onTagClick(tag)}>
              <Badge variant="outline" className="text-[10px] px-1 py-0 cursor-pointer hover:bg-muted">
                {tag}
              </Badge>
            </button>
          ))}
        </div>

        <div className="flex items-center gap-1 shrink-0">
          {!skill.has_skill_md && (
            <Badge variant="destructive" className="text-[10px] px-1 py-0">
              No SKILL.md
            </Badge>
          )}
          <Badge variant={skill.active ? 'default' : 'outline'} className="text-[10px] px-1.5 py-0">
            {skill.active ? 'Active' : 'Inactive'}
          </Badge>
          <Badge variant="outline" className={`text-[10px] px-1.5 py-0 ${statusBadgeClass(skill.status)}`}>
            {skill.status}
          </Badge>
          <Badge variant="outline" className="text-[10px] px-1.5 py-0" title={`Source: ${skill.source || 'unknown'}`}>
            {skill.source || 'unknown'}
          </Badge>
          <span
            className="text-[10px] text-muted-foreground whitespace-nowrap"
            title={`Installed ${localeDate(skill.created_at)}`}
          >
            {relativeTime(skill.updated_at)}
          </span>
        </div>

        <div className="flex items-center gap-1.5 shrink-0 ml-1 pl-2 border-l">
          <input
            type="checkbox"
            className="h-3.5 w-3.5 rounded border-input accent-primary"
            checked={skill.active}
            disabled={busy}
            onChange={onToggleActive}
            title={skill.active ? 'Deactivate' : 'Activate'}
          />
          <Button variant="ghost" size="icon-sm" disabled={busy} onClick={onDelete} title="Delete skill">
            <Trash2 className="h-3.5 w-3.5 text-destructive" />
          </Button>
        </div>
      </div>

      {expanded && (
        <div className="pl-8 pr-2 pb-2 pt-2 bg-muted/20 border-t">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 items-start">
            {/* Left: descriptive content */}
            <div className="space-y-2 min-w-0">
              <span className="text-[10px] uppercase tracking-wide text-muted-foreground">Description</span>
              {description ? (
                <p className="text-sm text-foreground/90 whitespace-pre-wrap break-words min-h-[1.25rem]">
                  {shownDescription}
                  {isLongDescription && (
                    <button
                      type="button"
                      className="ml-1.5 text-xs text-primary underline underline-offset-2 shrink-0"
                      onClick={() => setDescExpanded((v) => !v)}
                    >
                      {descExpanded ? 'Show less' : 'Show more'}
                    </button>
                  )}
                </p>
              ) : (
                <p className="text-sm text-muted-foreground italic min-h-[1.25rem]">No description yet.</p>
              )}

              {skill.tags.length > 0 && (
                <div className="flex flex-wrap items-center gap-1.5 pt-1">
                  {skill.tags.map((tag) => (
                    <button key={tag} onClick={() => onTagClick(tag)}>
                      <Badge variant="outline" className="text-[10px] px-1.5 py-0 cursor-pointer hover:bg-muted">
                        {tag}
                      </Badge>
                    </button>
                  ))}
                </div>
              )}
            </div>

            {/* Right: actionable controls */}
            <div className="space-y-2 min-w-0">
              {detectedClients.length > 0 && (
                <div className="flex flex-wrap items-center gap-3">
                  <span className="text-[10px] uppercase tracking-wide text-muted-foreground">Deploy to</span>
                  {detectedClients.map((client) => (
                    <label key={client.id} className="flex items-center gap-1.5 text-xs select-none">
                      <input
                        type="checkbox"
                        className="h-3.5 w-3.5 rounded border-input accent-primary"
                        checked={skill.targets.includes(client.id)}
                        disabled={busy}
                        onChange={() => onToggleTarget(client.id)}
                      />
                      {client.name}
                    </label>
                  ))}
                </div>
              )}

              <MetadataPanel skillName={skill.name} meta={skillToMeta(skill)} onMetaUpdated={onMetaUpdated} />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Search(props: React.SVGProps<SVGSVGElement>) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" {...props}>
      <circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" />
    </svg>
  );
}
