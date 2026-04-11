//! Calculation layer for stack navigate - pure functions, no I/O.
//!
//! All functions are deterministic and testable without a VCS backend.

use scp_stack::{BranchName, Stack};

use super::data::{NavigateDirection, NavigateError};

/// Resolve the target branch for a navigation operation.
///
/// Given the current branch and navigation direction, returns the branch
/// name to switch to, or a `NavigateError` if navigation is not possible.
///
/// # Invariants
///
/// - Returns `Ok(Some(target))` if a target branch exists
/// - Returns `Ok(None)` if already at the boundary (no-op)
/// - Returns `Err` if preconditions are not met
pub fn resolve_navigate_target(
    stack: &Stack,
    current_branch: &BranchName,
    direction: NavigateDirection,
) -> Result<Option<BranchName>, NavigateError> {
    // For Top/Bottom, trunk is always valid even though it's not in stack.branches
    if current_branch == &stack.main_branch {
        return match direction {
            NavigateDirection::Top => Ok(None), // Already at trunk
            NavigateDirection::Bottom => resolve_bottom(stack, current_branch),
            NavigateDirection::Up => Ok(None), // Already at top
            NavigateDirection::Down => {
                // Find first child of trunk
                let child = stack
                    .branches
                    .iter()
                    .find(|b| b.parent.as_ref() == Some(&stack.main_branch));
                Ok(child.map(|b| b.name.clone()))
            }
            NavigateDirection::Prev => Ok(None), // Trunk has no siblings
        };
    }

    // Validate current branch is in stack (non-trunk branches must be tracked)
    let current = stack
        .branches
        .iter()
        .find(|b| &b.name == current_branch)
        .ok_or_else(|| NavigateError::NotInStack(current_branch.to_string()))?;

    match direction {
        NavigateDirection::Up => resolve_up(stack, current),
        NavigateDirection::Down => resolve_down(stack, current),
        NavigateDirection::Top => resolve_top(stack, current_branch),
        NavigateDirection::Bottom => resolve_bottom(stack, current_branch),
        NavigateDirection::Prev => resolve_prev(stack, current),
    }
}

/// Navigate to parent branch (toward trunk).
fn resolve_up(
    _stack: &Stack,
    current: &scp_stack::StackBranch,
) -> Result<Option<BranchName>, NavigateError> {
    match &current.parent {
        Some(parent) => Ok(Some(parent.clone())),
        None => Ok(None), // Already at trunk — no-op
    }
}

/// Navigate to first child branch (away from trunk).
fn resolve_down(
    _stack: &Stack,
    current: &scp_stack::StackBranch,
) -> Result<Option<BranchName>, NavigateError> {
    if current.children.is_empty() {
        Ok(None) // Leaf node — no-op
    } else {
        // Return the first child (alphabetically sorted)
        let mut children = current.children.clone();
        children.sort();
        Ok(Some(
            children
                .into_iter()
                .next()
                .expect("children is non-empty"),
        ))
    }
}

/// Navigate to the trunk (top/root of the stack).
fn resolve_top(
    stack: &Stack,
    current_branch: &BranchName,
) -> Result<Option<BranchName>, NavigateError> {
    if current_branch == &stack.main_branch {
        Ok(None) // Already at trunk — no-op
    } else {
        Ok(Some(stack.main_branch.clone()))
    }
}

