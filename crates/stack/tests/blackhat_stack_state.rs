//! Black-hat test suite: Stack state machine — exhaustive typestate transitions
//!
//! Tests every valid and invalid state transition through the Stack<S> typestate
//! pattern. Since state marker types aren't publicly exported, we test transitions
//! by chaining operations and verifying data preservation.

use scp_stack::{BranchName, PrInfo, PrState, Stack, StackBranch, StackError};

// ── Helpers ──

fn main_branch() -> BranchName {
    BranchName::new("main")
}

fn make_branch(name: &str, parent: Option<&str>) -> StackBranch {
    StackBranch {
        name: BranchName::new(name),
        parent: parent.map(BranchName::new),
        children: vec![],
        needs_restack: false,
        pr_info: None,
    }
}

fn make_branch_with_pr(name: &str, parent: Option<&str>, pr_number: u32) -> StackBranch {
    StackBranch {
        name: BranchName::new(name),
        parent: parent.map(BranchName::new),
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

fn make_draft_stack() -> Stack {
    let mut s = Stack::new(main_branch());
    s.add_branch(make_branch("feature-a", Some("main")))
        .unwrap();
    s.add_branch(make_branch("feature-b", Some("main")))
        .unwrap();
    s
}

// ── Draft State ──

#[test]
fn draft_new_has_no_branches() {
    let s = Stack::new(main_branch());
    assert!(s.branches.is_empty());
}

#[test]
fn draft_new_preserves_main_branch() {
    let s = Stack::new(main_branch());
    assert_eq!(s.main_branch, main_branch());
}

#[test]
fn draft_can_transition_to_published() {
    let s = make_draft_stack();
    let _published = s.publish();
}

#[test]
fn draft_can_transition_to_failed() {
    let s = make_draft_stack();
    let _failed = s.fail();
}

#[test]
fn draft_add_branch_with_valid_parent_succeeds() {
    let mut s = Stack::new(main_branch());
    let result = s.add_branch(make_branch("feature", Some("main")));
    assert!(result.is_ok());
    assert_eq!(s.branches.len(), 1);
}

#[test]
fn draft_add_branch_with_existing_branch_parent_succeeds() {
    let mut s = Stack::new(main_branch());
    s.add_branch(make_branch("feature-a", Some("main")))
        .unwrap();
    let result = s.add_branch(make_branch("feature-a-1", Some("feature-a")));
    assert!(result.is_ok());
    assert_eq!(s.branches.len(), 2);
}

#[test]
fn draft_add_branch_with_no_parent_succeeds() {
    let mut s = Stack::new(main_branch());
    let result = s.add_branch(make_branch("orphan", None));
    assert!(result.is_ok());
}

#[test]
fn draft_add_branch_with_nonexistent_parent_fails() {
    let mut s = Stack::new(main_branch());
    let result = s.add_branch(make_branch("bad", Some("nonexistent")));
    assert!(result.is_err());
    match result {
        Err(StackError::OrphanedBranch(name)) => assert_eq!(name, "bad"),
        _ => panic!("Expected OrphanedBranch error"),
    }
}

#[test]
fn draft_add_branch_with_main_parent_succeeds_even_if_main_not_in_branches() {
    let mut s = Stack::new(main_branch());
    let result = s.add_branch(make_branch("feature", Some("main")));
    assert!(result.is_ok());
}

// ── State Transitions: Full Happy Path ──

#[test]
fn happy_path_draft_to_merged() {
    let s = make_draft_stack();
    let merged = s.publish().start_merge().complete_merge();
    assert!(merged.is_terminal());
    assert_eq!(merged.branches.len(), 2);
    assert_eq!(merged.main_branch, main_branch());
}

#[test]
fn happy_path_draft_fail_retry_publish() {
    let s = make_draft_stack();
    let _published = s.fail().retry().publish();
}

#[test]
fn happy_path_conflict_resolution() {
    let s = make_draft_stack();
    let _merging = s
        .publish()
        .start_merge()
        .mark_conflict()
        .resolve()
        .start_merge();
}

#[test]
fn happy_path_published_fail_then_retry() {
    let s = make_draft_stack();
    let _draft = s.publish().fail().retry();
}

#[test]
fn happy_path_merging_fail_then_retry() {
    let s = make_draft_stack();
    let _draft = s.publish().start_merge().fail().retry();
}

#[test]
fn happy_path_conflict_fail_then_retry() {
    let s = make_draft_stack();
    let _draft = s.publish().start_merge().mark_conflict().fail().retry();
}

#[test]
fn happy_path_retry_loop() {
    // Draft → Fail → retry → Draft → Fail → retry → Draft → Publish → Merge
    let s = make_draft_stack();
    let _merged = s
        .fail()
        .retry()
        .fail()
        .retry()
        .publish()
        .start_merge()
        .complete_merge();
}

// ── Data Preservation Through Transitions ──

#[test]
fn transition_preserves_branches() {
    let mut s = Stack::new(main_branch());
    s.add_branch(make_branch_with_pr("a", Some("main"), 1))
        .unwrap();
    s.add_branch(make_branch_with_pr("b", Some("main"), 2))
        .unwrap();

    let branches_len = s.branches.len();
    let published = s.publish();
    assert_eq!(published.branches.len(), branches_len);

    let merging = published.start_merge();
    assert_eq!(merging.branches.len(), branches_len);

    let merged = merging.complete_merge();
    assert_eq!(merged.branches.len(), branches_len);
}

#[test]
fn transition_preserves_main_branch_name() {
    let s = Stack::new(BranchName::new("develop"));
    let published = s.publish();
    assert_eq!(published.main_branch, BranchName::new("develop"));

    let merging = published.start_merge();
    assert_eq!(merging.main_branch, BranchName::new("develop"));

    let merged = merging.complete_merge();
    assert_eq!(merged.main_branch, BranchName::new("develop"));
}

#[test]
fn transition_preserves_pr_info_number() {
    let mut s = Stack::new(main_branch());
    s.add_branch(make_branch_with_pr("feature", Some("main"), 42))
        .unwrap();

    let pr_number_before = s.branches[0].pr_info.as_ref().map(|p| p.number);
    let published = s.publish();
    let pr_number_after = published.branches[0].pr_info.as_ref().map(|p| p.number);
    assert_eq!(pr_number_before, pr_number_after);
}

#[test]
fn transition_preserves_needs_restack_flag() {
    let mut s = Stack::new(main_branch());
    let mut branch = make_branch("feature", Some("main"));
    branch.needs_restack = true;
    s.add_branch(branch).unwrap();

    let merged = s.publish().start_merge().complete_merge();
    assert!(merged.branches[0].needs_restack);
}

#[test]
fn transition_preserves_children_vec() {
    let mut s = Stack::new(main_branch());
    let mut branch = make_branch("parent", Some("main"));
    branch.children = vec![BranchName::new("child-1"), BranchName::new("child-2")];
    s.add_branch(branch).unwrap();

    let merged = s.publish().start_merge().complete_merge();
    assert_eq!(merged.branches[0].children.len(), 2);
}

// ── StackBranch Construction ──

#[test]
fn stack_branch_with_all_fields() {
    let branch = StackBranch {
        name: BranchName::new("feature"),
        parent: Some(BranchName::new("main")),
        children: vec![BranchName::new("child-1"), BranchName::new("child-2")],
        needs_restack: true,
        pr_info: Some(PrInfo {
            number: 99,
            url: "https://github.com/org/repo/pull/99".to_string(),
            title: "My PR".to_string(),
            state: PrState::Merged,
            is_draft: None,
        }),
    };
    assert_eq!(branch.name.as_str(), "feature");
    assert_eq!(branch.children.len(), 2);
    assert!(branch.needs_restack);
    assert!(branch.pr_info.is_some());
    assert_eq!(branch.pr_info.as_ref().map(|p| p.number), Some(99));
}

#[test]
fn pr_state_open() {
    let info = PrInfo {
        number: 1,
        url: "url".to_string(),
        title: "t".to_string(),
        state: PrState::Open,
        is_draft: Some(true),
    };
    assert!(matches!(info.state, PrState::Open));
}

#[test]
fn pr_state_merged() {
    let info = PrInfo {
        number: 1,
        url: "url".to_string(),
        title: "t".to_string(),
        state: PrState::Merged,
        is_draft: None,
    };
    assert!(matches!(info.state, PrState::Merged));
}

#[test]
fn pr_state_closed() {
    let info = PrInfo {
        number: 1,
        url: "url".to_string(),
        title: "t".to_string(),
        state: PrState::Closed,
        is_draft: None,
    };
    assert!(matches!(info.state, PrState::Closed));
}

// ── Edge Cases ──

#[test]
fn empty_stack_can_transition() {
    let s = Stack::new(main_branch());
    let _merging = s.publish().start_merge();
}

#[test]
fn stack_with_many_branches_transitions() {
    let mut s = Stack::new(main_branch());
    for i in 0..50 {
        let name = format!("branch-{i}");
        s.add_branch(make_branch(&name, Some("main"))).unwrap();
    }
    assert_eq!(s.branches.len(), 50);

    let published = s.publish();
    assert_eq!(published.branches.len(), 50);

    let merged = published.start_merge().complete_merge();
    assert_eq!(merged.branches.len(), 50);
}

#[test]
fn deep_chain_transitions() {
    let mut s = Stack::new(main_branch());
    s.add_branch(make_branch("level-1", Some("main"))).unwrap();
    for i in 2..=20 {
        let name = format!("level-{i}");
        let parent = format!("level-{}", i - 1);
        s.add_branch(make_branch(&name, Some(&parent))).unwrap();
    }

    let merged = s.publish().start_merge().complete_merge();
    assert!(merged.is_terminal());
    assert_eq!(merged.branches.len(), 20);
}

// ── Merged Terminal State ──

#[test]
fn merged_is_terminal() {
    let s = make_draft_stack();
    let merged = s.publish().start_merge().complete_merge();
    assert!(merged.is_terminal());
}

// ── Serde Roundtrip ──

#[test]
fn pr_info_serde_roundtrip() {
    let info = PrInfo {
        number: 42,
        url: "https://github.com/test/42".to_string(),
        title: "Test PR".to_string(),
        state: PrState::Open,
        is_draft: Some(true),
    };
    let json = serde_json::to_string(&info).unwrap();
    let back: PrInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(back.number, 42);
    assert!(matches!(back.state, PrState::Open));
    assert_eq!(back.is_draft, Some(true));
}

#[test]
fn pr_state_serde_roundtrip_open() {
    let json = serde_json::to_string(&PrState::Open).unwrap();
    let back: PrState = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, PrState::Open));
}

