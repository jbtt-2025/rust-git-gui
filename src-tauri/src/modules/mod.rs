// Business logic modules

pub mod blame_service;
pub mod branch_manager;
pub mod commit_service;
pub mod config_manager;
pub mod rebase_service;
pub mod credential_manager;
pub mod dag_layout;
pub mod diff_service;
pub mod file_watcher;
pub mod remote_manager;
pub mod repository_manager;
pub mod search_engine;
pub mod stash_manager;
pub mod staging_service;
pub mod tag_manager;
pub mod submodule_manager;
pub mod view_filter;
pub mod undo_engine;
pub mod host_integration;
pub mod worktree_manager;

pub use blame_service::BlameService;
pub use branch_manager::BranchManager;
pub use commit_service::CommitService;
pub use config_manager::ConfigManager;
pub use rebase_service::RebaseService;
pub use credential_manager::CredentialManager;
pub use dag_layout::compute_dag_layout;
pub use diff_service::DiffService;
pub use host_integration::{
    GitHubProvider, GitLabProvider, HostError, HostIntegration, HostProvider, UserInfo,
};
pub use file_watcher::FileWatcher;
pub use remote_manager::RemoteManager;
pub use repository_manager::RepositoryManager;
pub use search_engine::{SearchEngine, SearchHighlight, SearchQuery, SearchResult};
pub use staging_service::StagingService;
pub use stash_manager::StashManager;
pub use submodule_manager::SubmoduleManager;
pub use tag_manager::TagManager;
pub use undo_engine::UndoEngine;
pub use view_filter::{apply_hide_filter, apply_pin_to_left, apply_solo_filter, ViewFilter};
pub use worktree_manager::WorktreeManager;
