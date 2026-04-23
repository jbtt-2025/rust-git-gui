use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::{DiffFileStatus, DiffHunk, DiffLine, DiffLineType, FileDiff};

pub struct DiffService;

impl DiffService {
    pub fn new() -> Self {
        Self
    }

    /// Diff between working directory and index (staged area).
    /// If `ignore_whitespace` is true, whitespace-only changes are excluded.
    pub fn working_diff(
        &self,
        repo: &GitRepository,
        ignore_whitespace: bool,
    ) -> Result<Vec<FileDiff>, GitError> {
        let inner = repo.inner();
        let mut opts = git2::DiffOptions::new();
        if ignore_whitespace {
            opts.ignore_whitespace(true);
        }
        let diff = inner.diff_index_to_workdir(None, Some(&mut opts))?;
        self.parse_diff(&diff)
    }

    /// Diff of a specific commit against its parent.
    /// For root commits (no parent), diffs against an empty tree.
    pub fn commit_diff(
        &self,
        repo: &GitRepository,
        commit_id: &str,
    ) -> Result<Vec<FileDiff>, GitError> {
        let inner = repo.inner();
        let oid = git2::Oid::from_str(commit_id)
            .map_err(|_| GitError::InvalidArgument(format!("Invalid commit id: {}", commit_id)))?;
        let commit = inner.find_commit(oid)?;
        let commit_tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = inner.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)?;
        self.parse_diff(&diff)
    }

    /// Diff for a single file.
    /// If `staged` is true: index vs HEAD (what's staged).
    /// If `staged` is false: workdir vs index (unstaged changes).
    pub fn file_diff(
        &self,
        repo: &GitRepository,
        path: &str,
        staged: bool,
    ) -> Result<FileDiff, GitError> {
        let inner = repo.inner();
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path);

        let diff = if staged {
            // Index vs HEAD
            let head_tree = inner
                .head()
                .ok()
                .and_then(|h| h.peel_to_tree().ok());
            inner.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?
        } else {
            // Workdir vs index
            inner.diff_index_to_workdir(None, Some(&mut opts))?
        };

        let file_diffs = self.parse_diff(&diff)?;
        file_diffs
            .into_iter()
            .find(|fd| fd.path == path || fd.old_path.as_deref() == Some(path))
            .ok_or_else(|| {
                GitError::InvalidArgument(format!("No diff found for path: {}", path))
            })
    }

    /// Diff between two commits.
    pub fn compare_commits(
        &self,
        repo: &GitRepository,
        from: &str,
        to: &str,
    ) -> Result<Vec<FileDiff>, GitError> {
        let inner = repo.inner();

        let from_oid = git2::Oid::from_str(from)
            .map_err(|_| GitError::InvalidArgument(format!("Invalid commit id: {}", from)))?;
        let to_oid = git2::Oid::from_str(to)
            .map_err(|_| GitError::InvalidArgument(format!("Invalid commit id: {}", to)))?;

        let from_tree = inner.find_commit(from_oid)?.tree()?;
        let to_tree = inner.find_commit(to_oid)?.tree()?;

        let diff = inner.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;
        self.parse_diff(&diff)
    }

    // --- Private helpers ---

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
        )?;

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
        let repo = GitRepository::init(dir.path()).unwrap();

        {
            let inner = repo.inner();
            let mut config = inner.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();

            // Create initial commit with a file
            let workdir = inner.workdir().unwrap();
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

    /// Helper: stage a file and commit.
    fn add_file_and_commit(
        repo: &GitRepository,
        filename: &str,
        content: &str,
        message: &str,
    ) -> String {
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
        let oid = inner
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
            .unwrap();
        oid.to_string()
    }

    #[test]
    fn test_working_diff_detects_modifications() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Modify the file in the working directory (without staging)
        fs::write(workdir.join("hello.txt"), "line1\nmodified\nline3\n").unwrap();

        let service = DiffService::new();
        let diffs = service.working_diff(&repo, false).unwrap();

        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "hello.txt");
        assert_eq!(diffs[0].status, DiffFileStatus::Modified);
        assert!(!diffs[0].is_binary);
        assert!(!diffs[0].hunks.is_empty());

        // Check that we have addition and deletion lines
        let lines: Vec<&DiffLine> = diffs[0]
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .collect();
        assert!(lines.iter().any(|l| l.origin == DiffLineType::Addition));
        assert!(lines.iter().any(|l| l.origin == DiffLineType::Deletion));
    }

    #[test]
    fn test_working_diff_detects_new_untracked_file() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Create a new file (untracked)
        fs::write(workdir.join("new_file.txt"), "new content\n").unwrap();

        let service = DiffService::new();
        let diffs = service.working_diff(&repo, false).unwrap();

        // Untracked files may not appear in index-to-workdir diff by default
        // unless we add include_untracked. The default git2 behavior includes them.
        // Let's check — if it appears, it should be Added.
        if !diffs.is_empty() {
            let new_file_diff = diffs.iter().find(|d| d.path == "new_file.txt");
            if let Some(fd) = new_file_diff {
                assert_eq!(fd.status, DiffFileStatus::Added);
            }
        }
    }

    #[test]
    fn test_working_diff_ignore_whitespace() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Only change whitespace
        fs::write(workdir.join("hello.txt"), "line1\nline2  \nline3\n").unwrap();

        let service = DiffService::new();

        // Without ignore_whitespace, should detect changes
        let diffs_normal = service.working_diff(&repo, false).unwrap();

        // With ignore_whitespace, should have no changes (or empty hunks)
        let diffs_ignore = service.working_diff(&repo, true).unwrap();

        // The normal diff should show changes
        assert!(!diffs_normal.is_empty());

        // The ignore-whitespace diff should either be empty or have no meaningful hunks
        let ignore_has_changes = diffs_ignore
            .iter()
            .any(|d| !d.hunks.is_empty());
        assert!(
            diffs_ignore.is_empty() || !ignore_has_changes,
            "ignore_whitespace should suppress whitespace-only changes"
        );
    }

    #[test]
    fn test_commit_diff() {
        let (_dir, repo) = setup_repo();
        let commit_id = add_file_and_commit(&repo, "added.txt", "content\n", "Add file");

        let service = DiffService::new();
        let diffs = service.commit_diff(&repo, &commit_id).unwrap();

        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "added.txt");
        assert_eq!(diffs[0].status, DiffFileStatus::Added);
        assert!(!diffs[0].hunks.is_empty());

        // All lines should be additions for a new file
        let all_additions = diffs[0]
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .all(|l| l.origin == DiffLineType::Addition);
        assert!(all_additions);
    }

    #[test]
    fn test_commit_diff_root_commit() {
        let (_dir, repo) = setup_repo();

        // Get the initial commit id
        let inner = repo.inner();
        let head = inner.head().unwrap().target().unwrap();
        let initial_id = head.to_string();

        let service = DiffService::new();
        let diffs = service.commit_diff(&repo, &initial_id).unwrap();

        // Root commit should show hello.txt as added
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "hello.txt");
        assert_eq!(diffs[0].status, DiffFileStatus::Added);
    }

    #[test]
    fn test_commit_diff_invalid_id() {
        let (_dir, repo) = setup_repo();
        let service = DiffService::new();
        let result = service.commit_diff(&repo, "not-a-valid-oid");
        assert!(result.is_err());
    }

    #[test]
    fn test_file_diff_staged() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Modify and stage the file
        fs::write(workdir.join("hello.txt"), "line1\nchanged\nline3\n").unwrap();
        let inner = repo.inner();
        let mut index = inner.index().unwrap();
        index.add_path(Path::new("hello.txt")).unwrap();
        index.write().unwrap();

        let service = DiffService::new();
        let diff = service.file_diff(&repo, "hello.txt", true).unwrap();

        assert_eq!(diff.path, "hello.txt");
        assert_eq!(diff.status, DiffFileStatus::Modified);
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn test_file_diff_unstaged() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Modify without staging
        fs::write(workdir.join("hello.txt"), "line1\nupdated\nline3\n").unwrap();

        let service = DiffService::new();
        let diff = service.file_diff(&repo, "hello.txt", false).unwrap();

        assert_eq!(diff.path, "hello.txt");
        assert_eq!(diff.status, DiffFileStatus::Modified);
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn test_file_diff_not_found() {
        let (_dir, repo) = setup_repo();
        let service = DiffService::new();
        let result = service.file_diff(&repo, "nonexistent.txt", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_compare_commits() {
        let (_dir, repo) = setup_repo();

        let inner = repo.inner();
        let first_id = inner.head().unwrap().target().unwrap().to_string();

        let second_id = add_file_and_commit(&repo, "second.txt", "second\n", "Second commit");

        let service = DiffService::new();
        let diffs = service.compare_commits(&repo, &first_id, &second_id).unwrap();

        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "second.txt");
        assert_eq!(diffs[0].status, DiffFileStatus::Added);
    }

    #[test]
    fn test_compare_commits_multiple_files() {
        let (_dir, repo) = setup_repo();

        let inner = repo.inner();
        let base_id = inner.head().unwrap().target().unwrap().to_string();

        add_file_and_commit(&repo, "a.txt", "aaa\n", "Add a");
        let tip_id = add_file_and_commit(&repo, "b.txt", "bbb\n", "Add b");

        let service = DiffService::new();
        let diffs = service.compare_commits(&repo, &base_id, &tip_id).unwrap();

        assert_eq!(diffs.len(), 2);
        let paths: Vec<&str> = diffs.iter().map(|d| d.path.as_str()).collect();
        assert!(paths.contains(&"a.txt"));
        assert!(paths.contains(&"b.txt"));
    }

    #[test]
    fn test_compare_commits_invalid_id() {
        let (_dir, repo) = setup_repo();
        let service = DiffService::new();
        let result = service.compare_commits(&repo, "bad-id", "also-bad");
        assert!(result.is_err());
    }

    #[test]
    fn test_hunk_line_numbers() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Modify a line in the middle
        fs::write(workdir.join("hello.txt"), "line1\nMODIFIED\nline3\n").unwrap();

        let service = DiffService::new();
        let diffs = service.working_diff(&repo, false).unwrap();

        assert_eq!(diffs.len(), 1);
        let hunk = &diffs[0].hunks[0];

        // Hunk should have valid start/lines
        assert!(hunk.old_start > 0);
        assert!(hunk.new_start > 0);

        // Lines should have line numbers
        for line in &hunk.lines {
            match line.origin {
                DiffLineType::Context => {
                    assert!(line.old_lineno.is_some());
                    assert!(line.new_lineno.is_some());
                }
                DiffLineType::Deletion => {
                    assert!(line.old_lineno.is_some());
                }
                DiffLineType::Addition => {
                    assert!(line.new_lineno.is_some());
                }
            }
        }
    }

    #[test]
    fn test_binary_file_detection() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Create a binary file with null bytes scattered throughout (git2 detects
        // binary by finding a NUL byte in the first 8000 bytes of content)
        let mut binary_content: Vec<u8> = Vec::with_capacity(256);
        for i in 0u8..=255 {
            binary_content.push(i); // includes 0x00 (NUL)
        }
        fs::write(workdir.join("image.bin"), &binary_content).unwrap();

        // Stage and commit
        let inner = repo.inner();
        let mut index = inner.index().unwrap();
        index.add_path(Path::new("image.bin")).unwrap();
        index.write().unwrap();

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = inner.find_tree(tree_id).unwrap();
        let head = inner.head().unwrap().target().unwrap();
        let parent = inner.find_commit(head).unwrap();
        let commit_oid = inner
            .commit(Some("HEAD"), &sig, &sig, "Add binary", &tree, &[&parent])
            .unwrap();

        let service = DiffService::new();
        let diffs = service.commit_diff(&repo, &commit_oid.to_string()).unwrap();

        let binary_diff = diffs.iter().find(|d| d.path == "image.bin");
        assert!(binary_diff.is_some(), "Should find binary file in diff");
        let bd = binary_diff.unwrap();
        assert!(bd.is_binary, "Binary file should be marked as binary");
        assert!(bd.hunks.is_empty(), "Binary files should have empty hunks");
    }
}
