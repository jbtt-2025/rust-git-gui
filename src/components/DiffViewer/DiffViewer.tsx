import { useState, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { FileDiff } from '../../ipc/types';
import { SplitDiffView } from './SplitDiffView';
import { InlineDiffView } from './InlineDiffView';
import { ImageDiff } from './ImageDiff';
import { findAllMatches } from './diffSearch';

export interface DiffViewerProps {
  diff: FileDiff;
  mode: 'split' | 'inline';
}

const IMAGE_EXTENSIONS = new Set([
  'png', 'jpg', 'jpeg', 'gif', 'bmp', 'svg', 'webp', 'ico',
]);

function isImageFile(path: string): boolean {
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  return IMAGE_EXTENSIONS.has(ext);
}

export function DiffViewer({ diff, mode }: DiffViewerProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const [searchOpen, setSearchOpen] = useState(false);
  const [currentMatchIndex, setCurrentMatchIndex] = useState(0);

  // Collect all line contents for search
  const allLineEntries = useMemo(() => {
    const entries: { key: string; content: string; hunkIndex: number; lineIndex: number }[] = [];
    diff.hunks.forEach((hunk, hi) => {
      hunk.lines.forEach((line, li) => {
        entries.push({
          key: mode === 'split' ? `${hi}-new-${li}` : `${hi}-${li}`,
          content: line.content,
          hunkIndex: hi,
          lineIndex: li,
        });
      });
    });
    return entries;
  }, [diff.hunks, mode]);

  // Find all matching line keys
  const matchKeys = useMemo(() => {
    if (!searchQuery) return [];
    const keys: string[] = [];
    for (const entry of allLineEntries) {
      const matches = findAllMatches(entry.content, searchQuery);
      if (matches.length > 0) {
        keys.push(entry.key);
      }
    }
    return keys;
  }, [allLineEntries, searchQuery]);

  const matchSet = useMemo(() => new Set(matchKeys), [matchKeys]);
  const currentMatchKey = matchKeys[currentMatchIndex] ?? undefined;

  const goToNext = useCallback(() => {
    setCurrentMatchIndex((i) => (matchKeys.length > 0 ? (i + 1) % matchKeys.length : 0));
  }, [matchKeys.length]);

  const goToPrev = useCallback(() => {
    setCurrentMatchIndex((i) => (matchKeys.length > 0 ? (i - 1 + matchKeys.length) % matchKeys.length : 0));
  }, [matchKeys.length]);

  const toggleSearch = useCallback(() => {
    setSearchOpen((o) => !o);
    if (searchOpen) {
      setSearchQuery('');
      setCurrentMatchIndex(0);
    }
  }, [searchOpen]);

  // Handle binary / image files
  if (diff.is_binary && isImageFile(diff.path)) {
    return (
      <ImageDiff
        path={diff.path}
        oldPath={diff.old_path}
        status={diff.status}
      />
    );
  }

  if (diff.is_binary) {
    return (
      <div className="p-4 text-gray-400 text-sm">
        Binary file changed
      </div>
    );
  }

  if (diff.hunks.length === 0) {
    return (
      <div className="p-4 text-gray-400 text-sm">
        {t('diff.noChanges')}
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Search bar */}
      <div className="flex items-center gap-2 px-3 py-1 bg-gray-800 border-b border-gray-700">
        <span className="text-xs text-gray-400 truncate flex-1">
          {diff.path}
          {diff.old_path && diff.old_path !== diff.path && (
            <span className="text-gray-500"> ← {diff.old_path}</span>
          )}
        </span>
        <button
          className="text-xs text-gray-400 hover:text-white px-2 py-0.5 rounded hover:bg-gray-700"
          onClick={toggleSearch}
          aria-label="Toggle search"
        >
          🔍
        </button>
      </div>

      {searchOpen && (
        <div className="flex items-center gap-2 px-3 py-1 bg-gray-800 border-b border-gray-700">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => {
              setSearchQuery(e.target.value);
              setCurrentMatchIndex(0);
            }}
            placeholder="Find in diff..."
            className="flex-1 bg-gray-900 text-white text-xs px-2 py-1 rounded border border-gray-600 focus:border-blue-500 outline-none"
            autoFocus
          />
          <span className="text-xs text-gray-400 min-w-[60px] text-center">
            {matchKeys.length > 0
              ? `${currentMatchIndex + 1}/${matchKeys.length}`
              : searchQuery
                ? '0/0'
                : ''}
          </span>
          <button
            className="text-xs text-gray-400 hover:text-white px-1"
            onClick={goToPrev}
            disabled={matchKeys.length === 0}
            aria-label="Previous match"
          >
            ▲
          </button>
          <button
            className="text-xs text-gray-400 hover:text-white px-1"
            onClick={goToNext}
            disabled={matchKeys.length === 0}
            aria-label="Next match"
          >
            ▼
          </button>
        </div>
      )}

      {/* Diff content */}
      <div className="flex-1 overflow-auto">
        {mode === 'split' ? (
          <SplitDiffView
            hunks={diff.hunks}
            searchMatchLines={matchSet.size > 0 ? matchSet : undefined}
            currentMatchKey={currentMatchKey}
          />
        ) : (
          <InlineDiffView
            hunks={diff.hunks}
            searchMatchLines={matchSet.size > 0 ? matchSet : undefined}
            currentMatchKey={currentMatchKey}
          />
        )}
      </div>
    </div>
  );
}
