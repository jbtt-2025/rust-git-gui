import { useState } from 'react';
import type { DiffHunk, DiffLine } from '../../ipc/types';

export interface SplitDiffViewProps {
  hunks: DiffHunk[];
  highlightedLines?: Map<string, string>;
  searchMatchLines?: Set<string>;
  currentMatchKey?: string;
}

export function SplitDiffView({
  hunks,
  highlightedLines,
  searchMatchLines,
  currentMatchKey,
}: SplitDiffViewProps) {
  return (
    <div className="overflow-x-auto">
      {hunks.map((hunk, hi) => (
        <SplitHunk
          key={hi}
          hunk={hunk}
          hunkIndex={hi}
          highlightedLines={highlightedLines}
          searchMatchLines={searchMatchLines}
          currentMatchKey={currentMatchKey}
        />
      ))}
    </div>
  );
}

interface SplitHunkProps {
  hunk: DiffHunk;
  hunkIndex: number;
  highlightedLines?: Map<string, string>;
  searchMatchLines?: Set<string>;
  currentMatchKey?: string;
}

function SplitHunk({
  hunk,
  hunkIndex,
  highlightedLines,
  searchMatchLines,
  currentMatchKey,
}: SplitHunkProps) {
  const [collapsed, setCollapsed] = useState(false);
  const rows = buildSplitRows(hunk.lines);

  return (
    <div className="border-b border-gray-700">
      <div
        className="flex items-center gap-2 px-3 py-1 bg-blue-900/30 text-blue-300 text-xs font-mono cursor-pointer select-none hover:bg-blue-900/50"
        onClick={() => setCollapsed((c) => !c)}
        role="button"
        aria-expanded={!collapsed}
        aria-label={`Hunk: ${hunk.header}`}
      >
        <span className="text-gray-400">{collapsed ? '▶' : '▼'}</span>
        <span>{hunk.header}</span>
      </div>

      {!collapsed && (
        <div className="grid grid-cols-2 text-xs font-mono">
          {/* Left (old) and Right (new) side */}
          <table className="w-full border-collapse border-r border-gray-700">
            <tbody>
              {rows.map((row, i) => {
                const key = `${hunkIndex}-old-${i}`;
                const isMatch = searchMatchLines?.has(key);
                const isCurrent = currentMatchKey === key;
                return (
                  <SplitLineRow
                    key={i}
                    line={row.old}
                    lineNo={row.old?.old_lineno ?? null}
                    side="old"
                    highlightedHtml={highlightedLines?.get(key)}
                    isSearchMatch={isMatch}
                    isCurrentMatch={isCurrent}
                  />
                );
              })}
            </tbody>
          </table>
          <table className="w-full border-collapse">
            <tbody>
              {rows.map((row, i) => {
                const key = `${hunkIndex}-new-${i}`;
                const isMatch = searchMatchLines?.has(key);
                const isCurrent = currentMatchKey === key;
                return (
                  <SplitLineRow
                    key={i}
                    line={row.new}
                    lineNo={row.new?.new_lineno ?? null}
                    side="new"
                    highlightedHtml={highlightedLines?.get(key)}
                    isSearchMatch={isMatch}
                    isCurrentMatch={isCurrent}
                  />
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

interface SplitRow {
  old: DiffLine | null;
  new: DiffLine | null;
}

/** Pair up old/new lines for split view */
function buildSplitRows(lines: DiffLine[]): SplitRow[] {
  const rows: SplitRow[] = [];
  const deletions: DiffLine[] = [];
  const additions: DiffLine[] = [];

  const flushPairs = () => {
    const max = Math.max(deletions.length, additions.length);
    for (let i = 0; i < max; i++) {
      rows.push({
        old: deletions[i] ?? null,
        new: additions[i] ?? null,
      });
    }
    deletions.length = 0;
    additions.length = 0;
  };

  for (const line of lines) {
    if (line.origin === 'Deletion') {
      deletions.push(line);
    } else if (line.origin === 'Addition') {
      additions.push(line);
    } else {
      flushPairs();
      rows.push({ old: line, new: line });
    }
  }
  flushPairs();

  return rows;
}

interface SplitLineRowProps {
  line: DiffLine | null;
  lineNo: number | null;
  side: 'old' | 'new';
  highlightedHtml?: string;
  isSearchMatch?: boolean;
  isCurrentMatch?: boolean;
}

function SplitLineRow({
  line,
  lineNo,
  highlightedHtml,
  isSearchMatch,
  isCurrentMatch,
}: SplitLineRowProps) {
  if (!line) {
    return (
      <tr className="bg-gray-800/50">
        <td className="w-10 text-right pr-2 text-gray-600 select-none border-r border-gray-700">&nbsp;</td>
        <td className="pl-2">&nbsp;</td>
      </tr>
    );
  }

  const bgClass = line.origin === 'Addition'
    ? 'bg-green-900/30'
    : line.origin === 'Deletion'
      ? 'bg-red-900/30'
      : '';

  const matchClass = isCurrentMatch
    ? 'ring-2 ring-yellow-400'
    : isSearchMatch
      ? 'bg-yellow-500/20'
      : '';

  return (
    <tr className={`${bgClass} ${matchClass}`}>
      <td className="w-10 text-right pr-2 text-gray-500 select-none border-r border-gray-700 align-top">
        {lineNo ?? ''}
      </td>
      <td className="pl-2 whitespace-pre-wrap break-all">
        {highlightedHtml ? (
          <span dangerouslySetInnerHTML={{ __html: highlightedHtml }} />
        ) : (
          line.content
        )}
      </td>
    </tr>
  );
}