#[test]
fn pr_state_serde_roundtrip_merged() {
    let json = serde_json::to_string(&PrState::Merged).unwrap();
    let back: PrState = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, PrState::Merged));
}

#[test]
fn pr_state_serde_roundtrip_closed() {
    let json = serde_json::to_string(&PrState::Closed).unwrap();
    let back: PrState = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, PrState::Closed));
}

#[test]
fn stack_branch_serde_roundtrip() {
    let branch = StackBranch {
        name: BranchName::new("feature-x"),
        parent: Some(BranchName::new("main")),
        children: vec![BranchName::new("sub-1")],
        needs_restack: true,
        pr_info: None,
    };
    let json = serde_json::to_string(&branch).unwrap();
    let back: StackBranch = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, branch.name);
    assert_eq!(back.parent, branch.parent);
    assert_eq!(back.children, branch.children);
    assert_eq!(back.needs_restack, branch.needs_restack);
    assert!(back.pr_info.is_none());
}

// ── BranchName Value Object ──

#[test]
fn branch_name_equality() {
    assert_eq!(BranchName::new("main"), BranchName::new("main"));
    assert_ne!(BranchName::new("main"), BranchName::new("develop"));
}

#[test]
fn branch_name_ordering() {
    assert!(BranchName::new("a") < BranchName::new("b"));
}

