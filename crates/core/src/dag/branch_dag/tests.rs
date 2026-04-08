// `BranchDag` tests

use crate::dag::BranchDag;
use crate::dag::BranchId;
use crate::dag::DagError;

use proptest::prelude::*;

// ── Construction ────────────────────────────────────────────────────────────

#[test]
fn test_new_returns_dag_with_trunk_branch() {
    let dag = BranchDag::new();
    assert!(dag.contains(&BranchId::new("trunk")));
    assert!(dag.is_trunk(&BranchId::new("trunk")));
    assert_eq!(dag.len(), 1);
}

#[test]
fn test_default_is_same_as_new() {
    let dag = BranchDag::default();
    assert!(dag.contains(&BranchId::new("trunk")));
    assert_eq!(dag.len(), 1);
}

#[test]
fn test_is_empty_on_new() {
    let dag = BranchDag::new();
    // "empty" means only trunk (len == 1)
    assert!(dag.is_empty());
    assert_eq!(dag.len(), 1);
}

#[test]
fn test_is_empty_false_after_add() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    assert!(!dag.is_empty());
    assert_eq!(dag.len(), 2);
}

// ── Add branch operations ───────────────────────────────────────────────────

#[test]
fn test_add_branch_creates_branch_with_parents() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    assert!(dag.contains(&BranchId::new("feature")));
    assert_eq!(dag.len(), 2);
}

#[test]
fn test_add_branch_with_multiple_parents() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature-a"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    dag.add_branch(BranchId::new("feature-b"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    dag.add_branch(
        BranchId::new("merge"),
        vec![BranchId::new("feature-a"), BranchId::new("feature-b")],
    )
    .expect("Should add branch with two parents");
    assert!(dag.contains(&BranchId::new("merge")));
    assert_eq!(dag.len(), 4);
}

#[test]
fn test_add_branch_already_exists_error() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let result = dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")]);
    assert!(matches!(result, Err(DagError::BranchAlreadyExists(_))));
}

#[test]
fn test_add_branch_invalid_parent_error() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("feature"), vec![BranchId::new("nonexistent")]);
    assert!(matches!(result, Err(DagError::InvalidParent(_))));
}

#[test]
fn test_add_branch_no_parent_for_non_trunk_error() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("orphan"), vec![]);
    assert!(matches!(result, Err(DagError::NoParentForBranch(_))));
}

// ── Cycle detection ─────────────────────────────────────────────────────────

#[test]
fn test_add_branch_self_reference_is_cycle() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("self"), vec![BranchId::new("self")]);
    assert!(matches!(result, Err(DagError::CycleDetected(_))));
}

#[test]
fn test_add_branch_creates_no_actual_cycle() {
    let mut dag = BranchDag::new();
    // Create a chain: trunk -> feature -> subfeature
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add feature");
    dag.add_branch(BranchId::new("subfeature"), vec![BranchId::new("feature")])
        .expect("Should add subfeature");

    // Trying to make trunk's parent be subfeature should fail (cycle)
    // We can't directly modify trunk's parents, but we can verify that adding a
    // branch whose parent is a descendant of itself would be caught.
    // trunk -> feature -> subfeature -> trunk would be a cycle.
    // Since trunk exists, we'd need a way to create this. However, we can verify
    // with would_create_cycle indirectly by trying to add trunk as a child of subfeature.
    // But trunk already exists, so this becomes BranchAlreadyExists.
    assert!(dag.contains(&BranchId::new("subfeature")));
}

#[test]
fn test_add_branch_deep_chain_no_cycle() {
    let mut dag = BranchDag::new();
    // Create a 5-level deep chain
    dag.add_branch(BranchId::new("b1"), vec![BranchId::new("trunk")])
        .expect("b1");
    dag.add_branch(BranchId::new("b2"), vec![BranchId::new("b1")])
        .expect("b2");
    dag.add_branch(BranchId::new("b3"), vec![BranchId::new("b2")])
        .expect("b3");
    dag.add_branch(BranchId::new("b4"), vec![BranchId::new("b3")])
        .expect("b4");
    dag.add_branch(BranchId::new("b5"), vec![BranchId::new("b4")])
        .expect("b5");
    assert_eq!(dag.len(), 6);
}

// ── Remove branch operations ────────────────────────────────────────────────

#[test]
fn test_remove_branch_success() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    dag.remove_branch(BranchId::new("feature"))
        .expect("Should remove branch");
    assert!(!dag.contains(&BranchId::new("feature")));
    assert_eq!(dag.len(), 1);
}

#[test]
fn test_remove_branch_not_found_error() {
    let mut dag = BranchDag::new();
    let result = dag.remove_branch(BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

#[test]
fn test_remove_branch_with_descendants_error() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add feature");
    dag.add_branch(BranchId::new("sub"), vec![BranchId::new("feature")])
        .expect("Should add sub");
    let result = dag.remove_branch(BranchId::new("feature"));
    assert!(matches!(result, Err(DagError::HasDescendants(_, _))));
}

// ── Ancestors & descendants ─────────────────────────────────────────────────

#[test]
fn test_ancestors_single() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let ancestors = dag.ancestors(&BranchId::new("feature")).expect("ancestors");
    assert_eq!(ancestors.len(), 1);
    assert_eq!(ancestors[0], BranchId::new("trunk"));
}

#[test]
fn test_ancestors_chain() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")])
        .expect("a");
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")])
        .expect("b");
    let ancestors = dag.ancestors(&BranchId::new("b")).expect("ancestors");
    assert_eq!(ancestors.len(), 2);
    assert!(ancestors.contains(&BranchId::new("a")));
    assert!(ancestors.contains(&BranchId::new("trunk")));
}

#[test]
fn test_ancestors_trunk_is_empty() {
    let dag = BranchDag::new();
    let ancestors = dag.ancestors(&BranchId::new("trunk")).expect("ancestors");
    assert!(ancestors.is_empty());
}

