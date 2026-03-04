import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { BookOpen, ArrowDownToLine, Activity } from 'lucide-react';
import { commands, type GuestManifest, type RelayStatus } from '@/lib/tauri';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';

export function CookDashboard() {
  const navigate = useNavigate();
  const [manifest, setManifest] = useState<GuestManifest | null>(null);
  const [status, setStatus] = useState<RelayStatus | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      commands.getGuestManifest().catch(() => null),
      commands.getRelayStatus().catch(() => null),
    ]).then(([m, s]) => {
      setManifest(m);
      setStatus(s);
      setLoading(false);
    });
  }, []);

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Available Skills</CardTitle>
            <BookOpen className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {loading ? '...' : manifest?.skills.length ?? 0}
            </div>
            <p className="text-xs text-muted-foreground">in manifest</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Auth Status</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {status?.authenticated ? (
                <span className="text-success">Connected</span>
              ) : (
                <span className="text-muted-foreground">Not logged in</span>
              )}
            </div>
            <p className="text-xs text-muted-foreground">{status?.user_email ?? ''}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">MCP Server</CardTitle>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {status?.mcp_server_running ? (
                <span className="text-success">Running</span>
              ) : (
                <span className="text-muted-foreground">Stopped</span>
              )}
            </div>
            <p className="text-xs text-muted-foreground">
              port {status?.mcp_server_port ?? '...'}
            </p>
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Card
          className="cursor-pointer hover:border-primary/50 transition-colors"
          onClick={() => navigate('/cook/manifest')}
        >
          <CardContent className="flex items-center gap-3 p-4">
            <BookOpen className="h-5 w-5 text-primary" />
            <div>
              <p className="font-medium">Available Skills</p>
              <p className="text-sm text-muted-foreground">Browse skill manifest</p>
            </div>
          </CardContent>
        </Card>

        <Card
          className="cursor-pointer hover:border-primary/50 transition-colors"
          onClick={() => navigate('/cook/sync')}
        >
          <CardContent className="flex items-center gap-3 p-4">
            <ArrowDownToLine className="h-5 w-5 text-primary" />
            <div>
              <p className="font-medium">Sync</p>
              <p className="text-sm text-muted-foreground">Update from server</p>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
