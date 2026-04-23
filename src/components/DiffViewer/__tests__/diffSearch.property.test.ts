/**
 * Property 13: 文本查找完备性
 *
 * For any text content and search string, the set of match positions returned
 * by the find function SHALL contain all occurrence positions of the search
 * string in the text, and SHALL NOT contain any non-match positions.
 *
 * Validates: Requirements 13.3, 13.4
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import { findAllMatches } from '../diffSearch';

describe('Property 13: Text search completeness', () => {
  it('every returned position is a valid match (soundness)', () => {
    fc.assert(
      fc.property(
        fc.string({ maxLength: 200 }),
        fc.string({ minLength: 1, maxLength: 20 }),
        (text, query) => {
          const results = findAllMatches(text, query);
          const lowerText = text.toLowerCase();
          const lowerQuery = query.toLowerCase();

          for (const pos of results) {
            expect(pos).toBeGreaterThanOrEqual(0);
            expect(pos + lowerQuery.length).toBeLessThanOrEqual(lowerText.length);
            expect(lowerText.substring(pos, pos + lowerQuery.length)).toBe(lowerQuery);
          }
        },
      ),
    );
  });

  it('all match positions in the text are included in the result (completeness)', () => {
    fc.assert(
      fc.property(
        fc.string({ maxLength: 200 }),
        fc.string({ minLength: 1, maxLength: 20 }),
        (text, query) => {
          const results = findAllMatches(text, query);
          const lowerText = text.toLowerCase();
          const lowerQuery = query.toLowerCase();

          // Collect all expected positions via brute-force scan
          const expected: number[] = [];
          for (let i = 0; i <= lowerText.length - lowerQuery.length; i++) {
            if (lowerText.substring(i, i + lowerQuery.length) === lowerQuery) {
              expected.push(i);
            }
          }

          expect(results).toEqual(expected);
        },
      ),
    );
  });

  it('results are in strictly ascending order', () => {
    fc.assert(
      fc.property(
        fc.string({ maxLength: 200 }),
        fc.string({ minLength: 1, maxLength: 20 }),
        (text, query) => {
          const results = findAllMatches(text, query);

          for (let i = 1; i < results.length; i++) {
            expect(results[i]).toBeGreaterThan(results[i - 1]);
          }
        },
      ),
    );
  });

  it('empty query always returns empty array', () => {
    fc.assert(
      fc.property(fc.string({ maxLength: 200 }), (text) => {
        expect(findAllMatches(text, '')).toEqual([]);
      }),
    );
  });

  it('empty text always returns empty array', () => {
    fc.assert(
      fc.property(fc.string({ minLength: 1, maxLength: 20 }), (query) => {
        expect(findAllMatches('', query)).toEqual([]);
      }),
    );
  });

  it('known substring produces at least one match', () => {
    fc.assert(
      fc.property(
        fc.string({ maxLength: 80 }),
        fc.string({ minLength: 1, maxLength: 20 }),
        fc.string({ maxLength: 80 }),
        (prefix, query, suffix) => {
          const text = prefix + query + suffix;
          const results = findAllMatches(text, query);
          expect(results.length).toBeGreaterThanOrEqual(1);
        },
      ),
    );
  });
});
