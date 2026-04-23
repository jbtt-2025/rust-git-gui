import { useCallback } from 'react';

interface Props {
  branchName: string;
  isSoloed: boolean;
  isHidden: boolean;
  onSolo: (branch: string) => void;
  onHide: (branch: string) => void;
}

/**
 * Solo and Hide quick-action buttons for a branch item.
 * Solo = focus icon (◎), Hide = eye icon (👁).
 */
export function BranchActions({ branchName, isSoloed, isHidden, onSolo, onHide }: Props) {
  const handleSolo = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onSolo(branchName);
    },
    [onSolo, branchName],
  );

  const handleHide = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onHide(branchName);
    },
    [onHide, branchName],
  );

  return (
    <span className="branch-actions">
      <button
        type="button"
        className="branch-action-btn"
        data-active={isSoloed || undefined}
        onClick={handleSolo}
        title="Solo"
        aria-label={`Solo ${branchName}`}
        aria-pressed={isSoloed}
      >
        ◎
      </button>
      <button
        type="button"
        className="branch-action-btn"
        data-active={isHidden || undefined}
        onClick={handleHide}
        title="Hide"
        aria-label={`Hide ${branchName}`}
        aria-pressed={isHidden}
      >
        {isHidden ? '🚫' : '👁'}
      </button>
    </span>
  );
}
