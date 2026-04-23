use std::path::Path;

use crate::error::GitError;
use crate::git_core::{GitRepository, ProgressSender};
use crate::models::{SubmoduleInfo, SubmoduleStatus};

pub struct SubmoduleManager;

impl SubmoduleManager {
    pub fn new() -> Self {
        Self
    }

    /// List all submodules in the repository with their current status.
    pub fn list_submodules(&self, repo: &GitRepository) -> Result<Vec<SubmoduleInfo>, GitError> {
        let inner = repo.inner();
        let submodules = inner
            .submodules()
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        let mut result = Vec::new();
        for sm in &submodules {
            let name = sm.name().unwrap_or("").to_string();
            let path = sm.path().to_string_lossy().to_string();
            let url = sm.url().unwrap_or("").to_string();
            let branch = sm.branch().map(|b| b.to_string());

            let head_id = sm.head_id().map(|oid| oid.to_string());

            let status = self.determine_status(sm);

            result.push(SubmoduleInfo {
                name,
                path,
                url,
                head_id,
                status,
                branch,
            });
        }

        Ok(result)
    }

    /// Initialize a submodule at the given path.
    pub fn init_submodule(
        &self,
        repo: &GitRepository,
        path: &str,
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        let mut sm = inner
            .find_submodule(path)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;
        sm.init(false)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;
        Ok(())
    }

    /// Update a submodule at the given path.
    /// If `recursive` is true, nested submodules are also updated.
    /// Progress is reported via the `progress` callback.
    pub fn update_submodule(
        &self,
        repo: &GitRepository,
        path: &str,
        recursive: bool,
        progress: ProgressSender,
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        let mut sm = inner
            .find_submodule(path)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        // Ensure the submodule is initialized before updating
        sm.init(false).ok();

        let progress_clone = progress.clone();
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.transfer_progress(move |stats| {
            progress_clone(
                stats.received_objects(),
                stats.total_objects(),
                stats.received_bytes(),
            );
            true
        });

        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let mut update_opts = git2::SubmoduleUpdateOptions::new();
        update_opts.fetch(fetch_opts);

        sm.update(true, Some(&mut update_opts))
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        // Handle recursive update of nested submodules
        if recursive {
            self.update_nested_submodules(repo, path, &progress)?;
        }

        Ok(())
    }

