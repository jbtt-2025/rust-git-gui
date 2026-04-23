use std::path::Path;

use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::{FileStatus, FileStatusType, LineRange};

pub struct StagingService;

impl StagingService {
    pub fn new() -> Self {
        Self
    }

    /// Add files to the index (stage).
    pub fn stage_files(&self, repo: &GitRepository, paths: &[&str]) -> Result<(), GitError> {
        let inner = repo.inner();
        let mut index = inner.index()?;
        for path in paths {
            index.add_path(Path::new(path))?;
        }
        index.write()?;
        Ok(())
    }

    /// Remove files from the index, resetting them to HEAD state (unstage).
    pub fn unstage_files(&self, repo: &GitRepository, paths: &[&str]) -> Result<(), GitError> {
        let inner = repo.inner();
        let head_commit = inner.head()?.peel_to_commit()?;
        let head_tree = head_commit.tree()?;
        let mut index = inner.index()?;

        for path in paths {
            match head_tree.get_path(Path::new(path)) {
                Ok(entry) => {
                    // File exists in HEAD — reset index entry to HEAD version
                    let mut idx_entry = git2::IndexEntry {
                        ctime: git2::IndexTime::new(0, 0),
                        mtime: git2::IndexTime::new(0, 0),
                        dev: 0,
                        ino: 0,
                        mode: entry.filemode() as u32,
                        uid: 0,
                        gid: 0,
                        file_size: 0,
                        id: entry.id(),
                        flags: 0,
                        flags_extended: 0,
                        path: path.as_bytes().to_vec(),
                    };
                    // Retrieve the blob to get the correct file_size
                    if let Ok(blob) = inner.find_blob(entry.id()) {
                        idx_entry.file_size = blob.content().len() as u32;
                    }
                    index.add(&idx_entry)?;
                }
                Err(_) => {
                    // File doesn't exist in HEAD — remove from index (was newly added)
                    index.remove_path(Path::new(path))?;
                }
            }
        }
        index.write()?;
        Ok(())
    }

    /// Stage only specific line ranges from a file's working directory changes.
    ///
    /// Algorithm:
    /// 1. Get the current index content for the file
    /// 2. Get the working directory content for the file
    /// 3. Compute a diff between index and workdir
    /// 4. For each changed line, if it falls within any of the specified ranges
    ///    (using the new/workdir line numbers), include the workdir version;
    ///    otherwise keep the index version
    /// 5. Write the resulting content as a blob and update the index entry
    pub fn stage_lines(
        &self,
        repo: &GitRepository,
        path: &str,
        line_ranges: &[LineRange],
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        let workdir = inner
            .workdir()
            .ok_or_else(|| GitError::InvalidArgument("Bare repository".into()))?;

        // Read working directory content
        let workdir_path = workdir.join(path);
        let workdir_content = std::fs::read_to_string(&workdir_path)
            .map_err(|e| GitError::Io(format!("Failed to read {}: {}", path, e)))?;

        // Read current index content (may be empty for new files)
        let index_content = self.get_index_content(inner, path)?;

        // Build new index content by selectively applying workdir changes
        let new_content =
            self.apply_line_ranges(&index_content, &workdir_content, line_ranges, true);

        // Write the new content as a blob and update the index
        self.write_blob_to_index(inner, path, new_content.as_bytes())?;
        Ok(())
    }

    /// Unstage specific line ranges — revert those lines in the index back to HEAD.
    ///
    /// Algorithm:
    /// 1. Get the HEAD content for the file
    /// 2. Get the current index content
    /// 3. Compute a diff between HEAD and index
    /// 4. For each changed line in the index, if it falls within any of the specified
    ///    ranges (using the new/index line numbers), revert to HEAD version;
    ///    otherwise keep the index version
    /// 5. Write the resulting content as a blob and update the index entry
    pub fn unstage_lines(
        &self,
        repo: &GitRepository,
        path: &str,
        line_ranges: &[LineRange],
    ) -> Result<(), GitError> {
        let inner = repo.inner();

        // Read HEAD content
        let head_content = self.get_head_content(inner, path)?;

        // Read current index content
        let index_content = self.get_index_content(inner, path)?;

        // Build new index content by selectively reverting to HEAD
        let new_content =
            self.apply_line_ranges(&head_content, &index_content, line_ranges, false);

        // Write the new content as a blob and update the index
        self.write_blob_to_index(inner, path, new_content.as_bytes())?;
        Ok(())
    }

