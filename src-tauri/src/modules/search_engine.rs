use serde::{Deserialize, Serialize};

use crate::models::CommitInfo;

/// A single highlight indicating where a match was found.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHighlight {
    /// The field name: "message", "author", "email", "id"
    pub field: String,
    /// Start byte position of the match within the field value.
    pub start: usize,
    /// Length (in bytes) of the match.
    pub length: usize,
}

/// A commit together with its match highlights.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub commit: CommitInfo,
    pub highlights: Vec<SearchHighlight>,
}

/// Query parameters for in-memory commit search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SearchQuery {
    /// Free-text search across message, author name/email, and commit hash (case-insensitive).
    pub text: Option<String>,
    /// Filter by author name or email (case-insensitive substring match).
    pub author: Option<String>,
    /// Epoch timestamp lower bound (inclusive) on author timestamp.
    pub since: Option<i64>,
    /// Epoch timestamp upper bound (inclusive) on author timestamp.
    pub until: Option<i64>,
    /// File path filter – not applicable for in-memory search, ignored.
    pub path: Option<String>,
    /// Filter by commit hash prefix (case-insensitive).
    pub commit_hash: Option<String>,
}

pub struct SearchEngine;

impl SearchEngine {
    pub fn new() -> Self {
        Self
    }

    /// Search `commits` using the given `query`. All non-None criteria must match (AND logic).
    /// Returns matching commits with highlight positions.
    pub fn search(&self, commits: &[CommitInfo], query: &SearchQuery) -> Vec<SearchResult> {
        commits
            .iter()
            .filter_map(|c| self.match_commit(c, query))
            .collect()
    }

    fn match_commit(&self, commit: &CommitInfo, query: &SearchQuery) -> Option<SearchResult> {
        let mut highlights = Vec::new();

        // --- text filter ---
        if let Some(ref text) = query.text {
            let needle = text.to_lowercase();
            let mut found = false;

            // message
            find_all_case_insensitive(&commit.message, &needle)
                .into_iter()
                .for_each(|start| {
                    found = true;
                    highlights.push(SearchHighlight {
                        field: "message".into(),
                        start,
                        length: text.len(),
                    });
                });

            // author name
            find_all_case_insensitive(&commit.author.name, &needle)
                .into_iter()
                .for_each(|start| {
                    found = true;
                    highlights.push(SearchHighlight {
                        field: "author".into(),
                        start,
                        length: text.len(),
                    });
                });

            // author email
            find_all_case_insensitive(&commit.author.email, &needle)
                .into_iter()
                .for_each(|start| {
                    found = true;
                    highlights.push(SearchHighlight {
                        field: "email".into(),
                        start,
                        length: text.len(),
                    });
                });

            // commit id
            find_all_case_insensitive(&commit.id, &needle)
                .into_iter()
                .for_each(|start| {
                    found = true;
                    highlights.push(SearchHighlight {
                        field: "id".into(),
                        start,
                        length: text.len(),
                    });
                });

            if !found {
                return None;
            }
        }

        // --- author filter ---
        if let Some(ref author) = query.author {
            let needle = author.to_lowercase();
            let name_lower = commit.author.name.to_lowercase();
            let email_lower = commit.author.email.to_lowercase();
            if !name_lower.contains(&needle) && !email_lower.contains(&needle) {
                return None;
            }
        }

        // --- date range filter ---
        if let Some(since) = query.since {
            if commit.author.timestamp < since {
                return None;
            }
        }
        if let Some(until) = query.until {
            if commit.author.timestamp > until {
                return None;
            }
        }

        // --- commit hash prefix filter ---
        if let Some(ref hash) = query.commit_hash {
            let hash_lower = hash.to_lowercase();
            let id_lower = commit.id.to_lowercase();
            if !id_lower.starts_with(&hash_lower) {
                return None;
            }
        }

        Some(SearchResult {
            commit: commit.clone(),
            highlights,
        })
    }
}