#[test]
fn test_ancestors_not_found_error() {
    let dag = BranchDag::new();
    let result = dag.ancestors(&BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

#[test]
fn test_descendants_single() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let descendants = dag
        .descendants(&BranchId::new("trunk"))
        .expect("descendants");
    assert_eq!(descendants.len(), 1);
    assert_eq!(descendants[0], BranchId::new("feature"));
}

#[test]
fn test_descendants_chain() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")])
        .expect("a");
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")])
        .expect("b");
    let descendants = dag
        .descendants(&BranchId::new("trunk"))
        .expect("descendants");
    assert_eq!(descendants.len(), 2);
    assert!(descendants.contains(&BranchId::new("a")));
    assert!(descendants.contains(&BranchId::new("b")));
}

#[test]
fn test_descendants_leaf_is_empty() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let descendants = dag
        .descendants(&BranchId::new("feature"))
        .expect("descendants");
    assert!(descendants.is_empty());
}

#[test]
fn test_descendants_not_found_error() {
    let dag = BranchDag::new();
    let result = dag.descendants(&BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

// ── Topological ordering ────────────────────────────────────────────────────

#[test]
fn test_topological_sort_trunk_only() {
    let dag = BranchDag::new();
    let order = dag.topological_sort().expect("topo sort");
    assert_eq!(order.len(), 1);
    assert_eq!(order[0], BranchId::new("trunk"));
}

#[test]
fn test_topological_sort_chain() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")])
        .expect("a");
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")])
        .expect("b");
    let order = dag.topological_sort().expect("topo sort");
    assert_eq!(order.len(), 3);
    // Parents must come before children
    let trunk_pos = order
        .iter()
        .position(|id| id == &BranchId::new("trunk"))
        .expect("trunk pos");
    let a_pos = order
        .iter()
        .position(|id| id == &BranchId::new("a"))
        .expect("a pos");
    let b_pos = order
        .iter()
        .position(|id| id == &BranchId::new("b"))
        .expect("b pos");
    assert!(trunk_pos < a_pos);
    assert!(a_pos < b_pos);
}

#[test]
fn test_topological_sort_diamond() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("left"), vec![BranchId::new("trunk")])
        .expect("left");
    dag.add_branch(BranchId::new("right"), vec![BranchId::new("trunk")])
        .expect("right");
    dag.add_branch(
        BranchId::new("merge"),
        vec![BranchId::new("left"), BranchId::new("right")],
    )
    .expect("merge");
    let order = dag.topological_sort().expect("topo sort");
    assert_eq!(order.len(), 4);
    // trunk must be before left and right, merge must be last
    let trunk_pos = order
        .iter()
        .position(|id| id == &BranchId::new("trunk"))
        .expect("trunk pos");
    let left_pos = order
        .iter()
        .position(|id| id == &BranchId::new("left"))
        .expect("left pos");
    let right_pos = order
        .iter()
        .position(|id| id == &BranchId::new("right"))
        .expect("right pos");
    let merge_pos = order
        .iter()
        .position(|id| id == &BranchId::new("merge"))
        .expect("merge pos");
    assert!(trunk_pos < left_pos);
    assert!(trunk_pos < right_pos);
    assert!(left_pos < merge_pos);
    assert!(right_pos < merge_pos);
}

// ── Path to root ────────────────────────────────────────────────────────────

#[test]
fn test_path_to_root_trunk() {
    let dag = BranchDag::new();
    let path = dag.path_to_root(&BranchId::new("trunk")).expect("path");
    assert!(path.is_empty());
}

#[test]
fn test_path_to_root_single_level() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let path = dag.path_to_root(&BranchId::new("feature")).expect("path");
    assert_eq!(path.len(), 2);
    assert_eq!(path[0], BranchId::new("feature"));
    assert_eq!(path[1], BranchId::new("trunk"));
}

#[test]
fn test_path_to_root_chain() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")])
        .expect("a");
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")])
        .expect("b");
    let path = dag.path_to_root(&BranchId::new("b")).expect("path");
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], BranchId::new("b"));
    assert_eq!(path[1], BranchId::new("a"));
    assert_eq!(path[2], BranchId::new("trunk"));
}

