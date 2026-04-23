use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::{RebaseProgress, RebaseStepStatus};

pub struct RebaseService;

impl RebaseService {
    pub fn new() -> Self {
        Self
    }

    /// Start a rebase of the current branch onto the target branch.
    pub fn start_rebase(
        &self,
        repo: &mut GitRepository,
        onto: &str,
    ) -> Result<RebaseProgress, GitError> {
        let onto_oid = {
            let inner = repo.inner();
            let reference = inner
                .resolve_reference_from_short_name(onto)
                .map_err(|e| GitError::InvalidArgument(format!("Cannot resolve '{}': {}", onto, e)))?;
            reference
                .peel_to_commit()
                .map_err(|e| GitError::InvalidArgument(format!("Cannot peel '{}' to commit: {}", onto, e)))?
                .id()
        };

        let inner = repo.inner_mut();
        let onto_annotated = inner.find_annotated_commit(onto_oid)?;

        let mut rebase = inner.rebase(None, None, Some(&onto_annotated), None)?;

        // Advance to the first step
        match rebase.next() {
            Some(Ok(_op)) => {
                let progress = self.build_progress_from_rebase(&mut rebase, inner)?;
                Ok(progress)
            }
            Some(Err(e)) => Err(GitError::from(e)),
            None => {
                // No operations to replay — rebase is already complete
                rebase.finish(None)?;
                Ok(RebaseProgress {
                    current_step: 0,
                    total_steps: 0,
                    status: RebaseStepStatus::Completed,
                })
            }
        }
    }

    /// Continue the rebase after resolving conflicts.
    /// Commits the current step and moves to the next.
    pub fn continue_rebase(
        &self,
        repo: &mut GitRepository,
    ) -> Result<RebaseProgress, GitError> {
        let inner = repo.inner_mut();
        let mut rebase = inner.open_rebase(None)?;

        let sig = inner.signature().or_else(|_| {
            git2::Signature::now("Unknown", "unknown@example.com")
        })?;

        // Commit the current (resolved) step
        rebase.commit(None, &sig, None)?;

        // Advance to the next step
        match rebase.next() {
            Some(Ok(_op)) => {
                let progress = self.build_progress_from_rebase(&mut rebase, inner)?;
                Ok(progress)
            }
            Some(Err(e)) => Err(GitError::from(e)),
            None => {
                // All steps done
                rebase.finish(None)?;
                let total = rebase.len();
                Ok(RebaseProgress {
                    current_step: total,
                    total_steps: total,
                    status: RebaseStepStatus::Completed,
                })
            }
        }
    }

    /// Abort the current rebase and restore the repository to its pre-rebase state.
    pub fn abort_rebase(&self, repo: &mut GitRepository) -> Result<(), GitError> {
        let inner = repo.inner_mut();
        let mut rebase = inner.open_rebase(None)?;
        rebase.abort()?;
        Ok(())
    }

    /// Get the current rebase status. Returns None if no rebase is in progress.
    pub fn rebase_status(&self, repo: &GitRepository) -> Result<Option<RebaseProgress>, GitError> {
        let inner = repo.inner();
        let state = inner.state();

        match state {
            git2::RepositoryState::Rebase
            | git2::RepositoryState::RebaseInteractive
            | git2::RepositoryState::RebaseMerge => {}
            _ => return Ok(None),
        }

        // Open the rebase to read progress. This requires &mut for open_rebase,
        // but we only need read access. We'll work around by checking the rebase
        // directory directly.
        // Unfortunately git2's open_rebase requires &Repository (not &mut), let's check.
        let mut rebase = inner.open_rebase(None)?;
        let current = rebase.operation_current();
        let total = rebase.len();

        let current_step = current.map(|c| c + 1).unwrap_or(0);

        // Check for conflicts in the index
        let status = self.check_conflict_status(inner)?;

        Ok(Some(RebaseProgress {
            current_step,
            total_steps: total,
            status,
        }))
    }

    /// Check the index for conflicts and return the appropriate status.
    fn check_conflict_status(
        &self,
        inner: &git2::Repository,
    ) -> Result<RebaseStepStatus, GitError> {
        let index = inner.index()?;
        if index.has_conflicts() {
            let mut conflict_files = Vec::new();
            for conflict in index.conflicts()? {
                let conflict = conflict?;
                if let Some(entry) = conflict.our.or(conflict.their).or(conflict.ancestor) {
                    let path = String::from_utf8_lossy(&entry.path).to_string();
                    if !conflict_files.contains(&path) {
                        conflict_files.push(path);
                    }
                }
            }
            Ok(RebaseStepStatus::Conflict {
                files: conflict_files,
            })
        } else {
            Ok(RebaseStepStatus::InProgress)
        }
    }

