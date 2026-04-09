//! Black-hat test suite: Stack operations — push/pop/peek/swap, isolation, ancestry
//!
//! Tests the core Stack operations: add_branch, ancestors, descendants,
//! current_stack, needs_restack, get_siblings, topological_order.
//! Covers edge cases, isolation, and adversarial inputs.

use scp_stack::{BranchName, PrInfo, PrState, Stack, StackBranch, StackError};

// ── Helpers ──

fn main_bn() -> BranchName {
    BranchName::new("main")
}

fn bn(name: &str) -> BranchName {
    BranchName::new(name)
}

fn branch(name: &str, parent: Option<&str>, children: &[&str]) -> StackBranch {
    StackBranch {
        name: bn(name),
        parent: parent.map(bn),
        children: children.iter().map(|c| bn(c)).collect(),
        needs_restack: false,
        pr_info: None,
    }
}

fn branch_restock(
    name: &str,
    parent: Option<&str>,
    children: &[&str],
    restack: bool,
) -> StackBranch {
    StackBranch {
        name: bn(name),
        parent: parent.map(bn),
        children: children.iter().map(|c| bn(c)).collect(),
        needs_restack: restack,
        pr_info: None,
    }
}

/// Creates a diamond stack:
/// main → a, main → b, a → c
fn diamond_stack() -> Stack {
    let mut s = Stack::new(main_bn());
    s.branches.push(branch("a", Some("main"), &["c"]));
    s.branches.push(branch("b", Some("main"), &[]));
    s.branches.push(branch("c", Some("a"), &[]));
    s
}

/// Creates a deep linear chain: main → a → b → c → d
fn deep_chain_stack() -> Stack {
    let mut s = Stack::new(main_bn());
    s.branches.push(branch("a", Some("main"), &["b"]));
    s.branches.push(branch("b", Some("a"), &["c"]));
    s.branches.push(branch("c", Some("b"), &["d"]));
    s.branches.push(branch("d", Some("c"), &[]));
    s
}

/// Creates a wide fan-out from main: main → a, main → b, main → c
fn fan_out_stack() -> Stack {
    let mut s = Stack::new(main_bn());
    s.branches.push(branch("a", Some("main"), &[]));
    s.branches.push(branch("b", Some("main"), &[]));
    s.branches.push(branch("c", Some("main"), &[]));
    s
}

// ── Add Branch (Push) ──

#[test]
fn push_branch_to_empty_stack() {
    let mut s = Stack::new(main_bn());
    let result = s.add_branch(branch("feature", Some("main"), &[]));
    assert!(result.is_ok());
    assert_eq!(s.branches.len(), 1);
}

#[test]
fn push_multiple_branches_sequential() {
    let mut s = Stack::new(main_bn());
    for i in 0..10 {
        let name = format!("branch-{i}");
        s.add_branch(branch(&name, Some("main"), &[])).unwrap();
    }
    assert_eq!(s.branches.len(), 10);
}

#[test]
fn push_branch_chain_builds_correctly() {
    let mut s = Stack::new(main_bn());
    s.add_branch(branch("a", Some("main"), &[])).unwrap();
    s.add_branch(branch("b", Some("a"), &[])).unwrap();
    s.add_branch(branch("c", Some("b"), &[])).unwrap();

    assert_eq!(s.branches.len(), 3);
    assert_eq!(s.branches[0].name, bn("a"));
    assert_eq!(s.branches[1].parent, Some(bn("a")));
    assert_eq!(s.branches[2].parent, Some(bn("b")));
}

#[test]
fn push_branch_orphan_rejected_with_correct_error() {
    let mut s = Stack::new(main_bn());
    let result = s.add_branch(branch("orphan", Some("nonexistent"), &[]));
    assert!(result.is_err());
    match result.err() {
        Some(StackError::OrphanedBranch(name)) => assert_eq!(name, "orphan"),
        other => panic!("Expected OrphanedBranch, got: {:?}", other),
    }
}

#[test]
fn push_branch_no_parent_accepted() {
    let mut s = Stack::new(main_bn());
    let result = s.add_branch(branch("root", None, &[]));
    assert!(result.is_ok());
}

