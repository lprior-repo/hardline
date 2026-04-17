//! VCS backend trait definition
//!
//! Unified interface for VCS operations supporting Git.

use super::types::{Branch, Commit, CommitId, RepoStatus, VcsStatus, Workspace};
use crate::error::Result;

/// VCS backend trait - unified interface for Git
pub trait VcsBackend: Send + Sync {
    /// Get current branch name
    fn current_branch(&self) -> Result<String>;

    /// List all branches
    fn list_branches(&self) -> Result<Vec<Branch>>;

    /// Create a new branch
    fn create_branch(&self, name: &str) -> Result<()>;

    /// Switch to a branch
    fn switch_branch(&self, name: &str) -> Result<()>;

    /// Push changes to remote
    fn push(&self) -> Result<()>;

    /// Pull changes from remote
    fn pull(&self) -> Result<()>;

    /// Rebase current branch onto another
    fn rebase(&self, onto: &str) -> Result<()>;

    /// Merge a branch into current
    fn merge(&self, branch: &str) -> Result<()>;

    /// Get commit log
    fn log(&self, limit: usize) -> Result<Vec<Commit>>;

    /// Get status of working copy
    fn status(&self) -> Result<VcsStatus>;

    /// Check if VCS is initialized (checks own repo_path)
    fn is_initialized(&self) -> Result<bool>;

    // ========================================================================
    // Extended operations (ported from Isolate VcsBackend)
    // ========================================================================

    /// Check if a repository exists at the given path
    fn repo_exists(&self, path: &str) -> bool;

    /// Checkout a branch or commit ref
    fn checkout(&self, target: &str) -> Result<()>;

    /// Commit all staged/current changes with the given message
    fn commit(&self, message: &str) -> Result<CommitId>;

    /// Get diff between two commits
    fn diff(&self, from: &CommitId, to: &CommitId) -> Result<String>;

    /// Get detailed repository status
    fn repo_status(&self) -> Result<RepoStatus>;

    // ========================================================================
    // Workspace operations (from Isolate)
    // ========================================================================

    /// Create a new workspace
    fn create_workspace(&self, name: &str) -> Result<()>;

    /// Switch to a workspace
    fn switch_workspace(&self, name: &str) -> Result<()>;

    /// List workspaces
    fn list_workspaces(&self) -> Result<Vec<Workspace>>;

    /// Delete a workspace
    fn delete_workspace(&self, name: &str) -> Result<()>;

    /// Fork a workspace from another workspace
    fn fork_workspace(&self, source: &str, target: &str) -> Result<()>;

    /// Merge a workspace into main
    fn merge_workspace(&self, name: &str) -> Result<()>;

    /// Abort workspace - restore working copy to last commit, discarding uncommitted changes
    fn abort_workspace(&self, name: &str) -> Result<()>;
}
