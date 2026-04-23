import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { CommitInfo } from '../../ipc/types';
import { classifyNode, getMenuItemsForNode, getLocalBranchName, getRemoteBranchName } from './menuItems';
import type { MenuItem } from './menuItems';

export interface CommitContextMenuProps {
  commit: CommitInfo;
  position: { x: number; y: number };
  visible: boolean;
  onAction: (actionId: string, commit: CommitInfo) => void;
  onClose: () => void;
}

export function CommitContextMenu({
  commit,
  position,
  visible,
  onAction,
  onClose,
}: CommitContextMenuProps) {
  const { t } = useTranslation();

  const nodeType = classifyNode(commit);
  const items = getMenuItemsForNode(nodeType);

  const handleAction = useCallback(
    (id: string) => {
      onAction(id, commit);
      onClose();
    },
    [onAction, commit, onClose],
  );

  // Close on outside click
  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      onClose();
    },
    [onClose],
  );

  if (!visible) return null;

  const _localBranch = getLocalBranchName(commit.refs);
  const _remoteBranch = getRemoteBranchName(commit.refs);

  return (
    <>
      {/* Invisible backdrop to catch outside clicks */}
      <div
        className="context-menu-backdrop"
        onClick={handleBackdropClick}
        onContextMenu={handleBackdropClick}
        role="presentation"
      />

      <div
        className="context-menu commit-context-menu"
        style={{ left: position.x, top: position.y, position: 'fixed' }}
        role="menu"
        aria-label="Commit actions"
      >
        {items.map((item) => {
          if (item.separator) {
            return <div key={item.id} className="context-menu-separator" role="separator" />;
          }

          if (item.children) {
            return (
              <SubmenuItem
                key={item.id}
                item={item}
                t={t}
                onAction={handleAction}
              />
            );
          }

          return (
            <button
              key={item.id}
              type="button"
              className={`context-menu-item ${item.danger ? 'context-menu-item--danger' : ''}`}
              disabled={!item.enabled}
              onClick={() => handleAction(item.id)}
              role="menuitem"
            >
              {t(item.labelKey)}
            </button>
          );
        })}
      </div>
    </>
  );
}

/** Submenu for items with children (e.g., Reset → Soft/Mixed/Hard). */
function SubmenuItem({
  item,
  t,
  onAction,
}: {
  item: MenuItem;
  t: (key: string) => string;
  onAction: (id: string) => void;
}) {
  return (
    <div className="context-menu-submenu" role="menuitem" aria-haspopup="true">
      <span className="context-menu-item">
        Reset ▸
      </span>
      <div className="context-menu-submenu-content context-menu" role="menu">
        {item.children!.map((child) => (
          <button
            key={child.id}
            type="button"
            className={`context-menu-item ${child.danger ? 'context-menu-item--danger' : ''}`}
            disabled={!child.enabled}
            onClick={() => onAction(child.id)}
            role="menuitem"
          >
            {t(child.labelKey)}
          </button>
        ))}
      </div>
    </div>
  );
}
