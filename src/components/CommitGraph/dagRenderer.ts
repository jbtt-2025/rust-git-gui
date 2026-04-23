import type { DagLayout, DagNode, DagEdge, CommitInfo, RefLabel } from '../../ipc/types';
import { getColorForIndex } from './dagColors';

export const NODE_RADIUS = 5;
export const ROW_HEIGHT = 28;
export const COL_WIDTH = 16;
export const GRAPH_PADDING_LEFT = 12;
export const GRAPH_WIDTH_MIN = 120;
export const TEXT_OFFSET_X = 8;
export const BADGE_HEIGHT = 16;
export const BADGE_PADDING = 6;
export const BADGE_GAP = 4;
export const CHERRY_PICK_MARKER = '🍒';

export interface RenderContext {
  ctx: CanvasRenderingContext2D;
  layout: DagLayout;
  commits: Map<string, CommitInfo>;
  scrollTop: number;
  viewportHeight: number;
  selectedCommitId: string | null;
  devicePixelRatio: number;
}

/** Compute the x position for a given column. */
function colX(col: number): number {
  return GRAPH_PADDING_LEFT + col * COL_WIDTH;
}

/** Compute the y position for a given row. */
function rowY(row: number, scrollTop: number): number {
  return row * ROW_HEIGHT + ROW_HEIGHT / 2 - scrollTop;
}

/** Determine visible row range for virtual scrolling. */
function visibleRange(scrollTop: number, viewportHeight: number, totalRows: number) {
  const first = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - 2);
  const last = Math.min(totalRows - 1, Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + 2);
  return { first, last };
}

/** Draw a single edge (branch/merge line). */
function drawEdge(ctx: CanvasRenderingContext2D, edge: DagEdge, fromRow: number, scrollTop: number) {
  const x1 = colX(edge.from_column);
  const y1 = rowY(fromRow, scrollTop);
  const x2 = colX(edge.to_column);
  const y2 = rowY(edge.to_row, scrollTop);

  ctx.strokeStyle = getColorForIndex(edge.color_index);
  ctx.lineWidth = 2;
  ctx.beginPath();

  if (edge.from_column === edge.to_column) {
    // Straight line
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
  } else {
    // Curved line for merge/branch
    const midY = (y1 + y2) / 2;
    ctx.moveTo(x1, y1);
    ctx.bezierCurveTo(x1, midY, x2, midY, x2, y2);
  }

  ctx.stroke();
}

/** Draw a commit node circle. */
function drawNode(
  ctx: CanvasRenderingContext2D,
  node: DagNode,
  scrollTop: number,
  isSelected: boolean,
) {
  const x = colX(node.column);
  const y = rowY(node.row, scrollTop);
  const color = getColorForIndex(node.color_index);

  ctx.beginPath();
  ctx.arc(x, y, NODE_RADIUS, 0, Math.PI * 2);
  ctx.fillStyle = isSelected ? '#ffffff' : color;
  ctx.fill();

  if (isSelected) {
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.stroke();
  }
}

/** Draw ref badges (branch names, tags) next to a commit node. */
function drawRefBadges(
  ctx: CanvasRenderingContext2D,
  refs: RefLabel[],
  graphRightX: number,
  y: number,
  isCherryPicked: boolean,
) {
  let offsetX = graphRightX + TEXT_OFFSET_X;

  if (isCherryPicked) {
    ctx.font = '12px sans-serif';
    ctx.fillText(CHERRY_PICK_MARKER, offsetX, y + 4);
    offsetX += 18;
  }

  for (const ref of refs) {
    const label = ref.name;
    ctx.font = '11px sans-serif';
    const textWidth = ctx.measureText(label).width;
    const badgeWidth = textWidth + BADGE_PADDING * 2;

    // Badge background
    let bgColor: string;
    if (ref.ref_type.type === 'Tag') {
      bgColor = 'rgba(249, 226, 175, 0.2)';
    } else if (ref.ref_type.type === 'RemoteBranch') {
      bgColor = 'rgba(137, 220, 235, 0.2)';
    } else {
      bgColor = ref.is_head ? 'rgba(166, 227, 161, 0.3)' : 'rgba(137, 180, 250, 0.2)';
    }

    ctx.fillStyle = bgColor;
    roundRect(ctx, offsetX, y - BADGE_HEIGHT / 2, badgeWidth, BADGE_HEIGHT, 3);
    ctx.fill();

    // Badge border
    if (ref.ref_type.type === 'Tag') {
      ctx.strokeStyle = 'rgba(249, 226, 175, 0.5)';
    } else if (ref.ref_type.type === 'RemoteBranch') {
      ctx.strokeStyle = 'rgba(137, 220, 235, 0.5)';
    } else {
      ctx.strokeStyle = ref.is_head ? 'rgba(166, 227, 161, 0.6)' : 'rgba(137, 180, 250, 0.4)';
    }
    ctx.lineWidth = 1;
    roundRect(ctx, offsetX, y - BADGE_HEIGHT / 2, badgeWidth, BADGE_HEIGHT, 3);
    ctx.stroke();

    // Badge text
    ctx.fillStyle = 'var(--color-text-primary, #cdd6f4)';
    ctx.fillText(label, offsetX + BADGE_PADDING, y + 4);

    offsetX += badgeWidth + BADGE_GAP;
  }
}

