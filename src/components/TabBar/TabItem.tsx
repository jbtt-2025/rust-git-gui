import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';

export interface TabItemProps {
  tabId: string;
  repoName: string;
  hasChanges: boolean;
  isActive: boolean;
  onActivate: (tabId: string) => void;
  onClose: (tabId: string) => void;
  onDragStart: (e: React.DragEvent, tabId: string) => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: (e: React.DragEvent, tabId: string) => void;
}

export function TabItem({
  tabId,
  repoName,
  hasChanges,
  isActive,
  onActivate,
  onClose,
  onDragStart,
  onDragOver,
  onDrop,
}: TabItemProps) {
  const { t } = useTranslation();

  const handleClick = useCallback(() => onActivate(tabId), [onActivate, tabId]);

  const handleClose = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onClose(tabId);
    },
    [onClose, tabId],
  );

  const handleDragStart = useCallback(
    (e: React.DragEvent) => onDragStart(e, tabId),
    [onDragStart, tabId],
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => onDrop(e, tabId),
    [onDrop, tabId],
  );

  return (
    <div
      className={`tab-item${isActive ? ' tab-item--active' : ''}`}
      role="tab"
      aria-selected={isActive}
      tabIndex={isActive ? 0 : -1}
      draggable
      onClick={handleClick}
      onDragStart={handleDragStart}
      onDragOver={onDragOver}
      onDrop={handleDrop}
    >
      <span className="tab-item-label">
        {repoName}
        {hasChanges && (
          <span
            className="tab-item-indicator"
            title={t('tabBar.unsavedChanges')}
            aria-label={t('tabBar.unsavedChanges')}
          >
            ●
          </span>
        )}
      </span>
      <button
        className="tab-item-close"
        onClick={handleClose}
        aria-label={t('tabBar.closeTab')}
        title={t('tabBar.closeTab')}
      >
        ×
      </button>
    </div>
  );
}
