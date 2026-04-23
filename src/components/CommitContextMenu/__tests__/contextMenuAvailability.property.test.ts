/**
 * Property 21: 上下文菜单动态可用性
 *
 * For any commit node (local branch, remote branch, or bare commit),
 * the context menu items SHALL have correct enabled/disabled states
 * based on the node type.
 *
 * Validates: Requirements 28.1, 28.2
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import { getMenuItemsForNode } from '../menuItems';
import type { NodeType, MenuItem } from '../menuItems';

const arbNodeType: fc.Arbitrary<NodeType> = fc.constantFrom(
  'local-branch',
  'remote-branch',
  'bare-commit',
);

/** Find a menu item by id, searching nested children too. */
function findItem(items: MenuItem[], id: string): MenuItem | undefined {
  for (const item of items) {
    if (item.id === id) return item;
    if (item.children) {
      const found = findItem(item.children, id);
      if (found) return found;
    }
  }
  return undefined;
}

describe('Property 21: Context menu dynamic availability', () => {
  it('bare-commit nodes disable branch-specific operations', () => {
    fc.assert(
      fc.property(fc.constant('bare-commit' as NodeType), (nodeType) => {
        const items = getMenuItemsForNode(nodeType);

        // Branch-specific items should be disabled
        const branchOnlyIds = [
          'pull', 'push', 'set-upstream', 'rename-branch',
          'delete-branch', 'copy-branch-name', 'copy-link-branch',
          'hide', 'pin-to-left', 'solo', 'merge', 'rebase',
          'create-worktree',
        ];

        for (const id of branchOnlyIds) {
          const item = findItem(items, id);
          if (item && !item.separator) {
            expect(item.enabled).toBe(false);
          }
        }
      }),
    );
  });

  it('all node types enable universal operations', () => {
    fc.assert(
      fc.property(arbNodeType, (nodeType) => {
        const items = getMenuItemsForNode(nodeType);

        // These should always be enabled regardless of node type
        const universalIds = [
          'checkout', 'cherry-pick', 'revert', 'create-branch',
          'copy-commit-sha', 'copy-link-commit', 'create-patch',
          'create-tag', 'create-annotated-tag',
        ];

        for (const id of universalIds) {
          const item = findItem(items, id);
          if (item) {
            expect(item.enabled).toBe(true);
          }
        }
      }),
    );
  });

  it('reset sub-items are always enabled', () => {
    fc.assert(
      fc.property(arbNodeType, (nodeType) => {
        const items = getMenuItemsForNode(nodeType);
        const resetItem = findItem(items, 'reset');
        expect(resetItem).toBeDefined();
        expect(resetItem!.children).toBeDefined();

        for (const child of resetItem!.children!) {
          expect(child.enabled).toBe(true);
        }
      }),
    );
  });

  it('local-branch nodes enable all operations', () => {
    fc.assert(
      fc.property(fc.constant('local-branch' as NodeType), (nodeType) => {
        const items = getMenuItemsForNode(nodeType);

        // All non-separator items should be enabled for local branches
        for (const item of items) {
          if (!item.separator) {
            expect(item.enabled).toBe(true);
          }
        }
      }),
    );
  });

  it('remote-branch nodes enable branch operations but disable local-only operations', () => {
    fc.assert(
      fc.property(fc.constant('remote-branch' as NodeType), (nodeType) => {
        const items = getMenuItemsForNode(nodeType);

        // Local-only operations should be disabled
        const localOnlyIds = ['pull', 'push', 'set-upstream', 'rename-branch'];
        for (const id of localOnlyIds) {
          const item = findItem(items, id);
          if (item) {
            expect(item.enabled).toBe(false);
          }
        }

        // Branch operations (available for both local and remote) should be enabled
        const branchIds = ['merge', 'rebase', 'delete-branch', 'copy-branch-name', 'hide', 'solo'];
        for (const id of branchIds) {
          const item = findItem(items, id);
          if (item) {
            expect(item.enabled).toBe(true);
          }
        }
      }),
    );
  });

  it('menu always contains expected number of actionable items', () => {
    fc.assert(
      fc.property(arbNodeType, (nodeType) => {
        const items = getMenuItemsForNode(nodeType);
        const actionable = items.filter((i) => !i.separator);
        // We expect at least 20 actionable menu items
        expect(actionable.length).toBeGreaterThanOrEqual(20);
      }),
    );
  });
});
