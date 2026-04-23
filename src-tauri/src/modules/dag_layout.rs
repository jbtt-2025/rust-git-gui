use std::collections::{HashMap, HashSet};

use crate::models::{CommitInfo, DagEdge, DagLayout, DagNode};

/// Compute a DAG layout from a topologically sorted list of commits
/// (most recent first, i.e. row 0 = newest).
///
/// The algorithm assigns each commit a (row, column) position and creates
/// parent edges. It ensures:
/// - No two nodes on the same row share the same column
/// - Branch lines are kept straight where possible
/// - Colors are assigned via greedy graph coloring on columns
pub fn compute_dag_layout(commits: &[CommitInfo]) -> DagLayout {
    if commits.is_empty() {
        return DagLayout {
            nodes: vec![],
            total_columns: 0,
            total_rows: 0,
        };
    }

    // Map commit id -> row index
    let id_to_row: HashMap<&str, usize> = commits
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.as_str(), i))
        .collect();

    // Track which columns are "active" at each row.
    // active_columns: maps column -> commit_id that is currently occupying that lane
    // We also track commit_id -> assigned column for lookups.
    let mut commit_column: HashMap<&str, usize> = HashMap::new();
    // Active lanes: each lane is identified by column index and holds the commit_id
    // that "owns" that lane (the commit whose branch line passes through it).
    let mut active_lanes: Vec<Option<String>> = Vec::new();
    // Column color assignments
    let mut column_colors: HashMap<usize, usize> = HashMap::new();

    let mut nodes: Vec<DagNode> = Vec::with_capacity(commits.len());

    for (row, commit) in commits.iter().enumerate() {
        // Find if this commit already has a reserved column (from a child's parent reservation)
        let column = if let Some(&col) = commit_column.get(commit.id.as_str()) {
            col
        } else {
            // No child reserved a column for us — find the leftmost free column
            let col = find_free_column(&active_lanes);
            if col >= active_lanes.len() {
                active_lanes.resize(col + 1, None);
            }
            active_lanes[col] = Some(commit.id.clone());
            commit_column.insert(&commit.id, col);
            col
        };

        // Assign color to this column if not yet assigned
        if !column_colors.contains_key(&column) {
            let color = assign_color(column, &active_lanes, &column_colors);
            column_colors.insert(column, color);
        }

        let node_color = column_colors[&column];

        // Build parent edges and reserve columns for parents
        let mut parent_edges = Vec::new();

        for (pi, parent_id) in commit.parent_ids.iter().enumerate() {
            if let Some(&parent_row) = id_to_row.get(parent_id.as_str()) {
                let parent_col = if pi == 0 {
                    // First parent: continues in the same column (straight line)
                    if !commit_column.contains_key(parent_id.as_str()) {
                        commit_column.insert(parent_id, column);
                        // Keep the lane active for the first parent
                        active_lanes[column] = Some(parent_id.clone());
                    }
                    commit_column[parent_id.as_str()]
                } else {
                    // Secondary parents (merge): assign a new column if not already placed
                    if let Some(&existing_col) = commit_column.get(parent_id.as_str()) {
                        existing_col
                    } else {
                        let new_col = find_free_column(&active_lanes);
                        if new_col >= active_lanes.len() {
                            active_lanes.resize(new_col + 1, None);
                        }
                        active_lanes[new_col] = Some(parent_id.clone());
                        commit_column.insert(parent_id, new_col);

                        // Assign color to the new column
                        if !column_colors.contains_key(&new_col) {
                            let color =
                                assign_color(new_col, &active_lanes, &column_colors);
                            column_colors.insert(new_col, color);
                        }
                        new_col
                    }
                };

                let edge_color = column_colors.get(&parent_col).copied().unwrap_or(node_color);

                parent_edges.push(DagEdge {
                    from_column: column,
                    to_column: parent_col,
                    to_row: parent_row,
                    color_index: edge_color,
                });
            }
        }

        // If this commit has no parents, free its lane
        if commit.parent_ids.is_empty() {
            if column < active_lanes.len() {
                active_lanes[column] = None;
            }
        } else if commit.parent_ids.len() == 1 {
            // Single parent: lane continues, already handled above
        } else {
            // Merge commit: the current lane continues to first parent.
            // Additional parent lanes are already set up above.
        }

        nodes.push(DagNode {
            commit_id: commit.id.clone(),
            column,
            row,
            color_index: node_color,
            parent_edges,
        });

        // After processing this commit, free lanes for commits that have been
        // fully processed (i.e., this row is their row and they won't appear again).
        // The lane is freed when the commit at this row is done and its first parent
        // takes over the lane. If the commit has no parents, we already freed it above.
    }

    let total_columns = if nodes.is_empty() {
        0
    } else {
        nodes.iter().map(|n| n.column).max().unwrap_or(0) + 1
    };

    DagLayout {
        nodes,
        total_columns,
        total_rows: commits.len(),
    }
}

