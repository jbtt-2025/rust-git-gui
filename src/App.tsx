import { useEffect, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useRepoStore, useUiStore, useSettingsStore } from './stores';
import type { TabState } from './stores';
import { applyTheme, applyFont, watchSystemTheme } from './themes';
import { gitApi, onFileChanged, onProgress } from './ipc';
import type {
  TabId,
  CommitInfo,
  CommitDetail,
  DagLayout,
  FileDiff,
  FileStatus,
  BlameInfo,
  BranchInfo,
  StashEntry,
  SubmoduleInfo,
  WorktreeInfo,
  TagInfo,
  PullRequest,
  CreatePrParams,
  LineRange,
  LogOptions,
} from './ipc/types';
import type { HostPlatform } from './components/Sidebar/HostIntegrationSection';
import type { SearchOptions } from './components/SearchPanel/SearchPanel';
import {
  TabBar,
  Toolbar,
  Sidebar,
  CommitGraph,
  CommitContextMenu,
  SearchPanel,
  DiffViewer,
  StagingPanel,
  CommitEditor,
  BlameViewer,
  ConflictResolver,
  TreeBrowser,
  TerminalPanel,
  SettingsPanel,
  BranchDragProvider,
} from './components';
import { useHotkeys, useSubmoduleUpdates } from './hooks';

type MainView = 'graph' | 'diff' | 'blame' | 'tree';

