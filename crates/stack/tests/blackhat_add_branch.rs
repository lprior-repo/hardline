//! Black-hat test suite: Stack::add_branch — exhaustive coverage
//!
//! Covers: happy path, duplicate branches, branch ordering, parent validation,
//! state-independent behavior, edge cases, adversarial inputs.

use scp_stack::{BranchName, PrInfo, PrState, Stack, StackBranch, StackError};

// ── Helpers ──

fn main_bn() -> BranchName {
    BranchName::new("main")
}

fn bn(name: &str) -> BranchName {
    BranchName::new(name)
}

fn branch(name: &str, parent: Option<&str>) -> StackBranch {
    StackBranch {
        name: bn(name),
        parent: parent.map(bn),
        children: vec![],
        needs_restack: false,
        pr_info: None,
    }
}

fn branch_with_children(name: &str, parent: Option<&str>, children: &[&str]) -> StackBranch {
    StackBranch {
        name: bn(name),
        parent: parent.map(bn),
        children: children.iter().map(|c| bn(c)).collect(),
        needs_restack: false,
        pr_info: None,
    }
}

fn branch_with_pr(name: &str, parent: Option<&str>, pr_number: u32) -> StackBranch {
    StackBranch {
        name: bn(name),
        parent: parent.map(bn),
        children: vec![],
        needs_restack: false,
        pr_info: Some(PrInfo {
            number: pr_number,
            url: format!("https://github.com/test/{pr_number}"),
            title: format!("PR for {name}"),
            state: PrState::Open,
            is_draft: Some(false),
        }),
    }
}

fn branch_restack(name: &str, parent: Option<&str>, restack: bool) -> StackBranch {
    StackBranch {
        name: bn(name),
        parent: parent.map(bn),
        children: vec![],
        needs_restack: restack,
        pr_info: None,
    }
}

fn empty_stack() -> Stack {
    Stack::new(main_bn())
}

fn chain_stack(n: usize) -> Stack {
    let mut s = Stack::new(main_bn());
    for i in 0..n {
        let name = format!("branch-{i}");
        let parent = if i == 0 { "main" } else { &format!("branch-{}", i - 1) };
        s.add_branch(branch(&name, Some(parent))).expect("add");
    }
    s
}

// ── 1. Happy path: add branch to Draft stack ──

#[test]
fn happy_path_add_single_branch_to_empty_stack() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("feature", Some("main")));
    assert!(result.is_ok(), "adding a branch with main as parent should succeed");
    assert_eq!(s.branches.len(), 1);
    assert_eq!(s.branches[0].name, bn("feature"));
}

#[test]
fn happy_path_add_multiple_branches_sequentially() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("add a");
    s.add_branch(branch("b", Some("a"))).expect("add b");
    s.add_branch(branch("c", Some("b"))).expect("add c");

    assert_eq!(s.branches.len(), 3);
    assert_eq!(s.branches[0].name, bn("a"));
    assert_eq!(s.branches[1].name, bn("b"));
    assert_eq!(s.branches[2].name, bn("c"));
}

#[test]
fn happy_path_branch_appears_in_stack() {
    let mut s = empty_stack();
    s.add_branch(branch("find-me", Some("main"))).expect("add");
    assert!(s.branches.iter().any(|b| b.name.as_str() == "find-me"));
}

#[test]
fn happy_path_add_branch_with_no_parent_to_empty_stack() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("orphan", None));
    assert!(result.is_ok(), "branch with no parent should be accepted");
    assert_eq!(s.branches.len(), 1);
}

#[test]
fn happy_path_add_branch_parent_is_main() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("feat", Some("main")));
    assert!(result.is_ok());
    assert_eq!(s.branches[0].parent, Some(bn("main")));
}

#[test]
fn happy_path_add_branch_parent_is_existing_branch() {
    let mut s = empty_stack();
    s.add_branch(branch("base", Some("main"))).expect("add base");
    let result = s.add_branch(branch("child", Some("base")));
    assert!(result.is_ok());
    assert_eq!(s.branches[1].parent, Some(bn("base")));
}

