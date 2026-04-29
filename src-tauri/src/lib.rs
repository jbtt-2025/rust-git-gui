pub mod error;
pub mod git_core;
pub mod ipc;
pub mod models;
pub mod modules;

use std::sync::Mutex;

use ipc::AppState;
use modules::{FileWatcher, RepositoryManager, UndoEngine};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            repo_manager: Mutex::new(RepositoryManager::new()),
            file_watcher: Mutex::new(FileWatcher::new()),
            undo_engine: Mutex::new(UndoEngine::new()),
        })
        .invoke_handler(tauri::generate_handler![
            ipc::open_repository,
            ipc::clone_repository,
            ipc::init_repository,
            ipc::close_repository,
            ipc::get_recent_repos,
            ipc::save_recent_repos,
            ipc::load_recent_repos,
            ipc::remove_recent_repo,
            ipc::get_repo_status,
            ipc::get_commit_log,
            ipc::get_commit_detail,
            ipc::create_commit,
            ipc::amend_commit,
            ipc::search_commits,
            ipc::get_dag_layout,
            ipc::get_working_diff,
            ipc::get_commit_diff,
            ipc::get_file_diff,
            ipc::compare_commits,
            ipc::stage_files,
            ipc::unstage_files,
            ipc::stage_lines,
            ipc::unstage_lines,
            ipc::discard_lines,
            ipc::get_status,
            ipc::get_blame,
            // Branch commands
            ipc::list_branches,
            ipc::create_branch,
            ipc::delete_branch,
            ipc::rename_branch,
            ipc::checkout_branch,
            ipc::set_upstream,
            ipc::merge_branch,
            ipc::reset_branch,
            // Tag commands
            ipc::list_tags,
            ipc::create_lightweight_tag,
            ipc::create_annotated_tag,
            ipc::delete_tag,
            // Remote commands
            ipc::fetch_remote,
            ipc::pull_remote,
            ipc::push_remote,
            ipc::add_remote,
            ipc::remove_remote,
            ipc::list_remotes,
            // Rebase commands
            ipc::start_rebase,
            ipc::continue_rebase,
            ipc::abort_rebase,
            ipc::get_rebase_status,
            // Cherry-pick, Revert, Patch commands
            ipc::cherry_pick,
            ipc::revert_commits,
            ipc::create_patch,
            // Stash commands
            ipc::create_stash,
            ipc::list_stashes,
            ipc::apply_stash,
            ipc::pop_stash,
            ipc::drop_stash,
            ipc::stash_diff,
            // Submodule commands
            ipc::list_submodules,
            ipc::init_submodule,
            ipc::update_submodule,
            ipc::deinit_submodule,
            ipc::set_submodule_url,
            ipc::set_submodule_branch,
            // Worktree commands
            ipc::create_worktree,
            ipc::list_worktrees,
            ipc::delete_worktree,
            // Undo/Redo commands
            ipc::undo_operation,
            ipc::redo_operation,
            ipc::can_undo,
            ipc::can_redo,
            // Config commands
            ipc::get_git_config,
            ipc::set_git_config,
            ipc::save_app_settings,
            ipc::load_app_settings,
            // Host integration commands
            ipc::get_repo_web_url,
            ipc::get_commit_web_url,
            ipc::get_branch_web_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
