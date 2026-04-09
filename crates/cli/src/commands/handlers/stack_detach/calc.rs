//! Calculation layer for stack detach - pure functions, no I/O.
//!
//! All functions are deterministic and testable without a VCS backend.

use scp_stack::{BranchName, Stack};

use super::data::DetachError;

// ============================================================================
// Precondition Validation
// ============================================================================

/// Validate that the detach operation can proceed.
///
/// # Errors
///
/// Returns `DetachError` if:
/// - Branch is the trunk
/// - Branch is not in the stack
/// - Branch has no parent
pub fn validate_detach_preconditions(
    stack: &Stack,
    branch: &BranchName,
    trunk: &BranchName,
) -> Result<(), DetachError> {
    if branch == trunk {
        return Err(DetachError::CannotDetachTrunk(branch.clone()));
    }

    let branch_info = stack
        .branches
        .iter()
        .find(|b| &b.name == branch)
        .ok_or_else(|| DetachError::NotTracked(branch.clone()))?;

    if branch_info.parent.is_none() {
        return Err(DetachError::NoParent(branch.clone()));
    }

    Ok(())
}

// ============================================================================
// Detach Planning
// ============================================================================

/// Compute the detach plan: which children need reparenting and to what parent.
///
/// # Returns
///
/// A tuple of (previous_parent, children_to_reparent) where each child entry
/// is (child_name, new_parent_name).
///
/// # Invariants
///
/// - Returns empty children if the branch has no children
/// - All children are reparented to the same new parent (the detached branch's parent)
/// - The previous parent is always the direct parent of the detached branch
pub fn plan_detach(
    stack: &Stack,
    branch: &BranchName,
) -> Result<(BranchName, Vec<(BranchName, BranchName)>), DetachError> {
    let branch_info = stack
        .branches
        .iter()
        .find(|b| &b.name == branch)
        .ok_or_else(|| DetachError::NotTracked(branch.clone()))?;

    let previous_parent = branch_info
        .parent
        .clone()
        .ok_or_else(|| DetachError::NoParent(branch.clone()))?;

    let children: Vec<(BranchName, BranchName)> = branch_info
        .children
        .iter()
        .map(|child| (child.clone(), previous_parent.clone()))
        .collect();

    Ok((previous_parent, children))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use scp_stack::{StackBranch, Stack};

    fn bn(name: &str) -> BranchName {
        BranchName::new(name.to_string())
    }

    fn make_stack(branches: Vec<(&str, Option<&str>, Vec<&str>)>) -> Stack {
        let main = bn("main");
        let mut stack = Stack::new(main);
        for (name, parent, children) in branches {
            stack
                .add_branch(StackBranch {
                    name: bn(name),
                    parent: parent.map(bn),
                    children: children.into_iter().map(bn).collect(),
                    needs_restack: false,
                    pr_info: None,
                })
                .ok();
        }
        stack
    }

    // ---- validate_detach_preconditions ----

    #[test]
    fn validate_valid_branch_with_parent() {
        let stack = make_stack(vec![("feat-a", Some("main"), vec![])]);
        assert!(validate_detach_preconditions(&stack, &bn("feat-a"), &bn("main")).is_ok());
    }

    #[test]
    fn validate_rejects_trunk() {
        let stack = make_stack(vec![("feat-a", Some("main"), vec![])]);
        let result = validate_detach_preconditions(&stack, &bn("main"), &bn("main"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DetachError::CannotDetachTrunk(_)));
    }

    #[test]
    fn validate_rejects_untracked() {
        let stack = make_stack(vec![("feat-a", Some("main"), vec![])]);
        let result = validate_detach_preconditions(&stack, &bn("ghost"), &bn("main"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DetachError::NotTracked(_)));
    }

    #[test]
    fn validate_rejects_no_parent() {
        let stack = make_stack(vec![("orphan", None, vec![])]);
        let result = validate_detach_preconditions(&stack, &bn("orphan"), &bn("main"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DetachError::NoParent(_)));
    }

    // ---- plan_detach ----

    #[test]
    fn plan_leaf_no_children() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), vec!["feat-b"]),
            ("feat-b", Some("feat-a"), vec![]),
        ]);
        let (parent, children) = plan_detach(&stack, &bn("feat-b")).expect("plan");
        assert_eq!(parent, bn("feat-a"));
        assert!(children.is_empty());
    }

    #[test]
    fn plan_mid_stack_with_children() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), vec!["feat-b"]),
            ("feat-b", Some("feat-a"), vec!["feat-c"]),
            ("feat-c", Some("feat-b"), vec![]),
        ]);
        let (parent, children) = plan_detach(&stack, &bn("feat-b")).expect("plan");
        assert_eq!(parent, bn("feat-a"));
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], (bn("feat-c"), bn("feat-a")));
    }

    #[test]
    fn plan_first_level_child() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), vec!["feat-a-1"]),
            ("feat-a-1", Some("feat-a"), vec!["feat-a-2"]),
            ("feat-a-2", Some("feat-a-1"), vec![]),
        ]);
        let (parent, children) = plan_detach(&stack, &bn("feat-a")).expect("plan");
        assert_eq!(parent, bn("main"));
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], (bn("feat-a-1"), bn("main")));
    }

    #[test]
    fn plan_multiple_children() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), vec!["feat-b", "feat-c"]),
            ("feat-b", Some("feat-a"), vec![]),
            ("feat-c", Some("feat-a"), vec![]),
        ]);
        let (parent, children) = plan_detach(&stack, &bn("feat-a")).expect("plan");
        assert_eq!(parent, bn("main"));
        assert_eq!(children.len(), 2);
        // Both children reparented to main
        assert!(children.contains(&(bn("feat-b"), bn("main"))));
        assert!(children.contains(&(bn("feat-c"), bn("main"))));
    }

    #[test]
    fn plan_untracked_branch_fails() {
        let stack = make_stack(vec![("feat-a", Some("main"), vec![])]);
        let result = plan_detach(&stack, &bn("ghost"));
        assert!(result.is_err());
    }
}
