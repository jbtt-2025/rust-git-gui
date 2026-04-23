use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::{SignatureInfo, TagInfo};

pub struct TagManager;

impl TagManager {
    pub fn new() -> Self {
        Self
    }

    /// List all tags with their target commit IDs.
    pub fn list_tags(&self, repo: &GitRepository) -> Result<Vec<TagInfo>, GitError> {
        let inner = repo.inner();
        let mut results = Vec::new();

        inner.tag_foreach(|oid, name_bytes| {
            let full_name = String::from_utf8_lossy(name_bytes).to_string();
            let name = full_name
                .strip_prefix("refs/tags/")
                .unwrap_or(&full_name)
                .to_string();

            if let Ok(obj) = inner.find_object(oid, None) {
                match obj.kind() {
                    Some(git2::ObjectType::Tag) => {
                        // Annotated tag
                        if let Ok(tag) = obj.peel_to_commit() {
                            let (message, tagger) = if let Ok(tag_obj) =
                                inner.find_tag(oid)
                            {
                                let msg =
                                    tag_obj.message().map(|m| m.to_string());
                                let tagger_info =
                                    tag_obj.tagger().map(|sig| SignatureInfo {
                                        name: sig.name().unwrap_or("").to_string(),
                                        email: sig.email().unwrap_or("").to_string(),
                                        timestamp: sig.when().seconds(),
                                    });
                                (msg, tagger_info)
                            } else {
                                (None, None)
                            };

                            results.push(TagInfo {
                                name,
                                target_commit_id: tag.id().to_string(),
                                is_annotated: true,
                                message,
                                tagger,
                            });
                        }
                    }
                    _ => {
                        // Lightweight tag (points directly to a commit)
                        let commit_id = if let Ok(commit) = obj.peel_to_commit() {
                            commit.id().to_string()
                        } else {
                            oid.to_string()
                        };

                        results.push(TagInfo {
                            name,
                            target_commit_id: commit_id,
                            is_annotated: false,
                            message: None,
                            tagger: None,
                        });
                    }
                }
            }
            true // continue iteration
        })?;

        results.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(results)
    }

    /// Create a lightweight tag pointing at a target commit (or HEAD if None).
    pub fn create_lightweight_tag(
        &self,
        repo: &GitRepository,
        name: &str,
        target: Option<&str>,
    ) -> Result<TagInfo, GitError> {
        let inner = repo.inner();
        let commit = self.resolve_target(inner, target)?;

        inner
            .tag_lightweight(name, commit.as_object(), false)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        Ok(TagInfo {
            name: name.to_string(),
            target_commit_id: commit.id().to_string(),
            is_annotated: false,
            message: None,
            tagger: None,
        })
    }

    /// Create an annotated tag with a message.
    pub fn create_annotated_tag(
        &self,
        repo: &GitRepository,
        name: &str,
        target: Option<&str>,
        message: &str,
    ) -> Result<TagInfo, GitError> {
        let inner = repo.inner();
        let commit = self.resolve_target(inner, target)?;

        let sig = inner.signature().map_err(|e| {
            GitError::InvalidArgument(format!("No signature configured: {}", e))
        })?;

        inner
            .tag(name, commit.as_object(), &sig, message, false)
            .map_err(|e| GitError::Git2(e.message().to_string()))?;

        Ok(TagInfo {
            name: name.to_string(),
            target_commit_id: commit.id().to_string(),
            is_annotated: true,
            message: Some(message.to_string()),
            tagger: Some(SignatureInfo {
                name: sig.name().unwrap_or("").to_string(),
                email: sig.email().unwrap_or("").to_string(),
                timestamp: sig.when().seconds(),
            }),
        })
    }

    /// Delete a tag by name.
    pub fn delete_tag(
        &self,
        repo: &GitRepository,
        name: &str,
    ) -> Result<(), GitError> {
        let inner = repo.inner();
        inner.tag_delete(name).map_err(|e| {
            GitError::InvalidArgument(format!("Failed to delete tag '{}': {}", name, e.message()))
        })?;
        Ok(())
    }

    // --- Private helpers ---

