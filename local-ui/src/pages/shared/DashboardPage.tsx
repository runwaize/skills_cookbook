import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FileText, Cloud, BookOpen, Activity } from 'lucide-react';
import { commands, isTauri, type LocalSkill, type GuestManifest, type RelayStatus } from '@/lib/tauri';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';

export function DashboardPage() {
  const navigate = useNavigate();
  const [skills, setSkills] = useState<LocalSkill[]>([]);
  const [manifest, setManifest] = useState<GuestManifest | null>(null);
  const [status, setStatus] = useState<RelayStatus | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!isTauri) {
      setLoading(false);
      return;
    }
    Promise.all([
      commands.listLocalSkills().catch(() => []),
      commands.getGuestManifest().catch(() => null),
      commands.getRelayStatus().catch(() => null),
    ]).then(([s, m, st]) => {
      setSkills(s);
      setManifest(m);
      setStatus(st);
      setLoading(false);
    });
  }, []);

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Chef Skills</CardTitle>
            <FileText className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{loading ? '...' : skills.length}</div>
            <p className="text-xs text-muted-foreground">in chef directory</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Cook Skills</CardTitle>
            <BookOpen className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{loading ? '...' : manifest?.skills.length ?? 0}</div>
            <p className="text-xs text-muted-foreground">in manifest</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Libraries</CardTitle>
            <Cloud className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{status?.library_count ?? '...'}</div>
            <p className="text-xs text-muted-foreground">on server</p>
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
                <span className="text-muted-foreground">{loading ? '...' : 'Stopped'}</span>
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
          onClick={() => navigate('/chef/skills')}
        >
          <CardContent className="flex items-center gap-3 p-4">
            <FileText className="h-5 w-5 text-primary" />
            <div>
              <p className="font-medium">Chef Local Skills</p>
              <p className="text-sm text-muted-foreground">View and edit your skills</p>
            </div>
          </CardContent>
        </Card>

        <Card
          className="cursor-pointer hover:border-primary/50 transition-colors"
          onClick={() => navigate('/cook/skills')}
        >
          <CardContent className="flex items-center gap-3 p-4">
            <BookOpen className="h-5 w-5 text-primary" />
            <div>
              <p className="font-medium">Cook Local Skills</p>
              <p className="text-sm text-muted-foreground">Browse available skills manifest</p>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
