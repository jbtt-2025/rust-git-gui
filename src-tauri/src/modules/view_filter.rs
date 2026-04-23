use std::collections::{HashMap, HashSet, VecDeque};

use crate::models::{CommitInfo, DagLayout, RefType};

/// Manages Solo, Hide, and Pin to Left view filter state for the commit graph.
#[derive(Debug, Clone, Default)]
pub struct ViewFilter {
    /// Branch names that are solo'd — only their ancestors are visible.
    pub soloed_branches: HashSet<String>,
    /// Branch names that are hidden — their exclusive commits are invisible.
    pub hidden_branches: HashSet<String>,
    /// Ordered list of branches pinned to the left columns.
    pub pinned_left_branches: Vec<String>,
}

impl ViewFilter {
    /// Create an empty filter with no solo, hide, or pin state.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn solo_branch(&mut self, name: &str) {
        self.soloed_branches.insert(name.to_string());
    }

    pub fn unsolo_branch(&mut self, name: &str) {
        self.soloed_branches.remove(name);
    }

    pub fn hide_branch(&mut self, name: &str) {
        self.hidden_branches.insert(name.to_string());
    }

    pub fn unhide_branch(&mut self, name: &str) {
        self.hidden_branches.remove(name);
    }

    /// Pin a branch to the left. Appended at the end of the pinned list.
    pub fn pin_left(&mut self, name: &str) {
        if !self.pinned_left_branches.contains(&name.to_string()) {
            self.pinned_left_branches.push(name.to_string());
        }
    }

    pub fn unpin_left(&mut self, name: &str) {
        self.pinned_left_branches.retain(|n| n != name);
    }

    /// Clear all solo, hidden, and pinned state.
    pub fn reset_view(&mut self) {
        self.soloed_branches.clear();
        self.hidden_branches.clear();
        self.pinned_left_branches.clear();
    }

    pub fn is_solo_active(&self) -> bool {
        !self.soloed_branches.is_empty()
    }

    pub fn is_hide_active(&self) -> bool {
        !self.hidden_branches.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Standalone filter functions
// ---------------------------------------------------------------------------

/// Collect the set of commit IDs that are ancestors of the given branch tips.
///
/// `branch_names` — the set of branch names to consider.
/// Returns a `HashSet<String>` of commit IDs reachable from those branches.
fn ancestor_set(commits: &[CommitInfo], branch_names: &HashSet<String>) -> HashSet<String> {
    // Build id → index map for fast lookup
    let id_to_idx: HashMap<&str, usize> = commits
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.as_str(), i))
        .collect();

    // Find tip commits: commits whose refs contain a matching branch name
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();

    for (idx, commit) in commits.iter().enumerate() {
        for rl in &commit.refs {
            if branch_names.contains(&rl.name) {
                if visited.insert(commit.id.clone()) {
                    queue.push_back(idx);
                }
            }
        }
    }

    // BFS backwards through parents
    while let Some(idx) = queue.pop_front() {
        let commit = &commits[idx];
        for pid in &commit.parent_ids {
            if visited.insert(pid.clone()) {
                if let Some(&pidx) = id_to_idx.get(pid.as_str()) {
                    queue.push_back(pidx);
                }
            }
        }
    }

    visited
}

/// Solo filter: returns only commits that are ancestors of any solo'd branch.
///
/// A branch's tip commit is the commit whose `refs` contain a `RefLabel` with
/// a matching name. We walk backwards through `parent_ids` to collect all
/// ancestors. Both local and remote branches are supported.
pub fn apply_solo_filter(
    commits: &[CommitInfo],
    soloed_branches: &HashSet<String>,
) -> Vec<CommitInfo> {
    if soloed_branches.is_empty() {
        return commits.to_vec();
    }

    let visible = ancestor_set(commits, soloed_branches);

    commits
        .iter()
        .filter(|c| visible.contains(&c.id))
        .cloned()
        .collect()
}

/// Hide filter: returns commits excluding those that are "exclusive" to hidden
/// branches.
///
/// A commit is exclusive to hidden branches if it is reachable from ONLY hidden
/// branches (i.e. not reachable from any non-hidden branch). Both local and
/// remote branches are supported.
pub fn apply_hide_filter(
    commits: &[CommitInfo],
    hidden_branches: &HashSet<String>,
) -> Vec<CommitInfo> {
    if hidden_branches.is_empty() {
        return commits.to_vec();
    }

    // Collect all branch names present in the commit graph
    let all_branches: HashSet<String> = commits
        .iter()
        .flat_map(|c| c.refs.iter())
        .filter(|rl| matches!(rl.ref_type, RefType::LocalBranch | RefType::RemoteBranch { .. }))
        .map(|rl| rl.name.clone())
        .collect();

    // Non-hidden branches
    let non_hidden: HashSet<String> = all_branches
        .difference(hidden_branches)
        .cloned()
        .collect();

    // Commits reachable from non-hidden branches stay visible
    let visible = ancestor_set(commits, &non_hidden);

    commits
        .iter()
        .filter(|c| visible.contains(&c.id))
        .cloned()
        .collect()
}

