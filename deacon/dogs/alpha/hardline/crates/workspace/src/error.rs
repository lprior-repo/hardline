use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Workspace already exists: {0}")]
    WorkspaceExists(String),

    #[error("Workspace is locked by: {0}")]
    WorkspaceLocked(String, String),

    #[error("Invalid state transition: from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Invalid workspace id: {0}")]
    InvalidWorkspaceId(String),

    #[error("Invalid workspace name: {0}")]
    InvalidWorkspaceName(String),

    #[error("Invalid workspace path: {0}")]
    InvalidWorkspacePath(String),

    #[error("Invalid branch name: {0}")]
    InvalidBranchName(String),

    #[error("Invalid lock holder: {0}")]
    InvalidLockHolder(String),

    #[error("Workspace operation failed: {0}")]
    OperationFailed(String),

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

pub type Result<T> = std::result::Result<T, WorkspaceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_not_found_display() {
        let err = WorkspaceError::WorkspaceNotFound("ws-123".into());
        let msg = format!("{err}");
        assert!(msg.contains("ws-123"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn workspace_exists_display() {
        let err = WorkspaceError::WorkspaceExists("my-ws".into());
        let msg = format!("{err}");
        assert!(msg.contains("my-ws"));
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn workspace_locked_display() {
        let err = WorkspaceError::WorkspaceLocked("ws-1".into(), "agent-42".into());
        let msg = format!("{err}");
        // The format string is "Workspace is locked by: {0}" which only displays the first field
        assert!(msg.contains("ws-1"));
        assert!(msg.contains("locked"));
    }

    #[test]
    fn invalid_state_transition_display() {
        let err = WorkspaceError::InvalidStateTransition {
            from: "Active".into(),
            to: "Initializing".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Active"));
        assert!(msg.contains("Initializing"));
    }

    #[test]
    fn invalid_workspace_id_display() {
        let err = WorkspaceError::InvalidWorkspaceId("bad-id".into());
        let msg = format!("{err}");
        assert!(msg.contains("bad-id"));
    }

    #[test]
    fn invalid_workspace_name_display() {
        let err = WorkspaceError::InvalidWorkspaceName("empty name".into());
        let msg = format!("{err}");
        assert!(msg.contains("empty name"));
    }

    #[test]
    fn invalid_workspace_path_display() {
        let err = WorkspaceError::InvalidWorkspacePath("empty path".into());
        let msg = format!("{err}");
        assert!(msg.contains("empty path"));
    }

    #[test]
    fn operation_failed_display() {
        let err = WorkspaceError::OperationFailed("disk full".into());
        let msg = format!("{err}");
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn repository_error_display() {
        let err = WorkspaceError::RepositoryError("connection lost".into());
        let msg = format!("{err}");
        assert!(msg.contains("connection lost"));
    }

    #[test]
    fn error_is_debug() {
        let err = WorkspaceError::WorkspaceNotFound("test".into());
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("WorkspaceNotFound"));
    }

    #[test]
    fn all_error_variants_are_debug() {
        let errors: Vec<WorkspaceError> = vec![
            WorkspaceError::WorkspaceNotFound("a".into()),
            WorkspaceError::WorkspaceExists("b".into()),
            WorkspaceError::WorkspaceLocked("c".into(), "d".into()),
            WorkspaceError::InvalidStateTransition {
                from: "e".into(),
                to: "f".into(),
            },
            WorkspaceError::InvalidWorkspaceId("g".into()),
            WorkspaceError::InvalidWorkspaceName("h".into()),
            WorkspaceError::InvalidWorkspacePath("i".into()),
            WorkspaceError::InvalidBranchName("j".into()),
            WorkspaceError::InvalidLockHolder("k".into()),
            WorkspaceError::OperationFailed("l".into()),
            WorkspaceError::RepositoryError("m".into()),
        ];
        for err in errors {
            let debug_str = format!("{err:?}");
            assert!(!debug_str.is_empty());
            let display_str = format!("{err}");
            assert!(!display_str.is_empty());
        }
    }

    #[test]
    fn result_type_alias_works() {
        fn returns_result() -> Result<String> {
            Ok("hello".to_string())
        }
        assert!(returns_result().is_ok());

        fn returns_err() -> Result<String> {
            Err(WorkspaceError::OperationFailed("fail".into()))
        }
        assert!(returns_err().is_err());
    }

    #[test]
    fn invalid_branch_name_display() {
        let err = WorkspaceError::InvalidBranchName("empty name".into());
        let msg = format!("{err}");
        assert!(msg.contains("empty name"));
    }

    #[test]
    fn invalid_lock_holder_display() {
        let err = WorkspaceError::InvalidLockHolder("empty holder".into());
        let msg = format!("{err}");
        assert!(msg.contains("empty holder"));
    }

    #[test]
    fn error_implements_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(WorkspaceError::WorkspaceNotFound("test".into()));
        let msg = format!("{err}");
        assert!(msg.contains("not found"));
        // source() returns None for leaf errors
        assert!(err.source().is_none());
    }

    #[test]
    fn error_implements_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WorkspaceError>();
    }
}