#[test]
fn happy_path_deep_chain_builds_correctly() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    s.add_branch(branch("b", Some("a"))).expect("b");
    s.add_branch(branch("c", Some("b"))).expect("c");
    s.add_branch(branch("d", Some("c"))).expect("d");

    assert_eq!(s.branches.len(), 4);
    assert_eq!(s.branches[0].parent, Some(bn("main")));
    assert_eq!(s.branches[1].parent, Some(bn("a")));
    assert_eq!(s.branches[2].parent, Some(bn("b")));
    assert_eq!(s.branches[3].parent, Some(bn("c")));
}

#[test]
fn happy_path_fan_out_from_main() {
    let mut s = empty_stack();
    s.add_branch(branch("feat-1", Some("main"))).expect("1");
    s.add_branch(branch("feat-2", Some("main"))).expect("2");
    s.add_branch(branch("feat-3", Some("main"))).expect("3");

    assert_eq!(s.branches.len(), 3);
    for b in &s.branches {
        assert_eq!(b.parent, Some(bn("main")));
    }
}

// ── 2. Duplicate branch name behavior ──

#[test]
fn duplicate_branch_name_same_name_added_twice() {
    // add_branch does NOT reject duplicates — it pushes unconditionally
    // after parent validation passes. This test documents that behavior.
    let mut s = empty_stack();
    s.add_branch(branch("dup", Some("main"))).expect("first");
    let result = s.add_branch(branch("dup", Some("main")));
    assert!(result.is_ok(), "add_branch allows duplicate names");
    assert_eq!(s.branches.len(), 2);
    assert_eq!(s.branches[0].name, bn("dup"));
    assert_eq!(s.branches[1].name, bn("dup"));
}

#[test]
fn duplicate_branch_name_with_different_parent() {
    let mut s = empty_stack();
    s.add_branch(branch("base", Some("main"))).expect("base");
    s.add_branch(branch("dup", Some("main"))).expect("first dup");
    let result = s.add_branch(branch("dup", Some("base")));
    assert!(result.is_ok(), "duplicate name with different parent accepted");
    assert_eq!(s.branches.len(), 3);
}

#[test]
fn duplicate_branch_name_three_times() {
    let mut s = empty_stack();
    for _ in 0..3 {
        s.add_branch(branch("triple", Some("main"))).expect("add");
    }
    assert_eq!(s.branches.len(), 3);
    for b in &s.branches {
        assert_eq!(b.name, bn("triple"));
    }
}

// ── 3. Branch ordering ──

#[test]
fn ordering_branches_appear_in_insertion_order() {
    let mut s = empty_stack();
    s.add_branch(branch("first", Some("main"))).expect("1");
    s.add_branch(branch("second", Some("first"))).expect("2");
    s.add_branch(branch("third", Some("second"))).expect("3");

    assert_eq!(s.branches[0].name.as_str(), "first");
    assert_eq!(s.branches[1].name.as_str(), "second");
    assert_eq!(s.branches[2].name.as_str(), "third");
}

#[test]
fn ordering_new_branch_always_appended_to_end() {
    let mut s = chain_stack(5);
    s.add_branch(branch("appended", Some("branch-4"))).expect("append");
    assert_eq!(s.branches.last().map(|b| b.name.as_str()), Some("appended"));
}

#[test]
fn ordering_fan_out_preserves_insertion_order() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    s.add_branch(branch("b", Some("main"))).expect("b");
    s.add_branch(branch("c", Some("main"))).expect("c");

    let names: Vec<&str> = s.branches.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);
}

#[test]
fn ordering_topological_order_matches_insertion_for_chain() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    s.add_branch(branch("b", Some("a"))).expect("b");
    s.add_branch(branch("c", Some("b"))).expect("c");

    let topo: Vec<&str> = s.topological_order().iter().map(|b| b.name.as_str()).collect();
    assert_eq!(topo, vec!["a", "b", "c"]);
}

// ── 4. Parent validation ──

#[test]
fn parent_validation_rejects_nonexistent_parent() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("orphan", Some("ghost")));
    assert!(result.is_err());
    let err = result.err().expect("should be error");
    assert!(matches!(err, StackError::OrphanedBranch(_)));
}

#[test]
fn parent_validation_rejects_nonexistent_parent_in_nonempty_stack() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    let result = s.add_branch(branch("b", Some("nonexistent")));
    assert!(result.is_err());
    assert!(matches!(result.err().expect("err"), StackError::OrphanedBranch(_)));
}

