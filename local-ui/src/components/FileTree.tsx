import { useState } from 'react';
import {
  ChevronRight,
  ChevronDown,
  File,
  Folder,
  FolderOpen,
  Plus,
  FolderPlus,
  Trash2,
  Pencil,
} from 'lucide-react';
import { type SkillFileEntry, isTextFile } from '@/lib/tauri';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

interface FileTreeProps {
  files: SkillFileEntry[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
  onNewFile: (parentDir: string) => void;
  onNewFolder: (parentDir: string) => void;
  onDelete: (path: string) => void;
  onRename: (oldPath: string, newPath: string) => void;
}

interface TreeNode {
  entry: SkillFileEntry;
  children: TreeNode[];
}

function buildTree(files: SkillFileEntry[]): TreeNode[] {
  const roots: TreeNode[] = [];
  const dirMap = new Map<string, TreeNode>();

  // Sort: SKILL.md first, then dirs, then files alphabetically
  const sorted = [...files].sort((a, b) => {
    if (a.name === 'SKILL.md') return -1;
    if (b.name === 'SKILL.md') return 1;
    if (a.is_dir && !b.is_dir) return -1;
    if (!a.is_dir && b.is_dir) return 1;
    return a.name.localeCompare(b.name);
  });

  for (const entry of sorted) {
    const node: TreeNode = { entry, children: [] };
    if (entry.is_dir) {
      dirMap.set(entry.relative_path, node);
    }

    const slashIdx = entry.relative_path.lastIndexOf('/');
    if (slashIdx === -1) {
      roots.push(node);
    } else {
      const parentPath = entry.relative_path.substring(0, slashIdx);
      const parent = dirMap.get(parentPath);
      if (parent) {
        parent.children.push(node);
      } else {
        roots.push(node);
      }
    }
  }

  return roots;
}

function TreeItem({
  node,
  depth,
  selectedPath,
  onSelect,
  onDelete,
  onRename,
}: {
  node: TreeNode;
  depth: number;
  selectedPath: string | null;
  onSelect: (path: string) => void;
  onDelete: (path: string) => void;
  onRename: (oldPath: string, newPath: string) => void;
}) {
  const [expanded, setExpanded] = useState(depth === 0);
  const [renaming, setRenaming] = useState(false);
  const [newName, setNewName] = useState(node.entry.name);
  const { entry } = node;
  const isSelected = entry.relative_path === selectedPath;
  const clickable = !entry.is_dir && isTextFile(entry.extension);

  const handleClick = () => {
    if (entry.is_dir) {
      setExpanded(!expanded);
    } else if (clickable) {
      onSelect(entry.relative_path);
    }
  };

  const handleRenameSubmit = () => {
    if (newName && newName !== entry.name) {
      const parentPath = entry.relative_path.substring(
        0,
        entry.relative_path.length - entry.name.length
      );
      onRename(entry.relative_path, parentPath + newName);
    }
    setRenaming(false);
  };

  return (
    <div>
      <div
        className={`flex items-center gap-1 py-0.5 px-1 rounded text-sm cursor-pointer group hover:bg-muted ${
          isSelected ? 'bg-primary/10 text-primary' : ''
        } ${!clickable && !entry.is_dir ? 'opacity-60' : ''}`}
        style={{ paddingLeft: `${depth * 16 + 4}px` }}
        onClick={handleClick}
      >
        {entry.is_dir ? (
          expanded ? (
            <ChevronDown className="h-3.5 w-3.5 shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 shrink-0" />
          )
        ) : (
          <span className="w-3.5" />
        )}
        {entry.is_dir ? (
          expanded ? (
            <FolderOpen className="h-4 w-4 shrink-0 text-amber-500" />
          ) : (
            <Folder className="h-4 w-4 shrink-0 text-amber-500" />
          )
        ) : (
          <File className="h-4 w-4 shrink-0 text-muted-foreground" />
        )}
        {renaming ? (
          <Input
            className="h-5 text-xs px-1 py-0"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onBlur={handleRenameSubmit}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleRenameSubmit();
              if (e.key === 'Escape') setRenaming(false);
            }}
            autoFocus
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <span className="truncate text-xs">{entry.name}</span>
        )}
        <span className="flex-1" />
        <div className="hidden group-hover:flex items-center gap-0.5">
          <button
            className="p-0.5 rounded hover:bg-muted-foreground/20"
            onClick={(e) => {
              e.stopPropagation();
              setRenaming(true);
              setNewName(entry.name);
            }}
            title="Rename"
          >
            <Pencil className="h-3 w-3" />
          </button>
          <button
            className="p-0.5 rounded hover:bg-destructive/20"
            onClick={(e) => {
              e.stopPropagation();
              onDelete(entry.relative_path);
            }}
            title="Delete"
          >
            <Trash2 className="h-3 w-3" />
          </button>
        </div>
      </div>
      {entry.is_dir && expanded && (
        <div>
          {node.children.map((child) => (
            <TreeItem
              key={child.entry.relative_path}
              node={child}
              depth={depth + 1}
              selectedPath={selectedPath}
              onSelect={onSelect}
              onDelete={onDelete}
              onRename={onRename}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export function FileTree({
  files,
  selectedPath,
  onSelect,
  onNewFile,
  onNewFolder,
  onDelete,
  onRename,
}: FileTreeProps) {
  const tree = buildTree(files);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-1 px-2 py-1.5 border-b">
        <span className="text-xs font-medium text-muted-foreground flex-1">FILES</span>
        <Button
          variant="ghost"
          size="icon"
          className="h-5 w-5"
          onClick={() => onNewFile('')}
          title="New File"
        >
          <Plus className="h-3 w-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-5 w-5"
          onClick={() => onNewFolder('')}
          title="New Folder"
        >
          <FolderPlus className="h-3 w-3" />
        </Button>
      </div>
      <div className="flex-1 overflow-y-auto py-1">
        {tree.map((node) => (
          <TreeItem
            key={node.entry.relative_path}
            node={node}
            depth={0}
            selectedPath={selectedPath}
            onSelect={onSelect}
            onDelete={onDelete}
            onRename={onRename}
          />
        ))}
      </div>
    </div>
  );
}
