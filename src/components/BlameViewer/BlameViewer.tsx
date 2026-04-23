import { useState, useEffect, useMemo, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { BlameInfo, BlameLine } from '../../ipc/types';
import { assignBlameColors } from './blameColorMap';
import { BlameTimeline } from './BlameTimeline';
import { highlightCode } from '../DiffViewer/diffHighlight';

export interface BlameViewerProps {
  blame: BlameInfo;
  onNavigateToCommit?: (commitId: string) => void;
}

/** Tailwind background classes for blame gutter color-coding */
const GUTTER_PALETTE = [
  'bg-blue-900/30',
  'bg-green-900/30',
  'bg-purple-900/30',
  'bg-yellow-900/30',
  'bg-red-900/30',
  'bg-cyan-900/30',
  'bg-pink-900/30',
  'bg-orange-900/30',
  'bg-teal-900/30',
  'bg-indigo-900/30',
  'bg-lime-900/30',
  'bg-amber-900/30',
];

function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

function shortHash(commitId: string): string {
  return commitId.slice(0, 7);
}

export function BlameViewer({ blame, onNavigateToCommit }: BlameViewerProps) {
  const { t } = useTranslation();
  const [showTimeline, setShowTimeline] = useState(false);
  const [highlightedLines, setHighlightedLines] = useState<string[]>([]);

  const colorMap = useMemo(() => assignBlameColors(blame.lines), [blame.lines]);

  // Syntax highlighting
  useEffect(() => {
    const code = blame.lines.map((l) => l.content).join('\n');
    highlightCode(code, blame.path).then(setHighlightedLines).catch(() => {
      setHighlightedLines(blame.lines.map((l) => escapeHtml(l.content)));
    });
  }, [blame.lines, blame.path]);

  const handleCommitClick = useCallback(
    (commitId: string) => {
      onNavigateToCommit?.(commitId);
    },
    [onNavigateToCommit],
  );

  if (blame.lines.length === 0) {
    return (
      <div className="p-4 text-gray-400 text-sm">
        {t('blame.noData')}
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-1 bg-gray-800 border-b border-gray-700">
        <span className="text-xs text-gray-400 truncate flex-1">
          {blame.path}
        </span>
        <button
          className={`text-xs px-2 py-0.5 rounded ${
            showTimeline
              ? 'bg-blue-600 text-white'
              : 'text-gray-400 hover:text-white hover:bg-gray-700'
          }`}
          onClick={() => setShowTimeline((v) => !v)}
          aria-label={t('blame.toggleTimeline')}
        >
          {t('blame.timeline')}
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto flex">
        {/* Timeline panel */}
        {showTimeline && (
          <div className="w-24 flex-shrink-0 border-r border-gray-700 overflow-hidden">
            <BlameTimeline lines={blame.lines} colorMap={colorMap} />
          </div>
        )}

        {/* Blame table */}
        <div className="flex-1 overflow-auto">
          <table className="w-full text-xs font-mono border-collapse">
            <tbody>
              {blame.lines.map((line, idx) => {
                const colorIndex = colorMap.get(line.commit_id) ?? 0;
                const bgClass = GUTTER_PALETTE[colorIndex % GUTTER_PALETTE.length];
                const htmlContent = highlightedLines[idx] ?? escapeHtml(line.content);

                return (
                  <BlameLineRow
                    key={line.line_number}
                    line={line}
                    bgClass={bgClass}
                    htmlContent={htmlContent}
                    onCommitClick={handleCommitClick}
                  />
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

interface BlameLineRowProps {
  line: BlameLine;
  bgClass: string;
  htmlContent: string;
  onCommitClick: (commitId: string) => void;
}

function BlameLineRow({ line, bgClass, htmlContent, onCommitClick }: BlameLineRowProps) {
  return (
    <tr className={`${bgClass} hover:bg-gray-700/50`}>
      {/* Commit hash */}
      <td className="px-2 py-0 whitespace-nowrap text-right">
        <button
          className="text-blue-400 hover:text-blue-300 hover:underline cursor-pointer"
          onClick={() => onCommitClick(line.commit_id)}
          title={line.commit_id}
          aria-label={`Navigate to commit ${shortHash(line.commit_id)}`}
        >
          {shortHash(line.commit_id)}
        </button>
      </td>
      {/* Author */}
      <td className="px-2 py-0 whitespace-nowrap text-gray-400 max-w-[120px] truncate">
        {line.author}
      </td>
      {/* Date */}
      <td className="px-2 py-0 whitespace-nowrap text-gray-500">
        {formatDate(line.date)}
      </td>
      {/* Line number */}
      <td className="px-2 py-0 whitespace-nowrap text-gray-500 text-right select-none border-r border-gray-700">
        {line.line_number}
      </td>
      {/* Code content with syntax highlighting */}
      <td className="px-2 py-0 whitespace-pre">
        <span dangerouslySetInnerHTML={{ __html: htmlContent }} />
      </td>
    </tr>
  );
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}
