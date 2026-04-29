// Tauri IPC command handlers

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::IpcError;
use crate::git_core::ProgressSender;
use crate::models::{
    AppSettings, BlameInfo, BranchFilter, BranchInfo, CherryPickResult, CommitDetail, CommitInfo,
    DagLayout, FileChangeEvent, FileDiff, FileStatus, GitConfig, LineRange, LogOptions,
    MergeResult, ProgressEvent, PullResult, RebaseProgress, RemoteInfo, RepoEntry,
    RepositoryState, ResetMode, RevertResult, StashEntry, StashPopResult, SubmoduleInfo, TabId,
    TagInfo, WorktreeInfo,
};
use crate::modules::config_manager::ConfigLevel;
use crate::modules::file_watcher::{FileChangeCallback, FileWatcher};
use crate::modules::{
    compute_dag_layout, BlameService, BranchManager, CommitService, ConfigManager, DiffService,
    GitHubProvider, GitLabProvider, HostProvider, RebaseService, RemoteManager,
    RepositoryManager, SearchEngine, SearchQuery, SearchResult, StagingService, StashManager,
    SubmoduleManager, TagManager, UndoEngine, WorktreeManager,
};

/// Shared application state managed by Tauri.
pub struct AppState {
    pub repo_manager: Mutex<RepositoryManager>,
    pub file_watcher: Mutex<FileWatcher>,
    pub undo_engine: Mutex<UndoEngine>,
}

/// Start the FileWatcher for a repo's workdir and wire events to the frontend.
fn start_watcher(
    tab_id: &TabId,
    workdir: &std::path::Path,
    state: &State<'_, AppState>,
    app: &AppHandle,
) -> Result<(), IpcError> {
    let app_handle = app.clone();
    let callback: FileChangeCallback = Arc::new(move |evt: FileChangeEvent| {
        let _ = app_handle.emit("file-changed", &evt);
    });
    let mut fw = state.file_watcher.lock().unwrap();
    fw.watch(tab_id.clone(), workdir, callback)
        .map_err(|e| IpcError::from(e))
}