#[test]
fn parent_validation_accepts_main_as_parent_even_though_not_in_branches() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("feat", Some("main")));
    assert!(result.is_ok());
    // Verify "main" is NOT in branches but is accepted as parent
    assert!(!s.branches.iter().any(|b| b.name.as_str() == "main"));
}

#[test]
fn parent_validation_accepts_existing_branch_as_parent() {
    let mut s = empty_stack();
    s.add_branch(branch("base", Some("main"))).expect("base");
    let result = s.add_branch(branch("child", Some("base")));
    assert!(result.is_ok());
}

#[test]
fn parent_validation_none_parent_is_allowed() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("no-parent", None));
    assert!(result.is_ok());
}

#[test]
fn parent_validation_orphan_in_nonempty_stack() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    // "b" has parent "x" which is neither a branch nor main
    let result = s.add_branch(branch("b", Some("x")));
    assert!(result.is_err());
    let err_msg = format!("{}", result.err().expect("err"));
    assert!(err_msg.contains("b"), "error should mention the orphaned branch name");
}

#[test]
fn parent_validation_deep_stack_orphan_at_depth() {
    let mut s = chain_stack(10);
    // Try to add branch with parent that doesn't exist
    let result = s.add_branch(branch("lost", Some("nonexistent")));
    assert!(result.is_err());
}

// ── 5. State-independent behavior ──
// add_branch is defined on `impl<S> Stack<S>`, so it works on ALL states.

#[test]
fn state_add_branch_works_on_published_stack() {
    let mut s = empty_stack();
    s.add_branch(branch("before-publish", Some("main"))).expect("add");
    let published = s.publish();
    // After publish, add_branch is no longer available (owned, not &mut)
    // But we can test the impl<S> behavior before transition
    assert_eq!(published.branches.len(), 1);
}

#[test]
fn state_add_branch_before_publish_preserves_on_transition() {
    let mut s = empty_stack();
    s.add_branch(branch("kept", Some("main"))).expect("add");
    let published = s.publish();
    assert!(published.branches.iter().any(|b| b.name.as_str() == "kept"));
}

#[test]
fn state_add_branch_multiple_then_transition_chain() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    s.add_branch(branch("b", Some("a"))).expect("b");
    s.add_branch(branch("c", Some("b"))).expect("c");

    let published = s.publish();
    assert_eq!(published.branches.len(), 3);

    let merging = published.start_merge();
    assert_eq!(merging.branches.len(), 3);

    let merged = merging.complete_merge();
    assert_eq!(merged.branches.len(), 3);
    assert!(merged.is_terminal());
}

// ── 6. Branch field preservation ──

#[test]
fn fields_children_vector_preserved() {
    let mut s = empty_stack();
    let br = branch_with_children("parent", Some("main"), &["child-a", "child-b"]);
    s.add_branch(br).expect("add");
    assert_eq!(s.branches[0].children.len(), 2);
    assert_eq!(s.branches[0].children[0], bn("child-a"));
    assert_eq!(s.branches[0].children[1], bn("child-b"));
}

#[test]
fn fields_pr_info_preserved() {
    let mut s = empty_stack();
    let br = branch_with_pr("pr-branch", Some("main"), 42);
    s.add_branch(br).expect("add");
    let pr = s.branches[0].pr_info.as_ref().expect("should have pr_info");
    assert_eq!(pr.number, 42);
    assert_eq!(pr.title, "PR for pr-branch");
    assert!(matches!(pr.state, PrState::Open));
    assert_eq!(pr.is_draft, Some(false));
}

#[test]
fn fields_needs_restack_false_preserved() {
    let mut s = empty_stack();
    s.add_branch(branch_restack("clean", Some("main"), false)).expect("add");
    assert!(!s.branches[0].needs_restack);
}

#[test]
fn fields_needs_restack_true_preserved() {
    let mut s = empty_stack();
    s.add_branch(branch_restack("dirty", Some("main"), true)).expect("add");
    assert!(s.branches[0].needs_restack);
}

