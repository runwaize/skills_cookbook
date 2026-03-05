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

const chefNav = [
  { label: 'Local Skills', icon: FileText, path: '/chef/skills' },
  { label: 'Remote Skills', icon: Cloud, path: '/chef/remote' },
  { label: 'Discover', icon: Search, path: '/chef/discover' },
  { label: 'Sync', icon: RefreshCw, path: '/chef/sync' },
];

const cookNav = [
  { label: 'Local Skills', icon: BookOpen, path: '/cook/skills' },
  { label: 'Sync', icon: ArrowDownToLine, path: '/cook/sync' },
];

const systemNav = [
  { label: 'Relay Status', icon: Activity, path: '/status' },
  { label: 'Doctor', icon: Stethoscope, path: '/doctor' },
  { label: 'Settings', icon: Settings, path: '/settings' },
];

export function AppSidebar() {
  const location = useLocation();
  const navigate = useNavigate();

  return (
    <Sidebar>
      <SidebarHeader className="p-4">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <ChefHat className="h-4 w-4" />
          </div>
          <div>
            <p className="text-sm font-semibold">Skill Cookbook</p>
          </div>
        </div>
      </SidebarHeader>

      <SidebarContent>
        {/* Dashboard — top level, outside groups */}
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton
                  isActive={location.pathname === '/'}
                  onClick={() => navigate('/')}
                >
                  <LayoutDashboard className="h-4 w-4" />
                  <span>Dashboard</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>
            <ChefHat className="h-3.5 w-3.5 mr-1" />
            Chef
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {chefNav.map((item) => (
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
          <SidebarGroupLabel>
            <UtensilsCrossed className="h-3.5 w-3.5 mr-1" />
            Cook
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {cookNav.map((item) => (
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
              {systemNav.map((item) => (
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
        <p className="text-[10px] text-muted-foreground">v0.1.0</p>
      </SidebarFooter>
    </Sidebar>
  );
}