function App() {
  const { t } = useTranslation();
  const theme = useUiStore((s) => s.theme);
  const terminalVisible = useUiStore((s) => s.terminalVisible);
  const diffViewMode = useUiStore((s) => s.diffViewMode);
  const toggleTerminal = useUiStore((s) => s.toggleTerminal);
  const toggleFullscreen = useUiStore((s) => s.toggleFullscreen);
  const { font_family, font_size, commit_templates } = useSettingsStore((s) => s.settings);

  // Repo store
  const activeTabId = useRepoStore((s) => s.activeTabId);
  const tabs = useRepoStore((s) => s.tabs);
  const updateTab = useRepoStore((s) => s.updateTab);
  const toggleSolo = useRepoStore((s) => s.toggleSolo);
  const toggleHide = useRepoStore((s) => s.toggleHide);
  const resetView = useRepoStore((s) => s.resetView);

  const activeTab: TabState | undefined = activeTabId
    ? tabs.get(activeTabId)
    : undefined;

  // Settings
  const addCommitTemplate = useSettingsStore((s) => s.addCommitTemplate);
  const removeCommitTemplate = useSettingsStore((s) => s.removeCommitTemplate);
  const updateCommitTemplate = useSettingsStore((s) => s.updateCommitTemplate);

  // UI state
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [mainView, setMainView] = useState<MainView>('graph');

  // Data state for active tab
  const [commits, setCommits] = useState<CommitInfo[]>([]);
  const [dagLayout, setDagLayout] = useState<DagLayout>({ nodes: [], total_columns: 0, total_rows: 0 });
  const [selectedCommitId, setSelectedCommitId] = useState<string | null>(null);
  const [commitDetail, setCommitDetail] = useState<CommitDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [selectedDiff, setSelectedDiff] = useState<FileDiff | null>(null);
  const [blameInfo, setBlameInfo] = useState<BlameInfo | null>(null);

  // Sidebar data
  const [localBranches, setLocalBranches] = useState<BranchInfo[]>([]);
  const [remoteBranches, setRemoteBranches] = useState<BranchInfo[]>([]);
  const [tags, setTags] = useState<TagInfo[]>([]);
  const [stashes, setStashes] = useState<StashEntry[]>([]);
  const [submodules, setSubmodules] = useState<SubmoduleInfo[]>([]);
  const [worktrees, setWorktrees] = useState<WorktreeInfo[]>([]);

  // Staging data
  const [unstagedFiles, setUnstagedFiles] = useState<FileStatus[]>([]);
  const [stagedFiles, setStagedFiles] = useState<FileStatus[]>([]);

  // Search state
  const [searchResults, setSearchResults] = useState<CommitInfo[]>([]);
  const [searchLoading, setSearchLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [showSearch, setShowSearch] = useState(false);

  // Context menu state
  const [contextMenu, setContextMenu] = useState<{
    commit: CommitInfo;
    x: number;
    y: number;
  } | null>(null);

  // Conflict state
  const [conflictFiles, setConflictFiles] = useState<string[]>([]);

  // Host integration state
  const [hostPlatform, setHostPlatform] = useState<HostPlatform>('github');
  const [hostAuthenticated, setHostAuthenticated] = useState(false);
  const [hostPullRequests, setHostPullRequests] = useState<PullRequest[]>([]);
  const [hostLoadingPrs, setHostLoadingPrs] = useState(false);
  const [hostError, setHostError] = useState<string | null>(null);

  // Submodule update banner
  const submoduleUpdates = useSubmoduleUpdates(activeTabId);

  // ── Theme & font effects ──
  useEffect(() => {
    applyTheme(theme);
    watchSystemTheme(theme);
  }, [theme]);

  useEffect(() => {
    applyFont(font_family, font_size);
  }, [font_family, font_size]);

  // ── Data refresh helper ──
  const refreshSidebarData = useCallback(async (tabId: TabId) => {
    try {
      const [local, remote, tagList, stashList, subList, wtList] = await Promise.all([
        gitApi.listBranches(tabId, 'Local'),
        gitApi.listBranches(tabId, 'Remote'),
        gitApi.listTags(tabId),
        gitApi.listStashes(tabId),
        gitApi.listSubmodules(tabId),
        gitApi.listWorktrees(tabId),
      ]);
      setLocalBranches(local);
      setRemoteBranches(remote);
      setTags(tagList);
      setStashes(stashList);
      setSubmodules(subList);
      setWorktrees(wtList);
    } catch {
      // Sidebar data fetch failed — keep stale data
    }
  }, []);

  const refreshStatus = useCallback(async (tabId: TabId) => {
    try {
      const statusList = await gitApi.getStatus(tabId);
      setUnstagedFiles(statusList.filter((f) => f.status !== 'Staged'));
      setStagedFiles(statusList.filter((f) => f.status === 'Staged'));

      // Check for conflicts
      const conflicts = statusList.filter((f) => f.status === 'Conflict').map((f) => f.path);
      setConflictFiles(conflicts);

      // Update tab hasChanges flag
      updateTab(tabId, { hasChanges: statusList.length > 0 });
    } catch {
      // Status fetch failed
    }
  }, [updateTab]);

  const refreshCommitHistory = useCallback(async (tabId: TabId) => {
    try {
      const options: LogOptions = { offset: 0, limit: 500 };
      const [log, layout] = await Promise.all([
        gitApi.getCommitLog(tabId, options),
        gitApi.getDagLayout(tabId),
      ]);
      setCommits(log);
      setDagLayout(layout);
    } catch {
      // Commit history fetch failed
    }
  }, []);

  const refreshAll = useCallback(async (tabId: TabId) => {
    await Promise.all([
      refreshSidebarData(tabId),
      refreshStatus(tabId),
      refreshCommitHistory(tabId),
    ]);
  }, [refreshSidebarData, refreshStatus, refreshCommitHistory]);

  // ── Tab switch: reload all data ──
  useEffect(() => {
    if (!activeTabId) {
      // Clear state when no tab is active
      setCommits([]);
      setDagLayout({ nodes: [], total_columns: 0, total_rows: 0 });
      setSelectedCommitId(null);
      setCommitDetail(null);
      setSelectedDiff(null);
      setBlameInfo(null);
      setLocalBranches([]);
      setRemoteBranches([]);
      setTags([]);
      setStashes([]);
      setSubmodules([]);
      setWorktrees([]);
      setUnstagedFiles([]);
      setStagedFiles([]);
      setConflictFiles([]);
      return;
    }
    refreshAll(activeTabId);
  }, [activeTabId, refreshAll]);

  // ── FileWatcher event: refresh status & sidebar on file changes ──
  useEffect(() => {
    const unlisten = onFileChanged((payload) => {
      if (payload.tab_id === activeTabId) {
        refreshStatus(payload.tab_id);
        refreshSidebarData(payload.tab_id);
        refreshCommitHistory(payload.tab_id);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [activeTabId, refreshStatus, refreshSidebarData, refreshCommitHistory]);

  // ── Progress event listener ──
  useEffect(() => {
    const unlisten = onProgress((_payload) => {
      // Progress events can be used for UI indicators in the future
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // ── Commit selection ──
  const handleSelectCommit = useCallback(async (commitId: string) => {
    setSelectedCommitId(commitId);
    setMainView('graph');
    if (!activeTabId) return;
    setDetailLoading(true);
    try {
      const detail = await gitApi.getCommitDetail(activeTabId, commitId);
      setCommitDetail(detail);
    } catch {
      setCommitDetail(null);
    } finally {
      setDetailLoading(false);
    }
  }, [activeTabId]);

  // ── Context menu ──
  const handleContextMenu = useCallback((commitId: string, x: number, y: number) => {
    const commit = commits.find((c) => c.id === commitId);
    if (commit) {
      setContextMenu({ commit, x, y });
    }
  }, [commits]);

  const handleContextMenuClose = useCallback(() => {
    setContextMenu(null);
  }, []);

  const handleContextMenuAction = useCallback(async (actionId: string, commit: CommitInfo) => {
    if (!activeTabId) return;
    switch (actionId) {
      case 'copy-sha':
        await navigator.clipboard.writeText(commit.id);
        break;
      case 'copy-branch-name': {
        const branchRef = commit.refs.find((r) => r.ref_type.type === 'LocalBranch');
        if (branchRef) await navigator.clipboard.writeText(branchRef.name);
        break;
      }
      case 'checkout': {
        const branchRef = commit.refs.find((r) => r.ref_type.type === 'LocalBranch');
        if (branchRef) {
          await gitApi.checkoutBranch(activeTabId, branchRef.name);
          refreshAll(activeTabId);
        }
        break;
      }
      case 'cherry-pick':
        await gitApi.cherryPick(activeTabId, [commit.id]);
        refreshAll(activeTabId);
        break;
      case 'revert':
        await gitApi.revertCommits(activeTabId, [commit.id]);
        refreshAll(activeTabId);
        break;
      default:
        break;
    }
  }, [activeTabId, refreshAll]);

  // ── Search ──
  const handleSearch = useCallback(async (query: string, _options: SearchOptions) => {
    if (!activeTabId || !query.trim()) {
      setSearchResults([]);
      return;
    }
    setSearchLoading(true);
    setSearchQuery(query);
    try {
      const results = await gitApi.searchCommits(activeTabId, query);
      setSearchResults(results);
    } catch {
      setSearchResults([]);
    } finally {
      setSearchLoading(false);
    }
  }, [activeTabId]);

  // ── Sidebar callbacks ──
  const handleSolo = useCallback((branch: string) => {
    if (activeTabId) toggleSolo(activeTabId, branch);
  }, [activeTabId, toggleSolo]);

  const handleHide = useCallback((branch: string) => {
    if (activeTabId) toggleHide(activeTabId, branch);
  }, [activeTabId, toggleHide]);

  const handleResetView = useCallback(() => {
    if (activeTabId) resetView(activeTabId);
  }, [activeTabId, resetView]);

  const handleCreateBranch = useCallback(async () => {
    if (!activeTabId) return;
    const name = window.prompt(t('sidebar.newBranchName'));
    if (name) {
      await gitApi.createBranch(activeTabId, name);
      refreshSidebarData(activeTabId);
    }
  }, [activeTabId, t, refreshSidebarData]);

  const handleDeleteBranch = useCallback(async (name: string) => {
    if (!activeTabId) return;
    await gitApi.deleteBranch(activeTabId, name, false);
    refreshSidebarData(activeTabId);
  }, [activeTabId, refreshSidebarData]);

  const handleCheckout = useCallback(async (name: string) => {
    if (!activeTabId) return;
    await gitApi.checkoutBranch(activeTabId, name);
    refreshAll(activeTabId);
  }, [activeTabId, refreshAll]);

  const handleFetch = useCallback(async (remote: string) => {
    if (!activeTabId) return;
    await gitApi.fetchRemote(activeTabId, remote);
    refreshSidebarData(activeTabId);
  }, [activeTabId, refreshSidebarData]);

  const handleCheckoutTag = useCallback(async (name: string) => {
    if (!activeTabId) return;
    await gitApi.checkoutBranch(activeTabId, name);
    refreshAll(activeTabId);
  }, [activeTabId, refreshAll]);

  const handleDeleteTag = useCallback(async (name: string) => {
    if (!activeTabId) return;
    await gitApi.deleteTag(activeTabId, name);
    refreshSidebarData(activeTabId);
  }, [activeTabId, refreshSidebarData]);

  const handlePushTag = useCallback(async (_name: string) => {
    if (!activeTabId) return;
    await gitApi.pushRemote(activeTabId);
    refreshSidebarData(activeTabId);
  }, [activeTabId, refreshSidebarData]);

  const handleCopyTagName = useCallback(async (name: string) => {
    await navigator.clipboard.writeText(name);
  }, []);

  const handleApplyStash = useCallback(async (index: number) => {
    if (!activeTabId) return;
    await gitApi.applyStash(activeTabId, index);
    refreshAll(activeTabId);
  }, [activeTabId, refreshAll]);

  const handlePopStash = useCallback(async (index: number) => {
    if (!activeTabId) return;
    await gitApi.popStash(activeTabId, index);
    refreshAll(activeTabId);
  }, [activeTabId, refreshAll]);

  const handleDropStash = useCallback(async (index: number) => {
    if (!activeTabId) return;
    await gitApi.dropStash(activeTabId, index);
    refreshSidebarData(activeTabId);
  }, [activeTabId, refreshSidebarData]);

  const handleSelectStash = useCallback(async (index: number) => {
    if (!activeTabId) return;
    try {
      const diffs = await gitApi.stashDiff(activeTabId, index);
      if (diffs.length > 0) {
        setSelectedDiff(diffs[0]);
        setMainView('diff');
      }
    } catch {
      // Stash diff failed
    }
  }, [activeTabId]);

  // ── Submodule callbacks ──
  const handleInitSubmodule = useCallback(async (path: string) => {
    if (!activeTabId) return;
    await gitApi.initSubmodule(activeTabId, path);
    refreshSidebarData(activeTabId);
  }, [activeTabId, refreshSidebarData]);

  const handleUpdateSubmodule = useCallback(async (path: string) => {
    if (!activeTabId) return;
    await gitApi.updateSubmodule(activeTabId, path, true);
    refreshSidebarData(activeTabId);
  }, [activeTabId, refreshSidebarData]);

  const handleDeinitSubmodule = useCallback(async (path: string) => {
    if (!activeTabId) return;
    await gitApi.deinitSubmodule(activeTabId, path);
    refreshSidebarData(activeTabId);
  }, [activeTabId, refreshSidebarData]);

  const handleOpenSubmodule = useCallback(async (path: string) => {
    // Open submodule in a new tab
    try {
      const tabId = await gitApi.openRepository(path);
      // Tab is added by the store via addTab in the open flow
      void tabId;
    } catch {
      // Failed to open submodule
    }
  }, []);

  const handleCopySubmodulePath = useCallback(async (path: string) => {
    await navigator.clipboard.writeText(path);
  }, []);

  const handleChangeSubmoduleUrl = useCallback(async (path: string) => {
    if (!activeTabId) return;
    const newUrl = window.prompt(t('sidebar.newSubmoduleUrl'));
    if (newUrl) {
      await gitApi.setSubmoduleUrl(activeTabId, path, newUrl);
      refreshSidebarData(activeTabId);
    }
  }, [activeTabId, t, refreshSidebarData]);

  // ── Worktree callbacks ──
  const handleSelectWorktree = useCallback(async (wt: WorktreeInfo) => {
    try {
      await gitApi.openRepository(wt.path);
    } catch {
      // Failed to open worktree
    }
  }, []);

  const handleDeleteWorktree = useCallback(async (name: string) => {
    if (!activeTabId) return;
    await gitApi.deleteWorktree(activeTabId, name);
    refreshSidebarData(activeTabId);
  }, [activeTabId, refreshSidebarData]);

  // ── Host integration callbacks ──
  const handleHostAuthenticate = useCallback(async (platform: HostPlatform, token: string) => {
    try {
      await gitApi.authenticateHost(platform, token);
      setHostAuthenticated(true);
      setHostError(null);
    } catch (e) {
      setHostError(String(e));
    }
  }, []);

  const handleHostRefreshPrs = useCallback(async () => {
    if (!activeTabId) return;
    setHostLoadingPrs(true);
    try {
      const remotes = await gitApi.listRemotes(activeTabId);
      if (remotes.length > 0) {
        const prs = await gitApi.listPullRequests(hostPlatform, remotes[0].url);
        setHostPullRequests(prs);
      }
    } catch (e) {
      setHostError(String(e));
    } finally {
      setHostLoadingPrs(false);
    }
  }, [activeTabId, hostPlatform]);

  const handleHostCreatePr = useCallback(async (params: CreatePrParams) => {
    try {
      await gitApi.createPullRequest(hostPlatform, params);
      handleHostRefreshPrs();
    } catch (e) {
      setHostError(String(e));
    }
  }, [hostPlatform, handleHostRefreshPrs]);

  // ── Staging callbacks ──
  const handleStageFiles = useCallback(async (tabId: TabId, paths: string[]) => {
    await gitApi.stageFiles(tabId, paths);
    refreshStatus(tabId);
  }, [refreshStatus]);

  const handleUnstageFiles = useCallback(async (tabId: TabId, paths: string[]) => {
    await gitApi.unstageFiles(tabId, paths);
    refreshStatus(tabId);
  }, [refreshStatus]);

  const handleStageLines = useCallback(async (tabId: TabId, path: string, ranges: LineRange[]) => {
    await gitApi.stageLines(tabId, path, ranges);
    refreshStatus(tabId);
  }, [refreshStatus]);

  const handleUnstageLines = useCallback(async (tabId: TabId, path: string, ranges: LineRange[]) => {
    await gitApi.unstageLines(tabId, path, ranges);
    refreshStatus(tabId);
  }, [refreshStatus]);

  const handleDiscardLines = useCallback(async (tabId: TabId, path: string, ranges: LineRange[]) => {
    await gitApi.discardLines(tabId, path, ranges);
    refreshStatus(tabId);
  }, [refreshStatus]);

  const handleGetFileDiff = useCallback(async (tabId: TabId, path: string, staged: boolean) => {
    return gitApi.getFileDiff(tabId, path, staged);
  }, []);

  // ── Commit callbacks ──
  const handleCommit = useCallback(async (tabId: TabId, message: string) => {
    await gitApi.createCommit(tabId, message);
    refreshAll(tabId);
  }, [refreshAll]);

  const handleAmendCommit = useCallback(async (tabId: TabId, message: string) => {
    await gitApi.amendCommit(tabId, message);
    refreshAll(tabId);
  }, [refreshAll]);

  // ── Blame navigation ──
  const handleViewBlame = useCallback(async (path: string) => {
    if (!activeTabId) return;
    try {
      const blame = await gitApi.getBlame(activeTabId, path);
      setBlameInfo(blame);
      setMainView('blame');
    } catch {
      // Blame fetch failed
    }
  }, [activeTabId]);

  const handleBlameNavigateToCommit = useCallback((commitId: string) => {
    handleSelectCommit(commitId);
  }, [handleSelectCommit]);

  // ── Conflict resolution callbacks ──
  const handleGetConflictFileContent = useCallback(async (
    tabId: TabId,
    path: string,
    _version: 'ours' | 'theirs' | 'merge',
  ) => {
    // Simplified: return file content from working directory
    try {
      if (selectedCommitId) {
        return await gitApi.getFileContent(tabId, selectedCommitId, path);
      }
      return '';
    } catch {
      return '';
    }
  }, [selectedCommitId]);

  const handleMergeContentChange = useCallback((_filePath: string, _content: string) => {
    // Content change tracked by ConflictResolver internally
  }, []);

  const handleMarkResolved = useCallback(async (tabId: TabId, path: string) => {
    await gitApi.stageFiles(tabId, [path]);
    refreshStatus(tabId);
  }, [refreshStatus]);

  const handleContinueOperation = useCallback(async (tabId: TabId) => {
    const repoState = activeTab?.repoState;
    if (!repoState) return;
    if (repoState.type === 'Rebasing') {
      await gitApi.continueRebase(tabId);
    }
    refreshAll(tabId);
  }, [activeTab?.repoState, refreshAll]);

  const handleAbortOperation = useCallback(async (tabId: TabId) => {
    const repoState = activeTab?.repoState;
    if (!repoState) return;
    if (repoState.type === 'Rebasing') {
      await gitApi.abortRebase(tabId);
    }
    refreshAll(tabId);
  }, [activeTab?.repoState, refreshAll]);

  // ── Hotkeys ──
  useHotkeys({
    commit: () => {
      // Focus commit editor — handled by CommitEditor internally
    },
    search: () => setShowSearch((prev) => !prev),
    toggleTerminal: () => toggleTerminal(),
    toggleFullscreen: () => toggleFullscreen(),
  });

  // ── Determine if we should show conflict resolver ──
  const isConflictState = activeTab?.repoState?.type === 'Merging'
    || activeTab?.repoState?.type === 'Rebasing'
    || activeTab?.repoState?.type === 'CherryPicking'
    || activeTab?.repoState?.type === 'Reverting';
  const showConflictResolver = isConflictState && conflictFiles.length > 0;

  const hasStagedChanges = stagedFiles.length > 0;

  // ── Render ──
  if (!activeTabId || !activeTab) {
    return (
      <div className="app-container app-empty">
        <h1 className="text-2xl font-bold text-center opacity-60">
          {t('app.title')}
        </h1>
        <p className="text-center opacity-40 mt-2">{t('app.openRepo')}</p>
      </div>
    );
  }

  return (
    <BranchDragProvider>
      <div className="app-container">
        {/* Top: TabBar */}
        <TabBar className="app-tabbar" />

        {/* Toolbar */}
        <Toolbar className="app-toolbar" />

        {/* Main content area */}
        <div className="app-body">
          {/* Left: Sidebar */}
          <Sidebar
            tabId={activeTabId}
            localBranches={localBranches}
            remoteBranches={remoteBranches}
            tags={tags}
            stashes={stashes}
            submodules={submodules}
            soloedBranches={activeTab.soloedBranches}
            hiddenBranches={activeTab.hiddenBranches}
            onSolo={handleSolo}
            onHide={handleHide}
            onCreateBranch={handleCreateBranch}
            onDeleteBranch={handleDeleteBranch}
            onCheckout={handleCheckout}
            onFetch={handleFetch}
            onCheckoutTag={handleCheckoutTag}
            onDeleteTag={handleDeleteTag}
            onPushTag={handlePushTag}
            onCopyTagName={handleCopyTagName}
            onApplyStash={handleApplyStash}
            onPopStash={handlePopStash}
            onDropStash={handleDropStash}
            onSelectStash={handleSelectStash}
            onInitSubmodule={handleInitSubmodule}
            onUpdateSubmodule={handleUpdateSubmodule}
            onDeinitSubmodule={handleDeinitSubmodule}
            onOpenSubmodule={handleOpenSubmodule}
            onCopySubmodulePath={handleCopySubmodulePath}
            onChangeSubmoduleUrl={handleChangeSubmoduleUrl}
            showSubmoduleUpdateBanner={submoduleUpdates.showBanner}
            changedSubmodules={submoduleUpdates.changedSubmodules}
            onUpdateAllSubmodules={submoduleUpdates.updateAll}
            onDismissSubmoduleBanner={submoduleUpdates.dismiss}
            worktrees={worktrees}
            onSelectWorktree={handleSelectWorktree}
            onDeleteWorktree={handleDeleteWorktree}
            hostPlatform={hostPlatform}
            onHostPlatformChange={setHostPlatform}
            hostAuthenticated={hostAuthenticated}
            onHostAuthenticate={handleHostAuthenticate}
            onHostLogout={() => setHostAuthenticated(false)}
            hostPullRequests={hostPullRequests}
            hostLoadingPrs={hostLoadingPrs}
            onHostRefreshPrs={handleHostRefreshPrs}
            onHostCreatePr={handleHostCreatePr}
            hostError={hostError}
          />

          {/* Center: Main content */}
          <div className="app-main">
            {/* Search panel (toggleable) */}
            {showSearch && (
              <SearchPanel
                onSearch={handleSearch}
                results={searchResults}
                loading={searchLoading}
                onSelectCommit={handleSelectCommit}
                searchQuery={searchQuery}
              />
            )}

            {/* Conflict resolver overlay */}
            {showConflictResolver && (
              <ConflictResolver
                tabId={activeTabId}
                repoState={activeTab.repoState}
                conflictFiles={conflictFiles}
                onGetFileContent={handleGetConflictFileContent}
                onMergeContentChange={handleMergeContentChange}
                onMarkResolved={handleMarkResolved}
                onContinue={handleContinueOperation}
                onAbort={handleAbortOperation}
              />
            )}

            {/* Main view switcher */}
            {!showConflictResolver && (
              <div className="app-main-content">
                {/* View tabs */}
                <div className="app-view-tabs" role="tablist">
                  <button
                    role="tab"
                    aria-selected={mainView === 'graph'}
                    className={`app-view-tab${mainView === 'graph' ? ' app-view-tab--active' : ''}`}
                    onClick={() => setMainView('graph')}
                  >
                    {t('views.graph')}
                  </button>
                  <button
                    role="tab"
                    aria-selected={mainView === 'diff'}
                    className={`app-view-tab${mainView === 'diff' ? ' app-view-tab--active' : ''}`}
                    onClick={() => setMainView('diff')}
                  >
                    {t('views.diff')}
                  </button>
                  <button
                    role="tab"
                    aria-selected={mainView === 'blame'}
                    className={`app-view-tab${mainView === 'blame' ? ' app-view-tab--active' : ''}`}
                    onClick={() => setMainView('blame')}
                  >
                    {t('views.blame')}
                  </button>
                  <button
                    role="tab"
                    aria-selected={mainView === 'tree'}
                    className={`app-view-tab${mainView === 'tree' ? ' app-view-tab--active' : ''}`}
                    onClick={() => setMainView('tree')}
                  >
                    {t('views.tree')}
                  </button>
                </div>

                {/* View content */}
                <div className="app-view-content">
                  {mainView === 'graph' && (
                    <CommitGraph
                      layout={dagLayout}
                      commits={commits}
                      selectedCommitId={selectedCommitId}
                      commitDetail={commitDetail}
                      detailLoading={detailLoading}
                      soloedBranches={activeTab.soloedBranches}
                      hiddenBranches={activeTab.hiddenBranches}
                      pinnedLeftBranches={activeTab.pinnedLeftBranches}
                      onSelectCommit={handleSelectCommit}
                      onContextMenu={handleContextMenu}
                      onResetView={handleResetView}
                    />
                  )}

                  {mainView === 'diff' && selectedDiff && (
                    <DiffViewer diff={selectedDiff} mode={diffViewMode} />
                  )}

                  {mainView === 'blame' && blameInfo && (
                    <BlameViewer
                      blame={blameInfo}
                      onNavigateToCommit={handleBlameNavigateToCommit}
                    />
                  )}

                  {mainView === 'tree' && (
                    <TreeBrowser
                      tabId={activeTabId}
                      commitId={selectedCommitId}
                      onViewBlame={handleViewBlame}
                    />
                  )}
                </div>
              </div>
            )}

            {/* Bottom: Staging + Commit editor */}
            <div className="app-staging-area">
              <StagingPanel
                tabId={activeTabId}
                unstagedFiles={unstagedFiles}
                stagedFiles={stagedFiles}
                onStageFiles={handleStageFiles}
                onUnstageFiles={handleUnstageFiles}
                onStageLines={handleStageLines}
                onUnstageLines={handleUnstageLines}
                onDiscardLines={handleDiscardLines}
                onGetFileDiff={handleGetFileDiff}
              />
              <CommitEditor
                tabId={activeTabId}
                hasStagedChanges={hasStagedChanges}
                templates={commit_templates}
                onCommit={handleCommit}
                onAmendCommit={handleAmendCommit}
                onAddTemplate={addCommitTemplate}
                onUpdateTemplate={updateCommitTemplate}
                onRemoveTemplate={removeCommitTemplate}
              />
            </div>
          </div>
        </div>

        {/* Bottom: Terminal */}
        {terminalVisible && <TerminalPanel className="app-terminal" />}

        {/* Context menu */}
        {contextMenu && (
          <CommitContextMenu
            commit={contextMenu.commit}
            position={{ x: contextMenu.x, y: contextMenu.y }}
            visible={true}
            onAction={handleContextMenuAction}
            onClose={handleContextMenuClose}
          />
        )}

        {/* Settings modal */}
        <SettingsPanel open={settingsOpen} onOpenChange={setSettingsOpen} />
      </div>
    </BranchDragProvider>
  );
}

export default App;
