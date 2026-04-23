//! Integration tests for the Git GUI backend.
//!
//! These tests exercise complete Git operation flows using temporary repositories,
//! verifying that the various service modules work together correctly.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

use git_gui_lib::error::GitError;
use git_gui_lib::git_core::GitRepository;
use git_gui_lib::models::*;
use git_gui_lib::modules::*;

// ── Helpers ──

fn temp_dir() -> TempDir {
    TempDir::new().expect("failed to create temp dir")
}

/// Create a repo with an initial commit using git2 directly, then open via GitRepository.
fn init_repo_with_commit(dir: &Path) -> GitRepository {
    {
        let raw = git2::Repository::init(dir).expect("git2 init");
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        fs::write(dir.join("README.md"), "# Test Repo\n").expect("write");
        let mut index = raw.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = raw.find_tree(tree_id).unwrap();
        raw.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
    } // raw dropped here

    GitRepository::open(dir).expect("open")
}

/// Add a file and create a commit using git2 directly.
fn add_file_and_commit_raw(dir: &Path, filename: &str, content: &str, message: &str) {
    let raw = git2::Repository::open(dir).expect("open raw");
    let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
    fs::write(dir.join(filename), content).expect("write");
    let mut index = raw.index().unwrap();
    index.add_path(Path::new(filename)).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = raw.find_tree(tree_id).unwrap();
    let parent = raw.head().unwrap().peel_to_commit().unwrap();
    raw.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
        .unwrap();
}

// ═══════════════════════════════════════════════════════════════════════
// 1. Repository Operations (init, clone, open)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_init_open_repo_flow() {
    let dir = temp_dir();
    let repo = GitRepository::init(dir.path()).expect("init");
    assert!(!repo.is_bare());
    assert_eq!(repo.state(), RepositoryState::Clean);

    let repo2 = GitRepository::open(dir.path()).expect("open");
    assert!(!repo2.is_bare());
}

#[test]
fn test_open_non_git_directory() {
    let dir = temp_dir();
    let result = GitRepository::open(dir.path());
    assert!(matches!(result, Err(GitError::NotARepository { .. })));
}

#[test]
fn test_clone_local_bare_repo() {
    let origin_dir = temp_dir();
    let origin = git2::Repository::init_bare(origin_dir.path()).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let tree_id = origin.index().unwrap().write_tree().unwrap();
    let tree = origin.find_tree(tree_id).unwrap();
    origin
        .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
        .unwrap();

    let clone_dir = temp_dir();
    let progress = std::sync::Arc::new(|_: usize, _: usize, _: usize| {});
    let cloned = GitRepository::clone(
        origin_dir.path().to_str().unwrap(),
        clone_dir.path(),
        progress,
    )
    .expect("clone");
    assert!(!cloned.is_bare());
    let head = cloned.head().expect("head");
    assert!(head.target.is_some());
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Branch Operations (create, delete, rename, checkout, merge)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_branch_create_checkout_delete_flow() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let bm = BranchManager::new();

    let branch = bm.create_branch(&repo, "feature-x", None).expect("create");
    assert_eq!(branch.name, "feature-x");

    let branches = bm.list_branches(&repo, BranchFilter::Local).expect("list");
    assert!(branches.len() >= 2);
    assert!(branches.iter().any(|b| b.name == "feature-x"));

    bm.checkout_branch(&repo, "feature-x").expect("checkout");
    let head = repo.head().expect("head");
    assert_eq!(head.shorthand.as_deref(), Some("feature-x"));

    let default_name = branches
        .iter()
        .find(|b| b.name != "feature-x")
        .map(|b| b.name.clone())
        .unwrap();
    bm.checkout_branch(&repo, &default_name).expect("checkout default");

    bm.delete_branch(&repo, "feature-x", false).expect("delete");
    let branches_after = bm.list_branches(&repo, BranchFilter::Local).expect("list");
    assert!(!branches_after.iter().any(|b| b.name == "feature-x"));
}

#[test]
fn test_branch_rename() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let bm = BranchManager::new();

    bm.create_branch(&repo, "old-name", None).expect("create");
    bm.rename_branch(&repo, "old-name", "new-name").expect("rename");

    let branches = bm.list_branches(&repo, BranchFilter::Local).expect("list");
    assert!(branches.iter().any(|b| b.name == "new-name"));
    assert!(!branches.iter().any(|b| b.name == "old-name"));
}

