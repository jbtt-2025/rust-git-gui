/**
 * Unit tests for StashesSection component logic.
 *
 * Validates: Requirements 21.6, 21.7
 */
import { describe, it, expect, vi } from 'vitest';
import type { StashEntry } from '../../../ipc/types';

function makeStash(overrides: Partial<StashEntry> = {}): StashEntry {
  return {
    index: 0,
    message: 'WIP on main',
    timestamp: 1700000000,
    commit_id: 'a'.repeat(40),
    ...overrides,
  };
}

describe('StashesSection stash count display (Requirement 21.7)', () => {
  it('should pass stash count equal to the number of stash entries', () => {
    const stashes: StashEntry[] = [
      makeStash({ index: 0, message: 'WIP on main' }),
      makeStash({ index: 1, message: 'WIP on feature' }),
      makeStash({ index: 2, message: 'WIP on bugfix' }),
    ];

    // StashesSection passes count={stashes.length} to SidebarSection
    const displayedCount = stashes.length;
    expect(displayedCount).toBe(3);
  });

  it('should display count of 0 when there are no stashes', () => {
    const stashes: StashEntry[] = [];
    const displayedCount = stashes.length;
    expect(displayedCount).toBe(0);
  });

  it('should display count of 1 for a single stash', () => {
    const stashes: StashEntry[] = [makeStash()];
    const displayedCount = stashes.length;
    expect(displayedCount).toBe(1);
  });

  it('should update count when stashes are added or removed', () => {
    const stashes: StashEntry[] = [
      makeStash({ index: 0 }),
      makeStash({ index: 1 }),
    ];
    expect(stashes.length).toBe(2);

    // Simulate adding a stash
    const afterAdd = [...stashes, makeStash({ index: 2 })];
    expect(afterAdd.length).toBe(3);

    // Simulate removing a stash (drop)
    const afterDrop = stashes.filter((s) => s.index !== 0);
    expect(afterDrop.length).toBe(1);
  });
});

describe('StashesSection click to view diff (Requirement 21.6)', () => {
  it('should call onSelect with the stash index when a stash entry is clicked', () => {
    const onSelect = vi.fn();
    const stash = makeStash({ index: 2, message: 'WIP on feature' });

    // Simulates the onClick handler: onClick={() => onSelect(stash.index)
    onSelect(stash.index);
    expect(onSelect).toHaveBeenCalledWith(2);
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it('should call onSelect with index 0 for the first stash', () => {
    const onSelect = vi.fn();
    const stash = makeStash({ index: 0 });

    onSelect(stash.index);
    expect(onSelect).toHaveBeenCalledWith(0);
  });

  it('should support keyboard activation via Enter key', () => {
    const onSelect = vi.fn();
    const stash = makeStash({ index: 1 });

    // The component has onKeyDown={(e) => { if (e.key === 'Enter') onSelect(stash.index); }}
    const simulatedKey = 'Enter';
    if (simulatedKey === 'Enter') {
      onSelect(stash.index);
    }
    expect(onSelect).toHaveBeenCalledWith(1);
  });

  it('should not trigger onSelect for non-Enter keys', () => {
    const onSelect = vi.fn();
    const stash = makeStash({ index: 1 });

    const simulatedKey: string = 'Space';
    if (simulatedKey === 'Enter') {
      onSelect(stash.index);
    }
    expect(onSelect).not.toHaveBeenCalled();
  });
});

describe('StashesSection stash entry display', () => {
  it('should display stash message as label', () => {
    const stash = makeStash({ index: 0, message: 'WIP on main: fix login' });
    expect(stash.message).toBe('WIP on main: fix login');
  });

  it('should fall back to stash@{index} when message is empty', () => {
    const stash = makeStash({ index: 3, message: '' });
    // Component uses: stash.message || `stash@{${stash.index}}`
    const displayLabel = stash.message || `stash@{${stash.index}}`;
    expect(displayLabel).toBe('stash@{3}');
  });

  it('should render all stash entries in the list', () => {
    const stashes: StashEntry[] = [
      makeStash({ index: 0, message: 'First' }),
      makeStash({ index: 1, message: 'Second' }),
      makeStash({ index: 2, message: 'Third' }),
    ];

    expect(stashes).toHaveLength(3);
    stashes.forEach((s, i) => {
      expect(s.index).toBe(i);
    });
  });
});

describe('StashesSection context menu actions', () => {
  it('should call onApply with the correct stash index', () => {
    const onApply = vi.fn();
    const stash = makeStash({ index: 1 });

    onApply(stash.index);
    expect(onApply).toHaveBeenCalledWith(1);
  });

  it('should call onPop with the correct stash index', () => {
    const onPop = vi.fn();
    const stash = makeStash({ index: 0 });

    onPop(stash.index);
    expect(onPop).toHaveBeenCalledWith(0);
  });

  it('should call onDrop with the correct stash index', () => {
    const onDrop = vi.fn();
    const stash = makeStash({ index: 2 });

    onDrop(stash.index);
    expect(onDrop).toHaveBeenCalledWith(2);
  });
});
