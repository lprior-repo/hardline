//! Calculation layer for stack split - pure functions, no I/O.
//!
//! All functions are deterministic and testable without a VCS backend.

use scp_stack::{BranchName, Stack};

use super::data::{SplitError, SplitPlan};

/// Validate that the split operation can proceed.
///
/// # Errors
///
/// Returns `SplitError` if:
/// - Branch is the trunk
/// - Branch is not in the stack
/// - Branch has no parent
pub fn validate_split_preconditions(
    stack: &Stack,
    branch: &BranchName,
    trunk: &BranchName,
) -> Result<(), SplitError> {
    if branch == trunk {
        return Err(SplitError::CannotSplitTrunk(branch.clone()));
    }

    let branch_info = stack
        .branches
        .iter()
        .find(|b| &b.name == branch)
        .ok_or_else(|| SplitError::NotTracked(branch.clone()))?;

    if branch_info.parent.is_none() {
        return Err(SplitError::NoParent(branch.clone()));
    }

    Ok(())
}

/// Resolve the effective branch names for the split result.
///
/// If custom names are provided, uses those. Otherwise derives from the source:
/// - Lower: `<source>-1`
/// - Upper: `<source>-2`
pub fn resolve_branch_names(
    source: &BranchName,
    lower_name: Option<&BranchName>,
    upper_name: Option<&BranchName>,
) -> (BranchName, BranchName) {
    let lower = lower_name
        .cloned()
        .unwrap_or_else(|| BranchName::new(format!("{}-1", source.as_str())));
    let upper = upper_name
        .cloned()
        .unwrap_or_else(|| BranchName::new(format!("{}-2", source.as_str())));
    (lower, upper)
}

