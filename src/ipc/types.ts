// TypeScript interfaces matching all Rust backend data models
// These types mirror the definitions in src-tauri/src/models.rs

// === Repository & Tab ===

export type TabId = string;

export interface RepoEntry {
  path: string;
  name: string;
  last_opened: string; // ISO 8601 DateTime
}

export type RepositoryState =
  | { type: 'Clean' }
  | { type: 'Merging' }
  | { type: 'Rebasing'; current: number; total: number }
  | { type: 'CherryPicking' }
  | { type: 'Reverting' };

// === Commit ===

export interface CommitInfo {
  id: string;
  short_id: string;
  message: string;
  author: SignatureInfo;
  committer: SignatureInfo;
  parent_ids: string[];
  refs: RefLabel[];
  is_cherry_picked: boolean;
}

export interface CommitDetail {
  commit: CommitInfo;
  files: FileStatus[];
  stats: DiffStats;
}

export interface SignatureInfo {
  name: string;
  email: string;
  timestamp: number;
}

export interface RefLabel {
  name: string;
  ref_type: RefType;
  is_head: boolean;
}

export type RefType =
  | { type: 'LocalBranch' }
  | { type: 'RemoteBranch'; remote: string }
  | { type: 'Tag' };

// === DAG Layout ===

export interface DagNode {
  commit_id: string;
  column: number;
  row: number;
  color_index: number;
  parent_edges: DagEdge[];
}

export interface DagEdge {
  from_column: number;
  to_column: number;
  to_row: number;
  color_index: number;
}

export interface DagLayout {
  nodes: DagNode[];
  total_columns: number;
  total_rows: number;
}

// === Branch ===

export interface BranchInfo {
  name: string;
  is_head: boolean;
  upstream: string | null;
  ahead: number;
  behind: number;
  last_commit_id: string;
  branch_type: BranchType;
}

export type BranchType =
  | { type: 'Local' }
  | { type: 'Remote'; remote_name: string };

export type BranchFilter = 'All' | 'Local' | 'Remote';

// === Tag ===

export interface TagInfo {
  name: string;
  target_commit_id: string;
  is_annotated: boolean;
  message: string | null;
  tagger: SignatureInfo | null;
}

// === Diff ===

export interface FileDiff {
  path: string;
  old_path: string | null;
  status: DiffFileStatus;
  hunks: DiffHunk[];
  is_binary: boolean;
}

export type DiffFileStatus = 'Added' | 'Deleted' | 'Modified' | 'Renamed' | 'Copied';

export interface DiffHunk {
  header: string;
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  lines: DiffLine[];
}

export interface DiffLine {
  origin: DiffLineType;
  old_lineno: number | null;
  new_lineno: number | null;
  content: string;
}

export type DiffLineType = 'Context' | 'Addition' | 'Deletion';

export interface DiffStats {
  files_changed: number;
  insertions: number;
  deletions: number;
}

export interface LineRange {
  start: number;
  end: number;
}

// === File Status ===

export interface FileStatus {
  path: string;
  status: FileStatusType;
}

export type FileStatusType =
  | 'Untracked'
  | 'Modified'
  | 'Staged'
  | 'Conflict'
  | 'Deleted'
  | 'Renamed';

// === Blame ===

export interface BlameInfo {
  path: string;
  lines: BlameLine[];
}

export interface BlameLine {
  line_number: number;
  content: string;
  commit_id: string;
  author: string;
  date: number;
  original_line: number;
}

// === Stash ===

export interface StashEntry {
  index: number;
  message: string;
  timestamp: number;
  commit_id: string;
}

export type StashPopResult =
  | { type: 'Success' }
  | { type: 'Conflict'; files: string[] };

// === Submodule ===

export interface SubmoduleInfo {
  name: string;
  path: string;
  url: string;
  head_id: string | null;
  status: SubmoduleStatus;
  branch: string | null;
}

