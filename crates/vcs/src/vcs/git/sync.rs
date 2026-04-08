//! Git sync (rebase) operations
//!
//! Pure gix implementation — no CLI spawning.
//! Delegates to `crate::gix::rebase` for the rebase algorithm.
//!
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::vcs::{BranchName, VcsError};

use super::helpers;
use super::types::GitBackend;

impl GitBackend {
    /// Rebase the given branch onto its parent branch
    ///
    /// Uses pure gix (no CLI). The rebase replays commits from `branch`
    /// that are not in `parent` onto the tip of `parent`.
    ///
    /// # Preconditions
    /// - Branch must exist in the repository
    /// - Working directory must be clean
    ///
    /// # Postconditions
    /// - Branch reference is updated to the new tip
    /// - If rebase encounters conflicts, branch points to last successful state
    ///
    /// # Errors
    /// - `VcsError::DirtyWorkingDirectory` if working tree has changes
    /// - `VcsError::NotFound` if branch or parent doesn't exist
    /// - `VcsError::RebaseConflict` if cherry-pick encounters conflicts
    /// - `VcsError::RebaseFailed` for other rebase failures
    /// - `VcsError::NoMergeBase` if branches share no common ancestor
    pub fn sync(&self, branch: &BranchName, parent: &BranchName) -> Result<(), VcsError> {
        // Precondition: working directory must be clean
        self.is_clean().and_then(|clean| {
            if clean {
                Ok(clean)
            } else {
                Err(VcsError::DirtyWorkingDirectory)
            }
        })?;

        // Precondition: branch must exist
        let branches = self.list_branches()?;
        let current = self.current_branch()?;

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
            })?;

        // Precondition: parent must exist (or be "trunk" special case)
        let parent_exists =
            parent.as_str() == "trunk" || branches.iter().any(|b| b.as_str() == parent.as_str());

        parent_exists
            .then_some(())
            .ok_or_else(|| VcsError::NotFound {
                entity: "Parent branch",
                id: parent.as_str().to_string(),
            })?;

        // Perform pure gix rebase
        let repo = helpers::lock_repo(&self.repo)?;

        let result = crate::gix::rebase::rebase_branch_onto(
            &repo,
            branch.as_str(),
            parent.as_str(),
        )
        .map_err(|e| VcsError::from(e))?;

        match result {
            crate::gix::rebase::RebaseResult::Success { .. } => Ok(()),
            crate::gix::rebase::RebaseResult::AlreadyUpToDate => Ok(()),
            crate::gix::rebase::RebaseResult::Conflict {
                conflicted_files,
                commits_replayed,
                remaining_commits,
            } => Err(VcsError::RebaseConflict {
                branch: branch.as_str().to_string(),
                conflicted_files,
            }
            .into()).map_err(|e| {
                // Log additional context via the error message
                let _ = (commits_replayed, remaining_commits);
                e
            }),
        }
    }
}
