use chrono::{DateTime, Utc};

use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::{GitOperation, RepositorySnapshot};

/// Internal entry tracking a single undoable operation.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    pub operation: GitOperation,
    pub description: String,
    pub before_state: RepositorySnapshot,
    pub after_state: RepositorySnapshot,
    pub timestamp: DateTime<Utc>,
}

/// Engine that records Git operations and supports undo/redo by restoring
/// repository snapshots (HEAD + ref state).
pub struct UndoEngine {
    history: Vec<UndoEntry>,
    cursor: usize, // points past the last recorded entry (i.e. next insert position)
}

impl UndoEngine {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            cursor: 0,
        }
    }

    /// Record a new undoable operation. Truncates any redo history beyond the
    /// current cursor position before appending.
    pub fn record(&mut self, entry: UndoEntry) {
        self.history.truncate(self.cursor);
        self.history.push(entry);
        self.cursor = self.history.len();
    }

    /// Undo the most recent operation by restoring its `before_state` snapshot.
    /// Returns the description of the undone operation.
    pub fn undo(&mut self, repo: &GitRepository) -> Result<String, GitError> {
        if self.cursor == 0 {
            return Err(GitError::InvalidArgument(
                "Nothing to undo".to_string(),
            ));
        }
        self.cursor -= 1;
        let entry = &self.history[self.cursor];
        let description = entry.description.clone();
        restore_snapshot(repo, &entry.before_state)?;
        Ok(description)
    }

    /// Redo the most recently undone operation by restoring its `after_state` snapshot.
    /// Returns the description of the redone operation.
    pub fn redo(&mut self, repo: &GitRepository) -> Result<String, GitError> {
        if self.cursor >= self.history.len() {
            return Err(GitError::InvalidArgument(
                "Nothing to redo".to_string(),
            ));
        }
        let entry = &self.history[self.cursor];
        let description = entry.description.clone();
        restore_snapshot(repo, &entry.after_state)?;
        self.cursor += 1;
        Ok(description)
    }

    /// Returns the description of the operation that would be undone, if any.
    pub fn can_undo(&self) -> Option<&str> {
        if self.cursor == 0 {
            None
        } else {
            Some(&self.history[self.cursor - 1].description)
        }
    }

    /// Returns the description of the operation that would be redone, if any.
    pub fn can_redo(&self) -> Option<&str> {
        if self.cursor >= self.history.len() {
            None
        } else {
            Some(&self.history[self.cursor].description)
        }
    }
}


