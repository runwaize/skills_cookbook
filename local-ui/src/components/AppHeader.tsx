import { useLocation } from 'react-router-dom';
import { Globe, Monitor } from 'lucide-react';
import { useRole } from '@/contexts/RoleContext';
import { commands } from '@/lib/tauri';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { SidebarTrigger } from '@/components/ui/sidebar';

const pageTitles: Record<string, string> = {
  '/chef': 'Dashboard',
  '/chef/skills': 'My Skills',
  '/chef/remote': 'Remote Skills',
  '/chef/discover': 'Discover Skills',
  '/chef/sync': 'Sync',
  '/cook': 'Dashboard',
  '/cook/manifest': 'Available Skills',
  '/cook/sync': 'Sync',
  '/status': 'Relay Status',
  '/doctor': 'Doctor',
  '/settings': 'Settings',
};

export function AppHeader() {
  const { role } = useRole();
  const location = useLocation();

  const title = pageTitles[location.pathname] ?? 'Skill Cookbook';

  const switchToOnline = async () => {
    await commands.switchUiMode('online');
  };

  return (
    <header className="flex h-14 shrink-0 items-center gap-2 border-b px-4">
      <SidebarTrigger className="-ml-1" />
      <Separator orientation="vertical" className="mr-2 h-4" />
      <h1 className="text-sm font-medium flex-1">{title}</h1>
      <Badge variant="outline" className="capitalize text-xs">
        {role}
      </Badge>
      <Button variant="ghost" size="sm" onClick={switchToOnline} title="Switch to Online UI">
        <Globe className="h-4 w-4 mr-1" />
        <span className="text-xs">Online</span>
      </Button>
    </header>
  );
}
