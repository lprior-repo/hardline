use thiserror::Error;

use crate::domain::entities::session::SessionState;
use crate::domain::workspace_state::WorkspaceState;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TaskIdError {
    #[error("task ID cannot be empty")]
    InvalidInput,
    #[error("task ID must start with 'bd-' prefix")]
    InvalidPrefix,
    #[error("task ID suffix must be hexadecimal characters only")]
    InvalidHex,
    #[error("task ID suffix after 'bd-' cannot be empty")]
    EmptySuffix,
}

#[derive(Error, Debug)]
pub enum SessionError {
    // Session errors
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Session already active: {0}")]
    AlreadyActive(String),

    #[error("Session expired: {0}")]
    Expired(String),

    // State transition errors
    #[error("Invalid workspace state transition: {from:?} -> {to:?}")]
    InvalidTransition {
        from: WorkspaceState,
        to: WorkspaceState,
    },

    #[error("Invalid session state transition: {from:?} -> {to:?}")]
    InvalidSessionTransition {
        from: SessionState,
        to: SessionState,
    },

    #[error("Invalid branch transition: {from:?} -> {to:?}")]
    InvalidBranchTransition { from: String, to: String },

    // Workspace errors (P1-P6 preconditions)
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Workspace already exists: {0}")]
    WorkspaceExists(String),

    #[error("Workspace is locked by: {0}")]
    WorkspaceLocked(String),

    #[error("Invalid workspace ID: {0}")]
    InvalidWorkspaceId(String),

    #[error("Invalid workspace name: {0}")]
    InvalidWorkspaceName(String),

    #[error("Invalid workspace path: {0}")]
    InvalidWorkspacePath(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Repository error: {0}")]
    RepositoryError(String),

    // Bead errors (P7-P10 preconditions)
    #[error("Bead not found: {0}")]
    BeadNotFound(String),

    #[error("Bead already exists: {0}")]
    BeadAlreadyExists(String),

    #[error("Bead already claimed: {0}")]
    BeadAlreadyClaimed(String),

    #[error("Invalid bead ID: {0}")]
    InvalidBeadId(String),

    #[error("Invalid bead title: {0}")]
    InvalidBeadTitle(String),

    #[error("Dependency cycle detected: {0}")]
    DependencyCycle(String),

    #[error("Bead is blocked by: {0}")]
    BlockedBy(String),

    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    // General identifier/path errors
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Invalid priority: {0}")]
    InvalidPriority(String),

    #[error("Invalid issue type: {0}")]
    InvalidIssueType(String),
}

pub type Result<T> = std::result::Result<T, SessionError>;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // TaskIdError Tests
    // =========================================================================

    mod task_id_error_tests {
        use super::*;

        #[test]
        fn task_id_error_invalid_input_display() {
            let err = TaskIdError::InvalidInput;
            assert_eq!(format!("{err}"), "task ID cannot be empty");
        }

        #[test]
        fn task_id_error_invalid_prefix_display() {
            let err = TaskIdError::InvalidPrefix;
            assert_eq!(format!("{err}"), "task ID must start with 'bd-' prefix");
        }

        #[test]
        fn task_id_error_invalid_hex_display() {
            let err = TaskIdError::InvalidHex;
            assert_eq!(
                format!("{err}"),
                "task ID suffix must be hexadecimal characters only"
            );
        }

        #[test]
        fn task_id_error_empty_suffix_display() {
            let err = TaskIdError::EmptySuffix;
            assert_eq!(
                format!("{err}"),
                "task ID suffix after 'bd-' cannot be empty"
            );
        }

        #[test]
        fn task_id_error_equality() {
            assert_eq!(TaskIdError::InvalidInput, TaskIdError::InvalidInput);
            assert_ne!(TaskIdError::InvalidInput, TaskIdError::InvalidPrefix);
        }

        #[test]
        fn task_id_error_clone() {
            let err = TaskIdError::InvalidHex;
            assert_eq!(err.clone(), err);
        }

        #[test]
        fn task_id_error_debug_format() {
            let err = TaskIdError::EmptySuffix;
            let debug = format!("{err:?}");
            assert!(debug.contains("EmptySuffix"));
        }
    }

    // =========================================================================
    // SessionError Tests
    // =========================================================================