/// Compute the split plan: which children need reparenting and to what.
///
/// Returns a `SplitPlan` with all information needed to execute the split.
///
/// # Invariants
///
/// - The lower branch gets the source branch's parent as its parent
/// - The upper branch gets the lower branch as its parent
/// - Children of the source are reparented to the upper branch
/// - The split commit becomes the tip of the lower branch
/// - The source tip becomes the tip of the upper branch
pub fn plan_split(
    stack: &Stack,
    branch: &BranchName,
    lower_branch: &BranchName,
    upper_branch: &BranchName,
    split_commit: &str,
    source_tip: &str,
    lower_parent_revision: &str,
) -> Result<SplitPlan, SplitError> {
    let branch_info = stack
        .branches
        .iter()
        .find(|b| &b.name == branch)
        .ok_or_else(|| SplitError::NotTracked(branch.clone()))?;

    let lower_parent = branch_info
        .parent
        .clone()
        .ok_or_else(|| SplitError::NoParent(branch.clone()))?;

    let children_to_reparent: Vec<BranchName> = branch_info.children.clone();

    Ok(SplitPlan {
        lower_branch: lower_branch.clone(),
        upper_branch: upper_branch.clone(),
        lower_parent,
        lower_parent_revision: lower_parent_revision.to_string(),
        split_commit: split_commit.to_string(),
        source_tip: source_tip.to_string(),
        children_to_reparent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_stack::{Stack, StackBranch};

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

    #[test]
    fn validate_valid_branch_with_parent() {
        let stack = make_stack(vec![("feat-a", Some("main"), vec![])]);
        assert!(validate_split_preconditions(&stack, &bn("feat-a"), &bn("main")).is_ok());
    }

    #[test]
    fn validate_rejects_trunk() {
        let stack = make_stack(vec![("feat-a", Some("main"), vec![])]);
        let result = validate_split_preconditions(&stack, &bn("main"), &bn("main"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SplitError::CannotSplitTrunk(_)
        ));
    }

    #[test]
    fn validate_rejects_untracked() {
        let stack = make_stack(vec![("feat-a", Some("main"), vec![])]);
        let result = validate_split_preconditions(&stack, &bn("ghost"), &bn("main"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SplitError::NotTracked(_)));
    }

    #[test]
    fn validate_rejects_no_parent() {
        let stack = make_stack(vec![("orphan", None, vec![])]);
        let result = validate_split_preconditions(&stack, &bn("orphan"), &bn("main"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SplitError::NoParent(_)));
    }

    #[test]
    fn resolve_names_custom() {
        let (lower, upper) =
            resolve_branch_names(&bn("feat"), Some(&bn("lower")), Some(&bn("upper")));
        assert_eq!(lower, bn("lower"));
        assert_eq!(upper, bn("upper"));
    }

    #[test]
    fn resolve_names_default() {
        let (lower, upper) = resolve_branch_names(&bn("feat"), None, None);
        assert_eq!(lower, bn("feat-1"));
        assert_eq!(upper, bn("feat-2"));
    }

    #[test]
    fn resolve_names_lower_only() {
        let (lower, upper) = resolve_branch_names(&bn("feat"), Some(&bn("custom-lower")), None);
        assert_eq!(lower, bn("custom-lower"));
        assert_eq!(upper, bn("feat-2"));
    }

    #[test]
    fn resolve_names_upper_only() {
        let (lower, upper) = resolve_branch_names(&bn("feat"), None, Some(&bn("custom-upper")));
        assert_eq!(lower, bn("feat-1"));
        assert_eq!(upper, bn("custom-upper"));
    }

    #[test]
    fn plan_split_leaf_no_children() {
        let stack = make_stack(vec![("feat-a", Some("main"), vec![])]);
        let plan = plan_split(
            &stack,
            &bn("feat-a"),
            &bn("feat-a-1"),
            &bn("feat-a-2"),
            "split-rev",
            "tip-rev",
            "parent-rev",
        )
        .expect("plan");

        assert_eq!(plan.lower_branch, bn("feat-a-1"));
        assert_eq!(plan.upper_branch, bn("feat-a-2"));
        assert_eq!(plan.lower_parent, bn("main"));
        assert!(plan.children_to_reparent.is_empty());
        assert_eq!(plan.split_commit, "split-rev");
        assert_eq!(plan.source_tip, "tip-rev");
    }

    #[test]
    fn plan_split_mid_stack_with_children() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), vec!["feat-b"]),
            ("feat-b", Some("feat-a"), vec!["feat-c"]),
            ("feat-c", Some("feat-b"), vec![]),
        ]);

        let plan = plan_split(
            &stack,
            &bn("feat-b"),
            &bn("feat-b-1"),
            &bn("feat-b-2"),
            "split-rev",
            "tip-rev",
            "parent-rev",
        )
        .expect("plan");

        assert_eq!(plan.lower_branch, bn("feat-b-1"));
        assert_eq!(plan.upper_branch, bn("feat-b-2"));
        assert_eq!(plan.lower_parent, bn("feat-a"));
        assert_eq!(plan.children_to_reparent, vec![bn("feat-c")]);
    }

    #[test]
    fn plan_split_first_level_child() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), vec!["feat-a-1"]),
            ("feat-a-1", Some("feat-a"), vec!["feat-a-2"]),
            ("feat-a-2", Some("feat-a-1"), vec![]),
        ]);

        let plan = plan_split(
            &stack,
            &bn("feat-a"),
            &bn("feat-a-lower"),
            &bn("feat-a-upper"),
            "split-rev",
            "tip-rev",
            "main-rev",
        )
        .expect("plan");

        assert_eq!(plan.lower_parent, bn("main"));
        assert_eq!(plan.children_to_reparent, vec![bn("feat-a-1")]);
    }

    #[test]
    fn plan_split_multiple_children() {
        let stack = make_stack(vec![
            ("feat-a", Some("main"), vec!["feat-b", "feat-c"]),
            ("feat-b", Some("feat-a"), vec![]),
            ("feat-c", Some("feat-a"), vec![]),
        ]);

        let plan = plan_split(
            &stack,
            &bn("feat-a"),
            &bn("feat-a-1"),
            &bn("feat-a-2"),
            "split-rev",
            "tip-rev",
            "main-rev",
        )
        .expect("plan");

        assert_eq!(plan.children_to_reparent.len(), 2);
        assert!(plan.children_to_reparent.contains(&bn("feat-b")));
        assert!(plan.children_to_reparent.contains(&bn("feat-c")));
    }

    #[test]
    fn plan_split_untracked_branch_fails() {
        let stack = make_stack(vec![("feat-a", Some("main"), vec![])]);
        let result = plan_split(
            &stack,
            &bn("ghost"),
            &bn("ghost-1"),
            &bn("ghost-2"),
            "split-rev",
            "tip-rev",
            "parent-rev",
        );
        assert!(result.is_err());
    }
}
