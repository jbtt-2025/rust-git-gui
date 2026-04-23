/**
 * Property 10: 文件状态图标唯一性
 *
 * For any FileStatusType enum value, the status-to-icon mapping function
 * SHALL produce a different icon identifier for each different status.
 *
 * **Validates: Requirements 4.7**
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import type { FileStatusType } from '../../../ipc/types';
import { getFileStatusIcon } from '../statusIconMap';

const allStatuses: FileStatusType[] = [
  'Untracked',
  'Modified',
  'Staged',
  'Conflict',
  'Deleted',
  'Renamed',
];

const arbFileStatusType: fc.Arbitrary<FileStatusType> = fc.constantFrom(...allStatuses);

describe('Property 10: File status icon uniqueness', () => {
  it('any two different FileStatusType values map to different icons', () => {
    fc.assert(
      fc.property(arbFileStatusType, arbFileStatusType, (statusA, statusB) => {
        if (statusA !== statusB) {
          const iconA = getFileStatusIcon(statusA);
          const iconB = getFileStatusIcon(statusB);
          expect(iconA.icon).not.toBe(iconB.icon);
        }
      }),
    );
  });

  it('every FileStatusType produces a non-empty icon, color, and label', () => {
    fc.assert(
      fc.property(arbFileStatusType, (status) => {
        const info = getFileStatusIcon(status);
        expect(info.icon.length).toBeGreaterThan(0);
        expect(info.color.length).toBeGreaterThan(0);
        expect(info.label.length).toBeGreaterThan(0);
      }),
    );
  });

  it('getFileStatusIcon is deterministic (same input always returns same result)', () => {
    fc.assert(
      fc.property(arbFileStatusType, (status) => {
        const first = getFileStatusIcon(status);
        const second = getFileStatusIcon(status);
        expect(first).toEqual(second);
      }),
    );
  });
});
