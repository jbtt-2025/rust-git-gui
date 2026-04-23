import { useState, useCallback, type ReactNode, type DragEvent } from 'react';
import { useBranchDrag } from './BranchDragProvider';

/** MIME type used to carry branch data through the native drag-and-drop API. */
const BRANCH_MIME = 'application/x-branch';

export interface BranchDropTargetProps {
  branchName: string;
  branchType: 'local' | 'remote';
  children: ReactNode;
}

/**
 * Wraps a branch item in the sidebar and makes it both a drag source and a
 * drop target using the native HTML5 drag-and-drop API.
 */
export function BranchDropTarget({
  branchName,
  branchType,
  children,
}: BranchDropTargetProps) {
  const { draggedBranch, setDraggedBranch, clearDraggedBranch, setDropAction } =
    useBranchDrag();
  const [isOver, setIsOver] = useState(false);

  // --- Drag source handlers ---

  const handleDragStart = useCallback(
    (e: DragEvent<HTMLDivElement>) => {
      const payload = JSON.stringify({ name: branchName, type: branchType });
      e.dataTransfer.setData(BRANCH_MIME, payload);
      e.dataTransfer.effectAllowed = 'move';
      setDraggedBranch({ name: branchName, type: branchType });
    },
    [branchName, branchType, setDraggedBranch],
  );

  const handleDragEnd = useCallback(() => {
    clearDraggedBranch();
    setIsOver(false);
  }, [clearDraggedBranch]);

  // --- Drop target handlers ---

  const canDrop = useCallback(
    (e: DragEvent<HTMLDivElement>): boolean => {
      if (!e.dataTransfer.types.includes(BRANCH_MIME)) return false;
      // Cannot drop on itself
      if (draggedBranch?.name === branchName) return false;
      return true;
    },
    [draggedBranch, branchName],
  );

  const handleDragOver = useCallback(
    (e: DragEvent<HTMLDivElement>) => {
      if (!canDrop(e)) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = 'move';
      setIsOver(true);
    },
    [canDrop],
  );

  const handleDragEnter = useCallback(
    (e: DragEvent<HTMLDivElement>) => {
      if (!canDrop(e)) return;
      e.preventDefault();
      setIsOver(true);
    },
    [canDrop],
  );

  const handleDragLeave = useCallback(() => {
    setIsOver(false);
  }, []);

  const handleDrop = useCallback(
    (e: DragEvent<HTMLDivElement>) => {
      e.preventDefault();
      setIsOver(false);

      const raw = e.dataTransfer.getData(BRANCH_MIME);
      if (!raw) return;

      try {
        const source = JSON.parse(raw) as { name: string; type: 'local' | 'remote' };
        if (source.name === branchName) return;

        setDropAction({
          source: source.name,
          target: branchName,
          targetType: branchType,
          position: { x: e.clientX, y: e.clientY },
        });
      } catch {
        // Ignore malformed data
      }
    },
    [branchName, branchType, setDropAction],
  );

  const isDraggingSelf = draggedBranch?.name === branchName;

  return (
    <div
      className={[
        'branch-drop-target',
        isOver ? 'branch-drop-target--over' : '',
        isDraggingSelf ? 'branch-drop-target--dragging' : '',
      ]
        .filter(Boolean)
        .join(' ')}
      draggable
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onDragOver={handleDragOver}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {children}
    </div>
  );
}