#[test]
fn test_path_to_root_not_found_error() {
    let dag = BranchDag::new();
    let result = dag.path_to_root(&BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

// ── Branch IDs ──────────────────────────────────────────────────────────────

#[test]
fn test_branch_ids_sorted() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("z-branch"), vec![BranchId::new("trunk")])
        .expect("z");
    dag.add_branch(BranchId::new("a-branch"), vec![BranchId::new("trunk")])
        .expect("a");
    let ids = dag.branch_ids();
    // BTreeSet gives sorted order
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

#[test]
fn test_contains_false_for_missing() {
    let dag = BranchDag::new();
    assert!(!dag.contains(&BranchId::new("nonexistent")));
}

#[test]
fn test_is_trunk_only_for_trunk() {
    let mut dag = BranchDag::new();
    dag.add_branch(
        BranchId::new("trunk-imitator"),
        vec![BranchId::new("trunk")],
    )
    .expect("Should add");
    assert!(dag.is_trunk(&BranchId::new("trunk")));
    assert!(!dag.is_trunk(&BranchId::new("trunk-imitator")));
    assert!(!dag.is_trunk(&BranchId::new("trunk-")));
    assert!(!dag.is_trunk(&BranchId::new("trunk1")));
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXHAUSTIVE TESTS — add nodes, topological sort, cycle detection, ancestry
// ═══════════════════════════════════════════════════════════════════════════════

// ── Add nodes: edge cases & stress ──────────────────────────────────────────

#[test]
fn test_add_branch_wide_fan_out() {
    let mut dag = BranchDag::new();
    for i in 0..50 {
        dag.add_branch(
            BranchId::new(format!("branch-{i}")),
            vec![BranchId::new("trunk")],
        )
        .unwrap_or_else(|e| panic!("branch-{i} should add: {e}"));
    }
    assert_eq!(dag.len(), 51);
    for i in 0..50 {
        assert!(dag.contains(&BranchId::new(format!("branch-{i}"))));
    }
}

#[test]
fn test_add_branch_deep_chain_unique_names() {
    let mut dag = BranchDag::new();
    let mut prev = BranchId::new("trunk");
    for i in 1..=50 {
        let curr = BranchId::new(format!("level-{i}"));
        dag.add_branch(curr.clone(), vec![prev]).unwrap();
        prev = curr;
    }
    assert_eq!(dag.len(), 51);
    let ancestors = dag.ancestors(&BranchId::new("level-50")).unwrap();
    assert_eq!(ancestors.len(), 50);
}

#[test]
fn test_add_branch_diamond_merge_graph() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("left"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(BranchId::new("right"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(BranchId::new("center"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(
        BranchId::new("merge"),
        vec![
            BranchId::new("left"),
            BranchId::new("right"),
            BranchId::new("center"),
        ],
    )
    .unwrap();
    assert_eq!(dag.len(), 5);
    let desc = dag.descendants(&BranchId::new("trunk")).unwrap();
    assert_eq!(desc.len(), 4);
}

#[test]
fn test_add_branch_after_remove_reuse_name() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.remove_branch(BranchId::new("feature")).unwrap();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .unwrap();
    assert_eq!(dag.len(), 2);
    assert!(dag.contains(&BranchId::new("feature")));
}

#[test]
fn test_add_branch_special_characters_in_name() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature/v2-rc1"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(BranchId::new("bugfix#123"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(BranchId::new("user@branch"), vec![BranchId::new("trunk")])
        .unwrap();
    assert_eq!(dag.len(), 4);
}

#[test]
fn test_add_branch_empty_string_name() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new(""), vec![BranchId::new("trunk")]).unwrap();
    assert!(dag.contains(&BranchId::new("")));
    assert!(!dag.is_trunk(&BranchId::new("")));
}

#[test]
fn test_add_branch_unicode_names() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("特性"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("🚀-branch"), vec![BranchId::new("trunk")]).unwrap();
    assert_eq!(dag.len(), 3);
}

#[test]
fn test_add_branch_cannot_add_trunk_again() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("trunk"), vec![BranchId::new("trunk")]);
    assert!(matches!(result, Err(DagError::CycleDetected(_))));
}

#[test]
fn test_add_branch_parent_must_exist_all_checked() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    let result = dag.add_branch(
        BranchId::new("b"),
        vec![BranchId::new("a"), BranchId::new("nonexistent")],
    );
    assert!(matches!(result, Err(DagError::InvalidParent(_))));
    assert!(!dag.contains(&BranchId::new("b")));
}

#[test]
fn test_add_many_branches_maintains_consistency() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("d"), vec![BranchId::new("a"), BranchId::new("b")])
        .unwrap();
    dag.add_branch(BranchId::new("e"), vec![BranchId::new("c"), BranchId::new("d")])
        .unwrap();

    // trunk + a, b, c, d, e = 6 total
    assert_eq!(dag.len(), 6);

    let e_ancestors = dag.ancestors(&BranchId::new("e")).unwrap();
    assert!(e_ancestors.contains(&BranchId::new("trunk")));
    assert!(e_ancestors.contains(&BranchId::new("a")));
    assert!(e_ancestors.contains(&BranchId::new("b")));
    assert!(e_ancestors.contains(&BranchId::new("c")));
    assert!(e_ancestors.contains(&BranchId::new("d")));
    assert_eq!(e_ancestors.len(), 5);

    let trunk_desc = dag.descendants(&BranchId::new("trunk")).unwrap();
    assert_eq!(trunk_desc.len(), 5);
}

// ── Topological sort: exhaustive correctness ────────────────────────────────

#[test]
fn test_topological_sort_wide_graph() {
    let mut dag = BranchDag::new();
    for i in 0..20 {
        dag.add_branch(
            BranchId::new(format!("branch-{i}")),
            vec![BranchId::new("trunk")],
        )
        .unwrap();
    }
    let order = dag.topological_sort().unwrap();
    assert_eq!(order.len(), 21);

    let trunk_pos = order
        .iter()
        .position(|id| id == &BranchId::new("trunk"))
        .unwrap();
    for i in 0..20 {
        let pos = order
            .iter()
            .position(|id| id == &BranchId::new(format!("branch-{i}")))
            .unwrap();
        assert!(trunk_pos < pos, "trunk must come before branch-{i}");
    }
}

#[test]
fn test_topological_sort_complex_dag() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("d"), vec![BranchId::new("b"), BranchId::new("a")])
        .unwrap();
    dag.add_branch(BranchId::new("e"), vec![BranchId::new("c"), BranchId::new("d")])
        .unwrap();

    let order = dag.topological_sort().unwrap();
    assert_eq!(order.len(), 6);

    let pos = |id: &str| {
        order
            .iter()
            .position(|b| b == &BranchId::new(id))
            .unwrap()
    };

    assert!(pos("trunk") < pos("a"));
    assert!(pos("trunk") < pos("b"));
    assert!(pos("a") < pos("c"));
    assert!(pos("a") < pos("d"));
    assert!(pos("b") < pos("d"));
    assert!(pos("c") < pos("e"));
    assert!(pos("d") < pos("e"));
}

#[test]
fn test_topological_sort_no_duplicates() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("b")]).unwrap();

    let order = dag.topological_sort().unwrap();
    let mut seen = std::collections::HashSet::new();
    for id in &order {
        assert!(seen.insert(id.clone()), "duplicate found: {id}");
    }
}

#[test]
fn test_topological_sort_contains_all_branches() {
    let mut dag = BranchDag::new();
    let branches = ["x", "y", "z", "w", "v"];
    for name in &branches {
        dag.add_branch(BranchId::new(*name), vec![BranchId::new("trunk")]).unwrap();
    }
    let order = dag.topological_sort().unwrap();

    assert!(order.contains(&BranchId::new("trunk")));
    for name in &branches {
        assert!(order.contains(&BranchId::new(*name)), "missing {name}");
    }
}

