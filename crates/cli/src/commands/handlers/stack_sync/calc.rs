//! Calculation layer for stack sync - pure functions, no I/O.
//!
//! All functions are deterministic and testable without a VCS backend.

use std::collections::HashSet;

use scp_stack::{BranchName, PrState, Stack, StackBranch};

use super::data::{
    DriftReport, MergedDetectionInput, MergedDetectionMethod, RestackOutcome, RestackStatus,
    StackSyncOptions,
};

// ============================================================================
// Precondition Validation
// ============================================================================

/// Validate that the sync operation can proceed.
///
/// # Errors
///
/// Returns a string describing why preconditions are not met.
pub fn validate_sync_preconditions(stack: &Stack, is_clean: bool) -> Result<(), String> {
    if !is_clean {
        return Err("Workspace has uncommitted changes. Stash or commit first.".to_string());
    }

    if stack.branches.is_empty() {
        return Err("No stack branches tracked. Nothing to sync.".to_string());
    }

    Ok(())
}

// ============================================================================
// Merged Branch Detection
// ============================================================================

/// Detect which tracked branches have been merged.
///
/// Uses multiple detection methods and deduplicates results.
/// The trunk branch is never included in the result.
///
/// # Invariants
///
/// - Trunk branch is never in the result
/// - Each branch appears at most once (first detection wins)
/// - Result is a subset of `input.tracked_branches`
pub fn detect_merged_branches(
    trunk: &BranchName,
    input: &MergedDetectionInput,
) -> Vec<(BranchName, MergedDetectionMethod)> {
    let mut merged: Vec<(BranchName, MergedDetectionMethod)> = Vec::new();
    let mut seen: HashSet<BranchName> = HashSet::new();

    // Method 1: git branch --merged <trunk>
    for branch in &input.local_merged {
        if branch == trunk || !input.tracked_branches.contains(branch) || seen.contains(branch) {
            continue;
        }
        seen.insert(branch.clone());
        merged.push((branch.clone(), MergedDetectionMethod::GitBranchMerged));
    }

    // Method 1b: git branch --merged origin/<trunk>
    for branch in &input.remote_merged {
        if branch == trunk || !input.tracked_branches.contains(branch) || seen.contains(branch) {
            continue;
        }
        seen.insert(branch.clone());
        merged.push((branch.clone(), MergedDetectionMethod::GitBranchMergedRemote));
    }

    // Method 2: PR state check
    for (branch, state) in &input.pr_states {
        if branch == trunk || !input.tracked_branches.contains(branch) || seen.contains(branch) {
            continue;
        }
        let method = match state {
            PrState::Merged => MergedDetectionMethod::PrStateMerged,
            PrState::Closed => MergedDetectionMethod::PrStateClosed,
            PrState::Open => continue,
        };
        seen.insert(branch.clone());
        merged.push((branch.clone(), method));
    }

    // Method 4: Remote branch deleted (had PR)
    // Only check if remote_branches was actually populated
    if !input.remote_branches.is_empty() {
        for branch in &input.tracked_branches {
            if branch == trunk || seen.contains(branch) {
                continue;
            }
            let has_pr = input.pr_states.contains_key(branch);
            if has_pr && !input.remote_branches.contains(branch) {
                seen.insert(branch.clone());
                merged.push((branch.clone(), MergedDetectionMethod::RemoteBranchDeleted));
            }
        }
    }

    // Method 5: Orphaned branches (no local, no remote)
    // Only detect as orphaned if local and remote branch lists were actually
    // populated (non-empty). Empty sets mean "didn't check", not "confirmed absent".
    if !input.local_branches.is_empty() || !input.remote_branches.is_empty() {
        for branch in &input.tracked_branches {
            if branch == trunk || seen.contains(branch) {
                continue;
            }
            let local_exists = input.local_branches.contains(branch);
            let remote_exists = input.remote_branches.contains(branch);
            if !local_exists && !remote_exists {
                seen.insert(branch.clone());
                merged.push((branch.clone(), MergedDetectionMethod::OrphanedBranch));
            }
        }
    }

    merged
}

// ============================================================================
// Restack Planning
// ============================================================================

/// Plan the order in which branches should be restacked.
///
/// Returns branches in topological order (parent before child),
/// filtered to only those that need restacking.
///
/// # Invariants
///
/// - Result is a subset of `branches_needing_restack`
/// - Parent always appears before child
pub fn plan_restack_order(
    stack: &Stack,
    branches_needing_restack: &[BranchName],
) -> Vec<BranchName> {
    let needs_set: HashSet<&BranchName> = branches_needing_restack.iter().collect();

    // Use topological order from stack, then filter
    let topo = stack.topological_order();
    topo.iter()
        .filter(|b| needs_set.contains(&b.name))
        .map(|b| b.name.clone())
        .collect()
}