    /// Build a RebaseProgress from the current rebase state.
    fn build_progress_from_rebase(
        &self,
        rebase: &mut git2::Rebase<'_>,
        inner: &git2::Repository,
    ) -> Result<RebaseProgress, GitError> {
        let current = rebase.operation_current();
        let total = rebase.len();
        let current_step = current.map(|c| c + 1).unwrap_or(0);

        let status = self.check_conflict_status(inner)?;

        Ok(RebaseProgress {
            current_step,
            total_steps: total,
            status,
        })
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

    /// Helper: create a file, stage it, and commit. Returns the new commit OID.
    fn add_file_and_commit(
        repo: &GitRepository,
        filename: &str,
        content: &str,
        message: &str,
    ) -> git2::Oid {
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
            .unwrap()
    }

    /// Helper: create a branch at the given commit OID.
    fn create_branch_at(repo: &GitRepository, name: &str, oid: git2::Oid) {
        let inner = repo.inner();
        let commit = inner.find_commit(oid).unwrap();
        inner.branch(name, &commit, false).unwrap();
    }

    /// Helper: checkout a branch by name.
    fn checkout_branch(repo: &GitRepository, name: &str) {
        let inner = repo.inner();
        let refname = format!("refs/heads/{}", name);
        inner.set_head(&refname).unwrap();
        inner.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();
    }

    /// Helper: get the current HEAD branch name.
    fn head_branch_name(repo: &GitRepository) -> String {
        let inner = repo.inner();
        inner.head().unwrap().shorthand().unwrap().to_string()
    }

    #[test]
    fn test_start_rebase_non_conflicting() {
        let (_dir, mut repo) = setup_repo();
        let default_branch = head_branch_name(&repo);

        // Create a commit on the default branch
        let base_oid = add_file_and_commit(&repo, "base.txt", "base content", "base commit");

        // Create a feature branch from base
        create_branch_at(&repo, "feature", base_oid);

        // Add another commit on the default branch
        add_file_and_commit(&repo, "main_file.txt", "main content", "main commit");

        // Switch to feature and add a commit there (non-conflicting file)
        checkout_branch(&repo, "feature");
        add_file_and_commit(&repo, "feature_file.txt", "feature content", "feature commit");

        // Rebase feature onto the default branch
        let svc = RebaseService::new();
        let progress = svc.start_rebase(&mut repo, &default_branch).unwrap();

        assert_eq!(progress.total_steps, 1);
        assert_eq!(progress.current_step, 1);
        // Should be InProgress (no conflicts)
        assert!(matches!(progress.status, RebaseStepStatus::InProgress));
    }

    #[test]
    fn test_abort_rebase() {
        let (_dir, mut repo) = setup_repo();
        let default_branch = head_branch_name(&repo);

        let base_oid = add_file_and_commit(&repo, "base.txt", "base content", "base commit");
        create_branch_at(&repo, "feature", base_oid);

        add_file_and_commit(&repo, "main_file.txt", "main content", "main commit");

        checkout_branch(&repo, "feature");
        add_file_and_commit(&repo, "feature_file.txt", "feature content", "feature commit");

        let svc = RebaseService::new();
        let _progress = svc.start_rebase(&mut repo, &default_branch).unwrap();

        // Abort the rebase
        svc.abort_rebase(&mut repo).unwrap();

        // After abort, repo should not be in rebase state
        let status = svc.rebase_status(&repo).unwrap();
        assert!(status.is_none());
    }

    #[test]
    fn test_rebase_status_no_rebase() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "file.txt", "content", "a commit");

        let svc = RebaseService::new();
        let status = svc.rebase_status(&repo).unwrap();
        assert!(status.is_none());
    }

    #[test]
    fn test_rebase_with_conflicts() {
        let (_dir, mut repo) = setup_repo();
        let default_branch = head_branch_name(&repo);

        // Create a base commit with a shared file
        let base_oid = add_file_and_commit(&repo, "shared.txt", "original content", "base commit");

        // Create feature branch from base
        create_branch_at(&repo, "feature", base_oid);

        // Modify shared.txt on the default branch
        add_file_and_commit(&repo, "shared.txt", "main's version", "main modifies shared");

        // Switch to feature and modify the same file differently
        checkout_branch(&repo, "feature");
        add_file_and_commit(&repo, "shared.txt", "feature's version", "feature modifies shared");

        // Rebase feature onto the default branch — should produce a conflict
        let svc = RebaseService::new();
        let progress = svc.start_rebase(&mut repo, &default_branch).unwrap();

        assert_eq!(progress.total_steps, 1);
        assert_eq!(progress.current_step, 1);
        match &progress.status {
            RebaseStepStatus::Conflict { files } => {
                assert!(files.contains(&"shared.txt".to_string()));
            }
            other => panic!("Expected Conflict status, got {:?}", other),
        }
    }
}
