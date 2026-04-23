use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::{BranchFilter, BranchInfo, BranchType, MergeResult, ResetMode};

pub struct BranchManager;

impl BranchManager {
    pub fn new() -> Self {
        Self
    }

    /// List branches filtered by BranchFilter.
    /// Computes ahead/behind counts relative to upstream, detects HEAD branch.
    pub fn list_branches(
        &self,
        repo: &GitRepository,
        filter: BranchFilter,
    ) -> Result<Vec<BranchInfo>, GitError> {
        let inner = repo.inner();
        let mut results = Vec::new();

        let head_ref = inner
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let branch_type_filter = match filter {
            BranchFilter::All => None,
            BranchFilter::Local => Some(git2::BranchType::Local),
            BranchFilter::Remote => Some(git2::BranchType::Remote),
        };

        let branches = inner.branches(branch_type_filter)?;

        for branch_result in branches {
            let (branch, bt) = branch_result?;
            let name = branch
                .name()?
                .unwrap_or("")
                .to_string();

            if name.is_empty() {
                continue;
            }

            let is_head = bt == git2::BranchType::Local
                && head_ref.as_deref() == Some(&name);

            let commit_id = branch
                .get()
                .target()
                .map(|oid| oid.to_string())
                .unwrap_or_default();

            let (upstream_name, ahead, behind) = self.compute_upstream_info(inner, &branch);

            let branch_type = match bt {
                git2::BranchType::Local => BranchType::Local,
                git2::BranchType::Remote => {
                    let remote_name = if let Some(pos) = name.find('/') {
                        name[..pos].to_string()
                    } else {
                        String::new()
                    };
                    BranchType::Remote { remote_name }
                }
            };

            results.push(BranchInfo {
                name,
                is_head,
                upstream: upstream_name,
                ahead,
                behind,
                last_commit_id: commit_id,
                branch_type,
            });
        }

        Ok(results)
    }

    /// Create a new branch from HEAD or a specified target commit.
    pub fn create_branch(
        &self,
        repo: &GitRepository,
        name: &str,
        target: Option<&str>,
    ) -> Result<BranchInfo, GitError> {
        let inner = repo.inner();

        let commit = match target {
            Some(rev) => {
                let obj = inner.revparse_single(rev).map_err(|_| {
                    GitError::InvalidArgument(format!("Invalid revision: {}", rev))
                })?;
                obj.peel_to_commit().map_err(|_| {
                    GitError::InvalidArgument(format!("Cannot resolve to commit: {}", rev))
                })?
            }
            None => {
                let head = inner.head()?;
                head.peel_to_commit().map_err(|_| {
                    GitError::InvalidArgument("HEAD does not point to a commit".to_string())
                })?
            }
        };

        let branch = inner.branch(name, &commit, false)?;

        let head_ref = inner
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let branch_name = branch
            .name()?
            .unwrap_or("")
            .to_string();

        let is_head = head_ref.as_deref() == Some(&branch_name);

        Ok(BranchInfo {
            name: branch_name,
            is_head,
            upstream: None,
            ahead: 0,
            behind: 0,
            last_commit_id: commit.id().to_string(),
            branch_type: BranchType::Local,
        })
    }