/// Restore a repository to the state described by a `RepositorySnapshot`.
/// Sets HEAD to the snapshot's commit and optionally restores the symbolic ref.
fn restore_snapshot(
    repo: &GitRepository,
    snapshot: &RepositorySnapshot,
) -> Result<(), GitError> {
    let inner = repo.inner();
    let oid = git2::Oid::from_str(&snapshot.head_id)
        .map_err(|e| GitError::InvalidArgument(e.message().to_string()))?;
    let commit = inner
        .find_commit(oid)
        .map_err(|e| GitError::Git2(e.message().to_string()))?;

    // If the snapshot had a symbolic HEAD ref (e.g. refs/heads/main), restore it.
    if let Some(ref head_ref) = snapshot.head_ref {
        inner
            .set_head(head_ref)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;
    } else {
        // Detached HEAD — point directly at the commit.
        inner
            .set_head_detached(oid)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;
    }

    // Reset the working directory and index to match the target commit.
    let obj = commit.as_object();
    inner
        .reset(obj, git2::ResetType::Hard, None)
        .map_err(|e| GitError::Git2(e.message().to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::GitOperation;
    use chrono::Utc;
    use tempfile::TempDir;

    /// Helper: create a temp repo with an initial commit, return (TempDir, GitRepository, commit_oid).
    fn setup_repo() -> (TempDir, GitRepository, String) {
        let dir = TempDir::new().unwrap();
        let repo = GitRepository::init(dir.path()).unwrap();

        // Create an initial commit so HEAD is valid.
        let oid = {
            let inner = repo.inner();
            let sig = git2::Signature::now("Test", "test@test.com").unwrap();
            let tree_id = inner.index().unwrap().write_tree().unwrap();
            let tree = inner.find_tree(tree_id).unwrap();
            inner
                .commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap()
        };

        (dir, repo, oid.to_string())
    }

    /// Helper: create a second commit on the repo, return its oid string.
    fn make_commit(repo: &GitRepository, message: &str) -> String {
        let inner = repo.inner();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let head = inner.head().unwrap().peel_to_commit().unwrap();
        let tree_id = inner.index().unwrap().write_tree().unwrap();
        let tree = inner.find_tree(tree_id).unwrap();
        let oid = inner
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
            .unwrap();
        drop(tree);
        drop(head);
        oid.to_string()
    }

    fn snapshot(head_id: &str, head_ref: Option<&str>) -> RepositorySnapshot {
        RepositorySnapshot {
            head_id: head_id.to_string(),
            head_ref: head_ref.map(|s| s.to_string()),
            index_tree_id: None,
        }
    }

    fn entry(
        op: GitOperation,
        desc: &str,
        before: RepositorySnapshot,
        after: RepositorySnapshot,
    ) -> UndoEntry {
        UndoEntry {
            operation: op,
            description: desc.to_string(),
            before_state: before,
            after_state: after,
            timestamp: Utc::now(),
        }
    }

    // ---- Pure logic tests (no repo needed) ----

    #[test]
    fn test_new_engine_empty() {
        let engine = UndoEngine::new();
        assert!(engine.can_undo().is_none());
        assert!(engine.can_redo().is_none());
    }

    #[test]
    fn test_record_and_can_undo() {
        let mut engine = UndoEngine::new();
        let e = entry(
            GitOperation::Commit { id: "abc".into() },
            "Commit abc",
            snapshot("000", Some("refs/heads/main")),
            snapshot("abc", Some("refs/heads/main")),
        );
        engine.record(e);
        assert_eq!(engine.can_undo(), Some("Commit abc"));
        assert!(engine.can_redo().is_none());
    }

    #[test]
    fn test_record_truncates_redo_history() {
        let mut engine = UndoEngine::new();
        let (_dir, repo, oid1) = setup_repo();
        let oid2 = make_commit(&repo, "second");

        let e1 = entry(
            GitOperation::Commit { id: oid2.clone() },
            "Commit second",
            snapshot(&oid1, Some("refs/heads/main")),
            snapshot(&oid2, Some("refs/heads/main")),
        );
        engine.record(e1);

        // Undo to create redo history
        engine.undo(&repo).unwrap();
        assert!(engine.can_redo().is_some());

        // Record a new operation — should truncate redo
        let oid3 = make_commit(&repo, "third");
        let e2 = entry(
            GitOperation::Commit { id: oid3.clone() },
            "Commit third",
            snapshot(&oid1, Some("refs/heads/main")),
            snapshot(&oid3, Some("refs/heads/main")),
        );
        engine.record(e2);
        assert!(engine.can_redo().is_none());
        assert_eq!(engine.can_undo(), Some("Commit third"));
    }

    #[test]
    fn test_undo_nothing_returns_error() {
        let mut engine = UndoEngine::new();
        let (_dir, repo, _) = setup_repo();
        let result = engine.undo(&repo);
        assert!(result.is_err());
    }

    #[test]
    fn test_redo_nothing_returns_error() {
        let mut engine = UndoEngine::new();
        let (_dir, repo, _) = setup_repo();
        let result = engine.redo(&repo);
        assert!(result.is_err());
    }

    #[test]
    fn test_undo_restores_before_state() {
        let (_dir, repo, oid1) = setup_repo();
        let oid2 = make_commit(&repo, "second");

        let mut engine = UndoEngine::new();
        engine.record(entry(
            GitOperation::Commit { id: oid2.clone() },
            "Commit second",
            snapshot(&oid1, Some("refs/heads/main")),
            snapshot(&oid2, Some("refs/heads/main")),
        ));

        let desc = engine.undo(&repo).unwrap();
        assert_eq!(desc, "Commit second");

        // HEAD should now point at oid1
        let head = repo.inner().head().unwrap();
        let head_oid = head.target().unwrap().to_string();
        assert_eq!(head_oid, oid1);
    }

    #[test]
    fn test_redo_restores_after_state() {
        let (_dir, repo, oid1) = setup_repo();
        let oid2 = make_commit(&repo, "second");

        let mut engine = UndoEngine::new();
        engine.record(entry(
            GitOperation::Commit { id: oid2.clone() },
            "Commit second",
            snapshot(&oid1, Some("refs/heads/main")),
            snapshot(&oid2, Some("refs/heads/main")),
        ));

        engine.undo(&repo).unwrap();
        let desc = engine.redo(&repo).unwrap();
        assert_eq!(desc, "Commit second");

        // HEAD should now point at oid2
        let head = repo.inner().head().unwrap();
        let head_oid = head.target().unwrap().to_string();
        assert_eq!(head_oid, oid2);
    }

    #[test]
    fn test_multiple_undo_redo() {
        let (_dir, repo, oid1) = setup_repo();
        let oid2 = make_commit(&repo, "second");
        let oid3 = make_commit(&repo, "third");

        let mut engine = UndoEngine::new();
        engine.record(entry(
            GitOperation::Commit { id: oid2.clone() },
            "Commit second",
            snapshot(&oid1, Some("refs/heads/main")),
            snapshot(&oid2, Some("refs/heads/main")),
        ));
        engine.record(entry(
            GitOperation::Commit { id: oid3.clone() },
            "Commit third",
            snapshot(&oid2, Some("refs/heads/main")),
            snapshot(&oid3, Some("refs/heads/main")),
        ));

        // Undo twice
        engine.undo(&repo).unwrap();
        engine.undo(&repo).unwrap();
        let head_oid = repo.inner().head().unwrap().target().unwrap().to_string();
        assert_eq!(head_oid, oid1);

        // Redo once
        engine.redo(&repo).unwrap();
        let head_oid = repo.inner().head().unwrap().target().unwrap().to_string();
        assert_eq!(head_oid, oid2);
    }

    #[test]
    fn test_can_undo_redo_descriptions() {
        let mut engine = UndoEngine::new();
        let (_dir, repo, oid1) = setup_repo();
        let oid2 = make_commit(&repo, "second");

        engine.record(entry(
            GitOperation::Checkout {
                from_branch: "main".into(),
                to_branch: "dev".into(),
            },
            "Checkout dev",
            snapshot(&oid1, Some("refs/heads/main")),
            snapshot(&oid2, Some("refs/heads/dev")),
        ));

        assert_eq!(engine.can_undo(), Some("Checkout dev"));
        assert_eq!(engine.can_redo(), None);

        engine.undo(&repo).unwrap();
        assert_eq!(engine.can_undo(), None);
        assert_eq!(engine.can_redo(), Some("Checkout dev"));
    }

    #[test]
    fn test_all_operation_types_recordable() {
        let mut engine = UndoEngine::new();
        let s = snapshot("abc", Some("refs/heads/main"));

        let operations = vec![
            (GitOperation::Commit { id: "abc".into() }, "commit"),
            (GitOperation::Checkout { from_branch: "a".into(), to_branch: "b".into() }, "checkout"),
            (GitOperation::Merge { source: "feat".into() }, "merge"),
            (GitOperation::Rebase { onto: "main".into() }, "rebase"),
            (GitOperation::BranchCreate { name: "feat".into() }, "branch create"),
            (GitOperation::BranchDelete { name: "feat".into(), target: "abc".into() }, "branch delete"),
            (GitOperation::Reset { mode: crate::models::ResetMode::Hard, target: "abc".into() }, "reset"),
            (GitOperation::Revert { commit_id: "abc".into() }, "revert"),
            (GitOperation::CherryPick { commit_id: "abc".into() }, "cherry-pick"),
            (GitOperation::Stash, "stash"),
            (GitOperation::StashPop { index: 0 }, "stash pop"),
        ];

        for (op, desc) in operations {
            engine.record(entry(op, desc, s.clone(), s.clone()));
        }

        // All 11 operations recorded
        assert_eq!(engine.history.len(), 11);
        assert_eq!(engine.can_undo(), Some("stash pop"));
    }

    /// **Validates: Requirements 22.3**
    ///
    /// Feature: rust-git-gui-client, Property 18: Undo/Redo 往返一致性
    ///
    /// For any Git operation, executing the operation then Undo then Redo
    /// SHALL leave the repository HEAD and ref state identical to the state
    /// right after the original operation.
    mod prop18_undo_redo_roundtrip {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn prop_undo_redo_roundtrip(
                num_commits in 1usize..=5,
            ) {
                // 1. Create a temp repo with an initial commit.
                let (_dir, repo, initial_oid) = setup_repo();

                // 2. Build a chain of commits, recording snapshots.
                let mut oids = vec![initial_oid];
                for i in 0..num_commits {
                    let msg = format!("commit_{}", i);
                    let new_oid = make_commit(&repo, &msg);
                    oids.push(new_oid);
                }

                // Record each commit as an UndoEntry.
                let mut engine = UndoEngine::new();
                for i in 0..num_commits {
                    let before_oid = &oids[i];
                    let after_oid = &oids[i + 1];
                    engine.record(entry(
                        GitOperation::Commit { id: after_oid.clone() },
                        &format!("Commit {}", i),
                        snapshot(before_oid, Some("refs/heads/main")),
                        snapshot(after_oid, Some("refs/heads/main")),
                    ));
                }

                // 3. For each recorded operation: capture after_state, undo, redo,
                //    then verify HEAD matches the after_state.
                //    We walk backwards from the most recent entry.
                for _ in 0..num_commits {
                    // The entry about to be undone is at cursor-1.
                    let expected_after_oid = engine.history[engine.cursor - 1]
                        .after_state
                        .head_id
                        .clone();

                    // Undo
                    engine.undo(&repo).unwrap();

                    // Redo — should restore to after_state
                    engine.redo(&repo).unwrap();

                    // Verify HEAD matches the after_state of the operation.
                    let head = repo.inner().head().unwrap();
                    let actual_oid = head.target().unwrap().to_string();
                    prop_assert_eq!(
                        &actual_oid,
                        &expected_after_oid,
                        "After undo+redo, HEAD should match the operation's after_state"
                    );

                    // Verify symbolic ref is still refs/heads/main.
                    let head_name = head.name().map(|s| s.to_string());
                    prop_assert_eq!(
                        head_name.as_deref(),
                        Some("refs/heads/main"),
                        "HEAD ref should still point to refs/heads/main after undo+redo"
                    );

                    // Move cursor back so we can test the next (earlier) entry.
                    engine.undo(&repo).unwrap();
                }
            }
        }
    }

    /// **Validates: Requirements 22.2**
    ///
    /// Feature: rust-git-gui-client, Property 17: Undo 恢复前状态
    ///
    /// For any Git operation and its before-state snapshot, after executing
    /// Undo the repository HEAD and ref state SHALL match the before_state.
    mod prop17_undo_restores_before_state {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn prop_undo_restores_before_state(
                num_commits in 1usize..=5,
                undo_target in 0usize..5,
            ) {
                // Create a temp repo with an initial commit.
                let (_dir, repo, initial_oid) = setup_repo();

                // Build a chain of commits, recording snapshots along the way.
                let mut oids = vec![initial_oid];
                for i in 0..num_commits {
                    let msg = format!("commit_{}", i);
                    let new_oid = make_commit(&repo, &msg);
                    oids.push(new_oid);
                }

                // Record each commit as an UndoEntry.
                let mut engine = UndoEngine::new();
                for i in 0..num_commits {
                    let before_oid = &oids[i];
                    let after_oid = &oids[i + 1];
                    engine.record(entry(
                        GitOperation::Commit { id: after_oid.clone() },
                        &format!("Commit {}", i),
                        snapshot(before_oid, Some("refs/heads/main")),
                        snapshot(after_oid, Some("refs/heads/main")),
                    ));
                }

                // Determine how many undos to perform (1..=num_commits).
                let undo_count = (undo_target % num_commits) + 1;

                // Perform undo_count undos.
                for _ in 0..undo_count {
                    engine.undo(&repo).unwrap();
                }

                // After undo_count undos from num_commits entries, the expected
                // HEAD should be oids[num_commits - undo_count].
                let expected_oid = &oids[num_commits - undo_count];

                // Verify HEAD matches the before_state snapshot.
                let head = repo.inner().head().unwrap();
                let actual_oid = head.target().unwrap().to_string();
                prop_assert_eq!(
                    &actual_oid,
                    expected_oid,
                    "After {} undo(s) from {} commits, HEAD should be at commit index {}",
                    undo_count,
                    num_commits,
                    num_commits - undo_count
                );

                // Verify the symbolic ref is still refs/heads/main.
                let head_name = head.name().map(|s| s.to_string());
                prop_assert_eq!(
                    head_name.as_deref(),
                    Some("refs/heads/main"),
                    "HEAD ref should still point to refs/heads/main after undo"
                );
            }
        }
    }
}
