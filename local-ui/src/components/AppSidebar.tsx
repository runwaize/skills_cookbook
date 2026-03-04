import { useLocation, useNavigate } from 'react-router-dom';
import {
  ChefHat,
  UtensilsCrossed,
  LayoutDashboard,
  FileText,
  Cloud,
  Search,
  RefreshCw,
  Activity,
  Stethoscope,
  Settings,
  BookOpen,
  ArrowDownToLine,
} from 'lucide-react';
import { useRole } from '@/contexts/RoleContext';
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarFooter,
  SidebarHeader,
} from '@/components/ui/sidebar';
import { Badge } from '@/components/ui/badge';

const chefNav = [
  { label: 'Dashboard', icon: LayoutDashboard, path: '/chef' },
  { label: 'My Skills', icon: FileText, path: '/chef/skills' },
  { label: 'Remote Skills', icon: Cloud, path: '/chef/remote' },
  { label: 'Discover', icon: Search, path: '/chef/discover' },
  { label: 'Sync', icon: RefreshCw, path: '/chef/sync' },
];

const cookNav = [
  { label: 'Dashboard', icon: LayoutDashboard, path: '/cook' },
  { label: 'Available Skills', icon: BookOpen, path: '/cook/manifest' },
  { label: 'Sync', icon: ArrowDownToLine, path: '/cook/sync' },
];

const sharedNav = [
  { label: 'Relay Status', icon: Activity, path: '/status' },
  { label: 'Doctor', icon: Stethoscope, path: '/doctor' },
  { label: 'Settings', icon: Settings, path: '/settings' },
];

export function AppSidebar() {
  const { role } = useRole();
  const location = useLocation();
  const navigate = useNavigate();

  const roleNav = role === 'chef' ? chefNav : cookNav;
  const RoleIcon = role === 'chef' ? ChefHat : UtensilsCrossed;

  return (
    <Sidebar>
      <SidebarHeader className="p-4">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <RoleIcon className="h-4 w-4" />
          </div>
          <div>
            <p className="text-sm font-semibold">Skill Cookbook</p>
            <p className="text-xs text-muted-foreground capitalize">{role} Mode</p>
          </div>
        </div>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>{role === 'chef' ? 'Chef' : 'Cook'}</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {roleNav.map((item) => (
                <SidebarMenuItem key={item.path}>
                  <SidebarMenuButton
                    isActive={location.pathname === item.path}
                    onClick={() => navigate(item.path)}
                  >
                    <item.icon className="h-4 w-4" />
                    <span>{item.label}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>System</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {sharedNav.map((item) => (
                <SidebarMenuItem key={item.path}>
                  <SidebarMenuButton
                    isActive={location.pathname === item.path}
                    onClick={() => navigate(item.path)}
                  >
                    <item.icon className="h-4 w-4" />
                    <span>{item.label}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter className="p-4">
        <div className="flex items-center justify-between">
          <Badge variant="outline" className="capitalize">
            {role}
          </Badge>
          <button
            className="text-xs text-muted-foreground hover:text-foreground transition-colors"
            onClick={() => navigate('/select-role')}
          >
            Switch Role
          </button>
        </div>
        <p className="text-[10px] text-muted-foreground mt-1">v0.1.0</p>
      </SidebarFooter>
    </Sidebar>
  );
}