/// Find the leftmost column that is not currently occupied.
fn find_free_column(active_lanes: &[Option<String>]) -> usize {
    for (i, lane) in active_lanes.iter().enumerate() {
        if lane.is_none() {
            return i;
        }
    }
    active_lanes.len()
}

/// Greedy color assignment: pick the smallest color_index not used by
/// any adjacent active column.
fn assign_color(
    column: usize,
    active_lanes: &[Option<String>],
    column_colors: &HashMap<usize, usize>,
) -> usize {
    let mut used_colors: HashSet<usize> = HashSet::new();

    // Collect colors of adjacent active columns
    for (col, lane) in active_lanes.iter().enumerate() {
        if lane.is_some() && col != column {
            if let Some(&color) = column_colors.get(&col) {
                // Consider columns that are "adjacent" — within distance 1
                if col + 1 == column || column + 1 == col {
                    used_colors.insert(color);
                }
            }
        }
    }

    // Also consider all currently active columns for better differentiation
    // when columns are close together
    for (col, lane) in active_lanes.iter().enumerate() {
        if lane.is_some() && col != column {
            if let Some(&color) = column_colors.get(&col) {
                used_colors.insert(color);
            }
        }
    }

    // Pick smallest unused color
    let mut color = 0;
    while used_colors.contains(&color) {
        color += 1;
    }
    color
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CommitInfo, SignatureInfo};
    use std::collections::HashSet;

    /// Helper to create a CommitInfo with minimal fields.
    fn make_commit(id: &str, parent_ids: Vec<&str>) -> CommitInfo {
        CommitInfo {
            id: id.to_string(),
            short_id: id[..std::cmp::min(7, id.len())].to_string(),
            message: format!("commit {}", id),
            author: SignatureInfo {
                name: "Test".to_string(),
                email: "test@test.com".to_string(),
                timestamp: 0,
            },
            committer: SignatureInfo {
                name: "Test".to_string(),
                email: "test@test.com".to_string(),
                timestamp: 0,
            },
            parent_ids: parent_ids.into_iter().map(String::from).collect(),
            refs: vec![],
            is_cherry_picked: false,
        }
    }

    /// Verify no column conflicts: for any row, all nodes have distinct columns.
    fn assert_no_column_conflicts(layout: &DagLayout) {
        let mut row_columns: HashMap<usize, Vec<usize>> = HashMap::new();
        for node in &layout.nodes {
            row_columns
                .entry(node.row)
                .or_default()
                .push(node.column);
        }
        for (row, cols) in &row_columns {
            let unique: HashSet<_> = cols.iter().collect();
            assert_eq!(
                unique.len(),
                cols.len(),
                "Column conflict at row {}: columns {:?}",
                row,
                cols
            );
        }
    }

    /// Verify edge correctness: every parent_edge connects to a valid parent node.
    fn assert_edge_correctness(layout: &DagLayout, commits: &[CommitInfo]) {
        let id_to_row: HashMap<&str, usize> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.as_str(), i))
            .collect();
        let id_to_col: HashMap<&str, usize> = layout
            .nodes
            .iter()
            .map(|n| (n.commit_id.as_str(), n.column))
            .collect();

        for node in &layout.nodes {
            assert_eq!(node.row, id_to_row[node.commit_id.as_str()]);
            for edge in &node.parent_edges {
                assert_eq!(edge.from_column, node.column, "Edge from_column mismatch");
                // Find the parent commit
                let parent_commit = commits
                    .iter()
                    .find(|c| {
                        let parent_row = id_to_row.get(c.id.as_str());
                        parent_row == Some(&edge.to_row)
                    });
                assert!(
                    parent_commit.is_some(),
                    "Edge to_row {} does not correspond to any commit",
                    edge.to_row
                );
                let parent = parent_commit.unwrap();
                assert_eq!(
                    edge.to_column,
                    id_to_col[parent.id.as_str()],
                    "Edge to_column mismatch for parent {}",
                    parent.id
                );
            }
        }
    }

    /// Verify topological ordering: child row < parent row.
    fn assert_topological_order(layout: &DagLayout) {
        for node in &layout.nodes {
            for edge in &node.parent_edges {
                assert!(
                    node.row < edge.to_row,
                    "Child row {} should be less than parent row {} (commit {})",
                    node.row,
                    edge.to_row,
                    node.commit_id
                );
            }
        }
    }

    /// Verify color assignment: adjacent active columns have different colors.
    fn assert_color_differentiation(layout: &DagLayout) {
        // For each row, check that nodes in adjacent columns have different colors
        let mut row_nodes: HashMap<usize, Vec<&DagNode>> = HashMap::new();
        for node in &layout.nodes {
            row_nodes.entry(node.row).or_default().push(node);
        }
        for (_row, nodes) in &row_nodes {
            let mut sorted = nodes.clone();
            sorted.sort_by_key(|n| n.column);
            for pair in sorted.windows(2) {
                if pair[0].column + 1 == pair[1].column {
                    // Adjacent columns — colors should differ
                    assert_ne!(
                        pair[0].color_index, pair[1].color_index,
                        "Adjacent columns {} and {} have same color {} at row {}",
                        pair[0].column, pair[1].column, pair[0].color_index, pair[0].row
                    );
                }
            }
        }
    }

    // === Test: Empty history ===
    #[test]
    fn test_empty_history() {
        let layout = compute_dag_layout(&[]);
        assert_eq!(layout.nodes.len(), 0);
        assert_eq!(layout.total_columns, 0);
        assert_eq!(layout.total_rows, 0);
    }

    // === Test: Single commit ===
    #[test]
    fn test_single_commit() {
        let commits = vec![make_commit("aaa", vec![])];
        let layout = compute_dag_layout(&commits);
        assert_eq!(layout.nodes.len(), 1);
        assert_eq!(layout.nodes[0].column, 0);
        assert_eq!(layout.nodes[0].row, 0);
        assert!(layout.nodes[0].parent_edges.is_empty());
        assert_eq!(layout.total_columns, 1);
        assert_eq!(layout.total_rows, 1);
    }

    // === Test: Linear history (single branch) ===
    #[test]
    fn test_linear_history() {
        // c3 -> c2 -> c1 -> c0 (most recent first)
        let commits = vec![
            make_commit("c3", vec!["c2"]),
            make_commit("c2", vec!["c1"]),
            make_commit("c1", vec!["c0"]),
            make_commit("c0", vec![]),
        ];
        let layout = compute_dag_layout(&commits);

        assert_eq!(layout.nodes.len(), 4);
        assert_eq!(layout.total_rows, 4);

        // All should be in column 0 (straight line)
        for node in &layout.nodes {
            assert_eq!(node.column, 0, "Linear history should use column 0");
        }

        assert_no_column_conflicts(&layout);
        assert_edge_correctness(&layout, &commits);
        assert_topological_order(&layout);
    }

    // === Test: Simple branch and merge ===
    #[test]
    fn test_branch_and_merge() {
        // Topology:
        //   merge (row 0) -> c2 (row 1), c3 (row 2)
        //   c2 (row 1) -> base (row 3)
        //   c3 (row 2) -> base (row 3)
        let commits = vec![
            make_commit("merge", vec!["c2", "c3"]),
            make_commit("c2", vec!["base"]),
            make_commit("c3", vec!["base"]),
            make_commit("base", vec![]),
        ];
        let layout = compute_dag_layout(&commits);

        assert_eq!(layout.nodes.len(), 4);
        assert_no_column_conflicts(&layout);
        assert_edge_correctness(&layout, &commits);
        assert_topological_order(&layout);

        // merge should have 2 parent edges
        let merge_node = layout.nodes.iter().find(|n| n.commit_id == "merge").unwrap();
        assert_eq!(merge_node.parent_edges.len(), 2);
    }

    // === Test: Multiple parallel branches ===
    #[test]
    fn test_parallel_branches() {
        // Three branches diverging from base:
        //   a2 -> a1 -> base
        //   b2 -> b1 -> base
        //   c2 -> c1 -> base
        // Topological order (most recent first):
        let commits = vec![
            make_commit("a2", vec!["a1"]),
            make_commit("b2", vec!["b1"]),
            make_commit("c2", vec!["c1"]),
            make_commit("a1", vec!["base"]),
            make_commit("b1", vec!["base"]),
            make_commit("c1", vec!["base"]),
            make_commit("base", vec![]),
        ];
        let layout = compute_dag_layout(&commits);

        assert_eq!(layout.nodes.len(), 7);
        assert_no_column_conflicts(&layout);
        assert_edge_correctness(&layout, &commits);
        assert_topological_order(&layout);

        // Should use at least 3 columns for the parallel branches
        assert!(
            layout.total_columns >= 3,
            "Expected at least 3 columns for 3 parallel branches, got {}",
            layout.total_columns
        );
    }

    // === Test: Complex merge scenario ===
    #[test]
    fn test_complex_merge() {
        // m2 merges b2 and c1
        // m2 -> b2, c1
        // b2 -> b1
        // c1 -> base
        // b1 -> base
        let commits = vec![
            make_commit("m2", vec!["b2", "c1"]),
            make_commit("b2", vec!["b1"]),
            make_commit("c1", vec!["base"]),
            make_commit("b1", vec!["base"]),
            make_commit("base", vec![]),
        ];
        let layout = compute_dag_layout(&commits);

        assert_no_column_conflicts(&layout);
        assert_edge_correctness(&layout, &commits);
        assert_topological_order(&layout);
    }

    // === Test: Verify color assignment ===
    #[test]
    fn test_color_assignment() {
        // Two parallel branches should get different colors
        let commits = vec![
            make_commit("a2", vec!["a1"]),
            make_commit("b2", vec!["b1"]),
            make_commit("a1", vec!["base"]),
            make_commit("b1", vec!["base"]),
            make_commit("base", vec![]),
        ];
        let layout = compute_dag_layout(&commits);

        assert_no_column_conflicts(&layout);
        assert_edge_correctness(&layout, &commits);
        assert_topological_order(&layout);
        assert_color_differentiation(&layout);
    }

    // === Test: Long linear chain ===
    #[test]
    fn test_long_linear_chain() {
        let n = 50;
        let mut commits = Vec::new();
        for i in (0..n).rev() {
            let id = format!("c{}", i);
            let parents = if i > 0 {
                vec![format!("c{}", i - 1)]
            } else {
                vec![]
            };
            commits.push(CommitInfo {
                id: id.clone(),
                short_id: id[..std::cmp::min(7, id.len())].to_string(),
                message: format!("commit {}", id),
                author: SignatureInfo {
                    name: "Test".to_string(),
                    email: "test@test.com".to_string(),
                    timestamp: 0,
                },
                committer: SignatureInfo {
                    name: "Test".to_string(),
                    email: "test@test.com".to_string(),
                    timestamp: 0,
                },
                parent_ids: parents,
                refs: vec![],
                is_cherry_picked: false,
            });
        }

        let layout = compute_dag_layout(&commits);
        assert_eq!(layout.total_rows, n);
        assert_eq!(layout.total_columns, 1);
        assert_no_column_conflicts(&layout);
        assert_edge_correctness(&layout, &commits);
        assert_topological_order(&layout);
    }

    // === Test: Octopus merge (multiple parents) ===
    #[test]
    fn test_octopus_merge() {
        let commits = vec![
            make_commit("octopus", vec!["p1", "p2", "p3"]),
            make_commit("p1", vec!["base"]),
            make_commit("p2", vec!["base"]),
            make_commit("p3", vec!["base"]),
            make_commit("base", vec![]),
        ];
        let layout = compute_dag_layout(&commits);

        assert_no_column_conflicts(&layout);
        assert_edge_correctness(&layout, &commits);
        assert_topological_order(&layout);

        let octopus_node = layout.nodes.iter().find(|n| n.commit_id == "octopus").unwrap();
        assert_eq!(octopus_node.parent_edges.len(), 3);
    }

    // === Property-Based Test: DAG Layout Invariants ===
    // Feature: rust-git-gui-client, Property 2: DAG Layout Invariants
    // **Validates: Requirements 2.1**

    use proptest::prelude::*;

    /// Strategy to generate a valid commit history graph.
    ///
    /// Generates 1..20 commits where:
    /// - Each commit has a unique ID (format: "c{index}")
    /// - The list is ordered most-recent-first (index 0 = newest)
    /// - Parent IDs reference only commits at higher indices (older commits)
    /// - Some commits have 0 parents (root), 1 parent (linear), or 2+ parents (merge)
    fn arb_commit_graph() -> impl Strategy<Value = Vec<CommitInfo>> {
        // Generate the number of commits first, then build the graph
        (1usize..=20).prop_flat_map(|num_commits| {
            // For each commit at index i, generate a parent count type:
            // 0 = no parents (root), 1 = one parent, 2 = two parents (merge)
            // The last commit (highest index) must be a root since there are no
            // later commits to reference. For others, we pick randomly.
            let parent_strategies: Vec<BoxedStrategy<u8>> = (0..num_commits)
                .map(|i| {
                    if i == num_commits - 1 {
                        // Last commit must be a root (no commits after it to reference)
                        Just(0u8).boxed()
                    } else if i == num_commits - 2 {
                        // Second-to-last can be root or have 1 parent
                        prop_oneof![
                            2 => Just(0u8),
                            8 => Just(1u8),
                        ].boxed()
                    } else {
                        // Can be root, linear, or merge
                        prop_oneof![
                            1 => Just(0u8),
                            6 => Just(1u8),
                            3 => Just(2u8),
                        ].boxed()
                    }
                })
                .collect();

            parent_strategies.prop_map(move |parent_types| {
                let mut commits = Vec::with_capacity(num_commits);
                for i in 0..num_commits {
                    let id = format!("c{}", i);
                    let available_parents: Vec<usize> =
                        ((i + 1)..num_commits).collect();

                    let parent_ids: Vec<String> = if available_parents.is_empty() {
                        // No later commits to reference — must be root
                        vec![]
                    } else {
                        match parent_types[i] {
                            0 => vec![],
                            1 => {
                                // Pick the first available parent (deterministic
                                // given the generated type)
                                vec![format!("c{}", available_parents[0])]
                            }
                            2 => {
                                if available_parents.len() >= 2 {
                                    vec![
                                        format!("c{}", available_parents[0]),
                                        format!("c{}", available_parents[1]),
                                    ]
                                } else {
                                    // Only one available — fall back to single parent
                                    vec![format!("c{}", available_parents[0])]
                                }
                            }
                            _ => vec![],
                        }
                    };

                    commits.push(CommitInfo {
                        id,
                        short_id: format!("c{}", i),
                        message: format!("commit c{}", i),
                        author: SignatureInfo {
                            name: "Test".to_string(),
                            email: "test@test.com".to_string(),
                            timestamp: 0,
                        },
                        committer: SignatureInfo {
                            name: "Test".to_string(),
                            email: "test@test.com".to_string(),
                            timestamp: 0,
                        },
                        parent_ids,
                        refs: vec![],
                        is_cherry_picked: false,
                    });
                }
                commits
            })
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Property 2: DAG Layout Invariants
        ///
        /// For any valid commit history graph, the DAG layout algorithm SHALL satisfy:
        /// (a) No column conflicts on the same row
        /// (b) Edges correctly connect parent-child nodes
        /// (c) Child row number < parent row number (topological order)
        ///
        /// **Validates: Requirements 2.1**
        #[test]
        fn prop_dag_layout_invariants(commits in arb_commit_graph()) {
            let layout = compute_dag_layout(&commits);

            // Basic structural checks
            prop_assert_eq!(layout.nodes.len(), commits.len());
            prop_assert_eq!(layout.total_rows, commits.len());

            // (a) No column conflicts on the same row
            let mut row_columns: HashMap<usize, Vec<usize>> = HashMap::new();
            for node in &layout.nodes {
                row_columns.entry(node.row).or_default().push(node.column);
            }
            for (row, cols) in &row_columns {
                let unique: HashSet<usize> = cols.iter().copied().collect();
                prop_assert_eq!(
                    unique.len(),
                    cols.len(),
                    "Column conflict at row {}: columns {:?}",
                    row,
                    cols
                );
            }

            // Build lookup maps for invariant (b)
            let id_to_row: HashMap<&str, usize> = commits
                .iter()
                .enumerate()
                .map(|(i, c)| (c.id.as_str(), i))
                .collect();
            let _id_to_col: HashMap<&str, usize> = layout
                .nodes
                .iter()
                .map(|n| (n.commit_id.as_str(), n.column))
                .collect();

            for node in &layout.nodes {
                // Verify node row matches its index
                prop_assert_eq!(
                    node.row,
                    id_to_row[node.commit_id.as_str()],
                    "Node row mismatch for commit {}",
                    node.commit_id
                );

                for edge in &node.parent_edges {
                    // (b) Edge from_column matches the node's column
                    prop_assert_eq!(
                        edge.from_column,
                        node.column,
                        "Edge from_column {} != node column {} for commit {}",
                        edge.from_column,
                        node.column,
                        node.commit_id
                    );

                    // (b) Edge to_row and to_column match the actual parent node
                    let parent_node = layout.nodes.iter().find(|n| n.row == edge.to_row);
                    prop_assert!(
                        parent_node.is_some(),
                        "Edge to_row {} does not correspond to any node",
                        edge.to_row
                    );
                    let parent = parent_node.unwrap();
                    prop_assert_eq!(
                        edge.to_column,
                        parent.column,
                        "Edge to_column {} != parent column {} for parent {}",
                        edge.to_column,
                        parent.column,
                        parent.commit_id
                    );

                    // (c) Child row < parent row (topological order)
                    prop_assert!(
                        node.row < edge.to_row,
                        "Child row {} should be < parent row {} (commit {} -> parent {})",
                        node.row,
                        edge.to_row,
                        node.commit_id,
                        parent.commit_id
                    );
                }
            }
        }

        /// Property 3: Branch Color Differentiation
        ///
        /// For any valid commit history graph, in the resulting DAG layout,
        /// nodes that are in adjacent columns (column difference of 1) on the
        /// same row should have different color_index values.
        ///
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_branch_color_differentiation(commits in arb_commit_graph()) {
            let layout = compute_dag_layout(&commits);

            // Group nodes by row
            let mut row_nodes: HashMap<usize, Vec<&DagNode>> = HashMap::new();
            for node in &layout.nodes {
                row_nodes.entry(node.row).or_default().push(node);
            }

            // For each row, sort nodes by column and check adjacent pairs
            for (row, mut nodes) in row_nodes {
                nodes.sort_by_key(|n| n.column);
                for pair in nodes.windows(2) {
                    if pair[0].column + 1 == pair[1].column {
                        prop_assert_ne!(
                            pair[0].color_index,
                            pair[1].color_index,
                            "Adjacent columns {} and {} have same color_index {} at row {}",
                            pair[0].column,
                            pair[1].column,
                            pair[0].color_index,
                            row
                        );
                    }
                }
            }
        }
    }
}