    mod session_error_tests {
        use super::*;
        use crate::domain::entities::session::SessionState;
        use crate::domain::workspace_state::WorkspaceState;

        #[test]
        fn session_error_not_found_display() {
            let err = SessionError::NotFound("test-id".to_string());
            let msg = format!("{err}");
            assert!(msg.contains("test-id"));
            assert!(msg.contains("not found"));
        }

        #[test]
        fn session_error_already_active_display() {
            let err = SessionError::AlreadyActive("test".to_string());
            let msg = format!("{err}");
            assert!(msg.contains("already active"));
        }

        #[test]
        fn session_error_expired_display() {
            let err = SessionError::Expired("session-1".to_string());
            let msg = format!("{err}");
            assert!(msg.contains("expired"));
        }

        #[test]
        fn session_error_invalid_workspace_transition_display() {
            let err = SessionError::InvalidTransition {
                from: WorkspaceState::Created,
                to: WorkspaceState::Merged,
            };
            let msg = format!("{err}");
            assert!(msg.contains("Created"));
            assert!(msg.contains("Merged"));
        }

        #[test]
        fn session_error_invalid_session_transition_display() {
            let err = SessionError::InvalidSessionTransition {
                from: SessionState::Completed,
                to: SessionState::Active,
            };
            let msg = format!("{err}");
            assert!(msg.contains("Completed"));
            assert!(msg.contains("Active"));
        }

        #[test]
        fn session_error_invalid_branch_transition_display() {
            let err = SessionError::InvalidBranchTransition {
                from: "Detached".to_string(),
                to: "Detached".to_string(),
            };
            let msg = format!("{err}");
            assert!(msg.contains("Detached"));
        }

        #[test]
        fn session_error_workspace_not_found_display() {
            let err = SessionError::WorkspaceNotFound("ws-1".to_string());
            let msg = format!("{err}");
            assert!(msg.contains("ws-1"));
        }

        #[test]
        fn session_error_workspace_exists_display() {
            let err = SessionError::WorkspaceExists("ws-1".to_string());
            assert!(format!("{err}").contains("already exists"));
        }

        #[test]
        fn session_error_workspace_locked_display() {
            let err = SessionError::WorkspaceLocked("user-1".to_string());
            assert!(format!("{err}").contains("locked"));
        }

        #[test]
        fn session_error_database_error_display() {
            let err = SessionError::DatabaseError("disk full".to_string());
            let msg = format!("{err}");
            assert!(msg.contains("disk full"));
        }

        #[test]
        fn session_error_serialization_error_display() {
            let err = SessionError::SerializationError("bad json".to_string());
            let msg = format!("{err}");
            assert!(msg.contains("bad json"));
        }

        #[test]
        fn session_error_invalid_priority_display() {
            let err = SessionError::InvalidPriority("99".to_string());
            let msg = format!("{err}");
            assert!(msg.contains("99"));
        }

        #[test]
        fn session_error_invalid_issue_type_display() {
            let err = SessionError::InvalidIssueType("invalid".to_string());
            assert!(format!("{err}").contains("invalid"));
        }

        #[test]
        fn session_error_bead_not_found_display() {
            let err = SessionError::BeadNotFound("bd-1".to_string());
            assert!(format!("{err}").contains("bd-1"));
        }

        #[test]
        fn session_error_bead_already_exists_display() {
            let err = SessionError::BeadAlreadyExists("bd-1".to_string());
            assert!(format!("{err}").contains("already exists"));
        }

        #[test]
        fn session_error_dependency_cycle_display() {
            let err = SessionError::DependencyCycle("bd-1 -> bd-2 -> bd-1".to_string());
            assert!(format!("{err}").contains("cycle"));
        }

        #[test]
        fn session_error_blocked_by_display() {
            let err = SessionError::BlockedBy("bd-999".to_string());
            assert!(format!("{err}").contains("bd-999"));
        }

        #[test]
        fn session_error_operation_failed_display() {
            let err = SessionError::OperationFailed("timeout".to_string());
            assert!(format!("{err}").contains("timeout"));
        }

        #[test]
        fn session_error_repository_error_display() {
            let err = SessionError::RepositoryError("connection refused".to_string());
            assert!(format!("{err}").contains("connection refused"));
        }

