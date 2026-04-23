// Core data models for the Git GUI application
//
// All types derive Debug, Clone, Serialize, Deserialize, and PartialEq.
// TabId additionally derives Eq and Hash for use as HashMap keys.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// === Repository & Tab ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TabId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoEntry {
    pub path: String,
    pub name: String,
    pub last_opened: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RepositoryState {
    Clean,
    Merging,
    Rebasing { current: usize, total: usize },
    CherryPicking,
    Reverting,
}

// === Commit ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author: SignatureInfo,
    pub committer: SignatureInfo,
    pub parent_ids: Vec<String>,
    pub refs: Vec<RefLabel>,
    pub is_cherry_picked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommitDetail {
    pub commit: CommitInfo,
    pub files: Vec<FileStatus>,
    pub stats: DiffStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignatureInfo {
    pub name: String,
    pub email: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefLabel {
    pub name: String,
    pub ref_type: RefType,
    pub is_head: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RefType {
    LocalBranch,
    RemoteBranch { remote: String },
    Tag,
}

// === DAG Layout ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagNode {
    pub commit_id: String,
    pub column: usize,
    pub row: usize,
    pub color_index: usize,
    pub parent_edges: Vec<DagEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagEdge {
    pub from_column: usize,
    pub to_column: usize,
    pub to_row: usize,
    pub color_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagLayout {
    pub nodes: Vec<DagNode>,
    pub total_columns: usize,
    pub total_rows: usize,
}

// === Branch ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub last_commit_id: String,
    pub branch_type: BranchType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BranchType {
    Local,
    Remote { remote_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BranchFilter {
    All,
    Local,
    Remote,
}

// === Tag ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagInfo {
    pub name: String,
    pub target_commit_id: String,
    pub is_annotated: bool,
    pub message: Option<String>,
    pub tagger: Option<SignatureInfo>,
}

// === Diff ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: DiffFileStatus,
    pub hunks: Vec<DiffHunk>,
    pub is_binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiffFileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffHunk {
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffLine {
    pub origin: DiffLineType,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiffLineType {
    Context,
    Addition,
    Deletion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

// === File Status ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileStatus {
    pub path: String,
    pub status: FileStatusType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileStatusType {
    Untracked,
    Modified,
    Staged,
    Conflict,
    Deleted,
    Renamed,
}

// === Blame ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlameInfo {
    pub path: String,
    pub lines: Vec<BlameLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlameLine {
    pub line_number: u32,
    pub content: String,
    pub commit_id: String,
    pub author: String,
    pub date: i64,
    pub original_line: u32,
}

// === Stash ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
    pub timestamp: i64,
    pub commit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StashPopResult {
    Success,
    Conflict { files: Vec<String> },
}

// === Submodule ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubmoduleInfo {
    pub name: String,
    pub path: String,
    pub url: String,
    pub head_id: Option<String>,
    pub status: SubmoduleStatus,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubmoduleStatus {
    Uninitialized,
    Initialized,
    Modified,
    DetachedHead,
}

// === Worktree ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorktreeInfo {
    pub name: String,
    pub path: String,
    pub branch: Option<String>,
    pub is_main: bool,
}

// === Remote Operations ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PullResult {
    FastForward,
    Merged,
    Conflict { files: Vec<String> },
    UpToDate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MergeResult {
    FastForward,
    Merged,
    Conflict { files: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CherryPickResult {
    Success { new_commits: Vec<String> },
    Conflict { files: Vec<String>, at_commit: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RevertResult {
    Success { new_commits: Vec<String> },
    Conflict { files: Vec<String>, at_commit: String },
}

// === Rebase ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RebaseProgress {
    pub current_step: usize,
    pub total_steps: usize,
    pub status: RebaseStepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RebaseStepStatus {
    InProgress,
    Conflict { files: Vec<String> },
    Completed,
}

// === Search ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogOptions {
    pub branch: Option<String>,
    pub author: Option<String>,
    pub since: Option<i64>,
    pub until: Option<i64>,
    pub path: Option<String>,
    pub search: Option<String>,
    pub offset: usize,
    pub limit: usize,
}

// === Config ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitConfig {
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub default_branch: Option<String>,
    pub merge_strategy: Option<String>,
}

// === Progress & Events ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgressEvent {
    pub operation: String,
    pub current: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileChangeEvent {
    pub tab_id: TabId,
    pub changed_paths: Vec<String>,
}

// === Undo ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GitOperation {
    Commit { id: String },
    Checkout { from_branch: String, to_branch: String },
    Merge { source: String },
    Rebase { onto: String },
    BranchCreate { name: String },
    BranchDelete { name: String, target: String },
    Reset { mode: ResetMode, target: String },
    Revert { commit_id: String },
    CherryPick { commit_id: String },
    Stash,
    StashPop { index: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResetMode {
    Soft,
    Mixed,
    Hard,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepositorySnapshot {
    pub head_id: String,
    pub head_ref: Option<String>,
    pub index_tree_id: Option<String>,
}

// === Remote Hosting Platform ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PullRequest {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub state: PrState,
    pub source_branch: String,
    pub target_branch: String,
    pub author: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PrState {
    Open,
    Closed,
    Merged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreatePrParams {
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
}

// === Commit Template ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommitTemplate {
    pub id: String,
    pub name: String,
    pub content: String,
}

// === App Settings ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    pub theme: ThemeMode,
    pub language: String,
    pub font_family: String,
    pub font_size: u32,
    pub hotkeys: HashMap<String, String>,
    pub window: WindowState,
    pub commit_templates: Vec<CommitTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use proptest::prelude::*;
    use proptest::collection::vec as prop_vec;
    use proptest::collection::hash_map;

    // === Arbitrary strategies for simple enums ===

    fn arb_repository_state() -> impl Strategy<Value = RepositoryState> {
        prop_oneof![
            Just(RepositoryState::Clean),
            Just(RepositoryState::Merging),
            (0..100usize, 0..100usize)
                .prop_map(|(c, t)| RepositoryState::Rebasing { current: c, total: t }),
            Just(RepositoryState::CherryPicking),
            Just(RepositoryState::Reverting),
        ]
    }

    fn arb_ref_type() -> impl Strategy<Value = RefType> {
        prop_oneof![
            Just(RefType::LocalBranch),
            "\\PC{1,20}".prop_map(|remote| RefType::RemoteBranch { remote }),
            Just(RefType::Tag),
        ]
    }

    fn arb_branch_type() -> impl Strategy<Value = BranchType> {
        prop_oneof![
            Just(BranchType::Local),
            "\\PC{1,20}".prop_map(|remote_name| BranchType::Remote { remote_name }),
        ]
    }

    fn arb_branch_filter() -> impl Strategy<Value = BranchFilter> {
        prop_oneof![
            Just(BranchFilter::All),
            Just(BranchFilter::Local),
            Just(BranchFilter::Remote),
        ]
    }

    fn arb_diff_file_status() -> impl Strategy<Value = DiffFileStatus> {
        prop_oneof![
            Just(DiffFileStatus::Added),
            Just(DiffFileStatus::Deleted),
            Just(DiffFileStatus::Modified),
            Just(DiffFileStatus::Renamed),
            Just(DiffFileStatus::Copied),
        ]
    }

    fn arb_diff_line_type() -> impl Strategy<Value = DiffLineType> {
        prop_oneof![
            Just(DiffLineType::Context),
            Just(DiffLineType::Addition),
            Just(DiffLineType::Deletion),
        ]
    }

    fn arb_file_status_type() -> impl Strategy<Value = FileStatusType> {
        prop_oneof![
            Just(FileStatusType::Untracked),
            Just(FileStatusType::Modified),
            Just(FileStatusType::Staged),
            Just(FileStatusType::Conflict),
            Just(FileStatusType::Deleted),
            Just(FileStatusType::Renamed),
        ]
    }

    fn arb_submodule_status() -> impl Strategy<Value = SubmoduleStatus> {
        prop_oneof![
            Just(SubmoduleStatus::Uninitialized),
            Just(SubmoduleStatus::Initialized),
            Just(SubmoduleStatus::Modified),
            Just(SubmoduleStatus::DetachedHead),
        ]
    }

    fn arb_stash_pop_result() -> impl Strategy<Value = StashPopResult> {
        prop_oneof![
            Just(StashPopResult::Success),
            prop_vec("\\PC{1,30}", 0..5)
                .prop_map(|files| StashPopResult::Conflict { files }),
        ]
    }

    fn arb_pull_result() -> impl Strategy<Value = PullResult> {
        prop_oneof![
            Just(PullResult::FastForward),
            Just(PullResult::Merged),
            prop_vec("\\PC{1,30}", 0..5).prop_map(|files| PullResult::Conflict { files }),
            Just(PullResult::UpToDate),
        ]
    }

    fn arb_merge_result() -> impl Strategy<Value = MergeResult> {
        prop_oneof![
            Just(MergeResult::FastForward),
            Just(MergeResult::Merged),
            prop_vec("\\PC{1,30}", 0..5).prop_map(|files| MergeResult::Conflict { files }),
        ]
    }

    fn arb_cherry_pick_result() -> impl Strategy<Value = CherryPickResult> {
        prop_oneof![
            prop_vec("\\PC{1,40}", 0..5)
                .prop_map(|new_commits| CherryPickResult::Success { new_commits }),
            (prop_vec("\\PC{1,30}", 0..5), "\\PC{1,40}")
                .prop_map(|(files, at_commit)| CherryPickResult::Conflict { files, at_commit }),
        ]
    }

    fn arb_revert_result() -> impl Strategy<Value = RevertResult> {
        prop_oneof![
            prop_vec("\\PC{1,40}", 0..5)
                .prop_map(|new_commits| RevertResult::Success { new_commits }),
            (prop_vec("\\PC{1,30}", 0..5), "\\PC{1,40}")
                .prop_map(|(files, at_commit)| RevertResult::Conflict { files, at_commit }),
        ]
    }

    fn arb_rebase_step_status() -> impl Strategy<Value = RebaseStepStatus> {
        prop_oneof![
            Just(RebaseStepStatus::InProgress),
            prop_vec("\\PC{1,30}", 0..5)
                .prop_map(|files| RebaseStepStatus::Conflict { files }),
            Just(RebaseStepStatus::Completed),
        ]
    }

    fn arb_reset_mode() -> impl Strategy<Value = ResetMode> {
        prop_oneof![
            Just(ResetMode::Soft),
            Just(ResetMode::Mixed),
            Just(ResetMode::Hard),
        ]
    }

    fn arb_pr_state() -> impl Strategy<Value = PrState> {
        prop_oneof![
            Just(PrState::Open),
            Just(PrState::Closed),
            Just(PrState::Merged),
        ]
    }

    fn arb_theme_mode() -> impl Strategy<Value = ThemeMode> {
        prop_oneof![
            Just(ThemeMode::Light),
            Just(ThemeMode::Dark),
            Just(ThemeMode::System),
        ]
    }

    fn arb_git_operation() -> impl Strategy<Value = GitOperation> {
        prop_oneof![
            "\\PC{1,40}".prop_map(|id| GitOperation::Commit { id }),
            ("\\PC{1,20}", "\\PC{1,20}")
                .prop_map(|(from_branch, to_branch)| GitOperation::Checkout { from_branch, to_branch }),
            "\\PC{1,20}".prop_map(|source| GitOperation::Merge { source }),
            "\\PC{1,20}".prop_map(|onto| GitOperation::Rebase { onto }),
            "\\PC{1,20}".prop_map(|name| GitOperation::BranchCreate { name }),
            ("\\PC{1,20}", "\\PC{1,40}")
                .prop_map(|(name, target)| GitOperation::BranchDelete { name, target }),
            (arb_reset_mode(), "\\PC{1,40}")
                .prop_map(|(mode, target)| GitOperation::Reset { mode, target }),
            "\\PC{1,40}".prop_map(|commit_id| GitOperation::Revert { commit_id }),
            "\\PC{1,40}".prop_map(|commit_id| GitOperation::CherryPick { commit_id }),
            Just(GitOperation::Stash),
            (0..100usize).prop_map(|index| GitOperation::StashPop { index }),
        ]
    }

    // === Arbitrary strategies for structs ===

    fn arb_tab_id() -> impl Strategy<Value = TabId> {
        "\\PC{1,50}".prop_map(TabId)
    }

    fn arb_repo_entry() -> impl Strategy<Value = RepoEntry> {
        ("\\PC{1,50}", "\\PC{1,30}", -1_000_000_000i64..1_000_000_000i64).prop_map(
            |(path, name, ts)| RepoEntry {
                path,
                name,
                last_opened: DateTime::from_timestamp(ts, 0).unwrap_or_default(),
            },
        )
    }

    fn arb_signature_info() -> impl Strategy<Value = SignatureInfo> {
        ("\\PC{1,30}", "\\PC{1,30}", any::<i64>()).prop_map(|(name, email, timestamp)| {
            SignatureInfo { name, email, timestamp }
        })
    }

    fn arb_ref_label() -> impl Strategy<Value = RefLabel> {
        ("\\PC{1,30}", arb_ref_type(), any::<bool>()).prop_map(|(name, ref_type, is_head)| {
            RefLabel { name, ref_type, is_head }
        })
    }

    fn arb_commit_info() -> impl Strategy<Value = CommitInfo> {
        (
            "\\PC{1,40}",
            "\\PC{1,10}",
            "\\PC{1,50}",
            arb_signature_info(),
            arb_signature_info(),
            prop_vec("\\PC{1,40}", 0..3),
            prop_vec(arb_ref_label(), 0..3),
            any::<bool>(),
        )
            .prop_map(
                |(id, short_id, message, author, committer, parent_ids, refs, is_cherry_picked)| {
                    CommitInfo { id, short_id, message, author, committer, parent_ids, refs, is_cherry_picked }
                },
            )
    }

    fn arb_diff_stats() -> impl Strategy<Value = DiffStats> {
        (0..1000usize, 0..1000usize, 0..1000usize).prop_map(
            |(files_changed, insertions, deletions)| DiffStats { files_changed, insertions, deletions },
        )
    }

    fn arb_file_status() -> impl Strategy<Value = FileStatus> {
        ("\\PC{1,50}", arb_file_status_type()).prop_map(|(path, status)| FileStatus { path, status })
    }

    fn arb_commit_detail() -> impl Strategy<Value = CommitDetail> {
        (arb_commit_info(), prop_vec(arb_file_status(), 0..3), arb_diff_stats()).prop_map(
            |(commit, files, stats)| CommitDetail { commit, files, stats },
        )
    }

    fn arb_dag_edge() -> impl Strategy<Value = DagEdge> {
        (0..100usize, 0..100usize, 0..100usize, 0..100usize).prop_map(
            |(from_column, to_column, to_row, color_index)| DagEdge {
                from_column, to_column, to_row, color_index,
            },
        )
    }

    fn arb_dag_node() -> impl Strategy<Value = DagNode> {
        (
            "\\PC{1,40}",
            0..100usize,
            0..100usize,
            0..100usize,
            prop_vec(arb_dag_edge(), 0..3),
        )
            .prop_map(|(commit_id, column, row, color_index, parent_edges)| DagNode {
                commit_id, column, row, color_index, parent_edges,
            })
    }

    fn arb_dag_layout() -> impl Strategy<Value = DagLayout> {
        (prop_vec(arb_dag_node(), 0..5), 0..100usize, 0..100usize).prop_map(
            |(nodes, total_columns, total_rows)| DagLayout { nodes, total_columns, total_rows },
        )
    }

    fn arb_branch_info() -> impl Strategy<Value = BranchInfo> {
        (
            "\\PC{1,30}",
            any::<bool>(),
            proptest::option::of("\\PC{1,30}"),
            0..100usize,
            0..100usize,
            "\\PC{1,40}",
            arb_branch_type(),
        )
            .prop_map(|(name, is_head, upstream, ahead, behind, last_commit_id, branch_type)| {
                BranchInfo { name, is_head, upstream, ahead, behind, last_commit_id, branch_type }
            })
    }

    fn arb_diff_line() -> impl Strategy<Value = DiffLine> {
        (
            arb_diff_line_type(),
            proptest::option::of(0..100000u32),
            proptest::option::of(0..100000u32),
            "\\PC{0,50}",
        )
            .prop_map(|(origin, old_lineno, new_lineno, content)| DiffLine {
                origin, old_lineno, new_lineno, content,
            })
    }

    fn arb_diff_hunk() -> impl Strategy<Value = DiffHunk> {
        (
            "\\PC{1,30}",
            0..10000u32,
            0..10000u32,
            0..10000u32,
            0..10000u32,
            prop_vec(arb_diff_line(), 0..3),
        )
            .prop_map(|(header, old_start, old_lines, new_start, new_lines, lines)| DiffHunk {
                header, old_start, old_lines, new_start, new_lines, lines,
            })
    }

    fn arb_file_diff() -> impl Strategy<Value = FileDiff> {
        (
            "\\PC{1,50}",
            proptest::option::of("\\PC{1,50}"),
            arb_diff_file_status(),
            prop_vec(arb_diff_hunk(), 0..3),
            any::<bool>(),
        )
            .prop_map(|(path, old_path, status, hunks, is_binary)| FileDiff {
                path, old_path, status, hunks, is_binary,
            })
    }

    fn arb_line_range() -> impl Strategy<Value = LineRange> {
        (0..100000u32, 0..100000u32).prop_map(|(start, end)| LineRange { start, end })
    }

    fn arb_blame_line() -> impl Strategy<Value = BlameLine> {
        (0..100000u32, "\\PC{0,50}", "\\PC{1,40}", "\\PC{1,30}", any::<i64>(), 0..100000u32)
            .prop_map(|(line_number, content, commit_id, author, date, original_line)| BlameLine {
                line_number, content, commit_id, author, date, original_line,
            })
    }

    fn arb_blame_info() -> impl Strategy<Value = BlameInfo> {
        ("\\PC{1,50}", prop_vec(arb_blame_line(), 0..5)).prop_map(|(path, lines)| BlameInfo { path, lines })
    }

    fn arb_stash_entry() -> impl Strategy<Value = StashEntry> {
        (0..100usize, "\\PC{1,50}", any::<i64>(), "\\PC{1,40}").prop_map(
            |(index, message, timestamp, commit_id)| StashEntry { index, message, timestamp, commit_id },
        )
    }

    fn arb_submodule_info() -> impl Strategy<Value = SubmoduleInfo> {
        (
            "\\PC{1,30}",
            "\\PC{1,50}",
            "\\PC{1,50}",
            proptest::option::of("\\PC{1,40}"),
            arb_submodule_status(),
            proptest::option::of("\\PC{1,20}"),
        )
            .prop_map(|(name, path, url, head_id, status, branch)| SubmoduleInfo {
                name, path, url, head_id, status, branch,
            })
    }

    fn arb_worktree_info() -> impl Strategy<Value = WorktreeInfo> {
        ("\\PC{1,30}", "\\PC{1,50}", proptest::option::of("\\PC{1,20}"), any::<bool>()).prop_map(
            |(name, path, branch, is_main)| WorktreeInfo { name, path, branch, is_main },
        )
    }

    fn arb_remote_info() -> impl Strategy<Value = RemoteInfo> {
        ("\\PC{1,20}", "\\PC{1,50}", proptest::option::of("\\PC{1,50}")).prop_map(
            |(name, url, push_url)| RemoteInfo { name, url, push_url },
        )
    }

    fn arb_rebase_progress() -> impl Strategy<Value = RebaseProgress> {
        (0..100usize, 0..100usize, arb_rebase_step_status()).prop_map(
            |(current_step, total_steps, status)| RebaseProgress { current_step, total_steps, status },
        )
    }

    fn arb_log_options() -> impl Strategy<Value = LogOptions> {
        (
            proptest::option::of("\\PC{1,20}"),
            proptest::option::of("\\PC{1,20}"),
            proptest::option::of(any::<i64>()),
            proptest::option::of(any::<i64>()),
            proptest::option::of("\\PC{1,50}"),
            proptest::option::of("\\PC{1,30}"),
            0..1000usize,
            0..1000usize,
        )
            .prop_map(|(branch, author, since, until, path, search, offset, limit)| LogOptions {
                branch, author, since, until, path, search, offset, limit,
            })
    }

    fn arb_git_config() -> impl Strategy<Value = GitConfig> {
        (
            proptest::option::of("\\PC{1,30}"),
            proptest::option::of("\\PC{1,30}"),
            proptest::option::of("\\PC{1,20}"),
            proptest::option::of("\\PC{1,20}"),
        )
            .prop_map(|(user_name, user_email, default_branch, merge_strategy)| GitConfig {
                user_name, user_email, default_branch, merge_strategy,
            })
    }

    fn arb_progress_event() -> impl Strategy<Value = ProgressEvent> {
        (
            "\\PC{1,30}",
            any::<u64>(),
            proptest::option::of(any::<u64>()),
            proptest::option::of("\\PC{1,50}"),
        )
            .prop_map(|(operation, current, total, message)| ProgressEvent {
                operation, current, total, message,
            })
    }

    fn arb_file_change_event() -> impl Strategy<Value = FileChangeEvent> {
        (arb_tab_id(), prop_vec("\\PC{1,50}", 0..5)).prop_map(|(tab_id, changed_paths)| {
            FileChangeEvent { tab_id, changed_paths }
        })
    }

    fn arb_repository_snapshot() -> impl Strategy<Value = RepositorySnapshot> {
        (
            "\\PC{1,40}",
            proptest::option::of("\\PC{1,30}"),
            proptest::option::of("\\PC{1,40}"),
        )
            .prop_map(|(head_id, head_ref, index_tree_id)| RepositorySnapshot {
                head_id, head_ref, index_tree_id,
            })
    }

    fn arb_pull_request() -> impl Strategy<Value = PullRequest> {
        (
            any::<u64>(),
            "\\PC{1,50}",
            "\\PC{1,100}",
            arb_pr_state(),
            "\\PC{1,30}",
            "\\PC{1,30}",
            "\\PC{1,30}",
            "\\PC{1,50}",
        )
            .prop_map(|(id, title, description, state, source_branch, target_branch, author, url)| {
                PullRequest { id, title, description, state, source_branch, target_branch, author, url }
            })
    }

    fn arb_create_pr_params() -> impl Strategy<Value = CreatePrParams> {
        ("\\PC{1,50}", "\\PC{1,100}", "\\PC{1,30}", "\\PC{1,30}").prop_map(
            |(title, description, source_branch, target_branch)| CreatePrParams {
                title, description, source_branch, target_branch,
            },
        )
    }

    fn arb_commit_template() -> impl Strategy<Value = CommitTemplate> {
        ("\\PC{1,30}", "\\PC{1,30}", "\\PC{1,100}").prop_map(|(id, name, content)| CommitTemplate {
            id, name, content,
        })
    }

    fn arb_window_state() -> impl Strategy<Value = WindowState> {
        (
            1..5000u32,
            1..5000u32,
            proptest::option::of(-5000..5000i32),
            proptest::option::of(-5000..5000i32),
            any::<bool>(),
        )
            .prop_map(|(width, height, x, y, maximized)| WindowState { width, height, x, y, maximized })
    }

    fn arb_app_settings() -> impl Strategy<Value = AppSettings> {
        (
            arb_theme_mode(),
            "\\PC{1,10}",
            "\\PC{1,30}",
            1..100u32,
            hash_map("\\PC{1,20}", "\\PC{1,20}", 0..5),
            arb_window_state(),
            prop_vec(arb_commit_template(), 0..3),
        )
            .prop_map(|(theme, language, font_family, font_size, hotkeys, window, commit_templates)| {
                AppSettings { theme, language, font_family, font_size, hotkeys, window, commit_templates }
            })
    }

    /// **Validates: Requirements 16.3, 16.4**
    ///
    /// Property 14: IPC Serialization Roundtrip Consistency
    /// For any valid IPC command request or response object, serializing via
    /// serde_json and then deserializing SHALL produce an object equal to the original.
    mod prop14_ipc_serialization_roundtrip {
        use super::*;

        /// Helper: serialize then deserialize, assert equality.
        fn roundtrip<T>(val: &T)
        where
            T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
        {
            let json = serde_json::to_string(val).expect("serialization failed");
            let back: T = serde_json::from_str(&json).expect("deserialization failed");
            assert_eq!(*val, back);
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn roundtrip_tab_id(v in arb_tab_id()) { roundtrip(&v); }

            #[test]
            fn roundtrip_repo_entry(v in arb_repo_entry()) { roundtrip(&v); }

            #[test]
            fn roundtrip_repository_state(v in arb_repository_state()) { roundtrip(&v); }

            #[test]
            fn roundtrip_signature_info(v in arb_signature_info()) { roundtrip(&v); }

            #[test]
            fn roundtrip_ref_label(v in arb_ref_label()) { roundtrip(&v); }

            #[test]
            fn roundtrip_ref_type(v in arb_ref_type()) { roundtrip(&v); }

            #[test]
            fn roundtrip_commit_info(v in arb_commit_info()) { roundtrip(&v); }

            #[test]
            fn roundtrip_commit_detail(v in arb_commit_detail()) { roundtrip(&v); }

            #[test]
            fn roundtrip_dag_edge(v in arb_dag_edge()) { roundtrip(&v); }

            #[test]
            fn roundtrip_dag_node(v in arb_dag_node()) { roundtrip(&v); }

            #[test]
            fn roundtrip_dag_layout(v in arb_dag_layout()) { roundtrip(&v); }

            #[test]
            fn roundtrip_branch_info(v in arb_branch_info()) { roundtrip(&v); }

            #[test]
            fn roundtrip_branch_type(v in arb_branch_type()) { roundtrip(&v); }

            #[test]
            fn roundtrip_branch_filter(v in arb_branch_filter()) { roundtrip(&v); }

            #[test]
            fn roundtrip_diff_line(v in arb_diff_line()) { roundtrip(&v); }

            #[test]
            fn roundtrip_diff_hunk(v in arb_diff_hunk()) { roundtrip(&v); }

            #[test]
            fn roundtrip_file_diff(v in arb_file_diff()) { roundtrip(&v); }

            #[test]
            fn roundtrip_diff_file_status(v in arb_diff_file_status()) { roundtrip(&v); }

            #[test]
            fn roundtrip_diff_line_type(v in arb_diff_line_type()) { roundtrip(&v); }

            #[test]
            fn roundtrip_diff_stats(v in arb_diff_stats()) { roundtrip(&v); }

            #[test]
            fn roundtrip_line_range(v in arb_line_range()) { roundtrip(&v); }

            #[test]
            fn roundtrip_file_status(v in arb_file_status()) { roundtrip(&v); }

            #[test]
            fn roundtrip_file_status_type(v in arb_file_status_type()) { roundtrip(&v); }

            #[test]
            fn roundtrip_blame_line(v in arb_blame_line()) { roundtrip(&v); }

            #[test]
            fn roundtrip_blame_info(v in arb_blame_info()) { roundtrip(&v); }

            #[test]
            fn roundtrip_stash_entry(v in arb_stash_entry()) { roundtrip(&v); }

            #[test]
            fn roundtrip_stash_pop_result(v in arb_stash_pop_result()) { roundtrip(&v); }

            #[test]
            fn roundtrip_submodule_info(v in arb_submodule_info()) { roundtrip(&v); }

            #[test]
            fn roundtrip_submodule_status(v in arb_submodule_status()) { roundtrip(&v); }

            #[test]
            fn roundtrip_worktree_info(v in arb_worktree_info()) { roundtrip(&v); }

            #[test]
            fn roundtrip_remote_info(v in arb_remote_info()) { roundtrip(&v); }

            #[test]
            fn roundtrip_pull_result(v in arb_pull_result()) { roundtrip(&v); }

            #[test]
            fn roundtrip_merge_result(v in arb_merge_result()) { roundtrip(&v); }

            #[test]
            fn roundtrip_cherry_pick_result(v in arb_cherry_pick_result()) { roundtrip(&v); }

            #[test]
            fn roundtrip_revert_result(v in arb_revert_result()) { roundtrip(&v); }

            #[test]
            fn roundtrip_rebase_progress(v in arb_rebase_progress()) { roundtrip(&v); }

            #[test]
            fn roundtrip_rebase_step_status(v in arb_rebase_step_status()) { roundtrip(&v); }

            #[test]
            fn roundtrip_log_options(v in arb_log_options()) { roundtrip(&v); }

            #[test]
            fn roundtrip_git_config(v in arb_git_config()) { roundtrip(&v); }

            #[test]
            fn roundtrip_progress_event(v in arb_progress_event()) { roundtrip(&v); }

            #[test]
            fn roundtrip_file_change_event(v in arb_file_change_event()) { roundtrip(&v); }

            #[test]
            fn roundtrip_git_operation(v in arb_git_operation()) { roundtrip(&v); }

            #[test]
            fn roundtrip_reset_mode(v in arb_reset_mode()) { roundtrip(&v); }

            #[test]
            fn roundtrip_repository_snapshot(v in arb_repository_snapshot()) { roundtrip(&v); }

            #[test]
            fn roundtrip_pull_request(v in arb_pull_request()) { roundtrip(&v); }

            #[test]
            fn roundtrip_pr_state(v in arb_pr_state()) { roundtrip(&v); }

            #[test]
            fn roundtrip_create_pr_params(v in arb_create_pr_params()) { roundtrip(&v); }

            #[test]
            fn roundtrip_commit_template(v in arb_commit_template()) { roundtrip(&v); }

            #[test]
            fn roundtrip_app_settings(v in arb_app_settings()) { roundtrip(&v); }

            #[test]
            fn roundtrip_theme_mode(v in arb_theme_mode()) { roundtrip(&v); }

            #[test]
            fn roundtrip_window_state(v in arb_window_state()) { roundtrip(&v); }
        }
    }
}