#[test]
fn test_merge_fast_forward() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let bm = BranchManager::new();

    bm.create_branch(&repo, "feature", None).expect("create");
    bm.checkout_branch(&repo, "feature").expect("checkout");
    add_file_and_commit_raw(dir.path(), "feature.txt", "feature content", "feature commit");

    // Re-open to pick up changes
    let repo = GitRepository::open(dir.path()).expect("reopen");
    let branches = bm.list_branches(&repo, BranchFilter::Local).expect("list");
    let default_name = branches
        .iter()
        .find(|b| b.name != "feature")
        .map(|b| b.name.clone())
        .unwrap();
    bm.checkout_branch(&repo, &default_name).expect("checkout default");

    let result = bm.merge(&repo, "feature").expect("merge");
    assert!(matches!(result, MergeResult::FastForward | MergeResult::Merged));
}

#[test]
fn test_merge_conflict() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let bm = BranchManager::new();

    let branches = bm.list_branches(&repo, BranchFilter::Local).expect("list");
    let default_name = branches.first().map(|b| b.name.clone()).unwrap();

    bm.create_branch(&repo, "conflict-branch", None).expect("create");

    // Modify file on default branch
    add_file_and_commit_raw(dir.path(), "conflict.txt", "default content\n", "default change");

    // Switch to feature and modify same file
    let repo = GitRepository::open(dir.path()).expect("reopen");
    bm.checkout_branch(&repo, "conflict-branch").expect("checkout");
    add_file_and_commit_raw(dir.path(), "conflict.txt", "feature content\n", "feature change");

    // Switch back and merge
    let repo = GitRepository::open(dir.path()).expect("reopen");
    bm.checkout_branch(&repo, &default_name).expect("checkout default");
    let result = bm.merge(&repo, "conflict-branch").expect("merge");
    assert!(matches!(result, MergeResult::Conflict { .. }));
}

// ═══════════════════════════════════════════════════════════════════════
// 3. Remote Operations (fetch, pull, push using local bare repo)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_remote_fetch_pull_push_flow() {
    let origin_dir = temp_dir();
    let origin = git2::Repository::init_bare(origin_dir.path()).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let tree_id = origin.index().unwrap().write_tree().unwrap();
    let tree = origin.find_tree(tree_id).unwrap();
    origin
        .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
        .unwrap();

    let clone_dir = temp_dir();
    let progress = std::sync::Arc::new(|_: usize, _: usize, _: usize| {});
    let repo = GitRepository::clone(
        origin_dir.path().to_str().unwrap(),
        clone_dir.path(),
        progress.clone(),
    )
    .expect("clone");

    let rm = RemoteManager::new();

    let remotes = rm.list_remotes(&repo).expect("list_remotes");
    assert!(remotes.iter().any(|r| r.name == "origin"));

    rm.fetch(&repo, Some("origin"), progress.clone()).expect("fetch");

    add_file_and_commit_raw(clone_dir.path(), "pushed.txt", "pushed\n", "push commit");
    let repo = GitRepository::open(clone_dir.path()).expect("reopen");
    rm.push(&repo, Some("origin"), false, progress.clone()).expect("push");

    let mut repo = GitRepository::open(clone_dir.path()).expect("reopen for pull");
    let pull_result = rm.pull(&mut repo, Some("origin"), progress).expect("pull");
    assert!(matches!(pull_result, PullResult::UpToDate));
}

#[test]
fn test_add_remove_remote() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let rm = RemoteManager::new();

    rm.add_remote(&repo, "upstream", "https://example.com/repo.git").expect("add");
    let remotes = rm.list_remotes(&repo).expect("list");
    assert!(remotes.iter().any(|r| r.name == "upstream"));

    rm.remove_remote(&repo, "upstream").expect("remove");
    let remotes_after = rm.list_remotes(&repo).expect("list after");
    assert!(!remotes_after.iter().any(|r| r.name == "upstream"));
}

