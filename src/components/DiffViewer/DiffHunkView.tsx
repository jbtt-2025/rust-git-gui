import { useState } from 'react';
import type { DiffHunk, DiffLine } from '../../ipc/types';

export interface DiffHunkViewProps {
  hunk: DiffHunk;
  hunkIndex: number;
  highlightedLines?: Map<number, string>;
  searchMatches?: Set<number>;
  currentMatchLine?: number;
}

export function DiffHunkView({
  hunk,
  hunkIndex,
  highlightedLines,
  searchMatches,
  currentMatchLine,
}: DiffHunkViewProps) {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div className="border-b border-gray-700" data-hunk-index={hunkIndex}>
      {/* Hunk header */}
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

      {/* Hunk lines */}
      {!collapsed && (
        <table className="w-full text-xs font-mono border-collapse">
          <tbody>
            {hunk.lines.map((line, i) => (
              <DiffLineRow
                key={i}
                line={line}
                lineIndex={i}
                highlightedHtml={highlightedLines?.get(i)}
                isSearchMatch={searchMatches?.has(i)}
                isCurrentMatch={currentMatchLine === i}
              />
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

interface DiffLineRowProps {
  line: DiffLine;
  lineIndex: number;
  highlightedHtml?: string;
  isSearchMatch?: boolean;
  isCurrentMatch?: boolean;
}

function DiffLineRow({
  line,
  highlightedHtml,
  isSearchMatch,
  isCurrentMatch,
}: DiffLineRowProps) {
  const bgClass = lineBgClass(line.origin);
  const matchClass = isCurrentMatch
    ? 'ring-2 ring-yellow-400'
    : isSearchMatch
      ? 'bg-yellow-500/20'
      : '';

  return (
    <tr className={`${bgClass} ${matchClass}`}>
      <td className="w-12 text-right pr-2 text-gray-500 select-none border-r border-gray-700 align-top">
        {line.old_lineno ?? ''}
      </td>
      <td className="w-12 text-right pr-2 text-gray-500 select-none border-r border-gray-700 align-top">
        {line.new_lineno ?? ''}
      </td>
      <td className="w-6 text-center text-gray-500 select-none align-top">
        {originSymbol(line.origin)}
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

function lineBgClass(origin: string): string {
  switch (origin) {
    case 'Addition':
      return 'bg-green-900/30';
    case 'Deletion':
      return 'bg-red-900/30';
    default:
      return '';
  }
}

function originSymbol(origin: string): string {
  switch (origin) {
    case 'Addition':
      return '+';
    case 'Deletion':
      return '-';
    default:
      return ' ';
  }
}
