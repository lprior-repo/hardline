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
    #[allow(dead_code)]
    CannotRemoveDefaultBranch,

    /// Worktree state transition invalid
    #[error("Invalid state transition from {0} to {1}")]
    InvalidStateTransition(super::WorktreeState, super::WorktreeState),

    /// Source path does not exist
    #[error("Source path does not exist: {0}")]
    #[allow(dead_code)]
    SourcePathNotFound(String),

    /// Repository path is not a valid git repository
    #[error("Not a valid git repository: {0}")]
    #[allow(dead_code)]
    InvalidRepository(String),

    /// Git operation failed
    #[error("Git operation failed: {0}")]
    #[allow(dead_code)]
    GitError(String),

    /// Worktree is not initialized
    #[error("Worktree '{0}' is not initialized")]
    #[allow(dead_code)]
    NotInitialized(super::WorktreeName),

    /// Worktree is already initialized
    #[error("Worktree '{0}' is already initialized")]
    AlreadyInitialized(super::WorktreeName),

    /// A pre-operation hook failed, aborting the operation
    #[error("Hook '{hook_name}' failed for event '{event}': {detail}")]
    HookFailed {
        event: String,
        hook_name: String,
        detail: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_name(s: &str) -> crate::domain::WorktreeName {
        crate::domain::WorktreeName::new_unchecked(s.to_string())
    }

    fn make_id() -> crate::domain::WorktreeId {
        crate::domain::WorktreeId::new_random()
    }

    #[test]
    fn name_already_exists_display() {
        let err = WorktreeDomainError::NameAlreadyExists("my-wt".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("my-wt"));
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn not_found_display() {
        let err = WorktreeDomainError::NotFound(make_id());
        let msg = format!("{err}");
        assert!(msg.contains("not found"));
    }

    #[test]
    fn invalid_name_display() {
        let err = WorktreeDomainError::InvalidName("bad name!".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bad name!"));
        assert!(msg.contains("Invalid worktree name"));
    }

    #[test]
    fn invalid_path_display() {
        let err = WorktreeDomainError::InvalidPath("relative/path".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("relative/path"));
        assert!(msg.contains("Invalid absolute path"));
    }

    #[test]
    fn invalid_branch_display() {
        let err = WorktreeDomainError::InvalidBranch("bad branch".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bad branch"));
        assert!(msg.contains("Invalid branch name"));
    }

    #[test]
    fn cannot_remove_default_branch_display() {
        let err = WorktreeDomainError::CannotRemoveDefaultBranch;
        let msg = format!("{err}");
        assert!(msg.contains("Cannot remove"));
    }

    #[test]
    fn invalid_state_transition_display() {
        let err = WorktreeDomainError::InvalidStateTransition(
            crate::domain::WorktreeState::Active,
            crate::domain::WorktreeState::Removed,
        );
        let msg = format!("{err}");
        assert!(msg.contains("Invalid state transition"));
    }

    #[test]
    fn source_path_not_found_display() {
        let err = WorktreeDomainError::SourcePathNotFound("/no/such/path".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("/no/such/path"));
        assert!(msg.contains("does not exist"));
    }

    #[test]
    fn invalid_repository_display() {
        let err = WorktreeDomainError::InvalidRepository("/not/repo".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("/not/repo"));
        assert!(msg.contains("Not a valid git repository"));
    }

    #[test]
    fn git_error_display() {
        let err = WorktreeDomainError::GitError("merge failed".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("merge failed"));
        assert!(msg.contains("Git operation"));
    }

    #[test]
    fn not_initialized_display() {
        let err = WorktreeDomainError::NotInitialized(make_name("my-wt"));
        let msg = format!("{err}");
        assert!(msg.contains("my-wt"));
        assert!(msg.contains("not initialized"));
    }

    #[test]
    fn already_initialized_display() {
        let err = WorktreeDomainError::AlreadyInitialized(make_name("my-wt"));
        let msg = format!("{err}");
        assert!(msg.contains("my-wt"));
        assert!(msg.contains("already initialized"));
    }

    #[test]
    fn all_variants_are_exhaustive() {
        let _ = WorktreeDomainError::NameAlreadyExists(String::new());
        let _ = WorktreeDomainError::NotFound(make_id());
        let _ = WorktreeDomainError::InvalidName(String::new());
        let _ = WorktreeDomainError::InvalidPath(String::new());
        let _ = WorktreeDomainError::InvalidBranch(String::new());
        let _ = WorktreeDomainError::CannotRemoveDefaultBranch;
        let _ = WorktreeDomainError::InvalidStateTransition(
            crate::domain::WorktreeState::Active,
            crate::domain::WorktreeState::Removed,
        );
        let _ = WorktreeDomainError::SourcePathNotFound(String::new());
        let _ = WorktreeDomainError::InvalidRepository(String::new());
        let _ = WorktreeDomainError::GitError(String::new());
        let _ = WorktreeDomainError::NotInitialized(make_name("wt"));
        let _ = WorktreeDomainError::AlreadyInitialized(make_name("wt"));
        let _ = WorktreeDomainError::HookFailed {
            event: "pre-create".to_string(),
            hook_name: "test".to_string(),
            detail: "detail".to_string(),
        };
    }
}