// ============================================================================
// Parent Resolution
// ============================================================================

/// Resolve the effective parent for a branch being cleaned up.
///
/// If the recorded parent still exists locally, use it.
/// Otherwise fall back to trunk.
///
/// # Returns
///
/// A tuple of `(effective_parent, fallback_from)` where `fallback_from`
/// is `Some(original_parent)` if we had to fall back to trunk.
pub fn resolve_effective_parent(
    recorded_parent: &BranchName,
    trunk: &BranchName,
    existing_branches: &HashSet<BranchName>,
) -> (BranchName, Option<BranchName>) {
    if existing_branches.contains(recorded_parent) {
        return (recorded_parent.clone(), None);
    }

    if recorded_parent != trunk && existing_branches.contains(trunk) {
        return (trunk.clone(), Some(recorded_parent.clone()));
    }

    // Can't resolve anything better, return as-is
    (recorded_parent.clone(), None)
}

// ============================================================================
// Drift Computation
// ============================================================================

/// Compute what changed between local and remote state.
pub fn compute_drift(
    old_remote_branches: &HashSet<BranchName>,
    new_remote_branches: &HashSet<BranchName>,
    trunk_old_sha: Option<&str>,
    trunk_new_sha: Option<&str>,
    stack: &Stack,
) -> DriftReport {
    let new_remote: Vec<BranchName> = new_remote_branches
        .difference(old_remote_branches)
        .cloned()
        .collect();

    let removed_remote: Vec<BranchName> = old_remote_branches
        .difference(new_remote_branches)
        .cloned()
        .collect();

    let trunk_advanced = match (trunk_old_sha, trunk_new_sha) {
        (Some(old), Some(new)) => old != new,
        _ => false,
    };

    let branches_needing_restack = stack.needs_restack();

    DriftReport {
        trunk_advanced,
        new_remote_branches: new_remote,
        removed_remote_branches: removed_remote,
        branches_needing_restack,
    }
}

// ============================================================================
// Branch reparenting
// ============================================================================