// ═══════════════════════════════════════════════════════════════════════
// 4. Stash Operations (save, apply, pop, drop)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_stash_save_apply_pop_drop() {
    let dir = temp_dir();
    let mut repo = init_repo_with_commit(dir.path());
    let sm = StashManager::new();

    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join("README.md"), "modified\n").expect("write");

    let stash = sm.create_stash(&mut repo, Some("test stash")).expect("create_stash");
    assert!(stash.message.contains("test stash"));

    let content = fs::read_to_string(workdir.join("README.md")).expect("read");
    assert_eq!(content.trim(), "# Test Repo");

    let stashes = sm.list_stashes(&mut repo).expect("list");
    assert_eq!(stashes.len(), 1);

    sm.apply_stash(&mut repo, 0).expect("apply");
    let content_after = fs::read_to_string(workdir.join("README.md")).expect("read");
    assert_eq!(content_after.trim(), "modified");

    // Re-stash for pop test
    fs::write(workdir.join("README.md"), "modified again\n").expect("write");
    sm.create_stash(&mut repo, Some("stash 2")).expect("create 2");

    let pop_result = sm.pop_stash(&mut repo, 0).expect("pop");
    assert!(matches!(pop_result, StashPopResult::Success));

    let stashes_before_drop = sm.list_stashes(&mut repo).expect("list");
    if !stashes_before_drop.is_empty() {
        sm.drop_stash(&mut repo, 0).expect("drop");
    }
    let stashes_after = sm.list_stashes(&mut repo).expect("list after");
    assert!(stashes_after.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════
// 5. Reset Operations (Soft, Mixed, Hard)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_reset_soft() {
    let dir = temp_dir();
    let _repo = init_repo_with_commit(dir.path());
    let bm = BranchManager::new();

    add_file_and_commit_raw(dir.path(), "file2.txt", "content\n", "second commit");
    let repo = GitRepository::open(dir.path()).expect("reopen");

    // Get parent commit ID
    let raw = git2::Repository::open(dir.path()).unwrap();
    let head_commit = raw.head().unwrap().peel_to_commit().unwrap();
    let parent_id = head_commit.parent_id(0).unwrap().to_string();

    bm.reset(&repo, &parent_id, ResetMode::Soft).expect("soft reset");

    let new_head = repo.head().expect("head after reset");
    assert_eq!(new_head.target.unwrap(), parent_id);
    assert!(dir.path().join("file2.txt").exists());
}

#[test]
fn test_reset_hard() {
    let dir = temp_dir();
    let _repo = init_repo_with_commit(dir.path());
    let bm = BranchManager::new();

    add_file_and_commit_raw(dir.path(), "file2.txt", "content\n", "second commit");
    let repo = GitRepository::open(dir.path()).expect("reopen");

    let raw = git2::Repository::open(dir.path()).unwrap();
    let head_commit = raw.head().unwrap().peel_to_commit().unwrap();
    let parent_id = head_commit.parent_id(0).unwrap().to_string();

    bm.reset(&repo, &parent_id, ResetMode::Hard).expect("hard reset");

    let new_head = repo.head().expect("head after reset");
    assert_eq!(new_head.target.unwrap(), parent_id);
    assert!(!dir.path().join("file2.txt").exists());
}

// ═══════════════════════════════════════════════════════════════════════
// 6. Commit, Diff, Staging, and Blame
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_commit_and_log_flow() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let cs = CommitService::new();

    let options = LogOptions {
        branch: None,
        author: None,
        since: None,
        until: None,
        path: None,
        search: None,
        offset: 0,
        limit: 10,
    };
    let log = cs.commit_log(&repo, options).expect("commit_log");
    assert!(!log.is_empty());
    assert_eq!(log[0].message, "Initial commit");

    let detail = cs.commit_detail(&repo, &log[0].id).expect("commit_detail");
    assert_eq!(detail.commit.id, log[0].id);
}

#[test]
fn test_staging_and_commit_flow() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let ss = StagingService::new();
    let cs = CommitService::new();

    fs::write(dir.path().join("new_file.txt"), "new content\n").expect("write");

    let status = ss.status(&repo).expect("status");
    assert!(status.iter().any(|f| f.path == "new_file.txt"));

    ss.stage_files(&repo, &["new_file.txt"]).expect("stage");

    let commit = cs.create_commit(&repo, "add new file").expect("create_commit");
    assert!(commit.message.contains("add new file"));
}

#[test]
fn test_diff_service() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let ds = DiffService::new();

    fs::write(dir.path().join("README.md"), "# Modified\n").expect("write");

    let diffs = ds.working_diff(&repo, false).expect("working_diff");
    assert!(!diffs.is_empty());
    assert!(diffs.iter().any(|d| d.path == "README.md"));
}

