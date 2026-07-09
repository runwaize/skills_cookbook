import { Routes, Route, Navigate } from 'react-router-dom';
import { useRole } from '@/contexts/RoleContext';
import { AppLayout } from '@/components/AppLayout';
import { ChefDashboard } from '@/pages/chef/ChefDashboard';
import { SkillsList } from '@/pages/chef/SkillsList';
import { RemoteSkillsList } from '@/pages/chef/RemoteSkillsList';
import { SkillEditor } from '@/pages/chef/SkillEditor';
import { SearchDiscover } from '@/pages/chef/SearchDiscover';
import { SyncPage } from '@/pages/chef/SyncPage';
import { CookDashboard } from '@/pages/cook/CookDashboard';
import { ManifestView } from '@/pages/cook/ManifestView';
import { ProjectSkills } from '@/pages/cook/ProjectSkills';
import { CookSyncView } from '@/pages/cook/CookSyncView';
import { RelayStatusPage } from '@/pages/shared/RelayStatusPage';
import { DoctorPage } from '@/pages/shared/DoctorPage';
import { SettingsPage } from '@/pages/shared/SettingsPage';
import { DashboardPage } from '@/pages/shared/DashboardPage';

export function App() {
  const { loading } = useRole();

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-background">
        <div className="loading loading-spinner loading-lg text-primary" />
      </div>
    );
  }

  return (
    <Routes>
      <Route element={<AppLayout />}>
        {/* Dashboard */}
        <Route path="/" element={<Navigate to="/chef/skills" replace />} />
        {/* Chef routes */}
        <Route path="/chef" element={<ChefDashboard />} />
        <Route path="/chef/skills" element={<SkillsList />} />
        <Route path="/chef/remote" element={<RemoteSkillsList />} />
        <Route path="/chef/edit/:name" element={<SkillEditor />} />
        <Route path="/chef/discover" element={<SearchDiscover />} />
        <Route path="/chef/sync" element={<SyncPage />} />
        <Route path="/chef/usage" element={<DashboardPage />} />
        {/* Cook routes */}
        <Route path="/cook" element={<CookDashboard />} />
        <Route path="/cook/skills" element={<ManifestView />} />
        <Route path="/cook/manifest" element={<ManifestView />} />
        <Route path="/cook/project-skills" element={<ProjectSkills />} />
        <Route path="/cook/sync" element={<CookSyncView />} />
        {/* Shared routes */}
        <Route path="/status" element={<RelayStatusPage />} />
        <Route path="/doctor" element={<DoctorPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
