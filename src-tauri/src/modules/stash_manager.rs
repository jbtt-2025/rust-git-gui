use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::{
    DiffFileStatus, DiffHunk, DiffLine, DiffLineType, FileDiff, StashEntry, StashPopResult,
};

pub struct StashManager;

impl StashManager {
    pub fn new() -> Self {
        Self
    }

    /// Save the current working directory and index state as a new stash entry.
    /// An optional description message can be provided.
    pub fn create_stash(
        &self,
        repo: &mut GitRepository,
        message: Option<&str>,
    ) -> Result<StashEntry, GitError> {
        let inner = repo.inner_mut();
        let sig = inner
            .signature()
            .or_else(|_| git2::Signature::now("Stash", "stash@localhost"))
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        let msg = message.unwrap_or("WIP on stash");
        let oid = inner
            .stash_save(&sig, msg, None)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        let commit = inner
            .find_commit(oid)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;
        let timestamp = commit.time().seconds();

        Ok(StashEntry {
            index: 0,
            message: msg.to_string(),
            timestamp,
            commit_id: oid.to_string(),
        })
    }

    /// List all stash entries in the repository.
    pub fn list_stashes(
        &self,
        repo: &mut GitRepository,
    ) -> Result<Vec<StashEntry>, GitError> {
        let mut entries = Vec::new();
        repo.inner_mut()
            .stash_foreach(|index, message, oid| {
                entries.push(StashEntry {
                    index,
                    message: message.to_string(),
                    timestamp: 0, // will be filled below
                    commit_id: oid.to_string(),
                });
                true
            })
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        // Fill in timestamps from the commit objects
        for entry in &mut entries {
            if let Ok(oid) = git2::Oid::from_str(&entry.commit_id) {
                if let Ok(commit) = repo.inner().find_commit(oid) {
                    entry.timestamp = commit.time().seconds();
                }
            }
        }

        Ok(entries)
    }

    /// Apply a stash entry to the working directory without removing it.
    pub fn apply_stash(
        &self,
        repo: &mut GitRepository,
        index: usize,
    ) -> Result<(), GitError> {
        repo.inner_mut()
            .stash_apply(index, None)
            .map_err(|e| {
                if e.class() == git2::ErrorClass::Stash
                    || e.class() == git2::ErrorClass::Checkout
                    || e.message().contains("conflict")
                {
                    GitError::MergeConflict { files: vec![] }
                } else {
                    GitError::Git2(e.message().to_string())
                }
            })
    }

    /// Pop a stash entry: apply it and remove it from the stash list.
    /// Returns StashPopResult::Success or StashPopResult::Conflict.
    pub fn pop_stash(
        &self,
        repo: &mut GitRepository,
        index: usize,
    ) -> Result<StashPopResult, GitError> {
        let result = repo.inner_mut().stash_pop(index, None);
        match result {
            Ok(()) => Ok(StashPopResult::Success),
            Err(e) => {
                // git2 stash_pop returns an error on conflict but still applies changes.
                // Check if it's a checkout conflict.
                if e.class() == git2::ErrorClass::Checkout
                    || e.class() == git2::ErrorClass::Merge
                    || e.message().contains("conflict")
                {
                    // Collect conflicted files from the index
                    let conflict_files = self.collect_conflict_files(repo);
                    Ok(StashPopResult::Conflict {
                        files: conflict_files,
                    })
                } else {
                    Err(GitError::Git2(e.message().to_string()))
                }
            }
        }
    }

    /// Drop (delete) a stash entry by index.
    pub fn drop_stash(
        &self,
        repo: &mut GitRepository,
        index: usize,
    ) -> Result<(), GitError> {
        repo.inner_mut()
            .stash_drop(index)
            .map_err(|e| GitError::Git2(e.message().to_string()))
    }