    /// Deinitialize a submodule at the given path.
    pub fn deinit_submodule(
        &self,
        repo: &GitRepository,
        path: &str,
    ) -> Result<(), GitError> {
        let inner = repo.inner();

        // Remove the submodule's config entries (equivalent to git submodule deinit)
        let mut config = inner
            .config()
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        let key = format!("submodule.{}.url", path);
        // Remove the url entry from local config; ignore error if not present
        let _ = config.remove(&key);

        // Clean the submodule working directory
        if let Some(workdir) = repo.workdir() {
            let sm_path = workdir.join(path);
            if sm_path.exists() && sm_path.is_dir() {
                // Remove contents but keep the directory
                if let Ok(entries) = std::fs::read_dir(&sm_path) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            let _ = std::fs::remove_dir_all(&p);
                        } else {
                            let _ = std::fs::remove_file(&p);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Set the remote URL for a submodule by modifying .gitmodules.
    pub fn set_submodule_url(
        &self,
        repo: &GitRepository,
        path: &str,
        url: &str,
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        // Verify the submodule exists
        inner
            .find_submodule(path)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        // Modify .gitmodules directly
        sm_set_config_in_gitmodules(inner, path, "url", url)?;

        // Sync the URL to the local config so git operations use the new URL
        let mut sm = inner
            .find_submodule(path)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;
        sm.sync()
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        Ok(())
    }

    /// Set the tracking branch for a submodule by modifying .gitmodules.
    pub fn set_submodule_branch(
        &self,
        repo: &GitRepository,
        path: &str,
        branch: &str,
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        // Verify the submodule exists
        inner
            .find_submodule(path)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        sm_set_config_in_gitmodules(inner, path, "branch", branch)?;
        Ok(())
    }

    // --- Private helpers ---

    /// Determine the status of a submodule.
    fn determine_status(&self, sm: &git2::Submodule) -> SubmoduleStatus {
        // Check workdir_id to see if the submodule is initialized
        let workdir_id = sm.workdir_id();
        let head_id = sm.head_id();

        match (head_id, workdir_id) {
            (_, None) => SubmoduleStatus::Uninitialized,
            (Some(head), Some(wd)) => {
                if head != wd {
                    SubmoduleStatus::Modified
                } else {
                    // Check if the submodule HEAD is detached
                    if let Ok(sub_repo) = sm.open() {
                        if sub_repo.head_detached().unwrap_or(false) {
                            SubmoduleStatus::DetachedHead
                        } else {
                            SubmoduleStatus::Initialized
                        }
                    } else {
                        SubmoduleStatus::Initialized
                    }
                }
            }
            (None, Some(_)) => SubmoduleStatus::Initialized,
        }
    }

    /// Recursively update nested submodules within a submodule.
    fn update_nested_submodules(
        &self,
        repo: &GitRepository,
        sm_path: &str,
        progress: &ProgressSender,
    ) -> Result<(), GitError> {
        let workdir = repo
            .workdir()
            .ok_or_else(|| GitError::Git2("bare repository has no workdir".to_string()))?;

        let sub_repo_path = workdir.join(sm_path);
        if !sub_repo_path.join(".git").exists() && !sub_repo_path.join(".git").is_file() {
            // Submodule not checked out, nothing to recurse into
            return Ok(());
        }

        let sub_repo = git2::Repository::open(&sub_repo_path)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        let nested = sub_repo
            .submodules()
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        for mut nested_sm in nested {
            nested_sm.init(false).ok();

            let progress_clone = progress.clone();
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.transfer_progress(move |stats| {
                progress_clone(
                    stats.received_objects(),
                    stats.total_objects(),
                    stats.received_bytes(),
                );
                true
            });

            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.remote_callbacks(callbacks);

            let mut update_opts = git2::SubmoduleUpdateOptions::new();
            update_opts.fetch(fetch_opts);

            nested_sm
                .update(true, Some(&mut update_opts))
                .map_err(|e| GitError::Git2(e.message().to_string()))?;
        }

        Ok(())
    }
}

/// Set a config value for a submodule in .gitmodules.
fn sm_set_config_in_gitmodules(
    repo: &git2::Repository,
    path: &str,
    key_name: &str,
    value: &str,
) -> Result<(), GitError> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Git2("bare repository has no workdir".to_string()))?;

    let gitmodules_path = workdir.join(".gitmodules");
    if !gitmodules_path.exists() {
        return Err(GitError::InvalidArgument(
            ".gitmodules file not found".to_string(),
        ));
    }

    let config = git2::Config::open(Path::new(&gitmodules_path))
        .map_err(|e| GitError::Git2(e.message().to_string()))?;

    let mut config = config
        .open_level(git2::ConfigLevel::Local)
        .unwrap_or(config);

    let key = format!("submodule.{}.{}", path, key_name);
    config
        .set_str(&key, value)
        .map_err(|e| GitError::Git2(e.message().to_string()))?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a temp dir with an initialized git repo and an initial commit.
    fn setup_repo() -> (TempDir, GitRepository) {
        let dir = TempDir::new().unwrap();
        let mut repo = GitRepository::init(dir.path()).unwrap();

        {
            let inner = repo.inner_mut();
            let mut config = inner.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();

            let workdir = inner.workdir().unwrap().to_path_buf();
            fs::write(workdir.join("README.md"), "# Test\n").unwrap();

            let mut index = inner.index().unwrap();
            index.add_path(Path::new("README.md")).unwrap();
            index.write().unwrap();

            let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = inner.find_tree(tree_id).unwrap();
            inner
                .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    /// Helper: create a bare repo to use as a submodule remote.
    fn setup_bare_remote() -> TempDir {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();

        // Create a file so the tree is non-empty
        fs::write(dir.path().join("README.md"), "# Sub\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        dir
    }

    /// Helper: add a submodule to a repo by manually writing .gitmodules
    /// and committing the submodule reference.
    fn add_submodule(repo: &GitRepository, remote_url: &str, sm_path: &str) {
        let inner = repo.inner();
        let workdir = inner.workdir().unwrap().to_path_buf();

        // Clone the remote into the submodule path first
        let sub_full_path = workdir.join(sm_path);
        git2::Repository::clone(remote_url, &sub_full_path).unwrap();

        // Get the HEAD commit of the cloned submodule
        let sub_head_oid = {
            let sub_repo = git2::Repository::open(&sub_full_path).unwrap();
            let sub_head = sub_repo.head().unwrap().peel_to_commit().unwrap();
            sub_head.id()
        };

        // Normalize URL for .gitmodules (use forward slashes on Windows)
        let normalized_url = remote_url.replace('\\', "/");

        // Write .gitmodules
        let gitmodules_content = format!(
            "[submodule \"{}\"]\n\tpath = {}\n\turl = {}\n",
            sm_path, sm_path, normalized_url
        );
        let gitmodules_path = workdir.join(".gitmodules");
        if gitmodules_path.exists() {
            let existing = fs::read_to_string(&gitmodules_path).unwrap();
            fs::write(&gitmodules_path, format!("{}{}", existing, gitmodules_content)).unwrap();
        } else {
            fs::write(&gitmodules_path, gitmodules_content).unwrap();
        }

        // Add .gitmodules and the submodule entry to the index
        let mut index = inner.index().unwrap();
        index.add_path(Path::new(".gitmodules")).unwrap();

        // Add the submodule as a gitlink entry (mode 0xe0 = 160000)
        index
            .add(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: 0o160000, // gitlink
                uid: 0,
                gid: 0,
                file_size: 0,
                id: sub_head_oid,
                flags: (sm_path.len() as u16) & 0xFFF,
                flags_extended: 0,
                path: sm_path.as_bytes().to_vec(),
            })
            .unwrap();
        index.write().unwrap();

        // Commit
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = inner.find_tree(tree_id).unwrap();
        let head = inner.head().unwrap().peel_to_commit().unwrap();
        inner
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Add submodule",
                &tree,
                &[&head],
            )
            .unwrap();
    }

    #[test]
    fn test_list_submodules_empty() {
        let (_dir, repo) = setup_repo();
        let manager = SubmoduleManager::new();
        let subs = manager.list_submodules(&repo).unwrap();
        assert!(subs.is_empty());
    }

    #[test]
    fn test_list_submodules_with_submodule() {
        let (_dir, repo) = setup_repo();
        let remote_dir = setup_bare_remote();
        let remote_url = remote_dir.path().to_str().unwrap();

        add_submodule(&repo, remote_url, "libs/sub");

        let manager = SubmoduleManager::new();
        let subs = manager.list_submodules(&repo).unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].path, "libs/sub");
        // URL may have forward slashes (normalized from Windows backslashes)
        assert!(!subs[0].url.is_empty());
    }

