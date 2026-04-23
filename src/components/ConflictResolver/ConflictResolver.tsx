import { useState, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { RepositoryState, TabId } from '../../ipc/types';
import { ConflictFileList } from './ConflictFileList';
import type { ConflictFileItem } from './ConflictFileList';
import { ThreeWayMerge } from './ThreeWayMerge';

export interface ConflictResolverProps {
  tabId: TabId;
  repoState: RepositoryState;
  conflictFiles: string[];
  onGetFileContent: (tabId: TabId, path: string, version: 'ours' | 'theirs' | 'merge') => Promise<string>;
  onMergeContentChange: (filePath: string, content: string) => void;
  onMarkResolved: (tabId: TabId, path: string) => void;
  onContinue: (tabId: TabId) => void;
  onAbort: (tabId: TabId) => void;
}

export function ConflictResolver({
  tabId,
  repoState,
  conflictFiles,
  onGetFileContent,
  onMergeContentChange,
  onMarkResolved,
  onContinue,
  onAbort,
}: ConflictResolverProps) {
  const { t } = useTranslation();
  const [resolvedFiles, setResolvedFiles] = useState<Set<string>>(new Set());
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [fileContents, setFileContents] = useState<Record<string, { ours: string; theirs: string; merge: string }>>({});

  const fileItems: ConflictFileItem[] = useMemo(
    () => conflictFiles.map((path) => ({ path, resolved: resolvedFiles.has(path) })),
    [conflictFiles, resolvedFiles],
  );

  const allResolved = conflictFiles.length > 0 && conflictFiles.every((f) => resolvedFiles.has(f));

  const handleSelectFile = useCallback(
    async (path: string) => {
      setSelectedFile(path);
      if (!fileContents[path]) {
        try {
          const [ours, theirs, merge] = await Promise.all([
            onGetFileContent(tabId, path, 'ours'),
            onGetFileContent(tabId, path, 'theirs'),
            onGetFileContent(tabId, path, 'merge'),
          ]);
          setFileContents((prev) => ({ ...prev, [path]: { ours, theirs, merge } }));
        } catch {
          setFileContents((prev) => ({ ...prev, [path]: { ours: '', theirs: '', merge: '' } }));
        }
      }
    },
    [tabId, fileContents, onGetFileContent],
  );

  const handleMarkResolved = useCallback(
    (path: string) => {
      setResolvedFiles((prev) => {
        const next = new Set(prev);
        next.add(path);
        return next;
      });
      onMarkResolved(tabId, path);
    },
    [tabId, onMarkResolved],
  );

  const handleContinue = useCallback(() => onContinue(tabId), [tabId, onContinue]);
  const handleAbort = useCallback(() => onAbort(tabId), [tabId, onAbort]);

  const operationLabel = getOperationLabel(repoState, t);
  const rebaseProgress = repoState.type === 'Rebasing' ? repoState : null;

  return (
    <div className="flex flex-col h-full text-sm text-gray-200">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 bg-gray-800 border-b border-gray-700">
        <div className="flex items-center gap-2">
          <span className="font-medium text-xs">{t('conflict.title')}</span>
          <span className="text-xs text-gray-400">— {operationLabel}</span>
          {rebaseProgress && (
            <span className="text-xs text-yellow-400">
              {t('rebase.progress', { current: rebaseProgress.current, total: rebaseProgress.total })}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <button
            className="text-xs px-2 py-1 rounded bg-red-700 hover:bg-red-600 text-white"
            onClick={handleAbort}
          >
            {getAbortLabel(repoState, t)}
          </button>
          <button
            className={`text-xs px-2 py-1 rounded text-white ${
              allResolved ? 'bg-green-700 hover:bg-green-600' : 'bg-gray-600 cursor-not-allowed opacity-50'
            }`}
            onClick={handleContinue}
            disabled={!allResolved}
          >
            {getContinueLabel(repoState, t)}
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex flex-1 min-h-0">
        {/* File list */}
        <div className="w-64 flex-shrink-0 border-r border-gray-700 overflow-auto">
          <ConflictFileList
            files={fileItems}
            selectedFile={selectedFile}
            onSelectFile={handleSelectFile}
            onMarkResolved={handleMarkResolved}
          />
        </div>

        {/* Three-way merge view */}
        <div className="flex-1 min-w-0">
          {selectedFile && fileContents[selectedFile] ? (
            <ThreeWayMerge
              filePath={selectedFile}
              oursContent={fileContents[selectedFile].ours}
              theirsContent={fileContents[selectedFile].theirs}
              initialMergeContent={fileContents[selectedFile].merge}
              onMergeContentChange={onMergeContentChange}
            />
          ) : (
            <div className="flex items-center justify-center h-full text-gray-500 text-xs">
              {conflictFiles.length === 0
                ? t('diff.noChanges')
                : t('conflict.title')}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}


function getOperationLabel(state: RepositoryState, t: (key: string) => string): string {
  switch (state.type) {
    case 'Merging':
      return t('branch.merge');
    case 'Rebasing':
      return t('rebase.start');
    case 'CherryPicking':
      return t('cherryPick.title');
    case 'Reverting':
      return t('revert.title');
    default:
      return '';
  }
}

function getContinueLabel(state: RepositoryState, t: (key: string) => string): string {
  switch (state.type) {
    case 'Rebasing':
      return t('rebase.continue');
    default:
      return t('common.confirm');
  }
}

function getAbortLabel(state: RepositoryState, t: (key: string) => string): string {
  switch (state.type) {
    case 'Rebasing':
      return t('rebase.abort');
    default:
      return t('common.cancel');
  }
}
