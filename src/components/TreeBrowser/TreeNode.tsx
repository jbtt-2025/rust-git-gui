import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import * as ContextMenu from '@radix-ui/react-context-menu';
import type { TreeEntry } from '../../ipc/types';

export interface TreeNodeProps {
  entry: TreeEntry;
  depth: number;
  selectedPath: string | null;
  onSelectFile: (path: string) => void;
  onViewBlame: (path: string) => void;
  onViewHistory: (path: string) => void;
  onCopyPath: (path: string) => void;
}

export function TreeNode({
  entry,
  depth,
  selectedPath,
  onSelectFile,
  onViewBlame,
  onViewHistory,
  onCopyPath,
}: TreeNodeProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const isDir = entry.type === 'directory';
  const isSelected = selectedPath === entry.path;

  const handleClick = useCallback(() => {
    if (isDir) {
      setExpanded((prev) => !prev);
    } else {
      onSelectFile(entry.path);
    }
  }, [isDir, entry.path, onSelectFile]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        handleClick();
      }
    },
    [handleClick],
  );

  const row = (
    <div
      className="tree-node-row"
      data-selected={isSelected}
      style={{ paddingLeft: `${10 + depth * 16}px` }}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      role="treeitem"
      aria-expanded={isDir ? expanded : undefined}
      aria-selected={isSelected}
      tabIndex={0}
    >
      {isDir ? (
        <span className="tree-node-chevron" data-expanded={expanded}>
          ▶
        </span>
      ) : (
        <span className="tree-node-chevron" />
      )}
      <span className="tree-node-icon">{isDir ? '📁' : '📄'}</span>
      <span className="tree-node-label">{entry.name}</span>
    </div>
  );

  const nodeContent = isDir ? (
    row
  ) : (
    <ContextMenu.Root>
      <ContextMenu.Trigger asChild>{row}</ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Content className="tree-context-menu">
          <ContextMenu.Item
            className="tree-context-menu-item"
            onSelect={() => onViewBlame(entry.path)}
          >
            {t('treeBrowser.viewBlame')}
          </ContextMenu.Item>
          <ContextMenu.Item
            className="tree-context-menu-item"
            onSelect={() => onViewHistory(entry.path)}
          >
            {t('treeBrowser.viewHistory')}
          </ContextMenu.Item>
          <ContextMenu.Item
            className="tree-context-menu-item"
            onSelect={() => onCopyPath(entry.path)}
          >
            {t('treeBrowser.copyPath')}
          </ContextMenu.Item>
        </ContextMenu.Content>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  );

  return (
    <div className="tree-node">
      {nodeContent}
      {isDir && expanded && entry.children && (
        <div className="tree-node-children" role="group">
          {entry.children.map((child) => (
            <TreeNode
              key={child.path}
              entry={child}
              depth={depth + 1}
              selectedPath={selectedPath}
              onSelectFile={onSelectFile}
              onViewBlame={onViewBlame}
              onViewHistory={onViewHistory}
              onCopyPath={onCopyPath}
            />
          ))}
        </div>
      )}
    </div>
  );
}
