import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { gitApi } from '../../ipc/client';
import { useRepoStore } from '../../stores/repoStore';
import type { TabState } from '../../stores/repoStore';
import type { RepositoryState } from '../../ipc/types';
import { ActionBar } from './ActionBar';
import { RepoSearch } from './RepoSearch';
import { RecentReposList } from './RecentReposList';
import { CloneDialog } from './CloneDialog';
import { filterRepos } from './filterRepos';

/**
 * Welcome/landing page displayed when no repository tabs are open.
 * Composes ActionBar, RepoSearch, RecentReposList, and CloneDialog.
 * Manages local state for search query, clone dialog visibility, and error messages.
 */
export function WelcomePage() {
  const { t } = useTranslation();
  const recentRepos = useRepoStore((s) => s.recentRepos);
  const setRecentRepos = useRepoStore((s) => s.setRecentRepos);
  const addTab = useRepoStore((s) => s.addTab);

  const [searchQuery, setSearchQuery] = useState('');
  const [cloneDialogOpen, setCloneDialogOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [missingRepoPath, setMissingRepoPath] = useState<string | null>(null);

  // Load recent repos on mount
  useEffect(() => {
    const loadRecent = async () => {
      try {
        const repos = await gitApi.loadRecentRepos();
        setRecentRepos(repos);
      } catch {
        // Degrade gracefully — use empty list
      }
    };
    loadRecent();
  }, [setRecentRepos]);

  const clearError = useCallback(() => {
    setError(null);
    setMissingRepoPath(null);
  }, []);

  // Helper: create a TabState from an opened/cloned/inited repo
  const createTabAndSave = useCallback(
    async (tabId: string, repoPath: string) => {
      const repoName = repoPath.split(/[\\/]/).pop() ?? repoPath;
      const defaultRepoState: RepositoryState = { type: 'Clean' };
      const tab: TabState = {
        tabId,
        repoName,
        repoPath,
        hasChanges: false,
        repoState: defaultRepoState,
        soloedBranches: new Set(),
        hiddenBranches: new Set(),
        pinnedLeftBranches: [],
      };
      addTab(tab);

      // Persist recent repos
      try {
        await gitApi.saveRecentRepos();
        const repos = await gitApi.loadRecentRepos();
        setRecentRepos(repos);
      } catch {
        // Silent — persistence failure is non-blocking
      }
    },
    [addTab, setRecentRepos],
  );

  // Handle "Open" button — native folder picker → openRepository
  const handleOpen = useCallback(async () => {
    clearError();
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected) return; // Dialog cancelled

      const path = selected as string;
      try {
        const tabId = await gitApi.openRepository(path);
        await createTabAndSave(tabId, path);
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        if (message.toLowerCase().includes('not a git') || message.toLowerCase().includes('not a repository')) {
          setError(t('welcome.notGitRepo'));
        } else {
          setError(message);
        }
      }
    } catch {
      // Dialog plugin error — silent
    }
  }, [clearError, createTabAndSave, t]);

  // Handle "Create" button — native folder picker → initRepository
  const handleInit = useCallback(async () => {
    clearError();
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected) return; // Dialog cancelled

      const path = selected as string;
      try {
        const tabId = await gitApi.initRepository(path);
        await createTabAndSave(tabId, path);
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        // Check if directory is already a git repo
        if (
          message.toLowerCase().includes('already') &&
          (message.toLowerCase().includes('git') || message.toLowerCase().includes('repository'))
        ) {
          // Offer to open the existing repo instead
          const shouldOpen = window.confirm(t('welcome.alreadyGitRepo'));
          if (shouldOpen) {
            try {
              const tabId = await gitApi.openRepository(path);
              await createTabAndSave(tabId, path);
            } catch (openErr: unknown) {
              const openMessage = openErr instanceof Error ? openErr.message : String(openErr);
              setError(openMessage);
            }
          }
        } else {
          setError(message);
        }
      }
    } catch {
      // Dialog plugin error — silent
    }
  }, [clearError, createTabAndSave, t]);

  // Handle clone from CloneDialog
  const handleClone = useCallback(
    async (url: string, path: string, recursive: boolean) => {
      clearError();
      const tabId = await gitApi.cloneRepository(url, path, recursive);
      await createTabAndSave(tabId, path);
      setCloneDialogOpen(false);
    },
    [clearError, createTabAndSave],
  );

  // Handle selecting a recent repo
  const handleSelectRepo = useCallback(
    async (path: string) => {
      clearError();
      try {
        const tabId = await gitApi.openRepository(path);
        await createTabAndSave(tabId, path);
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        if (
          message.toLowerCase().includes('not found') ||
          message.toLowerCase().includes('no such') ||
          message.toLowerCase().includes('does not exist') ||
          message.toLowerCase().includes('path')
        ) {
          setError(t('welcome.pathNotFound'));
          setMissingRepoPath(path);
        } else if (
          message.toLowerCase().includes('not a git') ||
          message.toLowerCase().includes('not a repository')
        ) {
          setError(t('welcome.notGitRepo'));
        } else {
          setError(message);
        }
      }
    },
    [clearError, createTabAndSave, t],
  );

  // Handle removing a recent repo from the list
  const handleRemoveRepo = useCallback(
    async (path: string) => {
      try {
        await gitApi.removeRecentRepo(path);
        const repos = await gitApi.loadRecentRepos();
        setRecentRepos(repos);
      } catch {
        // Silent
      }
      // Clear error if we just removed the missing repo
      if (missingRepoPath === path) {
        clearError();
      }
    },
    [setRecentRepos, missingRepoPath, clearError],
  );

  // Handle copying a repo path to clipboard
  const handleCopyPath = useCallback(async (path: string) => {
    try {
      await navigator.clipboard.writeText(path);
    } catch {
      // Clipboard API not available — silent
    }
  }, []);

  const handleSearchChange = useCallback((value: string) => {
    setSearchQuery(value);
  }, []);

  const handleSearchClear = useCallback(() => {
    setSearchQuery('');
  }, []);

  const filteredRepos = filterRepos(recentRepos ?? [], searchQuery);

  return (
    <div
      role="main"
      aria-label={t('welcome.title')}
      className="flex h-full flex-col items-center justify-start overflow-y-auto bg-neutral-900 px-8 py-12"
    >
      {/* App name and icon */}
      <div className="mb-8 flex flex-col items-center gap-2">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          className="h-12 w-12 text-blue-500"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={1.5}
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <circle cx="12" cy="12" r="4" />
          <line x1="1.05" y1="12" x2="7" y2="12" />
          <line x1="17.01" y1="12" x2="22.96" y2="12" />
        </svg>
        <h1 className="text-2xl font-bold text-neutral-200">
          {t('welcome.title')}
        </h1>
        <p className="text-sm text-neutral-500">
          {t('welcome.subtitle')}
        </p>
      </div>

      {/* Action buttons */}
      <ActionBar
        onOpen={handleOpen}
        onClone={() => setCloneDialogOpen(true)}
        onInit={handleInit}
      />

      {/* Error message */}
      {error && (
        <div
          className="mt-4 flex w-full max-w-lg items-center gap-2 rounded-md border border-red-500/30 bg-red-500/10 px-4 py-2 text-sm text-red-400"
          role="alert"
        >
          <span className="flex-1">{error}</span>
          {missingRepoPath && (
            <button
              type="button"
              onClick={() => handleRemoveRepo(missingRepoPath)}
              className="whitespace-nowrap rounded bg-red-500/20 px-2 py-1 text-xs font-medium text-red-300 hover:bg-red-500/30 focus:outline-none focus:ring-2 focus:ring-red-500"
            >
              {t('welcome.removeFromList')}
            </button>
          )}
          <button
            type="button"
            onClick={clearError}
            className="ml-1 text-red-400 hover:text-red-300 focus:outline-none focus:ring-2 focus:ring-red-500 rounded"
            aria-label={t('common.close')}
          >
            ×
          </button>
        </div>
      )}

      {/* Recent repos section */}
      <div className="mt-8 w-full max-w-lg">
        <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-neutral-400">
          {t('welcome.recentRepos')}
        </h2>

        <RepoSearch
          value={searchQuery}
          onChange={handleSearchChange}
          onClear={handleSearchClear}
        />

        <div className="mt-3">
          <RecentReposList
            repos={filteredRepos}
            searchQuery={searchQuery}
            onSelect={handleSelectRepo}
            onRemove={handleRemoveRepo}
            onCopyPath={handleCopyPath}
          />
        </div>
      </div>

      {/* Clone dialog */}
      <CloneDialog
        open={cloneDialogOpen}
        onOpenChange={setCloneDialogOpen}
        onClone={handleClone}
      />
    </div>
  );
}