#[test]
fn test_blame_service() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let bs = BlameService::new();

    let blame = bs.blame(&repo, "README.md").expect("blame");
    assert_eq!(blame.path, "README.md");
    assert!(!blame.lines.is_empty());
    for line in &blame.lines {
        assert!(!line.commit_id.is_empty());
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 7. Tag Operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_tag_create_list_delete() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let tm = TagManager::new();

    let tag = tm.create_lightweight_tag(&repo, "v1.0.0", None).expect("create tag");
    assert_eq!(tag.name, "v1.0.0");
    assert!(!tag.is_annotated);

    let atag = tm
        .create_annotated_tag(&repo, "v2.0.0", None, "Release 2.0")
        .expect("create annotated tag");
    assert_eq!(atag.name, "v2.0.0");
    assert!(atag.is_annotated);

    let tags = tm.list_tags(&repo).expect("list");
    assert!(tags.iter().any(|t| t.name == "v1.0.0"));
    assert!(tags.iter().any(|t| t.name == "v2.0.0"));

    tm.delete_tag(&repo, "v1.0.0").expect("delete");
    let tags_after = tm.list_tags(&repo).expect("list after");
    assert!(!tags_after.iter().any(|t| t.name == "v1.0.0"));
    assert!(tags_after.iter().any(|t| t.name == "v2.0.0"));
}

// ═══════════════════════════════════════════════════════════════════════
// 8. Cherry-pick and Revert
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_cherry_pick_no_conflict() {
    let dir = temp_dir();
    let repo = init_repo_with_commit(dir.path());
    let bm = BranchManager::new();
    let cs = CommitService::new();

    bm.create_branch(&repo, "feature-cp", None).expect("create");
    bm.checkout_branch(&repo, "feature-cp").expect("checkout");
    add_file_and_commit_raw(dir.path(), "cherry.txt", "cherry\n", "cherry commit");

    let repo = GitRepository::open(dir.path()).expect("reopen");
    let head = repo.head().expect("head");
    let cherry_id = head.target.unwrap();

    let branches = bm.list_branches(&repo, BranchFilter::Local).expect("list");
    let default_name = branches
        .iter()
        .find(|b| b.name != "feature-cp")
        .map(|b| b.name.clone())
        .unwrap();
    bm.checkout_branch(&repo, &default_name).expect("checkout default");

    let result = cs.cherry_pick(&repo, &[&cherry_id]).expect("cherry_pick");
    assert!(matches!(result, CherryPickResult::Success { .. }));
    assert!(dir.path().join("cherry.txt").exists());
}

#[test]
fn test_revert_commit() {
    let dir = temp_dir();
    let _repo = init_repo_with_commit(dir.path());
    let cs = CommitService::new();

    add_file_and_commit_raw(dir.path(), "to_revert.txt", "content\n", "commit to revert");
    let repo = GitRepository::open(dir.path()).expect("reopen");
    let head = repo.head().expect("head");
    let commit_id = head.target.unwrap();

    let result = cs.revert(&repo, &[&commit_id]).expect("revert");
    assert!(matches!(result, RevertResult::Success { .. }));
    assert!(!dir.path().join("to_revert.txt").exists());
}

// ═══════════════════════════════════════════════════════════════════════
// 9. Config and Settings
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_app_settings_save_load() {
    let dir = temp_dir();
    let cm = ConfigManager::new();
    let settings_path = dir.path().join("settings.json");

    let settings = AppSettings {
        theme: ThemeMode::Dark,
        language: "en".to_string(),
        font_family: "Fira Code".to_string(),
        font_size: 16,
        hotkeys: std::collections::HashMap::from([
            ("commit".to_string(), "Ctrl+Enter".to_string()),
        ]),
        window: WindowState {
            width: 1920,
            height: 1080,
            x: Some(100),
            y: Some(50),
            maximized: false,
        },
        commit_templates: vec![],
    };

    cm.save_app_settings(&settings_path, &settings).expect("save");
    let loaded = cm.load_app_settings(&settings_path).expect("load");

    assert_eq!(loaded.theme, ThemeMode::Dark);
    assert_eq!(loaded.language, "en");
    assert_eq!(loaded.font_family, "Fira Code");
    assert_eq!(loaded.font_size, 16);
    assert_eq!(loaded.window.width, 1920);
}