#[test]
fn fields_all_fields_together() {
    let mut s = empty_stack();
    let br = StackBranch {
        name: bn("full"),
        parent: Some(bn("main")),
        children: vec![bn("c1"), bn("c2")],
        needs_restack: true,
        pr_info: Some(PrInfo {
            number: 99,
            url: "https://github.com/test/99".to_string(),
            title: "Full Branch".to_string(),
            state: PrState::Merged,
            is_draft: None,
        }),
    };
    s.add_branch(br).expect("add");

    let stored = &s.branches[0];
    assert_eq!(stored.name, bn("full"));
    assert_eq!(stored.parent, Some(bn("main")));
    assert_eq!(stored.children.len(), 2);
    assert!(stored.needs_restack);
    let pr = stored.pr_info.as_ref().expect("pr");
    assert_eq!(pr.number, 99);
    assert!(matches!(pr.state, PrState::Merged));
}

// ── 7. Edge cases ──

#[test]
fn edge_case_empty_branch_name() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("", Some("main")));
    assert!(result.is_ok(), "empty branch name is accepted");
    assert_eq!(s.branches[0].name.as_str(), "");
}

#[test]
fn edge_case_empty_parent_name() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("feat", Some("")));
    // Empty string is not "main" and not in branches → orphan
    assert!(result.is_err());
}

#[test]
fn edge_case_unicode_branch_name() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("feature/日本語-branch", Some("main")));
    assert!(result.is_ok());
    assert_eq!(s.branches[0].name.as_str(), "feature/日本語-branch");
}

#[test]
fn edge_case_very_long_branch_name() {
    let mut s = empty_stack();
    let long_name = "a".repeat(10_000);
    let result = s.add_branch(branch(&long_name, Some("main")));
    assert!(result.is_ok());
    assert_eq!(s.branches[0].name.as_str().len(), 10_000);
}

#[test]
fn edge_case_branch_name_matches_parent_name() {
    let mut s = empty_stack();
    // "self-loop" has itself as parent, but it's not yet in the stack
    // when validation runs, so it fails
    let result = s.add_branch(branch("self-ref", Some("self-ref")));
    assert!(result.is_err(), "self-referencing as parent fails since not yet in stack");
    assert!(matches!(result.err().expect("err"), StackError::OrphanedBranch(_)));
}

#[test]
fn edge_case_self_reference_after_adding() {
    // First add the branch, then add another with it as parent
    let mut s = empty_stack();
    s.add_branch(branch("existing", Some("main"))).expect("add");
    let result = s.add_branch(branch("existing", Some("existing")));
    assert!(result.is_ok(), "can use an existing branch as parent, even for duplicate name");
}

#[test]
fn edge_case_special_characters_in_name() {
    let mut s = empty_stack();
    let names = vec![
        "feature/JIRA-123",
        "fix/bug#456",
        "release/v1.0.0",
        "test@branch",
        "user/branch%20name",
    ];
    for name in &names {
        s.add_branch(branch(name, Some("main"))).expect("add");
    }
    assert_eq!(s.branches.len(), names.len());
}

#[test]
fn edge_case_add_many_branches() {
    let mut s = empty_stack();
    for i in 0..100 {
        let name = format!("branch-{i}");
        s.add_branch(branch(&name, Some("main"))).expect("add");
    }
    assert_eq!(s.branches.len(), 100);
}

#[test]
fn edge_case_single_stack_branch() {
    let mut s = empty_stack();
    s.add_branch(branch("only", Some("main"))).expect("add");
    assert_eq!(s.branches.len(), 1);
    assert_eq!(s.branches[0].name, bn("only"));
    assert_eq!(s.branches[0].parent, Some(bn("main")));
}

// ── 8. Branch count tracking ──

#[test]
fn count_tracking_empty_stack_has_zero() {
    let s = empty_stack();
    assert_eq!(s.branches.len(), 0);
}

#[test]
fn count_tracking_increments_on_each_add() {
    let mut s = empty_stack();
    for i in 1..=10 {
        s.add_branch(branch(&format!("b-{i}"), Some("main"))).expect("add");
        assert_eq!(s.branches.len(), i);
    }
}

#[test]
fn count_tracking_does_not_increment_on_failure() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    let result = s.add_branch(branch("b", Some("ghost")));
    assert!(result.is_err());
    assert_eq!(s.branches.len(), 1, "failed add should not change count");
}

#[test]
fn count_tracking_after_multiple_failures() {
    let mut s = empty_stack();
    s.add_branch(branch("only", Some("main"))).expect("only");
    for i in 0..5 {
        let _ = s.add_branch(branch(&format!("orphan-{i}"), Some("nonexistent")));
    }
    assert_eq!(s.branches.len(), 1, "multiple failures should not change count");
}

