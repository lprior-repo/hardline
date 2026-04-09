//! Calculation layer for stack merge-remote — pure functions, no I/O.
//!
//! All functions are deterministic and testable without a forge client.

use scp_stack::{BranchName, Stack, StackBranch};

use super::data::{MergeRemoteScope, PrBranchInfo, RemainingBranchInfo};

// ============================================================================
// Scope Calculation
// ============================================================================

/// Calculate which branches to merge and which remain.
///
/// The merge scope includes the current branch plus all its ancestors
/// (bottom-up). If `all` is true, descendants are also included.
///
/// # Invariants
///
/// - Trunk is never in `to_merge`
/// - `to_merge` is in bottom-up order (ancestors before current)
/// - `remaining` and `to_merge` are disjoint
/// - If `all` is true, `remaining` is empty
pub fn calculate_merge_scope(
    stack: &Stack,
    current: &BranchName,
    all: bool,
) -> MergeRemoteScope {
    let mut to_merge = stack.ancestors(current);
    to_merge.reverse();
    to_merge.retain(|b| b != &stack.main_branch);
    to_merge.push(current.clone());

    let mut remaining = stack.descendants(current);

    if all && !remaining.is_empty() {
        to_merge.extend(remaining);
        remaining = Vec::new();
    }

    MergeRemoteScope {
        to_merge,
        remaining,
        trunk: stack.main_branch.clone(),
    }
}

// ============================================================================
// PR Number Resolution
// ============================================================================

/// Resolve PR numbers for branches that have `pr_info`.
///
/// Returns `Some(PrBranchInfo)` for branches with PR info,
/// `None` for branches that need API lookup.
///
/// # Invariants
///
/// - Order matches input order
/// - Each result corresponds 1:1 with the input branch
pub fn resolve_pr_numbers(
    branches: &[BranchName],
    stack_branches: &[StackBranch],
) -> Vec<(BranchName, Option<u64>)> {
    branches
        .iter()
        .map(|branch| {
            let pr_number = stack_branches
                .iter()
                .find(|b| &b.name == branch)
                .and_then(|b| b.pr_info.as_ref())
                .map(|pr| u64::from(pr.number));
            (branch.clone(), pr_number)
        })
        .collect()
}

/// Partition resolved branches into those with PRs (ready) and without (need lookup).
///
/// # Invariants
///
/// - All branches with PR numbers go into `ready`
/// - All branches without PR numbers go into `missing`
/// - No branch appears in both
pub fn partition_by_pr(
    resolved: &[(BranchName, Option<u64>)],
) -> (Vec<PrBranchInfo>, Vec<BranchName>) {
    let mut ready = Vec::new();
    let mut missing = Vec::new();

    for (branch, pr_number) in resolved {
        match pr_number {
            Some(num) => ready.push(PrBranchInfo {
                branch: branch.clone(),
                pr_number: *num,
            }),
            None => missing.push(branch.clone()),
        }
    }

    (ready, missing)
}

