//! Git sync (rebase) operations
//!
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::process::Command;

use crate::vcs::{BranchName, VcsError};

use super::types::GitBackend;

impl GitBackend {
    /// Rebase the given branch onto its parent branch
    ///
    /// # Preconditions
    /// - Branch must exist in the repository
    /// - Working directory must be clean
    ///
    /// # Errors
    /// Returns `VcsError` if the rebase fails.
    pub fn sync(&self, branch: &BranchName, parent: &BranchName) -> Result<(), VcsError> {
        self.is_clean().and_then(|clean| {
            if clean {
                Ok(clean)
            } else {
                Err(VcsError::DirtyWorkingDirectory)
            }
        })?;

        let branches = self.list_branches()?;
        let current = self.current_branch()?;
        validate_branch_exists(branch, &current, &branches)?;
        validate_parent_exists(parent, &branches)?;

        let original_branch = current;
        checkout_branch(self.path.as_path(), branch)?;
        rebase_branch(self.path.as_path(), parent)?;

        let _ = original_branch
            .filter(|orig| orig.as_str() != branch.as_str())
            .and_then(|orig| {
                Command::new("git")
                    .args(["checkout", orig.as_str()])
                    .current_dir(self.path.as_path())
                    .output()
                    .ok()
            });

        Ok(())
    }
}

/// Verify that a branch exists (either as current or in the list).
fn validate_branch_exists(
    branch: &BranchName,
    current: &Option<BranchName>,
    branches: &[BranchName],
) -> Result<(), VcsError> {
    let is_current_branch = current
        .as_ref()
        .map(|b| b.as_str() == branch.as_str())
        .unwrap_or(false);
    let branch_exists =
        is_current_branch || branches.iter().any(|b| b.as_str() == branch.as_str());

    branch_exists
        .then_some(())
        .ok_or_else(|| VcsError::NotFound {
            entity: "Branch",
            id: branch.as_str().to_string(),
        })
}

/// Verify that a parent branch exists (or is "trunk").
fn validate_parent_exists(parent: &BranchName, branches: &[BranchName]) -> Result<(), VcsError> {
    let parent_exists =
        parent.as_str() == "trunk" || branches.iter().any(|b| b.as_str() == parent.as_str());

    parent_exists
        .then_some(())
        .ok_or_else(|| VcsError::NotFound {
            entity: "Parent branch",
            id: parent.as_str().to_string(),
        })
}

/// Checkout a branch using git CLI.
fn checkout_branch(path: &std::path::Path, branch: &BranchName) -> Result<(), VcsError> {
    let _result = Command::new("git")
        .args(["checkout", "--", branch.as_str()])
        .current_dir(path)
        .output()
        .map_err(|e| VcsError::CommandFailed {
            message: format!("Failed to checkout branch '{}'", branch.as_str()),
            source: Some(e),
        })
        .and_then(|output| {
            output
                .status
                .success()
                .then_some(())
                .ok_or_else(|| VcsError::GitCliFailed {
                    command: format!("git checkout -- {}", branch.as_str()),
                    source: None,
                })
        })?;
    Ok(())
}

/// Rebase the current branch onto the parent using git CLI.
fn rebase_branch(path: &std::path::Path, parent: &BranchName) -> Result<(), VcsError> {
    let _result = Command::new("git")
        .args(["rebase", "--update-refs", "--", parent.as_str()])
        .current_dir(path)
        .output()
        .map_err(|e| VcsError::CommandFailed {
            message: format!("Failed to rebase onto '{}'", parent.as_str()),
            source: Some(e),
        })
        .and_then(|output| {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let is_up_to_date =
                stderr.contains("Current branch") && stderr.contains("is up to date");
            (output.status.success() || is_up_to_date)
                .then_some(())
                .ok_or_else(|| VcsError::GitCliFailed {
                    command: format!("git rebase --update-refs -- {}", parent.as_str()),
                    source: None,
                })
        })?;
    Ok(())
}
