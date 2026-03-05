import { useState, useEffect } from 'react';
import { FolderOpen, Globe, Save } from 'lucide-react';
import { useRole } from '@/contexts/RoleContext';
import { commands, isTauri } from '@/lib/tauri';
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
