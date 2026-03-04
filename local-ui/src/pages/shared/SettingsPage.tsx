import { useNavigate } from 'react-router-dom';
import { Settings, FolderOpen, Globe } from 'lucide-react';
import { useRole } from '@/contexts/RoleContext';
import { commands } from '@/lib/tauri';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';

export function SettingsPage() {
  const { role, config, setRole } = useRole();
  const navigate = useNavigate();

  const switchToOnline = async () => {
    await commands.switchUiMode('online');
  };

  return (
    <div className="space-y-6 max-w-xl">
      {/* Role */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Role</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-sm">Current role</span>
            <Badge className="capitalize">{role}</Badge>
          </div>
          <Button variant="outline" size="sm" onClick={() => navigate('/select-role')}>
            Change Role
          </Button>
        </CardContent>
      </Card>

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
          <Button variant="outline" size="sm" onClick={switchToOnline}>
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
        <CardContent className="space-y-2 text-sm">
          <div className="flex justify-between">
            <span className="text-muted-foreground">Chef dir</span>
            <code className="text-xs">{config?.chef_dir ?? '...'}</code>
          </div>
          <Separator />
          <div className="flex justify-between">
            <span className="text-muted-foreground">Guest dir</span>
            <code className="text-xs">{config?.guest_dir ?? '...'}</code>
          </div>
          <Separator />
          <div className="flex justify-between">
            <span className="text-muted-foreground">Workspace ID</span>
            <code className="text-xs">{config?.workspace_id ?? 'not set'}</code>
          </div>
        </CardContent>
      </Card>

      {/* Ports */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Ports</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 text-sm">
          <div className="flex justify-between">
            <span className="text-muted-foreground">MCP Server</span>
            <code className="text-xs">{config?.mcp_server_port ?? '...'}</code>
          </div>
          <Separator />
          <div className="flex justify-between">
            <span className="text-muted-foreground">Bridge</span>
            <code className="text-xs">{config?.bridge_port ?? '...'}</code>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
