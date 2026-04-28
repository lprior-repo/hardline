//! VCS abstraction - Git and JJ backend support
//!
//! This module provides VCS abstraction for Git and Jujutsu operations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VcsError {
    #[error("Repository not found: {0}")]
    RepoNotFound(String),
    #[error("Branch not found: {0}")]
    BranchNotFound(String),
    #[error("Commit not found: {0}")]
    CommitNotFound(String),
    #[error("Merge conflict: {0}")]
    Conflict(String),
    #[error("VCS operation failed: {0}")]
    OperationFailed(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

pub type VcsResult<T> = Result<T, VcsError>;

/// VCS backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendType {
    Git,
    Jj,
}

/// Branch name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

impl BranchName {
    pub fn new(name: impl Into<String>) -> VcsResult<Self> {
        let name = name.into();
        if name.is_empty() {
            Err(VcsError::InvalidOperation(
                "Branch name cannot be empty".into(),
            ))
        } else {
            Ok(Self(name))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BranchName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Commit ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitId(String);

impl CommitId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CommitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Change ID (JJ-specific)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeId(String);

impl ChangeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Repository status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatus {
    pub clean: bool,
    pub branch: Option<BranchName>,
    pub commit_id: Option<CommitId>,
    pub has_conflicts: bool,
    pub uncommitted_files: Vec<String>,
}

/// A change in the repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub id: ChangeId,
    pub commit_id: CommitId,
    pub branch: Option<BranchName>,
    pub description: String,
    pub author: String,
    pub timestamp: i64,
}

/// VCS Backend trait
pub trait VcsBackend: Send + Sync {
    /// Get the backend type
    fn backend_type(&self) -> BackendType;

    /// Check if repository exists
    fn repo_exists(&self, path: &str) -> bool;

    /// Get repository status
    fn status(&self, path: &str) -> VcsResult<RepoStatus>;

    /// Get current branch name
    fn current_branch(&self, path: &str) -> VcsResult<BranchName>;

    /// Get commit log
    fn log(&self, path: &str, limit: usize) -> VcsResult<Vec<Change>>;

    /// Create a new branch
    fn create_branch(
        &self,
        path: &str,
        name: &BranchName,
        base: Option<&CommitId>,
    ) -> VcsResult<()>;

    /// Delete a branch
    fn delete_branch(&self, path: &str, name: &BranchName) -> VcsResult<()>;

    /// Checkout a branch/commit
    fn checkout(&self, path: &str, target: &str) -> VcsResult<()>;

    /// Commit changes
    fn commit(&self, path: &str, message: &str) -> VcsResult<CommitId>;

    /// Pull changes
    fn pull(&self, path: &str) -> VcsResult<()>;

    /// Push changes
    fn push(&self, path: &str) -> VcsResult<()>;

    /// Get diff
    fn diff(&self, path: &str, from: &CommitId, to: &CommitId) -> VcsResult<String>;

    /// Merge branches
    fn merge(&self, path: &str, source: &BranchName, target: &BranchName) -> VcsResult<CommitId>;

    /// Rebase branch
    fn rebase(&self, path: &str, branch: &BranchName, onto: &BranchName) -> VcsResult<()>;
}

/// Detect which VCS backend to use
pub fn detect_backend(path: &str) -> BackendType {
    let jj_dir = std::path::Path::new(path).join(".jj");
    let git_dir = std::path::Path::new(path).join(".git");

    if jj_dir.exists() || std::path::Path::new(path).join(".jj").is_dir() {
        BackendType::Jj
    } else if git_dir.exists() {
        BackendType::Git
    } else {
        BackendType::Git // Default to git
    }
}
