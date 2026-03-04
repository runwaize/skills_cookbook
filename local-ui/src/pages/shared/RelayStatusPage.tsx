import { useEffect, useState } from 'react';
import { Activity, RefreshCw } from 'lucide-react';
import { commands, type RelayStatus } from '@/lib/tauri';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';

export function RelayStatusPage() {
  const [status, setStatus] = useState<RelayStatus | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = () => {
    setLoading(true);
    commands
      .getRelayStatus()
      .then(setStatus)
      .catch(() => setStatus(null))
      .finally(() => setLoading(false));
  };

  useEffect(refresh, []);

  if (loading) {
    return (
      <div className="space-y-4">
        {[1, 2, 3, 4].map((i) => (
          <Skeleton key={i} className="h-16 w-full" />
        ))}
      </div>
    );
  }

  if (!status) {
    return <p className="text-destructive">Failed to load relay status.</p>;
  }

  const items = [
    { label: 'Authenticated', value: status.authenticated ? 'Yes' : 'No', ok: status.authenticated },
    { label: 'User', value: status.user_email ?? 'N/A', ok: !!status.user_email },
    { label: 'Workspace', value: status.workspace_id ?? 'N/A', ok: !!status.workspace_id },
    { label: 'MCP Server', value: status.mcp_server_running ? `Running (:${status.mcp_server_port})` : 'Stopped', ok: status.mcp_server_running },
    { label: 'Libraries', value: String(status.library_count), ok: true },
    { label: 'Cached Artifacts', value: String(status.artifact_count), ok: true },
    { label: 'Cache Size', value: formatBytes(status.cache_size_bytes), ok: true },
    { label: 'Guest Skills', value: String(status.guest_skill_count), ok: true },
    { label: 'Last Sync', value: status.last_sync ?? 'Never', ok: !!status.last_sync },
  ];

  return (
    <div className="space-y-4 max-w-xl">
      <div className="flex justify-end">
        <Button variant="ghost" size="sm" onClick={refresh}>
          <RefreshCw className="h-4 w-4 mr-1" />
          Refresh
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <Activity className="h-4 w-4" />
            Relay Status
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            {items.map((item) => (
              <div key={item.label} className="flex items-center justify-between py-1 border-b last:border-0">
                <span className="text-sm text-muted-foreground">{item.label}</span>
                <span className="text-sm font-medium">{item.value}</span>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      <div className="flex gap-2">
        <Button variant="outline" size="sm" onClick={() => commands.clearCache()}>
          Clear Cache
        </Button>
        <Button variant="outline" size="sm" onClick={() => commands.refreshSkills()}>
          Refresh Skills
        </Button>
      </div>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}