    /// Discard working directory changes for specific lines,
    /// restoring those lines to the index version.
    pub fn discard_lines(
        &self,
        repo: &GitRepository,
        path: &str,
        line_ranges: &[LineRange],
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        let workdir = inner
            .workdir()
            .ok_or_else(|| GitError::InvalidArgument("Bare repository".into()))?;

        // Read index content (the "good" version)
        let index_content = self.get_index_content(inner, path)?;

        // Read working directory content
        let workdir_path = workdir.join(path);
        let workdir_content = std::fs::read_to_string(&workdir_path)
            .map_err(|e| GitError::Io(format!("Failed to read {}: {}", path, e)))?;

        // Build new workdir content: for lines in ranges, use index version;
        // for lines outside ranges, keep workdir version.
        // This is the reverse of stage_lines: we want to revert selected lines.
        let new_content =
            self.apply_line_ranges(&index_content, &workdir_content, line_ranges, false);

        // Write back to the working directory
        std::fs::write(&workdir_path, new_content)?;
        Ok(())
    }

    /// Return the list of file statuses in the repository.
    pub fn status(&self, repo: &GitRepository) -> Result<Vec<FileStatus>, GitError> {
        let inner = repo.inner();
        let statuses = inner.statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .recurse_untracked_dirs(true),
        ))?;

        let mut result = Vec::new();
        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let st = entry.status();

            let status_type = if st.contains(git2::Status::CONFLICTED) {
                FileStatusType::Conflict
            } else if st.contains(git2::Status::INDEX_NEW)
                || st.contains(git2::Status::INDEX_MODIFIED)
                || st.contains(git2::Status::INDEX_DELETED)
                || st.contains(git2::Status::INDEX_RENAMED)
            {
                FileStatusType::Staged
            } else if st.contains(git2::Status::WT_DELETED)
                || st.contains(git2::Status::INDEX_DELETED)
            {
                FileStatusType::Deleted
            } else if st.contains(git2::Status::INDEX_RENAMED)
                || st.contains(git2::Status::WT_RENAMED)
            {
                FileStatusType::Renamed
            } else if st.contains(git2::Status::WT_MODIFIED) {
                FileStatusType::Modified
            } else if st.contains(git2::Status::WT_NEW) {
                FileStatusType::Untracked
            } else {
                continue; // Skip statuses we don't map
            };

            result.push(FileStatus {
                path,
                status: status_type,
            });
        }
        Ok(result)
    }

    // === Private helpers ===

    /// Get file content from HEAD tree. Returns empty string if file doesn't exist in HEAD.
    fn get_head_content(&self, repo: &git2::Repository, path: &str) -> Result<String, GitError> {
        let head = match repo.head() {
            Ok(h) => h,
            Err(_) => return Ok(String::new()), // No HEAD (empty repo)
        };
        let tree = head.peel_to_tree()?;
        match tree.get_path(Path::new(path)) {
            Ok(entry) => {
                let blob = repo.find_blob(entry.id())?;
                Ok(String::from_utf8_lossy(blob.content()).to_string())
            }
            Err(_) => Ok(String::new()), // File doesn't exist in HEAD
        }
    }

    /// Get file content from the index. Returns empty string if file doesn't exist in index.
    fn get_index_content(&self, repo: &git2::Repository, path: &str) -> Result<String, GitError> {
        let index = repo.index()?;
        match index.get_path(Path::new(path), 0) {
            Some(entry) => {
                let blob = repo.find_blob(entry.id)?;
                Ok(String::from_utf8_lossy(blob.content()).to_string())
            }
            None => Ok(String::new()),
        }
    }

    /// Write content as a blob to the index for the given path.
    fn write_blob_to_index(
        &self,
        repo: &git2::Repository,
        path: &str,
        content: &[u8],
    ) -> Result<(), GitError> {
        let blob_oid = repo.blob(content)?;
        let mut index = repo.index()?;

        // Determine the file mode — use existing index entry mode or default to regular file
        let mode = index
            .get_path(Path::new(path), 0)
            .map(|e| e.mode)
            .unwrap_or(0o100644);

        let entry = git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode,
            uid: 0,
            gid: 0,
            file_size: content.len() as u32,
            id: blob_oid,
            flags: 0,
            flags_extended: 0,
            path: path.as_bytes().to_vec(),
        };
        index.add(&entry)?;
        index.write()?;
        Ok(())
    }

    /// Core line-level operation: given a "base" content and a "changed" content,
    /// produce new content by selectively applying or reverting changes.
    ///
    /// When `stage` is true (stage_lines):
    ///   - `base` = index content, `changed` = workdir content
    ///   - For lines in the diff that fall within `line_ranges` (by workdir/new line number),
    ///     use the workdir version; otherwise keep the index version.
    ///
    /// When `stage` is false (unstage_lines / discard_lines):
    ///   - `base` = HEAD/index content (the "good" version), `changed` = index/workdir content
    ///   - For lines in the diff that fall within `line_ranges` (by changed line number),
    ///     revert to the base version; otherwise keep the changed version.
    ///
    /// Uses a simple line-by-line diff approach.
    fn apply_line_ranges(
        &self,
        base: &str,
        changed: &str,
        line_ranges: &[LineRange],
        stage: bool,
    ) -> String {
        let base_lines: Vec<&str> = if base.is_empty() {
            Vec::new()
        } else {
            base.lines().collect()
        };
        let changed_lines: Vec<&str> = if changed.is_empty() {
            Vec::new()
        } else {
            changed.lines().collect()
        };

        // Build a simple LCS-based diff
        let ops = self.compute_diff_ops(&base_lines, &changed_lines);

        let mut result = Vec::new();

        for op in &ops {
            match op {
                DiffOp::Equal(_, text) => {
                    // Unchanged line — always include
                    result.push(*text);
                }
                DiffOp::Insert(changed_ln, text) => {
                    // Line exists only in changed content
                    if stage {
                        // stage_lines: include if in range
                        if self.line_in_ranges(*changed_ln, line_ranges) {
                            result.push(*text);
                        }
                        // else: skip (don't stage this addition)
                    } else {
                        // unstage/discard: revert if in range (= remove the line)
                        if !self.line_in_ranges(*changed_ln, line_ranges) {
                            result.push(*text);
                        }
                        // else: skip (revert = remove the addition)
                    }
                }
                DiffOp::Delete(_base_ln, text) => {
                    // Line exists only in base content
                    if stage {
                        // stage_lines: if the corresponding changed line is in range,
                        // we want to apply the deletion (= don't include base line).
                        // But deletions don't have a changed line number directly.
                        // We use the "next changed line number" context.
                        // For deletions, we check if the deletion's position in the
                        // changed content falls within the range.
                        // We track this via the changed_context_ln stored in Delete.
                        // Actually, let's use the base line number for deletions.
                        // The UI shows deletions with old_lineno, so we use that.
                        // But the line_ranges refer to the diff view line numbers.
                        // For stage_lines, ranges refer to workdir line numbers for additions
                        // and base line numbers for deletions.
                        // Let's keep it simple: for deletions during staging, we check
                        // if the base line number is in range.
                        if self.line_in_ranges(*_base_ln, line_ranges) {
                            // Stage the deletion = don't include the base line
                        } else {
                            // Don't stage = keep the base line
                            result.push(*text);
                        }
                    } else {
                        // unstage/discard: revert if in range (= restore the base line)
                        if self.line_in_ranges(*_base_ln, line_ranges) {
                            result.push(*text);
                        }
                        // else: keep it deleted
                    }
                }
            }
        }

        // Preserve trailing newline if the changed content had one
        let trailing = if stage {
            changed.ends_with('\n')
        } else {
            // For unstage/discard, preserve trailing newline from whichever is appropriate
            if result.is_empty() {
                base.ends_with('\n')
            } else {
                changed.ends_with('\n')
            }
        };

        let mut output = result.join("\n");
        if trailing && !output.is_empty() {
            output.push('\n');
        }
        output
    }

    /// Check if a 1-based line number falls within any of the given ranges (inclusive).
    fn line_in_ranges(&self, line: u32, ranges: &[LineRange]) -> bool {
        ranges.iter().any(|r| line >= r.start && line <= r.end)
    }

    /// Compute diff operations between base and changed line arrays.
    /// Returns a sequence of Equal/Insert/Delete operations with 1-based line numbers.
    fn compute_diff_ops<'a>(
        &self,
        base: &[&'a str],
        changed: &[&'a str],
    ) -> Vec<DiffOp<'a>> {
        let m = base.len();
        let n = changed.len();

        // Build LCS table
        let mut dp = vec![vec![0u32; n + 1]; m + 1];
        for i in 1..=m {
            for j in 1..=n {
                if base[i - 1] == changed[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        // Backtrack to produce diff ops
        let mut ops = Vec::new();
        let mut i = m;
        let mut j = n;

        while i > 0 || j > 0 {
            if i > 0 && j > 0 && base[i - 1] == changed[j - 1] {
                ops.push(DiffOp::Equal(j as u32, base[i - 1]));
                i -= 1;
                j -= 1;
            } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
                ops.push(DiffOp::Insert(j as u32, changed[j - 1]));
                j -= 1;
            } else {
                ops.push(DiffOp::Delete(i as u32, base[i - 1]));
                i -= 1;
            }
        }

        ops.reverse();
        ops
    }
}

/// Internal diff operation for line-level staging.
#[derive(Debug)]
#[allow(dead_code)]
enum DiffOp<'a> {
    /// Line is the same in both. (changed_line_1based, text)
    Equal(u32, &'a str),
    /// Line exists only in changed. (changed_line_1based, text)
    Insert(u32, &'a str),
    /// Line exists only in base. (base_line_1based, text)
    Delete(u32, &'a str),
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

    #[test]
    fn test_stage_files() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Create a new file in the working directory
        fs::write(workdir.join("new.txt"), "new content\n").unwrap();

        let service = StagingService::new();
        service.stage_files(&repo, &["new.txt"]).unwrap();

        // Verify the file is in the index
        let inner = repo.inner();
        let index = inner.index().unwrap();
        assert!(index.get_path(Path::new("new.txt"), 0).is_some());
    }

    #[test]
    fn test_unstage_files() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Modify and stage a file
        fs::write(workdir.join("hello.txt"), "modified\n").unwrap();
        let service = StagingService::new();
        service.stage_files(&repo, &["hello.txt"]).unwrap();

        // Verify it's staged (different from HEAD)
        let inner = repo.inner();
        {
            let index = inner.index().unwrap();
            let entry = index.get_path(Path::new("hello.txt"), 0).unwrap();
            let head_tree = inner.head().unwrap().peel_to_tree().unwrap();
            let head_entry = head_tree.get_path(Path::new("hello.txt")).unwrap();
            assert_ne!(entry.id, head_entry.id(), "File should be staged with different content");
        }

        // Unstage
        service.unstage_files(&repo, &["hello.txt"]).unwrap();

        // Verify index matches HEAD again
        {
            let index = inner.index().unwrap();
            let entry = index.get_path(Path::new("hello.txt"), 0).unwrap();
            let head_tree = inner.head().unwrap().peel_to_tree().unwrap();
            let head_entry = head_tree.get_path(Path::new("hello.txt")).unwrap();
            assert_eq!(entry.id, head_entry.id(), "File should be unstaged back to HEAD");
        }
    }

    #[test]
    fn test_unstage_new_file() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Create and stage a new file
        fs::write(workdir.join("brand_new.txt"), "content\n").unwrap();
        let service = StagingService::new();
        service.stage_files(&repo, &["brand_new.txt"]).unwrap();

        // Verify it's in the index
        let inner = repo.inner();
        assert!(inner.index().unwrap().get_path(Path::new("brand_new.txt"), 0).is_some());

        // Unstage — should remove from index since it doesn't exist in HEAD
        service.unstage_files(&repo, &["brand_new.txt"]).unwrap();
        let index = inner.index().unwrap();
        assert!(index.get_path(Path::new("brand_new.txt"), 0).is_none());
    }

    #[test]
    fn test_stage_lines_partial() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Modify multiple lines in working directory
        fs::write(
            workdir.join("hello.txt"),
            "line1_modified\nline2\nline3_modified\n",
        )
        .unwrap();

        let service = StagingService::new();

        // Stage only line 1 (the first modification)
        service
            .stage_lines(&repo, "hello.txt", &[LineRange { start: 1, end: 1 }])
            .unwrap();

        // Read the index content to verify
        let inner = repo.inner();
        let index = inner.index().unwrap();
        let entry = index.get_path(Path::new("hello.txt"), 0).unwrap();
        let blob = inner.find_blob(entry.id).unwrap();
        let staged_content = std::str::from_utf8(blob.content()).unwrap();

        // The staged content should have line1_modified but NOT line3_modified
        assert!(
            staged_content.contains("line1_modified"),
            "Line 1 should be staged. Got: {}",
            staged_content
        );
        assert!(
            staged_content.contains("line2"),
            "Line 2 should be unchanged. Got: {}",
            staged_content
        );
        assert!(
            !staged_content.contains("line3_modified"),
            "Line 3 should NOT be staged. Got: {}",
            staged_content
        );
        assert!(
            staged_content.contains("line3"),
            "Line 3 should remain as original. Got: {}",
            staged_content
        );
    }

    #[test]
    fn test_unstage_lines() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Modify and fully stage the file
        fs::write(
            workdir.join("hello.txt"),
            "line1_modified\nline2_modified\nline3\n",
        )
        .unwrap();

        let service = StagingService::new();
        service.stage_files(&repo, &["hello.txt"]).unwrap();

        // Now unstage only line 1
        service
            .unstage_lines(&repo, "hello.txt", &[LineRange { start: 1, end: 1 }])
            .unwrap();

        // Read the index content
        let inner = repo.inner();
        let index = inner.index().unwrap();
        let entry = index.get_path(Path::new("hello.txt"), 0).unwrap();
        let blob = inner.find_blob(entry.id).unwrap();
        let staged_content = std::str::from_utf8(blob.content()).unwrap();

        // line1 should be reverted to HEAD (line1), line2 should remain modified
        assert!(
            !staged_content.contains("line1_modified"),
            "Line 1 should be unstaged. Got: {}",
            staged_content
        );
        assert!(
            staged_content.contains("line2_modified"),
            "Line 2 should remain staged. Got: {}",
            staged_content
        );
    }

    #[test]
    fn test_discard_lines() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Modify the file in working directory
        fs::write(
            workdir.join("hello.txt"),
            "line1_modified\nline2_modified\nline3\n",
        )
        .unwrap();

        let service = StagingService::new();

        // Discard only line 2 changes
        service
            .discard_lines(&repo, "hello.txt", &[LineRange { start: 2, end: 2 }])
            .unwrap();

        // Read the working directory file
        let content = fs::read_to_string(workdir.join("hello.txt")).unwrap();

        // line1 should still be modified, line2 should be restored to index version
        assert!(
            content.contains("line1_modified"),
            "Line 1 should still be modified. Got: {}",
            content
        );
        assert!(
            !content.contains("line2_modified"),
            "Line 2 should be discarded. Got: {}",
            content
        );
        assert!(
            content.contains("line2"),
            "Line 2 should be restored to original. Got: {}",
            content
        );
    }

    #[test]
    fn test_status() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Create an untracked file
        fs::write(workdir.join("untracked.txt"), "untracked\n").unwrap();

        // Modify an existing file
        fs::write(workdir.join("hello.txt"), "modified content\n").unwrap();

        let service = StagingService::new();
        let statuses = service.status(&repo).unwrap();

        // Should have at least 2 entries
        assert!(statuses.len() >= 2, "Expected at least 2 status entries, got {}", statuses.len());

        let untracked = statuses.iter().find(|s| s.path == "untracked.txt");
        assert!(untracked.is_some(), "Should find untracked.txt");
        assert_eq!(untracked.unwrap().status, FileStatusType::Untracked);

        let modified = statuses.iter().find(|s| s.path == "hello.txt");
        assert!(modified.is_some(), "Should find hello.txt");
        assert_eq!(modified.unwrap().status, FileStatusType::Modified);
    }

    #[test]
    fn test_status_staged_file() {
        let (dir, repo) = setup_repo();
        let workdir = dir.path();

        // Modify and stage a file
        fs::write(workdir.join("hello.txt"), "staged content\n").unwrap();
        let service = StagingService::new();
        service.stage_files(&repo, &["hello.txt"]).unwrap();

        let statuses = service.status(&repo).unwrap();
        let staged = statuses.iter().find(|s| s.path == "hello.txt");
        assert!(staged.is_some(), "Should find hello.txt");
        assert_eq!(staged.unwrap().status, FileStatusType::Staged);
    }

    use proptest::prelude::*;
    use crate::modules::diff_service::DiffService;
    use crate::models::DiffLineType;

    /// Generate base file content: 3-10 lines of simple text.
    fn arb_base_content() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec("[a-z]{3,10}", 3..=10)
    }

    /// Convert a set of line numbers into contiguous LineRange values.
    fn lines_to_ranges(lines: &[u32]) -> Vec<LineRange> {
        if lines.is_empty() {
            return Vec::new();
        }
        let mut sorted: Vec<u32> = lines.to_vec();
        sorted.sort();
        sorted.dedup();
        let mut ranges = Vec::new();
        let mut start = sorted[0];
        let mut end = sorted[0];
        for &ln in &sorted[1..] {
            if ln == end + 1 {
                end = ln;
            } else {
                ranges.push(LineRange { start, end });
                start = ln;
                end = ln;
            }
        }
        ranges.push(LineRange { start, end });
        ranges
    }

    /// Helper: create a repo with base content committed, then write modified content.
    /// Uses scoped borrows to avoid lifetime issues with GitRepository.
    fn setup_staging_test(
        base_lines: &[String],
        modified_lines: &[String],
    ) -> (TempDir, GitRepository) {
        let dir = TempDir::new().unwrap();
        let repo = GitRepository::init(dir.path()).unwrap();

        {
            let inner = repo.inner();
            let mut config = inner.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();

            let workdir = inner.workdir().unwrap();
            let base_content = base_lines.join("\n") + "\n";
            fs::write(workdir.join("test.txt"), &base_content).unwrap();

            let mut index = inner.index().unwrap();
            index.add_path(Path::new("test.txt")).unwrap();
            index.write().unwrap();

            let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = inner.find_tree(tree_id).unwrap();
            inner
                .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        // Write modified content to working directory
        let modified_content = modified_lines.join("\n") + "\n";
        fs::write(dir.path().join("test.txt"), &modified_content).unwrap();

        (dir, repo)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 4.3, 4.4**
        ///
        /// Feature: rust-git-gui-client, Property 8: 行级暂存精确性 (Line-level staging precision)
        ///
        /// For any file content (base) and modified content (changed), and any subset
        /// of changed line numbers selected as a range:
        /// - After staging only the selected lines, the staged diff SHALL contain
        ///   only the selected line changes.
        /// - The unstaged diff SHALL still contain the non-selected line changes.
        #[test]
        fn prop_line_staging_precision(base in arb_base_content()) {
            // Deterministically modify every other line to create a diff
            let mut modified = base.clone();
            let mut changed_indices: Vec<u32> = Vec::new();

            for i in 0..modified.len() {
                if i % 2 == 0 {
                    let new_val = format!("MODIFIED_{}", modified[i]);
                    if new_val != modified[i] {
                        modified[i] = new_val;
                        changed_indices.push((i + 1) as u32);
                    }
                }
            }

            // Skip if no changes were made
            if changed_indices.is_empty() {
                return Ok(());
            }

            // Select roughly half the changed lines to stage
            let selected: Vec<u32> = changed_indices
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % 2 == 0)
                .map(|(_, &ln)| ln)
                .collect();
            let not_selected: Vec<u32> = changed_indices
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % 2 != 0)
                .map(|(_, &ln)| ln)
                .collect();

            let ranges = lines_to_ranges(&selected);
            if ranges.is_empty() {
                return Ok(());
            }

            let (_dir, repo) = setup_staging_test(&base, &modified);

            let staging = StagingService::new();
            let diff_svc = DiffService::new();

            // Stage selected lines
            staging.stage_lines(&repo, "test.txt", &ranges).unwrap();

            // Check staged diff: should contain only selected line changes
            if !selected.is_empty() {
                let staged_diff = diff_svc.file_diff(&repo, "test.txt", true).unwrap();
                let staged_additions: Vec<String> = staged_diff
                    .hunks
                    .iter()
                    .flat_map(|h| &h.lines)
                    .filter(|l| l.origin == DiffLineType::Addition)
                    .map(|l| l.content.trim_end().to_string())
                    .collect();

                // Every staged addition should correspond to a selected modified line
                for add_line in &staged_additions {
                    let is_selected_change = selected.iter().any(|&ln| {
                        let idx = (ln - 1) as usize;
                        idx < modified.len() && modified[idx].trim() == add_line.trim()
                    });
                    prop_assert!(
                        is_selected_change,
                        "Staged addition '{}' should be from selected lines {:?}",
                        add_line,
                        selected
                    );
                }
            }

            // Check unstaged diff: should contain non-selected line changes
            if !not_selected.is_empty() {
                let unstaged_diff = diff_svc.file_diff(&repo, "test.txt", false).unwrap();
                let unstaged_additions: Vec<String> = unstaged_diff
                    .hunks
                    .iter()
                    .flat_map(|h| &h.lines)
                    .filter(|l| l.origin == DiffLineType::Addition)
                    .map(|l| l.content.trim_end().to_string())
                    .collect();

                // Every non-selected modified line should appear in unstaged additions
                for &ln in &not_selected {
                    let idx = (ln - 1) as usize;
                    if idx < modified.len() {
                        let expected = modified[idx].trim();
                        let found = unstaged_additions
                            .iter()
                            .any(|a| a.trim() == expected);
                        prop_assert!(
                            found,
                            "Non-selected line {} ('{}') should remain in unstaged diff. Unstaged additions: {:?}",
                            ln,
                            expected,
                            unstaged_additions
                        );
                    }
                }
            }
        }

        /// **Validates: Requirements 4.5**
        ///
        /// Feature: rust-git-gui-client, Property 9: 暂存/取消暂存往返一致性 (Stage/Unstage roundtrip consistency)
        ///
        /// For any file content (base) and modified content (changed), and any line ranges:
        /// staging lines then unstaging the same lines restores the staging area
        /// to its pre-operation state (index blob OID is identical).
        #[test]
        fn prop_stage_unstage_roundtrip(base in arb_base_content()) {
            // Deterministically modify every other line to create a diff
            let mut modified = base.clone();
            let mut changed_indices: Vec<u32> = Vec::new();

            for i in 0..modified.len() {
                if i % 2 == 0 {
                    let new_val = format!("MODIFIED_{}", modified[i]);
                    if new_val != modified[i] {
                        modified[i] = new_val;
                        changed_indices.push((i + 1) as u32);
                    }
                }
            }

            // Skip if no changes were made
            if changed_indices.is_empty() {
                return Ok(());
            }

            // Select a subset of changed lines to stage/unstage
            let selected: Vec<u32> = changed_indices
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % 2 == 0)
                .map(|(_, &ln)| ln)
                .collect();

            let ranges = lines_to_ranges(&selected);
            if ranges.is_empty() {
                return Ok(());
            }

            let (_dir, repo) = setup_staging_test(&base, &modified);

            let staging = StagingService::new();

            // Record the initial index blob OID before any staging
            let initial_oid = {
                let inner = repo.inner();
                let index = inner.index().unwrap();
                let entry = index.get_path(Path::new("test.txt"), 0).unwrap();
                entry.id
            };

            // Stage the selected lines
            staging.stage_lines(&repo, "test.txt", &ranges).unwrap();

            // Unstage the same lines
            staging.unstage_lines(&repo, "test.txt", &ranges).unwrap();

            // Record the index blob OID after the roundtrip
            let final_oid = {
                let inner = repo.inner();
                let index = inner.index().unwrap();
                let entry = index.get_path(Path::new("test.txt"), 0).unwrap();
                entry.id
            };

            prop_assert_eq!(
                initial_oid,
                final_oid,
                "Index blob OID should be identical after stage+unstage roundtrip. \
                 Initial: {}, Final: {}",
                initial_oid,
                final_oid
            );
        }
    }
}
