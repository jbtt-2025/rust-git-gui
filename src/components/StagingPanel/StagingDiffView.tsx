import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { FileDiff, DiffHunk, DiffLine, LineRange } from '../../ipc/types';

export interface StagingDiffViewProps {
  diff: FileDiff;
  staged: boolean;
  onStageLines: (path: string, ranges: LineRange[]) => void;
  onUnstageLines: (path: string, ranges: LineRange[]) => void;
  onDiscardLines: (path: string, ranges: LineRange[]) => void;
}

export function StagingDiffView({
  diff,
  staged,
  onStageLines,
  onUnstageLines,
  onDiscardLines,
}: StagingDiffViewProps) {
  const { t } = useTranslation();
  const [selectedLines, setSelectedLines] = useState<Set<string>>(new Set());

  const toggleLine = useCallback((key: string) => {
    setSelectedLines((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }, []);

  // Build line ranges from selected lines for staging operations
  const getSelectedRanges = useCallback((): LineRange[] => {
    const lineNumbers: number[] = [];
    for (const key of selectedLines) {
      const num = parseInt(key, 10);
      if (!isNaN(num)) lineNumbers.push(num);
    }
    if (lineNumbers.length === 0) return [];
    lineNumbers.sort((a, b) => a - b);

    const ranges: LineRange[] = [];
    let start = lineNumbers[0];
    let end = lineNumbers[0];
    for (let i = 1; i < lineNumbers.length; i++) {
      if (lineNumbers[i] === end + 1) {
        end = lineNumbers[i];
      } else {
        ranges.push({ start, end });
        start = lineNumbers[i];
        end = lineNumbers[i];
      }
    }
    ranges.push({ start, end });
    return ranges;
  }, [selectedLines]);

  const handleStageSelected = useCallback(() => {
    const ranges = getSelectedRanges();
    if (ranges.length === 0) return;
    onStageLines(diff.path, ranges);
    setSelectedLines(new Set());
  }, [getSelectedRanges, onStageLines, diff.path]);

  const handleUnstageSelected = useCallback(() => {
    const ranges = getSelectedRanges();
    if (ranges.length === 0) return;
    onUnstageLines(diff.path, ranges);
    setSelectedLines(new Set());
  }, [getSelectedRanges, onUnstageLines, diff.path]);

  const handleDiscardSelected = useCallback(() => {
    const ranges = getSelectedRanges();
    if (ranges.length === 0) return;
    onDiscardLines(diff.path, ranges);
    setSelectedLines(new Set());
  }, [getSelectedRanges, onDiscardLines, diff.path]);

  // Handle keyboard shortcuts for staging/discard on selected lines
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (selectedLines.size === 0) return;
      // Ctrl+S / Cmd+S to stage selected lines
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        if (staged) handleUnstageSelected();
        else handleStageSelected();
      }
      // Ctrl+Z / Cmd+Z to discard selected lines
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
        e.preventDefault();
        handleDiscardSelected();
      }
    },
    [selectedLines, staged, handleStageSelected, handleUnstageSelected, handleDiscardSelected],
  );

  if (diff.is_binary) {
    return (
      <div className="p-4 text-gray-400 text-sm">Binary file changed</div>
    );
  }

  if (diff.hunks.length === 0) {
    return (
      <div className="p-4 text-gray-400 text-sm">{t('diff.noChanges')}</div>
    );
  }

  return (
    <div className="flex flex-col h-full" onKeyDown={handleKeyDown} tabIndex={0}>
      {/* Toolbar for selected lines */}
      {selectedLines.size > 0 && (
        <div className="flex items-center gap-2 px-3 py-1 bg-gray-800 border-b border-gray-700">
          <span className="text-xs text-gray-400">
            {selectedLines.size} line(s) selected
          </span>
          {staged ? (
            <button
              className="text-xs px-2 py-0.5 rounded bg-yellow-600 hover:bg-yellow-500 text-white"
              onClick={handleUnstageSelected}
            >
              {t('staging.unstage')}
            </button>
          ) : (
            <button
              className="text-xs px-2 py-0.5 rounded bg-green-600 hover:bg-green-500 text-white"
              onClick={handleStageSelected}
            >
              {t('staging.stage')}
            </button>
          )}
          {!staged && (
            <button
              className="text-xs px-2 py-0.5 rounded bg-red-600 hover:bg-red-500 text-white"
              onClick={handleDiscardSelected}
            >
              {t('staging.discard')}
            </button>
          )}
        </div>
      )}

      {/* Diff content with line-level checkboxes */}
      <div className="flex-1 overflow-auto">
        {diff.hunks.map((hunk, hi) => (
          <StagingHunkView
            key={hi}
            hunk={hunk}
            hunkIndex={hi}
            staged={staged}
            selectedLines={selectedLines}
            onToggleLine={toggleLine}
            onStageLines={onStageLines}
            onUnstageLines={onUnstageLines}
            filePath={diff.path}
          />
        ))}
      </div>
    </div>
  );
}

