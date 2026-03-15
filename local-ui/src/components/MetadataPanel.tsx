import { useState, useEffect } from 'react';
import { Save, X } from 'lucide-react';
import { type SkillMeta, commands } from '@/lib/tauri';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';

const STATUS_OPTIONS = ['draft', 'review', 'published', 'archived'] as const;

const STATUS_COLORS: Record<string, string> = {
  draft: 'bg-gray-500/15 text-gray-700 border-gray-300',
  review: 'bg-yellow-500/15 text-yellow-700 border-yellow-300',
  published: 'bg-green-500/15 text-green-700 border-green-300',
  archived: 'bg-red-500/15 text-red-700 border-red-300',
};

export function statusBadgeClass(status: string): string {
  return STATUS_COLORS[status] || STATUS_COLORS.draft;
}

interface MetadataPanelProps {
  skillName: string;
  meta: SkillMeta | null;
  onMetaUpdated: (meta: SkillMeta) => void;
}

export function MetadataPanel({ skillName, meta, onMetaUpdated }: MetadataPanelProps) {
  const [status, setStatus] = useState(meta?.status ?? 'draft');
  const [version, setVersion] = useState(meta?.version ?? '0.1.0');
  const [tagInput, setTagInput] = useState('');
  const [tags, setTags] = useState<string[]>(meta?.tags ?? []);
  const [description, setDescription] = useState(meta?.description ?? '');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (meta) {
      setStatus(meta.status);
      setVersion(meta.version);
      setTags(meta.tags);
      setDescription(meta.description ?? '');
    }
  }, [meta]);

  const addTag = () => {
    const trimmed = tagInput.trim();
    if (trimmed && !tags.includes(trimmed)) {
      setTags([...tags, trimmed]);
    }
    setTagInput('');
  };

  const removeTag = (tag: string) => {
    setTags(tags.filter((t) => t !== tag));
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const updated = await commands.updateSkillMeta(
        skillName,
        status,
        version,
        tags,
        description || null
      );
      onMetaUpdated(updated);
    } catch (e) {
      console.error('Failed to save metadata:', e);
    } finally {
      setSaving(false);
    }
  };

  const dirty =
    meta &&
    (status !== meta.status ||
      version !== meta.version ||
      JSON.stringify(tags) !== JSON.stringify(meta.tags) ||
      (description || '') !== (meta.description || ''));

  return (
    <div className="flex items-center gap-3 px-3 py-2 border-b bg-muted/30 flex-wrap">
      {/* Status */}
      <div className="flex items-center gap-1.5">
        <span className="text-xs text-muted-foreground">Status</span>
        <select
          className={`text-xs rounded px-2 py-0.5 border cursor-pointer ${statusBadgeClass(status)}`}
          value={status}
          onChange={(e) => setStatus(e.target.value)}
        >
          {STATUS_OPTIONS.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </div>

      {/* Version */}
      <div className="flex items-center gap-1.5">
        <span className="text-xs text-muted-foreground">Version</span>
        <Input
          className="h-6 w-20 text-xs px-1.5"
          value={version}
          onChange={(e) => setVersion(e.target.value)}
          placeholder="0.1.0"
        />
      </div>

      {/* Tags */}
      <div className="flex items-center gap-1.5 flex-wrap">
        <span className="text-xs text-muted-foreground">Tags</span>
        {tags.map((tag) => (
          <Badge key={tag} variant="outline" className="text-xs py-0 px-1.5 gap-1">
            {tag}
            <button onClick={() => removeTag(tag)}>
              <X className="h-2.5 w-2.5" />
            </button>
          </Badge>
        ))}
        <Input
          className="h-6 w-24 text-xs px-1.5"
          value={tagInput}
          onChange={(e) => setTagInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ',') {
              e.preventDefault();
              addTag();
            }
          }}
          onBlur={addTag}
          placeholder="add tag..."
        />
      </div>

      {/* Save */}
      <Button
        variant="outline"
        size="sm"
        className="h-6 text-xs ml-auto"
        disabled={!dirty || saving}
        onClick={handleSave}
      >
        <Save className="h-3 w-3 mr-1" />
        {saving ? 'Saving...' : 'Save Meta'}
      </Button>
    </div>
  );
}
