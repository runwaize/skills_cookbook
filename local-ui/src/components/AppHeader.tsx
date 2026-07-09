import { useLocation } from 'react-router-dom';
import { Separator } from '@/components/ui/separator';
import { SidebarTrigger } from '@/components/ui/sidebar';

const pageTitles: Record<string, string> = {
  '/': 'Dashboard',
  '/chef': 'Chef Overview',
  '/chef/skills': 'Chef Local Skills',
  '/chef/remote': 'Remote Skills',
  '/chef/discover': 'Discover Skills',
  '/chef/sync': 'Chef Sync',
  '/cook': 'Cook Overview',
  '/cook/skills': 'Cook Local Skills',
  '/cook/manifest': 'Cook Local Skills',
  '/cook/sync': 'Cook Sync',
  '/status': 'Relay Status',
  '/doctor': 'Doctor',
  '/settings': 'Settings',
};

export function AppHeader() {
  const location = useLocation();

  const title = pageTitles[location.pathname] ?? 'Skill Cookbook';

  return (
    <header className="flex h-14 shrink-0 items-center gap-2 border-b px-4">
      <SidebarTrigger className="-ml-1" />
      <Separator orientation="vertical" className="mr-2 h-4" />
      <h1 className="text-sm font-medium flex-1">{title}</h1>
    </header>
  );
}
