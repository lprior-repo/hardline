//! VCS backend trait definition
//!
//! This module provides the `VcsBackend` trait for Git operations.

use super::change::RepoStatus;
use super::errors::VcsError;
use super::types::{BranchName, CommitId, RepositoryPath};
use super::BackendType;

// ============================================================================
// VcsBackend Trait
// ============================================================================

/// Unified VCS backend trait for Git operations
///
/// # Type Invariants
/// - All methods return `Result<T, VcsError>` - never panic
/// - Implementations must be thread-safe (Send + Sync)
/// - Operations are idempotent where semantically meaningful
pub trait VcsBackend: Send + Sync {
    /// Get the backend type for this implementation
    fn backend_type(&self) -> BackendType;

    /// Get the repository path
    fn path(&self) -> &RepositoryPath;

    /// Detect the current branch
    ///
    /// # Returns
    /// - `Ok(Some(BranchName))` if on a branch
    /// - `Ok(None)` if in detached HEAD state (Git) or equivalent
    ///
    /// # Errors
    /// Returns `VcsError` if the branch cannot be determined.
    fn current_branch(&self) -> Result<Option<BranchName>, VcsError>;

    /// List all branches in the repository
    ///
    /// # Errors
    /// Returns `VcsError` if branches cannot be listed.
    fn list_branches(&self) -> Result<Vec<BranchName>, VcsError>;

    /// Get the repository status
    ///
    /// # Errors
    /// Returns `VcsError` if status cannot be determined.
    fn status(&self) -> Result<RepoStatus, VcsError>;

    /// Check if a commit exists in the repository
    ///
    /// # Errors
    /// Returns `VcsError` if the commit check fails.
    fn commit_exists(&self, id: &CommitId) -> Result<bool, VcsError>;

    /// Check if the working directory is clean (no uncommitted changes)
    ///
    /// # Default Implementation
    /// Uses `status()` to determine if there are changes
    ///
    /// # Errors
    /// Returns `VcsError` if status cannot be determined.
    fn is_clean(&self) -> Result<bool, VcsError> {
        self.status().map(|s| !s.has_changes)
    }

    /// Rebase the given branch onto its parent branch
    ///
    /// # Preconditions
    /// - Branch must exist in the repository
    /// - Working directory must be clean
    ///
    /// # Errors
    /// Returns `VcsError` if the rebase fails.
    fn sync(&self, branch: &BranchName, parent: &BranchName) -> Result<(), VcsError>;
}