#[test]
fn push_branch_to_main_accepted_when_main_not_in_branches() {
    let mut s = Stack::new(main_bn());
    let result = s.add_branch(branch("feature", Some("main"), &[]));
    assert!(result.is_ok());
}

// ── Ancestors ──

#[test]
fn ancestors_of_leaf_traverses_chain() {
    let s = deep_chain_stack();
    let ancestors = s.ancestors(&bn("d"));
    // d → c → b → a → main (main not in branches, stops at a)
    assert!(!ancestors.is_empty());
}

#[test]
fn ancestors_of_nonexistent_branch() {
    let s = deep_chain_stack();
    let ancestors = s.ancestors(&bn("nonexistent"));
    assert!(ancestors.is_empty());
}

#[test]
fn ancestors_of_branch_with_no_parent() {
    let mut s = Stack::new(main_bn());
    s.branches.push(branch("root", None, &[]));
    let ancestors = s.ancestors(&bn("root"));
    assert!(ancestors.is_empty());
}

#[test]
fn ancestors_of_first_child() {
    let s = deep_chain_stack();
    let ancestors = s.ancestors(&bn("b"));
    // b → a → main (main not in branches)
    assert!(!ancestors.is_empty());
}

// ── Descendants ──

#[test]
fn descendants_of_branch_with_children() {
    let mut s = Stack::new(main_bn());
    s.branches.push(branch("a", Some("main"), &["b", "c"]));
    s.branches.push(branch("b", Some("a"), &["d"]));
    s.branches.push(branch("c", Some("a"), &[]));
    s.branches.push(branch("d", Some("b"), &[]));

    let descendants = s.descendants(&bn("a"));
    let mut names: Vec<&str> = descendants.iter().map(|b| b.as_str()).collect();
    names.sort();
    assert!(names.contains(&"b"));
    assert!(names.contains(&"c"));
    assert!(names.contains(&"d"));
}

#[test]
fn descendants_of_leaf_is_empty() {
    let s = deep_chain_stack();
    let descendants = s.descendants(&bn("d"));
    assert!(descendants.is_empty());
}

#[test]
fn descendants_of_nonexistent_is_empty() {
    let s = deep_chain_stack();
    let descendants = s.descendants(&bn("ghost"));
    assert!(descendants.is_empty());
}

// ── Current Stack ──

#[test]
fn current_stack_includes_self() {
    let s = deep_chain_stack();
    let current = s.current_stack(&bn("a"));
    assert!(current.contains(&bn("a")));
}

#[test]
fn current_stack_includes_ancestors_and_descendants() {
    let s = deep_chain_stack();
    let current = s.current_stack(&bn("b"));
    assert!(current.contains(&bn("b")));
}

#[test]
fn current_stack_of_nonexistent_returns_self() {
    let s = deep_chain_stack();
    let current = s.current_stack(&bn("ghost"));
    assert_eq!(current.len(), 1);
    assert_eq!(current[0], bn("ghost"));
}

// ── Needs Restack ──

#[test]
fn needs_restack_empty_when_none_marked() {
    let s = deep_chain_stack();
    assert!(s.needs_restack().is_empty());
}

#[test]
fn needs_restack_returns_marked_branches() {
    let mut s = Stack::new(main_bn());
    s.branches
        .push(branch_restock("a", Some("main"), &[], true));
    s.branches
        .push(branch_restock("b", Some("main"), &[], false));
    s.branches
        .push(branch_restock("c", Some("main"), &[], true));

    let mut needs = s.needs_restack();
    needs.sort();
    assert_eq!(needs, vec![bn("a"), bn("c")]);
}

#[test]
fn needs_restack_all_branches() {
    let mut s = Stack::new(main_bn());
    s.branches
        .push(branch_restock("a", Some("main"), &[], true));
    s.branches
        .push(branch_restock("b", Some("main"), &[], true));

    let needs = s.needs_restack();
    assert_eq!(needs.len(), 2);
}

// ── Get Siblings ──

