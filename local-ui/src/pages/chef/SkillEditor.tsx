import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Save, ExternalLink, ArrowLeft } from 'lucide-react';
import { commands } from '@/lib/tauri';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Skeleton } from '@/components/ui/skeleton';

export function SkillEditor() {
  const { name } = useParams<{ name: string }>();
  const navigate = useNavigate();
  const [content, setContent] = useState('');
  const [original, setOriginal] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const decodedName = decodeURIComponent(name ?? '');

  useEffect(() => {
    if (!decodedName) return;
    commands
      .readSkillContent(decodedName)
      .then((c) => {
        setContent(c);
        setOriginal(c);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [decodedName]);

  const save = async () => {
    setSaving(true);
    try {
      await commands.writeSkillContent(decodedName, content);
      setOriginal(content);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const openExternal = () => commands.openSkillInEditor(decodedName);

  const dirty = content !== original;

  if (loading) return <Skeleton className="h-96 w-full" />;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={() => navigate('/chef/skills')}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <h2 className="text-lg font-medium">{decodedName}</h2>
          {dirty && <span className="text-xs text-warning">unsaved</span>}
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={openExternal}>
            <ExternalLink className="h-4 w-4 mr-1" />
            External Editor
          </Button>
          <Button size="sm" disabled={!dirty || saving} onClick={save}>
            <Save className="h-4 w-4 mr-1" />
            {saving ? 'Saving...' : 'Save'}
          </Button>
        </div>
      </div>

      {error && <p className="text-sm text-destructive">{error}</p>}

      <Textarea
        className="font-mono text-sm min-h-[60vh] resize-y"
        value={content}
        onChange={(e) => setContent(e.target.value)}
      />
    </div>
  );
}