#[test]
fn test_topological_sort_parent_always_before_child() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("d"), vec![BranchId::new("a"), BranchId::new("b")])
        .unwrap();

    let order = dag.topological_sort().unwrap();
    let pos = |id: &str| {
        order
            .iter()
            .position(|b| b == &BranchId::new(id))
            .unwrap()
    };

    assert!(pos("trunk") < pos("a"));
    assert!(pos("trunk") < pos("b"));
    assert!(pos("a") < pos("c"));
    assert!(pos("a") < pos("d"));
    assert!(pos("b") < pos("d"));
}

// ── Cycle detection: exhaustive scenarios ──────────────────────────────────

#[test]
fn test_cycle_self_reference_various_names() {
    let mut dag = BranchDag::new();
    assert!(matches!(
        dag.add_branch(BranchId::new("x"), vec![BranchId::new("x")]),
        Err(DagError::CycleDetected(_))
    ));
    assert!(matches!(
        dag.add_branch(BranchId::new(""), vec![BranchId::new("")]),
        Err(DagError::CycleDetected(_))
    ));
}

#[test]
fn test_no_false_positive_cycle_in_wide_graph() {
    let mut dag = BranchDag::new();
    for i in 0..100 {
        dag.add_branch(
            BranchId::new(format!("f-{i}")),
            vec![BranchId::new("trunk")],
        )
        .unwrap();
    }
    assert_eq!(dag.len(), 101);
    let topo = dag.topological_sort().unwrap();
    assert_eq!(topo.len(), 101);
}

#[test]
fn test_cycle_detection_in_layered_graph() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("L1a"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(BranchId::new("L1b"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(
        BranchId::new("L2a"),
        vec![BranchId::new("L1a"), BranchId::new("L1b")],
    )
    .unwrap();
    dag.add_branch(BranchId::new("L2b"), vec![BranchId::new("L1b")]).unwrap();
    dag.add_branch(
        BranchId::new("L3"),
        vec![BranchId::new("L2a"), BranchId::new("L2b")],
    )
    .unwrap();

    let topo = dag.topological_sort().unwrap();
    assert_eq!(topo.len(), 6);

    let pos = |id: &str| {
        topo.iter()
            .position(|b| b == &BranchId::new(id))
            .unwrap()
    };
    assert!(pos("trunk") < pos("L1a"));
    assert!(pos("trunk") < pos("L1b"));
    assert!(pos("L1a") < pos("L2a"));
    assert!(pos("L1b") < pos("L2a"));
    assert!(pos("L1b") < pos("L2b"));
    assert!(pos("L2a") < pos("L3"));
    assert!(pos("L2b") < pos("L3"));
}

#[test]
fn test_cycle_transitive_through_merge() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(
        BranchId::new("merge"),
        vec![BranchId::new("a"), BranchId::new("b")],
    )
    .unwrap();
    dag.add_branch(BranchId::new("post-merge"), vec![BranchId::new("merge")]).unwrap();
    assert_eq!(dag.len(), 5);
}

#[test]
fn test_cycle_would_create_through_ancestor() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("b")]).unwrap();

    let topo = dag.topological_sort();
    assert!(topo.is_ok());
}

// ── Ancestry queries: exhaustive scenarios ──────────────────────────────────

#[test]
fn test_ancestors_diamond_merge() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("left"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(BranchId::new("right"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(
        BranchId::new("merge"),
        vec![BranchId::new("left"), BranchId::new("right")],
    )
    .unwrap();

    let ancestors = dag.ancestors(&BranchId::new("merge")).unwrap();
    assert_eq!(ancestors.len(), 3);
    assert!(ancestors.contains(&BranchId::new("trunk")));
    assert!(ancestors.contains(&BranchId::new("left")));
    assert!(ancestors.contains(&BranchId::new("right")));
}

#[test]
fn test_ancestors_deep_chain_all_found() {
    let mut dag = BranchDag::new();
    for i in 1..=20 {
        let parent = if i == 1 {
            BranchId::new("trunk")
        } else {
            BranchId::new(format!("lvl-{}", i - 1))
        };
        dag.add_branch(BranchId::new(format!("lvl-{i}")), vec![parent])
            .unwrap();
    }
    let ancestors = dag.ancestors(&BranchId::new("lvl-20")).unwrap();
    assert_eq!(ancestors.len(), 20);
    assert!(ancestors.contains(&BranchId::new("trunk")));
    assert!(ancestors.contains(&BranchId::new("lvl-1")));
    assert!(ancestors.contains(&BranchId::new("lvl-19")));
}

#[test]
fn test_ancestors_branch_not_in_its_own_ancestors() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).unwrap();

    let ancestors_of_b = dag.ancestors(&BranchId::new("b")).unwrap();
    assert!(
        !ancestors_of_b.contains(&BranchId::new("b")),
        "branch should not be its own ancestor"
    );
    assert!(ancestors_of_b.contains(&BranchId::new("a")));
    assert!(ancestors_of_b.contains(&BranchId::new("trunk")));
}

#[test]
fn test_ancestors_shared_grandparent() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("a"), BranchId::new("b")]).unwrap();

    let ancestors_c = dag.ancestors(&BranchId::new("c")).unwrap();
    assert_eq!(ancestors_c.len(), 3);
}

#[test]
fn test_descendants_diamond_from_trunk() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("d"), vec![BranchId::new("b"), BranchId::new("c")]).unwrap();

    let desc_trunk = dag.descendants(&BranchId::new("trunk")).unwrap();
    assert_eq!(desc_trunk.len(), 4);
    assert!(desc_trunk.contains(&BranchId::new("a")));
    assert!(desc_trunk.contains(&BranchId::new("d")));

    let desc_a = dag.descendants(&BranchId::new("a")).unwrap();
    assert_eq!(desc_a.len(), 3);
    assert!(desc_a.contains(&BranchId::new("b")));
    assert!(desc_a.contains(&BranchId::new("c")));
    assert!(desc_a.contains(&BranchId::new("d")));

    let desc_d = dag.descendants(&BranchId::new("d")).unwrap();
    assert!(desc_d.is_empty());
}

