use crate::error::GitError;
use crate::git_core::{GitRepository, ProgressSender};
use crate::models::{PullResult, RemoteInfo};

use super::credential_manager::CredentialManager;

pub struct RemoteManager;

impl RemoteManager {
    pub fn new() -> Self {
        Self
    }

    /// Fetch from a specific remote or all remotes.
    pub fn fetch(
        &self,
        repo: &GitRepository,
        remote_name: Option<&str>,
        progress: ProgressSender,
    ) -> Result<(), GitError> {
        let inner = repo.inner();

        match remote_name {
            Some(name) => {
                self.fetch_one(inner, name, progress)?;
            }
            None => {
                let remote_names: Vec<String> = inner
                    .remotes()?
                    .iter()
                    .filter_map(|n| n.map(String::from))
                    .collect();
                for name in &remote_names {
                    self.fetch_one(inner, name, progress.clone())?;
                }
            }
        }
        Ok(())
    }

    /// Pull from remote (fetch + merge). Returns PullResult.
    /// If working directory has uncommitted changes, auto-stash before pull and pop after.
    pub fn pull(
        &self,
        repo: &mut GitRepository,
        remote_name: Option<&str>,
        progress: ProgressSender,
    ) -> Result<PullResult, GitError> {
        let did_stash = self.auto_stash_if_dirty(repo.inner_mut())?;

        // Perform fetch + merge using immutable borrow in a block so it's dropped before stash pop
        let result = {
            let inner = repo.inner();

            let remote_str = self.resolve_remote_name(inner, remote_name)?;
            self.fetch_one(inner, &remote_str, progress)?;

            let head_ref = inner.head()?;
            let head_branch_name = head_ref
                .shorthand()
                .ok_or_else(|| GitError::InvalidArgument("HEAD is not on a branch".into()))?
                .to_string();

            // Resolve the remote commit to merge
            let remote_oid = self.resolve_fetch_target(inner, &remote_str, &head_branch_name)?;
            let remote_commit = inner.find_annotated_commit(remote_oid)?;

            let (analysis, _preference) = inner.merge_analysis(&[&remote_commit])?;

            if analysis.is_up_to_date() {
                PullResult::UpToDate
            } else if analysis.is_fast_forward() {
                self.fast_forward(inner, &head_branch_name, &remote_commit)?;
                PullResult::FastForward
            } else if analysis.is_normal() {
                self.normal_merge(inner, &remote_commit)?
            } else {
                PullResult::UpToDate
            }
        };

        // Pop stash if we stashed (needs &mut)
        if did_stash {
            let inner_mut = repo.inner_mut();
            let mut checkout_builder = git2::build::CheckoutBuilder::new();
            checkout_builder.force();
            let mut apply_opts = git2::StashApplyOptions::new();
            apply_opts.checkout_options(checkout_builder);
            inner_mut.stash_pop(0, Some(&mut apply_opts))?;
        }

        Ok(result)
    }

    /// Push to remote. Supports force push.
    pub fn push(
        &self,
        repo: &GitRepository,
        remote_name: Option<&str>,
        force: bool,
        progress: ProgressSender,
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        let remote_str = self.resolve_remote_name(inner, remote_name)?;

        let head_ref = inner.head()?;
        let branch_name = head_ref
            .shorthand()
            .ok_or_else(|| GitError::InvalidArgument("HEAD is not on a branch".into()))?;

        let refspec = if force {
            format!("+refs/heads/{}:refs/heads/{}", branch_name, branch_name)
        } else {
            format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name)
        };

        let cred_mgr = CredentialManager::new();
        let callbacks = cred_mgr.create_callbacks(Some(progress));
        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        let mut remote = inner.find_remote(&remote_str)?;
        remote
            .push(&[&refspec], Some(&mut push_opts))
            .map_err(|e| {
                if e.message().contains("rejected") {
                    GitError::RemoteRejected {
                        reason: e.message().to_string(),
                    }
                } else {
                    GitError::from(e)
                }
            })?;

