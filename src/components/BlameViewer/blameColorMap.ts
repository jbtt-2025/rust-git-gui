import type { BlameLine } from '../../ipc/types';

/**
 * Assigns a unique color index to each unique commit_id found in the blame lines.
 * Same commit_id → same color index, different commit_id → different color index.
 *
 * This is a pure function suitable for property-based testing.
 */
export function assignBlameColors(lines: BlameLine[]): Map<string, number> {
  const colorMap = new Map<string, number>();
  let nextIndex = 0;

  for (const line of lines) {
    if (!colorMap.has(line.commit_id)) {
      colorMap.set(line.commit_id, nextIndex);
      nextIndex++;
    }
  }

  return colorMap;
}
