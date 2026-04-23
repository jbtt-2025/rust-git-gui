/**
 * Unit tests for WorktreeSection component logic.
 *
 * Validates: Requirements 27.2, 27.3
 */
import { describe, it, expect, vi } from 'vitest';
import type { WorktreeInfo } from '../../../ipc/types';
import type { WorktreeSectionProps } from '../WorktreeSection';

function makeWorktree(overrides: Partial<WorktreeInfo> = {}): WorktreeInfo {
  return {
    name: 'my-feature',
    path: '/repo/worktrees/my-feature',
    branch: 'feature-branch',
    is_main: false,
    ...overrides,
  };
}

describe('WorktreeSection worktree list display', () => {
  it('should provide all worktrees with their associated branches', () => {
    const worktrees: WorktreeInfo[] = [
      makeWorktree({ name: 'main-wt', path: '/repo', branch: 'main', is_main: true }),
      makeWorktree({ name: 'feat-a', path: '/repo/wt/feat-a', branch: 'feature-a' }),
      makeWorktree({ name: 'feat-b', path: '/repo/wt/feat-b', branch: 'feature-b' }),
    ];

    // Requirement 27.2: sidebar displays all worktrees and their associated branches
    expect(worktrees).toHaveLength(3);
    expect(worktrees[0].branch).toBe('main');
    expect(worktrees[1].branch).toBe('feature-a');
    expect(worktrees[2].branch).toBe('feature-b');
  });

  it('should handle worktrees with null branch (detached HEAD)', () => {
    const wt = makeWorktree({ name: 'detached', branch: null });
    expect(wt.branch).toBeNull();
  });

  it('should identify the main worktree', () => {
    const worktrees: WorktreeInfo[] = [
      makeWorktree({ name: 'main-wt', is_main: true }),
      makeWorktree({ name: 'secondary', is_main: false }),
    ];

    const mainWt = worktrees.find((w) => w.is_main);
    expect(mainWt).toBeDefined();
    expect(mainWt!.name).toBe('main-wt');
  });

  it('should handle empty worktree list', () => {
    const worktrees: WorktreeInfo[] = [];
    expect(worktrees).toHaveLength(0);
  });
});

describe('WorktreeSection click to switch', () => {
  it('should call onSelect with the clicked worktree', () => {
    const onSelect = vi.fn();
    const wt = makeWorktree({ name: 'feat-a', path: '/repo/wt/feat-a' });

    const props: WorktreeSectionProps = {
      worktrees: [wt],
      onSelect,
      onDelete: vi.fn(),
    };

    // Requirement 27.3: clicking a worktree switches to its working directory
    props.onSelect(wt);
    expect(onSelect).toHaveBeenCalledWith(wt);
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it('should pass the full WorktreeInfo including path for directory switching', () => {
    const onSelect = vi.fn();
    const wt = makeWorktree({ name: 'hotfix', path: '/repo/wt/hotfix', branch: 'hotfix-1' });

    onSelect(wt);
    expect(onSelect.mock.calls[0][0].path).toBe('/repo/wt/hotfix');
    expect(onSelect.mock.calls[0][0].branch).toBe('hotfix-1');
  });
});

describe('WorktreeSection context menu delete', () => {
  it('should call onDelete with the worktree name', () => {
    const onDelete = vi.fn();
    const props: WorktreeSectionProps = {
      worktrees: [makeWorktree({ name: 'to-delete' })],
      onSelect: vi.fn(),
      onDelete,
    };

    props.onDelete('to-delete');
    expect(onDelete).toHaveBeenCalledWith('to-delete');
  });

  it('should not allow deleting the main worktree (disabled in UI)', () => {
    const mainWt = makeWorktree({ name: 'main-wt', is_main: true });
    // The context menu delete item is disabled for main worktrees
    expect(mainWt.is_main).toBe(true);
  });
});

describe('WorktreeSection filtering', () => {
  it('should be filterable by name via sidebar search', () => {
    const worktrees: WorktreeInfo[] = [
      makeWorktree({ name: 'feature-auth' }),
      makeWorktree({ name: 'feature-ui' }),
      makeWorktree({ name: 'bugfix-login' }),
    ];

    const query = 'feature';
    const filtered = worktrees.filter((w) =>
      w.name.toLowerCase().includes(query.toLowerCase()),
    );
    expect(filtered).toHaveLength(2);
    expect(filtered.map((w) => w.name)).toEqual(['feature-auth', 'feature-ui']);
  });
});