    /// Delete a local branch. If force is false, refuses to delete unmerged branches.
    pub fn delete_branch(
        &self,
        repo: &GitRepository,
        name: &str,
        force: bool,
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        let mut branch = inner
            .find_branch(name, git2::BranchType::Local)
            .map_err(|_| {
                GitError::InvalidArgument(format!("Branch not found: {}", name))
            })?;

        if branch.is_head() {
            return Err(GitError::InvalidArgument(
                "Cannot delete the currently checked out branch".to_string(),
            ));
        }

        if !force {
            let is_merged = branch.is_head()
                || inner
                    .head()
                    .ok()
                    .and_then(|h| h.target())
                    .and_then(|head_oid| {
                        branch.get().target().map(|branch_oid| {
                            inner
                                .graph_descendant_of(head_oid, branch_oid)
                                .unwrap_or(false)
                                || head_oid == branch_oid
                        })
                    })
                    .unwrap_or(false);

            if !is_merged {
                // Check if branch is merged into HEAD using merge_base
                let head_oid = inner.head().ok().and_then(|h| h.target());
                let branch_oid = branch.get().target();
                if let (Some(h), Some(b)) = (head_oid, branch_oid) {
                    if let Ok(merge_base) = inner.merge_base(h, b) {
                        if merge_base != b {
                            return Err(GitError::InvalidArgument(format!(
                                "Branch '{}' is not fully merged. Use force to delete.",
                                name
                            )));
                        }
                    } else {
                        return Err(GitError::InvalidArgument(format!(
                            "Branch '{}' is not fully merged. Use force to delete.",
                            name
                        )));
                    }
                }
            }
        }

        branch.delete()?;
        Ok(())
    }

    /// Rename a local branch.
    pub fn rename_branch(
        &self,
        repo: &GitRepository,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        let mut branch = inner
            .find_branch(old_name, git2::BranchType::Local)
            .map_err(|_| {
                GitError::InvalidArgument(format!("Branch not found: {}", old_name))
            })?;

        branch.rename(new_name, false)?;
        Ok(())
    }

    /// Checkout a branch by name. Sets HEAD and updates the working directory.
    pub fn checkout_branch(
        &self,
        repo: &GitRepository,
        name: &str,
    ) -> Result<(), GitError> {
        let inner = repo.inner();

        // Verify branch exists
        let _branch = inner
            .find_branch(name, git2::BranchType::Local)
            .map_err(|_| {
                GitError::InvalidArgument(format!("Branch not found: {}", name))
            })?;

        let refname = format!("refs/heads/{}", name);
        inner.set_head(&refname)?;
        inner.checkout_head(Some(
            git2::build::CheckoutBuilder::new().force(),
        ))?;

        Ok(())
    }

    /// Set the upstream tracking branch for a local branch.
    pub fn set_upstream(
        &self,
        repo: &GitRepository,
        local: &str,
        remote: &str,
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        let mut branch = inner
            .find_branch(local, git2::BranchType::Local)
            .map_err(|_| {
                GitError::InvalidArgument(format!("Branch not found: {}", local))
            })?;

        branch.set_upstream(Some(remote))?;
        Ok(())
    }

    /// Merge source branch into current HEAD.
    /// Returns MergeResult indicating fast-forward, normal merge, or conflict.
    pub fn merge(
        &self,
        repo: &GitRepository,
        source: &str,
    ) -> Result<MergeResult, GitError> {
        let inner = repo.inner();

        // Find the source branch reference
        let source_branch = inner
            .find_branch(source, git2::BranchType::Local)
            .or_else(|_| inner.find_branch(source, git2::BranchType::Remote))
            .map_err(|_| {
                GitError::InvalidArgument(format!("Branch not found: {}", source))
            })?;

        let source_oid = source_branch
            .get()
            .target()
            .ok_or_else(|| {
                GitError::InvalidArgument(format!("Branch '{}' has no target", source))
            })?;

        let source_annotated = inner.find_annotated_commit(source_oid)?;

        // Perform merge analysis
        let (analysis, _preference) = inner.merge_analysis(&[&source_annotated])?;

        if analysis.is_up_to_date() {
            return Ok(MergeResult::FastForward);
        }

        if analysis.is_fast_forward() {
            // Fast-forward: move HEAD to source commit
            let source_commit = inner.find_commit(source_oid)?;
            let refname = inner
                .head()?
                .name()
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    GitError::InvalidArgument("HEAD is detached".to_string())
                })?;

            inner.reference(
                &refname,
                source_oid,
                true,
                &format!("Fast-forward to {}", source),
            )?;
            inner.checkout_tree(
                source_commit.as_object(),
                Some(git2::build::CheckoutBuilder::new().force()),
            )?;
            inner.set_head(&refname)?;

