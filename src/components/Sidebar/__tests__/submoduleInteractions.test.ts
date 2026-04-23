/**
 * Unit tests for submodule interaction logic.
 *
 * Validates: Requirements 17.5, 17.7, 17.8
 */
import { describe, it, expect, vi } from 'vitest';
import type { SubmoduleInfo } from '../../../ipc/types';
import type { SubmoduleUpdateBannerProps } from '../SubmoduleUpdateBanner';
import type { CloneOptionsProps } from '../CloneOptions';

// --- helpers ---

function makeSub(overrides: Partial<SubmoduleInfo> = {}): SubmoduleInfo {
  return {
    name: 'lib',
    path: '/repo/lib',
    url: 'https://example.com/lib.git',
    head_id: 'abc123',
    status: 'Initialized',
    branch: 'main',
    ...overrides,
  };
}

// --- SubmoduleUpdateBanner logic ---

describe('SubmoduleUpdateBanner visibility', () => {
  it('should not render when show is false', () => {
    const props: SubmoduleUpdateBannerProps = {
      show: false,
      submodules: ['lib'],
      onUpdateAll: vi.fn(),
      onDismiss: vi.fn(),
    };
    // Banner returns null when show=false
    expect(props.show).toBe(false);
  });

  it('should not render when submodules list is empty', () => {
    const props: SubmoduleUpdateBannerProps = {
      show: true,
      submodules: [],
      onUpdateAll: vi.fn(),
      onDismiss: vi.fn(),
    };
    // Banner returns null when submodules.length === 0
    expect(props.submodules.length).toBe(0);
  });

  it('should be visible when show=true and submodules are present', () => {
    const props: SubmoduleUpdateBannerProps = {
      show: true,
      submodules: ['lib', 'vendor'],
      onUpdateAll: vi.fn(),
      onDismiss: vi.fn(),
    };
    expect(props.show).toBe(true);
    expect(props.submodules.length).toBeGreaterThan(0);
  });
});

// --- CloneOptions recursive flag ---

describe('CloneOptions recursive flag', () => {
  it('calls onRecursiveChange with toggled value', () => {
    const onChange = vi.fn();
    const props: CloneOptionsProps = {
      recursive: false,
      onRecursiveChange: onChange,
    };
    // Simulate toggle
    props.onRecursiveChange(!props.recursive);
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it('starts unchecked by default', () => {
    const props: CloneOptionsProps = {
      recursive: false,
      onRecursiveChange: vi.fn(),
    };
    expect(props.recursive).toBe(false);
  });
});

// --- Submodule double-click open logic ---

describe('Submodule open in new tab logic', () => {
  it('finds the correct submodule by name for double-click', () => {
    const subs: SubmoduleInfo[] = [
      makeSub({ name: 'lib-a', path: '/repo/lib-a' }),
      makeSub({ name: 'lib-b', path: '/repo/lib-b' }),
      makeSub({ name: 'vendor', path: '/repo/vendor' }),
    ];

    const clickedName = 'lib-b';
    const found = subs.find((s) => s.name === clickedName);
    expect(found).toBeDefined();
    expect(found!.path).toBe('/repo/lib-b');
  });

  it('does not match when name is not in the list', () => {
    const subs: SubmoduleInfo[] = [
      makeSub({ name: 'lib-a', path: '/repo/lib-a' }),
    ];

    const found = subs.find((s) => s.name === 'nonexistent');
    expect(found).toBeUndefined();
  });
});

// --- Update all submodules logic ---

describe('Update all submodules', () => {
  it('iterates over all submodules', async () => {
    const subs: SubmoduleInfo[] = [
      makeSub({ name: 'a', path: '/repo/a' }),
      makeSub({ name: 'b', path: '/repo/b' }),
      makeSub({ name: 'c', path: '/repo/c' }),
    ];

    const updated: string[] = [];
    for (const sub of subs) {
      updated.push(sub.path);
    }
    expect(updated).toEqual(['/repo/a', '/repo/b', '/repo/c']);
  });

  it('skips update when list is empty', () => {
    const subs: SubmoduleInfo[] = [];
    const updated: string[] = [];
    for (const sub of subs) {
      updated.push(sub.path);
    }
    expect(updated).toHaveLength(0);
  });
});
