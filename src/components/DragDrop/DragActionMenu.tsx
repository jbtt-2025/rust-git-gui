import { useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import './DragDrop.css';

export interface DragActionMenuProps {
  source: string;
  target: string;
  targetType: 'local' | 'remote';
  position: { x: number; y: number };
  onClose: () => void;
  onMerge?: (source: string, target: string) => void;
  onRebase?: (source: string, target: string) => void;
  onCherryPick?: (source: string, target: string) => void;
  onPush?: (source: string, target: string) => void;
}

/**
 * Popup menu shown after a branch is dropped onto another branch.
 *
 * - Local → Local: Merge / Rebase / Cherry-pick
 * - Local → Remote: Push
 * - Remote → Local: Merge / Rebase
 */
export function DragActionMenu({
  source,
  target,
  targetType,
  position,
  onClose,
  onMerge,
  onRebase,
  onCherryPick,
  onPush,
}: DragActionMenuProps) {
  const { t } = useTranslation();
  const menuRef = useRef<HTMLDivElement>(null);

  // Close on outside click or Escape
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose();
    }
    document.addEventListener('mousedown', handleClick);
    document.addEventListener('keydown', handleKey);
    return () => {
      document.removeEventListener('mousedown', handleClick);
      document.removeEventListener('keydown', handleKey);
    };
  }, [onClose]);

  // Clamp position so the menu stays within the viewport
  const style: React.CSSProperties = {
    position: 'fixed',
    left: position.x,
    top: position.y,
    zIndex: 200,
  };

  const handleMerge = useCallback(() => {
    onMerge?.(source, target);
    onClose();
  }, [onMerge, source, target, onClose]);

  const handleRebase = useCallback(() => {
    onRebase?.(source, target);
    onClose();
  }, [onRebase, source, target, onClose]);

  const handleCherryPick = useCallback(() => {
    onCherryPick?.(source, target);
    onClose();
  }, [onCherryPick, source, target, onClose]);

  const handlePush = useCallback(() => {
    onPush?.(source, target);
    onClose();
  }, [onPush, source, target, onClose]);

  const showLocalToLocal = targetType === 'local';
  const showPush = targetType === 'remote';

  return (
    <div ref={menuRef} className="drag-action-menu" style={style} role="menu">
      <div className="drag-action-menu-header">
        <span className="drag-action-menu-label">
          {source} → {target}
        </span>
      </div>

      {showLocalToLocal && (
        <>
          <button
            className="drag-action-menu-item"
            role="menuitem"
            onClick={handleMerge}
          >
            {t('dragDrop.merge')}
          </button>
          <button
            className="drag-action-menu-item"
            role="menuitem"
            onClick={handleRebase}
          >
            {t('dragDrop.rebase')}
          </button>
          <button
            className="drag-action-menu-item"
            role="menuitem"
            onClick={handleCherryPick}
          >
            {t('dragDrop.cherryPick')}
          </button>
        </>
      )}

      {showPush && (
        <button
          className="drag-action-menu-item"
          role="menuitem"
          onClick={handlePush}
        >
          {t('dragDrop.push')}
        </button>
      )}
    </div>
  );
}