/// Find children of a merged branch that need reparenting.
///
/// Returns pairs of (child_name, new_parent_name).
pub fn find_children_to_reparent(
    stack: &Stack,
    merged_branch: &BranchName,
    merged_parent: &BranchName,
) -> Vec<(BranchName, BranchName)> {
    stack
        .branches
        .iter()
        .filter(|b| b.parent.as_ref() == Some(merged_branch))
        .map(|b| (b.name.clone(), merged_parent.clone()))
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use scp_stack::{PrInfo, StackBranch};

    use super::*;

    fn bn(name: &str) -> BranchName {
        BranchName::new(name.to_string())
    }

    fn make_stack(branches: Vec<(&str, Option<&str>)>) -> Stack {
        let main = bn("main");
        let mut stack = Stack::new(main.clone());
        for (name, parent) in branches {
            let parent_bn = parent.map(|p| bn(p));
            let children = Vec::new();
            stack
                .add_branch(StackBranch {
                    name: bn(name),
                    parent: parent_bn,
                    children,
                    needs_restack: false,
                    pr_info: None,
                })
                .ok();
        }
        stack
    }

    fn make_stack_with_restack(branches: Vec<(&str, Option<&str>, bool)>) -> Stack {
        let main = bn("main");
        let mut stack = Stack::new(main);
        for (name, parent, needs_restack) in branches {
            let parent_bn = parent.map(|p| bn(p));
            let children = Vec::new();
            stack
                .add_branch(StackBranch {
                    name: bn(name),
                    parent: parent_bn,
                    children,
                    needs_restack,
                    pr_info: None,
                })
                .ok();
        }
        stack
    }

    // ---- validate_sync_preconditions ----

    #[test]
    fn validate_clean_workspace_with_branches() {
        let stack = make_stack(vec![("feat-a", Some("main"))]);
        assert!(validate_sync_preconditions(&stack, true).is_ok());
    }

    #[test]
    fn validate_dirty_workspace_rejected() {
        let stack = make_stack(vec![("feat-a", Some("main"))]);
        let result = validate_sync_preconditions(&stack, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("uncommitted"));
    }

    #[test]
    fn validate_empty_stack_rejected() {
        let stack = make_stack(vec![]);
        let result = validate_sync_preconditions(&stack, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No stack branches"));
    }

    // ---- detect_merged_branches ----

    #[test]
    fn detect_merged_via_local_merged() {
        let trunk = bn("main");
        let tracked = vec![bn("feat-a"), bn("feat-b")];
        let local_merged: HashSet<BranchName> = [bn("feat-a")].into_iter().collect();

        let input = MergedDetectionInput {
            tracked_branches: tracked,
            local_merged,
            ..Default::default()
        };

        let result = detect_merged_branches(&trunk, &input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, bn("feat-a"));
        assert_eq!(result[0].1, MergedDetectionMethod::GitBranchMerged);
    }

    #[test]
    fn detect_merged_trunk_never_included() {
        let trunk = bn("main");
        let tracked = vec![bn("main"), bn("feat-a")];
        let local_merged: HashSet<BranchName> = [bn("main"), bn("feat-a")].into_iter().collect();

        let input = MergedDetectionInput {
            tracked_branches: tracked,
            local_merged,
            ..Default::default()
        };

        let result = detect_merged_branches(&trunk, &input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, bn("feat-a"));
    }

    #[test]
    fn detect_merged_deduplicates() {
        let trunk = bn("main");
        let tracked = vec![bn("feat-a")];
        let local_merged: HashSet<BranchName> = [bn("feat-a")].into_iter().collect();
        let remote_merged: HashSet<BranchName> = [bn("feat-a")].into_iter().collect();

        let input = MergedDetectionInput {
            tracked_branches: tracked,
            local_merged,
            remote_merged,
            ..Default::default()
        };

        let result = detect_merged_branches(&trunk, &input);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn detect_merged_via_pr_state() {
        let trunk = bn("main");
        let tracked = vec![bn("feat-a")];
        let pr_states = [(bn("feat-a"), PrState::Merged)].into_iter().collect();

        let input = MergedDetectionInput {
            tracked_branches: tracked,
            pr_states,
            ..Default::default()
        };

        let result = detect_merged_branches(&trunk, &input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, MergedDetectionMethod::PrStateMerged);
    }

    #[test]
    fn detect_merged_via_pr_closed() {
        let trunk = bn("main");
        let tracked = vec![bn("feat-a")];
        let pr_states = [(bn("feat-a"), PrState::Closed)].into_iter().collect();

        let input = MergedDetectionInput {
            tracked_branches: tracked,
            pr_states,
            ..Default::default()
        };

        let result = detect_merged_branches(&trunk, &input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, MergedDetectionMethod::PrStateClosed);
    }

    #[test]
    fn detect_merged_pr_open_not_merged() {
        let trunk = bn("main");
        let tracked = vec![bn("feat-a")];
        let pr_states = [(bn("feat-a"), PrState::Open)].into_iter().collect();

        let input = MergedDetectionInput {
            tracked_branches: tracked,
            pr_states,
            ..Default::default()
        };

        let result = detect_merged_branches(&trunk, &input);
        assert!(result.is_empty());
    }

    #[test]
    fn detect_merged_via_remote_deleted() {
        let trunk = bn("main");
        let tracked = vec![bn("feat-a")];
        let pr_states = [(bn("feat-a"), PrState::Merged)].into_iter().collect();
        let remote_branches: HashSet<BranchName> = HashSet::new(); // empty = no remote

        let input = MergedDetectionInput {
            tracked_branches: tracked,
            pr_states,
            remote_branches,
            ..Default::default()
        };

        let result = detect_merged_branches(&trunk, &input);
        // Should be detected via PrStateMerged first
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn detect_merged_via_orphaned() {
        let trunk = bn("main");
        let tracked = vec![bn("feat-a")];
        // Populate local branches (with other branches) to confirm we checked
        let local_branches: HashSet<BranchName> = [bn("main"), bn("other")].into_iter().collect();
        let input = MergedDetectionInput {
            tracked_branches: tracked,
            local_branches,
            remote_branches: HashSet::new(),
            ..Default::default()
        };

        let result = detect_merged_branches(&trunk, &input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, MergedDetectionMethod::OrphanedBranch);
    }

    #[test]
    fn detect_merged_nothing_merged() {
        let trunk = bn("main");
        let tracked = vec![bn("feat-a"), bn("feat-b")];
        let input = MergedDetectionInput {
            tracked_branches: tracked,
            remote_branches: [bn("feat-a"), bn("feat-b")].into_iter().collect(),
            local_branches: [bn("feat-a"), bn("feat-b")].into_iter().collect(),
            ..Default::default()
        };

        let result = detect_merged_branches(&trunk, &input);
        assert!(result.is_empty());
    }

    // ---- plan_restack_order ----

    #[test]
    fn plan_restack_linear_chain() {
        let stack = make_stack_with_restack(vec![
            ("a", Some("main"), true),
            ("b", Some("a"), true),
            ("c", Some("b"), true),
        ]);
        let needs = vec![bn("a"), bn("b"), bn("c")];
        let order = plan_restack_order(&stack, &needs);
        assert_eq!(order.len(), 3);
        // a must come before b, b before c
        let a_pos = order.iter().position(|b| b == &bn("a")).expect("a");
        let b_pos = order.iter().position(|b| b == &bn("b")).expect("b");
        let c_pos = order.iter().position(|b| b == &bn("c")).expect("c");
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn plan_restack_filters_non_restack() {
        let stack = make_stack_with_restack(vec![
            ("a", Some("main"), true),
            ("b", Some("a"), false),
            ("c", Some("b"), true),
        ]);
        let needs = vec![bn("a"), bn("c")];
        let order = plan_restack_order(&stack, &needs);
        assert_eq!(order.len(), 2);
        assert!(order.contains(&bn("a")));
        assert!(order.contains(&bn("c")));
    }

    #[test]
    fn plan_restack_empty() {
        let stack = make_stack(vec![]);
        let order = plan_restack_order(&stack, &[]);
        assert!(order.is_empty());
    }

    // ---- resolve_effective_parent ----

    #[test]
    fn resolve_parent_exists() {
        let existing: HashSet<BranchName> = [bn("main"), bn("feat-a")].into_iter().collect();
        let (parent, fallback) = resolve_effective_parent(&bn("feat-a"), &bn("main"), &existing);
        assert_eq!(parent, bn("feat-a"));
        assert!(fallback.is_none());
    }

    #[test]
    fn resolve_parent_gone_fallback_trunk() {
        let existing: HashSet<BranchName> = [bn("main")].into_iter().collect();
        let (parent, fallback) = resolve_effective_parent(&bn("feat-x"), &bn("main"), &existing);
        assert_eq!(parent, bn("main"));
        assert_eq!(fallback, Some(bn("feat-x")));
    }

    #[test]
    fn resolve_parent_gone_no_trunk() {
        let existing: HashSet<BranchName> = HashSet::new();
        let (parent, fallback) = resolve_effective_parent(&bn("feat-x"), &bn("main"), &existing);
        assert_eq!(parent, bn("feat-x"));
        assert!(fallback.is_none());
    }

    #[test]
    fn resolve_parent_is_trunk() {
        let existing: HashSet<BranchName> = [bn("main")].into_iter().collect();
        let (parent, fallback) = resolve_effective_parent(&bn("main"), &bn("main"), &existing);
        assert_eq!(parent, bn("main"));
        assert!(fallback.is_none());
    }

    // ---- compute_drift ----

    #[test]
    fn compute_drift_trunk_advanced() {
        let stack = make_stack(vec![("feat-a", Some("main"))]);
        let report = compute_drift(
            &HashSet::new(),
            &HashSet::new(),
            Some("abc123"),
            Some("def456"),
            &stack,
        );
        assert!(report.trunk_advanced);
    }

    #[test]
    fn compute_drift_trunk_same() {
        let stack = make_stack(vec![("feat-a", Some("main"))]);
        let report = compute_drift(
            &HashSet::new(),
            &HashSet::new(),
            Some("abc123"),
            Some("abc123"),
            &stack,
        );
        assert!(!report.trunk_advanced);
    }

    #[test]
    fn compute_drift_new_remote_branches() {
        let stack = make_stack(vec![]);
        let old: HashSet<BranchName> = HashSet::new();
        let new: HashSet<BranchName> = [bn("feat-new")].into_iter().collect();
        let report = compute_drift(&old, &new, None, None, &stack);
        assert_eq!(report.new_remote_branches.len(), 1);
    }

    #[test]
    fn compute_drift_removed_remote_branches() {
        let stack = make_stack(vec![]);
        let old: HashSet<BranchName> = [bn("feat-old")].into_iter().collect();
        let new: HashSet<BranchName> = HashSet::new();
        let report = compute_drift(&old, &new, None, None, &stack);
        assert_eq!(report.removed_remote_branches.len(), 1);
    }

    // ---- find_children_to_reparent ----

    #[test]
    fn find_children_simple() {
        let stack = make_stack(vec![
            ("feat-a", Some("main")),
            ("feat-b", Some("feat-a")),
            ("feat-c", Some("feat-a")),
        ]);
        let children = find_children_to_reparent(&stack, &bn("feat-a"), &bn("main"));
        assert_eq!(children.len(), 2);
        assert!(children.iter().all(|(_, p)| p == &bn("main")));
    }

    #[test]
    fn find_children_none() {
        let stack = make_stack(vec![("feat-a", Some("main"))]);
        let children = find_children_to_reparent(&stack, &bn("feat-a"), &bn("main"));
        assert!(children.is_empty());
    }
}
