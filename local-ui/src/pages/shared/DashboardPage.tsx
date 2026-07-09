import { useEffect, useMemo, useState } from 'react';
import { BarChart3, RotateCw, X } from 'lucide-react';
import { commands, type DiscoveredClient, type UsageEvent } from '@/lib/tauri';
import { relativeTime } from '@/lib/format';
import { useRole } from '@/contexts/RoleContext';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

/** Friendly display name for a client id. */
function clientLabel(client: string): string {
  return client === 'codex' ? 'Codex' : 'Claude Code';
}

/** Shorten a long absolute path to its last few segments for compact display. */
function shortPath(path: string, segments = 3): string {
  if (!path) return '';
  const parts = path.split('/').filter(Boolean);
  if (parts.length <= segments) return path;
  return `.../${parts.slice(-segments).join('/')}`;
}

const PERIOD_OPTIONS = [7, 30, 90] as const;

/** Clients we actually track usage for today. */
const TRACKED = ['claude_code', 'codex'];

/** Format a Date as a local YYYY-MM-DD string (matches the backend's date format). */
function toDateKey(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/** Short "Mon D" label for axis ticks. */
function shortLabel(dateKey: string): string {
  const d = new Date(`${dateKey}T00:00:00`);
  if (Number.isNaN(d.getTime())) return dateKey;
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

interface DayBucket {
  date: string;
  claudeCode: number;
  codex: number;
  total: number;
}

interface SkillRow {
  name: string;
  count: number;
  claudeCode: number;
  codex: number;
  lastUsedIso: string;
}

type ClientFilter = 'all' | 'claude_code' | 'codex';

export function DashboardPage() {
  const { config } = useRole();
  const [days, setDays] = useState<number>(30);
  const [events, setEvents] = useState<UsageEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [resyncing, setResyncing] = useState(false);
  const [clients, setClients] = useState<DiscoveredClient[]>([]);
  const [clientFilter, setClientFilter] = useState<ClientFilter>('all');
  const [selectedSkill, setSelectedSkill] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    commands
      .getSkillUsage(days)
      .then((result) => {
        if (!cancelled) setEvents(result);
      })
      .catch(() => {
        if (!cancelled) setEvents([]);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [days]);

  useEffect(() => {
    commands.discoverAgents().then(setClients).catch(() => setClients([]));
  }, []);

  const handleResync = async () => {
    setResyncing(true);
    try {
      const result = await commands.resyncSkillUsage(days);
      setEvents(result);
    } catch (e) {
      console.error('Re-sync failed:', e);
    } finally {
      setResyncing(false);
    }
  };

  const managedButUntracked = useMemo(
    () => (config?.managed_client_ids ?? []).filter((id) => !TRACKED.includes(id)),
    [config?.managed_client_ids],
  );

  const untrackedNames = useMemo(
    () =>
      managedButUntracked.map((id) => clients.find((c) => c.id === id)?.name ?? id),
    [managedButUntracked, clients],
  );

  const filteredEvents = useMemo(
    () => (clientFilter === 'all' ? events : events.filter((ev) => ev.client === clientFilter)),
    [events, clientFilter],
  );

  // If the selected skill no longer has any events under the current
  // source filter / period (e.g. filtered to a client it was never used
  // with), fall back to the unfiltered view rather than showing an
  // orphaned selection with a permanently-empty chart.
  useEffect(() => {
    if (selectedSkill && !filteredEvents.some((ev) => ev.skill === selectedSkill)) {
      setSelectedSkill(null);
    }
  }, [selectedSkill, filteredEvents]);

  const chartEvents = useMemo(
    () =>
      selectedSkill ? filteredEvents.filter((ev) => ev.skill === selectedSkill) : filteredEvents,
    [filteredEvents, selectedSkill],
  );

  const detailEvents = useMemo(
    () =>
      selectedSkill
        ? filteredEvents
            .filter((ev) => ev.skill === selectedSkill)
            .slice()
            .sort((a, b) => (a.date < b.date ? 1 : a.date > b.date ? -1 : 0))
        : [],
    [filteredEvents, selectedSkill],
  );

  const handleSelectSkill = (name: string) => {
    setSelectedSkill((prev) => (prev === name ? null : name));
  };

  const dayBuckets = useMemo<DayBucket[]>(() => {
    const byDate = new Map<string, { claudeCode: number; codex: number }>();
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    for (let i = days - 1; i >= 0; i--) {
      const d = new Date(today);
      d.setDate(d.getDate() - i);
      byDate.set(toDateKey(d), { claudeCode: 0, codex: 0 });
    }
    for (const ev of chartEvents) {
      const bucket = byDate.get(ev.date);
      if (!bucket) continue;
      if (ev.client === 'codex') bucket.codex += ev.count;
      else bucket.claudeCode += ev.count;
    }
    return Array.from(byDate.entries()).map(([date, v]) => ({
      date,
      claudeCode: v.claudeCode,
      codex: v.codex,
      total: v.claudeCode + v.codex,
    }));
  }, [chartEvents, days]);

  const skillRows = useMemo<SkillRow[]>(() => {
    const bySkill = new Map<string, { claudeCode: number; codex: number; lastDate: string }>();
    for (const ev of filteredEvents) {
      const existing = bySkill.get(ev.skill);
      const isCodex = ev.client === 'codex';
      if (existing) {
        if (isCodex) existing.codex += ev.count;
        else existing.claudeCode += ev.count;
        if (ev.date > existing.lastDate) existing.lastDate = ev.date;
      } else {
        bySkill.set(ev.skill, {
          claudeCode: isCodex ? 0 : ev.count,
          codex: isCodex ? ev.count : 0,
          lastDate: ev.date,
        });
      }
    }
    return Array.from(bySkill.entries())
      .map(([name, v]) => ({
        name,
        count: v.claudeCode + v.codex,
        claudeCode: v.claudeCode,
        codex: v.codex,
        lastUsedIso: `${v.lastDate}T12:00:00Z`,
      }))
      .sort((a, b) => b.count - a.count);
  }, [filteredEvents]);

  const maxDayTotal = dayBuckets.reduce((max, b) => Math.max(max, b.total), 0);
  const maxSkillCount = skillRows.reduce((max, s) => Math.max(max, s.count), 0);
  const hasUsage = filteredEvents.length > 0;
  const hasChartUsage = chartEvents.length > 0;

  // Axis tick spacing: avoid overlapping labels for long windows.
  const tickEvery = days <= 7 ? 1 : days <= 30 ? 5 : 10;

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-lg font-semibold flex items-center gap-2">
          <BarChart3 className="h-4 w-4" />
          Skill Usage
        </h1>
        <p className="text-xs text-muted-foreground mt-1">
          {untrackedNames.length > 0
            ? `Usage is tracked from local Claude Code and Codex session history — other clients (${untrackedNames.join(', ')}) aren't tracked yet.`
            : 'Usage is tracked from local Claude Code and Codex session history.'}
        </p>
      </div>

      <div className="flex items-center gap-2 flex-wrap">
        <span className="text-xs text-muted-foreground">Period:</span>
        <div className="inline-flex gap-1">
          {PERIOD_OPTIONS.map((opt) => (
            <Button
              key={opt}
              size="sm"
              variant={days === opt ? 'default' : 'outline'}
              onClick={() => setDays(opt)}
            >
              {opt}d
            </Button>
          ))}
        </div>
        <span className="text-xs text-muted-foreground ml-2">Source:</span>
        <div className="inline-flex gap-1">
          {(
            [
              ['all', 'All'],
              ['claude_code', 'Claude Code'],
              ['codex', 'Codex'],
            ] as [ClientFilter, string][]
          ).map(([value, label]) => (
            <Button
              key={value}
              size="sm"
              variant={clientFilter === value ? 'default' : 'outline'}
              onClick={() => setClientFilter(value)}
            >
              {label}
            </Button>
          ))}
        </div>
        <Button
          size="sm"
          variant="outline"
          className="ml-auto"
          onClick={handleResync}
          disabled={resyncing}
          title="Re-sync usage from full transcript history"
        >
          <RotateCw className={cn('h-3.5 w-3.5', resyncing && 'animate-spin')} />
          Re-sync
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2 flex-wrap">
            {selectedSkill ? (
              <>
                <span>
                  Daily activity — <span className="font-mono">{selectedSkill}</span>
                </span>
                <button
                  type="button"
                  onClick={() => setSelectedSkill(null)}
                  className="inline-flex items-center gap-0.5 text-xs font-normal text-muted-foreground hover:text-foreground transition-colors"
                >
                  <X className="h-3 w-3" /> Clear
                </button>
              </>
            ) : (
              'Daily activity'
            )}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <p className="text-sm text-muted-foreground">Loading usage data...</p>
          ) : !hasChartUsage ? (
            <p className="text-sm text-muted-foreground text-center py-6">
              No usage
              {selectedSkill ? ` of ${selectedSkill}` : ''}
              {clientFilter !== 'all' ? ` from ${clientFilter === 'codex' ? 'Codex' : 'Claude Code'}` : ''} in
              the last {days} days.
            </p>
          ) : (
            <div className="space-y-1">
              <div className="flex items-end gap-px h-28">
                {dayBuckets.map((b) => {
                  const claudeHeight =
                    maxDayTotal > 0 ? (b.claudeCode / maxDayTotal) * 100 : 0;
                  const codexHeight = maxDayTotal > 0 ? (b.codex / maxDayTotal) * 100 : 0;
                  return (
                    <div
                      key={b.date}
                      className="flex-1 min-w-0 h-full flex flex-col justify-end"
                      title={`${b.date}: ${b.total} use${b.total === 1 ? '' : 's'} (${b.claudeCode} claude_code, ${b.codex} codex)`}
                    >
                      <div className="w-full flex flex-col justify-end h-full rounded-sm overflow-hidden bg-muted">
                        {b.total > 0 ? (
                          <>
                            <div
                              className="w-full bg-primary/60"
                              style={{ height: `${codexHeight}%` }}
                            />
                            <div
                              className="w-full bg-primary"
                              style={{ height: `${claudeHeight}%` }}
                            />
                          </>
                        ) : null}
                      </div>
                    </div>
                  );
                })}
              </div>
              <div className="flex gap-px">
                {dayBuckets.map((b, i) => {
                  const isEdge = i === 0 || i === dayBuckets.length - 1;
                  const show = isEdge || i % tickEvery === 0;
                  return (
                    <div key={b.date} className="flex-1 min-w-0 text-center">
                      {show ? (
                        <span className="text-[9px] text-muted-foreground whitespace-nowrap">
                          {shortLabel(b.date)}
                        </span>
                      ) : null}
                    </div>
                  );
                })}
              </div>
              <div className="flex items-center gap-3 pt-1 text-[10px] text-muted-foreground">
                <span className="flex items-center gap-1">
                  <span className="inline-block h-2 w-2 rounded-sm bg-primary" /> claude_code
                </span>
                <span className="flex items-center gap-1">
                  <span className="inline-block h-2 w-2 rounded-sm bg-primary/60" /> codex
                </span>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {hasUsage ? (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">By skill</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {skillRows.map((s) => {
                const pct = maxSkillCount > 0 ? (s.count / maxSkillCount) * 100 : 0;
                const claudePctOfBar = s.count > 0 ? (s.claudeCode / s.count) * 100 : 0;
                const codexPctOfBar = s.count > 0 ? (s.codex / s.count) * 100 : 0;
                const breakdown =
                  clientFilter !== 'all'
                    ? null
                    : s.claudeCode > 0 && s.codex > 0
                      ? `${s.claudeCode} Claude Code, ${s.codex} Codex`
                      : s.codex > 0
                        ? `${s.codex} Codex`
                        : `${s.claudeCode} Claude Code`;
                const isSelected = selectedSkill === s.name;
                return (
                  <div
                    key={s.name}
                    role="button"
                    tabIndex={0}
                    onClick={() => handleSelectSkill(s.name)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        handleSelectSkill(s.name);
                      }
                    }}
                    className={cn(
                      'space-y-1 rounded-md px-2 py-1.5 -mx-2 cursor-pointer transition-colors hover:bg-muted/60',
                      isSelected && 'bg-muted border border-primary/40',
                    )}
                  >
                    <div className="flex items-center justify-between text-sm">
                      <span className="font-medium truncate">{s.name}</span>
                      <span className="text-xs text-muted-foreground shrink-0 ml-2">
                        {s.count} {s.count === 1 ? 'use' : 'uses'}
                        {breakdown ? ` (${breakdown})` : ''} · last used{' '}
                        {relativeTime(s.lastUsedIso)}
                      </span>
                    </div>
                    <div
                      className="h-2 rounded bg-muted overflow-hidden flex"
                      style={{ width: `${pct}%` }}
                      title={`${s.claudeCode} Claude Code, ${s.codex} Codex`}
                    >
                      <div
                        className="h-full bg-primary"
                        style={{ width: `${claudePctOfBar}%` }}
                      />
                      <div
                        className="h-full bg-primary/60"
                        style={{ width: `${codexPctOfBar}%` }}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>
      ) : null}

      {selectedSkill ? (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">
              Usage details — <span className="font-mono">{selectedSkill}</span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            {detailEvents.length === 0 ? (
              <p className="text-sm text-muted-foreground text-center py-6">
                No usage events for this skill in the current filter.
              </p>
            ) : (
              <div className="space-y-1">
                {detailEvents.map((ev, i) => (
                  <div
                    key={`${ev.date}-${ev.client}-${ev.context}-${i}`}
                    className="flex items-center gap-3 text-sm py-1 border-b border-border/50 last:border-0"
                  >
                    <span className="text-xs text-muted-foreground shrink-0 w-24">{ev.date}</span>
                    <span className="flex items-center gap-1 shrink-0 w-28 text-xs">
                      <span
                        className={cn(
                          'inline-block h-2 w-2 rounded-sm',
                          ev.client === 'codex' ? 'bg-primary/60' : 'bg-primary',
                        )}
                      />
                      {clientLabel(ev.client)}
                    </span>
                    {ev.count > 1 ? (
                      <span
                        className="shrink-0 rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary"
                        title={`${ev.count} calls in this session`}
                      >
                        × {ev.count}
                      </span>
                    ) : null}
                    <span
                      className="truncate text-xs text-muted-foreground"
                      title={ev.context || 'Unknown folder'}
                    >
                      {ev.context ? shortPath(ev.context) : '—'}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}
