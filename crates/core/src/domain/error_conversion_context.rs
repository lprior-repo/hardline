//! Context-preserving conversion trait implementations.
//!
//! This module provides the `IntoRepositoryError` trait for converting
//! errors with additional entity and operation context.

use crate::domain::{
    aggregates::{bead::BeadError, session::SessionError, workspace::WorkspaceError},
    repository::RepositoryError,
};

/// Trait for converting errors with additional context.
///
/// This trait provides context-preserving error conversion,
/// allowing errors to be enriched with additional information
/// while preserving the original error details.
pub trait IntoRepositoryError {
    /// Convert the error into a `RepositoryError` with context.
    ///
    /// # Parameters
    ///
    /// - `entity`: The type of entity being operated on (e.g., "session", "workspace")
    /// - `operation`: The operation being performed (e.g., "load", "save", "delete")
    fn into_repository_error(self, entity: &str, operation: &str) -> RepositoryError;
}

impl IntoRepositoryError for SessionError {
    fn into_repository_error(self, entity: &str, operation: &str) -> RepositoryError {
        match self {
            Self::NameAlreadyExists(name) => RepositoryError::Conflict(format!(
                "{entity} '{name}' already exists during {operation}",
            )),
            Self::WorkspaceNotFound(path) => RepositoryError::NotFound(format!(
                "workspace not found at {} during {operation} of {entity}",
                path.display(),
            )),
            other => {
                RepositoryError::InvalidInput(format!("failed to {operation} {entity}: {other}",))
            }
        }
    }
}

impl IntoRepositoryError for WorkspaceError {
    fn into_repository_error(self, entity: &str, operation: &str) -> RepositoryError {
        match self {
            Self::NameAlreadyExists(name) => RepositoryError::Conflict(format!(
                "{entity} '{name}' already exists during {operation}",
            )),
            Self::PathNotFound(path) => RepositoryError::NotFound(format!(
                "path not found at {} during {operation} of {entity}",
                path.display(),
            )),
            Self::Removed => {
                RepositoryError::NotFound(format!("{entity} has been removed during {operation}",))
            }
            other => {
                RepositoryError::InvalidInput(format!("failed to {operation} {entity}: {other}",))
            }
        }
    }
}

impl IntoRepositoryError for BeadError {
    fn into_repository_error(self, entity: &str, operation: &str) -> RepositoryError {
        RepositoryError::InvalidInput(format!("failed to {operation} {entity}: {self}"))
    }
}