    #[test]
    fn test_init_submodule() {
        let (_dir, repo) = setup_repo();
        let remote_dir = setup_bare_remote();
        let remote_url = remote_dir.path().to_str().unwrap();

        add_submodule(&repo, remote_url, "libs/sub");

        let manager = SubmoduleManager::new();
        // init_submodule should succeed (may already be initialized from add)
        let result = manager.init_submodule(&repo, "libs/sub");
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_submodule_not_found() {
        let (_dir, repo) = setup_repo();
        let manager = SubmoduleManager::new();
        let result = manager.init_submodule(&repo, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_deinit_submodule() {
        let (dir, repo) = setup_repo();
        let remote_dir = setup_bare_remote();
        let remote_url = remote_dir.path().to_str().unwrap();

        add_submodule(&repo, remote_url, "libs/sub");

        // Ensure the submodule dir has content
        let sm_dir = dir.path().join("libs/sub");
        assert!(sm_dir.exists());

        let manager = SubmoduleManager::new();
        let result = manager.deinit_submodule(&repo, "libs/sub");
        assert!(result.is_ok());

        // The submodule directory contents should be cleaned
        if sm_dir.exists() {
            let entries: Vec<_> = fs::read_dir(&sm_dir).unwrap().collect();
            assert!(entries.is_empty(), "submodule dir should be empty after deinit");
        }
    }

    #[test]
    fn test_set_submodule_url() {
        let (_dir, repo) = setup_repo();
        let remote_dir = setup_bare_remote();
        let remote_url = remote_dir.path().to_str().unwrap();

        add_submodule(&repo, remote_url, "libs/sub");

        let manager = SubmoduleManager::new();
        let result = manager.set_submodule_url(&repo, "libs/sub", "https://example.com/new.git");
        assert!(result.is_ok());

        // Verify the URL was updated in .gitmodules
        let workdir = repo.workdir().unwrap();
        let gitmodules = fs::read_to_string(workdir.join(".gitmodules")).unwrap();
        assert!(gitmodules.contains("https://example.com/new.git"));
    }

    #[test]
    fn test_set_submodule_branch() {
        let (_dir, repo) = setup_repo();
        let remote_dir = setup_bare_remote();
        let remote_url = remote_dir.path().to_str().unwrap();

        add_submodule(&repo, remote_url, "libs/sub");

        let manager = SubmoduleManager::new();
        let result = manager.set_submodule_branch(&repo, "libs/sub", "develop");
        assert!(result.is_ok());

        // Verify by reading .gitmodules
        let workdir = repo.workdir().unwrap();
        let gitmodules = fs::read_to_string(workdir.join(".gitmodules")).unwrap();
        assert!(gitmodules.contains("develop"));
    }

    #[test]
    fn test_set_submodule_branch_no_gitmodules() {
        let (_dir, repo) = setup_repo();
        let manager = SubmoduleManager::new();
        // No .gitmodules file exists
        let result = manager.set_submodule_branch(&repo, "nonexistent", "main");
        assert!(result.is_err());
    }

    #[test]
    fn test_determine_status_uninitialized() {
        let (_dir, repo) = setup_repo();
        let remote_dir = setup_bare_remote();
        let remote_url = remote_dir.path().to_str().unwrap();

        add_submodule(&repo, remote_url, "libs/sub");

        let manager = SubmoduleManager::new();
        let subs = manager.list_submodules(&repo).unwrap();
        assert_eq!(subs.len(), 1);
        // After add_submodule (which clones), the submodule should be initialized
        assert_ne!(subs[0].status, SubmoduleStatus::Uninitialized);
    }
}
