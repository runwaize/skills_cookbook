import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FileText, ExternalLink } from 'lucide-react';
import { commands, type LocalSkill } from '@/lib/tauri';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { statusBadgeClass } from '@/components/MetadataPanel';

export function SkillsList() {
  const navigate = useNavigate();
  const [skills, setSkills] = useState<LocalSkill[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = () => {
    setLoading(true);
    commands.listLocalSkills().then(setSkills).catch(() => setSkills([])).finally(() => setLoading(false));
  };

  useEffect(refresh, []);

  const openEditor = (name: string) => {
    navigate(`/chef/edit/${encodeURIComponent(name)}`);
  };

  const openExternal = async (name: string) => {
    try {
      await commands.openSkillInEditor(name);
    } catch (e) {
      console.error('Failed to open in editor:', e);
    }
  };

  if (loading) {
    return (
      <div className="space-y-3">
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-16 w-full" />
        ))}
      </div>
    );
  }

  if (skills.length === 0) {
    return (
      <div className="text-center py-12">
        <FileText className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <h2 className="text-lg font-medium">No skills yet</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Add skills from the Discover page or use <code>chef add</code> in the CLI.
        </p>
        <Button className="mt-4" onClick={() => navigate('/chef/discover')}>
          <Search className="h-4 w-4 mr-2" />
          Discover Skills
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {skills.map((skill) => (
        <Card key={skill.name} className="hover:border-primary/30 transition-colors">
          <CardContent className="flex items-center justify-between p-4">
            <div className="flex items-center gap-3">
              <FileText className="h-5 w-5 text-primary" />
              <div>
                <div className="flex items-center gap-2">
                  <p className="font-medium">{skill.name}</p>
                  <Badge
                    variant="outline"
                    className={`text-[10px] px-1.5 py-0 ${statusBadgeClass(skill.status)}`}
                  >
                    {skill.status}
                  </Badge>
                  <span className="text-[10px] text-muted-foreground">v{skill.version}</span>
                </div>
                <div className="flex items-center gap-1.5 mt-0.5">
                  {skill.description && (
                    <p className="text-xs text-muted-foreground">{skill.description}</p>
                  )}
                  {skill.tags.map((tag) => (
                    <Badge key={tag} variant="outline" className="text-[10px] px-1 py-0">
                      {tag}
                    </Badge>
                  ))}
                </div>
              </div>
              {!skill.has_skill_md && (
                <Badge variant="destructive" className="text-xs">
                  Missing SKILL.md
                </Badge>
              )}
            </div>
            <div className="flex items-center gap-2">
              <Button variant="ghost" size="sm" onClick={() => openEditor(skill.name)}>
                Edit
              </Button>
              <Button variant="ghost" size="sm" onClick={() => openExternal(skill.name)}>
                <ExternalLink className="h-4 w-4" />
              </Button>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function Search(props: React.SVGProps<SVGSVGElement>) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" {...props}>
      <circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" />
    </svg>
  );
}
