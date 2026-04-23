import type { DiffHunk } from '../../ipc/types';
import { DiffHunkView } from './DiffHunkView';

export interface InlineDiffViewProps {
  hunks: DiffHunk[];
  highlightedLines?: Map<string, Map<number, string>>;
  searchMatchLines?: Set<string>;
  currentMatchKey?: string;
}

export function InlineDiffView({
  hunks,
  searchMatchLines,
  currentMatchKey,
}: InlineDiffViewProps) {
  return (
    <div className="overflow-x-auto">
      {hunks.map((hunk, hi) => {
        // Build per-line search match sets for this hunk
        const hunkSearchMatches = new Set<number>();
        let currentLine: number | undefined;

        if (searchMatchLines) {
          for (let li = 0; li < hunk.lines.length; li++) {
            const key = `${hi}-${li}`;
            if (searchMatchLines.has(key)) hunkSearchMatches.add(li);
            if (currentMatchKey === key) currentLine = li;
          }
        }

        return (
          <DiffHunkView
            key={hi}
            hunk={hunk}
            hunkIndex={hi}
            searchMatches={hunkSearchMatches.size > 0 ? hunkSearchMatches : undefined}
            currentMatchLine={currentLine}
          />
        );
      })}
    </div>
  );
}