interface StagingHunkViewProps {
  hunk: DiffHunk;
  hunkIndex: number;
  staged: boolean;
  selectedLines: Set<string>;
  onToggleLine: (key: string) => void;
  onStageLines: (path: string, ranges: LineRange[]) => void;
  onUnstageLines: (path: string, ranges: LineRange[]) => void;
  filePath: string;
}

function StagingHunkView({
  hunk,
  hunkIndex,
  staged,
  selectedLines,
  onToggleLine,
  onStageLines,
  onUnstageLines,
  filePath,
}: StagingHunkViewProps) {
  const [collapsed, setCollapsed] = useState(false);

  const handleLineCheckbox = (line: DiffLine) => {
    const lineNo = line.new_lineno ?? line.old_lineno;
    if (lineNo == null) return;
    // Single-line stage/unstage via checkbox
    const range: LineRange = { start: lineNo, end: lineNo };
    if (staged) {
      onUnstageLines(filePath, [range]);
    } else {
      onStageLines(filePath, [range]);
    }
  };

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
        <table className="w-full text-xs font-mono border-collapse">
          <tbody>
            {hunk.lines.map((line, li) => {
              const isChangeLine = line.origin === 'Addition' || line.origin === 'Deletion';
              const lineNo = line.new_lineno ?? line.old_lineno;
              const lineKey = lineNo != null ? String(lineNo) : `${hunkIndex}-${li}`;
              const isSelected = selectedLines.has(lineKey);

              const bgClass =
                line.origin === 'Addition'
                  ? 'bg-green-900/30'
                  : line.origin === 'Deletion'
                    ? 'bg-red-900/30'
                    : '';

              return (
                <tr
                  key={li}
                  className={`${bgClass} ${isSelected ? 'ring-1 ring-blue-400' : ''}`}
                  onClick={() => isChangeLine && onToggleLine(lineKey)}
                >
                  {/* Checkbox column for change lines */}
                  <td className="w-6 text-center align-top">
                    {isChangeLine && (
                      <input
                        type="checkbox"
                        className="accent-blue-500 cursor-pointer"
                        checked={isSelected}
                        onChange={() => handleLineCheckbox(line)}
                        onClick={(e) => e.stopPropagation()}
                        aria-label={`${staged ? 'Unstage' : 'Stage'} line ${lineNo}`}
                      />
                    )}
                  </td>
                  <td className="w-12 text-right pr-2 text-gray-500 select-none border-r border-gray-700 align-top">
                    {line.old_lineno ?? ''}
                  </td>
                  <td className="w-12 text-right pr-2 text-gray-500 select-none border-r border-gray-700 align-top">
                    {line.new_lineno ?? ''}
                  </td>
                  <td className="w-6 text-center text-gray-500 select-none align-top">
                    {line.origin === 'Addition' ? '+' : line.origin === 'Deletion' ? '-' : ' '}
                  </td>
                  <td className="pl-2 whitespace-pre-wrap break-all">{line.content}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}
