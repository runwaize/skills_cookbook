import { useEffect, useState, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Save, ExternalLink, ArrowLeft } from 'lucide-react';
import { commands, type SkillFileEntry, type SkillMeta, isTextFile } from '@/lib/tauri';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Skeleton } from '@/components/ui/skeleton';
import { FileTree } from '@/components/FileTree';
import { MetadataPanel } from '@/components/MetadataPanel';

export function SkillEditor() {
  const { name } = useParams<{ name: string }>();
  const navigate = useNavigate();
  const decodedName = decodeURIComponent(name ?? '');

  const [files, setFiles] = useState<SkillFileEntry[]>([]);
  const [meta, setMeta] = useState<SkillMeta | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [content, setContent] = useState('');
  const [original, setOriginal] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshFiles = useCallback(async () => {
    try {
      const entries = await commands.listSkillFiles(decodedName);
      setFiles(entries);
    } catch (e) {
      setError(String(e));
    }
  }, [decodedName]);

  // Load file tree and metadata on mount
  useEffect(() => {
    if (!decodedName) return;
    Promise.all([
      commands.listSkillFiles(decodedName),
      commands.getSkillMeta(decodedName),
    ])
      .then(([entries, skillMeta]) => {
        setFiles(entries);
        setMeta(skillMeta);
        // Auto-select SKILL.md if it exists
        const skillMd = entries.find((e) => e.relative_path === 'SKILL.md');
        if (skillMd) {
          setSelectedPath('SKILL.md');
        }
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [decodedName]);

  // Load file content when selection changes
  useEffect(() => {
    if (!selectedPath || !decodedName) return;
    const entry = files.find((f) => f.relative_path === selectedPath);
    if (!entry || entry.is_dir || !isTextFile(entry.extension)) return;

    commands
      .readSkillFile(decodedName, selectedPath)
      .then((c) => {
        setContent(c);
        setOriginal(c);
      })
      .catch((e) => setError(String(e)));
  }, [selectedPath, decodedName, files]);

  const save = async () => {
    if (!selectedPath) return;
    setSaving(true);
    try {
      await commands.writeSkillFile(decodedName, selectedPath, content);
      setOriginal(content);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const openExternal = () => commands.openSkillInEditor(decodedName);

  const handleNewFile = async (parentDir: string) => {
    const fileName = window.prompt('File name:', 'new-file.md');
    if (!fileName) return;
    const rel = parentDir ? `${parentDir}/${fileName}` : fileName;
    try {
      await commands.createSkillFile(decodedName, rel, '');
      await refreshFiles();
      setSelectedPath(rel);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleNewFolder = async (parentDir: string) => {
    const folderName = window.prompt('Folder name:');
    if (!folderName) return;
    const rel = parentDir ? `${parentDir}/${folderName}` : folderName;
    try {
      await commands.createSkillFolder(decodedName, rel);
      await refreshFiles();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDelete = async (path: string) => {
    if (!window.confirm(`Delete "${path}"?`)) return;
    try {
      await commands.deleteSkillFile(decodedName, path);
      if (selectedPath === path) {
        setSelectedPath(null);
        setContent('');
        setOriginal('');
      }
      await refreshFiles();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRename = async (oldPath: string, newPath: string) => {
    try {
      await commands.renameSkillFile(decodedName, oldPath, newPath);
      if (selectedPath === oldPath) {
        setSelectedPath(newPath);
      }
      await refreshFiles();
    } catch (e) {
      setError(String(e));
    }
  };

  const dirty = content !== original;
  const selectedEntry = files.find((f) => f.relative_path === selectedPath);
  const isEditable = selectedEntry && !selectedEntry.is_dir && isTextFile(selectedEntry.extension);

  if (loading) return <Skeleton className="h-96 w-full" />;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-2 py-2 border-b">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={() => navigate('/chef/skills')}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <h2 className="text-lg font-medium">{decodedName}</h2>
          {dirty && <span className="text-xs text-yellow-600">unsaved</span>}
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

      {/* Metadata bar */}
      <MetadataPanel
        skillName={decodedName}
        meta={meta}
        onMetaUpdated={setMeta}
      />

      {error && <p className="text-sm text-destructive px-3 py-1">{error}</p>}

      {/* Two-panel layout */}
      <div className="flex flex-1 min-h-0">
        {/* Left: File tree */}
        <div className="w-56 border-r shrink-0 overflow-hidden">
          <FileTree
            files={files}
            selectedPath={selectedPath}
            onSelect={setSelectedPath}
            onNewFile={handleNewFile}
            onNewFolder={handleNewFolder}
            onDelete={handleDelete}
            onRename={handleRename}
          />
        </div>

        {/* Right: Editor */}
        <div className="flex-1 flex flex-col min-w-0">
          {selectedPath && isEditable ? (
            <>
              <div className="px-3 py-1 border-b bg-muted/20">
                <span className="text-xs text-muted-foreground">{selectedPath}</span>
              </div>
              <Textarea
                className="flex-1 font-mono text-sm resize-none rounded-none border-0 focus-visible:ring-0"
                value={content}
                onChange={(e) => setContent(e.target.value)}
              />
            </>
          ) : selectedPath && selectedEntry && !isEditable ? (
            <div className="flex items-center justify-center h-full text-muted-foreground">
              <div className="text-center">
                <p className="text-sm">
                  {selectedEntry.is_dir
                    ? `Folder: ${selectedEntry.name}`
                    : `Binary file: ${selectedEntry.name}`}
                </p>
                {!selectedEntry.is_dir && (
                  <p className="text-xs mt-1">
                    {selectedEntry.size} bytes
                  </p>
                )}
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-center h-full text-muted-foreground">
              <p className="text-sm">Select a file to edit</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
