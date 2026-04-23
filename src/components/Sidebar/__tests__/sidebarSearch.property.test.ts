/**
 * Property 20: 侧边栏搜索过滤正确性
 *
 * For any set of sidebar items and a search keyword, the filter result
 * SHALL contain all and only items whose name contains the keyword
 * (case-insensitive).
 *
 * Validates: Requirement 26.5
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import { filterByName } from '../sidebarUtils';

interface NamedItem {
  name: string;
}

const arbNamedItem: fc.Arbitrary<NamedItem> = fc.record({
  name: fc.string({ minLength: 0, maxLength: 60 }),
});

describe('Property 20: Sidebar search filter correctness', () => {
  it('filtered results contain only items whose name includes the query (case-insensitive)', () => {
    fc.assert(
      fc.property(
        fc.array(arbNamedItem, { maxLength: 200 }),
        fc.string({ maxLength: 20 }),
        (items, query) => {
          const result = filterByName(items, query, (i) => i.name);

          // When the query is whitespace-only, filterByName returns all items
          // (no filtering applied), so we skip the containment check.
          if (!query.trim()) {
            expect(result.length).toBe(items.length);
            return;
          }

          const lower = query.toLowerCase();
          for (const item of result) {
            expect(item.name.toLowerCase()).toContain(lower);
          }
        },
      ),
    );
  });

  it('filtered results contain all items whose name includes the query', () => {
    fc.assert(
      fc.property(
        fc.array(arbNamedItem, { maxLength: 200 }),
        fc.string({ maxLength: 20 }),
        (items, query) => {
          const result = filterByName(items, query, (i) => i.name);

          // filterByName treats whitespace-only queries as "no filter"
          if (!query.trim()) {
            expect(result.length).toBe(items.length);
            return;
          }

          const lower = query.toLowerCase();
          const expected = items.filter((i) => i.name.toLowerCase().includes(lower));
          expect(result.length).toBe(expected.length);
        },
      ),
    );
  });

  it('empty query returns all items', () => {
    fc.assert(
      fc.property(fc.array(arbNamedItem, { maxLength: 200 }), (items) => {
        const result = filterByName(items, '', (i) => i.name);
        expect(result.length).toBe(items.length);
      }),
    );
  });

  it('whitespace-only query returns all items', () => {
    fc.assert(
      fc.property(
        fc.array(arbNamedItem, { maxLength: 200 }),
        fc.stringOf(fc.constant(' '), { minLength: 1, maxLength: 5 }),
        (items, query) => {
          const result = filterByName(items, query, (i) => i.name);
          expect(result.length).toBe(items.length);
        },
      ),
    );
  });

  it('filter is idempotent: filtering twice gives same result', () => {
    fc.assert(
      fc.property(
        fc.array(arbNamedItem, { maxLength: 100 }),
        fc.string({ minLength: 1, maxLength: 10 }),
        (items, query) => {
          const first = filterByName(items, query, (i) => i.name);
          const second = filterByName(first, query, (i) => i.name);
          expect(second.length).toBe(first.length);
        },
      ),
    );
  });

  it('result is a subset of the original items (preserves order)', () => {
    fc.assert(
      fc.property(
        fc.array(arbNamedItem, { maxLength: 100 }),
        fc.string({ maxLength: 10 }),
        (items, query) => {
          const result = filterByName(items, query, (i) => i.name);
          let idx = 0;
          for (const item of result) {
            while (idx < items.length && items[idx] !== item) idx++;
            expect(idx).toBeLessThan(items.length);
            idx++;
          }
        },
      ),
    );
  });
});