/// Navigate to the deepest descendant branch.
fn resolve_bottom(
    stack: &Stack,
    current_branch: &BranchName,
) -> Result<Option<BranchName>, NavigateError> {
    let is_trunk = current_branch == &stack.main_branch;

    // If not trunk, check if current branch is already a leaf (no descendants)
    if !is_trunk {
        let descendants = stack.descendants(current_branch);
        if descendants.is_empty() {
            // Current branch is already at the bottom — no-op
            return Ok(None);
        }
    }

    // Collect all leaf branches (no children) and pick the deepest one
    let mut deepest: Option<(BranchName, usize)> = None;

    for branch in &stack.branches {
        let is_descendant = if is_trunk {
            true // From trunk, all branches are potential targets
        } else {
            stack.descendants(current_branch).contains(&branch.name)
        };

        let is_leaf = branch.children.is_empty();
        if is_descendant && is_leaf {
            let depth = count_ancestors(stack, &branch.name);
            match &deepest {
                Some((_, d)) if depth > *d => {
                    deepest = Some((branch.name.clone(), depth));
                }
                None => {
                    deepest = Some((branch.name.clone(), depth));
                }
                _ => {}
            }
        }
    }

    Ok(deepest.map(|(name, _)| name))
}

/// Count how many ancestors a branch has (depth in the stack).
fn count_ancestors(stack: &Stack, branch: &BranchName) -> usize {
    let mut count = 0;
    let mut current = branch.clone();
    loop {
        let parent = stack
            .branches
            .iter()
            .find(|b| b.name == current)
            .and_then(|b| b.parent.clone());
        match parent {
            Some(p) => {
                count += 1;
                current = p;
            }
            None => break,
        }
    }
    count
}

