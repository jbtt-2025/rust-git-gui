import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import * as ContextMenu from '@radix-ui/react-context-menu';
import type { RepoEntry } from '../../ipc/types';

export interface RecentReposListProps {
  repos: RepoEntry[];
  searchQuery: string;
  onSelect: (path: string) => void;
  onRemove: (path: string) => void;
  onCopyPath: (path: string) => void;
}

/**
 * Displays a list of recent repositories with right-click context menu support.
 * Shows empty state guidance when no repos exist, or "no results" when search
 * yields nothing. Each item is keyboard-navigable with Enter to open.
 */
export function RecentReposList({
  repos,
  searchQuery,
  onSelect,
  onRemove,
  onCopyPath,
}: RecentReposListProps) {
  const { t } = useTranslation();

  const handleKeyDown = useCallback(
    (path: string, e: React.KeyboardEvent<HTMLDivElement>) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        onSelect(path);
      }
    },
    [onSelect],
  );

  if (repos.length === 0) {
    const isSearching = searchQuery.trim().length > 0;
    return (
      <div className="py-8 text-center text-sm text-neutral-500">
        {isSearching
          ? t('welcome.noSearchResults')
          : t('welcome.noRecentRepos')}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1" role="list" aria-label={t('welcome.recentRepos')}>
      {repos.map((repo) => (
        <ContextMenu.Root key={repo.path}>
          <ContextMenu.Trigger asChild>
            <div
              role="listitem"
              tabIndex={0}
              aria-label={`${repo.name} — ${repo.path}`}
              className="flex cursor-pointer flex-col rounded-md px-3 py-2 hover:bg-neutral-700 focus:bg-neutral-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
              onClick={() => onSelect(repo.path)}
              onKeyDown={(e) => handleKeyDown(repo.path, e)}
            >
              <span className="text-sm font-medium text-neutral-200">
                {repo.name}
              </span>
              <span className="truncate text-xs text-neutral-500">
                {repo.path}
              </span>
            </div>
          </ContextMenu.Trigger>

          <ContextMenu.Portal>
            <ContextMenu.Content className="min-w-[160px] rounded-md border border-neutral-600 bg-neutral-800 p-1 shadow-lg">
              <ContextMenu.Item
                className="cursor-pointer rounded px-3 py-1.5 text-sm text-neutral-200 outline-none hover:bg-neutral-700 focus:bg-neutral-700"
                onSelect={() => onSelect(repo.path)}
              >
                {t('welcome.openRepo')}
              </ContextMenu.Item>
              <ContextMenu.Item
                className="cursor-pointer rounded px-3 py-1.5 text-sm text-neutral-200 outline-none hover:bg-neutral-700 focus:bg-neutral-700"
                onSelect={() => onRemove(repo.path)}
              >
                {t('welcome.removeFromList')}
              </ContextMenu.Item>
              <ContextMenu.Item
                className="cursor-pointer rounded px-3 py-1.5 text-sm text-neutral-200 outline-none hover:bg-neutral-700 focus:bg-neutral-700"
                onSelect={() => onCopyPath(repo.path)}
              >
                {t('welcome.copyPath')}
              </ContextMenu.Item>
            </ContextMenu.Content>
          </ContextMenu.Portal>
        </ContextMenu.Root>
      ))}
    </div>
  );
}
