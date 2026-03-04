import { useEffect, useState } from 'react';
import { Search, Download, Monitor } from 'lucide-react';
import { commands, type DiscoveredClient, type ScannedSkillInfo } from '@/lib/tauri';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';

export function SearchDiscover() {
  const [agents, setAgents] = useState<DiscoveredClient[]>([]);
  const [skills, setSkills] = useState<ScannedSkillInfo[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [scanning, setScanning] = useState(false);
  const [importing, setImporting] = useState(false);
  const [customPath, setCustomPath] = useState('');
  const [loadingAgents, setLoadingAgents] = useState(true);

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
      await commands.importSkills(paths);
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

  return (
    <div className="space-y-6">
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
      <Card>
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