#[test]
fn branch_name_display() {
    assert_eq!(
        format!("{}", BranchName::new("feature/test")),
        "feature/test"
    );
}

#[test]
fn branch_name_from_conversions() {
    let from_str: BranchName = "main".into();
    assert_eq!(from_str.as_str(), "main");

    let from_string: BranchName = "develop".to_string().into();
    assert_eq!(from_string.as_str(), "develop");
}

// ── StackError Variants ──

#[test]
fn stack_error_display_messages() {
    assert_eq!(
        format!("{}", StackError::NotFound("stack-1".to_string())),
        "Stack not found: stack-1"
    );
    assert_eq!(
        format!("{}", StackError::OrphanedBranch("feat".to_string())),
        "Stack orphaned branch: feat"
    );
    assert_eq!(
        format!("{}", StackError::CyclicDependency),
        "Stack cyclic dependency"
    );
    assert_eq!(
        format!("{}", StackError::BranchNotFound("b".to_string())),
        "Branch not found: b"
    );
    assert_eq!(
        format!("{}", StackError::InvalidBranchName("x".to_string())),
        "Invalid branch name: x"
    );
    assert_eq!(
        format!("{}", StackError::GitError("fail".to_string())),
        "Git error: fail"
    );
    assert_eq!(
        format!("{}", StackError::GitHubError("rate".to_string())),
        "GitHub error: rate"
    );
    assert_eq!(
        format!("{}", StackError::ForgeError("conn".to_string())),
        "Forge error: conn"
    );
    assert_eq!(
        format!("{}", StackError::TransactionError("tx".to_string())),
        "Transaction error: tx"
    );
}
