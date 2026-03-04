import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { commands, type UserRole, type AppConfig } from '@/lib/tauri';

interface RoleContextValue {
  role: UserRole;
  config: AppConfig | null;
  loading: boolean;
  setRole: (role: UserRole) => Promise<void>;
  refreshConfig: () => Promise<void>;
}

const RoleContext = createContext<RoleContextValue | null>(null);

export function RoleProvider({ children }: { children: ReactNode }) {
  const [role, setRoleState] = useState<UserRole>('cook');
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);

  const refreshConfig = async () => {
    try {
      const cfg = await commands.getConfig();
      setConfig(cfg);
      setRoleState(cfg.user_role);
    } catch {
      // Config not available yet (first run)
    } finally {
      setLoading(false);
    }
  };

  const setRole = async (newRole: UserRole) => {
    // Update local state immediately so navigation works
    setRoleState(newRole);
    try {
      await commands.setUserRole(newRole);
      await refreshConfig();
    } catch (err) {
      console.warn('Failed to persist role to backend:', err);
    }
  };

  useEffect(() => {
    refreshConfig();
  }, []);

  // Listen for tray menu role changes
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen<string>('role-changed', (event) => {
        const newRole = event.payload as UserRole;
        setRoleState(newRole);
        refreshConfig();
      }).then((fn) => {
        unlisten = fn;
      });
    });
    return () => unlisten?.();
  }, []);

  return (
    <RoleContext value={{ role, config, loading, setRole, refreshConfig }}>
      {children}
    </RoleContext>
  );
}

export function useRole() {
  const ctx = useContext(RoleContext);
  if (!ctx) throw new Error('useRole must be used within RoleProvider');
  return ctx;
}