/// Find all byte-offset positions of `needle` (already lowercased) in `haystack` (case-insensitive).
fn find_all_case_insensitive(haystack: &str, needle_lower: &str) -> Vec<usize> {
    if needle_lower.is_empty() {
        return vec![];
    }
    let hay_lower = haystack.to_lowercase();
    let mut positions = Vec::new();
    let mut start = 0;
    while let Some(pos) = hay_lower[start..].find(needle_lower) {
        positions.push(start + pos);
        start += pos + 1;
    }
    positions
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SignatureInfo;

    fn make_commit(id: &str, message: &str, author_name: &str, author_email: &str, timestamp: i64) -> CommitInfo {
        CommitInfo {
            id: id.to_string(),
            short_id: id[..7.min(id.len())].to_string(),
            message: message.to_string(),
            author: SignatureInfo {
                name: author_name.to_string(),
                email: author_email.to_string(),
                timestamp,
            },
            committer: SignatureInfo {
                name: author_name.to_string(),
                email: author_email.to_string(),
                timestamp,
            },
            parent_ids: vec![],
            refs: vec![],
            is_cherry_picked: false,
        }
    }

    fn sample_commits() -> Vec<CommitInfo> {
        vec![
            make_commit("abc1234567890", "Fix login bug", "Alice", "alice@example.com", 1000),
            make_commit("def4567890123", "Add search feature", "Bob", "bob@example.com", 2000),
            make_commit("ghi7890123456", "Refactor auth module", "Alice", "alice@example.com", 3000),
            make_commit("jkl0123456789", "Update README", "Charlie", "charlie@example.com", 4000),
        ]
    }

    #[test]
    fn text_search_in_message() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        let query = SearchQuery {
            text: Some("login".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit.id, "abc1234567890");
    }

    #[test]
    fn text_search_in_author() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        let query = SearchQuery {
            text: Some("bob".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit.id, "def4567890123");
    }

    #[test]
    fn author_filter() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        let query = SearchQuery {
            author: Some("alice".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.commit.author.name == "Alice"));
    }

    #[test]
    fn author_filter_by_email() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        let query = SearchQuery {
            author: Some("charlie@example".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit.author.name, "Charlie");
    }

    #[test]
    fn date_range_filter() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        let query = SearchQuery {
            since: Some(1500),
            until: Some(3500),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 2);
        let ids: Vec<&str> = results.iter().map(|r| r.commit.id.as_str()).collect();
        assert!(ids.contains(&"def4567890123"));
        assert!(ids.contains(&"ghi7890123456"));
    }

    #[test]
    fn commit_hash_prefix_filter() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        let query = SearchQuery {
            commit_hash: Some("def456".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit.id, "def4567890123");
    }

    #[test]
    fn combined_filters_and_logic() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        // text matches "auth" in message of commit ghi, author filter matches Alice
        let query = SearchQuery {
            text: Some("auth".into()),
            author: Some("alice".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit.id, "ghi7890123456");
    }

    #[test]
    fn combined_filters_no_match() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        // text matches "login" (commit abc by Alice), but author filter is Bob → no match
        let query = SearchQuery {
            text: Some("login".into()),
            author: Some("bob".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert!(results.is_empty());
    }

    #[test]
    fn highlight_positions_correct() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        let query = SearchQuery {
            text: Some("Fix".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 1);
        let hl: Vec<&SearchHighlight> = results[0]
            .highlights
            .iter()
            .filter(|h| h.field == "message")
            .collect();
        assert_eq!(hl.len(), 1);
        assert_eq!(hl[0].start, 0);
        assert_eq!(hl[0].length, 3);
    }

    #[test]
    fn case_insensitive_matching() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        let query = SearchQuery {
            text: Some("FIX LOGIN".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit.id, "abc1234567890");
    }

    #[test]
    fn case_insensitive_commit_hash() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        let query = SearchQuery {
            commit_hash: Some("DEF456".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn empty_query_returns_all() {
        let engine = SearchEngine::new();
        let commits = sample_commits();
        let query = SearchQuery::default();
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), commits.len());
    }

    #[test]
    fn multiple_highlights_in_same_field() {
        let engine = SearchEngine::new();
        let commits = vec![make_commit(
            "aaa1234567890",
            "fix bug and fix test",
            "Dev",
            "dev@test.com",
            100,
        )];
        let query = SearchQuery {
            text: Some("fix".into()),
            ..Default::default()
        };
        let results = engine.search(&commits, &query);
        assert_eq!(results.len(), 1);
        let msg_highlights: Vec<&SearchHighlight> = results[0]
            .highlights
            .iter()
            .filter(|h| h.field == "message")
            .collect();
        assert_eq!(msg_highlights.len(), 2);
        assert_eq!(msg_highlights[0].start, 0);
        assert_eq!(msg_highlights[1].start, 12);
    }

    // === Property-Based Tests ===

    use proptest::prelude::*;
    use std::collections::HashSet;

    /// Generate a hex string of exactly 12 characters (simulating a commit id).
    fn arb_hex_id() -> impl Strategy<Value = String> {
        proptest::collection::vec(prop::sample::select(
            &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'][..],
        ), 12..=12)
        .prop_map(|chars| chars.into_iter().collect::<String>())
    }

    /// Generate a simple ASCII string (1..30 chars) for messages/names.
    fn arb_ascii_text() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,30}"
    }

    /// Generate a simple email-like string.
    fn arb_email() -> impl Strategy<Value = String> {
        ("[a-z]{1,10}", "[a-z]{1,6}").prop_map(|(user, domain)| format!("{}@{}.com", user, domain))
    }

    /// Generate a CommitInfo with random fields.
    fn arb_commit_info() -> impl Strategy<Value = CommitInfo> {
        (arb_hex_id(), arb_ascii_text(), arb_ascii_text(), arb_email(), 0i64..100_000i64)
            .prop_map(|(id, message, author_name, author_email, timestamp)| {
                let short_id = id[..7].to_string();
                CommitInfo {
                    id,
                    short_id,
                    message,
                    author: SignatureInfo {
                        name: author_name.clone(),
                        email: author_email.clone(),
                        timestamp,
                    },
                    committer: SignatureInfo {
                        name: author_name,
                        email: author_email,
                        timestamp,
                    },
                    parent_ids: vec![],
                    refs: vec![],
                    is_cherry_picked: false,
                }
            })
    }

    /// Generate an optional filter string (short ASCII).
    fn arb_opt_filter() -> impl Strategy<Value = Option<String>> {
        prop::option::of("[a-zA-Z0-9]{1,8}")
    }

    /// Generate an optional hex prefix for commit_hash filter.
    fn arb_opt_hex_prefix() -> impl Strategy<Value = Option<String>> {
        prop::option::of(
            proptest::collection::vec(prop::sample::select(
                &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'][..],
            ), 1..=6)
            .prop_map(|chars| chars.into_iter().collect::<String>())
        )
    }

    /// Generate an optional timestamp bound.
    fn arb_opt_timestamp() -> impl Strategy<Value = Option<i64>> {
        prop::option::of(0i64..100_000i64)
    }

    /// Generate a SearchQuery with random combinations of filters.
    fn arb_search_query() -> impl Strategy<Value = SearchQuery> {
        (arb_opt_filter(), arb_opt_filter(), arb_opt_timestamp(), arb_opt_timestamp(), arb_opt_hex_prefix())
            .prop_map(|(text, author, since, until, commit_hash)| {
                SearchQuery {
                    text,
                    author,
                    since,
                    until,
                    path: None,
                    commit_hash,
                }
            })
    }

    /// Independently check if a commit matches a query (reference implementation).
    fn commit_matches_query(commit: &CommitInfo, query: &SearchQuery) -> bool {
        // text: case-insensitive substring match in message OR author name OR email OR id
        if let Some(ref text) = query.text {
            let needle = text.to_lowercase();
            let found = commit.message.to_lowercase().contains(&needle)
                || commit.author.name.to_lowercase().contains(&needle)
                || commit.author.email.to_lowercase().contains(&needle)
                || commit.id.to_lowercase().contains(&needle);
            if !found {
                return false;
            }
        }

        // author: case-insensitive substring match in author name OR email
        if let Some(ref author) = query.author {
            let needle = author.to_lowercase();
            let found = commit.author.name.to_lowercase().contains(&needle)
                || commit.author.email.to_lowercase().contains(&needle);
            if !found {
                return false;
            }
        }

        // since: author.timestamp >= since
        if let Some(since) = query.since {
            if commit.author.timestamp < since {
                return false;
            }
        }

        // until: author.timestamp <= until
        if let Some(until) = query.until {
            if commit.author.timestamp > until {
                return false;
            }
        }

        // commit_hash: commit id starts with hash prefix (case-insensitive)
        if let Some(ref hash) = query.commit_hash {
            if !commit.id.to_lowercase().starts_with(&hash.to_lowercase()) {
                return false;
            }
        }

        true
    }

    // **Validates: Requirements 2.12, 13.1, 13.2**
    proptest! {
        #[test]
        fn prop_search_completeness(
            commits in proptest::collection::vec(arb_commit_info(), 1..20),
            query in arb_search_query(),
        ) {
            let engine = SearchEngine::new();
            let results = engine.search(&commits, &query);

            // Compute expected matching set independently
            let expected_ids: HashSet<String> = commits
                .iter()
                .filter(|c| commit_matches_query(c, &query))
                .map(|c| c.id.clone())
                .collect();

            let actual_ids: HashSet<String> = results
                .iter()
                .map(|r| r.commit.id.clone())
                .collect();

            prop_assert_eq!(actual_ids, expected_ids);
        }
    }
}
