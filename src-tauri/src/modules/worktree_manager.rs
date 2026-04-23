use std::path::Path;

use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::WorktreeInfo;

pub struct WorktreeManager;

impl WorktreeManager {
    pub fn new() -> Self {
        Self
    }

    /// Create a new worktree at the given path, optionally checking out the specified branch.
    /// If `branch` is provided, the worktree will be associated with that branch.
    /// If the branch doesn't exist, it will be created from HEAD.
    pub fn create_worktree(
        &self,
        repo: &GitRepository,
        name: &str,
        path: &Path,
        branch: Option<&str>,
    ) -> Result<WorktreeInfo, GitError> {
        let inner = repo.inner();

        let mut opts = git2::WorktreeAddOptions::new();

        // If a branch is specified, resolve or create it and set it in options.
        let reference;
        if let Some(branch_name) = branch {
            // Try to find the branch; if it doesn't exist, create it from HEAD.
            reference = match inner.find_branch(branch_name, git2::BranchType::Local) {
                Ok(b) => b.into_reference(),
                Err(_) => {
                    let head_commit = inner
                        .head()
                        .and_then(|h| h.peel_to_commit())
                        .map_err(|e| GitError::Git2(e.message().to_string()))?;
                    inner
                        .branch(branch_name, &head_commit, false)
                        .map_err(|e| GitError::Git2(e.message().to_string()))?
                        .into_reference()
                }
            };
            opts.reference(Some(&reference));
        }

        inner
            .worktree(name, path, Some(&opts))
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        Ok(WorktreeInfo {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            branch: branch.map(|b| b.to_string()),
            is_main: false,
        })
    }

    /// List all worktrees for the repository, including the main worktree.
    pub fn list_worktrees(&self, repo: &GitRepository) -> Result<Vec<WorktreeInfo>, GitError> {
        let inner = repo.inner();
        let mut result = Vec::new();

        // Add the main worktree first.
        let main_branch = inner
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));
        let main_path = inner
            .workdir()
            .unwrap_or_else(|| inner.path())
            .to_string_lossy()
            .to_string();
        result.push(WorktreeInfo {
            name: "main".to_string(),
            path: main_path,
            branch: main_branch,
            is_main: true,
        });

        // List linked worktrees.
        let worktree_names = inner
            .worktrees()
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        for i in 0..worktree_names.len() {
            if let Some(name) = worktree_names.get(i) {
                let wt = match inner.find_worktree(name) {
                    Ok(wt) => wt,
                    Err(_) => continue,
                };

                let wt_path = wt.path().to_string_lossy().to_string();

                // Try to determine the branch by opening the worktree's repo.
                let branch = self.worktree_branch(wt.path());

                result.push(WorktreeInfo {
                    name: name.to_string(),
                    path: wt_path,
                    branch,
                    is_main: false,
                });
            }
        }

        Ok(result)
    }

    /// Delete a worktree by name, pruning it from the repository.
    pub fn delete_worktree(&self, repo: &GitRepository, name: &str) -> Result<(), GitError> {
        let inner = repo.inner();

        let wt = inner
            .find_worktree(name)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        let mut prune_opts = git2::WorktreePruneOptions::new();
        prune_opts.working_tree(true);
        prune_opts.valid(true);

        wt.prune(Some(&mut prune_opts))
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        Ok(())
    }

    /// Try to determine the branch checked out in a worktree by opening its repo.
    fn worktree_branch(&self, wt_path: &Path) -> Option<String> {
        let repo = git2::Repository::open(wt_path).ok()?;
        let head = repo.head().ok()?;
        head.shorthand().map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_repo() -> (TempDir, GitRepository) {
        let dir = TempDir::new().expect("failed to create temp dir");
        let repo = GitRepository::init(dir.path()).expect("init failed");

        // Create an initial commit so HEAD is valid.
        {
            let inner = repo.inner();
            let sig = git2::Signature::now("Test", "test@test.com").unwrap();
            let tree_id = inner.index().unwrap().write_tree().unwrap();
            let tree = inner.find_tree(tree_id).unwrap();
            inner
                .commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    #[test]
    fn test_list_worktrees_main_only() {
        let (_dir, repo) = setup_repo();
        let mgr = WorktreeManager::new();

        let worktrees = mgr.list_worktrees(&repo).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].is_main);
        assert_eq!(worktrees[0].name, "main");
        assert!(worktrees[0].branch.is_some());
    }

    #[test]
    fn test_create_and_list_worktree() {
        let (dir, repo) = setup_repo();
        let mgr = WorktreeManager::new();

        let wt_path = dir.path().join("wt-feature");
        let info = mgr
            .create_worktree(&repo, "feature", &wt_path, Some("feature-branch"))
            .unwrap();

        assert_eq!(info.name, "feature");
        assert!(!info.is_main);
        assert_eq!(info.branch, Some("feature-branch".to_string()));

        let worktrees = mgr.list_worktrees(&repo).unwrap();
        assert_eq!(worktrees.len(), 2);

        let linked = worktrees.iter().find(|w| !w.is_main).unwrap();
        assert_eq!(linked.name, "feature");
        assert_eq!(linked.branch, Some("feature-branch".to_string()));
    }

    #[test]
    fn test_create_worktree_existing_branch() {
        let (dir, repo) = setup_repo();
        let mgr = WorktreeManager::new();

        // Create a branch first.
        let inner = repo.inner();
        let head = inner.head().unwrap().peel_to_commit().unwrap();
        inner.branch("existing-branch", &head, false).unwrap();

        let wt_path = dir.path().join("wt-existing");
        let info = mgr
            .create_worktree(&repo, "existing", &wt_path, Some("existing-branch"))
            .unwrap();

        assert_eq!(info.branch, Some("existing-branch".to_string()));
    }

    #[test]
    fn test_create_worktree_no_branch() {
        let (dir, repo) = setup_repo();
        let mgr = WorktreeManager::new();

        let wt_path = dir.path().join("wt-detached");
        let info = mgr
            .create_worktree(&repo, "detached", &wt_path, None)
            .unwrap();

        assert_eq!(info.name, "detached");
        assert!(info.branch.is_none());
        assert!(!info.is_main);
    }

    #[test]
    fn test_delete_worktree() {
        let (dir, repo) = setup_repo();
        let mgr = WorktreeManager::new();

        let wt_path = dir.path().join("wt-to-delete");
        mgr.create_worktree(&repo, "to-delete", &wt_path, Some("delete-branch"))
            .unwrap();

        // Verify it exists.
        let worktrees = mgr.list_worktrees(&repo).unwrap();
        assert_eq!(worktrees.len(), 2);

        // Remove the worktree directory first (simulating cleanup).
        std::fs::remove_dir_all(&wt_path).ok();

        // Delete the worktree.
        mgr.delete_worktree(&repo, "to-delete").unwrap();

        let worktrees = mgr.list_worktrees(&repo).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].is_main);
    }

    #[test]
    fn test_delete_nonexistent_worktree() {
        let (_dir, repo) = setup_repo();
        let mgr = WorktreeManager::new();

        let result = mgr.delete_worktree(&repo, "nonexistent");
        assert!(result.is_err());
    }
}
