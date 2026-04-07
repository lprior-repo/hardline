//! BranchDag comprehensive test suite.

use super::{BranchDag, BranchId, DagError};

// ── Construction ────────────────────────────────────────────────────────────

#[test]
fn new_returns_dag_with_trunk_branch() {
    let dag = BranchDag::new();
    assert!(dag.contains(&BranchId::new("trunk")));
    assert!(dag.is_trunk(&BranchId::new("trunk")));
    assert_eq!(dag.len(), 1);
}

#[test]
fn default_is_same_as_new() {
    let dag = BranchDag::default();
    assert!(dag.contains(&BranchId::new("trunk")));
    assert_eq!(dag.len(), 1);
}

#[test]
fn is_empty_on_new() {
    let dag = BranchDag::new();
    assert!(dag.is_empty());
    assert_eq!(dag.len(), 1);
}

#[test]
fn is_empty_false_after_add() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    assert!(!dag.is_empty());
    assert_eq!(dag.len(), 2);
}

// ── Add branch operations ───────────────────────────────────────────────────

#[test]
fn add_branch_creates_branch_with_parents() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    assert!(dag.contains(&BranchId::new("feature")));
    assert_eq!(dag.len(), 2);
}

#[test]
fn add_branch_with_multiple_parents() {
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
fn add_branch_already_exists_error() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let result = dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")]);
    assert!(matches!(result, Err(DagError::BranchAlreadyExists(_))));
}

#[test]
fn add_branch_invalid_parent_error() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("feature"), vec![BranchId::new("nonexistent")]);
    assert!(matches!(result, Err(DagError::InvalidParent(_))));
}

#[test]
fn add_branch_no_parent_for_non_trunk_error() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("orphan"), vec![]);
    assert!(matches!(result, Err(DagError::NoParentForBranch(_))));
}

// ── Cycle detection ─────────────────────────────────────────────────────────

#[test]
fn add_branch_self_reference_is_cycle() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("self"), vec![BranchId::new("self")]);
    assert!(matches!(result, Err(DagError::CycleDetected(_))));
}

#[test]
fn add_branch_deep_chain_no_cycle() {
    let mut dag = BranchDag::new();
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
fn remove_branch_success() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    dag.remove_branch(BranchId::new("feature"))
        .expect("Should remove branch");
    assert!(!dag.contains(&BranchId::new("feature")));
    assert_eq!(dag.len(), 1);
}

