//! VcsBackend trait implementation for GitBackend (read operations)
//!
//! Delegates to `crate::gix` module functions where possible,
//! with inline gix for operations requiring detailed data (e.g., status counts).
//!
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::vcs::{
    BackendType, BranchName, CommitId, RepoStatus, RepositoryPath, VcsBackend, VcsError,
};

use super::helpers;
use super::types::GitBackend;

impl VcsBackend for GitBackend {
    /// Get the backend type
    ///
    /// # Postconditions
    /// - Q12: Always returns `BackendType::Git`
    fn backend_type(&self) -> BackendType {
        BackendType::Git
    }

    /// Get the repository path
    ///
    /// # Postconditions
    /// - I6: Returns absolute, canonical path
    fn path(&self) -> &RepositoryPath {
        &self.path
    }

    /// Get the current branch name
    ///
    /// Delegates to `crate::gix::branch::current()` via `helpers::current_branch_via_gix`.
    /// Maps detached HEAD (error in gix module) to `Ok(None)`.
    ///
    /// # Preconditions
    /// - P5: Repository is open and valid
    ///
    /// # Postconditions
    /// - Q2: Branch name has no `refs/heads/` prefix
    /// - Q3: Returns `None` for detached HEAD
    /// - Q3b: Returns `None` for unborn branch (empty repo)
    ///
    /// # Errors
    /// - `VcsError::GitReferenceError` if HEAD is unreadable (corrupt)
    fn current_branch(&self) -> Result<Option<BranchName>, VcsError> {
        let repo = helpers::lock_repo(&self.repo)?;
        helpers::current_branch_via_gix(&repo)
    }

    /// List all local branches
    ///
    /// Delegates to `crate::gix::branch::list()` via `helpers::list_branches_via_gix`.
    ///
    /// # Preconditions
    /// - P5: Repository is open and valid
    ///
    /// # Postconditions
    /// - Q4: Returns only local branches (refs/heads/*)
    /// - Q5: Branch names have no `refs/heads/` prefix
    ///
    /// # Errors
    /// - `VcsError::GitReferenceError` if references unreadable
    fn list_branches(&self) -> Result<Vec<BranchName>, VcsError> {
        let repo = helpers::lock_repo(&self.repo)?;
        helpers::list_branches_via_gix(&repo)
    }

    /// Get repository status
    ///
    /// Uses inline gix rather than `crate::gix::status` because the trait
    /// requires detailed change counts (added/modified/deleted), while the
    /// gix module's `status()` returns only `VcsStatus::Clean/Dirty`.
    ///
    /// # Preconditions
    /// - P5: Repository is open and valid
    ///
    /// # Postconditions
    /// - Q6: Accurately reflects working directory state
    /// - Q7: `has_changes` is false when clean
    ///
    /// # Errors
    /// - `VcsError::GitOpenFailed` if status check fails
    fn status(&self) -> Result<RepoStatus, VcsError> {
        let (added, modified, deleted) = {
            let repo = helpers::lock_repo(&self.repo)?;

            let mut opts = gix::status::Options::default();
            opts.include_untracked(false)
                .include_ignored(false)
                .include_unmodified(false);

            let statuses = repo
                .status_files_with_index(opts, std::iter::empty::<&std::path::Path>())
                .map_err(|e| VcsError::GitOpenFailed {
                    path: self.path.as_path().to_path_buf(),
                    message: format!("Failed to get status: {e}"),
                    source: None,
                })?;

            let mut added = 0u32;
            let mut modified = 0u32;
            let mut deleted = 0u32;

            for entry in statuses {
                let entry = entry.map_err(|e| VcsError::GitOpenFailed {
                    path: self.path.as_path().to_path_buf(),
                    message: format!("Failed to read status entry: {e}"),
                    source: None,
                })?;
                let change = entry.index_to_worktree_entry();
                if let Some(change) = change {
                    match change.kind() {
                        gix::status::change::Kind::New => added += 1,
                        gix::status::change::Kind::Modified => modified += 1,
                        gix::status::change::Kind::Deleted => deleted += 1,
                        _ => {}
                    }
                }
            }

            (added, modified, deleted)
        };

        let has_changes = added > 0 || modified > 0 || deleted > 0;

        let current_branch = self.current_branch()?;

        Ok(RepoStatus {
            has_changes,
            added,
            modified,
            deleted,
            current_branch,
        })
    }

    /// Check if a commit exists
    ///
    /// Delegates to `helpers::resolve_ref` which uses gix's `rev_parse`.
    ///
    /// # Preconditions
    /// - P5: Repository is open and valid
    /// - P8: Commit ID is not empty (validated by `CommitId`)
    ///
    /// # Postconditions
    /// - Q8: Returns `true` for valid commit
    /// - Q9: Returns `false` for non-existent commit
    /// - Q9b: Returns `false` for malformed/invalid revision specifiers
    ///
    /// # Errors
    /// - `VcsError::GitOpenFailed` if lookup fails due to repository corruption
    fn commit_exists(&self, id: &CommitId) -> Result<bool, VcsError> {
        let repo = helpers::lock_repo(&self.repo)?;
        match helpers::resolve_ref(&repo, id.as_str()) {
            Ok(_) => Ok(true),
            Err(VcsError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Rebase the given branch onto its parent branch
    ///
    /// # Preconditions
    /// - Branch must exist in the repository
    /// - Working directory must be clean
    ///
    /// # Errors
    /// Returns `VcsError` if the rebase fails.
    fn sync(&self, branch: &BranchName, parent: &BranchName) -> Result<(), VcsError> {
        GitBackend::sync(self, branch, parent)
    }
}