        #[test]
        fn session_error_debug_format() {
            let err = SessionError::NotFound("id-1".to_string());
            let debug = format!("{err:?}");
            assert!(debug.contains("NotFound"));
            assert!(debug.contains("id-1"));
        }

        #[test]
        fn session_error_invalid_workspace_id_display() {
            let err = SessionError::InvalidWorkspaceId("bad-id".to_string());
            assert!(format!("{err}").contains("bad-id"));
            assert!(format!("{err}").contains("Invalid workspace ID"));
        }

        #[test]
        fn session_error_invalid_workspace_name_display() {
            let err = SessionError::InvalidWorkspaceName("".to_string());
            assert!(format!("{err}").contains("Invalid workspace name"));
        }

        #[test]
        fn session_error_invalid_workspace_path_display() {
            let err = SessionError::InvalidWorkspacePath("/bad/path".to_string());
            assert!(format!("{err}").contains("/bad/path"));
            assert!(format!("{err}").contains("Invalid workspace path"));
        }

        #[test]
        fn session_error_invalid_bead_id_display() {
            let err = SessionError::InvalidBeadId("xyz".to_string());
            assert!(format!("{err}").contains("xyz"));
            assert!(format!("{err}").contains("Invalid bead ID"));
        }

        #[test]
        fn session_error_invalid_bead_title_display() {
            let err = SessionError::InvalidBeadTitle("".to_string());
            assert!(format!("{err}").contains("Invalid bead title"));
        }

        #[test]
        fn session_error_bead_already_claimed_display() {
            let err = SessionError::BeadAlreadyClaimed("bd-1".to_string());
            assert!(format!("{err}").contains("bd-1"));
            assert!(format!("{err}").contains("already claimed"));
        }

        #[test]
        fn session_error_invalid_dependency_display() {
            let err = SessionError::InvalidDependency("nonexistent".to_string());
            assert!(format!("{err}").contains("nonexistent"));
            assert!(format!("{err}").contains("Invalid dependency"));
        }

        #[test]
        fn session_error_invalid_identifier_display() {
            let err = SessionError::InvalidIdentifier("bad@id".to_string());
            assert!(format!("{err}").contains("bad@id"));
            assert!(format!("{err}").contains("Invalid identifier"));
        }

        #[test]
        fn session_error_invalid_path_display() {
            let err = SessionError::InvalidPath("relative/path".to_string());
            assert!(format!("{err}").contains("relative/path"));
            assert!(format!("{err}").contains("Invalid path"));
        }

        #[test]
        fn session_error_all_variants_constructible() {
            let _ = SessionError::NotFound(String::new());
            let _ = SessionError::AlreadyActive(String::new());
            let _ = SessionError::Expired(String::new());
            let _ = SessionError::InvalidTransition {
                from: WorkspaceState::Created,
                to: WorkspaceState::Merged,
            };
            let _ = SessionError::InvalidSessionTransition {
                from: SessionState::Active,
                to: SessionState::Completed,
            };
            let _ = SessionError::InvalidBranchTransition { from: String::new(), to: String::new() };
            let _ = SessionError::WorkspaceNotFound(String::new());
            let _ = SessionError::WorkspaceExists(String::new());
            let _ = SessionError::WorkspaceLocked(String::new());
            let _ = SessionError::InvalidWorkspaceId(String::new());
            let _ = SessionError::InvalidWorkspaceName(String::new());
            let _ = SessionError::InvalidWorkspacePath(String::new());
            let _ = SessionError::OperationFailed(String::new());
            let _ = SessionError::RepositoryError(String::new());
            let _ = SessionError::BeadNotFound(String::new());
            let _ = SessionError::BeadAlreadyExists(String::new());
            let _ = SessionError::BeadAlreadyClaimed(String::new());
            let _ = SessionError::InvalidBeadId(String::new());
            let _ = SessionError::InvalidBeadTitle(String::new());
            let _ = SessionError::DependencyCycle(String::new());
            let _ = SessionError::BlockedBy(String::new());
            let _ = SessionError::InvalidDependency(String::new());
            let _ = SessionError::DatabaseError(String::new());
            let _ = SessionError::SerializationError(String::new());
            let _ = SessionError::InvalidIdentifier(String::new());
            let _ = SessionError::InvalidPath(String::new());
            let _ = SessionError::InvalidPriority(String::new());
            let _ = SessionError::InvalidIssueType(String::new());
        }
    }
}
