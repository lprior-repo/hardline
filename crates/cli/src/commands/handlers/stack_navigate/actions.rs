//! Action layer for stack navigate - I/O operations via git CLI.
//!
//! Loads the stack from git metadata, resolves the navigation target,
//! and performs the git checkout.

use std::path::Path;
use std::process::Command;

use scp_core::output::Output;
use scp_stack::Stack;

use super::calc::resolve_navigate_target;
use super::data::{NavigateDirection, NavigateError, StackNavigateOptions, StackNavigateResult};

/// Run the stack navigate command.
///
/// # Errors
///
/// Returns `NavigateError` for any failure during the navigate operation.
pub fn run_stack_navigate(
    workdir: &Path,
    stack: &Stack,
    options: &StackNavigateOptions,
) -> Result<StackNavigateResult, NavigateError> {
    // 1. Get current branch
    let current_branch = get_current_branch(workdir)?;

    // 2. Check for dirty workspace
    if !check_workspace_clean(workdir) {
        return Err(NavigateError::DirtyWorkspace);
    }

    // 3. Validate stack has branches
    if stack.branches.is_empty() {
        return Err(NavigateError::EmptyStack);
    }

    // 4. Resolve target branch
    let target = resolve_navigate_target(stack, &current_branch, options.direction)?;

    match target {
        Some(target_branch) => {
            // 5. Checkout the target branch
            checkout_branch(workdir, &target_branch)?;

            let direction_label = match options.direction {
                NavigateDirection::Up => "parent",
                NavigateDirection::Down => "child",
                NavigateDirection::Top => "trunk",
                NavigateDirection::Bottom => "deepest",
                NavigateDirection::Prev => "previous sibling",
            };

            Output::success(&format!(
                "Navigated {} from '{}' to '{}' ({})",
                options.direction,
                current_branch.as_str(),
                target_branch.as_str(),
                direction_label,
            ));

            Ok(StackNavigateResult {
                from_branch: current_branch.as_str().to_string(),
                to_branch: Some(target_branch.as_str().to_string()),
                checked_out: true,
            })
        }
        None => {
            let direction_label = match options.direction {
                NavigateDirection::Up => "trunk",
                NavigateDirection::Down => "leaf",
                NavigateDirection::Top => "trunk",
                NavigateDirection::Bottom => "deepest branch",
                NavigateDirection::Prev => "first sibling",
            };
            Output::info(&format!(
                "Already at {} — nowhere to go {}",
                direction_label, options.direction,
            ));

            Ok(StackNavigateResult {
                from_branch: current_branch.as_str().to_string(),
                to_branch: None,
                checked_out: false,
            })
        }
    }
}

/// Get the current branch name.
fn get_current_branch(workdir: &Path) -> Result<scp_stack::BranchName, NavigateError> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(workdir)
        .output()
        .map_err(|e| NavigateError::CurrentBranchFailed(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("detached") {
            return Err(NavigateError::DetachedHead);
        }
        return Err(NavigateError::CurrentBranchFailed(stderr.to_string()));
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        return Err(NavigateError::DetachedHead);
    }

    Ok(scp_stack::BranchName::new(branch))
}

/// Check if workspace has uncommitted changes.
fn check_workspace_clean(workdir: &Path) -> bool {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) => out.status.success() && out.stdout.is_empty(),
        Err(_) => false,
    }
}

/// Checkout a branch.
fn checkout_branch(workdir: &Path, branch: &scp_stack::BranchName) -> Result<(), NavigateError> {
    let output = Command::new("git")
        .args(["checkout", branch.as_str()])
        .current_dir(workdir)
        .output()
        .map_err(|e| NavigateError::CheckoutFailed {
            branch: branch.as_str().to_string(),
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(NavigateError::CheckoutFailed {
            branch: branch.as_str().to_string(),
            stderr,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_workspace_clean_non_git_dir() {
        // Not a git repo, should return false (command fails)
        assert!(!check_workspace_clean(Path::new("/tmp")));
    }
}