export type SubmoduleStatus =
  | 'Uninitialized'
  | 'Initialized'
  | 'Modified'
  | 'DetachedHead';

// === Worktree ===

export interface WorktreeInfo {
  name: string;
  path: string;
  branch: string | null;
  is_main: boolean;
}

// === Remote Operations ===

export interface RemoteInfo {
  name: string;
  url: string;
  push_url: string | null;
}

export type PullResult =
  | { type: 'FastForward' }
  | { type: 'Merged' }
  | { type: 'Conflict'; files: string[] }
  | { type: 'UpToDate' };

export type MergeResult =
  | { type: 'FastForward' }
  | { type: 'Merged' }
  | { type: 'Conflict'; files: string[] };

export type CherryPickResult =
  | { type: 'Success'; new_commits: string[] }
  | { type: 'Conflict'; files: string[]; at_commit: string };

export type RevertResult =
  | { type: 'Success'; new_commits: string[] }
  | { type: 'Conflict'; files: string[]; at_commit: string };

// === Rebase ===

export interface RebaseProgress {
  current_step: number;
  total_steps: number;
  status: RebaseStepStatus;
}

export type RebaseStepStatus =
  | { type: 'InProgress' }
  | { type: 'Conflict'; files: string[] }
  | { type: 'Completed' };

// === Search ===

export interface LogOptions {
  branch?: string | null;
  author?: string | null;
  since?: number | null;
  until?: number | null;
  path?: string | null;
  search?: string | null;
  offset: number;
  limit: number;
}

// === Config ===

export interface GitConfig {
  user_name: string | null;
  user_email: string | null;
  default_branch: string | null;
  merge_strategy: string | null;
}

// === Progress & Events ===

export interface ProgressEvent {
  operation: string;
  current: number;
  total: number | null;
  message: string | null;
}

export interface FileChangeEvent {
  tab_id: TabId;
  changed_paths: string[];
}

// === Undo ===

export type GitOperation =
  | { type: 'Commit'; id: string }
  | { type: 'Checkout'; from_branch: string; to_branch: string }
  | { type: 'Merge'; source: string }
  | { type: 'Rebase'; onto: string }
  | { type: 'BranchCreate'; name: string }
  | { type: 'BranchDelete'; name: string; target: string }
  | { type: 'Reset'; mode: ResetMode; target: string }
  | { type: 'Revert'; commit_id: string }
  | { type: 'CherryPick'; commit_id: string }
  | { type: 'Stash' }
  | { type: 'StashPop'; index: number };

export type ResetMode = 'Soft' | 'Mixed' | 'Hard';

export interface RepositorySnapshot {
  head_id: string;
  head_ref: string | null;
  index_tree_id: string | null;
}

// === Remote Hosting Platform ===

export interface PullRequest {
  id: number;
  title: string;
  description: string;
  state: PrState;
  source_branch: string;
  target_branch: string;
  author: string;
  url: string;
}

export type PrState = 'Open' | 'Closed' | 'Merged';

export interface CreatePrParams {
  title: string;
  description: string;
  source_branch: string;
  target_branch: string;
}

// === Commit Template ===

export interface CommitTemplate {
  id: string;
  name: string;
  content: string;
}

// === App Settings ===

export interface AppSettings {
  theme: ThemeMode;
  language: string;
  font_family: string;
  font_size: number;
  hotkeys: Record<string, string>;
  window: WindowState;
  commit_templates: CommitTemplate[];
}

export type ThemeMode = 'Light' | 'Dark' | 'System';

export interface WindowState {
  width: number;
  height: number;
  x: number | null;
  y: number | null;
  maximized: boolean;
}

// === Tree Browser ===

export interface TreeEntry {
  name: string;
  path: string;
  type: 'file' | 'directory';
  children?: TreeEntry[];
}

// === IPC Error ===

export interface IpcError {
  error_type: string;
  message: string;
}
