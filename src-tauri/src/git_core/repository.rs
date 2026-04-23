use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::GitError;
use crate::models::RepositoryState;

/// Callback type for reporting clone/fetch progress.
/// Receives (received_objects, total_objects, received_bytes).
pub type ProgressSender = Arc<dyn Fn(usize, usize, usize) + Send + Sync>;

/// A reference returned by `head()`, wrapping git2 reference info
/// without exposing any git2 types.
#[derive(Debug, Clone)]
pub struct GitReference {
    /// The full reference name, e.g. "refs/heads/main".
    pub name: Option<String>,
    /// The short human-readable name, e.g. "main".
    pub shorthand: Option<String>,
    /// The OID (SHA-1) the reference points to, if it can be resolved.
    pub target: Option<String>,
    /// Whether this reference is a branch.
    pub is_branch: bool,
    /// Whether this reference is a tag.
    pub is_tag: bool,
}

/// Wraps `git2::Repository` and provides a safe, high-level API.
/// No git2 raw types are exposed outside this module.
pub struct GitRepository {
    repo: git2::Repository,
    path: PathBuf,
}

impl GitRepository {
    /// Open an existing Git repository at the given path.
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let repo = git2::Repository::open(path).map_err(|e| {
            if e.class() == git2::ErrorClass::Repository {
                GitError::NotARepository {
                    path: path.display().to_string(),
                }
            } else {
                GitError::from(e)
            }
        })?;
        let canonical = repo
            .workdir()
            .unwrap_or_else(|| repo.path())
            .to_path_buf();
        Ok(Self {
            repo,
            path: canonical,
        })
    }

    /// Initialise a new Git repository at the given path.
    pub fn init(path: &Path) -> Result<Self, GitError> {
        let repo = git2::Repository::init(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            repo,
        })
    }

    /// Clone a remote repository into `path`, reporting progress via `progress`.
    pub fn clone(
        url: &str,
        path: &Path,
        progress: ProgressSender,
    ) -> Result<Self, GitError> {
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.transfer_progress(move |stats| {
            progress(
                stats.received_objects(),
                stats.total_objects(),
                stats.received_bytes(),
            );
            true
        });

        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_opts);

        let repo = builder.clone(url, path)?;
        let canonical = repo
            .workdir()
            .unwrap_or_else(|| repo.path())
            .to_path_buf();
        Ok(Self {
            repo,
            path: canonical,
        })
    }

    /// Return the HEAD reference information.
    pub fn head(&self) -> Result<GitReference, GitError> {
        let head = self.repo.head()?;
        let target = head.target().map(|oid| oid.to_string());
        Ok(GitReference {
            name: head.name().map(|s| s.to_string()),
            shorthand: head.shorthand().map(|s| s.to_string()),
            target,
            is_branch: head.is_branch(),
            is_tag: head.is_tag(),
        })
    }

    /// Whether the repository is bare (no working directory).
    pub fn is_bare(&self) -> bool {
        self.repo.is_bare()
    }

    /// The working directory path, if the repository is not bare.
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// The current repository state mapped to our domain enum.
    pub fn state(&self) -> RepositoryState {
        match self.repo.state() {
            git2::RepositoryState::Clean => RepositoryState::Clean,
            git2::RepositoryState::Merge => RepositoryState::Merging,
            git2::RepositoryState::Rebase
            | git2::RepositoryState::RebaseInteractive
            | git2::RepositoryState::RebaseMerge => {
                RepositoryState::Rebasing {
                    current: 0,
                    total: 0,
                }
            }
            git2::RepositoryState::CherryPick | git2::RepositoryState::CherryPickSequence => {
                RepositoryState::CherryPicking
            }
            git2::RepositoryState::Revert | git2::RepositoryState::RevertSequence => {
                RepositoryState::Reverting
            }
            // All other states (Apply*, Bisect) map to Clean for now.
            _ => RepositoryState::Clean,
        }
    }

    /// The path this repository was opened from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Provide a reference to the inner git2::Repository.
    /// This is `pub(crate)` so other modules inside the crate can use it,
    /// but it is NOT re-exported from `git_core` (mod.rs only exports the
    /// public items of this file).
    pub(crate) fn inner(&self) -> &git2::Repository {
        &self.repo
    }

    /// Provide a mutable reference to the inner git2::Repository.
    /// Needed for operations like stash that require `&mut`.
    pub(crate) fn inner_mut(&mut self) -> &mut git2::Repository {
        &mut self.repo
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        TempDir::new().expect("failed to create temp dir")
    }

    #[test]
    fn test_init_and_open() {
        let dir = temp_dir();
        let repo = GitRepository::init(dir.path()).expect("init failed");
        assert!(!repo.is_bare());
        assert!(repo.workdir().is_some());
        assert_eq!(repo.state(), RepositoryState::Clean);

        // Re-open the same repo
        let repo2 = GitRepository::open(dir.path()).expect("open failed");
        assert!(!repo2.is_bare());
    }

    #[test]
    fn test_open_non_repo_returns_error() {
        let dir = temp_dir();
        let result = GitRepository::open(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_head_on_fresh_repo() {
        let dir = temp_dir();
        let repo = GitRepository::init(dir.path()).expect("init failed");
        // A fresh repo with no commits has an unborn HEAD — head() returns an error.
        let result = repo.head();
        assert!(result.is_err());
    }

    #[test]
    fn test_head_after_commit() {
        let dir = temp_dir();
        let repo = GitRepository::init(dir.path()).expect("init failed");

        // Create an initial commit so HEAD is valid.
        let inner = repo.inner();
        let sig = inner.signature().unwrap_or_else(|_| {
            git2::Signature::now("Test", "test@test.com").unwrap()
        });
        let tree_id = inner.index().unwrap().write_tree().unwrap();
        let tree = inner.find_tree(tree_id).unwrap();
        inner
            .commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        let head = repo.head().expect("head should succeed after commit");
        assert!(head.target.is_some());
        assert!(head.is_branch);
    }

    #[test]
    fn test_clone_local_bare() {
        // Create a bare repo to clone from.
        let origin_dir = temp_dir();
        let origin = git2::Repository::init_bare(origin_dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = origin.index().unwrap().write_tree().unwrap();
        let tree = origin.find_tree(tree_id).unwrap();
        origin
            .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        let clone_dir = temp_dir();
        let received = Arc::new(AtomicUsize::new(0));
        let received_clone = received.clone();
        let progress: ProgressSender =
            Arc::new(move |_recv, _total, _bytes| {
                received_clone.fetch_add(1, Ordering::Relaxed);
            });

        let url = origin_dir.path().to_str().expect("path to str");
        let cloned =
            GitRepository::clone(url, clone_dir.path(), progress).expect("clone failed");
        assert!(!cloned.is_bare());
        assert!(cloned.workdir().is_some());
    }
}