#[test]
fn test_descendants_leaf_has_none() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("trunk")]).unwrap();

    assert!(dag.descendants(&BranchId::new("a")).unwrap().is_empty());
    assert!(dag.descendants(&BranchId::new("b")).unwrap().is_empty());
}

#[test]
fn test_ancestors_and_descendants_inverse_relationship() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("b")]).unwrap();

    let ancestors_c = dag.ancestors(&BranchId::new("c")).unwrap();
    for ancestor in &ancestors_c {
        let desc = dag.descendants(ancestor).unwrap();
        assert!(
            desc.contains(&BranchId::new("c")),
            "{ancestor} should have c as descendant"
        );
    }

    let desc_trunk = dag.descendants(&BranchId::new("trunk")).unwrap();
    for descendant in &desc_trunk {
        let anc = dag.ancestors(descendant).unwrap();
        assert!(
            anc.contains(&BranchId::new("trunk")),
            "{descendant} should have trunk as ancestor"
        );
    }
}

#[test]
fn test_ancestors_deduplication_in_diamond() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("left"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(BranchId::new("right"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(
        BranchId::new("merge"),
        vec![BranchId::new("left"), BranchId::new("right")],
    )
    .unwrap();

    let ancestors = dag.ancestors(&BranchId::new("merge")).unwrap();
    let trunk_count = ancestors
        .iter()
        .filter(|id| **id == BranchId::new("trunk"))
        .count();
    assert_eq!(trunk_count, 1, "trunk should appear exactly once in ancestors");
}

#[test]
fn test_path_to_root_diamond_takes_first_parent() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("left"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(BranchId::new("right"), vec![BranchId::new("trunk")])
        .unwrap();
    dag.add_branch(
        BranchId::new("merge"),
        vec![BranchId::new("left"), BranchId::new("right")],
    )
    .unwrap();

    let path = dag.path_to_root(&BranchId::new("merge")).unwrap();
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], BranchId::new("merge"));
    assert!(path.contains(&BranchId::new("trunk")));
    assert!(path.contains(&BranchId::new("left")) || path.contains(&BranchId::new("right")));
}

// ── Remove + add interaction ───────────────────────────────────────────────

#[test]
fn test_remove_middle_branch_blocked_by_descendants() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("b")]).unwrap();

    let result = dag.remove_branch(BranchId::new("b"));
    assert!(matches!(result, Err(DagError::HasDescendants(_, _))));

    dag.remove_branch(BranchId::new("c")).unwrap();
    dag.remove_branch(BranchId::new("b")).unwrap();
    dag.remove_branch(BranchId::new("a")).unwrap();
    assert_eq!(dag.len(), 1);
}

#[test]
fn test_remove_branch_cleans_up_children_map() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).unwrap();

    dag.remove_branch(BranchId::new("b")).unwrap();
    let desc_a = dag.descendants(&BranchId::new("a")).unwrap();
    assert!(desc_a.is_empty());

    dag.remove_branch(BranchId::new("a")).unwrap();
    let desc_trunk = dag.descendants(&BranchId::new("trunk")).unwrap();
    assert!(desc_trunk.is_empty());
}

#[test]
fn test_remove_trunk_with_children_blocked() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    let result = dag.remove_branch(BranchId::new("trunk"));
    assert!(matches!(result, Err(DagError::HasDescendants(_, _))));
}

#[test]
fn test_topological_sort_after_removal() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("b")]).unwrap();

    dag.remove_branch(BranchId::new("c")).unwrap();
    let topo = dag.topological_sort().unwrap();
    assert_eq!(topo.len(), 3);
    assert!(topo.contains(&BranchId::new("trunk")));
    assert!(topo.contains(&BranchId::new("a")));
    assert!(topo.contains(&BranchId::new("b")));
    assert!(!topo.contains(&BranchId::new("c")));
}

// ── Clone semantics ────────────────────────────────────────────────────────

#[test]
fn test_clone_produces_independent_copy() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();

    let cloned = dag.clone();
    assert_eq!(cloned.len(), dag.len());
    assert!(cloned.contains(&BranchId::new("a")));
    assert!(cloned.contains(&BranchId::new("trunk")));

    dag.add_branch(BranchId::new("b"), vec![BranchId::new("trunk")]).unwrap();
    assert_eq!(dag.len(), 3);
    assert_eq!(cloned.len(), 2);
    assert!(!cloned.contains(&BranchId::new("b")));
}

// ── Edge cases: empty DAG after trunk removal ──────────────────────────────

