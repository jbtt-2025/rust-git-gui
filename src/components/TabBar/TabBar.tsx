import { useCallback } from 'react';
import { useRepoStore } from '../../stores/repoStore';
import { gitApi } from '../../ipc';
import { TabItem } from './TabItem';
import './TabBar.css';

export interface TabBarProps {
  /** Optional class name appended to the root element. */
  className?: string;
}

export function TabBar({ className }: TabBarProps) {
  const tabs = useRepoStore((s) => s.tabs);
  const activeTabId = useRepoStore((s) => s.activeTabId);
  const setActiveTab = useRepoStore((s) => s.setActiveTab);
  const removeTab = useRepoStore((s) => s.removeTab);
  const reorderTabs = useRepoStore((s) => s.reorderTabs);

  const handleClose = useCallback(
    async (tabId: string) => {
      try {
        await gitApi.closeRepository(tabId);
      } catch {
        // Best-effort: still remove the tab from UI even if backend fails
      }
      removeTab(tabId);
    },
    [removeTab],
  );

  const handleDragStart = useCallback((e: React.DragEvent, tabId: string) => {
    e.dataTransfer.setData('text/plain', tabId);
    e.dataTransfer.effectAllowed = 'move';
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent, toTabId: string) => {
      e.preventDefault();
      const fromTabId = e.dataTransfer.getData('text/plain');
      if (fromTabId && fromTabId !== toTabId) {
        reorderTabs(fromTabId, toTabId);
      }
    },
    [reorderTabs],
  );

  const tabEntries = Array.from(tabs.values());

  if (tabEntries.length === 0) return null;

  return (
    <div
      className={`tab-bar${className ? ` ${className}` : ''}`}
      role="tablist"
      aria-label="Repository tabs"
    >
      {tabEntries.map((tab) => (
        <TabItem
          key={tab.tabId}
          tabId={tab.tabId}
          repoName={tab.repoName}
          hasChanges={tab.hasChanges}
          isActive={tab.tabId === activeTabId}
          onActivate={setActiveTab}
          onClose={handleClose}
          onDragStart={handleDragStart}
          onDragOver={handleDragOver}
          onDrop={handleDrop}
        />
      ))}
    </div>
  );
}
