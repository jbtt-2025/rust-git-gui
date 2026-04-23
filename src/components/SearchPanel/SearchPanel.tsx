import { useState, useCallback, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { CommitInfo } from '../../ipc/types';

export interface SearchPanelProps {
  onSearch: (query: string, options: SearchOptions) => void;
  results: CommitInfo[];
  loading: boolean;
  onSelectCommit: (commitId: string) => void;
  searchQuery: string;
}

export interface SearchOptions {
  author?: string;
  since?: string; // ISO date string
  until?: string; // ISO date string
  filePath?: string;
  commitHash?: string;
}

/** Highlight matching text in a string. */
function highlightMatch(text: string, query: string): (string | JSX.Element)[] {
  if (!query.trim()) return [text];
  const lower = text.toLowerCase();
  const qLower = query.toLowerCase();
  const parts: (string | JSX.Element)[] = [];
  let lastIdx = 0;
  let idx = lower.indexOf(qLower);
  let key = 0;

  while (idx >= 0) {
    if (idx > lastIdx) {
      parts.push(text.slice(lastIdx, idx));
    }
    parts.push(
      <mark key={key++} className="search-highlight">
        {text.slice(idx, idx + query.length)}
      </mark>,
    );
    lastIdx = idx + query.length;
    idx = lower.indexOf(qLower, lastIdx);
  }

  if (lastIdx < text.length) {
    parts.push(text.slice(lastIdx));
  }

  return parts;
}

function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleDateString();
}

export function SearchPanel({
  onSearch,
  results,
  loading,
  onSelectCommit,
  searchQuery,
}: SearchPanelProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [author, setAuthor] = useState('');
  const [since, setSince] = useState('');
  const [until, setUntil] = useState('');
  const [filePath, setFilePath] = useState('');
  const [commitHash, setCommitHash] = useState('');
  const debounceRef = useRef<ReturnType<typeof setTimeout>>();

  // Debounced search
  const triggerSearch = useCallback(
    (q: string, opts: SearchOptions) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        onSearch(q, opts);
      }, 300);
    },
    [onSearch],
  );

  const handleQueryChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const val = e.target.value;
      setQuery(val);
      triggerSearch(val, { author, since, until, filePath, commitHash });
    },
    [triggerSearch, author, since, until, filePath, commitHash],
  );

  const handleAdvancedSearch = useCallback(() => {
    triggerSearch(query, { author, since, until, filePath, commitHash });
  }, [triggerSearch, query, author, since, until, filePath, commitHash]);

  // Cleanup debounce on unmount
  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  return (
    <div className="search-panel" role="search" aria-label="Commit search">
      {/* Main search input */}
      <div className="search-panel-input-row">
        <input
          type="search"
          className="search-panel-input"
          placeholder={t('search.placeholder')}
          value={query}
          onChange={handleQueryChange}
          aria-label={t('search.placeholder')}
        />
        <button
          type="button"
          className="search-panel-advanced-toggle"
          onClick={() => setShowAdvanced(!showAdvanced)}
          aria-expanded={showAdvanced}
        >
          ▼
        </button>
      </div>

      {/* Advanced search fields */}
      {showAdvanced && (
        <div className="search-panel-advanced">
          <label className="search-panel-field">
            <span>{t('search.byAuthor')}</span>
            <input
              type="text"
              value={author}
              onChange={(e) => setAuthor(e.target.value)}
            />
          </label>
          <label className="search-panel-field">
            <span>{t('search.byDate')}</span>
            <div className="search-panel-date-range">
              <input
                type="date"
                value={since}
                onChange={(e) => setSince(e.target.value)}
                aria-label="From date"
              />
              <span>–</span>
              <input
                type="date"
                value={until}
                onChange={(e) => setUntil(e.target.value)}
                aria-label="To date"
              />
            </div>
          </label>
          <label className="search-panel-field">
            <span>{t('search.byFile')}</span>
            <input
              type="text"
              value={filePath}
              onChange={(e) => setFilePath(e.target.value)}
            />
          </label>
          <label className="search-panel-field">
            <span>{t('search.byHash')}</span>
            <input
              type="text"
              value={commitHash}
              onChange={(e) => setCommitHash(e.target.value)}
              className="code-font"
            />
          </label>
          <button
            type="button"
            className="search-panel-apply-btn"
            onClick={handleAdvancedSearch}
          >
            {t('search.placeholder')}
          </button>
        </div>
      )}

      {/* Results */}
      {loading && (
        <div className="search-panel-loading">{t('common.loading')}</div>
      )}

      {!loading && results.length > 0 && (
        <ul className="search-panel-results" role="list">
          {results.map((commit) => (
            <li
              key={commit.id}
              className="search-panel-result"
              role="listitem"
              onClick={() => onSelectCommit(commit.id)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') onSelectCommit(commit.id);
              }}
              tabIndex={0}
            >
              <div className="search-panel-result-header">
                <span className="search-panel-result-sha">{commit.short_id}</span>
                <span className="search-panel-result-author">{commit.author.name}</span>
                <span className="search-panel-result-date">
                  {formatDate(commit.author.timestamp)}
                </span>
              </div>
              <div className="search-panel-result-message">
                {highlightMatch(commit.message.split('\n')[0], searchQuery || query)}
              </div>
            </li>
          ))}
        </ul>
      )}

      {!loading && results.length === 0 && query.trim() && (
        <div className="search-panel-empty">No results found</div>
      )}
    </div>
  );
}
