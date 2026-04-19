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
    let desc = dag
        .descendants(&BranchId::new("trunk"))
        .expect("descendants");
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
    let desc = dag
        .descendants(&BranchId::new("trunk"))
        .expect("descendants");
    assert_eq!(desc.len(), 2);
    assert!(desc.contains(&BranchId::new("a")));
    assert!(desc.contains(&BranchId::new("b")));
}

#[test]
fn descendants_leaf_is_empty() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    let desc = dag
        .descendants(&BranchId::new("feature"))
        .expect("descendants");
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
    let trunk_pos = order
        .iter()
        .position(|id| id == &BranchId::new("trunk"))
        .expect("trunk");
    let a_pos = order
        .iter()
        .position(|id| id == &BranchId::new("a"))
        .expect("a");
    let b_pos = order
        .iter()
        .position(|id| id == &BranchId::new("b"))
        .expect("b");
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
    let trunk_pos = order
        .iter()
        .position(|id| id == &BranchId::new("trunk"))
        .expect("trunk");
    let left_pos = order
        .iter()
        .position(|id| id == &BranchId::new("left"))
        .expect("left");
    let right_pos = order
        .iter()
        .position(|id| id == &BranchId::new("right"))
        .expect("right");
    let merge_pos = order
        .iter()
        .position(|id| id == &BranchId::new("merge"))
        .expect("merge");
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
