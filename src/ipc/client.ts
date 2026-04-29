// IPC client — type-safe invoke wrappers for all Tauri commands
import { invoke } from '@tauri-apps/api/core';
import type {
  TabId,
  RepoEntry,
  RepositoryState,
  CommitInfo,
  CommitDetail,
  DagLayout,
  FileDiff,
  FileStatus,
  BlameInfo,
  BranchInfo,
  BranchFilter,
  MergeResult,
  RemoteInfo,
  RebaseProgress,
  CherryPickResult,
  RevertResult,
  StashEntry,
  StashPopResult,
  SubmoduleInfo,
  WorktreeInfo,
  LogOptions,
  GitConfig,
  AppSettings,
  PullResult,
  LineRange,
  ResetMode,
  TagInfo,
  TreeEntry,
  PullRequest,
  CreatePrParams,
} from './types';

export const gitApi = {
  // === Repository Management ===
  openRepository: (path: string) =>
    invoke<TabId>('open_repository', { path }),
  cloneRepository: (url: string, path: string, recursive?: boolean) =>
    invoke<TabId>('clone_repository', { url, path, recursive: recursive ?? false }),
  initRepository: (path: string) =>
    invoke<TabId>('init_repository', { path }),
  closeRepository: (tabId: TabId) =>
    invoke<void>('close_repository', { tabId }),
  getRecentRepos: () =>
    invoke<RepoEntry[]>('get_recent_repos'),
  saveRecentRepos: () =>
    invoke<void>('save_recent_repos'),
  loadRecentRepos: () =>
    invoke<RepoEntry[]>('load_recent_repos'),
  removeRecentRepo: (path: string) =>
    invoke<void>('remove_recent_repo', { path }),
  getRepoStatus: (tabId: TabId) =>
    invoke<RepositoryState>('get_repo_status', { tabId }),

  // === Commit History & Search ===
  getCommitLog: (tabId: TabId, options: LogOptions) =>
    invoke<CommitInfo[]>('get_commit_log', { tabId, options }),
  getCommitDetail: (tabId: TabId, commitId: string) =>
    invoke<CommitDetail>('get_commit_detail', { tabId, commitId }),
  createCommit: (tabId: TabId, message: string) =>
    invoke<CommitInfo>('create_commit', { tabId, message }),
  amendCommit: (tabId: TabId, message: string) =>
    invoke<CommitInfo>('amend_commit', { tabId, message }),
  searchCommits: (tabId: TabId, query: string) =>
    invoke<CommitInfo[]>('search_commits', { tabId, query }),
  getDagLayout: (tabId: TabId) =>
    invoke<DagLayout>('get_dag_layout', { tabId }),

  // === Diff ===
  getWorkingDiff: (tabId: TabId, ignoreWhitespace: boolean) =>
    invoke<FileDiff[]>('get_working_diff', { tabId, ignoreWhitespace }),
  getCommitDiff: (tabId: TabId, commitId: string) =>
    invoke<FileDiff[]>('get_commit_diff', { tabId, commitId }),
  getFileDiff: (tabId: TabId, path: string, staged: boolean) =>
    invoke<FileDiff>('get_file_diff', { tabId, path, staged }),
  compareCommits: (tabId: TabId, from: string, to: string) =>
    invoke<FileDiff[]>('compare_commits', { tabId, from, to }),

  // === Staging ===
  stageFiles: (tabId: TabId, paths: string[]) =>
    invoke<void>('stage_files', { tabId, paths }),
  unstageFiles: (tabId: TabId, paths: string[]) =>
    invoke<void>('unstage_files', { tabId, paths }),
  stageLines: (tabId: TabId, path: string, lineRanges: LineRange[]) =>
    invoke<void>('stage_lines', { tabId, path, lineRanges }),
  unstageLines: (tabId: TabId, path: string, lineRanges: LineRange[]) =>
    invoke<void>('unstage_lines', { tabId, path, lineRanges }),
  discardLines: (tabId: TabId, path: string, lineRanges: LineRange[]) =>
    invoke<void>('discard_lines', { tabId, path, lineRanges }),
  getStatus: (tabId: TabId) =>
    invoke<FileStatus[]>('get_status', { tabId }),

  // === Blame ===
  getBlame: (tabId: TabId, path: string) =>
    invoke<BlameInfo>('get_blame', { tabId, path }),

  // === Branch ===
  listBranches: (tabId: TabId, filter: BranchFilter) =>
    invoke<BranchInfo[]>('list_branches', { tabId, filter }),
  createBranch: (tabId: TabId, name: string, target?: string) =>
    invoke<BranchInfo>('create_branch', { tabId, name, target }),
  deleteBranch: (tabId: TabId, name: string, force: boolean) =>
    invoke<void>('delete_branch', { tabId, name, force }),
  renameBranch: (tabId: TabId, oldName: string, newName: string) =>
    invoke<void>('rename_branch', { tabId, oldName, newName }),
  checkoutBranch: (tabId: TabId, name: string) =>
    invoke<void>('checkout_branch', { tabId, name }),
  setUpstream: (tabId: TabId, local: string, remote: string) =>
    invoke<void>('set_upstream', { tabId, local, remote }),
  mergeBranch: (tabId: TabId, source: string) =>
    invoke<MergeResult>('merge_branch', { tabId, source }),
  resetBranch: (tabId: TabId, target: string, mode: ResetMode) =>
    invoke<void>('reset_branch', { tabId, target, mode }),

  // === Tag ===
  listTags: (tabId: TabId) =>
    invoke<TagInfo[]>('list_tags', { tabId }),
  createLightweightTag: (tabId: TabId, name: string, target?: string) =>
    invoke<TagInfo>('create_lightweight_tag', { tabId, name, target }),
  createAnnotatedTag: (tabId: TabId, name: string, message: string, target?: string) =>
    invoke<TagInfo>('create_annotated_tag', { tabId, name, target, message }),
  deleteTag: (tabId: TabId, name: string) =>
    invoke<void>('delete_tag', { tabId, name }),

  // === Remote ===
  fetchRemote: (tabId: TabId, remote?: string) =>
    invoke<void>('fetch_remote', { tabId, remote }),
  pullRemote: (tabId: TabId, remote?: string) =>
    invoke<PullResult>('pull_remote', { tabId, remote }),
  pushRemote: (tabId: TabId, remote?: string, force?: boolean) =>
    invoke<void>('push_remote', { tabId, remote, force: force ?? false }),
  addRemote: (tabId: TabId, name: string, url: string) =>
    invoke<void>('add_remote', { tabId, name, url }),
  removeRemote: (tabId: TabId, name: string) =>
    invoke<void>('remove_remote', { tabId, name }),
  listRemotes: (tabId: TabId) =>
    invoke<RemoteInfo[]>('list_remotes', { tabId }),

  // === Rebase ===
  startRebase: (tabId: TabId, onto: string) =>
    invoke<RebaseProgress>('start_rebase', { tabId, onto }),
  continueRebase: (tabId: TabId) =>
    invoke<RebaseProgress>('continue_rebase', { tabId }),
  abortRebase: (tabId: TabId) =>
    invoke<void>('abort_rebase', { tabId }),
  getRebaseStatus: (tabId: TabId) =>
    invoke<RebaseProgress | null>('get_rebase_status', { tabId }),

  // === Cherry-pick & Revert ===
  cherryPick: (tabId: TabId, commitIds: string[]) =>
    invoke<CherryPickResult>('cherry_pick', { tabId, commitIds }),
  revertCommits: (tabId: TabId, commitIds: string[]) =>
    invoke<RevertResult>('revert_commits', { tabId, commitIds }),
  createPatch: (tabId: TabId, commitId: string) =>
    invoke<number[]>('create_patch', { tabId, commitId }),

  // === Stash ===
  createStash: (tabId: TabId, message?: string) =>
    invoke<StashEntry>('create_stash', { tabId, message }),
  listStashes: (tabId: TabId) =>
    invoke<StashEntry[]>('list_stashes', { tabId }),
  applyStash: (tabId: TabId, index: number) =>
    invoke<void>('apply_stash', { tabId, index }),
  popStash: (tabId: TabId, index: number) =>
    invoke<StashPopResult>('pop_stash', { tabId, index }),
  dropStash: (tabId: TabId, index: number) =>
    invoke<void>('drop_stash', { tabId, index }),
  stashDiff: (tabId: TabId, index: number) =>
    invoke<FileDiff[]>('stash_diff', { tabId, index }),

  // === Submodule ===
  listSubmodules: (tabId: TabId) =>
    invoke<SubmoduleInfo[]>('list_submodules', { tabId }),
  initSubmodule: (tabId: TabId, path: string) =>
    invoke<void>('init_submodule', { tabId, path }),
  updateSubmodule: (tabId: TabId, path: string, recursive: boolean) =>
    invoke<void>('update_submodule', { tabId, path, recursive }),
  deinitSubmodule: (tabId: TabId, path: string) =>
    invoke<void>('deinit_submodule', { tabId, path }),
  setSubmoduleUrl: (tabId: TabId, path: string, url: string) =>
    invoke<void>('set_submodule_url', { tabId, path, url }),
  setSubmoduleBranch: (tabId: TabId, path: string, branch: string) =>
    invoke<void>('set_submodule_branch', { tabId, path, branch }),

  // === Worktree ===
  createWorktree: (tabId: TabId, name: string, path: string, branch?: string) =>
    invoke<WorktreeInfo>('create_worktree', { tabId, name, path, branch }),
  listWorktrees: (tabId: TabId) =>
    invoke<WorktreeInfo[]>('list_worktrees', { tabId }),
  deleteWorktree: (tabId: TabId, name: string) =>
    invoke<void>('delete_worktree', { tabId, name }),

  // === Undo/Redo ===
  undo: (tabId: TabId) =>
    invoke<string>('undo_operation', { tabId }),
  redo: (tabId: TabId) =>
    invoke<string>('redo_operation', { tabId }),
  canUndo: (tabId: TabId) =>
    invoke<string | null>('can_undo', { tabId }),
  canRedo: (tabId: TabId) =>
    invoke<string | null>('can_redo', { tabId }),

  // === Config ===
  getGitConfig: (tabId: TabId, level: 'local' | 'global') =>
    invoke<GitConfig>('get_git_config', { tabId, level }),
  setGitConfig: (tabId: TabId, level: 'local' | 'global', config: GitConfig) =>
    invoke<void>('set_git_config', { tabId, level, config }),
  saveAppSettings: (path: string, settings: AppSettings) =>
    invoke<void>('save_app_settings', { path, settings }),
  loadAppSettings: (path: string) =>
    invoke<AppSettings>('load_app_settings', { path }),

  // === Host Integration ===
  getRepoWebUrl: (remoteUrl: string, platform: 'github' | 'gitlab') =>
    invoke<string | null>('get_repo_web_url', { remoteUrl, platform }),
  getCommitWebUrl: (remoteUrl: string, sha: string, platform: 'github' | 'gitlab') =>
    invoke<string | null>('get_commit_web_url', { remoteUrl, sha, platform }),
  getBranchWebUrl: (remoteUrl: string, branch: string, platform: 'github' | 'gitlab') =>
    invoke<string | null>('get_branch_web_url', { remoteUrl, branch, platform }),
  authenticateHost: (platform: 'github' | 'gitlab', token: string) =>
    invoke<void>('authenticate_host', { platform, token }),
  listPullRequests: (platform: 'github' | 'gitlab', remoteUrl: string) =>
    invoke<PullRequest[]>('list_pull_requests', { platform, remoteUrl }),
  createPullRequest: (platform: 'github' | 'gitlab', params: CreatePrParams) =>
    invoke<PullRequest>('create_pull_request', { platform, params }),

  // === Tree Browser ===
  getCommitTree: (tabId: TabId, commitId: string) =>
    invoke<TreeEntry[]>('get_commit_tree', { tabId, commitId }),
  getFileContent: (tabId: TabId, commitId: string, path: string) =>
    invoke<string>('get_file_content', { tabId, commitId, path }),
};
