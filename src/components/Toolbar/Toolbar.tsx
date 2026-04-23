import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import * as Tooltip from '@radix-ui/react-tooltip';
import { useRepoStore } from '../../stores/repoStore';
import { useUiStore } from '../../stores/uiStore';
import { gitApi } from '../../ipc/client';
import './Toolbar.css';

export interface ToolbarProps {
  className?: string;
}

export function Toolbar({ className }: ToolbarProps) {
  const { t } = useTranslation();
  const activeTabId = useRepoStore((s) => s.activeTabId);
  const toggleTerminal = useUiStore((s) => s.toggleTerminal);
  const terminalVisible = useUiStore((s) => s.terminalVisible);
  const toggleFullscreen = useUiStore((s) => s.toggleFullscreen);
  const fullscreen = useUiStore((s) => s.fullscreen);

  const [undoDesc, setUndoDesc] = useState<string | null>(null);
  const [redoDesc, setRedoDesc] = useState<string | null>(null);

  // Poll undo/redo availability when activeTabId changes
  const refreshUndoRedo = useCallback(async () => {
    if (!activeTabId) {
      setUndoDesc(null);
      setRedoDesc(null);
      return;
    }
    try {
      const [canUndo, canRedo] = await Promise.all([
        gitApi.canUndo(activeTabId),
        gitApi.canRedo(activeTabId),
      ]);
      setUndoDesc(canUndo);
      setRedoDesc(canRedo);
    } catch {
      setUndoDesc(null);
      setRedoDesc(null);
    }
  }, [activeTabId]);

  useEffect(() => {
    refreshUndoRedo();
  }, [refreshUndoRedo]);

  const handleUndo = useCallback(async () => {
    if (!activeTabId || !undoDesc) return;
    try {
      await gitApi.undo(activeTabId);
      refreshUndoRedo();
    } catch {
      // error handled upstream
    }
  }, [activeTabId, undoDesc, refreshUndoRedo]);

  const handleRedo = useCallback(async () => {
    if (!activeTabId || !redoDesc) return;
    try {
      await gitApi.redo(activeTabId);
      refreshUndoRedo();
    } catch {
      // error handled upstream
    }
  }, [activeTabId, redoDesc, refreshUndoRedo]);

  return (
    <Tooltip.Provider delayDuration={400}>
      <div className={`toolbar${className ? ` ${className}` : ''}`} role="toolbar" aria-label="Main toolbar">
        {/* Undo */}
        <Tooltip.Root>
          <Tooltip.Trigger asChild>
            <button
              className="toolbar-btn"
              type="button"
              disabled={!undoDesc}
              onClick={handleUndo}
              aria-label={undoDesc ? t('toolbar.undoAction', { action: undoDesc }) : t('toolbar.undo')}
            >
              ↩
            </button>
          </Tooltip.Trigger>
          <Tooltip.Portal>
            <Tooltip.Content className="toolbar-tooltip" sideOffset={5}>
              {undoDesc ? t('toolbar.undoAction', { action: undoDesc }) : t('toolbar.undo')}
              <Tooltip.Arrow className="toolbar-tooltip-arrow" />
            </Tooltip.Content>
          </Tooltip.Portal>
        </Tooltip.Root>

        {/* Redo */}
        <Tooltip.Root>
          <Tooltip.Trigger asChild>
            <button
              className="toolbar-btn"
              type="button"
              disabled={!redoDesc}
              onClick={handleRedo}
              aria-label={redoDesc ? t('toolbar.redoAction', { action: redoDesc }) : t('toolbar.redo')}
            >
              ↪
            </button>
          </Tooltip.Trigger>
          <Tooltip.Portal>
            <Tooltip.Content className="toolbar-tooltip" sideOffset={5}>
              {redoDesc ? t('toolbar.redoAction', { action: redoDesc }) : t('toolbar.redo')}
              <Tooltip.Arrow className="toolbar-tooltip-arrow" />
            </Tooltip.Content>
          </Tooltip.Portal>
        </Tooltip.Root>

        <div className="toolbar-separator" />

        {/* Terminal toggle */}
        <Tooltip.Root>
          <Tooltip.Trigger asChild>
            <button
              className={`toolbar-btn${terminalVisible ? ' toolbar-btn--active' : ''}`}
              type="button"
              onClick={toggleTerminal}
              aria-label={t('toolbar.toggleTerminal')}
              aria-pressed={terminalVisible}
            >
              ⌨
            </button>
          </Tooltip.Trigger>
          <Tooltip.Portal>
            <Tooltip.Content className="toolbar-tooltip" sideOffset={5}>
              {t('toolbar.toggleTerminal')}
              <Tooltip.Arrow className="toolbar-tooltip-arrow" />
            </Tooltip.Content>
          </Tooltip.Portal>
        </Tooltip.Root>

        {/* Fullscreen toggle */}
        <Tooltip.Root>
          <Tooltip.Trigger asChild>
            <button
              className={`toolbar-btn${fullscreen ? ' toolbar-btn--active' : ''}`}
              type="button"
              onClick={toggleFullscreen}
              aria-label={t('toolbar.toggleFullscreen')}
              aria-pressed={fullscreen}
            >
              ⛶
            </button>
          </Tooltip.Trigger>
          <Tooltip.Portal>
            <Tooltip.Content className="toolbar-tooltip" sideOffset={5}>
              {t('toolbar.toggleFullscreen')}
              <Tooltip.Arrow className="toolbar-tooltip-arrow" />
            </Tooltip.Content>
          </Tooltip.Portal>
        </Tooltip.Root>
      </div>
    </Tooltip.Provider>
  );
}