    fn resolve_target<'r>(
        &self,
        inner: &'r git2::Repository,
        target: Option<&str>,
    ) -> Result<git2::Commit<'r>, GitError> {
        match target {
            Some(rev) => {
                let obj = inner.revparse_single(rev).map_err(|_| {
                    GitError::InvalidArgument(format!("Invalid revision: {}", rev))
                })?;
                obj.peel_to_commit().map_err(|_| {
                    GitError::InvalidArgument(format!(
                        "Cannot resolve to commit: {}",
                        rev
                    ))
                })
            }
            None => {
                let head = inner.head()?;
                head.peel_to_commit().map_err(|_| {
                    GitError::InvalidArgument(
                        "HEAD does not point to a commit".to_string(),
                    )
                })
            }
        }
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
    fn test_list_tags_empty() {
        let (_dir, repo) = setup_repo();
        let mgr = TagManager::new();

        let tags = mgr.list_tags(&repo).unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn test_create_lightweight_tag() {
        let (_dir, repo) = setup_repo();
        let mgr = TagManager::new();

        let tag = mgr.create_lightweight_tag(&repo, "v1.0", None).unwrap();
        assert_eq!(tag.name, "v1.0");
        assert!(!tag.is_annotated);
        assert!(tag.message.is_none());
        assert!(tag.tagger.is_none());
        assert!(!tag.target_commit_id.is_empty());
    }

    #[test]
    fn test_create_annotated_tag() {
        let (_dir, repo) = setup_repo();
        let mgr = TagManager::new();

        let tag = mgr
            .create_annotated_tag(&repo, "v2.0", None, "Release 2.0")
            .unwrap();
        assert_eq!(tag.name, "v2.0");
        assert!(tag.is_annotated);
        assert_eq!(tag.message.as_deref(), Some("Release 2.0"));
        assert!(tag.tagger.is_some());
        let tagger = tag.tagger.unwrap();
        assert_eq!(tagger.name, "Test User");
        assert_eq!(tagger.email, "test@example.com");
    }

    #[test]
    fn test_create_tag_at_specific_commit() {
        let (_dir, repo) = setup_repo();
        add_file_and_commit(&repo, "a.txt", "a", "Second commit");

        let mgr = TagManager::new();

        // Get the first commit id (parent of HEAD)
        let head_oid = repo.inner().head().unwrap().target().unwrap();
        let head_commit = repo.inner().find_commit(head_oid).unwrap();
        let parent_id = head_commit.parent_id(0).unwrap().to_string();

        let tag = mgr
            .create_lightweight_tag(&repo, "v0.1", Some(&parent_id))
            .unwrap();
        assert_eq!(tag.target_commit_id, parent_id);
    }

    #[test]
    fn test_list_tags_after_creation() {
        let (_dir, repo) = setup_repo();
        let mgr = TagManager::new();

        mgr.create_lightweight_tag(&repo, "v1.0", None).unwrap();
        mgr.create_annotated_tag(&repo, "v2.0", None, "Release 2.0")
            .unwrap();

        let tags = mgr.list_tags(&repo).unwrap();
        assert_eq!(tags.len(), 2);

        // Tags should be sorted by name
        assert_eq!(tags[0].name, "v1.0");
        assert!(!tags[0].is_annotated);
        assert_eq!(tags[1].name, "v2.0");
        assert!(tags[1].is_annotated);
    }

    #[test]
    fn test_delete_tag() {
        let (_dir, repo) = setup_repo();
        let mgr = TagManager::new();

        mgr.create_lightweight_tag(&repo, "to-delete", None).unwrap();
        assert_eq!(mgr.list_tags(&repo).unwrap().len(), 1);

        mgr.delete_tag(&repo, "to-delete").unwrap();
        assert!(mgr.list_tags(&repo).unwrap().is_empty());
    }

    #[test]
    fn test_delete_annotated_tag() {
        let (_dir, repo) = setup_repo();
        let mgr = TagManager::new();

        mgr.create_annotated_tag(&repo, "annotated-del", None, "msg")
            .unwrap();
        mgr.delete_tag(&repo, "annotated-del").unwrap();
        assert!(mgr.list_tags(&repo).unwrap().is_empty());
    }

    #[test]
    fn test_delete_nonexistent_tag_fails() {
        let (_dir, repo) = setup_repo();
        let mgr = TagManager::new();

        let result = mgr.delete_tag(&repo, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_duplicate_tag_fails() {
        let (_dir, repo) = setup_repo();
        let mgr = TagManager::new();

        mgr.create_lightweight_tag(&repo, "dup", None).unwrap();
        let result = mgr.create_lightweight_tag(&repo, "dup", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_tag_invalid_target_fails() {
        let (_dir, repo) = setup_repo();
        let mgr = TagManager::new();

        let result = mgr.create_lightweight_tag(&repo, "bad", Some("nonexistent_ref"));
        assert!(result.is_err());
    }
}