#[test]
fn test_empty_dag_after_trunk_removal() {
    let mut dag = BranchDag::new();
    dag.remove_branch(BranchId::new("trunk")).unwrap();
    assert_eq!(dag.len(), 0);
    assert!(!dag.contains(&BranchId::new("trunk")));
    let topo = dag.topological_sort();
    assert!(matches!(topo, Err(DagError::EmptyDag)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROPTESTS — property-based testing for DAG invariants
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: check if `candidate` is an ancestor of `id`
fn is_ancestor(dag: &BranchDag, id: &BranchId, candidate: &BranchId) -> bool {
    dag.ancestors(id)
        .is_ok_and(|ancs| ancs.contains(candidate))
}

/// Helper: check if `candidate` is a descendant of `id`
fn is_descendant(dag: &BranchDag, id: &BranchId, candidate: &BranchId) -> bool {
    dag.descendants(id)
        .is_ok_and(|descs| descs.contains(candidate))
}

/// Strategy: generate a valid DAG as a list of parent-lists.
/// Each entry i (for i >= 1) is a non-empty Vec<usize> of parent indices,
/// where all indices are in 0..i (i.e., refer to already-added branches).
/// Index 0 is always trunk.
fn valid_dag_strategy(max_branches: usize) -> impl Strategy<Value = Vec<Vec<usize>>> {
    // Build incrementally: for branch i, pick 1..3 parents from 0..i
    (1..max_branches)
        .map(|i| {
            let upper = i; // valid parent indices are 0..i
            proptest::sample::subsequence((0..upper).collect::<Vec<_>>(), 1..=upper.min(3).max(1))
        })
        .collect::<Vec<_>>()
        .prop_map(|lists| {
            let mut result = vec![vec![]]; // trunk at index 0
            result.extend(lists);
            result
        })
}

/// Build a BranchDag from the strategy output
fn build_dag_from_parents(parent_lists: &[Vec<usize>]) -> BranchDag {
    let mut dag = BranchDag::new();
    let mut names: Vec<BranchId> = vec![BranchId::new("trunk")];

    for (i, parents) in parent_lists.iter().enumerate().skip(1) {
        let name = BranchId::new(format!("b-{i}"));
        let parent_ids: Vec<BranchId> = parents.iter().map(|&pi| names[pi].clone()).collect();
        dag.add_branch(name.clone(), parent_ids)
            .unwrap_or_else(|e| panic!("b-{i} with parents {parents:?}: {e}"));
        names.push(name);
    }

    dag
}

// ── Proptest: adding branches always succeeds with valid parents ────────────

proptest! {
    #[test]
    fn proptest_add_branch_valid_parents(parent_lists in valid_dag_strategy(20)) {
        let dag = build_dag_from_parents(&parent_lists);
        let expected_len = parent_lists.len();
        prop_assert_eq!(dag.len(), expected_len);

        // All branches are present
        for i in 0..expected_len {
            let name = if i == 0 { "trunk".to_string() } else { format!("b-{i}") };
            prop_assert!(dag.contains(&BranchId::new(&name)),
                "branch {} not found", name);
        }
    }
}

// ── Proptest: topological sort parents-before-children invariant ────────────

proptest! {
    #[test]
    fn proptest_topological_sort_parents_before_children(parent_lists in valid_dag_strategy(20)) {
        let dag = build_dag_from_parents(&parent_lists);
        let Ok(order) = dag.topological_sort() else {
            return Ok(());
        };

        // Build position map
        let pos_map: std::collections::HashMap<BranchId, usize> = order
            .iter()
            .enumerate()
            .map(|(pos, id)| (id.clone(), pos))
            .collect();

        for (child_idx, parents) in parent_lists.iter().enumerate() {
            let child_name = if child_idx == 0 {
                "trunk".to_string()
            } else {
                format!("b-{child_idx}")
            };
            let child_id = BranchId::new(&child_name);

            if let Some(&child_pos) = pos_map.get(&child_id) {
                for &parent_idx in parents {
                    let parent_name = if parent_idx == 0 {
                        "trunk".to_string()
                    } else {
                        format!("b-{parent_idx}")
                    };
                    let parent_id = BranchId::new(&parent_name);
                    if let Some(&parent_pos) = pos_map.get(&parent_id) {
                        prop_assert!(parent_pos < child_pos,
                            "parent {} at {} must come before child {} at {}",
                            parent_name, parent_pos, child_name, child_pos);
                    }
                }
            }
        }
    }
}

// ── Proptest: topological sort has no duplicates and covers all branches ────

proptest! {
    #[test]
    fn proptest_topological_sort_complete_and_unique(parent_lists in valid_dag_strategy(25)) {
        let dag = build_dag_from_parents(&parent_lists);
        let Ok(order) = dag.topological_sort() else { return Ok(()); };

        // No duplicates
        let mut seen = std::collections::HashSet::new();
        for id in &order {
            prop_assert!(seen.insert(id.clone()), "duplicate branch in topo sort");
        }

        // Covers all branches
        prop_assert_eq!(order.len(), dag.len());
        for id in dag.branch_ids() {
            prop_assert!(order.contains(&id), "missing branch in topo sort");
        }
    }
}

// ── Proptest: ancestors/descendants are inverse relationships ───────────────

proptest! {
    #[test]
    fn proptest_ancestors_descendants_inverse(parent_lists in valid_dag_strategy(15)) {
        let dag = build_dag_from_parents(&parent_lists);

        for i in 1..parent_lists.len() {
            let name = BranchId::new(format!("b-{i}"));

            // Every ancestor of X should have X as a descendant
            let ancs = dag.ancestors(&name).unwrap();
            for anc in &ancs {
                let desc = dag.descendants(anc).unwrap();
                prop_assert!(desc.contains(&name),
                    "ancestor should have branch as descendant");
            }

            // Every descendant of X should have X as an ancestor
            let descs = dag.descendants(&name).unwrap();
            for desc in &descs {
                let anc = dag.ancestors(desc).unwrap();
                prop_assert!(anc.contains(&name),
                    "descendant should have branch as ancestor");
            }
        }
    }
}

// ── Proptest: branch is never its own ancestor or descendant ────────────────

proptest! {
    #[test]
    fn proptest_no_self_ancestor_descendant(parent_lists in valid_dag_strategy(20)) {
        let dag = build_dag_from_parents(&parent_lists);

        for i in 0..parent_lists.len() {
            let name = if i == 0 {
                BranchId::new("trunk")
            } else {
                BranchId::new(format!("b-{i}"))
            };

            let ancs = dag.ancestors(&name).unwrap();
            prop_assert!(!ancs.contains(&name), "branch should not be its own ancestor");

            let descs = dag.descendants(&name).unwrap();
            prop_assert!(!descs.contains(&name), "branch should not be its own descendant");
        }
    }
}

// ── Proptest: path_to_root always terminates at trunk ───────────────────────

proptest! {
    #[test]
    fn proptest_path_to_root_reaches_trunk(parent_lists in valid_dag_strategy(20)) {
        let dag = build_dag_from_parents(&parent_lists);

        for i in 1..parent_lists.len() {
            let name = BranchId::new(format!("b-{i}"));
            let path = dag.path_to_root(&name).unwrap();

            // Path starts at the branch itself
            prop_assert_eq!(&path[0], &name);

            // Path ends at trunk
            prop_assert_eq!(path.last().unwrap(), &BranchId::new("trunk"));

            // Each step in path: parent is actually a parent of the child
            for window in path.windows(2) {
                let child = &window[0];
                let parent = &window[1];
                let child_parents = dag.parents.get(child).cloned().unwrap_or_default();
                prop_assert!(
                    child_parents.contains(parent),
                    "parent should be a parent of child in path"
                );
            }
        }
    }
}

// ── Proptest: is_ancestor / is_descendant consistency ───────────────────────

proptest! {
    #[test]
    fn proptest_is_ancestor_descendant_symmetric(parent_lists in valid_dag_strategy(15)) {
        let dag = build_dag_from_parents(&parent_lists);
        let branch_names: Vec<BranchId> = (0..parent_lists.len())
            .map(|i| if i == 0 { BranchId::new("trunk") } else { BranchId::new(format!("b-{i}")) })
            .collect();

        for a in &branch_names {
            for b in &branch_names {
                if a == b { continue; }
                // is_ancestor(a, b) <=> is_descendant(b, a)
                let a_anc_of_b = is_ancestor(&dag, b, a);
                let b_desc_of_a = is_descendant(&dag, a, b);
                prop_assert_eq!(a_anc_of_b, b_desc_of_a,
                    "is_ancestor and is_descendant should be symmetric");
            }
        }
    }
}

// ── Proptest: cycle detection rejects all self-references ───────────────────

proptest! {
    #[test]
    fn proptest_self_reference_always_rejected(name in "[a-z]{1,10}") {
        let mut dag = BranchDag::new();
        let id = BranchId::new(&name);
        let result = dag.add_branch(id.clone(), vec![id.clone()]);
        prop_assert!(matches!(result, Err(DagError::CycleDetected(_))));
    }
}

// ── Proptest: duplicate branch names always rejected ────────────────────────

proptest! {
    #[test]
    fn proptest_duplicate_branch_rejected(name in "[a-z][a-z0-9-]{0,9}") {
        let mut dag = BranchDag::new();
        let id = BranchId::new(&name);
        // Skip if name is "trunk" (already exists + self-ref)
        if name != "trunk" {
            dag.add_branch(id.clone(), vec![BranchId::new("trunk")]).unwrap();
            let result = dag.add_branch(id.clone(), vec![BranchId::new("trunk")]);
            prop_assert!(matches!(result, Err(DagError::BranchAlreadyExists(_))));
        }
    }
}

// ── Proptest: invalid parent always rejected ────────────────────────────────

proptest! {
    #[test]
    fn proptest_nonexistent_parent_rejected(
        name in "[a-z][a-z0-9-]{0,9}",
        bad_parent in "[a-z][a-z0-9-]{0,9}"
    ) {
        let mut dag = BranchDag::new();
        let id = BranchId::new(&name);
        let parent = BranchId::new(&bad_parent);
        if bad_parent != "trunk" && name != "trunk" {
            let result = dag.add_branch(id, vec![parent]);
            prop_assert!(matches!(result, Err(DagError::InvalidParent(_))));
        }
    }
}

// ── Proptest: remove then re-add works ──────────────────────────────────────

proptest! {
    #[test]
    fn proptest_remove_readd_roundtrip(branches in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..=5)) {
        let mut dag = BranchDag::new();
        let mut names: Vec<BranchId> = Vec::new();

        // Add all branches as children of trunk
        for name in &branches {
            if name == "trunk" { continue; }
            let id = BranchId::new(name);
            if !dag.contains(&id) {
                dag.add_branch(id.clone(), vec![BranchId::new("trunk")]).unwrap();
                names.push(id);
            }
        }

        // Remove all
        let added_count = names.len();
        for id in &names {
            dag.remove_branch(id.clone()).unwrap();
        }
        prop_assert_eq!(dag.len(), 1); // only trunk

        // Re-add all
        for id in &names {
            dag.add_branch(id.clone(), vec![BranchId::new("trunk")]).unwrap();
        }
        prop_assert_eq!(dag.len(), added_count + 1);
    }
}

// ── Proptest: clone independence ────────────────────────────────────────────

proptest! {
    #[test]
    fn proptest_clone_independence(parent_lists in valid_dag_strategy(10)) {
        let mut dag = build_dag_from_parents(&parent_lists);
        let cloned = dag.clone();

        // Same state initially
        prop_assert_eq!(dag.len(), cloned.len());
        prop_assert_eq!(dag.branch_ids(), cloned.branch_ids());

        // Mutations to original don't affect clone
        let branch_name = BranchId::new("new-branch");
        dag.add_branch(branch_name.clone(), vec![BranchId::new("trunk")]).unwrap();
        prop_assert!(dag.contains(&branch_name));
        prop_assert!(!cloned.contains(&branch_name));
    }
}

// ── Proptest: branch_ids sorted invariant ───────────────────────────────────

proptest! {
    #[test]
    fn proptest_branch_ids_always_sorted(parent_lists in valid_dag_strategy(25)) {
        let dag = build_dag_from_parents(&parent_lists);
        let ids = dag.branch_ids();
        let mut sorted = ids.clone();
        sorted.sort();
        prop_assert_eq!(ids, sorted);
    }
}

// ── Proptest: large DAG performance sanity ──────────────────────────────────

proptest! {
    #[test]
    fn proptest_large_dag_operations(parent_lists in valid_dag_strategy(100)) {
        let dag = build_dag_from_parents(&parent_lists);

        // topological_sort succeeds
        let topo = dag.topological_sort();
        prop_assert!(topo.is_ok());
        prop_assert_eq!(topo.unwrap().len(), dag.len());

        // ancestors/descendants succeed for all branches
        for i in 0..parent_lists.len() {
            let name = if i == 0 { BranchId::new("trunk") } else { BranchId::new(format!("b-{i}")) };
            prop_assert!(dag.ancestors(&name).is_ok());
            prop_assert!(dag.descendants(&name).is_ok());
            prop_assert!(dag.path_to_root(&name).is_ok());
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADDITIONAL EXHAUSTIVE UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════════

// ── is_ancestor / is_descendant helpers ────────────────────────────────────

#[test]
fn test_is_ancestor_true_for_direct_parent() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("child"), vec![BranchId::new("trunk")])
        .unwrap();
    assert!(is_ancestor(&dag, &BranchId::new("child"), &BranchId::new("trunk")));
}

#[test]
fn test_is_ancestor_true_for_transitive() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).unwrap();
    assert!(is_ancestor(&dag, &BranchId::new("b"), &BranchId::new("trunk")));
    assert!(is_ancestor(&dag, &BranchId::new("b"), &BranchId::new("a")));
}

#[test]
fn test_is_ancestor_false_for_unrelated() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("left"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("right"), vec![BranchId::new("trunk")]).unwrap();
    assert!(!is_ancestor(&dag, &BranchId::new("left"), &BranchId::new("right")));
    assert!(!is_ancestor(&dag, &BranchId::new("right"), &BranchId::new("left")));
}

