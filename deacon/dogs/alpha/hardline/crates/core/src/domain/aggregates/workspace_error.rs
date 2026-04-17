//! Workspace domain errors.
//!
//! Defines all errors that can occur during workspace operations.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use thiserror::Error;

use crate::domain::workspace::WorkspaceState;

/// Errors that can occur during workspace operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkspaceError {
    /// Invalid state transition
    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition {
        from: WorkspaceState,
        to: WorkspaceState,
    },

    /// Workspace path does not exist
    #[error("workspace path does not exist: {0}")]
    PathNotFound(PathBuf),

    /// Workspace is not in a ready state
    #[error("workspace is not ready: {0:?}")]
    NotReady(WorkspaceState),

    /// Workspace is not active
    #[error("workspace is not active: {0:?}")]
    NotActive(WorkspaceState),

    /// Workspace has been removed
    #[error("workspace has been removed")]
    Removed,

    /// Cannot use workspace in current state
    #[error("cannot use workspace in state: {0:?}")]
    CannotUse(WorkspaceState),

    /// Workspace name already exists
    #[error("workspace name already exists: {0}")]
    NameAlreadyExists(crate::domain::identifiers::WorkspaceName),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::workspace::WorkspaceState;

    #[test]
    fn invalid_state_transition_display() {
        let err = WorkspaceError::InvalidStateTransition {
            from: WorkspaceState::Creating,
            to: WorkspaceState::Removed,
        };
        let msg = format!("{err}");
        assert!(msg.contains("Creating"));
        assert!(msg.contains("Removed"));
    }

    #[test]
    fn path_not_found_display() {
        let err = WorkspaceError::PathNotFound("/nonexistent".into());
        let msg = format!("{err}");
        assert!(msg.contains("/nonexistent"));
        assert!(msg.contains("does not exist"));
    }

    #[test]
    fn not_ready_display() {
        let err = WorkspaceError::NotReady(WorkspaceState::Creating);
        let msg = format!("{err}");
        assert!(msg.contains("not ready"));
    }

    #[test]
    fn not_active_display() {
        let err = WorkspaceError::NotActive(WorkspaceState::Cleaning);
        let msg = format!("{err}");
        assert!(msg.contains("not active"));
    }

    #[test]
    fn removed_display() {
        let err = WorkspaceError::Removed;
        let msg = format!("{err}");
        assert!(msg.contains("removed"));
    }

    #[test]
    fn cannot_use_display() {
        let err = WorkspaceError::CannotUse(WorkspaceState::Cleaning);
        let msg = format!("{err}");
        assert!(msg.contains("cannot use"));
    }

    #[test]
    fn all_variants_are_exhaustive() {
        let _ = WorkspaceError::InvalidStateTransition {
            from: WorkspaceState::Creating,
            to: WorkspaceState::Creating,
        };
        let _ = WorkspaceError::PathNotFound("/tmp".into());
        let _ = WorkspaceError::NotReady(WorkspaceState::Creating);
        let _ = WorkspaceError::NotActive(WorkspaceState::Creating);
        let _ = WorkspaceError::Removed;
        let _ = WorkspaceError::CannotUse(WorkspaceState::Creating);
    }
}
