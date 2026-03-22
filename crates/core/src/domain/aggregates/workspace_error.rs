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
