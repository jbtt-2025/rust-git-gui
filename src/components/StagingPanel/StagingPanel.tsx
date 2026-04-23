import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { FileStatus, FileDiff, LineRange, TabId } from '../../ipc/types';
import { FileStatusIcon } from './FileStatusIcon';
import { StagingDiffView } from './StagingDiffView';

export interface StagingPanelProps {
  tabId: TabId;
  unstagedFiles: FileStatus[];
  stagedFiles: FileStatus[];
  onStageFiles: (tabId: TabId, paths: string[]) => void;
  onUnstageFiles: (tabId: TabId, paths: string[]) => void;
  onStageLines: (tabId: TabId, path: string, ranges: LineRange[]) => void;
  onUnstageLines: (tabId: TabId, path: string, ranges: LineRange[]) => void;
  onDiscardLines: (tabId: TabId, path: string, ranges: LineRange[]) => void;
  onGetFileDiff: (tabId: TabId, path: string, staged: boolean) => Promise<FileDiff>;
}

export function StagingPanel({
  tabId,
  unstagedFiles,
  stagedFiles,
  onStageFiles,
  onUnstageFiles,
  onStageLines,
  onUnstageLines,
  onDiscardLines,
  onGetFileDiff,
}: StagingPanelProps) {
  const { t } = useTranslation();
  const [selectedFile, setSelectedFile] = useState<{ path: string; staged: boolean } | null>(null);
  const [fileDiff, setFileDiff] = useState<FileDiff | null>(null);
  const [unstagedCollapsed, setUnstagedCollapsed] = useState(false);
  const [stagedCollapsed, setStagedCollapsed] = useState(false);

  const handleSelectFile = useCallback(
    async (path: string, staged: boolean) => {
      setSelectedFile({ path, staged });
      try {
        const diff = await onGetFileDiff(tabId, path, staged);
        setFileDiff(diff);
      } catch {
        setFileDiff(null);
      }
    },
    [tabId, onGetFileDiff],
  );

  const handleStageFile = useCallback(
    (path: string) => onStageFiles(tabId, [path]),
    [tabId, onStageFiles],
  );

  const handleUnstageFile = useCallback(
    (path: string) => onUnstageFiles(tabId, [path]),
    [tabId, onUnstageFiles],
  );

  const handleStageAll = useCallback(
    () => onStageFiles(tabId, unstagedFiles.map((f) => f.path)),
    [tabId, unstagedFiles, onStageFiles],
  );

  const handleUnstageAll = useCallback(
    () => onUnstageFiles(tabId, stagedFiles.map((f) => f.path)),
    [tabId, stagedFiles, onUnstageFiles],
  );

  const handleStageLines = useCallback(
    (path: string, ranges: LineRange[]) => onStageLines(tabId, path, ranges),
    [tabId, onStageLines],
  );

  const handleUnstageLines = useCallback(
    (path: string, ranges: LineRange[]) => onUnstageLines(tabId, path, ranges),
    [tabId, onUnstageLines],
  );

  const handleDiscardLines = useCallback(
    (path: string, ranges: LineRange[]) => onDiscardLines(tabId, path, ranges),
    [tabId, onDiscardLines],
  );

  return (
    <div className="flex flex-col h-full text-sm text-gray-200">
      {/* File lists */}
      <div className="flex-shrink-0 overflow-auto max-h-[50%]">
        {/* Unstaged Changes */}
        <div className="border-b border-gray-700">
          <div
            className="flex items-center justify-between px-3 py-1.5 bg-gray-800 cursor-pointer select-none hover:bg-gray-750"
            onClick={() => setUnstagedCollapsed((c) => !c)}
            role="button"
            aria-expanded={!unstagedCollapsed}
          >
            <div className="flex items-center gap-1">
              <span className="text-gray-400 text-xs">{unstagedCollapsed ? '▶' : '▼'}</span>
              <span className="font-medium text-xs">{t('staging.unstaged')}</span>
              <span className="text-gray-500 text-xs">({unstagedFiles.length})</span>
            </div>
            {unstagedFiles.length > 0 && (
              <button
                className="text-xs px-2 py-0.5 rounded bg-green-700 hover:bg-green-600 text-white"
                onClick={(e) => { e.stopPropagation(); handleStageAll(); }}
              >
                {t('staging.stageAll')}
              </button>
            )}
          </div>
          {!unstagedCollapsed && (
            <ul className="divide-y divide-gray-700/50">
              {unstagedFiles.map((file) => (
                <FileEntry
                  key={file.path}
                  file={file}
                  isSelected={selectedFile?.path === file.path && !selectedFile.staged}
                  onSelect={() => handleSelectFile(file.path, false)}
                  actionLabel={t('staging.stage')}
                  onAction={() => handleStageFile(file.path)}
                  actionColor="bg-green-700 hover:bg-green-600"
                />
              ))}
            </ul>
          )}
        </div>

        {/* Staged Changes */}
        <div className="border-b border-gray-700">
          <div
            className="flex items-center justify-between px-3 py-1.5 bg-gray-800 cursor-pointer select-none hover:bg-gray-750"
            onClick={() => setStagedCollapsed((c) => !c)}
            role="button"
            aria-expanded={!stagedCollapsed}
          >
            <div className="flex items-center gap-1">
              <span className="text-gray-400 text-xs">{stagedCollapsed ? '▶' : '▼'}</span>
              <span className="font-medium text-xs">{t('staging.staged')}</span>
              <span className="text-gray-500 text-xs">({stagedFiles.length})</span>
            </div>
            {stagedFiles.length > 0 && (
              <button
                className="text-xs px-2 py-0.5 rounded bg-yellow-700 hover:bg-yellow-600 text-white"
                onClick={(e) => { e.stopPropagation(); handleUnstageAll(); }}
              >
                {t('staging.unstageAll')}
              </button>
            )}
          </div>
          {!stagedCollapsed && (
            <ul className="divide-y divide-gray-700/50">
              {stagedFiles.map((file) => (
                <FileEntry
                  key={file.path}
                  file={file}
                  isSelected={selectedFile?.path === file.path && selectedFile.staged}
                  onSelect={() => handleSelectFile(file.path, true)}
                  actionLabel={t('staging.unstage')}
                  onAction={() => handleUnstageFile(file.path)}
                  actionColor="bg-yellow-700 hover:bg-yellow-600"
                />
              ))}
            </ul>
          )}
        </div>
      </div>

      {/* Diff view for selected file */}
      <div className="flex-1 overflow-auto border-t border-gray-700">
        {selectedFile && fileDiff ? (
          <StagingDiffView
            diff={fileDiff}
            staged={selectedFile.staged}
            onStageLines={handleStageLines}
            onUnstageLines={handleUnstageLines}
            onDiscardLines={handleDiscardLines}
          />
        ) : (
          <div className="flex items-center justify-center h-full text-gray-500 text-xs">
            {t('diff.noChanges')}
          </div>
        )}
      </div>
    </div>
  );
}

interface FileEntryProps {
  file: FileStatus;
  isSelected: boolean;
  onSelect: () => void;
  actionLabel: string;
  onAction: () => void;
  actionColor: string;
}

function FileEntry({ file, isSelected, onSelect, actionLabel, onAction, actionColor }: FileEntryProps) {
  return (
    <li
      className={`flex items-center gap-2 px-3 py-1 cursor-pointer hover:bg-gray-700/50 ${
        isSelected ? 'bg-blue-900/30' : ''
      }`}
      onClick={onSelect}
      role="option"
      aria-selected={isSelected}
    >
      <FileStatusIcon status={file.status} />
      <span className="flex-1 truncate text-xs">{file.path}</span>
      <button
        className={`text-xs px-1.5 py-0.5 rounded text-white ${actionColor}`}
        onClick={(e) => {
          e.stopPropagation();
          onAction();
        }}
      >
        {actionLabel}
      </button>
    </li>
  );
}
