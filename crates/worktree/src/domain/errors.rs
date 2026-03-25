use thiserror::Error;

/// Domain-specific error types for worktree operations
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum WorktreeDomainError {
    /// Worktree with this name already exists
    #[error("Worktree with name '{0}' already exists")]
    NameAlreadyExists(String),

    /// Worktree with this ID does not exist
    #[error("Worktree with ID '{0}' not found")]
    NotFound(super::WorktreeId),

    /// Invalid worktree name (empty or contains invalid characters)
    #[error("Invalid worktree name: {0}")]
    InvalidName(String),

    /// Invalid absolute path format
    #[error("Invalid absolute path: {0}")]
    InvalidPath(String),

    /// Invalid branch name
    #[error("Invalid branch name: {0}")]
    InvalidBranch(String),

    /// Cannot remove default branch worktree
    #[error("Cannot remove worktree for default branch")]
    CannotRemoveDefaultBranch,

    /// Worktree state transition invalid
    #[error("Invalid state transition from {0} to {1}")]
    InvalidStateTransition(super::WorktreeState, super::WorktreeState),

    /// Source path does not exist
    #[error("Source path does not exist: {0}")]
    SourcePathNotFound(String),

    /// Repository path is not a valid git repository
    #[error("Not a valid git repository: {0}")]
    InvalidRepository(String),

    /// Git operation failed
    #[error("Git operation failed: {0}")]
    GitError(String),

    /// Worktree is not initialized
    #[error("Worktree '{0}' is not initialized")]
    NotInitialized(super::WorktreeName),

    /// Worktree is already initialized
    #[error("Worktree '{0}' is already initialized")]
    AlreadyInitialized(super::WorktreeName),
}