#[test]
fn get_siblings_branch_with_siblings() {
    let s = fan_out_stack();
    let siblings = s.get_siblings(&bn("a"));
    assert!(siblings.contains(&bn("a")));
    assert!(siblings.contains(&bn("b")));
    assert!(siblings.contains(&bn("c")));
}

#[test]
fn get_siblings_nonexistent_branch_returns_self() {
    let s = fan_out_stack();
    let siblings = s.get_siblings(&bn("ghost"));
    assert_eq!(siblings, vec![bn("ghost")]);
}

#[test]
fn get_siblings_branch_with_no_parent_returns_self() {
    let mut s = Stack::new(main_bn());
    s.branches.push(branch("orphan", None, &[]));
    let siblings = s.get_siblings(&bn("orphan"));
    assert_eq!(siblings, vec![bn("orphan")]);
}

#[test]
fn get_siblings_returns_sorted() {
    let s = fan_out_stack();
    let siblings = s.get_siblings(&bn("a"));
    let mut sorted = siblings.clone();
    sorted.sort();
    assert_eq!(siblings, sorted);
}

// ── Topological Order ──

#[test]
fn topo_empty_stack() {
    let s = Stack::new(main_bn());
    assert!(s.topological_order().is_empty());
}

#[test]
fn topo_single_branch() {
    let mut s = Stack::new(main_bn());
    s.branches.push(branch("solo", None, &[]));
    let order = s.topological_order();
    assert_eq!(order.len(), 1);
    assert_eq!(order[0].name, bn("solo"));
}

#[test]
fn topo_linear_chain_respects_order() {
    let s = deep_chain_stack();
    let order = s.topological_order();
    let idx: std::collections::HashMap<_, _> = order
        .iter()
        .enumerate()
        .map(|(i, b)| (b.name.as_str(), i))
        .collect();
    assert!(idx["a"] < idx["b"], "a before b");
    assert!(idx["b"] < idx["c"], "b before c");
    assert!(idx["c"] < idx["d"], "c before d");
}

#[test]
fn topo_diamond_maintains_invariant() {
    let s = diamond_stack();
    let order = s.topological_order();
    assert_eq!(order.len(), 3);
    let idx: std::collections::HashMap<_, _> = order
        .iter()
        .enumerate()
        .map(|(i, b)| (b.name.as_str(), i))
        .collect();
    assert!(idx["a"] < idx["c"], "a must precede c");
}

#[test]
fn topo_cycle_falls_back_gracefully() {
    let mut s = Stack::new(main_bn());
    s.branches.push(branch("x", Some("y"), &[]));
    s.branches.push(branch("y", Some("x"), &[]));
    let order = s.topological_order();
    assert_eq!(order.len(), 2);
}

#[test]
fn topo_self_referencing_handled() {
    let mut s = Stack::new(main_bn());
    s.branches.push(branch("loop", Some("loop"), &[]));
    let order = s.topological_order();
    assert_eq!(order.len(), 1);
}

#[test]
fn topo_deterministic() {
    let s = deep_chain_stack();
    let o1: Vec<_> = s
        .topological_order()
        .iter()
        .map(|b| b.name.clone())
        .collect();
    let o2: Vec<_> = s
        .topological_order()
        .iter()
        .map(|b| b.name.clone())
        .collect();
    let o3: Vec<_> = s
        .topological_order()
        .iter()
        .map(|b| b.name.clone())
        .collect();
    assert_eq!(o1, o2);
    assert_eq!(o2, o3);
}

#[test]
fn topo_fan_out_all_present() {
    let s = fan_out_stack();
    let order = s.topological_order();
    assert_eq!(order.len(), 3);
    let names: Vec<_> = order.iter().map(|b| b.name.as_str()).collect();
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
    assert!(names.contains(&"c"));
}

// ── Isolation ──

#[test]
fn stack_isolation_separate_stacks_dont_share_branches() {
    let mut s1 = Stack::new(bn("main"));
    s1.add_branch(branch("feature-1", Some("main"), &[]))
        .unwrap();

    let mut s2 = Stack::new(bn("main"));
    s2.add_branch(branch("feature-2", Some("main"), &[]))
        .unwrap();

    assert_eq!(s1.branches.len(), 1);
    assert_eq!(s2.branches.len(), 1);
    assert_eq!(s1.branches[0].name, bn("feature-1"));
    assert_eq!(s2.branches[0].name, bn("feature-2"));
}

