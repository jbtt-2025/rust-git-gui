import {
  createContext,
  useContext,
  useState,
  useCallback,
  type ReactNode,
} from 'react';

// --- Types ---

export interface DraggedBranch {
  name: string;
  type: 'local' | 'remote';
}

export interface DropAction {
  source: string;
  target: string;
  targetType: 'local' | 'remote';
  position: { x: number; y: number };
}

interface BranchDragContextValue {
  draggedBranch: DraggedBranch | null;
  dropAction: DropAction | null;
  setDraggedBranch: (branch: DraggedBranch | null) => void;
  clearDraggedBranch: () => void;
  setDropAction: (action: DropAction | null) => void;
  clearDropAction: () => void;
}

// --- Context ---

const BranchDragContext = createContext<BranchDragContextValue | null>(null);

// --- Provider ---

export interface BranchDragProviderProps {
  children: ReactNode;
}

export function BranchDragProvider({ children }: BranchDragProviderProps) {
  const [draggedBranch, setDraggedBranchState] = useState<DraggedBranch | null>(null);
  const [dropAction, setDropActionState] = useState<DropAction | null>(null);

  const setDraggedBranch = useCallback(
    (branch: DraggedBranch | null) => setDraggedBranchState(branch),
    [],
  );

  const clearDraggedBranch = useCallback(() => setDraggedBranchState(null), []);

  const setDropAction = useCallback(
    (action: DropAction | null) => setDropActionState(action),
    [],
  );

  const clearDropAction = useCallback(() => setDropActionState(null), []);

  return (
    <BranchDragContext.Provider
      value={{
        draggedBranch,
        dropAction,
        setDraggedBranch,
        clearDraggedBranch,
        setDropAction,
        clearDropAction,
      }}
    >
      {children}
    </BranchDragContext.Provider>
  );
}

// --- Hook ---

export function useBranchDrag(): BranchDragContextValue {
  const ctx = useContext(BranchDragContext);
  if (!ctx) {
    throw new Error('useBranchDrag must be used within a BranchDragProvider');
  }
  return ctx;
}
