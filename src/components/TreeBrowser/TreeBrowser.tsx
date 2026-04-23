import { useState, useEffect, useMemo, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { TabId, TreeEntry } from '../../ipc/types';
import { gitApi } from '../../ipc/client';
import { TreeNode } from './TreeNode';
import { FileContentViewer } from './FileContentViewer';
import './TreeBrowser.css';

export interface TreeBrowserProps {
  tabId: TabId;
  commitId: string | null;
  onViewBlame?: (path: string) => void;
  onViewHistory?: (path: string) => void;
}

/** Recursively filter tree entries by filename (case-insensitive). */
function filterTree(entries: TreeEntry[], query: string): TreeEntry[] {
  if (!query) return entries;
  const lower = query.toLowerCase();
  const result: TreeEntry[] = [];

  for (const entry of entries) {
    if (entry.type === 'directory') {
      const filteredChildren = filterTree(entry.children ?? [], query);
      if (filteredChildren.length > 0) {
        result.push({ ...entry, children: filteredChildren });
      }
    } else if (entry.name.toLowerCase().includes(lower)) {
      result.push(entry);
    }
  }

  return result;
}

export function TreeBrowser({ tabId, commitId, onViewBlame, onViewHistory }: TreeBrowserProps) {
  const { t } = useTranslation();
  const [tree, setTree] = useState<TreeEntry[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  // Fetch tree when commitId changes
  useEffect(() => {
    if (!commitId) {
      setTree([]);
      setSelectedPath(null);
      return;
    }

    let cancelled = false;

    gitApi.getCommitTree(tabId, commitId).then(
      (entries) => {
        if (!cancelled) {
          setTree(entries);
          setSelectedPath(null);
        }
      },
      () => {
        if (!cancelled) {
          setTree([]);
        }
      },
    );

    return () => {
      cancelled = true;
    };
  }, [tabId, commitId]);

  const filteredTree = useMemo(
    () => filterTree(tree, searchQuery),
    [tree, searchQuery],
  );

  const handleSearchChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => setSearchQuery(e.target.value),
    [],
  );

  const handleSelectFile = useCallback((path: string) => {
    setSelectedPath(path);
  }, []);

  const handleViewBlame = useCallback(
    (path: string) => onViewBlame?.(path),
    [onViewBlame],
  );

  const handleViewHistory = useCallback(
    (path: string) => onViewHistory?.(path),
    [onViewHistory],
  );

  const handleCopyPath = useCallback((path: string) => {
    navigator.clipboard.writeText(path).catch(() => {
      // clipboard write may fail silently
    });
  }, []);

  return (
    <div className="tree-browser" role="complementary" aria-label={t('treeBrowser.title')}>
      <div className="tree-browser-header">{t('treeBrowser.title')}</div>

      <div className="tree-browser-search">
        <input
          type="search"
          className="tree-browser-search-input"
          placeholder={t('treeBrowser.search')}
          value={searchQuery}
          onChange={handleSearchChange}
          aria-label={t('treeBrowser.search')}
        />
      </div>

      <div className="tree-browser-tree" role="tree" aria-label={t('treeBrowser.title')}>
        {filteredTree.map((entry) => (
          <TreeNode
            key={entry.path}
            entry={entry}
            depth={0}
            selectedPath={selectedPath}
            onSelectFile={handleSelectFile}
            onViewBlame={handleViewBlame}
            onViewHistory={handleViewHistory}
            onCopyPath={handleCopyPath}
          />
        ))}
      </div>

      {commitId && (
        <FileContentViewer
          tabId={tabId}
          commitId={commitId}
          filePath={selectedPath}
        />
      )}
    </div>
  );
}
