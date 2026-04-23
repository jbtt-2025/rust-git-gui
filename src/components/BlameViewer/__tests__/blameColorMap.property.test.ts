/**
 * Property 12: Blame 颜色一致性
 *
 * Validates: Requirements 10.2
 *
 * For any Blame dataset, lines with the same commit_id SHALL be assigned
 * the same color index, and lines with different commit_ids SHALL be
 * assigned different color indices.
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import type { BlameLine } from '../../../ipc/types';
import { assignBlameColors } from '../blameColorMap';

const arbBlameLine: fc.Arbitrary<BlameLine> = fc.record({
  line_number: fc.nat(),
  content: fc.string({ maxLength: 100 }),
  commit_id: fc.string({ minLength: 1, maxLength: 40 }),
  author: fc.string({ maxLength: 40 }),
  date: fc.integer(),
  original_line: fc.nat(),
});

const arbBlameLines: fc.Arbitrary<BlameLine[]> = fc.array(arbBlameLine, { maxLength: 50 });

describe('Property 12: Blame color consistency', () => {
  /**
   * **Validates: Requirements 10.2**
   */
  it('same commit_id always maps to the same color index', () => {
    fc.assert(
      fc.property(arbBlameLines, (lines) => {
        const colorMap = assignBlameColors(lines);
        for (const line of lines) {
          const idx = colorMap.get(line.commit_id);
          expect(idx).toBeDefined();
          // All lines with this commit_id should get the same index
          const sameCommitLines = lines.filter((l) => l.commit_id === line.commit_id);
          for (const other of sameCommitLines) {
            expect(colorMap.get(other.commit_id)).toBe(idx);
          }
        }
      }),
    );
  });

  /**
   * **Validates: Requirements 10.2**
   */
  it('different commit_ids map to different color indices', () => {
    fc.assert(
      fc.property(arbBlameLines, (lines) => {
        const colorMap = assignBlameColors(lines);
        const entries = [...colorMap.entries()];
        for (let i = 0; i < entries.length; i++) {
          for (let j = i + 1; j < entries.length; j++) {
            expect(entries[i][1]).not.toBe(entries[j][1]);
          }
        }
      }),
    );
  });

  /**
   * **Validates: Requirements 10.2**
   */
  it('every unique commit_id in the input appears in the output map', () => {
    fc.assert(
      fc.property(arbBlameLines, (lines) => {
        const colorMap = assignBlameColors(lines);
        const uniqueIds = new Set(lines.map((l) => l.commit_id));
        expect(colorMap.size).toBe(uniqueIds.size);
        for (const id of uniqueIds) {
          expect(colorMap.has(id)).toBe(true);
        }
      }),
    );
  });

  /**
   * **Validates: Requirements 10.2**
   */
  it('is deterministic: same input always produces the same output', () => {
    fc.assert(
      fc.property(arbBlameLines, (lines) => {
        const map1 = assignBlameColors(lines);
        const map2 = assignBlameColors(lines);
        expect([...map1.entries()]).toEqual([...map2.entries()]);
      }),
    );
  });

  /**
   * **Validates: Requirements 10.2**
   */
  it('empty input returns empty map', () => {
    const colorMap = assignBlameColors([]);
    expect(colorMap.size).toBe(0);
  });
});