/// Build `RemainingBranchInfo` list from remaining branch names and stack data.
///
/// # Invariants
///
/// - Order matches input order
/// - PR number is `Some` only if the branch has `pr_info`
pub fn build_remaining_infos(
    remaining: &[BranchName],
    stack_branches: &[StackBranch],
) -> Vec<RemainingBranchInfo> {
    remaining
        .iter()
        .map(|branch| {
            let pr_number = stack_branches
                .iter()
                .find(|b| &b.name == branch)
                .and_then(|b| b.pr_info.as_ref())
                .map(|pr| u64::from(pr.number));
            RemainingBranchInfo {
                branch: branch.clone(),
                pr_number,
            }
        })
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use scp_stack::{PrInfo, PrState};

    fn bn(name: &str) -> BranchName {
        BranchName::new(name.to_string())
    }

    fn make_stack(branches: Vec<(&str, Option<&str>, Option<u32>)>) -> Stack {
        let main = bn("main");
        let mut stack = Stack::new(main);
        let parent_map: std::collections::HashMap<BranchName, BranchName> = branches
            .iter()
            .filter_map(|(name, parent, _)| parent.map(|p| (bn(name), bn(p))))
            .collect();
        for &(name, parent, pr_num) in &branches {
            let pr_info = pr_num.map(|n| PrInfo {
                number: n,
                url: format!("https://github.com/test/{n}"),
                title: format!("PR #{n}"),
                state: PrState::Open,
                is_draft: Some(false),
            });
            stack
                .add_branch(StackBranch {
                    name: bn(name),
                    parent: parent.map(bn),
                    children: vec![],
                    needs_restack: false,
                    pr_info,
                })
                .ok();
        }
        // Populate children from parent relationships
        for (child, parent) in &parent_map {
            if let Some(parent_branch) = stack.branches.iter_mut().find(|b| &b.name == parent) {
                if !parent_branch.children.contains(child) {
                    parent_branch.children.push(child.clone());
                }
            }
        }
        stack
    }

    // ---- calculate_merge_scope ----

    #[test]
    fn scope_single_branch() {
        let stack = make_stack(vec![("feat-a", Some("main"), Some(1))]);
        let scope = calculate_merge_scope(&stack, &bn("feat-a"), false);
        assert_eq!(scope.to_merge, vec![bn("feat-a")]);
        assert!(scope.remaining.is_empty());
        assert_eq!(scope.trunk, bn("main"));
    }

    #[test]
    fn scope_linear_chain_bottom_up() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), Some(1)),
            ("feat-b", Some("feat-a"), Some(2)),
            ("feat-c", Some("feat-b"), Some(3)),
        ]);
        let scope = calculate_merge_scope(&stack, &bn("feat-c"), false);
        // bottom-up: ancestors reversed + current
        assert_eq!(scope.to_merge.len(), 3);
        // feat-a and feat-b (ancestors) should come before feat-c
        let a_pos = scope
            .to_merge
            .iter()
            .position(|b| b == &bn("feat-a"))
            .expect("a");
        let b_pos = scope
            .to_merge
            .iter()
            .position(|b| b == &bn("feat-b"))
            .expect("b");
        let c_pos = scope
            .to_merge
            .iter()
            .position(|b| b == &bn("feat-c"))
            .expect("c");
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
        assert!(scope.remaining.is_empty());
    }

    #[test]
    fn scope_excludes_trunk() {
        let stack = make_stack(vec![("feat-a", Some("main"), Some(1))]);
        let scope = calculate_merge_scope(&stack, &bn("feat-a"), false);
        assert!(!scope.to_merge.contains(&bn("main")));
    }

    #[test]
    fn scope_with_descendants_remaining() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), Some(1)),
            ("feat-b", Some("feat-a"), Some(2)),
            ("feat-c", Some("feat-b"), Some(3)),
        ]);
        // Merge feat-b: ancestors=[feat-a], descendants=[feat-c]
        let scope = calculate_merge_scope(&stack, &bn("feat-b"), false);
        assert_eq!(scope.to_merge.len(), 2); // feat-a, feat-b
        assert_eq!(scope.remaining.len(), 1); // feat-c
        assert!(scope.remaining.contains(&bn("feat-c")));
    }

    #[test]
    fn scope_all_includes_everything() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), Some(1)),
            ("feat-b", Some("feat-a"), Some(2)),
            ("feat-c", Some("feat-b"), Some(3)),
        ]);
        let scope = calculate_merge_scope(&stack, &bn("feat-b"), true);
        assert_eq!(scope.to_merge.len(), 3); // feat-a, feat-b, feat-c
        assert!(scope.remaining.is_empty());
    }

    #[test]
    fn scope_no_descendants() {
        let stack = make_stack(vec![("feat-a", Some("main"), Some(1))]);
        let scope = calculate_merge_scope(&stack, &bn("feat-a"), true);
        assert_eq!(scope.to_merge.len(), 1);
        assert!(scope.remaining.is_empty());
    }

    // ---- resolve_pr_numbers ----

    #[test]
    fn resolve_pr_all_have_prs() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), Some(10)),
            ("feat-b", Some("main"), Some(20)),
        ]);
        let resolved = resolve_pr_numbers(
            &[bn("feat-a"), bn("feat-b")],
            &stack.branches,
        );
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].1, Some(10));
        assert_eq!(resolved[1].1, Some(20));
    }

    #[test]
    fn resolve_pr_some_missing() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), Some(10)),
            ("feat-b", Some("main"), None),
        ]);
        let resolved = resolve_pr_numbers(
            &[bn("feat-a"), bn("feat-b")],
            &stack.branches,
        );
        assert_eq!(resolved[0].1, Some(10));
        assert_eq!(resolved[1].1, None);
    }

    #[test]
    fn resolve_pr_unknown_branch() {
        let stack = make_stack(vec![("feat-a", Some("main"), Some(10))]);
        let resolved = resolve_pr_numbers(&[bn("ghost")], &stack.branches);
        assert_eq!(resolved[0].1, None);
    }

    // ---- partition_by_pr ----

    #[test]
    fn partition_all_ready() {
        let resolved = vec![
            (bn("feat-a"), Some(10u64)),
            (bn("feat-b"), Some(20u64)),
        ];
        let (ready, missing) = partition_by_pr(&resolved);
        assert_eq!(ready.len(), 2);
        assert!(missing.is_empty());
        assert_eq!(ready[0].pr_number, 10);
        assert_eq!(ready[1].pr_number, 20);
    }

    #[test]
    fn partition_all_missing() {
        let resolved = vec![
            (bn("feat-a"), None),
            (bn("feat-b"), None),
        ];
        let (ready, missing) = partition_by_pr(&resolved);
        assert!(ready.is_empty());
        assert_eq!(missing.len(), 2);
    }

    #[test]
    fn partition_mixed() {
        let resolved = vec![
            (bn("feat-a"), Some(10u64)),
            (bn("feat-b"), None),
            (bn("feat-c"), Some(30u64)),
        ];
        let (ready, missing) = partition_by_pr(&resolved);
        assert_eq!(ready.len(), 2);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], bn("feat-b"));
    }

    // ---- build_remaining_infos ----

    #[test]
    fn remaining_with_prs() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), Some(1)),
            ("feat-b", Some("main"), Some(2)),
        ]);
        let infos = build_remaining_infos(&[bn("feat-a"), bn("feat-b")], &stack.branches);
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].pr_number, Some(1));
        assert_eq!(infos[1].pr_number, Some(2));
    }

    #[test]
    fn remaining_without_prs() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), None),
            ("feat-b", Some("main"), Some(2)),
        ]);
        let infos = build_remaining_infos(&[bn("feat-a"), bn("feat-b")], &stack.branches);
        assert!(infos[0].pr_number.is_none());
        assert_eq!(infos[1].pr_number, Some(2));
    }

    #[test]
    fn remaining_empty() {
        let stack = make_stack(vec![]);
        let infos = build_remaining_infos(&[], &stack.branches);
        assert!(infos.is_empty());
    }
}