// ── 9. Main branch reference behavior ──

#[test]
fn main_ref_main_is_not_in_branches_vec() {
    let mut s = empty_stack();
    s.add_branch(branch("feat", Some("main"))).expect("add");
    assert!(!s.branches.iter().any(|b| b.name.as_str() == "main"));
}

#[test]
fn main_ref_main_accepted_even_with_different_stack_main() {
    let s = Stack::new(BranchName::new("develop"));
    let mut s = s;
    let result = s.add_branch(branch("feat", Some("develop")));
    assert!(result.is_ok(), "develop is the main_branch and should be accepted as parent");
}

#[test]
fn main_ref_non_main_branch_not_accepted_if_not_in_stack() {
    let mut s = empty_stack();
    // "develop" is not main and not in branches → orphan
    let result = s.add_branch(branch("feat", Some("develop")));
    assert!(result.is_err());
    assert!(matches!(result.err().expect("err"), StackError::OrphanedBranch(_)));
}

// ── 10. Error message quality ──

#[test]
fn error_message_contains_branch_name() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("my-orphan", Some("ghost")));
    let err_msg = format!("{}", result.err().expect("err"));
    assert!(err_msg.contains("my-orphan"), "error should contain the branch name: {err_msg}");
}

#[test]
fn error_is_orphaned_branch_variant() {
    let mut s = empty_stack();
    let result = s.add_branch(branch("lost", Some("nowhere")));
    assert!(matches!(result.err().expect("err"), StackError::OrphanedBranch(_)));
}

// ── 11. Integration with stack queries ──

#[test]
fn integration_ancestors_after_add_branch() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    s.add_branch(branch("b", Some("a"))).expect("b");
    s.add_branch(branch("c", Some("b"))).expect("c");

    let ancestors = s.ancestors(&bn("c"));
    assert_eq!(ancestors, vec![bn("b"), bn("a")]);
}

#[test]
fn integration_descendants_after_add_branch() {
    let mut s = empty_stack();
    s.add_branch(branch_with_children("a", Some("main"), &["b", "c"])).expect("a");
    s.add_branch(branch("b", Some("a"))).expect("b");
    s.add_branch(branch("c", Some("a"))).expect("c");

    let mut descendants = s.descendants(&bn("a"));
    descendants.sort();
    assert!(descendants.contains(&bn("b")));
    assert!(descendants.contains(&bn("c")));
}

#[test]
fn integration_needs_restack_after_add_branch() {
    let mut s = empty_stack();
    s.add_branch(branch_restack("clean", Some("main"), false)).expect("clean");
    s.add_branch(branch_restack("dirty", Some("main"), true)).expect("dirty");

    let needs = s.needs_restack();
    assert_eq!(needs.len(), 1);
    assert_eq!(needs[0], bn("dirty"));
}

#[test]
fn integration_get_siblings_after_add_branch() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    s.add_branch(branch("b", Some("main"))).expect("b");

    let mut siblings = s.get_siblings(&bn("a"));
    siblings.sort();
    assert!(siblings.contains(&bn("a")));
    assert!(siblings.contains(&bn("b")));
}

#[test]
fn integration_current_stack_after_add_branch() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    s.add_branch(branch("b", Some("a"))).expect("b");

    let current = s.current_stack(&bn("b"));
    assert!(current.contains(&bn("a")));
    assert!(current.contains(&bn("b")));
}

#[test]
fn integration_topological_order_after_add_branch() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    s.add_branch(branch("b", Some("a"))).expect("b");
    s.add_branch(branch("c", Some("b"))).expect("c");

    let topo = s.topological_order();
    assert_eq!(topo.len(), 3);
    let names: Vec<&str> = topo.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);
}

// ── 12. Branch with all PrState variants ──

#[test]
fn pr_info_open_state() {
    let mut s = empty_stack();
    let br = StackBranch {
        name: bn("open-pr"),
        parent: Some(bn("main")),
        children: vec![],
        needs_restack: false,
        pr_info: Some(PrInfo {
            number: 1,
            url: "u".to_string(),
            title: "t".to_string(),
            state: PrState::Open,
            is_draft: Some(false),
        }),
    };
    s.add_branch(br).expect("add");
    assert!(matches!(s.branches[0].pr_info.as_ref().expect("pr").state, PrState::Open));
}