#[tauri::command]
pub async fn open_repository(
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<TabId, IpcError> {
    let tab_id = {
        let mut mgr = state.repo_manager.lock().unwrap();
        let tab_id = mgr.open_repo(PathBuf::from(&path))?;
        tab_id
    };

    // Start file watcher for the repo's workdir
    {
        let mgr = state.repo_manager.lock().unwrap();
        if let Some(repo) = mgr.get_repo(&tab_id) {
            if let Some(workdir) = repo.workdir() {
                let wd = workdir.to_path_buf();
                drop(mgr);
                start_watcher(&tab_id, &wd, &state, &app)?;
            }
        }
    }

    Ok(tab_id)
}

#[tauri::command]
pub async fn clone_repository(
    url: String,
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<TabId, IpcError> {
    // Build a progress sender that emits Tauri events
    let app_handle = app.clone();
    let progress: ProgressSender = Arc::new(move |received, total, bytes| {
        let _ = app_handle.emit(
            "operation-progress",
            &ProgressEvent {
                operation: "clone".to_string(),
                current: received as u64,
                total: Some(total as u64),
                message: Some(format!("Received {} bytes", bytes)),
            },
        );
    });

    let tab_id = {
        let mut mgr = state.repo_manager.lock().unwrap();
        mgr.clone_repo(url, PathBuf::from(&path), progress)?
    };

    // Start file watcher
    {
        let mgr = state.repo_manager.lock().unwrap();
        if let Some(repo) = mgr.get_repo(&tab_id) {
            if let Some(workdir) = repo.workdir() {
                let wd = workdir.to_path_buf();
                drop(mgr);
                start_watcher(&tab_id, &wd, &state, &app)?;
            }
        }
    }

    Ok(tab_id)
}

#[tauri::command]
pub async fn init_repository(
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<TabId, IpcError> {
    let tab_id = {
        let mut mgr = state.repo_manager.lock().unwrap();
        mgr.init_repo(PathBuf::from(&path))?
    };

    // Start file watcher
    {
        let mgr = state.repo_manager.lock().unwrap();
        if let Some(repo) = mgr.get_repo(&tab_id) {
            if let Some(workdir) = repo.workdir() {
                let wd = workdir.to_path_buf();
                drop(mgr);
                start_watcher(&tab_id, &wd, &state, &app)?;
            }
        }
    }

    Ok(tab_id)
}

#[tauri::command]
pub async fn close_repository(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    // Stop file watcher first
    {
        let mut fw = state.file_watcher.lock().unwrap();
        fw.unwatch(&tab_id);
    }
    // Close the repo
    {
        let mut mgr = state.repo_manager.lock().unwrap();
        mgr.close_repo(&tab_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_recent_repos(
    state: State<'_, AppState>,
) -> Result<Vec<RepoEntry>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    Ok(mgr.recent_repos().iter().cloned().collect())
}

// === Recent Repos Persistence Commands ===

#[tauri::command]
pub async fn save_recent_repos(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let repos: Vec<RepoEntry> = {
        let mgr = state.repo_manager.lock().unwrap();
        mgr.recent_repos().iter().cloned().collect()
    };

    let app_data_dir = app.path().app_data_dir().map_err(|e| IpcError {
        error_type: "Io".to_string(),
        message: format!("Failed to resolve app data directory: {}", e),
    })?;

    std::fs::create_dir_all(&app_data_dir).map_err(|e| IpcError {
        error_type: "Io".to_string(),
        message: format!("Failed to create app data directory: {}", e),
    })?;

    let file_path = app_data_dir.join("recent_repos.json");
    let json = serde_json::to_string_pretty(&repos).map_err(|e| IpcError {
        error_type: "Io".to_string(),
        message: format!("Failed to serialize recent repos: {}", e),
    })?;

    std::fs::write(&file_path, json).map_err(|e| IpcError {
        error_type: "Io".to_string(),
        message: format!("Failed to write recent_repos.json: {}", e),
    })?;

    Ok(())
}

#[tauri::command]
pub async fn load_recent_repos(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<RepoEntry>, IpcError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| IpcError {
        error_type: "Io".to_string(),
        message: format!("Failed to resolve app data directory: {}", e),
    })?;

    let file_path = app_data_dir.join("recent_repos.json");

    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let contents = std::fs::read_to_string(&file_path).map_err(|e| IpcError {
        error_type: "Io".to_string(),
        message: format!("Failed to read recent_repos.json: {}", e),
    })?;

    let repos: Vec<RepoEntry> = serde_json::from_str(&contents).map_err(|e| IpcError {
        error_type: "Io".to_string(),
        message: format!("Failed to deserialize recent_repos.json: {}", e),
    })?;

    // Load into RepositoryManager's in-memory state
    {
        let mut mgr = state.repo_manager.lock().unwrap();
        mgr.set_recent_repos(repos.clone().into());
    }

    Ok(repos)
}

#[tauri::command]
pub async fn remove_recent_repo(
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    {
        let mut mgr = state.repo_manager.lock().unwrap();
        mgr.remove_recent_repo(&path);
    }

    // Persist the updated list
    save_recent_repos(state, app).await
}

#[tauri::command]
pub async fn get_repo_status(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<RepositoryState, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    Ok(mgr.repo_status(&tab_id)?)
}

#[tauri::command]
pub async fn get_commit_log(
    tab_id: TabId,
    options: LogOptions,
    state: State<'_, AppState>,
) -> Result<Vec<CommitInfo>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = CommitService::new();
    Ok(service.commit_log(repo, options).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn get_commit_detail(
    tab_id: TabId,
    commit_id: String,
    state: State<'_, AppState>,
) -> Result<CommitDetail, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = CommitService::new();
    Ok(service.commit_detail(repo, &commit_id).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn create_commit(
    tab_id: TabId,
    message: String,
    state: State<'_, AppState>,
) -> Result<CommitInfo, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = CommitService::new();
    Ok(service.create_commit(repo, &message).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn amend_commit(
    tab_id: TabId,
    message: String,
    state: State<'_, AppState>,
) -> Result<CommitInfo, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = CommitService::new();
    Ok(service.amend_commit(repo, &message).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn search_commits(
    tab_id: TabId,
    query: SearchQuery,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = CommitService::new();
    let all_options = LogOptions {
        branch: None,
        author: None,
        since: None,
        until: None,
        path: None,
        search: None,
        offset: 0,
        limit: usize::MAX,
    };
    let commits = service.commit_log(repo, all_options).map_err(IpcError::from)?;
    let engine = SearchEngine::new();
    Ok(engine.search(&commits, &query))
}

#[tauri::command]
pub async fn get_dag_layout(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<DagLayout, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = CommitService::new();
    let all_options = LogOptions {
        branch: None,
        author: None,
        since: None,
        until: None,
        path: None,
        search: None,
        offset: 0,
        limit: usize::MAX,
    };
    let commits = service.commit_log(repo, all_options).map_err(IpcError::from)?;
    Ok(compute_dag_layout(&commits))
}

// === Diff Commands ===

#[tauri::command]
pub async fn get_working_diff(
    tab_id: TabId,
    ignore_whitespace: bool,
    state: State<'_, AppState>,
) -> Result<Vec<FileDiff>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = DiffService::new();
    Ok(service.working_diff(repo, ignore_whitespace).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn get_commit_diff(
    tab_id: TabId,
    commit_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<FileDiff>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = DiffService::new();
    Ok(service.commit_diff(repo, &commit_id).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn get_file_diff(
    tab_id: TabId,
    path: String,
    staged: bool,
    state: State<'_, AppState>,
) -> Result<FileDiff, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = DiffService::new();
    Ok(service.file_diff(repo, &path, staged).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn compare_commits(
    tab_id: TabId,
    from: String,
    to: String,
    state: State<'_, AppState>,
) -> Result<Vec<FileDiff>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = DiffService::new();
    Ok(service.compare_commits(repo, &from, &to).map_err(IpcError::from)?)
}

// === Staging Commands ===

#[tauri::command]
pub async fn stage_files(
    tab_id: TabId,
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StagingService::new();
    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
    Ok(service.stage_files(repo, &path_refs).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn unstage_files(
    tab_id: TabId,
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StagingService::new();
    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
    Ok(service.unstage_files(repo, &path_refs).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn stage_lines(
    tab_id: TabId,
    path: String,
    line_ranges: Vec<LineRange>,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StagingService::new();
    Ok(service.stage_lines(repo, &path, &line_ranges).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn unstage_lines(
    tab_id: TabId,
    path: String,
    line_ranges: Vec<LineRange>,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StagingService::new();
    Ok(service.unstage_lines(repo, &path, &line_ranges).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn discard_lines(
    tab_id: TabId,
    path: String,
    line_ranges: Vec<LineRange>,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StagingService::new();
    Ok(service.discard_lines(repo, &path, &line_ranges).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn get_status(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<Vec<FileStatus>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StagingService::new();
    Ok(service.status(repo).map_err(IpcError::from)?)
}

// === Blame Commands ===

#[tauri::command]
pub async fn get_blame(
    tab_id: TabId,
    path: String,
    state: State<'_, AppState>,
) -> Result<BlameInfo, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = BlameService::new();
    Ok(service.blame(repo, &path).map_err(IpcError::from)?)
}

// === Branch Commands ===

#[tauri::command]
pub async fn list_branches(
    tab_id: TabId,
    filter: BranchFilter,
    state: State<'_, AppState>,
) -> Result<Vec<BranchInfo>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = BranchManager::new();
    Ok(service.list_branches(repo, filter).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn create_branch(
    tab_id: TabId,
    name: String,
    target: Option<String>,
    state: State<'_, AppState>,
) -> Result<BranchInfo, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = BranchManager::new();
    Ok(service
        .create_branch(repo, &name, target.as_deref())
        .map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn delete_branch(
    tab_id: TabId,
    name: String,
    force: bool,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = BranchManager::new();
    Ok(service.delete_branch(repo, &name, force).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn rename_branch(
    tab_id: TabId,
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = BranchManager::new();
    Ok(service.rename_branch(repo, &old_name, &new_name).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn checkout_branch(
    tab_id: TabId,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = BranchManager::new();
    Ok(service.checkout_branch(repo, &name).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn set_upstream(
    tab_id: TabId,
    local: String,
    remote: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = BranchManager::new();
    Ok(service.set_upstream(repo, &local, &remote).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn merge_branch(
    tab_id: TabId,
    source: String,
    state: State<'_, AppState>,
) -> Result<MergeResult, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = BranchManager::new();
    Ok(service.merge(repo, &source).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn reset_branch(
    tab_id: TabId,
    target: String,
    mode: ResetMode,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = BranchManager::new();
    Ok(service.reset(repo, &target, mode).map_err(IpcError::from)?)
}

// === Tag Commands ===

#[tauri::command]
pub async fn list_tags(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<Vec<TagInfo>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = TagManager::new();
    Ok(service.list_tags(repo).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn create_lightweight_tag(
    tab_id: TabId,
    name: String,
    target: Option<String>,
    state: State<'_, AppState>,
) -> Result<TagInfo, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = TagManager::new();
    Ok(service
        .create_lightweight_tag(repo, &name, target.as_deref())
        .map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn create_annotated_tag(
    tab_id: TabId,
    name: String,
    target: Option<String>,
    message: String,
    state: State<'_, AppState>,
) -> Result<TagInfo, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = TagManager::new();
    Ok(service
        .create_annotated_tag(repo, &name, target.as_deref(), &message)
        .map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn delete_tag(
    tab_id: TabId,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = TagManager::new();
    Ok(service.delete_tag(repo, &name).map_err(IpcError::from)?)
}

// === Remote Commands ===

fn make_progress_sender(app: &AppHandle, operation: &str) -> ProgressSender {
    let app_handle = app.clone();
    let op = operation.to_string();
    Arc::new(move |received, total, bytes| {
        let _ = app_handle.emit(
            "operation-progress",
            &ProgressEvent {
                operation: op.clone(),
                current: received as u64,
                total: Some(total as u64),
                message: Some(format!("Received {} bytes", bytes)),
            },
        );
    })
}

#[tauri::command]
pub async fn fetch_remote(
    tab_id: TabId,
    remote: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let progress = make_progress_sender(&app, "fetch");
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = RemoteManager::new();
    Ok(service.fetch(repo, remote.as_deref(), progress).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn pull_remote(
    tab_id: TabId,
    remote: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<PullResult, IpcError> {
    let progress = make_progress_sender(&app, "pull");
    let mut mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo_mut(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = RemoteManager::new();
    Ok(service.pull(repo, remote.as_deref(), progress).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn push_remote(
    tab_id: TabId,
    remote: Option<String>,
    force: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let progress = make_progress_sender(&app, "push");
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = RemoteManager::new();
    Ok(service.push(repo, remote.as_deref(), force, progress).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn add_remote(
    tab_id: TabId,
    name: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = RemoteManager::new();
    Ok(service.add_remote(repo, &name, &url).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn remove_remote(
    tab_id: TabId,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = RemoteManager::new();
    Ok(service.remove_remote(repo, &name).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn list_remotes(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<Vec<RemoteInfo>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = RemoteManager::new();
    Ok(service.list_remotes(repo).map_err(IpcError::from)?)
}

// === Rebase Commands ===

#[tauri::command]
pub async fn start_rebase(
    tab_id: TabId,
    onto: String,
    state: State<'_, AppState>,
) -> Result<RebaseProgress, IpcError> {
    let mut mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo_mut(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = RebaseService::new();
    Ok(service.start_rebase(repo, &onto).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn continue_rebase(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<RebaseProgress, IpcError> {
    let mut mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo_mut(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = RebaseService::new();
    Ok(service.continue_rebase(repo).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn abort_rebase(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mut mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo_mut(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = RebaseService::new();
    Ok(service.abort_rebase(repo).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn get_rebase_status(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<Option<RebaseProgress>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = RebaseService::new();
    Ok(service.rebase_status(repo).map_err(IpcError::from)?)
}

// === Cherry-pick & Revert Commands ===

#[tauri::command]
pub async fn cherry_pick(
    tab_id: TabId,
    commit_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<CherryPickResult, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = CommitService::new();
    let id_refs: Vec<&str> = commit_ids.iter().map(|s| s.as_str()).collect();
    Ok(service.cherry_pick(repo, &id_refs).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn revert_commits(
    tab_id: TabId,
    commit_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<RevertResult, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = CommitService::new();
    let id_refs: Vec<&str> = commit_ids.iter().map(|s| s.as_str()).collect();
    Ok(service.revert(repo, &id_refs).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn create_patch(
    tab_id: TabId,
    commit_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<u8>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = CommitService::new();
    Ok(service.create_patch(repo, &commit_id).map_err(IpcError::from)?)
}

// === Stash Commands ===

#[tauri::command]
pub async fn create_stash(
    tab_id: TabId,
    message: Option<String>,
    state: State<'_, AppState>,
) -> Result<StashEntry, IpcError> {
    let mut mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo_mut(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StashManager::new();
    Ok(service.create_stash(repo, message.as_deref()).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn list_stashes(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<Vec<StashEntry>, IpcError> {
    let mut mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo_mut(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StashManager::new();
    Ok(service.list_stashes(repo).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn apply_stash(
    tab_id: TabId,
    index: usize,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mut mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo_mut(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StashManager::new();
    Ok(service.apply_stash(repo, index).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn pop_stash(
    tab_id: TabId,
    index: usize,
    state: State<'_, AppState>,
) -> Result<StashPopResult, IpcError> {
    let mut mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo_mut(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StashManager::new();
    Ok(service.pop_stash(repo, index).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn drop_stash(
    tab_id: TabId,
    index: usize,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mut mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo_mut(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StashManager::new();
    Ok(service.drop_stash(repo, index).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn stash_diff(
    tab_id: TabId,
    index: usize,
    state: State<'_, AppState>,
) -> Result<Vec<FileDiff>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = StashManager::new();
    Ok(service.stash_diff(repo, index).map_err(IpcError::from)?)
}

// === Submodule Commands ===

#[tauri::command]
pub async fn list_submodules(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<Vec<SubmoduleInfo>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = SubmoduleManager::new();
    Ok(service.list_submodules(repo).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn init_submodule(
    tab_id: TabId,
    path: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = SubmoduleManager::new();
    Ok(service.init_submodule(repo, &path).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn update_submodule(
    tab_id: TabId,
    path: String,
    recursive: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let progress = make_progress_sender(&app, "submodule-update");
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = SubmoduleManager::new();
    Ok(service.update_submodule(repo, &path, recursive, progress).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn deinit_submodule(
    tab_id: TabId,
    path: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = SubmoduleManager::new();
    Ok(service.deinit_submodule(repo, &path).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn set_submodule_url(
    tab_id: TabId,
    path: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = SubmoduleManager::new();
    Ok(service.set_submodule_url(repo, &path, &url).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn set_submodule_branch(
    tab_id: TabId,
    path: String,
    branch: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = SubmoduleManager::new();
    Ok(service.set_submodule_branch(repo, &path, &branch).map_err(IpcError::from)?)
}

// === Worktree Commands ===

#[tauri::command]
pub async fn create_worktree(
    tab_id: TabId,
    name: String,
    path: String,
    branch: Option<String>,
    state: State<'_, AppState>,
) -> Result<WorktreeInfo, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = WorktreeManager::new();
    Ok(service
        .create_worktree(repo, &name, std::path::Path::new(&path), branch.as_deref())
        .map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn list_worktrees(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<Vec<WorktreeInfo>, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = WorktreeManager::new();
    Ok(service.list_worktrees(repo).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn delete_worktree(
    tab_id: TabId,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = WorktreeManager::new();
    Ok(service.delete_worktree(repo, &name).map_err(IpcError::from)?)
}

// === Undo/Redo Commands ===

#[tauri::command]
pub async fn undo_operation(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<String, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let mut undo = state.undo_engine.lock().unwrap();
    Ok(undo.undo(repo).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn redo_operation(
    tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<String, IpcError> {
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let mut undo = state.undo_engine.lock().unwrap();
    Ok(undo.redo(repo).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn can_undo(
    _tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<Option<String>, IpcError> {
    let undo = state.undo_engine.lock().unwrap();
    Ok(undo.can_undo().map(|s| s.to_string()))
}

#[tauri::command]
pub async fn can_redo(
    _tab_id: TabId,
    state: State<'_, AppState>,
) -> Result<Option<String>, IpcError> {
    let undo = state.undo_engine.lock().unwrap();
    Ok(undo.can_redo().map(|s| s.to_string()))
}

// === Config Commands ===

fn parse_config_level(level: &str) -> Result<ConfigLevel, IpcError> {
    match level {
        "local" => Ok(ConfigLevel::Local),
        "global" => Ok(ConfigLevel::Global),
        _ => Err(IpcError {
            error_type: "InvalidArgument".to_string(),
            message: format!("Invalid config level: '{}'. Expected 'local' or 'global'.", level),
        }),
    }
}

#[tauri::command]
pub async fn get_git_config(
    tab_id: TabId,
    level: String,
    state: State<'_, AppState>,
) -> Result<GitConfig, IpcError> {
    let config_level = parse_config_level(&level)?;
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = ConfigManager::new();
    Ok(service.get_git_config(repo, config_level).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn set_git_config(
    tab_id: TabId,
    level: String,
    config: GitConfig,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let config_level = parse_config_level(&level)?;
    let mgr = state.repo_manager.lock().unwrap();
    let repo = mgr
        .get_repo(&tab_id)
        .ok_or_else(|| IpcError {
            error_type: "RepositoryNotFound".to_string(),
            message: format!("No repository open for tab {:?}", tab_id),
        })?;
    let service = ConfigManager::new();
    Ok(service.set_git_config(repo, config_level, &config).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn save_app_settings(
    path: String,
    settings: AppSettings,
) -> Result<(), IpcError> {
    let service = ConfigManager::new();
    Ok(service.save_app_settings(std::path::Path::new(&path), &settings).map_err(IpcError::from)?)
}

#[tauri::command]
pub async fn load_app_settings(
    path: String,
) -> Result<AppSettings, IpcError> {
    let service = ConfigManager::new();
    Ok(service.load_app_settings(std::path::Path::new(&path)).map_err(IpcError::from)?)
}

// === Host Integration Commands ===

fn get_host_provider(platform: &str) -> Result<Box<dyn HostProvider>, IpcError> {
    match platform {
        "github" => Ok(Box::new(GitHubProvider::new())),
        "gitlab" => Ok(Box::new(GitLabProvider::new())),
        _ => Err(IpcError {
            error_type: "InvalidArgument".to_string(),
            message: format!("Unknown platform: '{}'. Expected 'github' or 'gitlab'.", platform),
        }),
    }
}

#[tauri::command]
pub async fn get_repo_web_url(
    remote_url: String,
    platform: String,
) -> Result<Option<String>, IpcError> {
    let provider = get_host_provider(&platform)?;
    Ok(provider.repo_web_url(&remote_url))
}

#[tauri::command]
pub async fn get_commit_web_url(
    remote_url: String,
    sha: String,
    platform: String,
) -> Result<Option<String>, IpcError> {
    let provider = get_host_provider(&platform)?;
    Ok(provider.commit_web_url(&remote_url, &sha))
}

#[tauri::command]
pub async fn get_branch_web_url(
    remote_url: String,
    branch: String,
    platform: String,
) -> Result<Option<String>, IpcError> {
    let provider = get_host_provider(&platform)?;
    Ok(provider.branch_web_url(&remote_url, &branch))
}