/** Draw commit message text. */
function drawCommitText(
  ctx: CanvasRenderingContext2D,
  commit: CommitInfo,
  x: number,
  y: number,
  maxWidth: number,
) {
  ctx.font = '13px sans-serif';
  ctx.fillStyle = 'var(--color-text-primary, #cdd6f4)';

  // First line of commit message
  const firstLine = commit.message.split('\n')[0];
  const truncated =
    ctx.measureText(firstLine).width > maxWidth
      ? firstLine.slice(0, Math.floor(maxWidth / 7)) + '…'
      : firstLine;
  ctx.fillText(truncated, x, y + 4);
}

/** Helper to draw a rounded rectangle path. */
function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
  ctx.closePath();
}

/** Main render function — draws the visible portion of the DAG. */
export function renderDag(rc: RenderContext) {
  const { ctx, layout, commits, scrollTop, viewportHeight, selectedCommitId, devicePixelRatio } = rc;
  const canvasWidth = ctx.canvas.width / devicePixelRatio;
  const { first, last } = visibleRange(scrollTop, viewportHeight, layout.total_rows);

  // Clear
  ctx.clearRect(0, 0, ctx.canvas.width / devicePixelRatio, ctx.canvas.height / devicePixelRatio);

  const graphRight = colX(layout.total_columns) + COL_WIDTH;

  // Build a set of visible rows for quick lookup
  const visibleNodes: DagNode[] = [];
  for (const node of layout.nodes) {
    if (node.row >= first && node.row <= last) {
      visibleNodes.push(node);
    }
  }

  // Draw edges first (behind nodes)
  for (const node of visibleNodes) {
    for (const edge of node.parent_edges) {
      // Draw edge if either end is visible
      if (
        (node.row >= first && node.row <= last) ||
        (edge.to_row >= first && edge.to_row <= last)
      ) {
        drawEdge(ctx, edge, node.row, scrollTop);
      }
    }
  }

  // Draw nodes and labels
  for (const node of visibleNodes) {
    const isSelected = node.commit_id === selectedCommitId;
    drawNode(ctx, node, scrollTop, isSelected);

    const commit = commits.get(node.commit_id);
    if (commit) {
      const y = rowY(node.row, scrollTop);

      // Draw ref badges
      if (commit.refs.length > 0 || commit.is_cherry_picked) {
        drawRefBadges(ctx, commit.refs, graphRight, y, commit.is_cherry_picked);
      }

      // Draw commit message after badges
      const textX = graphRight + 200; // approximate space for badges
      drawCommitText(ctx, commit, textX, y, canvasWidth - textX - 20);
    }
  }
}

/** Hit-test: find which commit node was clicked. */
export function hitTestNode(
  layout: DagLayout,
  scrollTop: number,
  clickX: number,
  clickY: number,
): string | null {
  for (const node of layout.nodes) {
    const x = colX(node.column);
    const y = rowY(node.row, scrollTop);
    const dx = clickX - x;
    const dy = clickY - y;
    if (dx * dx + dy * dy <= (NODE_RADIUS + 4) * (NODE_RADIUS + 4)) {
      return node.commit_id;
    }
  }
  return null;
}

/** Calculate total canvas height for the DAG. */
export function totalHeight(layout: DagLayout): number {
  return layout.total_rows * ROW_HEIGHT;
}