#[test]
fn remove_branch_not_found_error() {
    let mut dag = BranchDag::new();
    let result = dag.remove_branch(BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

#[test]
fn remove_branch_with_descendants_error() {
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
fn ancestors_single() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let ancestors = dag.ancestors(&BranchId::new("feature")).expect("ancestors");
    assert_eq!(ancestors.len(), 1);
    assert_eq!(ancestors[0], BranchId::new("trunk"));
}

#[test]
fn ancestors_chain() {
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
fn ancestors_trunk_is_empty() {
    let dag = BranchDag::new();
    let ancestors = dag.ancestors(&BranchId::new("trunk")).expect("ancestors");
    assert!(ancestors.is_empty());
}

#[test]
fn ancestors_not_found_error() {
    let dag = BranchDag::new();
    let result = dag.ancestors(&BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

#[test]
fn descendants_single() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let desc = dag.descendants(&BranchId::new("trunk")).expect("descendants");
    assert_eq!(desc.len(), 1);
    assert_eq!(desc[0], BranchId::new("feature"));
}

#[test]
fn descendants_chain() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")])
        .expect("a");
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")])
        .expect("b");
    let desc = dag.descendants(&BranchId::new("trunk")).expect("descendants");
    assert_eq!(desc.len(), 2);
    assert!(desc.contains(&BranchId::new("a")));
    assert!(desc.contains(&BranchId::new("b")));
}

#[test]
fn descendants_leaf_is_empty() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let desc = dag.descendants(&BranchId::new("feature")).expect("descendants");
    assert!(desc.is_empty());
}

#[test]
fn descendants_not_found_error() {
    let dag = BranchDag::new();
    let result = dag.descendants(&BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

// ── Topological ordering ────────────────────────────────────────────────────

#[test]
fn topological_sort_trunk_only() {
    let dag = BranchDag::new();
    let order = dag.topological_sort().expect("topo sort");
    assert_eq!(order.len(), 1);
    assert_eq!(order[0], BranchId::new("trunk"));
}

#[test]
fn topological_sort_chain() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")])
        .expect("a");
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")])
        .expect("b");
    let order = dag.topological_sort().expect("topo sort");
    assert_eq!(order.len(), 3);
    let trunk_pos = order.iter().position(|id| id == &BranchId::new("trunk")).expect("trunk");
    let a_pos = order.iter().position(|id| id == &BranchId::new("a")).expect("a");
    let b_pos = order.iter().position(|id| id == &BranchId::new("b")).expect("b");
    assert!(trunk_pos < a_pos);
    assert!(a_pos < b_pos);
}

#[test]
fn topological_sort_diamond() {
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
    let trunk_pos = order.iter().position(|id| id == &BranchId::new("trunk")).expect("trunk");
    let left_pos = order.iter().position(|id| id == &BranchId::new("left")).expect("left");
    let right_pos = order.iter().position(|id| id == &BranchId::new("right")).expect("right");
    let merge_pos = order.iter().position(|id| id == &BranchId::new("merge")).expect("merge");
    assert!(trunk_pos < left_pos);
    assert!(trunk_pos < right_pos);
    assert!(left_pos < merge_pos);
    assert!(right_pos < merge_pos);
}

// ── Path to root ────────────────────────────────────────────────────────────

#[test]
fn path_to_root_trunk() {
    let dag = BranchDag::new();
    let path = dag.path_to_root(&BranchId::new("trunk")).expect("path");
    assert!(path.is_empty());
}

#[test]
fn path_to_root_single_level() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let path = dag.path_to_root(&BranchId::new("feature")).expect("path");
    assert_eq!(path.len(), 2);
    assert_eq!(path[0], BranchId::new("feature"));
    assert_eq!(path[1], BranchId::new("trunk"));
}

#[test]
fn path_to_root_chain() {
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
fn path_to_root_not_found_error() {
    let dag = BranchDag::new();
    let result = dag.path_to_root(&BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

// ── Branch IDs & accessors ──────────────────────────────────────────────────

#[test]
fn branch_ids_sorted() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("z-branch"), vec![BranchId::new("trunk")])
        .expect("z");
    dag.add_branch(BranchId::new("a-branch"), vec![BranchId::new("trunk")])
        .expect("a");
    let ids = dag.branch_ids();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

#[test]
fn contains_false_for_missing() {
    let dag = BranchDag::new();
    assert!(!dag.contains(&BranchId::new("nonexistent")));
}

#[test]
fn is_trunk_only_for_trunk() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("trunk-imitator"), vec![BranchId::new("trunk")])
        .expect("Should add");
    assert!(dag.is_trunk(&BranchId::new("trunk")));
    assert!(!dag.is_trunk(&BranchId::new("trunk-imitator")));
    assert!(!dag.is_trunk(&BranchId::new("trunk-")));
    assert!(!dag.is_trunk(&BranchId::new("trunk1")));
}
<<<<<<< HEAD

// ═══════════════════════════════════════════════════════════════════════════════
// EXHAUSTIVE TESTS — add nodes, topological sort, cycle detection, ancestry
// ═══════════════════════════════════════════════════════════════════════════════

// ── Add nodes: edge cases & stress ──────────────────────────────────────────

#[test]
fn add_branch_wide_fan_out() {
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
fn add_branch_deep_chain_unique_names() {
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
fn add_branch_diamond_merge_graph() {
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
fn add_branch_after_remove_reuse_name() {
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
fn add_branch_special_characters_in_name() {
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
fn add_branch_empty_string_name() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new(""), vec![BranchId::new("trunk")]).unwrap();
    assert!(dag.contains(&BranchId::new("")));
    assert!(!dag.is_trunk(&BranchId::new("")));
}

#[test]
fn add_branch_unicode_names() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("特性"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("🚀-branch"), vec![BranchId::new("trunk")]).unwrap();
    assert_eq!(dag.len(), 3);
}

#[test]
fn add_branch_cannot_add_trunk_again() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("trunk"), vec![BranchId::new("trunk")]);
    assert!(matches!(result, Err(DagError::CycleDetected(_))));
}

#[test]
fn add_branch_parent_must_exist_all_checked() {
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
fn add_many_branches_maintains_consistency() {
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
fn topological_sort_wide_graph() {
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
fn topological_sort_complex_dag() {
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
fn topological_sort_no_duplicates() {
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
fn topological_sort_contains_all_branches() {
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
fn topological_sort_parent_always_before_child() {
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
fn cycle_self_reference_various_names() {
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
fn no_false_positive_cycle_in_wide_graph() {
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
fn cycle_detection_in_layered_graph() {
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
fn cycle_transitive_through_merge() {
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

// ── Ancestry queries: exhaustive scenarios ──────────────────────────────────

#[test]
fn ancestors_diamond_merge() {
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
fn ancestors_deep_chain_all_found() {
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
fn ancestors_branch_not_in_its_own_ancestors() {
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
fn ancestors_shared_grandparent() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("a"), BranchId::new("b")]).unwrap();

    let ancestors_c = dag.ancestors(&BranchId::new("c")).unwrap();
    assert_eq!(ancestors_c.len(), 3);
}

#[test]
fn descendants_diamond_from_trunk() {
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
fn descendants_leaf_has_none() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("trunk")]).unwrap();

    assert!(dag.descendants(&BranchId::new("a")).unwrap().is_empty());
    assert!(dag.descendants(&BranchId::new("b")).unwrap().is_empty());
}

#[test]
fn ancestors_and_descendants_inverse_relationship() {
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
fn ancestors_deduplication_in_diamond() {
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
fn path_to_root_diamond_takes_first_parent() {
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
fn remove_middle_branch_blocked_by_descendants() {
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
fn remove_branch_cleans_up_children_map() {
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
fn remove_trunk_with_children_blocked() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")]).unwrap();
    let result = dag.remove_branch(BranchId::new("trunk"));
    assert!(matches!(result, Err(DagError::HasDescendants(_, _))));
}

#[test]
fn topological_sort_after_removal() {
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
fn clone_produces_independent_copy() {
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
fn empty_dag_after_trunk_removal() {
    let mut dag = BranchDag::new();
    dag.remove_branch(BranchId::new("trunk")).unwrap();
    assert_eq!(dag.len(), 0);
    assert!(!dag.contains(&BranchId::new("trunk")));
    let topo = dag.topological_sort();
    assert!(matches!(topo, Err(DagError::EmptyDag)));
}
=======
>>>>>>> 44984de (feat: port isolate crate from hardline source (3,151 lines))
