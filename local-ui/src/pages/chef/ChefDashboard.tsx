import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FileText, Cloud, Search, RefreshCw } from 'lucide-react';
import { commands, type LocalSkill, type RelayStatus } from '@/lib/tauri';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';

export function ChefDashboard() {
  const navigate = useNavigate();
  const [skills, setSkills] = useState<LocalSkill[]>([]);
  const [status, setStatus] = useState<RelayStatus | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      commands.listLocalSkills().catch(() => []),
      commands.getRelayStatus().catch(() => null),
    ]).then(([s, st]) => {
      setSkills(s);
      setStatus(st);
      setLoading(false);
    });
  }, []);

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Local Skills</CardTitle>
            <FileText className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{loading ? '...' : skills.length}</div>
            <p className="text-xs text-muted-foreground">in chef directory</p>
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
        <Card className="cursor-pointer hover:border-primary/50 transition-colors" onClick={() => navigate('/chef/skills')}>
          <CardContent className="flex items-center gap-3 p-4">
            <FileText className="h-5 w-5 text-primary" />
            <div>
              <p className="font-medium">My Skills</p>
              <p className="text-sm text-muted-foreground">View and edit local skills</p>
            </div>
          </CardContent>
        </Card>

        <Card className="cursor-pointer hover:border-primary/50 transition-colors" onClick={() => navigate('/chef/discover')}>
          <CardContent className="flex items-center gap-3 p-4">
            <Search className="h-5 w-5 text-primary" />
            <div>
              <p className="font-medium">Discover Skills</p>
              <p className="text-sm text-muted-foreground">Find skills from AI agents</p>
            </div>
          </CardContent>
        </Card>

        <Card className="cursor-pointer hover:border-primary/50 transition-colors" onClick={() => navigate('/chef/remote')}>
          <CardContent className="flex items-center gap-3 p-4">
            <Cloud className="h-5 w-5 text-primary" />
            <div>
              <p className="font-medium">Remote Skills</p>
              <p className="text-sm text-muted-foreground">Browse server libraries</p>
            </div>
          </CardContent>
        </Card>

        <Card className="cursor-pointer hover:border-primary/50 transition-colors" onClick={() => navigate('/chef/sync')}>
          <CardContent className="flex items-center gap-3 p-4">
            <RefreshCw className="h-5 w-5 text-primary" />
            <div>
              <p className="font-medium">Sync</p>
              <p className="text-sm text-muted-foreground">Commit and push changes</p>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