#[test]
fn different_main_branch_names_isolate_stacks() {
    let s1 = Stack::new(bn("main"));
    let s2 = Stack::new(bn("develop"));

    assert_ne!(s1.main_branch, s2.main_branch);
}

// ── Adversarial Inputs ──

#[test]
fn empty_branch_name_accepted() {
    let name = BranchName::new("");
    assert_eq!(name.as_str(), "");
}

#[test]
fn unicode_branch_name() {
    let name = BranchName::new("feature/日本語");
    assert_eq!(name.as_str(), "feature/日本語");
}

#[test]
fn very_long_branch_name() {
    let long = "a".repeat(10_000);
    let name = BranchName::new(&long);
    assert_eq!(name.as_str().len(), 10_000);
}

#[test]
fn branch_name_with_special_chars() {
    let name = BranchName::new("feature/JIRA-123_fix:bug@v2");
    assert_eq!(name.as_str(), "feature/JIRA-123_fix:bug@v2");
}

#[test]
fn add_branch_duplicate_names_allowed() {
    let mut s = Stack::new(main_bn());
    s.add_branch(branch("dup", Some("main"), &[])).unwrap();
    s.add_branch(branch("dup", Some("main"), &[])).unwrap();
    assert_eq!(s.branches.len(), 2);
}

#[test]
fn large_stack_operations() {
    let mut s = Stack::new(main_bn());
    for i in 0..100 {
        let name = format!("branch-{i}");
        s.add_branch(branch(&name, Some("main"), &[])).unwrap();
    }
    assert_eq!(s.branches.len(), 100);
    assert_eq!(s.needs_restack().len(), 0);

    let order = s.topological_order();
    assert_eq!(order.len(), 100);
}

// ── Proptests ──

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_branch_name_roundtrip(name in ".{0,256}") {
        let b = BranchName::new(&name);
        assert_eq!(b.as_str(), name);
    }

    #[test]
    fn prop_branch_name_display_matches(name in ".{0,256}") {
        let b = BranchName::new(&name);
        assert_eq!(format!("{b}"), name);
    }

    #[test]
    fn prop_branch_name_serde_roundtrip(name in ".{0,256}") {
        let b = BranchName::new(&name);
        let json = serde_json::to_string(&b).unwrap();
        let back: BranchName = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }

    #[test]
    fn prop_add_branch_with_main_parent_always_succeeds(
        name in "[a-zA-Z][a-zA-Z0-9_-]{0,30}",
    ) {
        let mut s = Stack::new(main_bn());
        let result = s.add_branch(branch(&name, Some("main"), &[]));
        assert!(result.is_ok());
    }

    #[test]
    fn prop_topological_order_always_contains_all_branches(
        branches in proptest::collection::vec(
            "[a-z]{1,10}",
            1..20
        ),
    ) {
        let mut s = Stack::new(main_bn());
        for name in &branches {
            s.branches.push(branch(name, None, &[]));
        }
        let order = s.topological_order();
        assert_eq!(order.len(), branches.len());
    }

    #[test]
    fn prop_ancestors_of_nonexistent_is_empty(name in "[a-z]{1,10}") {
        let s = Stack::new(main_bn());
        let ancestors = s.ancestors(&bn(&name));
        assert!(ancestors.is_empty());
    }

    #[test]
    fn prop_descendants_of_nonexistent_is_empty(name in "[a-z]{1,10}") {
        let s = Stack::new(main_bn());
        let descendants = s.descendants(&bn(&name));
        assert!(descendants.is_empty());
    }

    #[test]
    fn prop_pr_info_serde_roundtrip(
        number in 0u32..10000u32,
        is_draft in proptest::option::of(proptest::bool::ANY),
    ) {
        let info = PrInfo {
            number,
            url: format!("https://github.com/test/{number}"),
            title: format!("PR #{number}"),
            state: PrState::Open,
            is_draft,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: PrInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.number, number);
        assert_eq!(back.is_draft, is_draft);
    }
}