    /// Show the diff of a stash entry (what changes the stash contains).
    /// Diffs the stash commit's tree against its first parent's tree.
    pub fn stash_diff(
        &self,
        repo: &GitRepository,
        index: usize,
    ) -> Result<Vec<FileDiff>, GitError> {
        let inner = repo.inner();

        // Find the stash reference at the given index.
        // stash@{index} is stored as refs/stash with reflog entries.
        let stash_oid = self.get_stash_oid(repo, index)?;
        let stash_commit = inner
            .find_commit(stash_oid)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        let stash_tree = stash_commit
            .tree()
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        // The first parent of a stash commit is the commit HEAD was at when stash was created
        let parent_tree = if stash_commit.parent_count() > 0 {
            Some(
                stash_commit
                    .parent(0)
                    .map_err(|e| GitError::Git2(e.message().to_string()))?
                    .tree()
                    .map_err(|e| GitError::Git2(e.message().to_string()))?,
            )
        } else {
            None
        };

        let diff = inner
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&stash_tree), None)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        self.parse_diff(&diff)
    }

    // --- Private helpers ---

    /// Get the OID of a stash entry by index using stash_foreach.
    fn get_stash_oid(
        &self,
        repo: &GitRepository,
        target_index: usize,
    ) -> Result<git2::Oid, GitError> {
        // We need &mut for stash_foreach, but we only have &self and &GitRepository.
        // Instead, read the stash reflog directly.
        let inner = repo.inner();
        let reflog = inner
            .reflog("refs/stash")
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        let entry = reflog.get(target_index).ok_or_else(|| {
            GitError::InvalidArgument(format!("Stash index {} not found", target_index))
        })?;

        Ok(entry.id_new())
    }

    /// Collect conflicted file paths from the repository index.
    fn collect_conflict_files(&self, repo: &GitRepository) -> Vec<String> {
        let inner = repo.inner();
        let mut files = Vec::new();
        if let Ok(index) = inner.index() {
            if let Ok(conflicts) = index.conflicts() {
                for conflict in conflicts.flatten() {
                    if let Some(entry) = conflict.our.or(conflict.their).or(conflict.ancestor) {
                        let path = String::from_utf8_lossy(&entry.path).to_string();
                        if !files.contains(&path) {
                            files.push(path);
                        }
                    }
                }
            }
        }
        files
    }

    /// Parse a git2::Diff into our domain Vec<FileDiff>.
    fn parse_diff(&self, diff: &git2::Diff) -> Result<Vec<FileDiff>, GitError> {
        use std::cell::RefCell;

        let file_diffs: RefCell<Vec<FileDiff>> = RefCell::new(Vec::new());

        diff.foreach(
            &mut |delta, _progress| {
                let new_file = delta.new_file();
                let old_file = delta.old_file();

                let path = new_file
                    .path()
                    .or_else(|| old_file.path())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let old_path = if delta.status() == git2::Delta::Renamed
                    || delta.status() == git2::Delta::Copied
                {
                    old_file.path().map(|p| p.to_string_lossy().to_string())
                } else {
                    None
                };

                let status = match delta.status() {
                    git2::Delta::Added | git2::Delta::Untracked => DiffFileStatus::Added,
                    git2::Delta::Deleted => DiffFileStatus::Deleted,
                    git2::Delta::Modified => DiffFileStatus::Modified,
                    git2::Delta::Renamed => DiffFileStatus::Renamed,
                    git2::Delta::Copied => DiffFileStatus::Copied,
                    _ => DiffFileStatus::Modified,
                };

                let is_binary = delta.flags().contains(git2::DiffFlags::BINARY)
                    || new_file.is_binary()
                    || old_file.is_binary();

                file_diffs.borrow_mut().push(FileDiff {
                    path,
                    old_path,
                    status,
                    hunks: Vec::new(),
                    is_binary,
                });
                true
            },
            Some(&mut |_delta, _binary| {
                if let Some(last) = file_diffs.borrow_mut().last_mut() {
                    last.is_binary = true;
                }
                true
            }),
            Some(&mut |_delta, hunk| {
                let mut files = file_diffs.borrow_mut();
                if let Some(file_diff) = files.last_mut() {
                    if file_diff.is_binary {
                        return true;
                    }
                    let header = std::str::from_utf8(hunk.header())
                        .unwrap_or("")
                        .trim_end()
                        .to_string();
                    file_diff.hunks.push(DiffHunk {
                        header,
                        old_start: hunk.old_start(),
                        old_lines: hunk.old_lines(),
                        new_start: hunk.new_start(),
                        new_lines: hunk.new_lines(),
                        lines: Vec::new(),
                    });
                }
                true
            }),
            Some(&mut |_delta, _hunk, line| {
                let mut files = file_diffs.borrow_mut();
                if let Some(file_diff) = files.last_mut() {
                    if file_diff.is_binary {
                        return true;
                    }
                    if let Some(current_hunk) = file_diff.hunks.last_mut() {
                        let origin = match line.origin() {
                            '+' => DiffLineType::Addition,
                            '-' => DiffLineType::Deletion,
                            ' ' => DiffLineType::Context,
                            _ => return true,
                        };

                        let content = std::str::from_utf8(line.content())
                            .unwrap_or("")
                            .to_string();

                        current_hunk.lines.push(DiffLine {
                            origin,
                            old_lineno: line.old_lineno(),
                            new_lineno: line.new_lineno(),
                            content,
                        });
                    }
                }
                true
            }),
        )
        .map_err(|e| GitError::Git2(e.message().to_string()))?;

        Ok(file_diffs.into_inner())
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
        let mut repo = GitRepository::init(dir.path()).unwrap();

        {
            let inner = repo.inner_mut();
            let mut config = inner.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();

            let workdir = inner.workdir().unwrap().to_path_buf();
            fs::write(workdir.join("hello.txt"), "line1\nline2\nline3\n").unwrap();

            let mut index = inner.index().unwrap();
            index.add_path(Path::new("hello.txt")).unwrap();
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

    #[test]
    fn test_create_stash_with_message() {
        let (dir, mut repo) = setup_repo();
        let workdir = dir.path();

        // Make a change in the working directory
        fs::write(workdir.join("hello.txt"), "modified\n").unwrap();

        let manager = StashManager::new();
        let entry = manager
            .create_stash(&mut repo, Some("my stash message"))
            .unwrap();

        assert_eq!(entry.index, 0);
        assert_eq!(entry.message, "my stash message");
        assert!(!entry.commit_id.is_empty());
        assert!(entry.timestamp > 0);
    }

    #[test]
    fn test_create_stash_default_message() {
        let (dir, mut repo) = setup_repo();
        let workdir = dir.path();

        fs::write(workdir.join("hello.txt"), "modified\n").unwrap();

        let manager = StashManager::new();
        let entry = manager.create_stash(&mut repo, None).unwrap();

        assert_eq!(entry.message, "WIP on stash");
    }

    #[test]
    fn test_create_stash_no_changes_fails() {
        let (_dir, mut repo) = setup_repo();

        let manager = StashManager::new();
        let result = manager.create_stash(&mut repo, Some("empty"));

        assert!(result.is_err());
    }

    #[test]
    fn test_list_stashes() {
        let (dir, mut repo) = setup_repo();
        let workdir = dir.path();

        let manager = StashManager::new();

        // Create two stashes
        fs::write(workdir.join("hello.txt"), "change 1\n").unwrap();
        manager
            .create_stash(&mut repo, Some("first stash"))
            .unwrap();

        fs::write(workdir.join("hello.txt"), "change 2\n").unwrap();
        manager
            .create_stash(&mut repo, Some("second stash"))
            .unwrap();

        let stashes = manager.list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 2);

        // Most recent stash is at index 0
        assert!(stashes[0].message.contains("second stash"));
        assert!(stashes[1].message.contains("first stash"));
        assert_eq!(stashes[0].index, 0);
        assert_eq!(stashes[1].index, 1);
    }

    #[test]
    fn test_list_stashes_empty() {
        let (_dir, mut repo) = setup_repo();

        let manager = StashManager::new();
        let stashes = manager.list_stashes(&mut repo).unwrap();
        assert!(stashes.is_empty());
    }

    #[test]
    fn test_apply_stash() {
        let (dir, mut repo) = setup_repo();
        let workdir = dir.path();

        // Modify and stash
        fs::write(workdir.join("hello.txt"), "stashed content\n").unwrap();
        let manager = StashManager::new();
        manager
            .create_stash(&mut repo, Some("test apply"))
            .unwrap();

        // Working dir should be clean after stash
        let content = fs::read_to_string(workdir.join("hello.txt")).unwrap();
        assert!(content.contains("line1") && content.contains("line2"));

        // Apply the stash
        manager.apply_stash(&mut repo, 0).unwrap();

        // Working dir should have the stashed content back
        let content = fs::read_to_string(workdir.join("hello.txt")).unwrap();
        assert!(content.contains("stashed content"));

        // Stash should still exist after apply
        let stashes = manager.list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
    }

    #[test]
    fn test_pop_stash_success() {
        let (dir, mut repo) = setup_repo();
        let workdir = dir.path();

        fs::write(workdir.join("hello.txt"), "stashed content\n").unwrap();
        let manager = StashManager::new();
        manager.create_stash(&mut repo, Some("test pop")).unwrap();

        let result = manager.pop_stash(&mut repo, 0).unwrap();
        assert_eq!(result, StashPopResult::Success);

        // Working dir should have the stashed content
        let content = fs::read_to_string(workdir.join("hello.txt")).unwrap();
        assert!(content.contains("stashed content"));

        // Stash should be removed after pop
        let stashes = manager.list_stashes(&mut repo).unwrap();
        assert!(stashes.is_empty());
    }

    #[test]
    fn test_drop_stash() {
        let (dir, mut repo) = setup_repo();
        let workdir = dir.path();

        fs::write(workdir.join("hello.txt"), "change\n").unwrap();
        let manager = StashManager::new();
        manager.create_stash(&mut repo, Some("to drop")).unwrap();

        let stashes = manager.list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);

        manager.drop_stash(&mut repo, 0).unwrap();

        let stashes = manager.list_stashes(&mut repo).unwrap();
        assert!(stashes.is_empty());
    }

    #[test]
    fn test_drop_stash_invalid_index() {
        let (_dir, mut repo) = setup_repo();

        let manager = StashManager::new();
        let result = manager.drop_stash(&mut repo, 99);
        assert!(result.is_err());
    }

    #[test]
    fn test_stash_diff() {
        let (dir, mut repo) = setup_repo();
        let workdir = dir.path();

        // Modify a file and create a stash
        fs::write(workdir.join("hello.txt"), "line1\nmodified\nline3\n").unwrap();
        let manager = StashManager::new();
        manager
            .create_stash(&mut repo, Some("diff test"))
            .unwrap();

        let diffs = manager.stash_diff(&repo, 0).unwrap();
        assert!(!diffs.is_empty());

        let file_diff = diffs.iter().find(|d| d.path == "hello.txt").unwrap();
        assert_eq!(file_diff.status, DiffFileStatus::Modified);
        assert!(!file_diff.hunks.is_empty());
    }

    #[test]
    fn test_stash_diff_new_file() {
        let (dir, mut repo) = setup_repo();
        let workdir = dir.path();

        // Add a new file and stash
        fs::write(workdir.join("new_file.txt"), "new content\n").unwrap();
        {
            let inner = repo.inner();
            let mut index = inner.index().unwrap();
            index.add_path(Path::new("new_file.txt")).unwrap();
            index.write().unwrap();
        }

        let manager = StashManager::new();
        manager
            .create_stash(&mut repo, Some("new file stash"))
            .unwrap();

        let diffs = manager.stash_diff(&repo, 0).unwrap();
        let new_file_diff = diffs.iter().find(|d| d.path == "new_file.txt");
        assert!(new_file_diff.is_some());
    }

    #[test]
    fn test_stash_diff_invalid_index() {
        let (_dir, repo) = setup_repo();

        let manager = StashManager::new();
        let result = manager.stash_diff(&repo, 99);
        assert!(result.is_err());
    }
}