        Ok(())
    }

    /// Add a new remote.
    pub fn add_remote(
        &self,
        repo: &GitRepository,
        name: &str,
        url: &str,
    ) -> Result<(), GitError> {
        repo.inner().remote(name, url)?;
        Ok(())
    }

    /// Remove a remote.
    pub fn remove_remote(&self, repo: &GitRepository, name: &str) -> Result<(), GitError> {
        repo.inner().remote_delete(name)?;
        Ok(())
    }

    /// List all remotes.
    pub fn list_remotes(&self, repo: &GitRepository) -> Result<Vec<RemoteInfo>, GitError> {
        let inner = repo.inner();
        let remotes = inner.remotes()?;
        let mut results = Vec::new();

        for name in remotes.iter().flatten() {
            if let Ok(remote) = inner.find_remote(name) {
                results.push(RemoteInfo {
                    name: name.to_string(),
                    url: remote.url().unwrap_or("").to_string(),
                    push_url: remote.pushurl().map(|s| s.to_string()),
                });
            }
        }

        Ok(results)
    }

    // --- Private helpers ---

    fn resolve_remote_name(
        &self,
        inner: &git2::Repository,
        remote_name: Option<&str>,
    ) -> Result<String, GitError> {
        match remote_name {
            Some(name) => Ok(name.to_string()),
            None => {
                let remotes = inner.remotes()?;
                if remotes.iter().flatten().any(|n| n == "origin") {
                    Ok("origin".to_string())
                } else {
                    remotes
                        .iter()
                        .flatten()
                        .next()
                        .map(|n| n.to_string())
                        .ok_or_else(|| {
                            GitError::InvalidArgument("No remotes configured".into())
                        })
                }
            }
        }
    }

    fn fetch_one(
        &self,
        inner: &git2::Repository,
        name: &str,
        progress: ProgressSender,
    ) -> Result<(), GitError> {
        let cred_mgr = CredentialManager::new();
        let callbacks = cred_mgr.create_callbacks(Some(progress));
        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let mut remote = inner.find_remote(name)?;
        remote.fetch(&[] as &[&str], Some(&mut fetch_opts), None)?;
        Ok(())
    }

    fn resolve_fetch_target(
        &self,
        inner: &git2::Repository,
        remote_str: &str,
        head_branch_name: &str,
    ) -> Result<git2::Oid, GitError> {
        // Try FETCH_HEAD first
        if let Ok(fh) = inner.find_reference("FETCH_HEAD") {
            if let Some(oid) = fh.target() {
                return Ok(oid);
            }
        }

        // Fall back to remote tracking branch
        let tracking_ref = format!("refs/remotes/{}/{}", remote_str, head_branch_name);
        let reference = inner.find_reference(&tracking_ref).map_err(|_| {
            GitError::Git2(format!(
                "No tracking branch found for {}/{}",
                remote_str, head_branch_name
            ))
        })?;
        reference
            .target()
            .ok_or_else(|| GitError::Git2("Remote ref has no target".into()))
    }

    fn auto_stash_if_dirty(
        &self,
        inner: &mut git2::Repository,
    ) -> Result<bool, GitError> {
        // Check status with a temporary immutable borrow
        let is_dirty = {
            let statuses = inner.statuses(Some(
                git2::StatusOptions::new()
                    .include_untracked(true)
                    .recurse_untracked_dirs(false),
            ))?;
            !statuses.is_empty()
        };

        if !is_dirty {
            return Ok(false);
        }

        let sig = inner
            .signature()
            .unwrap_or_else(|_| git2::Signature::now("git-gui", "git-gui@local").unwrap());

        match inner.stash_save(&sig, "auto-stash before pull", Some(git2::StashFlags::DEFAULT)) {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.message().contains("nothing to stash") {
                    Ok(false)
                } else {
                    Err(GitError::from(e))
                }
            }
        }
    }

    fn fast_forward(
        &self,
        inner: &git2::Repository,
        branch_name: &str,
        remote_commit: &git2::AnnotatedCommit,
    ) -> Result<(), GitError> {
        let refname = format!("refs/heads/{}", branch_name);
        let mut reference = inner.find_reference(&refname)?;
        reference.set_target(
            remote_commit.id(),
            &format!("Fast-forward {} to {}", branch_name, remote_commit.id()),
        )?;
        inner.set_head(&refname)?;
        inner.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
        Ok(())
    }

    fn normal_merge(
        &self,
        inner: &git2::Repository,
        remote_commit: &git2::AnnotatedCommit,
    ) -> Result<PullResult, GitError> {
        inner.merge(&[remote_commit], None, None)?;

        let index = inner.index()?;
        if index.has_conflicts() {
            let conflicts: Vec<String> = index
                .conflicts()?
                .filter_map(|c| c.ok())
                .filter_map(|c| {
                    c.our
                        .as_ref()
                        .and_then(|e| String::from_utf8(e.path.clone()).ok())
                })
                .collect();
            return Ok(PullResult::Conflict { files: conflicts });
        }

        // Create merge commit
        let mut index = inner.index()?;
        let tree_oid = index.write_tree()?;
        let tree = inner.find_tree(tree_oid)?;

        let sig = inner
            .signature()
            .unwrap_or_else(|_| git2::Signature::now("git-gui", "git-gui@local").unwrap());

        let head_commit = inner.head()?.peel_to_commit()?;
        let remote_commit_obj = inner.find_commit(remote_commit.id())?;

        inner.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Merge remote-tracking branch",
            &tree,
            &[&head_commit, &remote_commit_obj],
        )?;

        inner.cleanup_state()?;

        Ok(PullResult::Merged)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn setup_bare_remote() -> (TempDir, git2::Oid) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init_bare(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
        (dir, oid)
    }

    fn clone_from_bare(bare_dir: &TempDir) -> (TempDir, GitRepository) {
        let clone_dir = TempDir::new().unwrap();
        let progress: ProgressSender = Arc::new(|_, _, _| {});
        let url = bare_dir.path().to_str().unwrap();
        let repo = GitRepository::clone(url, clone_dir.path(), progress).unwrap();
        {
            let inner = repo.inner();
            let mut config = inner.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        (clone_dir, repo)
    }

    fn noop_progress() -> ProgressSender {
        Arc::new(|_, _, _| {})
    }

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

    #[test]
    fn test_list_remotes() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir, repo) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        let remotes = mgr.list_remotes(&repo).unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].name, "origin");
        assert!(!remotes[0].url.is_empty());
    }

    #[test]
    fn test_add_remote() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir, repo) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        mgr.add_remote(&repo, "upstream", "https://example.com/repo.git")
            .unwrap();

        let remotes = mgr.list_remotes(&repo).unwrap();
        assert_eq!(remotes.len(), 2);
        let names: Vec<&str> = remotes.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"origin"));
        assert!(names.contains(&"upstream"));
    }

    #[test]
    fn test_remove_remote() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir, repo) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        mgr.add_remote(&repo, "upstream", "https://example.com/repo.git")
            .unwrap();
        assert_eq!(mgr.list_remotes(&repo).unwrap().len(), 2);

        mgr.remove_remote(&repo, "upstream").unwrap();
        let remotes = mgr.list_remotes(&repo).unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].name, "origin");
    }

    #[test]
    fn test_fetch() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir, repo) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        mgr.fetch(&repo, Some("origin"), noop_progress()).unwrap();
    }

    #[test]
    fn test_fetch_all() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir, repo) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        mgr.fetch(&repo, None, noop_progress()).unwrap();
    }

    #[test]
    fn test_push() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir, repo) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        add_file_and_commit(&repo, "new.txt", "content", "new commit");
        mgr.push(&repo, Some("origin"), false, noop_progress())
            .unwrap();

        // Verify the bare repo received the commit
        let bare_repo = git2::Repository::open_bare(bare_dir.path()).unwrap();
        let head = bare_repo.head().unwrap().target().unwrap();
        let commit = bare_repo.find_commit(head).unwrap();
        assert_eq!(commit.message().unwrap(), "new commit");
    }

    #[test]
    fn test_push_force() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir, repo) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        add_file_and_commit(&repo, "f.txt", "data", "commit to push");
        mgr.push(&repo, Some("origin"), true, noop_progress())
            .unwrap();
    }

    #[test]
    fn test_pull_up_to_date() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir, mut repo) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        let result = mgr.pull(&mut repo, Some("origin"), noop_progress()).unwrap();
        assert_eq!(result, PullResult::UpToDate);
    }

    #[test]
    fn test_pull_fast_forward() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir1, repo1) = clone_from_bare(&bare_dir);
        let (_clone_dir2, mut repo2) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        // Push a new commit from repo1
        add_file_and_commit(&repo1, "ff.txt", "data", "ff commit");
        mgr.push(&repo1, Some("origin"), false, noop_progress())
            .unwrap();

        // Pull from repo2 should fast-forward
        let result = mgr
            .pull(&mut repo2, Some("origin"), noop_progress())
            .unwrap();
        assert_eq!(result, PullResult::FastForward);

        // Verify the file exists in repo2
        let workdir = repo2.inner().workdir().unwrap();
        assert!(workdir.join("ff.txt").exists());
    }

    #[test]
    fn test_add_duplicate_remote_fails() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir, repo) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        let result = mgr.add_remote(&repo, "origin", "https://example.com/other.git");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_nonexistent_remote_fails() {
        let (bare_dir, _) = setup_bare_remote();
        let (_clone_dir, repo) = clone_from_bare(&bare_dir);
        let mgr = RemoteManager::new();

        let result = mgr.remove_remote(&repo, "nonexistent");
        assert!(result.is_err());
    }
}
