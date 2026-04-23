use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

use chrono::Utc;
use uuid::Uuid;

use crate::error::GitError;
use crate::git_core::{GitRepository, ProgressSender};
use crate::models::{RepoEntry, RepositoryState, TabId};

const MAX_RECENT_REPOS: usize = 20;

pub struct RepositoryManager {
    repos: HashMap<TabId, GitRepository>,
    recent_repos: VecDeque<RepoEntry>,
}

impl RepositoryManager {
    pub fn new() -> Self {
        Self {
            repos: HashMap::new(),
            recent_repos: VecDeque::new(),
        }
    }

    /// Open an existing Git repository, returning a new TabId.
    pub fn open_repo(&mut self, path: PathBuf) -> Result<TabId, GitError> {
        let repo = GitRepository::open(&path)?;
        let tab_id = Self::generate_tab_id();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        self.add_recent(&path, &name);
        self.repos.insert(tab_id.clone(), repo);
        Ok(tab_id)
    }

    /// Clone a remote repository, returning a new TabId.
    pub fn clone_repo(
        &mut self,
        url: String,
        path: PathBuf,
        progress: ProgressSender,
    ) -> Result<TabId, GitError> {
        let repo = GitRepository::clone(&url, &path, progress)?;
        let tab_id = Self::generate_tab_id();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        self.add_recent(&path, &name);
        self.repos.insert(tab_id.clone(), repo);
        Ok(tab_id)
    }

    /// Initialise a new Git repository, returning a new TabId.
    pub fn init_repo(&mut self, path: PathBuf) -> Result<TabId, GitError> {
        let repo = GitRepository::init(&path)?;
        let tab_id = Self::generate_tab_id();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        self.add_recent(&path, &name);
        self.repos.insert(tab_id.clone(), repo);
        Ok(tab_id)
    }

    /// Close a repository tab, removing it from the active repos map.
    pub fn close_repo(&mut self, tab_id: &TabId) {
        self.repos.remove(tab_id);
    }

    /// Get a reference to an open repository by TabId.
    pub fn get_repo(&self, tab_id: &TabId) -> Option<&GitRepository> {
        self.repos.get(tab_id)
    }

    /// Get a mutable reference to an open repository by TabId.
    /// Needed for operations that require `&mut GitRepository` (e.g., rebase, pull with auto-stash).
    pub fn get_repo_mut(&mut self, tab_id: &TabId) -> Option<&mut GitRepository> {
        self.repos.get_mut(tab_id)
    }

    /// Return the recent repos list.
    pub fn recent_repos(&self) -> &VecDeque<RepoEntry> {
        &self.recent_repos
    }

    /// Get the current state of a repository.
    pub fn repo_status(&self, tab_id: &TabId) -> Result<RepositoryState, GitError> {
        let repo = self.repos.get(tab_id).ok_or_else(|| GitError::RepositoryNotFound {
            path: tab_id.0.clone(),
        })?;
        Ok(repo.state())
    }

    // --- private helpers ---

    fn generate_tab_id() -> TabId {
        TabId(Uuid::new_v4().to_string())
    }

    /// Add or update a path in the recent repos list.
    /// If the path already exists, update its last_opened and move to front.
    /// If the list exceeds MAX_RECENT_REPOS, remove the oldest entry.
    fn add_recent(&mut self, path: &PathBuf, name: &str) {
        self.add_recent_entry(&path.display().to_string(), name);
    }

