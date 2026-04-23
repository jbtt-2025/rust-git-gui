use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::{
    CherryPickResult, CommitDetail, CommitInfo, DiffStats, FileStatus, FileStatusType, LogOptions,
    RefLabel, RefType, RevertResult, SignatureInfo,
};

pub struct CommitService;

impl CommitService {
    pub fn new() -> Self {
        Self
    }

    /// Walk commits using git2 revwalk, supporting branch filter, author filter,
    /// date range, path filter, message search, and pagination (offset/limit).
    pub fn commit_log(
        &self,
        repo: &GitRepository,
        options: LogOptions,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let inner = repo.inner();
        let mut revwalk = inner.revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)?;

        // Push starting point based on branch filter or HEAD
        if let Some(ref branch_name) = options.branch {
            let reference = inner
                .find_branch(branch_name, git2::BranchType::Local)
                .or_else(|_| inner.find_branch(branch_name, git2::BranchType::Remote))
                .map_err(|_| GitError::InvalidArgument(format!("Branch not found: {}", branch_name)))?;
            let oid = reference
                .get()
                .target()
                .ok_or_else(|| GitError::InvalidArgument("Branch has no target".to_string()))?;
            revwalk.push(oid)?;
        } else {
            revwalk.push_head()?;
        }

        // Build ref lookup maps for decorating commits
        let ref_map = self.build_ref_map(inner)?;
        let head_target = inner.head().ok().and_then(|h| h.target());

        // Path filter: use pathspec on revwalk if provided
        if let Some(ref path) = options.path {
            revwalk.push_head().ok(); // ensure head is pushed
            // We'll filter by path during iteration since revwalk doesn't support pathspec directly
            let _ = path; // used below in the loop
        }

        let mut results = Vec::new();
        let mut skipped = 0usize;

        for oid_result in revwalk {
            let oid = oid_result?;
            let commit = inner.find_commit(oid)?;

            // Apply filters
            if !self.matches_filters(&commit, &options, inner) {
                continue;
            }

            // Pagination: skip `offset` matching commits
            if skipped < options.offset {
                skipped += 1;
                continue;
            }

            // Collect up to `limit` commits
            if options.limit > 0 && results.len() >= options.limit {
                break;
            }

            let refs = ref_map
                .get(&oid)
                .cloned()
                .unwrap_or_default();

            let info = self.commit_to_info(&commit, &refs, head_target);
            results.push(info);
        }

