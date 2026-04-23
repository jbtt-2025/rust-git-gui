import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { SubmoduleInfo } from '../../ipc/types';
import { gitApi } from '../../ipc/client';
import { useRepoStore } from '../../stores';
import { SubmodulesSection } from './SubmodulesSection';

export interface SubmoduleInteractionsProps {
  tabId: string;
  submodules: SubmoduleInfo[];
  onInit: (path: string) => void;
  onUpdate: (path: string) => void;
  onDeinit: (path: string) => void;
  onOpen: (path: string) => void;
  onCopyPath: (path: string) => void;
  onChangeUrl: (path: string) => void;
}

/**
 * Enhanced submodule section with double-click to open in new tab
 * and "Update All Submodules" button.
 */
export function SubmoduleInteractions({
  tabId,
  submodules,
  onInit,
  onUpdate,
  onDeinit,
  onOpen,
  onCopyPath,
  onChangeUrl,
}: SubmoduleInteractionsProps) {
  const { t } = useTranslation();
  const addTab = useRepoStore((s) => s.addTab);
  const [updating, setUpdating] = useState(false);

  const handleDoubleClick = useCallback(
    async (submodulePath: string, submoduleName: string) => {
      try {
        const newTabId = await gitApi.openRepository(submodulePath);
        addTab({
          tabId: newTabId,
          repoName: submoduleName,
          repoPath: submodulePath,
          hasChanges: false,
          repoState: { type: 'Clean' },
          soloedBranches: new Set(),
          hiddenBranches: new Set(),
          pinnedLeftBranches: [],
        });
      } catch {
        // Opening failed — the onOpen callback can handle fallback
        onOpen(submodulePath);
      }
    },
    [addTab, onOpen],
  );

  const handleUpdateAll = useCallback(async () => {
    if (updating || submodules.length === 0) return;
    setUpdating(true);
    try {
      for (const sub of submodules) {
        await gitApi.updateSubmodule(tabId, sub.path, true);
      }
    } finally {
      setUpdating(false);
    }
  }, [tabId, submodules, updating]);

  return (
    <div
      className="submodule-interactions"
      onDoubleClick={(e) => {
        // Find the closest sidebar-item to determine which submodule was double-clicked
        const target = e.target as HTMLElement;
        const item = target.closest('.sidebar-item');
        if (!item) return;
        const label = item.querySelector('.sidebar-item-label');
        if (!label) return;
        const name = label.textContent ?? '';
        const sub = submodules.find((s) => s.name === name);
        if (sub) {
          e.preventDefault();
          handleDoubleClick(sub.path, sub.name);
        }
      }}
    >
      {submodules.length > 0 && (
        <div className="submodule-actions">
          <button
            type="button"
            className="submodule-update-all-btn"
            onClick={handleUpdateAll}
            disabled={updating}
            title={t('submodule.updateAll')}
          >
            {updating ? t('submodule.updating') : t('submodule.updateAll')}
          </button>
        </div>
      )}

      <SubmodulesSection
        submodules={submodules}
        onInit={onInit}
        onUpdate={onUpdate}
        onDeinit={onDeinit}
        onOpen={onOpen}
        onCopyPath={onCopyPath}
        onChangeUrl={onChangeUrl}
      />
    </div>
  );
}