            return Ok(MergeResult::FastForward);
        }

        // Normal merge
        inner.merge(&[&source_annotated], None, None)?;

        // Check for conflicts
        let index = inner.index()?;
        if index.has_conflicts() {
            let conflict_files: Vec<String> = index
                .conflicts()?
                .filter_map(|c| c.ok())
                .filter_map(|conflict| {
                    conflict
                        .our
                        .as_ref()
                        .or(conflict.their.as_ref())
                        .and_then(|entry| {
                            String::from_utf8(entry.path.clone()).ok()
                        })
                })
                .collect();

            return Ok(MergeResult::Conflict {
                files: conflict_files,
            });
        }

        // No conflicts — create merge commit
        let sig = inner.signature().map_err(|e| {
            GitError::InvalidArgument(format!("No signature configured: {}", e))
        })?;

        let mut index = inner.index()?;
        let tree_oid = index.write_tree()?;
        let tree = inner.find_tree(tree_oid)?;

        let head_commit = inner.head()?.peel_to_commit()?;
        let source_commit = inner.find_commit(source_oid)?;

        inner.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("Merge branch '{}'", source),
            &tree,
            &[&head_commit, &source_commit],
        )?;

        // Clean up merge state
        inner.cleanup_state()?;

        Ok(MergeResult::Merged)
    }

    /// Reset the current branch to a target commit.
    /// - Soft: moves HEAD only, keeps staging area and working directory
    /// - Mixed: moves HEAD and resets staging area, keeps working directory
    /// - Hard: moves HEAD, resets staging area and working directory
    pub fn reset(
        &self,
        repo: &GitRepository,
        target: &str,
        mode: ResetMode,
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        let obj = inner.revparse_single(target)
            .map_err(|_| GitError::InvalidArgument(format!("Invalid revision: {}", target)))?;
        let commit = obj.peel_to_commit()
            .map_err(|_| GitError::InvalidArgument(format!("Cannot resolve to commit: {}", target)))?;

        let reset_type = match mode {
            ResetMode::Soft => git2::ResetType::Soft,
            ResetMode::Mixed => git2::ResetType::Mixed,
            ResetMode::Hard => git2::ResetType::Hard,
        };

        inner.reset(commit.as_object(), reset_type, None)?;
        Ok(())
    }

    // --- Private helpers ---

    fn compute_upstream_info(
        &self,
        inner: &git2::Repository,
        branch: &git2::Branch,
    ) -> (Option<String>, usize, usize) {
        let upstream = match branch.upstream() {
            Ok(u) => u,
            Err(_) => return (None, 0, 0),
        };

        let upstream_name = upstream
            .name()
            .ok()
            .flatten()
            .map(|s| s.to_string());

        let local_oid = match branch.get().target() {
            Some(oid) => oid,
            None => return (upstream_name, 0, 0),
        };

        let upstream_oid = match upstream.get().target() {
            Some(oid) => oid,
            None => return (upstream_name, 0, 0),
        };

        let (ahead, behind) = inner
            .graph_ahead_behind(local_oid, upstream_oid)
            .unwrap_or((0, 0));

        (upstream_name, ahead, behind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    /// Helper: create a temp dir with an initialized git repo and an initial commit.
    fn setup_repo() -> (TempDir, GitRepository) {
        let dir = TempDir::new().unwrap();
        let repo = GitRepository::init(dir.path()).unwrap();

        {
            let inner = repo.inner();
            let mut config = inner.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();

            let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
            let tree_id = inner.index().unwrap().write_tree().unwrap();
            let tree = inner.find_tree(tree_id).unwrap();
            inner
                .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    /// Helper: create a file, stage it, and commit.
    fn add_file_and_commit(repo: &GitRepository, filename: &str, content: &str, message: &str) {
        let inner = repo.inner();
        let workdir = inner.workdir().unwrap();
        fs::write(workdir.join(filename), content).unwrap();

        let mut index = inner.index().unwrap();
        index.add_path(Path::new(filename)).unwrap();
        index.write().unwrap();

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = inner.find_tree(tree_id).unwrap();
        let head = inner.head().unwrap().target().unwrap();
        let parent = inner.find_commit(head).unwrap();
        inner
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
            .unwrap();
    }

    #[test]
    fn test_list_branches_local() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        let branches = mgr.list_branches(&repo, BranchFilter::Local).unwrap();
        assert_eq!(branches.len(), 1);
        assert!(branches[0].is_head);
        assert_eq!(branches[0].branch_type, BranchType::Local);
    }

    #[test]
    fn test_list_branches_all_after_create() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        mgr.create_branch(&repo, "feature-a", None).unwrap();
        mgr.create_branch(&repo, "feature-b", None).unwrap();

        let branches = mgr.list_branches(&repo, BranchFilter::All).unwrap();
        let local_count = branches
            .iter()
            .filter(|b| b.branch_type == BranchType::Local)
            .count();
        // default branch + feature-a + feature-b
        assert_eq!(local_count, 3);
    }

    #[test]
    fn test_create_branch_from_head() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        let info = mgr.create_branch(&repo, "new-branch", None).unwrap();
        assert_eq!(info.name, "new-branch");
        assert_eq!(info.branch_type, BranchType::Local);
        assert!(!info.last_commit_id.is_empty());
    }

    #[test]
    fn test_create_branch_from_target() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "a.txt", "a", "Second commit");

        let mgr = BranchManager::new();
        // Create branch from HEAD~1 (the initial commit)
        let head_oid = repo.inner().head().unwrap().target().unwrap();
        let head_commit = repo.inner().find_commit(head_oid).unwrap();
        let parent_id = head_commit.parent_id(0).unwrap().to_string();

        let info = mgr
            .create_branch(&repo, "from-parent", Some(&parent_id))
            .unwrap();
        assert_eq!(info.name, "from-parent");
        assert_eq!(info.last_commit_id, parent_id);
    }

    #[test]
    fn test_delete_branch() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        mgr.create_branch(&repo, "to-delete", None).unwrap();
        mgr.delete_branch(&repo, "to-delete", false).unwrap();

        let branches = mgr.list_branches(&repo, BranchFilter::Local).unwrap();
        assert!(!branches.iter().any(|b| b.name == "to-delete"));
    }

    #[test]
    fn test_delete_current_branch_fails() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        let head_name = repo
            .inner()
            .head()
            .unwrap()
            .shorthand()
            .unwrap()
            .to_string();

        let result = mgr.delete_branch(&repo, &head_name, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_unmerged_branch_without_force_fails() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        // Create a branch, switch to it, add a commit, switch back
        mgr.create_branch(&repo, "unmerged", None).unwrap();
        mgr.checkout_branch(&repo, "unmerged").unwrap();
        add_file_and_commit(&repo, "unmerged.txt", "data", "Unmerged commit");

        // Switch back to default branch
        // HEAD is now on "unmerged", we need to go back to the original
        // Find the original branch name
        let branches = mgr.list_branches(&repo, BranchFilter::Local).unwrap();
        let other = branches
            .iter()
            .find(|b| b.name != "unmerged")
            .unwrap();
        mgr.checkout_branch(&repo, &other.name).unwrap();

        // Try to delete without force — should fail because it has commits not in HEAD
        let result = mgr.delete_branch(&repo, "unmerged", false);
        assert!(result.is_err());

        // Force delete should succeed
        mgr.delete_branch(&repo, "unmerged", true).unwrap();
    }

    #[test]
    fn test_rename_branch() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        mgr.create_branch(&repo, "old-name", None).unwrap();
        mgr.rename_branch(&repo, "old-name", "new-name").unwrap();

        let branches = mgr.list_branches(&repo, BranchFilter::Local).unwrap();
        assert!(branches.iter().any(|b| b.name == "new-name"));
        assert!(!branches.iter().any(|b| b.name == "old-name"));
    }

    #[test]
    fn test_checkout_branch() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        mgr.create_branch(&repo, "feature", None).unwrap();
        mgr.checkout_branch(&repo, "feature").unwrap();

        let head = repo.inner().head().unwrap();
        assert_eq!(head.shorthand().unwrap(), "feature");
    }

    #[test]
    fn test_checkout_nonexistent_branch_fails() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        let result = mgr.checkout_branch(&repo, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_fast_forward() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        // Create feature branch, add commit, switch back, merge
        mgr.create_branch(&repo, "feature-ff", None).unwrap();
        mgr.checkout_branch(&repo, "feature-ff").unwrap();
        add_file_and_commit(&repo, "ff.txt", "ff", "FF commit");

        // Get the default branch name
        let branches = mgr.list_branches(&repo, BranchFilter::Local).unwrap();
        let default_name = branches
            .iter()
            .find(|b| b.name != "feature-ff")
            .unwrap()
            .name
            .clone();

        mgr.checkout_branch(&repo, &default_name).unwrap();

        let result = mgr.merge(&repo, "feature-ff").unwrap();
        assert_eq!(result, MergeResult::FastForward);
    }

    #[test]
    fn test_merge_normal() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        let branches = mgr.list_branches(&repo, BranchFilter::Local).unwrap();
        let default_name = branches[0].name.clone();

        // Create feature branch
        mgr.create_branch(&repo, "feature-merge", None).unwrap();

        // Add commit on default branch
        add_file_and_commit(&repo, "main.txt", "main content", "Main commit");

        // Switch to feature, add commit
        mgr.checkout_branch(&repo, "feature-merge").unwrap();
        add_file_and_commit(&repo, "feature.txt", "feature content", "Feature commit");

        // Switch back and merge
        mgr.checkout_branch(&repo, &default_name).unwrap();
        let result = mgr.merge(&repo, "feature-merge").unwrap();
        assert_eq!(result, MergeResult::Merged);
    }

    #[test]
    fn test_merge_conflict() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        let branches = mgr.list_branches(&repo, BranchFilter::Local).unwrap();
        let default_name = branches[0].name.clone();

        // Create feature branch
        mgr.create_branch(&repo, "feature-conflict", None).unwrap();

        // Add conflicting commit on default branch
        add_file_and_commit(&repo, "conflict.txt", "main version", "Main conflict");

        // Switch to feature, add conflicting commit on same file
        mgr.checkout_branch(&repo, "feature-conflict").unwrap();
        add_file_and_commit(&repo, "conflict.txt", "feature version", "Feature conflict");

        // Switch back and merge
        mgr.checkout_branch(&repo, &default_name).unwrap();
        let result = mgr.merge(&repo, "feature-conflict").unwrap();

        match result {
            MergeResult::Conflict { files } => {
                assert!(!files.is_empty());
                assert!(files.iter().any(|f| f.contains("conflict.txt")));
            }
            other => panic!("Expected Conflict, got {:?}", other),
        }

        // Clean up merge state for the temp dir cleanup
        repo.inner().cleanup_state().unwrap();
    }

    #[test]
    fn test_list_branches_head_detection() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        mgr.create_branch(&repo, "other", None).unwrap();

        let branches = mgr.list_branches(&repo, BranchFilter::Local).unwrap();
        let head_branches: Vec<_> = branches.iter().filter(|b| b.is_head).collect();
        assert_eq!(head_branches.len(), 1);
    }

    #[test]
    fn test_create_branch_invalid_target() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        let result = mgr.create_branch(&repo, "bad", Some("nonexistent_ref"));
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_nonexistent_branch_fails() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        let result = mgr.rename_branch(&repo, "no-such-branch", "new-name");
        assert!(result.is_err());
    }

    #[test]
    fn test_soft_reset_preserves_staging_and_workdir() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        // Create two commits
        add_file_and_commit(&repo, "a.txt", "content-a", "Second commit");
        add_file_and_commit(&repo, "b.txt", "content-b", "Third commit");

        // Record the first commit (parent of second) as reset target
        let head_oid = repo.inner().head().unwrap().target().unwrap();
        let head_commit = repo.inner().find_commit(head_oid).unwrap();
        let parent_id = head_commit.parent_id(0).unwrap().to_string();

        // Soft reset to parent
        mgr.reset(&repo, &parent_id, ResetMode::Soft).unwrap();

        // HEAD should now point to the parent commit
        let new_head = repo.inner().head().unwrap().target().unwrap().to_string();
        assert_eq!(new_head, parent_id);

        // Working directory should still have b.txt
        let workdir = repo.inner().workdir().unwrap();
        assert!(workdir.join("b.txt").exists());

        // Staging area should have b.txt staged (since soft reset keeps index)
        let index = repo.inner().index().unwrap();
        let has_b = index.iter().any(|e| {
            String::from_utf8(e.path.clone()).unwrap() == "b.txt"
        });
        assert!(has_b, "b.txt should still be in the staging area after soft reset");
    }

    #[test]
    fn test_mixed_reset_resets_staging_preserves_workdir() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        // Create two commits
        add_file_and_commit(&repo, "a.txt", "content-a", "Second commit");
        let second_oid = repo.inner().head().unwrap().target().unwrap().to_string();
        add_file_and_commit(&repo, "b.txt", "content-b", "Third commit");

        // Mixed reset to second commit
        mgr.reset(&repo, &second_oid, ResetMode::Mixed).unwrap();

        // HEAD should point to second commit
        let new_head = repo.inner().head().unwrap().target().unwrap().to_string();
        assert_eq!(new_head, second_oid);

        // Working directory should still have b.txt
        let workdir = repo.inner().workdir().unwrap();
        assert!(workdir.join("b.txt").exists());

        // Staging area should NOT have b.txt (mixed reset resets the index)
        let index = repo.inner().index().unwrap();
        let has_b = index.iter().any(|e| {
            String::from_utf8(e.path.clone()).unwrap() == "b.txt"
        });
        assert!(!has_b, "b.txt should NOT be in the staging area after mixed reset");
    }

    #[test]
    fn test_hard_reset_resets_staging_and_workdir() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        // Create two commits
        add_file_and_commit(&repo, "a.txt", "content-a", "Second commit");
        let second_oid = repo.inner().head().unwrap().target().unwrap().to_string();
        add_file_and_commit(&repo, "b.txt", "content-b", "Third commit");

        // Hard reset to second commit
        mgr.reset(&repo, &second_oid, ResetMode::Hard).unwrap();

        // HEAD should point to second commit
        let new_head = repo.inner().head().unwrap().target().unwrap().to_string();
        assert_eq!(new_head, second_oid);

        // Working directory should NOT have b.txt (hard reset discards changes)
        let workdir = repo.inner().workdir().unwrap();
        assert!(!workdir.join("b.txt").exists(), "b.txt should be removed after hard reset");

        // Staging area should NOT have b.txt
        let index = repo.inner().index().unwrap();
        let has_b = index.iter().any(|e| {
            String::from_utf8(e.path.clone()).unwrap() == "b.txt"
        });
        assert!(!has_b, "b.txt should NOT be in the staging area after hard reset");
    }

    #[test]
    fn test_reset_invalid_target_fails() {
        let (_dir, repo) = setup_repo();
        let mgr = BranchManager::new();

        let result = mgr.reset(&repo, "nonexistent_revision_abc123", ResetMode::Soft);
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::InvalidArgument(msg) => {
                assert!(msg.contains("Invalid revision"));
            }
            other => panic!("Expected InvalidArgument, got {:?}", other),
        }
    }
}