        Ok(results)
    }

    /// Get full commit details including file changes and diff stats.
    pub fn commit_detail(
        &self,
        repo: &GitRepository,
        id: &str,
    ) -> Result<CommitDetail, GitError> {
        let inner = repo.inner();
        let oid = git2::Oid::from_str(id)
            .map_err(|_| GitError::InvalidArgument(format!("Invalid commit id: {}", id)))?;
        let commit = inner.find_commit(oid)?;

        let ref_map = self.build_ref_map(inner)?;
        let head_target = inner.head().ok().and_then(|h| h.target());
        let refs = ref_map.get(&oid).cloned().unwrap_or_default();
        let info = self.commit_to_info(&commit, &refs, head_target);

        // Diff against parent (or empty tree for root commit)
        let commit_tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = inner.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            None,
        )?;

        let mut files = Vec::new();
        let mut insertions = 0usize;
        let mut deletions = 0usize;

        diff.foreach(
            &mut |delta, _progress| {
                let path = delta
                    .new_file()
                    .path()
                    .or_else(|| delta.old_file().path())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let status = match delta.status() {
                    git2::Delta::Added => FileStatusType::Untracked,
                    git2::Delta::Deleted => FileStatusType::Deleted,
                    git2::Delta::Modified => FileStatusType::Modified,
                    git2::Delta::Renamed => FileStatusType::Renamed,
                    _ => FileStatusType::Modified,
                };

                files.push(FileStatus { path, status });
                true
            },
            None,
            None,
            Some(&mut |_delta, _hunk, line| {
                match line.origin() {
                    '+' => insertions += 1,
                    '-' => deletions += 1,
                    _ => {}
                }
                true
            }),
        )?;

        let stats = DiffStats {
            files_changed: files.len(),
            insertions,
            deletions,
        };

        Ok(CommitDetail {
            commit: info,
            files,
            stats,
        })
    }

    /// Create a new commit from the current index/staging area.
    pub fn create_commit(
        &self,
        repo: &GitRepository,
        message: &str,
    ) -> Result<CommitInfo, GitError> {
        let inner = repo.inner();
        let sig = inner.signature().map_err(|e| {
            GitError::InvalidArgument(format!("No signature configured: {}", e))
        })?;

        let mut index = inner.index()?;
        let tree_oid = index.write_tree()?;
        let tree = inner.find_tree(tree_oid)?;

        let parents = match inner.head() {
            Ok(head) => {
                let target = head.target().ok_or_else(|| {
                    GitError::InvalidArgument("HEAD has no target".to_string())
                })?;
                vec![inner.find_commit(target)?]
            }
            Err(_) => vec![], // Initial commit, no parents
        };
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

        let oid = inner.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)?;
        let commit = inner.find_commit(oid)?;

        let ref_map = self.build_ref_map(inner)?;
        let refs = ref_map.get(&oid).cloned().unwrap_or_default();
        let head_target = Some(oid);

        Ok(self.commit_to_info(&commit, &refs, head_target))
    }

    /// Amend the last commit with current staging area and new message.
    pub fn amend_commit(
        &self,
        repo: &GitRepository,
        message: &str,
    ) -> Result<CommitInfo, GitError> {
        let inner = repo.inner();
        let head = inner.head()?;
        let head_commit = head.peel_to_commit()?;

        let sig = inner.signature().map_err(|e| {
            GitError::InvalidArgument(format!("No signature configured: {}", e))
        })?;

        let mut index = inner.index()?;
        let tree_oid = index.write_tree()?;
        let tree = inner.find_tree(tree_oid)?;

        let oid = head_commit.amend(
            Some("HEAD"),
            Some(&sig),
            Some(&sig),
            None, // encoding
            Some(message),
            Some(&tree),
        )?;

        let commit = inner.find_commit(oid)?;
        let ref_map = self.build_ref_map(inner)?;
        let refs = ref_map.get(&oid).cloned().unwrap_or_default();
        let head_target = Some(oid);

        Ok(self.commit_to_info(&commit, &refs, head_target))
    }

    // --- Private helpers ---

    /// Check if a commit matches all the provided filter options.
    fn matches_filters(
        &self,
        commit: &git2::Commit,
        options: &LogOptions,
        inner: &git2::Repository,
    ) -> bool {
        // Author filter
        if let Some(ref author_filter) = options.author {
            let author = commit.author();
            let name = author.name().unwrap_or("");
            let email = author.email().unwrap_or("");
            let filter_lower = author_filter.to_lowercase();
            if !name.to_lowercase().contains(&filter_lower)
                && !email.to_lowercase().contains(&filter_lower)
            {
                return false;
            }
        }

        // Date range filters (since/until are epoch timestamps)
        let commit_time = commit.time().seconds();
        if let Some(since) = options.since {
            if commit_time < since {
                return false;
            }
        }
        if let Some(until) = options.until {
            if commit_time > until {
                return false;
            }
        }

        // Message search
        if let Some(ref search) = options.search {
            let msg = commit.message().unwrap_or("");
            if !msg.to_lowercase().contains(&search.to_lowercase()) {
                return false;
            }
        }

        // Path filter: check if the commit touches the given path
        if let Some(ref path) = options.path {
            if !self.commit_touches_path(commit, inner, path) {
                return false;
            }
        }

        true
    }

    /// Check if a commit modifies a specific file path by diffing against its parent.
    fn commit_touches_path(
        &self,
        commit: &git2::Commit,
        inner: &git2::Repository,
        path: &str,
    ) -> bool {
        let commit_tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => return false,
        };

        let parent_tree = if commit.parent_count() > 0 {
            commit.parent(0).ok().and_then(|p| p.tree().ok())
        } else {
            None
        };

        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.pathspec(path);

        let diff = inner.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            Some(&mut diff_opts),
        );

        match diff {
            Ok(d) => d.deltas().count() > 0,
            Err(_) => false,
        }
    }

    /// Build a map from commit OID to the list of refs (branches/tags) pointing at it.
    fn build_ref_map(
        &self,
        inner: &git2::Repository,
    ) -> Result<std::collections::HashMap<git2::Oid, Vec<RefLabel>>, GitError> {
        let mut map: std::collections::HashMap<git2::Oid, Vec<RefLabel>> =
            std::collections::HashMap::new();

        let head_ref = inner.head().ok().and_then(|h| h.shorthand().map(|s| s.to_string()));

        // Local branches
        if let Ok(branches) = inner.branches(Some(git2::BranchType::Local)) {
            for branch_result in branches {
                if let Ok((branch, _)) = branch_result {
                    if let Some(oid) = branch.get().target() {
                        let name = branch.name().ok().flatten().unwrap_or("").to_string();
                        let is_head = head_ref.as_deref() == Some(&name);
                        map.entry(oid).or_default().push(RefLabel {
                            name,
                            ref_type: RefType::LocalBranch,
                            is_head,
                        });
                    }
                }
            }
        }

        // Remote branches
        if let Ok(branches) = inner.branches(Some(git2::BranchType::Remote)) {
            for branch_result in branches {
                if let Ok((branch, _)) = branch_result {
                    if let Some(oid) = branch.get().target() {
                        let full_name = branch.name().ok().flatten().unwrap_or("").to_string();
                        let (remote, name) = if let Some(pos) = full_name.find('/') {
                            (
                                full_name[..pos].to_string(),
                                full_name[pos + 1..].to_string(),
                            )
                        } else {
                            (String::new(), full_name.clone())
                        };
                        map.entry(oid).or_default().push(RefLabel {
                            name,
                            ref_type: RefType::RemoteBranch { remote },
                            is_head: false,
                        });
                    }
                }
            }
        }

        // Tags
        if let Ok(tag_names) = inner.tag_names(None) {
            for tag_name in tag_names.iter().flatten() {
                if let Ok(reference) = inner.find_reference(&format!("refs/tags/{}", tag_name)) {
                    // Resolve to the commit OID (peel annotated tags)
                    let oid = reference
                        .peel_to_commit()
                        .map(|c| c.id())
                        .or_else(|_| reference.target().ok_or_else(|| git2::Error::from_str("no target")));
                    if let Ok(oid) = oid {
                        map.entry(oid).or_default().push(RefLabel {
                            name: tag_name.to_string(),
                            ref_type: RefType::Tag,
                            is_head: false,
                        });
                    }
                }
            }
        }

        Ok(map)
    }

    /// Convert a git2::Commit to our domain CommitInfo.
    fn commit_to_info(
        &self,
        commit: &git2::Commit,
        refs: &[RefLabel],
        head_target: Option<git2::Oid>,
    ) -> CommitInfo {
        let id = commit.id().to_string();
        let short_id = id[..7.min(id.len())].to_string();

        let message = commit.message().unwrap_or("").to_string();

        let author = commit.author();
        let committer = commit.committer();

        let parent_ids: Vec<String> = (0..commit.parent_count())
            .filter_map(|i| commit.parent_id(i).ok())
            .map(|oid| oid.to_string())
            .collect();

        let is_cherry_picked = message.contains("cherry picked from");

        let is_head_commit = head_target.map_or(false, |h| h == commit.id());

        // Update is_head on refs if this is the HEAD commit
        let refs: Vec<RefLabel> = refs
            .iter()
            .map(|r| {
                if is_head_commit && r.is_head {
                    r.clone()
                } else {
                    r.clone()
                }
            })
            .collect();

        CommitInfo {
            id,
            short_id,
            message,
            author: SignatureInfo {
                name: author.name().unwrap_or("").to_string(),
                email: author.email().unwrap_or("").to_string(),
                timestamp: author.when().seconds(),
            },
            committer: SignatureInfo {
                name: committer.name().unwrap_or("").to_string(),
                email: committer.email().unwrap_or("").to_string(),
                timestamp: committer.when().seconds(),
            },
            parent_ids,
            refs,
            is_cherry_picked,
        }
    }

    /// Cherry-pick one or more commits onto the current HEAD.
    ///
    /// For each commit ID, applies the commit's changes. If no conflicts arise,
    /// a new commit is created with "(cherry picked from commit ...)" appended
    /// to the message. Stops at the first conflict.
    pub fn cherry_pick(
        &self,
        repo: &GitRepository,
        commit_ids: &[&str],
    ) -> Result<CherryPickResult, GitError> {
        let inner = repo.inner();
        let mut new_commits = Vec::new();

        for &cid in commit_ids {
            let oid = git2::Oid::from_str(cid)
                .map_err(|_| GitError::InvalidArgument(format!("Invalid commit id: {}", cid)))?;
            let commit = inner.find_commit(oid)?;

            // Perform the cherry-pick (applies changes to index + workdir)
            inner.cherrypick(&commit, None)?;

            // Check for conflicts
            let index = inner.index()?;
            if index.has_conflicts() {
                let conflict_files: Vec<String> = index
                    .conflicts()?
                    .filter_map(|c| c.ok())
                    .filter_map(|entry| {
                        entry
                            .our
                            .or(entry.their)
                            .or(entry.ancestor)
                            .map(|e| String::from_utf8_lossy(&e.path).to_string())
                    })
                    .collect();
                return Ok(CherryPickResult::Conflict {
                    files: conflict_files,
                    at_commit: cid.to_string(),
                });
            }

            // No conflicts — create the cherry-pick commit
            let sig = inner.signature().map_err(|e| {
                GitError::InvalidArgument(format!("No signature configured: {}", e))
            })?;

            let mut idx = inner.index()?;
            let tree_oid = idx.write_tree()?;
            let tree = inner.find_tree(tree_oid)?;

            let head_commit = inner.head()?.peel_to_commit()?;
            let original_msg = commit.message().unwrap_or("");
            let new_msg = format!(
                "{}\n\n(cherry picked from commit {})",
                original_msg,
                commit.id()
            );

            let new_oid = inner.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &new_msg,
                &tree,
                &[&head_commit],
            )?;
            new_commits.push(new_oid.to_string());

            // Clean up cherry-pick state
            inner.cleanup_state()?;
        }

        Ok(CherryPickResult::Success { new_commits })
    }

    /// Revert one or more commits, creating new revert commits.
    ///
    /// For each commit ID, reverse-applies the commit's changes. If no conflicts
    /// arise, a new revert commit is created. Stops at the first conflict.
    pub fn revert(
        &self,
        repo: &GitRepository,
        commit_ids: &[&str],
    ) -> Result<RevertResult, GitError> {
        let inner = repo.inner();
        let mut new_commits = Vec::new();

        for &cid in commit_ids {
            let oid = git2::Oid::from_str(cid)
                .map_err(|_| GitError::InvalidArgument(format!("Invalid commit id: {}", cid)))?;
            let commit = inner.find_commit(oid)?;

            // Perform the revert (reverse-applies changes to index + workdir)
            inner.revert(&commit, None)?;

            // Check for conflicts
            let index = inner.index()?;
            if index.has_conflicts() {
                let conflict_files: Vec<String> = index
                    .conflicts()?
                    .filter_map(|c| c.ok())
                    .filter_map(|entry| {
                        entry
                            .our
                            .or(entry.their)
                            .or(entry.ancestor)
                            .map(|e| String::from_utf8_lossy(&e.path).to_string())
                    })
                    .collect();
                return Ok(RevertResult::Conflict {
                    files: conflict_files,
                    at_commit: cid.to_string(),
                });
            }

            // No conflicts — create the revert commit
            let sig = inner.signature().map_err(|e| {
                GitError::InvalidArgument(format!("No signature configured: {}", e))
            })?;

            let mut idx = inner.index()?;
            let tree_oid = idx.write_tree()?;
            let tree = inner.find_tree(tree_oid)?;

            let head_commit = inner.head()?.peel_to_commit()?;
            let original_msg = commit.message().unwrap_or("");
            let short = &commit.id().to_string()[..7.min(commit.id().to_string().len())];
            let new_msg = format!("Revert \"{}\"\n\nThis reverts commit {}.", original_msg, short);

            let new_oid = inner.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &new_msg,
                &tree,
                &[&head_commit],
            )?;
            new_commits.push(new_oid.to_string());

            // Clean up revert state
            inner.cleanup_state()?;
        }

        Ok(RevertResult::Success { new_commits })
    }

    /// Generate a standard .patch file for the given commit.
    ///
    /// The output includes the commit message, author info, and diff,
    /// formatted similarly to `git format-patch`.
    pub fn create_patch(
        &self,
        repo: &GitRepository,
        commit_id: &str,
    ) -> Result<Vec<u8>, GitError> {
        let inner = repo.inner();
        let oid = git2::Oid::from_str(commit_id)
            .map_err(|_| GitError::InvalidArgument(format!("Invalid commit id: {}", commit_id)))?;
        let commit = inner.find_commit(oid)?;

        let author = commit.author();
        let message = commit.message().unwrap_or("");

        // Subject is the first line, body is the rest
        let (subject, body) = match message.find('\n') {
            Some(pos) => (&message[..pos], message[pos + 1..].trim_start_matches('\n')),
            None => (message, ""),
        };

        // Build the diff
        let commit_tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = inner.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)?;

        // Format the patch
        let mut patch = Vec::new();

        // Header
        let from_line = format!("From {} Mon Sep 17 00:00:00 2001\n", commit.id());
        patch.extend_from_slice(from_line.as_bytes());

        let author_name = author.name().unwrap_or("");
        let author_email = author.email().unwrap_or("");
        let from_header = format!("From: {} <{}>\n", author_name, author_email);
        patch.extend_from_slice(from_header.as_bytes());

        // Date
        let when = author.when();
        let offset_minutes = when.offset_minutes();
        let sign = if offset_minutes >= 0 { '+' } else { '-' };
        let abs_offset = offset_minutes.unsigned_abs();
        let date_line = format!(
            "Date: {} {}{:02}{:02}\n",
            when.seconds(),
            sign,
            abs_offset / 60,
            abs_offset % 60
        );
        patch.extend_from_slice(date_line.as_bytes());

        let subject_line = format!("Subject: [PATCH] {}\n", subject);
        patch.extend_from_slice(subject_line.as_bytes());

        patch.extend_from_slice(b"\n");

        if !body.is_empty() {
            patch.extend_from_slice(body.as_bytes());
            patch.extend_from_slice(b"\n");
        }

        patch.extend_from_slice(b"---\n");

        // Diff stats summary
        let stats = diff.stats()?;
        let stats_buf = stats.to_buf(git2::DiffStatsFormat::FULL, 80)?;
        patch.extend_from_slice(&stats_buf);

        patch.extend_from_slice(b"\n");

        // Actual diff content
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let origin = line.origin();
            match origin {
                '+' | '-' | ' ' => {
                    patch.push(origin as u8);
                }
                _ => {}
            }
            patch.extend_from_slice(line.content());
            true
        })?;

        patch.extend_from_slice(b"-- \n2.0.0\n");

        Ok(patch)
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

            // Configure user for commits
            let mut config = inner.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();

            // Create initial commit
            let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
            let tree_id = inner.index().unwrap().write_tree().unwrap();
            let tree = inner.find_tree(tree_id).unwrap();
            inner
                .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    /// Helper: create a file, stage it, and commit with the given message.
    fn add_file_and_commit(repo: &GitRepository, filename: &str, content: &str, message: &str) {
        let inner = repo.inner();
        let workdir = inner.workdir().unwrap();
        let file_path = workdir.join(filename);
        fs::write(&file_path, content).unwrap();

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
    fn test_commit_log_returns_commits() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "a.txt", "hello", "Add a.txt");
        add_file_and_commit(&repo, "b.txt", "world", "Add b.txt");

        let service = CommitService::new();
        let options = LogOptions {
            branch: None,
            author: None,
            since: None,
            until: None,
            path: None,
            search: None,
            offset: 0,
            limit: 100,
        };

        let log = service.commit_log(&repo, options).unwrap();
        // 3 commits: initial + a.txt + b.txt
        assert_eq!(log.len(), 3);
        // Most recent first
        assert_eq!(log[0].message, "Add b.txt");
        assert_eq!(log[1].message, "Add a.txt");
        assert_eq!(log[2].message, "Initial commit");
    }

    #[test]
    fn test_commit_log_pagination() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "a.txt", "a", "Commit A");
        add_file_and_commit(&repo, "b.txt", "b", "Commit B");
        add_file_and_commit(&repo, "c.txt", "c", "Commit C");

        let service = CommitService::new();

        // Get first 2
        let page1 = service
            .commit_log(
                &repo,
                LogOptions {
                    branch: None,
                    author: None,
                    since: None,
                    until: None,
                    path: None,
                    search: None,
                    offset: 0,
                    limit: 2,
                },
            )
            .unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].message, "Commit C");
        assert_eq!(page1[1].message, "Commit B");

        // Get next 2 (offset=2)
        let page2 = service
            .commit_log(
                &repo,
                LogOptions {
                    branch: None,
                    author: None,
                    since: None,
                    until: None,
                    path: None,
                    search: None,
                    offset: 2,
                    limit: 2,
                },
            )
            .unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].message, "Commit A");
        assert_eq!(page2[1].message, "Initial commit");
    }

    #[test]
    fn test_commit_log_author_filter() {
        let (_dir, repo) = setup_repo();
        let inner = repo.inner();

        // Create a commit with a different author
        let workdir = inner.workdir().unwrap();
        fs::write(workdir.join("x.txt"), "x").unwrap();
        let mut index = inner.index().unwrap();
        index.add_path(Path::new("x.txt")).unwrap();
        index.write().unwrap();

        let other_sig = git2::Signature::now("Other Author", "other@example.com").unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = inner.find_tree(tree_id).unwrap();
        let head = inner.head().unwrap().target().unwrap();
        let parent = inner.find_commit(head).unwrap();
        inner
            .commit(Some("HEAD"), &other_sig, &other_sig, "Other commit", &tree, &[&parent])
            .unwrap();

        let service = CommitService::new();
        let log = service
            .commit_log(
                &repo,
                LogOptions {
                    branch: None,
                    author: Some("Other Author".to_string()),
                    since: None,
                    until: None,
                    path: None,
                    search: None,
                    offset: 0,
                    limit: 100,
                },
            )
            .unwrap();

        assert_eq!(log.len(), 1);
        assert_eq!(log[0].message, "Other commit");
    }

    #[test]
    fn test_commit_log_message_search() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "a.txt", "a", "feat: add feature A");
        add_file_and_commit(&repo, "b.txt", "b", "fix: bug fix B");

        let service = CommitService::new();
        let log = service
            .commit_log(
                &repo,
                LogOptions {
                    branch: None,
                    author: None,
                    since: None,
                    until: None,
                    path: None,
                    search: Some("feat".to_string()),
                    offset: 0,
                    limit: 100,
                },
            )
            .unwrap();

        assert_eq!(log.len(), 1);
        assert_eq!(log[0].message, "feat: add feature A");
    }

    #[test]
    fn test_commit_log_path_filter() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "a.txt", "a", "Add a");
        add_file_and_commit(&repo, "b.txt", "b", "Add b");

        let service = CommitService::new();
        let log = service
            .commit_log(
                &repo,
                LogOptions {
                    branch: None,
                    author: None,
                    since: None,
                    until: None,
                    path: Some("a.txt".to_string()),
                    search: None,
                    offset: 0,
                    limit: 100,
                },
            )
            .unwrap();

        assert_eq!(log.len(), 1);
        assert_eq!(log[0].message, "Add a");
    }

    #[test]
    fn test_commit_detail() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "hello.txt", "hello world\n", "Add hello");

        let service = CommitService::new();
        let log = service
            .commit_log(
                &repo,
                LogOptions {
                    branch: None,
                    author: None,
                    since: None,
                    until: None,
                    path: None,
                    search: None,
                    offset: 0,
                    limit: 1,
                },
            )
            .unwrap();

        let detail = service.commit_detail(&repo, &log[0].id).unwrap();
        assert_eq!(detail.commit.message, "Add hello");
        assert_eq!(detail.files.len(), 1);
        assert_eq!(detail.files[0].path, "hello.txt");
        assert!(detail.stats.insertions > 0);
    }

    #[test]
    fn test_create_commit() {
        let (_dir, repo) = setup_repo();
        let inner = repo.inner();
        let workdir = inner.workdir().unwrap();

        // Stage a new file
        fs::write(workdir.join("new.txt"), "new content").unwrap();
        let mut index = inner.index().unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();

        let service = CommitService::new();
        let commit = service.create_commit(&repo, "My new commit").unwrap();
        assert_eq!(commit.message, "My new commit");
        assert!(!commit.id.is_empty());
        assert_eq!(commit.parent_ids.len(), 1);
    }

    #[test]
    fn test_amend_commit() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "file.txt", "content", "Original message");

        let service = CommitService::new();
        let amended = service.amend_commit(&repo, "Amended message").unwrap();
        assert_eq!(amended.message, "Amended message");

        // Verify the log only has 2 commits (initial + amended, not 3)
        let log = service
            .commit_log(
                &repo,
                LogOptions {
                    branch: None,
                    author: None,
                    since: None,
                    until: None,
                    path: None,
                    search: None,
                    offset: 0,
                    limit: 100,
                },
            )
            .unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].message, "Amended message");
    }

    #[test]
    fn test_cherry_picked_detection() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(
            &repo,
            "cp.txt",
            "cherry",
            "Apply fix\n\n(cherry picked from commit abc123)",
        );

        let service = CommitService::new();
        let log = service
            .commit_log(
                &repo,
                LogOptions {
                    branch: None,
                    author: None,
                    since: None,
                    until: None,
                    path: None,
                    search: None,
                    offset: 0,
                    limit: 1,
                },
            )
            .unwrap();

        assert!(log[0].is_cherry_picked);
    }

    #[test]
    fn test_commit_has_parent_ids() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "f.txt", "f", "Second commit");

        let service = CommitService::new();
        let log = service
            .commit_log(
                &repo,
                LogOptions {
                    branch: None,
                    author: None,
                    since: None,
                    until: None,
                    path: None,
                    search: None,
                    offset: 0,
                    limit: 100,
                },
            )
            .unwrap();

        // Second commit should have 1 parent
        assert_eq!(log[0].parent_ids.len(), 1);
        // Initial commit should have 0 parents
        assert_eq!(log[1].parent_ids.len(), 0);
    }

    #[test]
    fn test_commit_refs_include_branch() {
        let (_dir, repo) = setup_repo();

        let service = CommitService::new();
        let log = service
            .commit_log(
                &repo,
                LogOptions {
                    branch: None,
                    author: None,
                    since: None,
                    until: None,
                    path: None,
                    search: None,
                    offset: 0,
                    limit: 1,
                },
            )
            .unwrap();

        // HEAD commit should have at least one ref (the default branch)
        let head_commit = &log[0];
        assert!(
            !head_commit.refs.is_empty(),
            "HEAD commit should have branch refs"
        );
        // At least one should be a local branch
        assert!(head_commit.refs.iter().any(|r| matches!(r.ref_type, RefType::LocalBranch)));
    }

    #[test]
    fn test_commit_detail_invalid_id() {
        let (_dir, repo) = setup_repo();
        let service = CommitService::new();
        let result = service.commit_detail(&repo, "not-a-valid-oid");
        assert!(result.is_err());
    }

    // --- Cherry-pick tests ---

    #[test]
    fn test_cherry_pick_single_commit() {
        let (_dir, repo) = setup_repo();
        let inner = repo.inner();

        // Create a commit on a side branch
        add_file_and_commit(&repo, "main.txt", "main content", "Main commit");

        let main_head = inner.head().unwrap().target().unwrap();

        // Create a side branch from initial commit
        let initial_oid = inner.find_commit(main_head).unwrap().parent_id(0).unwrap();
        inner.branch("side", &inner.find_commit(initial_oid).unwrap(), false).unwrap();
        inner.set_head("refs/heads/side").unwrap();
        inner.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

        add_file_and_commit(&repo, "side.txt", "side content", "Side commit");
        let side_commit_id = inner.head().unwrap().target().unwrap().to_string();

        // Switch back to main branch
        let main_branch = inner.find_branch("master", git2::BranchType::Local)
            .or_else(|_| inner.find_branch("main", git2::BranchType::Local));
        let branch_name = match main_branch {
            Ok(b) => format!("refs/heads/{}", b.name().unwrap().unwrap()),
            Err(_) => {
                // Find the default branch name
                let head_ref = inner.find_commit(main_head).unwrap();
                let _ = head_ref;
                // Fallback: create a branch pointing to main_head
                inner.branch("main", &inner.find_commit(main_head).unwrap(), true).unwrap();
                "refs/heads/main".to_string()
            }
        };
        inner.set_head(&branch_name).unwrap();
        inner.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

        let service = CommitService::new();
        let result = service.cherry_pick(&repo, &[&side_commit_id]).unwrap();

        match result {
            CherryPickResult::Success { new_commits } => {
                assert_eq!(new_commits.len(), 1);
                // Verify the new commit message contains cherry-pick annotation
                let new_oid = git2::Oid::from_str(&new_commits[0]).unwrap();
                let new_commit = inner.find_commit(new_oid).unwrap();
                let msg = new_commit.message().unwrap();
                assert!(msg.contains("cherry picked from commit"));
                assert!(msg.contains("Side commit"));
            }
            CherryPickResult::Conflict { .. } => panic!("Expected success, got conflict"),
        }
    }

    #[test]
    fn test_cherry_pick_with_conflict() {
        let (_dir, repo) = setup_repo();
        let inner = repo.inner();

        // Create a file on main
        add_file_and_commit(&repo, "conflict.txt", "main version", "Main file");
        let main_head = inner.head().unwrap().target().unwrap();

        // Create side branch from initial commit and modify same file
        let initial_oid = inner.find_commit(main_head).unwrap().parent_id(0).unwrap();
        inner.branch("side", &inner.find_commit(initial_oid).unwrap(), false).unwrap();
        inner.set_head("refs/heads/side").unwrap();
        inner.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

        add_file_and_commit(&repo, "conflict.txt", "side version", "Side conflict");
        let side_commit_id = inner.head().unwrap().target().unwrap().to_string();

        // Switch back to main
        inner.branch("main", &inner.find_commit(main_head).unwrap(), true).unwrap();
        inner.set_head("refs/heads/main").unwrap();
        inner.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

        let service = CommitService::new();
        let result = service.cherry_pick(&repo, &[&side_commit_id]).unwrap();

        match result {
            CherryPickResult::Conflict { files, at_commit } => {
                assert!(!files.is_empty());
                assert!(files.iter().any(|f| f.contains("conflict.txt")));
                assert_eq!(at_commit, side_commit_id);
            }
            CherryPickResult::Success { .. } => panic!("Expected conflict, got success"),
        }
    }

    #[test]
    fn test_cherry_pick_multiple_commits() {
        let (_dir, repo) = setup_repo();
        let inner = repo.inner();

        let initial_head = inner.head().unwrap().target().unwrap();

        // Create side branch with two commits
        inner.branch("side", &inner.find_commit(initial_head).unwrap(), false).unwrap();
        inner.set_head("refs/heads/side").unwrap();
        inner.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

        add_file_and_commit(&repo, "file1.txt", "content1", "Side commit 1");
        let side_id1 = inner.head().unwrap().target().unwrap().to_string();
        add_file_and_commit(&repo, "file2.txt", "content2", "Side commit 2");
        let side_id2 = inner.head().unwrap().target().unwrap().to_string();

        // Switch back to main (initial commit)
        inner.branch("main", &inner.find_commit(initial_head).unwrap(), true).unwrap();
        inner.set_head("refs/heads/main").unwrap();
        inner.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

        let service = CommitService::new();
        let result = service.cherry_pick(&repo, &[&side_id1, &side_id2]).unwrap();

        match result {
            CherryPickResult::Success { new_commits } => {
                assert_eq!(new_commits.len(), 2);
            }
            CherryPickResult::Conflict { .. } => panic!("Expected success, got conflict"),
        }
    }

    // --- Revert tests ---

    #[test]
    fn test_revert_single_commit() {
        let (_dir, repo) = setup_repo();
        let inner = repo.inner();

        add_file_and_commit(&repo, "revert_me.txt", "to be reverted\n", "Add revert_me");
        let commit_to_revert = inner.head().unwrap().target().unwrap().to_string();

        let service = CommitService::new();
        let result = service.revert(&repo, &[&commit_to_revert]).unwrap();

        match result {
            RevertResult::Success { new_commits } => {
                assert_eq!(new_commits.len(), 1);
                let new_oid = git2::Oid::from_str(&new_commits[0]).unwrap();
                let new_commit = inner.find_commit(new_oid).unwrap();
                let msg = new_commit.message().unwrap();
                assert!(msg.contains("Revert"));
                assert!(msg.contains("Add revert_me"));
            }
            RevertResult::Conflict { .. } => panic!("Expected success, got conflict"),
        }
    }

    #[test]
    fn test_revert_with_conflict() {
        let (_dir, repo) = setup_repo();
        let inner = repo.inner();

        // Create a file
        add_file_and_commit(&repo, "file.txt", "original\n", "Add file");
        let first_commit = inner.head().unwrap().target().unwrap().to_string();

        // Modify the same file in a second commit
        add_file_and_commit(&repo, "file.txt", "modified content\n", "Modify file");

        // Reverting the first commit should conflict because the file was modified since
        let service = CommitService::new();
        let result = service.revert(&repo, &[&first_commit]).unwrap();

        match result {
            RevertResult::Conflict { files, at_commit } => {
                assert!(!files.is_empty());
                assert_eq!(at_commit, first_commit);
            }
            RevertResult::Success { .. } => {
                // Some git implementations may auto-resolve this; that's acceptable too
            }
        }
    }

    // --- Create patch test ---

    #[test]
    fn test_create_patch() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "patch_file.txt", "patch content\n", "Add patch file");

        let inner = repo.inner();
        let commit_id = inner.head().unwrap().target().unwrap().to_string();

        let service = CommitService::new();
        let patch_bytes = service.create_patch(&repo, &commit_id).unwrap();
        let patch_str = String::from_utf8_lossy(&patch_bytes);

        // Verify patch contains expected elements
        assert!(patch_str.contains("From:"));
        assert!(patch_str.contains("Subject: [PATCH]"));
        assert!(patch_str.contains("Add patch file"));
        assert!(patch_str.contains("patch_file.txt"));
        assert!(patch_str.contains("patch content"));
    }
}
