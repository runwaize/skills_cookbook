import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { commands, type AppConfig } from '@/lib/tauri';

interface RoleContextValue {
  config: AppConfig | null;
  loading: boolean;
  refreshConfig: () => Promise<void>;
}

const RoleContext = createContext<RoleContextValue | null>(null);

export function RoleProvider({ children }: { children: ReactNode }) {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);

  const refreshConfig = async () => {
    try {
      const cfg = await commands.getConfig();
      setConfig(cfg);
    } catch {
      // Config not available yet
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refreshConfig();
  }, []);

  return (
    <RoleContext value={{ config, loading, refreshConfig }}>
      {children}
    </RoleContext>
  );
}

export function useRole() {
  const ctx = useContext(RoleContext);
  if (!ctx) throw new Error('useRole must be used within RoleProvider');
  return ctx;
}
