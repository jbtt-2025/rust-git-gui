import { useRef, useEffect, useCallback, useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { DagLayout, CommitInfo, CommitDetail as CommitDetailType } from '../../ipc/types';
import { renderDag, hitTestNode, totalHeight, ROW_HEIGHT } from './dagRenderer';
import { CommitDetailPanel } from './CommitDetail';

export interface CommitGraphProps {
  layout: DagLayout;
  commits: CommitInfo[];
  selectedCommitId: string | null;
  commitDetail: CommitDetailType | null;
  detailLoading: boolean;
  soloedBranches: Set<string>;
  hiddenBranches: Set<string>;
  pinnedLeftBranches: string[];
  onSelectCommit: (commitId: string) => void;
  onContextMenu: (commitId: string, x: number, y: number) => void;
  onResetView: () => void;
}

export function CommitGraph({
  layout,
  commits,
  selectedCommitId,
  commitDetail,
  detailLoading,
  soloedBranches,
  hiddenBranches,
  pinnedLeftBranches: _pinnedLeftBranches,
  onSelectCommit,
  onContextMenu,
  onResetView,
}: CommitGraphProps) {
  const { t } = useTranslation();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(600);

  // Build a lookup map for commits
  const commitMap = useMemo(() => {
    const map = new Map<string, CommitInfo>();
    for (const c of commits) {
      map.set(c.id, c);
    }
    return map;
  }, [commits]);

  const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;

  // Resize observer for the container
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setViewportHeight(entry.contentRect.height);
      }
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  // Render the DAG whenever dependencies change
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size
    const container = containerRef.current;
    const width = container?.clientWidth ?? 800;
    canvas.width = width * dpr;
    canvas.height = viewportHeight * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${viewportHeight}px`;
    ctx.scale(dpr, dpr);

    renderDag({
      ctx,
      layout,
      commits: commitMap,
      scrollTop,
      viewportHeight,
      selectedCommitId,
      devicePixelRatio: dpr,
    });
  }, [layout, commitMap, scrollTop, viewportHeight, selectedCommitId, dpr]);

  // Handle scroll for virtual scrolling
  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    setScrollTop(e.currentTarget.scrollTop);
  }, []);

  // Handle click on canvas to select a commit
  const handleClick = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      const commitId = hitTestNode(layout, scrollTop, x, y);
      if (commitId) {
        onSelectCommit(commitId);
      }
    },
    [layout, scrollTop, onSelectCommit],
  );

  // Handle right-click for context menu
  const handleContextMenu = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      const commitId = hitTestNode(layout, scrollTop, x, y);
      if (commitId) {
        e.preventDefault();
        onContextMenu(commitId, e.clientX, e.clientY);
      }
    },
    [layout, scrollTop, onContextMenu],
  );

  const total = totalHeight(layout);
  const hasFilters = soloedBranches.size > 0 || hiddenBranches.size > 0;

  return (
    <div className="commit-graph-container">
      {/* Toolbar */}
      <div className="commit-graph-toolbar">
        {hasFilters && (
          <button
            type="button"
            className="commit-graph-reset-btn"
            onClick={onResetView}
            title={t('contextMenu.solo')}
          >
            Reset View
          </button>
        )}
      </div>

      {/* Scrollable DAG area */}
      <div className="commit-graph-scroll-area" ref={containerRef} onScroll={handleScroll}>
        {/* Spacer for virtual scroll height */}
        <div style={{ height: total, position: 'relative' }}>
          <canvas
            ref={canvasRef}
            className="commit-graph-canvas"
            onClick={handleClick}
            onContextMenu={handleContextMenu}
            style={{ position: 'sticky', top: 0 }}
            role="img"
            aria-label="Commit history graph"
          />
        </div>
      </div>

      {/* Commit detail panel */}
      {(selectedCommitId || detailLoading) && (
        <CommitDetailPanel detail={commitDetail} loading={detailLoading} />
      )}
    </div>
  );
}