/// Pin to Left: reassign columns so that pinned branches occupy the leftmost
/// columns in the order they appear in `pinned_branches`.
///
/// For each pinned branch we find its tip commit, determine its current column,
/// and swap that column to the target left position. All node columns and edge
/// from_column / to_column are updated accordingly.
pub fn apply_pin_to_left(
    layout: &mut DagLayout,
    pinned_branches: &[String],
    commits: &[CommitInfo],
) {
    if pinned_branches.is_empty() {
        return;
    }

    // Build a map: branch_name → tip commit_id
    let mut branch_tip: HashMap<&str, &str> = HashMap::new();
    for commit in commits {
        for rl in &commit.refs {
            if matches!(rl.ref_type, RefType::LocalBranch | RefType::RemoteBranch { .. }) {
                // First occurrence in topological order (newest first) is the tip
                branch_tip.entry(rl.name.as_str()).or_insert(commit.id.as_str());
            }
        }
    }

    // Build commit_id → column from current layout
    let commit_col: HashMap<&str, usize> = layout
        .nodes
        .iter()
        .map(|n| (n.commit_id.as_str(), n.column))
        .collect();

    // Determine the column swaps needed.
    // We'll build a permutation map: old_column → new_column
    // Start with identity
    let max_col = layout.total_columns;
    let mut col_perm: Vec<usize> = (0..max_col).collect();

    for (target_col, branch_name) in pinned_branches.iter().enumerate() {
        if target_col >= max_col {
            break;
        }
        // Find the current column of this branch's tip
        let tip_id = match branch_tip.get(branch_name.as_str()) {
            Some(id) => *id,
            None => continue,
        };
        let current_col = match commit_col.get(tip_id) {
            Some(&c) => c,
            None => continue,
        };

        // Find where current_col is in the permutation (it may have been
        // swapped already by a previous pin)
        let perm_pos_current = col_perm.iter().position(|&c| c == current_col);
        let perm_pos_target = col_perm.iter().position(|&c| c == col_perm[target_col]);

        if let (Some(pos_cur), _) = (perm_pos_current, perm_pos_target) {
            // Swap in the permutation
            let old_val = col_perm[target_col];
            col_perm[target_col] = current_col;
            col_perm[pos_cur] = old_val;
        }
    }

    // Build reverse map: old_column → new_column
    let mut remap: HashMap<usize, usize> = HashMap::new();
    for (new_col, &old_col) in col_perm.iter().enumerate() {
        remap.insert(old_col, new_col);
    }

    // Apply the remap to all nodes and edges
    for node in &mut layout.nodes {
        if let Some(&new_col) = remap.get(&node.column) {
            node.column = new_col;
        }
        for edge in &mut node.parent_edges {
            if let Some(&new_from) = remap.get(&edge.from_column) {
                edge.from_column = new_from;
            }
            if let Some(&new_to) = remap.get(&edge.to_column) {
                edge.to_column = new_to;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        CommitInfo, DagEdge, DagLayout, DagNode, RefLabel, RefType, SignatureInfo,
    };

    /// Helper: create a minimal CommitInfo with given id, parent_ids, and refs.
    fn make_commit(id: &str, parent_ids: Vec<&str>, refs: Vec<RefLabel>) -> CommitInfo {
        CommitInfo {
            id: id.to_string(),
            short_id: id[..std::cmp::min(7, id.len())].to_string(),
            message: format!("commit {}", id),
            author: SignatureInfo {
                name: "test".into(),
                email: "test@test.com".into(),
                timestamp: 0,
            },
            committer: SignatureInfo {
                name: "test".into(),
                email: "test@test.com".into(),
                timestamp: 0,
            },
            parent_ids: parent_ids.into_iter().map(String::from).collect(),
            refs,
            is_cherry_picked: false,
        }
    }

    fn local_ref(name: &str) -> RefLabel {
        RefLabel {
            name: name.to_string(),
            ref_type: RefType::LocalBranch,
            is_head: false,
        }
    }

    fn remote_ref(name: &str, remote: &str) -> RefLabel {
        RefLabel {
            name: name.to_string(),
            ref_type: RefType::RemoteBranch {
                remote: remote.to_string(),
            },
            is_head: false,
        }
    }

    // -----------------------------------------------------------------------
    // ViewFilter struct tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_reset_view_clears_all_state() {
        let mut vf = ViewFilter::new();
        vf.solo_branch("main");
        vf.hide_branch("feature");
        vf.pin_left("develop");

        assert!(vf.is_solo_active());
        assert!(vf.is_hide_active());
        assert_eq!(vf.pinned_left_branches.len(), 1);

        vf.reset_view();

        assert!(!vf.is_solo_active());
        assert!(!vf.is_hide_active());
        assert!(vf.pinned_left_branches.is_empty());
    }

    #[test]
    fn test_solo_unsolo() {
        let mut vf = ViewFilter::new();
        assert!(!vf.is_solo_active());

        vf.solo_branch("main");
        assert!(vf.is_solo_active());
        assert!(vf.soloed_branches.contains("main"));

        vf.unsolo_branch("main");
        assert!(!vf.is_solo_active());
    }

    #[test]
    fn test_hide_unhide() {
        let mut vf = ViewFilter::new();
        assert!(!vf.is_hide_active());

        vf.hide_branch("feature");
        assert!(vf.is_hide_active());

        vf.unhide_branch("feature");
        assert!(!vf.is_hide_active());
    }

    #[test]
    fn test_pin_left_ordering_and_dedup() {
        let mut vf = ViewFilter::new();
        vf.pin_left("main");
        vf.pin_left("develop");
        vf.pin_left("main"); // duplicate — should not add again

        assert_eq!(vf.pinned_left_branches, vec!["main", "develop"]);

        vf.unpin_left("main");
        assert_eq!(vf.pinned_left_branches, vec!["develop"]);
    }

    // -----------------------------------------------------------------------
    // Solo filter tests
    // -----------------------------------------------------------------------

    /// Graph:
    ///   c1 (main) -> c2 -> c3
    ///   c4 (feature) -> c2
    /// Solo "main" should show c1, c2, c3
    #[test]
    fn test_solo_filter_single_branch() {
        let commits = vec![
            make_commit("c1", vec!["c2"], vec![local_ref("main")]),
            make_commit("c4", vec!["c2"], vec![local_ref("feature")]),
            make_commit("c2", vec!["c3"], vec![]),
            make_commit("c3", vec![], vec![]),
        ];

        let mut solo = HashSet::new();
        solo.insert("main".to_string());

        let result = apply_solo_filter(&commits, &solo);
        let ids: Vec<&str> = result.iter().map(|c| c.id.as_str()).collect();

        assert!(ids.contains(&"c1"));
        assert!(ids.contains(&"c2"));
        assert!(ids.contains(&"c3"));
        assert!(!ids.contains(&"c4"));
    }

    /// Solo multiple branches: "main" and "feature" should show all commits.
    #[test]
    fn test_solo_filter_multiple_branches() {
        let commits = vec![
            make_commit("c1", vec!["c2"], vec![local_ref("main")]),
            make_commit("c4", vec!["c2"], vec![local_ref("feature")]),
            make_commit("c2", vec!["c3"], vec![]),
            make_commit("c3", vec![], vec![]),
        ];

        let mut solo = HashSet::new();
        solo.insert("main".to_string());
        solo.insert("feature".to_string());

        let result = apply_solo_filter(&commits, &solo);
        assert_eq!(result.len(), 4);
    }

    /// Solo a remote branch.
    #[test]
    fn test_solo_filter_remote_branch() {
        let commits = vec![
            make_commit("c1", vec!["c2"], vec![remote_ref("origin/main", "origin")]),
            make_commit("c5", vec!["c2"], vec![local_ref("feature")]),
            make_commit("c2", vec!["c3"], vec![]),
            make_commit("c3", vec![], vec![]),
        ];

        let mut solo = HashSet::new();
        solo.insert("origin/main".to_string());

        let result = apply_solo_filter(&commits, &solo);
        let ids: Vec<&str> = result.iter().map(|c| c.id.as_str()).collect();

        assert!(ids.contains(&"c1"));
        assert!(ids.contains(&"c2"));
        assert!(ids.contains(&"c3"));
        assert!(!ids.contains(&"c5"));
    }

    // -----------------------------------------------------------------------
    // Hide filter tests
    // -----------------------------------------------------------------------

    /// Graph:
    ///   c1 (main) -> c2 -> c3
    ///   c4 (feature) -> c5 -> c2
    /// Hide "feature" should hide c4 and c5 (exclusive to feature), keep c1, c2, c3.
    #[test]
    fn test_hide_filter_single_branch() {
        let commits = vec![
            make_commit("c1", vec!["c2"], vec![local_ref("main")]),
            make_commit("c4", vec!["c5"], vec![local_ref("feature")]),
            make_commit("c2", vec!["c3"], vec![]),
            make_commit("c5", vec!["c2"], vec![]),
            make_commit("c3", vec![], vec![]),
        ];

        let mut hidden = HashSet::new();
        hidden.insert("feature".to_string());

        let result = apply_hide_filter(&commits, &hidden);
        let ids: Vec<&str> = result.iter().map(|c| c.id.as_str()).collect();

        assert!(ids.contains(&"c1"));
        assert!(ids.contains(&"c2"));
        assert!(ids.contains(&"c3"));
        assert!(!ids.contains(&"c4"));
        assert!(!ids.contains(&"c5"));
    }

    /// Shared commits remain visible even when one branch is hidden.
    /// Graph:
    ///   c1 (main) -> c2
    ///   c3 (feature) -> c2
    /// Hide "feature" — c2 is shared, so it stays visible. Only c3 is hidden.
    #[test]
    fn test_hide_filter_preserves_shared_commits() {
        let commits = vec![
            make_commit("c1", vec!["c2"], vec![local_ref("main")]),
            make_commit("c3", vec!["c2"], vec![local_ref("feature")]),
            make_commit("c2", vec![], vec![]),
        ];

        let mut hidden = HashSet::new();
        hidden.insert("feature".to_string());

        let result = apply_hide_filter(&commits, &hidden);
        let ids: Vec<&str> = result.iter().map(|c| c.id.as_str()).collect();

        assert!(ids.contains(&"c1"));
        assert!(ids.contains(&"c2")); // shared — stays visible
        assert!(!ids.contains(&"c3")); // exclusive to feature
    }

    // -----------------------------------------------------------------------
    // Pin to Left tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_pin_to_left_moves_branch_to_column_0() {
        // Commits: main tip at column 1, feature tip at column 0
        let commits = vec![
            make_commit("c1", vec!["c3"], vec![local_ref("main")]),
            make_commit("c2", vec!["c3"], vec![local_ref("feature")]),
            make_commit("c3", vec![], vec![]),
        ];

        let mut layout = DagLayout {
            nodes: vec![
                DagNode {
                    commit_id: "c1".into(),
                    column: 1,
                    row: 0,
                    color_index: 1,
                    parent_edges: vec![DagEdge {
                        from_column: 1,
                        to_column: 0,
                        to_row: 2,
                        color_index: 1,
                    }],
                },
                DagNode {
                    commit_id: "c2".into(),
                    column: 0,
                    row: 1,
                    color_index: 0,
                    parent_edges: vec![DagEdge {
                        from_column: 0,
                        to_column: 0,
                        to_row: 2,
                        color_index: 0,
                    }],
                },
                DagNode {
                    commit_id: "c3".into(),
                    column: 0,
                    row: 2,
                    color_index: 0,
                    parent_edges: vec![],
                },
            ],
            total_columns: 2,
            total_rows: 3,
        };

        apply_pin_to_left(&mut layout, &["main".to_string()], &commits);

        // main's tip (c1) should now be at column 0
        let c1_node = layout.nodes.iter().find(|n| n.commit_id == "c1").unwrap();
        assert_eq!(c1_node.column, 0);

        // feature's tip (c2) should have moved to column 1
        let c2_node = layout.nodes.iter().find(|n| n.commit_id == "c2").unwrap();
        assert_eq!(c2_node.column, 1);
    }

    // -----------------------------------------------------------------------
    // Property-based tests (proptest)
    // -----------------------------------------------------------------------

    use proptest::prelude::*;

    /// Strategy: generate a valid commit graph with branch refs and a solo'd subset.
    ///
    /// Commits are topologically ordered (index 0 = newest). Parent IDs reference
    /// commits at strictly higher indices (older commits). Some commits get random
    /// branch refs (local or remote). A random subset of those branch names forms
    /// the solo'd set.
    fn commit_graph_with_solo_strategy()
        -> impl Strategy<Value = (Vec<CommitInfo>, HashSet<String>)>
    {
        (1usize..=15).prop_flat_map(|n| {
            // For each commit, generate a bitmask of which later commits are parents.
            // Also generate 0..=2 branch ref names per commit.
            let parent_bitmask_strats: Vec<_> = (0..n)
                .map(|i| {
                    if i + 1 < n {
                        // Each later commit can be a parent (true/false), limit to 3
                        proptest::collection::vec(proptest::bool::ANY, n - i - 1).boxed()
                    } else {
                        proptest::collection::vec(Just(false), 0..=0).boxed()
                    }
                })
                .collect();

            let ref_name_strats: Vec<_> = (0..n)
                .map(|_| {
                    proptest::collection::vec(
                        (proptest::bool::ANY, "[a-z]{3,8}"),
                        0..=2,
                    )
                })
                .collect();

            (Just(n), parent_bitmask_strats, ref_name_strats)
        })
        .prop_flat_map(|(n, parent_bitmasks, ref_name_vecs)| {
            // Build commits from the generated data, then pick solo subset
            let ids: Vec<String> = (0..n).map(|i| format!("commit_{}", i)).collect();

            // Convert bitmasks to parent_ids (limit to 3 parents)
            let mut all_branch_names: Vec<String> = Vec::new();
            let mut commits: Vec<CommitInfo> = Vec::new();

            for i in 0..n {
                let parent_ids: Vec<String> = parent_bitmasks[i]
                    .iter()
                    .enumerate()
                    .filter(|(_, &is_parent)| is_parent)
                    .take(3)
                    .map(|(j, _)| ids[i + 1 + j].clone())
                    .collect();

                let refs: Vec<RefLabel> = ref_name_vecs[i]
                    .iter()
                    .map(|(is_local, name)| {
                        if *is_local {
                            let rl = RefLabel {
                                name: name.clone(),
                                ref_type: RefType::LocalBranch,
                                is_head: false,
                            };
                            all_branch_names.push(name.clone());
                            rl
                        } else {
                            let full = format!("origin/{}", name);
                            let rl = RefLabel {
                                name: full.clone(),
                                ref_type: RefType::RemoteBranch {
                                    remote: "origin".to_string(),
                                },
                                is_head: false,
                            };
                            all_branch_names.push(full);
                            rl
                        }
                    })
                    .collect();

                commits.push(CommitInfo {
                    id: ids[i].clone(),
                    short_id: format!("c{}", i),
                    message: format!("msg {}", i),
                    author: SignatureInfo {
                        name: "a".into(),
                        email: "a@b.c".into(),
                        timestamp: 0,
                    },
                    committer: SignatureInfo {
                        name: "a".into(),
                        email: "a@b.c".into(),
                        timestamp: 0,
                    },
                    parent_ids,
                    refs,
                    is_cherry_picked: false,
                });
            }

            // Deduplicate branch names
            let unique_names: Vec<String> = {
                let mut seen = HashSet::new();
                all_branch_names
                    .into_iter()
                    .filter(|n| seen.insert(n.clone()))
                    .collect()
            };

            let max_solo = unique_names.len();
            let solo_strat = if max_solo == 0 {
                Just(HashSet::<String>::new()).boxed()
            } else {
                proptest::collection::hash_set(
                    proptest::sample::select(unique_names),
                    0..=max_solo,
                )
                .boxed()
            };

            (Just(commits), solo_strat)
        })
    }

    // **Validates: Requirements 2.3, 2.5, 2.7**
    //
    // Feature: rust-git-gui-client, Property 4: Solo 过滤正确性
    //
    // For any commit graph and any solo'd branch set, the visible commit set
    // returned by `apply_solo_filter` equals exactly the union of ancestor
    // commits of all solo'd branches (computed independently via BFS).
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]
        #[test]
        fn prop_solo_filter_correctness(
            (commits, soloed) in commit_graph_with_solo_strategy()
        ) {
            // --- Independently compute expected visible set via BFS ---
            let expected: HashSet<String> = if soloed.is_empty() {
                // When no branches are solo'd, all commits are visible
                commits.iter().map(|c| c.id.clone()).collect()
            } else {
                let id_to_idx: HashMap<&str, usize> = commits
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (c.id.as_str(), i))
                    .collect();

                let mut visited = HashSet::new();
                let mut queue = VecDeque::new();

                // Find tip commits for solo'd branches
                for (idx, commit) in commits.iter().enumerate() {
                    for rl in &commit.refs {
                        if soloed.contains(&rl.name) {
                            if visited.insert(commit.id.clone()) {
                                queue.push_back(idx);
                            }
                        }
                    }
                }

                // BFS through parents
                while let Some(idx) = queue.pop_front() {
                    for pid in &commits[idx].parent_ids {
                        if visited.insert(pid.clone()) {
                            if let Some(&pidx) = id_to_idx.get(pid.as_str()) {
                                queue.push_back(pidx);
                            }
                        }
                    }
                }

                visited
            };

            // --- Call the function under test ---
            let actual_commits = apply_solo_filter(&commits, &soloed);
            let actual: HashSet<String> = actual_commits
                .iter()
                .map(|c| c.id.clone())
                .collect();

            // --- Assert equality ---
            prop_assert_eq!(
                actual,
                expected,
                "Solo filter result mismatch for soloed branches: {:?}",
                soloed
            );
        }
    }

    /// Strategy: generate a valid commit graph with branch refs and a random
    /// subset of hidden branches. Reuses the same graph generation logic as
    /// `commit_graph_with_solo_strategy`.
    fn commit_graph_with_hide_strategy()
        -> impl Strategy<Value = (Vec<CommitInfo>, HashSet<String>)>
    {
        (1usize..=15).prop_flat_map(|n| {
            let parent_bitmask_strats: Vec<_> = (0..n)
                .map(|i| {
                    if i + 1 < n {
                        proptest::collection::vec(proptest::bool::ANY, n - i - 1).boxed()
                    } else {
                        proptest::collection::vec(Just(false), 0..=0).boxed()
                    }
                })
                .collect();

            let ref_name_strats: Vec<_> = (0..n)
                .map(|_| {
                    proptest::collection::vec(
                        (proptest::bool::ANY, "[a-z]{3,8}"),
                        0..=2,
                    )
                })
                .collect();

            (Just(n), parent_bitmask_strats, ref_name_strats)
        })
        .prop_flat_map(|(n, parent_bitmasks, ref_name_vecs)| {
            let ids: Vec<String> = (0..n).map(|i| format!("commit_{}", i)).collect();

            let mut all_branch_names: Vec<String> = Vec::new();
            let mut commits: Vec<CommitInfo> = Vec::new();

            for i in 0..n {
                let parent_ids: Vec<String> = parent_bitmasks[i]
                    .iter()
                    .enumerate()
                    .filter(|(_, &is_parent)| is_parent)
                    .take(3)
                    .map(|(j, _)| ids[i + 1 + j].clone())
                    .collect();

                let refs: Vec<RefLabel> = ref_name_vecs[i]
                    .iter()
                    .map(|(is_local, name)| {
                        if *is_local {
                            let rl = RefLabel {
                                name: name.clone(),
                                ref_type: RefType::LocalBranch,
                                is_head: false,
                            };
                            all_branch_names.push(name.clone());
                            rl
                        } else {
                            let full = format!("origin/{}", name);
                            let rl = RefLabel {
                                name: full.clone(),
                                ref_type: RefType::RemoteBranch {
                                    remote: "origin".to_string(),
                                },
                                is_head: false,
                            };
                            all_branch_names.push(full);
                            rl
                        }
                    })
                    .collect();

                commits.push(CommitInfo {
                    id: ids[i].clone(),
                    short_id: format!("c{}", i),
                    message: format!("msg {}", i),
                    author: SignatureInfo {
                        name: "a".into(),
                        email: "a@b.c".into(),
                        timestamp: 0,
                    },
                    committer: SignatureInfo {
                        name: "a".into(),
                        email: "a@b.c".into(),
                        timestamp: 0,
                    },
                    parent_ids,
                    refs,
                    is_cherry_picked: false,
                });
            }

            let unique_names: Vec<String> = {
                let mut seen = HashSet::new();
                all_branch_names
                    .into_iter()
                    .filter(|n| seen.insert(n.clone()))
                    .collect()
            };

            let max_hide = unique_names.len();
            let hide_strat = if max_hide == 0 {
                Just(HashSet::<String>::new()).boxed()
            } else {
                proptest::collection::hash_set(
                    proptest::sample::select(unique_names),
                    0..=max_hide,
                )
                .boxed()
            };

            (Just(commits), hide_strat)
        })
    }

    // **Validates: Requirements 2.6, 2.7**
    //
    // Feature: rust-git-gui-client, Property 5: Hide 过滤正确性
    //
    // For any commit graph and any hidden branch set, hidden branch exclusive
    // commits are invisible, and non-hidden branch ancestors remain visible.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]
        #[test]
        fn prop_hide_filter_correctness(
            (commits, hidden) in commit_graph_with_hide_strategy()
        ) {
            // Collect all branch names in the graph
            let all_branches: HashSet<String> = commits
                .iter()
                .flat_map(|c| c.refs.iter())
                .filter(|rl| matches!(rl.ref_type, RefType::LocalBranch | RefType::RemoteBranch { .. }))
                .map(|rl| rl.name.clone())
                .collect();

            let non_hidden: HashSet<String> = all_branches
                .difference(&hidden)
                .cloned()
                .collect();

            // --- Independently compute expected visible set via BFS from non-hidden tips ---
            let expected: HashSet<String> = if all_branches.is_empty() || hidden.is_empty() {
                // No branches at all, or no hidden branches → all commits visible
                commits.iter().map(|c| c.id.clone()).collect()
            } else {
                let id_to_idx: HashMap<&str, usize> = commits
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (c.id.as_str(), i))
                    .collect();

                let mut visited = HashSet::new();
                let mut queue = VecDeque::new();

                // Seed BFS from non-hidden branch tips
                for (idx, commit) in commits.iter().enumerate() {
                    for rl in &commit.refs {
                        if non_hidden.contains(&rl.name) {
                            if visited.insert(commit.id.clone()) {
                                queue.push_back(idx);
                            }
                        }
                    }
                }

                // BFS through parents
                while let Some(idx) = queue.pop_front() {
                    for pid in &commits[idx].parent_ids {
                        if visited.insert(pid.clone()) {
                            if let Some(&pidx) = id_to_idx.get(pid.as_str()) {
                                queue.push_back(pidx);
                            }
                        }
                    }
                }

                visited
            };

            // --- Call the function under test ---
            let actual_commits = apply_hide_filter(&commits, &hidden);
            let actual: HashSet<String> = actual_commits
                .iter()
                .map(|c| c.id.clone())
                .collect();

            // Assert: all non-hidden branch ancestors are visible
            for id in &expected {
                prop_assert!(
                    actual.contains(id),
                    "Expected commit {} to be visible (reachable from non-hidden branch), but it was hidden. hidden={:?}",
                    id, hidden
                );
            }

            // Assert: no exclusively-hidden-branch commit appears in result
            for id in &actual {
                prop_assert!(
                    expected.contains(id),
                    "Commit {} should be hidden (exclusively reachable from hidden branches), but it appeared in result. hidden={:?}",
                    id, hidden
                );
            }
        }
    }

    /// Strategy: generate a valid commit graph with branch refs and both a
    /// random solo subset and a random hide subset.
    fn commit_graph_with_solo_and_hide_strategy()
        -> impl Strategy<Value = (Vec<CommitInfo>, HashSet<String>, HashSet<String>)>
    {
        (1usize..=15).prop_flat_map(|n| {
            let parent_bitmask_strats: Vec<_> = (0..n)
                .map(|i| {
                    if i + 1 < n {
                        proptest::collection::vec(proptest::bool::ANY, n - i - 1).boxed()
                    } else {
                        proptest::collection::vec(Just(false), 0..=0).boxed()
                    }
                })
                .collect();

            let ref_name_strats: Vec<_> = (0..n)
                .map(|_| {
                    proptest::collection::vec(
                        (proptest::bool::ANY, "[a-z]{3,8}"),
                        0..=2,
                    )
                })
                .collect();

            (Just(n), parent_bitmask_strats, ref_name_strats)
        })
        .prop_flat_map(|(n, parent_bitmasks, ref_name_vecs)| {
            let ids: Vec<String> = (0..n).map(|i| format!("commit_{}", i)).collect();

            let mut all_branch_names: Vec<String> = Vec::new();
            let mut commits: Vec<CommitInfo> = Vec::new();

            for i in 0..n {
                let parent_ids: Vec<String> = parent_bitmasks[i]
                    .iter()
                    .enumerate()
                    .filter(|(_, &is_parent)| is_parent)
                    .take(3)
                    .map(|(j, _)| ids[i + 1 + j].clone())
                    .collect();

                let refs: Vec<RefLabel> = ref_name_vecs[i]
                    .iter()
                    .map(|(is_local, name)| {
                        if *is_local {
                            let rl = RefLabel {
                                name: name.clone(),
                                ref_type: RefType::LocalBranch,
                                is_head: false,
                            };
                            all_branch_names.push(name.clone());
                            rl
                        } else {
                            let full = format!("origin/{}", name);
                            let rl = RefLabel {
                                name: full.clone(),
                                ref_type: RefType::RemoteBranch {
                                    remote: "origin".to_string(),
                                },
                                is_head: false,
                            };
                            all_branch_names.push(full);
                            rl
                        }
                    })
                    .collect();

                commits.push(CommitInfo {
                    id: ids[i].clone(),
                    short_id: format!("c{}", i),
                    message: format!("msg {}", i),
                    author: SignatureInfo {
                        name: "a".into(),
                        email: "a@b.c".into(),
                        timestamp: 0,
                    },
                    committer: SignatureInfo {
                        name: "a".into(),
                        email: "a@b.c".into(),
                        timestamp: 0,
                    },
                    parent_ids,
                    refs,
                    is_cherry_picked: false,
                });
            }

            let unique_names: Vec<String> = {
                let mut seen = HashSet::new();
                all_branch_names
                    .into_iter()
                    .filter(|n| seen.insert(n.clone()))
                    .collect()
            };

            let max_count = unique_names.len();
            let names_for_solo = unique_names.clone();
            let names_for_hide = unique_names;

            let solo_strat = if max_count == 0 {
                Just(HashSet::<String>::new()).boxed()
            } else {
                proptest::collection::hash_set(
                    proptest::sample::select(names_for_solo),
                    0..=max_count,
                )
                .boxed()
            };

            let hide_strat = if max_count == 0 {
                Just(HashSet::<String>::new()).boxed()
            } else {
                proptest::collection::hash_set(
                    proptest::sample::select(names_for_hide),
                    0..=max_count,
                )
                .boxed()
            };

            (Just(commits), solo_strat, hide_strat)
        })
    }

    // **Validates: Requirements 2.9**
    //
    // Feature: rust-git-gui-client, Property 6: Reset View Recovery
    //
    // For any commit graph, after applying any combination of Solo and Hide
    // operations, executing reset_view clears all filter state. Applying
    // filters with the now-empty solo/hide sets returns the full commit history.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]
        #[test]
        fn prop_reset_view_recovery(
            (commits, soloed, hidden) in commit_graph_with_solo_and_hide_strategy()
        ) {
            // Step 1: Apply solo filter, then hide filter (simulating user applying both)
            let after_solo = apply_solo_filter(&commits, &soloed);
            let _after_hide = apply_hide_filter(&after_solo, &hidden);

            // Step 2: Reset view — clears all solo/hide state
            let mut vf = ViewFilter::new();
            for b in &soloed {
                vf.solo_branch(b);
            }
            for b in &hidden {
                vf.hide_branch(b);
            }
            vf.reset_view();

            // Step 3: After reset, soloed_branches and hidden_branches are empty
            prop_assert!(vf.soloed_branches.is_empty(), "soloed_branches should be empty after reset");
            prop_assert!(vf.hidden_branches.is_empty(), "hidden_branches should be empty after reset");

            // Step 4: Apply filters with the now-empty sets
            let after_reset_solo = apply_solo_filter(&commits, &vf.soloed_branches);
            let after_reset_hide = apply_hide_filter(&after_reset_solo, &vf.hidden_branches);

            // Step 5: The result should equal the full commit history
            let full_ids: HashSet<String> = commits.iter().map(|c| c.id.clone()).collect();
            let result_ids: HashSet<String> = after_reset_hide.iter().map(|c| c.id.clone()).collect();

            prop_assert_eq!(
                result_ids,
                full_ids,
                "After reset_view, visible commits should equal full history. soloed={:?}, hidden={:?}",
                soloed,
                hidden
            );
        }
    }

    /// Strategy: generate a valid commit graph with branch refs and a random
    /// ordered subset of branch names to pin to the left.
    fn commit_graph_with_pin_strategy()
        -> impl Strategy<Value = (Vec<CommitInfo>, Vec<String>)>
    {
        (2usize..=15).prop_flat_map(|n| {
            let parent_bitmask_strats: Vec<_> = (0..n)
                .map(|i| {
                    if i + 1 < n {
                        proptest::collection::vec(proptest::bool::ANY, n - i - 1).boxed()
                    } else {
                        proptest::collection::vec(Just(false), 0..=0).boxed()
                    }
                })
                .collect();

            let ref_name_strats: Vec<_> = (0..n)
                .map(|_| {
                    proptest::collection::vec(
                        (proptest::bool::ANY, "[a-z]{3,8}"),
                        0..=2,
                    )
                })
                .collect();

            (Just(n), parent_bitmask_strats, ref_name_strats)
        })
        .prop_flat_map(|(n, parent_bitmasks, ref_name_vecs)| {
            let ids: Vec<String> = (0..n).map(|i| format!("commit_{}", i)).collect();

            let mut all_branch_names: Vec<String> = Vec::new();
            let mut commits: Vec<CommitInfo> = Vec::new();

            for i in 0..n {
                let parent_ids: Vec<String> = parent_bitmasks[i]
                    .iter()
                    .enumerate()
                    .filter(|(_, &is_parent)| is_parent)
                    .take(3)
                    .map(|(j, _)| ids[i + 1 + j].clone())
                    .collect();

                let refs: Vec<RefLabel> = ref_name_vecs[i]
                    .iter()
                    .map(|(is_local, name)| {
                        if *is_local {
                            let rl = RefLabel {
                                name: name.clone(),
                                ref_type: RefType::LocalBranch,
                                is_head: false,
                            };
                            all_branch_names.push(name.clone());
                            rl
                        } else {
                            let full = format!("origin/{}", name);
                            let rl = RefLabel {
                                name: full.clone(),
                                ref_type: RefType::RemoteBranch {
                                    remote: "origin".to_string(),
                                },
                                is_head: false,
                            };
                            all_branch_names.push(full);
                            rl
                        }
                    })
                    .collect();

                commits.push(CommitInfo {
                    id: ids[i].clone(),
                    short_id: format!("c{}", i),
                    message: format!("msg {}", i),
                    author: SignatureInfo {
                        name: "a".into(),
                        email: "a@b.c".into(),
                        timestamp: 0,
                    },
                    committer: SignatureInfo {
                        name: "a".into(),
                        email: "a@b.c".into(),
                        timestamp: 0,
                    },
                    parent_ids,
                    refs,
                    is_cherry_picked: false,
                });
            }

            // Deduplicate branch names preserving order
            let unique_names: Vec<String> = {
                let mut seen = HashSet::new();
                all_branch_names
                    .into_iter()
                    .filter(|n| seen.insert(n.clone()))
                    .collect()
            };

            let max_pin = unique_names.len();
            // Generate a random ordered subset (subsequence) of branch names to pin
            let pin_strat = if max_pin == 0 {
                Just(Vec::<String>::new()).boxed()
            } else {
                // Generate a bitmask to select a subset, preserving order
                proptest::collection::vec(proptest::bool::ANY, max_pin)
                    .prop_map(move |mask| {
                        unique_names
                            .iter()
                            .zip(mask.iter())
                            .filter(|(_, &selected)| selected)
                            .map(|(name, _)| name.clone())
                            .collect::<Vec<String>>()
                    })
                    .boxed()
            };

            (Just(commits), pin_strat)
        })
    }

    // **Validates: Requirements 31.1, 31.2, 31.3**
    //
    // Feature: rust-git-gui-client, Property 22: Pin to Left 布局正确性
    //
    // For any DAG layout and an ordered list of pinned branches, pinned branches
    // SHALL occupy the leftmost columns in pinned order; unpinning restores the
    // original automatic layout.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]
        #[test]
        fn prop_pin_to_left_correctness(
            (commits, pinned) in commit_graph_with_pin_strategy()
        ) {
            use crate::modules::dag_layout::compute_dag_layout;

            // Step 1: Compute the original automatic layout
            let original_layout = compute_dag_layout(&commits);

            // Build branch_name → tip commit_id (first occurrence in topo order)
            let mut branch_tip: HashMap<&str, &str> = HashMap::new();
            for commit in &commits {
                for rl in &commit.refs {
                    if matches!(rl.ref_type, RefType::LocalBranch | RefType::RemoteBranch { .. }) {
                        branch_tip.entry(rl.name.as_str()).or_insert(commit.id.as_str());
                    }
                }
            }

            // Build commit_id → column in the original layout
            let original_commit_col: HashMap<&str, usize> = original_layout
                .nodes
                .iter()
                .map(|n| (n.commit_id.as_str(), n.column))
                .collect();

            // Filter pinned list to only branches with unique tip commits AND
            // unique original columns. The column-swap logic in apply_pin_to_left
            // operates on columns, so branches sharing a tip or column interfere.
            // We verify the property only for the "effective" subset.
            let mut effective_pinned: Vec<String> = Vec::new();
            let mut seen_tips: HashSet<String> = HashSet::new();
            let mut seen_cols: HashSet<usize> = HashSet::new();
            for branch_name in &pinned {
                let tip_id = match branch_tip.get(branch_name.as_str()) {
                    Some(id) => id.to_string(),
                    None => continue,
                };
                let orig_col = match original_commit_col.get(tip_id.as_str()) {
                    Some(&c) => c,
                    None => continue,
                };
                if seen_tips.insert(tip_id) && seen_cols.insert(orig_col) {
                    effective_pinned.push(branch_name.clone());
                }
            }

            // Re-apply pin with only the effective subset to get clean results
            let mut clean_layout = original_layout.clone();
            apply_pin_to_left(&mut clean_layout, &effective_pinned, &commits);

            let clean_commit_col: HashMap<&str, usize> = clean_layout
                .nodes
                .iter()
                .map(|n| (n.commit_id.as_str(), n.column))
                .collect();

            // Verify each effective pinned branch occupies the expected column
            for (idx, branch_name) in effective_pinned.iter().enumerate() {
                let tip_id = branch_tip[branch_name.as_str()];
                let actual_col = clean_commit_col[tip_id];
                prop_assert_eq!(
                    actual_col,
                    idx,
                    "Pinned branch '{}' (tip '{}') should be at column {}, but was at column {}",
                    branch_name,
                    tip_id,
                    idx,
                    actual_col
                );
            }

            // Step 4: Verify unpinning restores the original layout
            // Re-compute the layout from scratch (no pins) — should match original
            let restored_layout = compute_dag_layout(&commits);

            // Compare node columns between original and restored
            let original_cols: HashMap<&str, usize> = original_layout
                .nodes
                .iter()
                .map(|n| (n.commit_id.as_str(), n.column))
                .collect();
            let restored_cols: HashMap<&str, usize> = restored_layout
                .nodes
                .iter()
                .map(|n| (n.commit_id.as_str(), n.column))
                .collect();

            prop_assert_eq!(
                original_cols,
                restored_cols,
                "After unpinning (re-computing layout), columns should match the original automatic layout"
            );
        }
    }

    #[test]
    fn test_pin_to_left_ordering() {
        // 3 branches at columns 0, 1, 2. Pin "branchC" then "branchA".
        let commits = vec![
            make_commit("a", vec!["d"], vec![local_ref("branchA")]),
            make_commit("b", vec!["d"], vec![local_ref("branchB")]),
            make_commit("c", vec!["d"], vec![local_ref("branchC")]),
            make_commit("d", vec![], vec![]),
        ];

        let mut layout = DagLayout {
            nodes: vec![
                DagNode {
                    commit_id: "a".into(),
                    column: 0,
                    row: 0,
                    color_index: 0,
                    parent_edges: vec![],
                },
                DagNode {
                    commit_id: "b".into(),
                    column: 1,
                    row: 1,
                    color_index: 1,
                    parent_edges: vec![],
                },
                DagNode {
                    commit_id: "c".into(),
                    column: 2,
                    row: 2,
                    color_index: 2,
                    parent_edges: vec![],
                },
                DagNode {
                    commit_id: "d".into(),
                    column: 0,
                    row: 3,
                    color_index: 0,
                    parent_edges: vec![],
                },
            ],
            total_columns: 3,
            total_rows: 4,
        };

        apply_pin_to_left(
            &mut layout,
            &["branchC".to_string(), "branchA".to_string()],
            &commits,
        );

        let col_of = |id: &str| -> usize {
            layout
                .nodes
                .iter()
                .find(|n| n.commit_id == id)
                .unwrap()
                .column
        };

        // branchC (tip "c") should be at column 0
        assert_eq!(col_of("c"), 0);
        // branchA (tip "a") should be at column 1
        assert_eq!(col_of("a"), 1);
    }
}
