use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::{BlameInfo, BlameLine};

pub struct BlameService;

impl BlameService {
    pub fn new() -> Self {
        Self
    }

    /// Perform git blame on the specified file, returning per-line attribution.
    pub fn blame(
        &self,
        repo: &GitRepository,
        path: &str,
    ) -> Result<BlameInfo, GitError> {
        let inner = repo.inner();
        let blame = inner
            .blame_file(std::path::Path::new(path), None)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        // Read file content to get line text.
        // Try the working directory first, fall back to HEAD blob.
        let lines_content = self.read_file_lines(repo, path)?;

        let mut blame_lines = Vec::new();
        for (i, content) in lines_content.iter().enumerate() {
            let line_no = (i + 1) as u32;
            if let Some(hunk) = blame.get_line(line_no as usize) {
                let sig = hunk.final_signature();
                blame_lines.push(BlameLine {
                    line_number: line_no,
                    content: content.clone(),
                    commit_id: hunk.final_commit_id().to_string(),
                    author: sig.name().unwrap_or("Unknown").to_string(),
                    date: sig.when().seconds(),
                    original_line: hunk.orig_start_line() as u32
                        + (line_no - hunk.final_start_line() as u32),
                });
            }
        }

        Ok(BlameInfo {
            path: path.to_string(),
            lines: blame_lines,
        })
    }

    /// Read file lines from the working directory, falling back to HEAD blob.
    fn read_file_lines(
        &self,
        repo: &GitRepository,
        path: &str,
    ) -> Result<Vec<String>, GitError> {
        // Try working directory first
        if let Some(workdir) = repo.workdir() {
            let file_path = workdir.join(path);
            if file_path.exists() {
                let content = std::fs::read_to_string(&file_path)?;
                return Ok(content.lines().map(|l| l.to_string()).collect());
            }
        }

        // Fall back to HEAD blob
        let inner = repo.inner();
        let head = inner.head().map_err(|e| GitError::Git2(e.message().to_string()))?;
        let tree = head
            .peel_to_tree()
            .map_err(|e| GitError::Git2(e.message().to_string()))?;
        let entry = tree
            .get_path(std::path::Path::new(path))
            .map_err(|e| GitError::Git2(e.message().to_string()))?;
        let blob = inner
            .find_blob(entry.id())
            .map_err(|e| GitError::Git2(e.message().to_string()))?;
        let content = std::str::from_utf8(blob.content())
            .map_err(|e| GitError::InvalidArgument(e.to_string()))?;
        Ok(content.lines().map(|l| l.to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_repo_with_commit(
        dir: &TempDir,
        filename: &str,
        content: &str,
        author: &str,
        message: &str,
    ) -> git2::Oid {
        let repo = git2::Repository::open(dir.path()).unwrap();
        let sig = git2::Signature::now(author, &format!("{}@test.com", author)).unwrap();

        // Write file
        std::fs::write(dir.path().join(filename), content).unwrap();

        // Stage
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(filename)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        // Commit
        let parents: Vec<git2::Commit> = if let Ok(head) = repo.head() {
            vec![head.peel_to_commit().unwrap()]
        } else {
            vec![]
        };
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .unwrap()
    }

    #[test]
    fn test_blame_single_commit() {
        let dir = TempDir::new().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        let content = "line one\nline two\nline three\n";
        let oid = init_repo_with_commit(&dir, "test.txt", content, "Alice", "initial commit");

        let repo = GitRepository::open(dir.path()).unwrap();
        let service = BlameService::new();
        let info = service.blame(&repo, "test.txt").unwrap();

        assert_eq!(info.path, "test.txt");
        assert_eq!(info.lines.len(), 3);

        let commit_id = oid.to_string();
        for line in &info.lines {
            assert_eq!(line.commit_id, commit_id);
            assert_eq!(line.author, "Alice");
        }

        assert_eq!(info.lines[0].line_number, 1);
        assert_eq!(info.lines[0].content, "line one");
        assert_eq!(info.lines[1].line_number, 2);
        assert_eq!(info.lines[1].content, "line two");
        assert_eq!(info.lines[2].line_number, 3);
        assert_eq!(info.lines[2].content, "line three");
    }

    #[test]
    fn test_blame_multiple_commits() {
        let dir = TempDir::new().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        let oid1 = init_repo_with_commit(
            &dir,
            "test.txt",
            "line one\nline two\n",
            "Alice",
            "first commit",
        );

        let oid2 = init_repo_with_commit(
            &dir,
            "test.txt",
            "line one\nmodified line two\nline three\n",
            "Bob",
            "second commit",
        );

        let repo = GitRepository::open(dir.path()).unwrap();
        let service = BlameService::new();
        let info = service.blame(&repo, "test.txt").unwrap();

        assert_eq!(info.lines.len(), 3);

        // Line 1 should still be from Alice's commit
        assert_eq!(info.lines[0].commit_id, oid1.to_string());
        assert_eq!(info.lines[0].author, "Alice");

        // Line 2 was modified by Bob
        assert_eq!(info.lines[1].commit_id, oid2.to_string());
        assert_eq!(info.lines[1].author, "Bob");
        assert_eq!(info.lines[1].content, "modified line two");

        // Line 3 was added by Bob
        assert_eq!(info.lines[2].commit_id, oid2.to_string());
        assert_eq!(info.lines[2].author, "Bob");
    }

    #[test]
    fn test_blame_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        // Need at least one commit for blame to work
        init_repo_with_commit(&dir, "other.txt", "content\n", "Alice", "init");

        let repo = GitRepository::open(dir.path()).unwrap();
        let service = BlameService::new();
        let result = service.blame(&repo, "nonexistent.txt");

        assert!(result.is_err());
    }
}
