//! Aggregate to repository error conversions.
//!
//! This module provides `From<AggregateError> for RepositoryError` implementations
//! for converting aggregate-specific errors to repository-level errors.

use crate::domain::{
    aggregates::{bead::BeadError, session::SessionError, workspace::WorkspaceError},
    repository::RepositoryError,
};

impl From<SessionError> for RepositoryError {
    fn from(err: SessionError) -> Self {
        match &err {
            SessionError::InvalidBranchTransition { from, to } => {
                Self::InvalidInput(format!("invalid branch transition: {from:?} -> {to:?}"))
            }
            SessionError::WorkspaceNotFound(path) => {
                Self::NotFound(format!("workspace not found: {}", path.display()))
            }
            SessionError::NotActive => Self::InvalidInput("session is not active".into()),
            SessionError::CannotActivate => Self::InvalidInput("cannot activate session".into()),
            SessionError::NameAlreadyExists(name) => {
                Self::Conflict(format!("session name already exists: {name}"))
            }
        }
    }
}

impl From<WorkspaceError> for RepositoryError {
    fn from(err: WorkspaceError) -> Self {
        match &err {
            WorkspaceError::InvalidStateTransition { from, to } => {
                Self::InvalidInput(format!("invalid state transition: {from:?} -> {to:?}"))
            }
            WorkspaceError::PathNotFound(path) => {
                Self::NotFound(format!("path not found: {}", path.display()))
            }
            WorkspaceError::NotReady(state) => {
                Self::InvalidInput(format!("workspace is not ready: {state:?}"))
            }
            WorkspaceError::NotActive(state) => {
                Self::InvalidInput(format!("workspace is not active: {state:?}"))
            }
            WorkspaceError::Removed => Self::NotFound("workspace has been removed".into()),
            WorkspaceError::CannotUse(state) => {
                Self::InvalidInput(format!("cannot use workspace in state: {state:?}"))
            }
            WorkspaceError::NameAlreadyExists(name) => {
                Self::Conflict(format!("workspace name already exists: {name}"))
            }
        }
    }
}

impl From<BeadError> for RepositoryError {
    fn from(err: BeadError) -> Self {
        match &err {
            BeadError::InvalidTitle(msg) => {
                Self::InvalidInput(format!("invalid bead title: {msg}"))
            }
            BeadError::InvalidDescription(msg) => {
                Self::InvalidInput(format!("invalid bead description: {msg}"))
            }
            BeadError::InvalidStateTransition { from, to } => {
                Self::InvalidInput(format!("invalid state transition: {from:?} -> {to:?}"))
            }
            BeadError::CannotModifyClosed => {
                Self::InvalidInput("cannot modify closed bead".into())
            }
            BeadError::NonMonotonicTimestamps { created_at, updated_at } => {
                Self::InvalidInput(format!(
                    "timestamps must be monotonic: updated_at ({updated_at:?}) < created_at ({created_at:?})"
                ))
            }
            BeadError::TitleRequired => {
                Self::InvalidInput("bead title is required".into())
            }
            BeadError::Domain(domain_err) => {
                Self::InvalidInput(format!("domain error: {domain_err}"))
            }
        }
    }
}
