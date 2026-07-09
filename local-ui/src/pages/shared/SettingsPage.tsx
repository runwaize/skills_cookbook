import { useState, useEffect } from 'react';
import { FolderOpen, Globe, Save, Monitor, ShieldCheck } from 'lucide-react';
import { useRole } from '@/contexts/RoleContext';
import { commands, isTauri, type DiscoveredClient } from '@/lib/tauri';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

function PathInput({
  id,
  label,
  value,
  onChange,
  dialogTitle,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  dialogTitle: string;
}) {
  const browse = async () => {
    try {
      const picked = await commands.pickFolder(dialogTitle);
      if (picked) onChange(picked);
    } catch {
      // File picker not available (browser dev mode) — user can type manually
    }
  };

  return (
    <div className="space-y-1.5">
      <Label htmlFor={id} className="text-sm text-muted-foreground">{label}</Label>
      <div className="flex gap-2">
        <Input id={id} value={value} onChange={e => onChange(e.target.value)} className="flex-1" />
        <Button type="button" variant="outline" size="icon" onClick={browse} title="Browse..." disabled={!isTauri}>
          <FolderOpen className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}

function ManagedClientsCard() {
  const { config, refreshConfig } = useRole();
  const [clients, setClients] = useState<DiscoveredClient[]>([]);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState<string | null>(null);

  useEffect(() => {
    commands
      .discoverAgents()
      .then(setClients)
      .catch(() => setClients([]))
      .finally(() => setLoading(false));
  }, []);

  const managedIds = config?.managed_client_ids ?? [];

  const toggle = async (clientId: string) => {
    const nextIds = managedIds.includes(clientId)
      ? managedIds.filter((id) => id !== clientId)
      : [...managedIds, clientId];
    setPending(clientId);
    try {
      await commands.setManagedClients(nextIds);
      await refreshConfig();
    } catch (e) {
      console.error('Failed to update managed clients:', e);
    } finally {
      setPending(null);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base flex items-center gap-2">
          <Monitor className="h-4 w-4" />
          Manage LLMs / IDEs
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        <p className="text-xs text-muted-foreground -mt-1 mb-2">
          Choose which clients you want managed here. Unmanaged clients won't appear as deploy or adopt targets,
          even if detected on this machine.
        </p>
        {loading ? (
          <p className="text-sm text-muted-foreground">Loading clients...</p>
        ) : clients.length === 0 ? (
          <p className="text-sm text-muted-foreground">No clients found.</p>
        ) : (
          <div className="space-y-1.5">
            {clients.map((client) => (
              <label
                key={client.id}
                className="flex items-center justify-between gap-3 py-1 text-sm select-none"
              >
                <span className="flex items-center gap-2">
                  {client.name}
                  <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                    {client.detected ? 'detected' : 'not installed'}
                  </Badge>
                </span>
                <input
                  type="checkbox"
                  className="h-3.5 w-3.5 rounded border-input accent-primary"
                  checked={managedIds.includes(client.id)}
                  disabled={pending === client.id}
                  onChange={() => toggle(client.id)}
                />
              </label>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function DefaultReviewCliCard() {
  const { config, refreshConfig } = useRole();
  const [saving, setSaving] = useState(false);

  const value = config?.default_review_cli ?? '';

  const onChange = async (next: string) => {
    setSaving(true);
    try {
      await commands.setDefaultReviewCli(next === '' ? null : next);
      await refreshConfig();
    } catch (e) {
      console.error('Failed to update default review CLI:', e);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base flex items-center gap-2">
          <ShieldCheck className="h-4 w-4" />
          Default Security Review CLI
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        <p className="text-xs text-muted-foreground -mt-1 mb-2">
          Used to run an AI security review on skills imported from GitHub before installing them.
        </p>
        <select
          className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm"
          value={value}
          disabled={saving}
          onChange={(e) => onChange(e.target.value)}
        >
          <option value="">None selected</option>
          <option value="claude_code">Claude Code</option>
          <option value="codex">Codex</option>
        </select>
      </CardContent>
    </Card>
  );
}

export function SettingsPage() {
  const { config, refreshConfig } = useRole();

  const [settingsDir, setSettingsDir] = useState('');
  const [originalSettingsDir, setOriginalSettingsDir] = useState('');
  const [chefDir, setChefDir] = useState('');
  const [cookDir, setCookDir] = useState('');
  const [mcpPort, setMcpPort] = useState(9876);
  const [bridgePort, setBridgePort] = useState(9123);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (config) {
      setSettingsDir(config.settings_dir);
      setOriginalSettingsDir(config.settings_dir);
      setChefDir(config.chef_dir);
      setCookDir(config.cook_dir);
      setMcpPort(config.mcp_server_port);
      setBridgePort(config.bridge_port);
    }
  }, [config]);

  const handleSave = async () => {
    setSaving(true);
    setSaved(false);
    setError('');
    try {
      // If settings dir changed, move it first (copies files + writes redirect)
      if (settingsDir !== originalSettingsDir) {
        await commands.moveSettingsDir(settingsDir);
      }

      await commands.updateConfig({
        chefDir,
        cookDir,
        mcpServerPort: mcpPort,
        bridgePort,
      });
      await refreshConfig();
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  const switchToOnline = async () => {
    const proceed = window.confirm(
      'This replaces this view with the online Supervaize web app (requires an account) — it will ' +
      'navigate away from this local UI.\n\nTo come back, use the Skill Cookbook icon in your system ' +
      'tray / menu bar and choose "Local UI".\n\nContinue?'
    );
    if (!proceed) return;
    await commands.switchUiMode('online');
  };

  return (
    <div className="space-y-6 max-w-xl">
      {/* UI Mode */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">UI Mode</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-sm">Current mode</span>
            <Badge variant="outline">Local</Badge>
          </div>
          <Button variant="outline" size="sm" onClick={switchToOnline} disabled={!isTauri}>
            <Globe className="h-4 w-4 mr-1" />
            Switch to Online UI
          </Button>
        </CardContent>
      </Card>

      {/* Manage LLMs / IDEs */}
      <ManagedClientsCard />

      {/* Default Security Review CLI */}
      <DefaultReviewCliCard />

      {/* Directories */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <FolderOpen className="h-4 w-4" />
            Directories
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <PathInput
            id="settings-dir"
            label="Settings folder"
            value={settingsDir}
            onChange={setSettingsDir}
            dialogTitle="Select Settings folder"
          />
          <p className="text-xs text-muted-foreground -mt-2">
            Config, device ID, and webview data. Moving copies existing files.
          </p>
          <PathInput
            id="chef-dir"
            label="Chef directory"
            value={chefDir}
            onChange={setChefDir}
            dialogTitle="Select Chef directory"
          />
          <PathInput
            id="cook-dir"
            label="Cook directory"
            value={cookDir}
            onChange={setCookDir}
            dialogTitle="Select Cook directory"
          />
        </CardContent>
      </Card>

      {/* Ports */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Ports</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="mcp-port" className="text-sm text-muted-foreground">MCP Server</Label>
            <Input id="mcp-port" type="number" value={mcpPort} onChange={e => setMcpPort(Number(e.target.value))} />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="bridge-port" className="text-sm text-muted-foreground">Bridge</Label>
            <Input id="bridge-port" type="number" value={bridgePort} onChange={e => setBridgePort(Number(e.target.value))} />
          </div>
        </CardContent>
      </Card>

      {/* Save */}
      <div className="flex items-center gap-3">
        <Button onClick={handleSave} disabled={saving}>
          <Save className="h-4 w-4 mr-1" />
          {saving ? 'Saving...' : 'Save Settings'}
        </Button>
        {saved && <span className="text-sm text-green-600">Saved!</span>}
        {error && <span className="text-sm text-red-600">{error}</span>}
      </div>
    </div>
  );
}