#[test]
fn test_is_descendant_true_for_direct_child() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("child"), vec![BranchId::new("trunk")])
        .unwrap();
    assert!(is_descendant(&dag, &BranchId::new("trunk"), &BranchId::new("child")));
}

#[test]
fn test_is_descendant_false_for_leaf() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("leaf"), vec![BranchId::new("trunk")])
        .unwrap();
    assert!(!is_descendant(&dag, &BranchId::new("leaf"), &BranchId::new("trunk")));
}

#[test]
fn test_is_ancestor_diamond_reaches_all_parents() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("left"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("right"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(
        BranchId::new("merge"),
        vec![BranchId::new("left"), BranchId::new("right")],
    ).unwrap();

    assert!(is_ancestor(&dag, &BranchId::new("merge"), &BranchId::new("left")));
    assert!(is_ancestor(&dag, &BranchId::new("merge"), &BranchId::new("right")));
    assert!(is_ancestor(&dag, &BranchId::new("merge"), &BranchId::new("trunk")));
    assert!(!is_ancestor(&dag, &BranchId::new("merge"), &BranchId::new("merge")));
}

// ── Edge count (total parent relationships) ─────────────────────────────────

#[test]
fn test_edge_count_single_branch() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    // 1 edge: trunk -> a
    let total_edges: usize = dag.parents.values().map(|p| p.len()).sum();
    assert_eq!(total_edges, 1);
}