/// Navigate to the previous sibling (alphabetically before current).
fn resolve_prev(
    stack: &Stack,
    current: &scp_stack::StackBranch,
) -> Result<Option<BranchName>, NavigateError> {
    let siblings = stack.get_siblings(&current.name);
    if siblings.len() <= 1 {
        Ok(None) // No siblings — no-op
    } else {
        let pos = siblings
            .iter()
            .position(|s| s == &current.name)
            .unwrap_or(0);
        if pos == 0 {
            Ok(None) // Already first sibling — no-op
        } else {
            Ok(Some(siblings[pos - 1].clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_stack::{StackBranch, Stack};

    fn bn(name: &str) -> BranchName {
        BranchName::new(name.to_string())
    }

    /// Create a linear stack: main -> a -> b -> c
    fn linear_stack() -> Stack {
        let main = bn("main");
        let mut stack = Stack::new(main.clone());

        stack
            .add_branch(StackBranch {
                name: bn("a"),
                parent: Some(main),
                children: vec![bn("b")],
                needs_restack: false,
                pr_info: None,
            })
            .ok();

        stack
            .add_branch(StackBranch {
                name: bn("b"),
                parent: Some(bn("a")),
                children: vec![bn("c")],
                needs_restack: false,
                pr_info: None,
            })
            .ok();

        stack
            .add_branch(StackBranch {
                name: bn("c"),
                parent: Some(bn("b")),
                children: vec![],
                needs_restack: false,
                pr_info: None,
            })
            .ok();

        stack
    }

    /// Create a fan-out stack: main -> a, main -> b, main -> c
    fn fan_out_stack() -> Stack {
        let main = bn("main");
        let mut stack = Stack::new(main);

        for name in ["a", "b", "c"] {
            stack
                .add_branch(StackBranch {
                    name: bn(name),
                    parent: Some(bn("main")),
                    children: vec![],
                    needs_restack: false,
                    pr_info: None,
                })
                .ok();
        }

        stack
    }

    // ---- Up navigation ----

    #[test]
    fn up_from_child_returns_parent() {
        let stack = linear_stack();
        let target = resolve_navigate_target(&stack, &bn("c"), NavigateDirection::Up);
        assert_eq!(target.ok().flatten(), Some(bn("b")));
    }

    #[test]
    fn up_from_first_level_returns_trunk() {
        let stack = linear_stack();
        let target = resolve_navigate_target(&stack, &bn("a"), NavigateDirection::Up);
        assert_eq!(target.ok().flatten(), Some(bn("main")));
    }

    #[test]
    fn up_from_trunk_returns_none() {
        let stack = linear_stack();
        let target = resolve_navigate_target(&stack, &bn("main"), NavigateDirection::Up);
        assert_eq!(target.ok().flatten(), None);
    }

    // ---- Down navigation ----

    #[test]
    fn down_from_parent_returns_child() {
        let stack = linear_stack();
        let target = resolve_navigate_target(&stack, &bn("a"), NavigateDirection::Down);
        assert_eq!(target.ok().flatten(), Some(bn("b")));
    }

    #[test]
    fn down_from_leaf_returns_none() {
        let stack = linear_stack();
        let target = resolve_navigate_target(&stack, &bn("c"), NavigateDirection::Down);
        assert_eq!(target.ok().flatten(), None);
    }

    // ---- Top navigation ----

    #[test]
    fn top_from_any_branch_returns_trunk() {
        let stack = linear_stack();
        let target = resolve_navigate_target(&stack, &bn("c"), NavigateDirection::Top);
        assert_eq!(target.ok().flatten(), Some(bn("main")));
    }

    #[test]
    fn top_from_trunk_returns_none() {
        let stack = linear_stack();
        let target = resolve_navigate_target(&stack, &bn("main"), NavigateDirection::Top);
        assert_eq!(target.ok().flatten(), None);
    }

    // ---- Bottom navigation ----

    #[test]
    fn bottom_from_trunk_returns_deepest_leaf() {
        let stack = linear_stack();
        // Debug: verify stack structure
        assert_eq!(stack.branches.len(), 3, "stack should have 3 branches");
        assert_eq!(stack.main_branch, bn("main"), "main_branch should be main");

        // Verify c is a leaf
        let c_branch = stack.branches.iter().find(|b| b.name == bn("c"));
        assert!(c_branch.is_some(), "c should exist in stack");
        assert!(c_branch.expect("c").children.is_empty(), "c should have no children");

        let target = resolve_navigate_target(&stack, &bn("main"), NavigateDirection::Bottom);
        assert_eq!(target.ok().flatten(), Some(bn("c")));
    }

    #[test]
    fn bottom_from_middle_returns_leaf() {
        let stack = linear_stack();
        let target = resolve_navigate_target(&stack, &bn("a"), NavigateDirection::Bottom);
        assert_eq!(target.ok().flatten(), Some(bn("c")));
    }

    #[test]
    fn bottom_from_leaf_returns_none() {
        let stack = linear_stack();
        let target = resolve_navigate_target(&stack, &bn("c"), NavigateDirection::Bottom);
        assert_eq!(target.ok().flatten(), None);
    }

    // ---- Prev navigation ----

    #[test]
    fn prev_from_middle_sibling() {
        let stack = fan_out_stack();
        // Siblings are sorted: [a, b, c]. Prev from b -> a
        let target = resolve_navigate_target(&stack, &bn("b"), NavigateDirection::Prev);
        assert_eq!(target.ok().flatten(), Some(bn("a")));
    }

    #[test]
    fn prev_from_first_sibling_returns_none() {
        let stack = fan_out_stack();
        let target = resolve_navigate_target(&stack, &bn("a"), NavigateDirection::Prev);
        assert_eq!(target.ok().flatten(), None);
    }

    #[test]
    fn prev_from_only_child_returns_none() {
        let stack = linear_stack();
        // c has no siblings
        let target = resolve_navigate_target(&stack, &bn("c"), NavigateDirection::Prev);
        assert_eq!(target.ok().flatten(), None);
    }

    // ---- Error cases ----

    #[test]
    fn not_in_stack_returns_error() {
        let stack = linear_stack();
        let target = resolve_navigate_target(&stack, &bn("nonexistent"), NavigateDirection::Up);
        assert!(target.is_err());
        assert!(matches!(target, Err(NavigateError::NotInStack(_))));
    }

    #[test]
    fn empty_stack_returns_error_for_any_branch() {
        let stack = Stack::new(bn("main"));
        let target = resolve_navigate_target(&stack, &bn("anything"), NavigateDirection::Up);
        assert!(target.is_err());
    }
}
