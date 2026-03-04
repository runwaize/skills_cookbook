import { useEffect, useState } from 'react';
import { Cloud } from 'lucide-react';
import { commands, type RemoteSkillInfo } from '@/lib/tauri';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';

export function RemoteSkillsList() {
  const [skills, setSkills] = useState<RemoteSkillInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    commands
      .listRemoteSkills()
      .then(setSkills)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="space-y-3">
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-20 w-full" />
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="text-center py-12">
        <p className="text-destructive">{error}</p>
        <p className="text-sm text-muted-foreground mt-1">Make sure you are logged in.</p>
      </div>
    );
  }

  // Group by library
  const grouped = skills.reduce(
    (acc, s) => {
      const key = `${s.library_name} (${s.library_id})`;
      if (!acc[key]) acc[key] = [];
      acc[key].push(s);
      return acc;
    },
    {} as Record<string, RemoteSkillInfo[]>,
  );

  if (Object.keys(grouped).length === 0) {
    return (
      <div className="text-center py-12">
        <Cloud className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <h2 className="text-lg font-medium">No remote skills</h2>
        <p className="text-sm text-muted-foreground mt-1">No libraries found on the server.</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {Object.entries(grouped).map(([libLabel, libSkills]) => (
        <Card key={libLabel}>
          <CardHeader>
            <CardTitle className="text-base">{libLabel}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {libSkills.map((s) => (
                <div key={s.skill_id} className="flex items-center justify-between py-2 border-b last:border-0">
                  <div>
                    <p className="text-sm font-medium">{s.skill_name}</p>
                    {s.description && (
                      <p className="text-xs text-muted-foreground">{s.description}</p>
                    )}
                  </div>
                  <Badge variant="outline" className="text-[10px]">
                    {s.skill_id.slice(0, 8)}
                  </Badge>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