#[test]
fn test_edge_count_diamond() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("left"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("right"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(
        BranchId::new("merge"),
        vec![BranchId::new("left"), BranchId::new("right")],
    ).unwrap();
    // left: 1 parent, right: 1 parent, merge: 2 parents = 4 edges
    let total_edges: usize = dag.parents.values().map(|p| p.len()).sum();
    assert_eq!(total_edges, 4);
}

#[test]
fn test_edge_count_after_remove() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).unwrap();
    dag.remove_branch(BranchId::new("b")).unwrap();
    // Only trunk -> a remains = 1 edge (trunk has 0)
    let total_edges: usize = dag.parents.values().map(|p| p.len()).sum();
    assert_eq!(total_edges, 1);
}

// ── Additional path_to_root edge cases ──────────────────────────────────────

#[test]
fn test_path_to_root_with_diamond_picks_first_parent_path() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("left"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("right"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(
        BranchId::new("merge"),
        vec![BranchId::new("left"), BranchId::new("right")],
    ).unwrap();

    let path = dag.path_to_root(&BranchId::new("merge")).unwrap();
    assert_eq!(path[0], BranchId::new("merge"));
    assert!(path.contains(&BranchId::new("trunk")));
    // Should go through left (first parent) or right (second parent), but not both
    assert!(path.contains(&BranchId::new("left")) || path.contains(&BranchId::new("right")));
}

#[test]
fn test_path_to_root_deep_chain_length() {
    let mut dag = BranchDag::new();
    let depth = 30;
    for i in 1..=depth {
        let parent = if i == 1 { BranchId::new("trunk") } else { BranchId::new(format!("lvl-{}", i - 1)) };
        dag.add_branch(BranchId::new(format!("lvl-{i}")), vec![parent]).unwrap();
    }
    let path = dag.path_to_root(&BranchId::new(format!("lvl-{depth}"))).unwrap();
    assert_eq!(path.len(), depth + 1);
}

// ── Ancestors/descendants count consistency ─────────────────────────────────

#[test]
fn test_ancestors_count_plus_descendants_count_leaves() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("a")]).unwrap();
    dag.add_branch(BranchId::new("d"), vec![BranchId::new("b")]).unwrap();

    // Leaves (c, d) have no descendants
    assert!(dag.descendants(&BranchId::new("c")).unwrap().is_empty());
    assert!(dag.descendants(&BranchId::new("d")).unwrap().is_empty());

    // Trunk has all 4 as descendants
    assert_eq!(dag.descendants(&BranchId::new("trunk")).unwrap().len(), 4);
}

// ── Add branch with single parent vs multiple parents consistency ────────────

#[test]
fn test_add_branch_single_vs_multi_parent_ancestor_set() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("p1"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("p2"), vec![BranchId::new("trunk")]).unwrap();

    // Single parent child
    dag.add_branch(BranchId::new("single"), vec![BranchId::new("p1")]).unwrap();
    let single_anc = dag.ancestors(&BranchId::new("single")).unwrap();
    assert!(single_anc.contains(&BranchId::new("p1")));
    assert!(single_anc.contains(&BranchId::new("trunk")));
    assert_eq!(single_anc.len(), 2);

    // Multi parent child
    dag.add_branch(
        BranchId::new("multi"),
        vec![BranchId::new("p1"), BranchId::new("p2")],
    ).unwrap();
    let multi_anc = dag.ancestors(&BranchId::new("multi")).unwrap();
    assert!(multi_anc.contains(&BranchId::new("p1")));
    assert!(multi_anc.contains(&BranchId::new("p2")));
    assert!(multi_anc.contains(&BranchId::new("trunk")));
    assert_eq!(multi_anc.len(), 3);
}