#[test]
fn pr_info_merged_state() {
    let mut s = empty_stack();
    let br = StackBranch {
        name: bn("merged-pr"),
        parent: Some(bn("main")),
        children: vec![],
        needs_restack: false,
        pr_info: Some(PrInfo {
            number: 2,
            url: "u".to_string(),
            title: "t".to_string(),
            state: PrState::Merged,
            is_draft: None,
        }),
    };
    s.add_branch(br).expect("add");
    assert!(matches!(s.branches[0].pr_info.as_ref().expect("pr").state, PrState::Merged));
}

#[test]
fn pr_info_closed_state() {
    let mut s = empty_stack();
    let br = StackBranch {
        name: bn("closed-pr"),
        parent: Some(bn("main")),
        children: vec![],
        needs_restack: false,
        pr_info: Some(PrInfo {
            number: 3,
            url: "u".to_string(),
            title: "t".to_string(),
            state: PrState::Closed,
            is_draft: Some(true),
        }),
    };
    s.add_branch(br).expect("add");
    assert!(matches!(s.branches[0].pr_info.as_ref().expect("pr").state, PrState::Closed));
}

// ── 13. Stress / boundary tests ──

#[test]
fn stress_many_branches_same_parent() {
    let mut s = empty_stack();
    for i in 0..50 {
        s.add_branch(branch(&format!("branch-{i}"), Some("main"))).expect("add");
    }
    assert_eq!(s.branches.len(), 50);
    for b in &s.branches {
        assert_eq!(b.parent, Some(bn("main")));
    }
}

#[test]
fn stress_deep_chain() {
    let mut s = empty_stack();
    s.add_branch(branch("b-0", Some("main"))).expect("0");
    for i in 1..50 {
        let parent = format!("b-{}", i - 1);
        s.add_branch(branch(&format!("b-{i}"), Some(&parent))).expect("add");
    }
    assert_eq!(s.branches.len(), 50);
    // Verify chain integrity
    assert_eq!(s.branches[0].parent, Some(bn("main")));
    for i in 1..50 {
        assert_eq!(s.branches[i].parent, Some(bn(&format!("b-{}", i - 1))));
    }
}

#[test]
fn boundary_zero_children() {
    let mut s = empty_stack();
    s.add_branch(branch_with_children("no-kids", Some("main"), &[])).expect("add");
    assert!(s.branches[0].children.is_empty());
}

#[test]
fn boundary_many_children() {
    let children: Vec<String> = (0..100).map(|i| format!("child-{i}")).collect();
    let child_refs: Vec<&str> = children.iter().map(|s| s.as_str()).collect();
    let mut s = empty_stack();
    s.add_branch(branch_with_children("parent", Some("main"), &child_refs)).expect("add");
    assert_eq!(s.branches[0].children.len(), 100);
}

// ── 14. Serde round-trip after add_branch ──

#[test]
fn serde_roundtrip_after_add_branch() {
    let mut s = empty_stack();
    s.add_branch(branch("a", Some("main"))).expect("a");
    s.add_branch(branch("b", Some("a"))).expect("b");

    let json = serde_json::to_string(&s).expect("serialize");
    let back: Stack = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.branches.len(), 2);
    assert_eq!(back.branches[0].name, bn("a"));
    assert_eq!(back.branches[1].name, bn("b"));
}

#[test]
fn serde_roundtrip_preserves_all_fields() {
    let mut s = empty_stack();
    let br = StackBranch {
        name: bn("full"),
        parent: Some(bn("main")),
        children: vec![bn("c1")],
        needs_restack: true,
        pr_info: Some(PrInfo {
            number: 42,
            url: "u".to_string(),
            title: "t".to_string(),
            state: PrState::Open,
            is_draft: Some(false),
        }),
    };
    s.add_branch(br).expect("add");

    let json = serde_json::to_string(&s).expect("serialize");
    let back: Stack = serde_json::from_str(&json).expect("deserialize");
    let stored = &back.branches[0];
    assert_eq!(stored.name, bn("full"));
    assert_eq!(stored.children, vec![bn("c1")]);
    assert!(stored.needs_restack);
    assert_eq!(stored.pr_info.as_ref().expect("pr").number, 42);
}