    /// Testable helper: add or update an entry in the recent repos list by string path.
    /// Exposed as pub(crate) so property tests can exercise the bounded-list logic
    /// without requiring real git repositories on disk.
    pub(crate) fn add_recent_entry(&mut self, path: &str, name: &str) {
        // Remove existing entry with the same path (if any)
        self.recent_repos.retain(|e| e.path != path);

        // Push new entry to front
        self.recent_repos.push_front(RepoEntry {
            path: path.to_string(),
            name: name.to_string(),
            last_opened: Utc::now(),
        });

        // Trim to max size
        while self.recent_repos.len() > MAX_RECENT_REPOS {
            self.recent_repos.pop_back();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        TempDir::new().expect("failed to create temp dir")
    }

    fn noop_progress() -> ProgressSender {
        Arc::new(|_recv, _total, _bytes| {})
    }

    #[test]
    fn test_open_repo_valid_git_dir() {
        let dir = temp_dir();
        // Init a git repo first so we can open it
        git2::Repository::init(dir.path()).unwrap();

        let mut mgr = RepositoryManager::new();
        let tab_id = mgr.open_repo(dir.path().to_path_buf());
        assert!(tab_id.is_ok());

        let tab_id = tab_id.unwrap();
        assert!(mgr.get_repo(&tab_id).is_some());
        assert_eq!(mgr.recent_repos().len(), 1);
    }

    #[test]
    fn test_open_repo_non_git_dir_returns_error() {
        let dir = temp_dir();
        // dir is just a plain directory, not a git repo
        let mut mgr = RepositoryManager::new();
        let result = mgr.open_repo(dir.path().to_path_buf());
        assert!(result.is_err());

        match result.unwrap_err() {
            GitError::NotARepository { .. } => {} // expected
            other => panic!("Expected NotARepository, got: {:?}", other),
        }
    }

    #[test]
    fn test_init_repo() {
        let dir = temp_dir();
        let mut mgr = RepositoryManager::new();
        let tab_id = mgr.init_repo(dir.path().to_path_buf()).unwrap();
        assert!(mgr.get_repo(&tab_id).is_some());
        assert_eq!(mgr.recent_repos().len(), 1);
    }

    #[test]
    fn test_close_repo_removes_it() {
        let dir = temp_dir();
        let mut mgr = RepositoryManager::new();
        let tab_id = mgr.init_repo(dir.path().to_path_buf()).unwrap();
        assert!(mgr.get_repo(&tab_id).is_some());

        mgr.close_repo(&tab_id);
        assert!(mgr.get_repo(&tab_id).is_none());
    }

    #[test]
    fn test_repo_status_clean() {
        let dir = temp_dir();
        let mut mgr = RepositoryManager::new();
        let tab_id = mgr.init_repo(dir.path().to_path_buf()).unwrap();
        let status = mgr.repo_status(&tab_id).unwrap();
        assert_eq!(status, RepositoryState::Clean);
    }

    #[test]
    fn test_repo_status_unknown_tab() {
        let mgr = RepositoryManager::new();
        let fake_tab = TabId("nonexistent".to_string());
        let result = mgr.repo_status(&fake_tab);
        assert!(result.is_err());
    }

    #[test]
    fn test_recent_repos_bounded_at_20() {
        let mut mgr = RepositoryManager::new();
        let dirs: Vec<TempDir> = (0..25).map(|_| temp_dir()).collect();

        for dir in &dirs {
            git2::Repository::init(dir.path()).unwrap();
            mgr.open_repo(dir.path().to_path_buf()).unwrap();
        }

        assert_eq!(mgr.recent_repos().len(), MAX_RECENT_REPOS);
    }

    #[test]
    fn test_recent_repos_duplicate_path_updates_time() {
        let dir = temp_dir();
        git2::Repository::init(dir.path()).unwrap();

        let mut mgr = RepositoryManager::new();

        // Open the same repo twice
        let _tab1 = mgr.open_repo(dir.path().to_path_buf()).unwrap();
        let first_time = mgr.recent_repos().front().unwrap().last_opened;

        // Small sleep to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        let _tab2 = mgr.open_repo(dir.path().to_path_buf()).unwrap();
        let second_time = mgr.recent_repos().front().unwrap().last_opened;

        // Should still be only 1 entry (deduplicated)
        assert_eq!(mgr.recent_repos().len(), 1);
        // The time should have been updated
        assert!(second_time >= first_time);
    }

    #[test]
    fn test_recent_repos_most_recent_first() {
        let mut mgr = RepositoryManager::new();
        let dir1 = temp_dir();
        let dir2 = temp_dir();
        git2::Repository::init(dir1.path()).unwrap();
        git2::Repository::init(dir2.path()).unwrap();

        mgr.open_repo(dir1.path().to_path_buf()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        mgr.open_repo(dir2.path().to_path_buf()).unwrap();

        let recent = mgr.recent_repos();
        assert_eq!(recent.len(), 2);
        // Most recently opened should be first
        assert!(recent[0].last_opened >= recent[1].last_opened);
    }

    #[test]
    fn test_clone_repo() {
        // Create a bare repo to clone from
        let origin_dir = temp_dir();
        let origin = git2::Repository::init_bare(origin_dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = origin.index().unwrap().write_tree().unwrap();
        let tree = origin.find_tree(tree_id).unwrap();
        origin
            .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        let clone_dir = temp_dir();
        let mut mgr = RepositoryManager::new();
        let url = origin_dir.path().to_str().unwrap().to_string();
        let tab_id = mgr
            .clone_repo(url, clone_dir.path().to_path_buf(), noop_progress())
            .unwrap();

        assert!(mgr.get_repo(&tab_id).is_some());
        assert_eq!(mgr.recent_repos().len(), 1);
    }

    /// Strategy that generates a Vec of (path, name) string pairs representing
    /// a sequence of open-repo operations. Paths may repeat to exercise dedup.
    fn arb_repo_entries() -> impl Strategy<Value = Vec<(String, String)>> {
        prop::collection::vec(
            (
                // Generate paths from a small pool so duplicates are likely
                prop::sample::select(
                    (0..60u32)
                        .map(|i| format!("/tmp/repo_{}", i))
                        .collect::<Vec<_>>(),
                ),
                "[a-z]{1,8}".prop_map(|s| s),
            ),
            1..50,
        )
    }

    proptest! {
        /// **Validates: Requirements 1.4**
        ///
        /// Feature: rust-git-gui-client, Property 1: 最近仓库列表有界性
        ///
        /// For any sequence of open-repo operations, the recent repos list
        /// SHALL have length ≤ 20 and be sorted by last_opened descending.
        #[test]
        fn prop_recent_repos_bounded_and_sorted(entries in arb_repo_entries()) {
            let mut mgr = RepositoryManager::new();

            for (path, name) in &entries {
                mgr.add_recent_entry(path, name);

                // Invariant must hold after every single operation
                let recent = mgr.recent_repos();
                prop_assert!(
                    recent.len() <= MAX_RECENT_REPOS,
                    "recent_repos length {} exceeds max {}",
                    recent.len(),
                    MAX_RECENT_REPOS
                );
            }

            // Final check: sorted by last_opened descending
            let recent = mgr.recent_repos();
            for window in recent.as_slices().0.windows(2) {
                prop_assert!(
                    window[0].last_opened >= window[1].last_opened,
                    "recent_repos not sorted descending: {:?} < {:?}",
                    window[0].last_opened,
                    window[1].last_opened
                );
            }
            // Also check across the two slices of VecDeque
            let items: Vec<_> = recent.iter().collect();
            for pair in items.windows(2) {
                prop_assert!(
                    pair[0].last_opened >= pair[1].last_opened,
                    "recent_repos not sorted descending: {:?} < {:?}",
                    pair[0].last_opened,
                    pair[1].last_opened
                );
            }
        }
    }
}
