import { useEffect, useState } from 'react';
import { BookOpen } from 'lucide-react';
import { commands, type GuestManifest } from '@/lib/tauri';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';

export function ManifestView() {
  const [manifest, setManifest] = useState<GuestManifest | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    commands
      .getGuestManifest()
      .then(setManifest)
      .catch(() => setManifest({ skills: [] }))
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="space-y-3">
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-16 w-full" />
        ))}
      </div>
    );
  }

  const skills = manifest?.skills ?? [];

  if (skills.length === 0) {
    return (
      <div className="text-center py-12">
        <BookOpen className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <h2 className="text-lg font-medium">No skills in manifest</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Sync from the server to populate your manifest.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {skills.map((skill) => (
        <Card key={skill.skill_id}>
          <CardContent className="flex items-center justify-between p-4">
            <div>
              <p className="font-medium">{skill.name}</p>
              {skill.description && (
                <p className="text-sm text-muted-foreground">{skill.description}</p>
              )}
            </div>
            <div className="flex items-center gap-2">
              {skill.library_id && (
                <Badge variant="outline" className="text-[10px]">
                  {skill.library_id.slice(0, 8)}
                </Badge>
              )}
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
