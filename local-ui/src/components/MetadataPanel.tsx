import { useState, useEffect } from 'react';
import { Save, X } from 'lucide-react';
import { type SkillMeta, commands } from '@/lib/tauri';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';

const STATUS_OPTIONS = ['draft', 'review', 'published', 'archived'] as const;

const STATUS_COLORS: Record<string, string> = {
  draft: 'bg-gray-100 text-gray-800 border-gray-300 dark:bg-gray-800 dark:text-gray-200 dark:border-gray-600',
  review: 'bg-yellow-100 text-yellow-800 border-yellow-300 dark:bg-yellow-900 dark:text-yellow-200 dark:border-yellow-700',
  published: 'bg-green-100 text-green-800 border-green-300 dark:bg-green-900 dark:text-green-200 dark:border-green-700',
  archived: 'bg-red-100 text-red-800 border-red-300 dark:bg-red-900 dark:text-red-200 dark:border-red-700',
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
  const [source, setSource] = useState(meta?.source ?? '');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (meta) {
      setStatus(meta.status);
      setVersion(meta.version);
      setTags(meta.tags);
      setDescription(meta.description ?? '');
      setSource(meta.source);
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
        description || null,
        source
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
      (description || '') !== (meta.description || '') ||
      source !== meta.source);

  return (
    <div className="flex flex-col gap-2.5 p-3 rounded-md border bg-muted/30">
      <div className="flex items-center gap-3 flex-wrap">
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

      {/* Source + Save */}
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5">
          <span className="text-xs text-muted-foreground">Source</span>
          <Input
            className="h-6 w-28 text-xs px-1.5"
            value={source}
            onChange={(e) => setSource(e.target.value)}
            placeholder="source..."
            list="skill-source-options"
          />
          <datalist id="skill-source-options">
            <option value="created" />
            <option value="downloaded" />
            <option value="adapted" />
            <option value="imported" />
            <option value="existing" />
          </datalist>
        </div>
        <Button
          variant="outline"
          size="sm"
          className="h-6 text-xs"
          disabled={!dirty || saving}
          onClick={handleSave}
        >
          <Save className="h-3 w-3 mr-1" />
          {saving ? 'Saving...' : 'Save Meta'}
        </Button>
      </div>
    </div>
  );
}
