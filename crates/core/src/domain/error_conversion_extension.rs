//! Extension traits for ergonomic error handling.
//!
//! This module provides extension traits that add convenient methods
//! for converting between error types with context.

use crate::domain::{
    aggregates::{bead::BeadError, session::SessionError, workspace::WorkspaceError},
    identifiers::IdentifierError,
    repository::RepositoryError,
};

use crate::domain::error_conversion_context::IntoRepositoryError;

// ============================================================================
// IDENTIFIER ERROR EXT
// ============================================================================

/// Extension trait for adding context to `IdentifierError`.
pub trait IdentifierErrorExt {
    /// Convert `IdentifierError` to `SessionError` with context.
    fn to_session_error(self) -> SessionError;

    /// Convert `IdentifierError` to `WorkspaceError` with context.
    fn to_workspace_error(self) -> WorkspaceError;

    /// Convert `IdentifierError` to `BeadError` with context.
    fn to_bead_error(self) -> BeadError;
}

impl IdentifierErrorExt for IdentifierError {
    fn to_session_error(self) -> SessionError {
        self.into()
    }

    fn to_workspace_error(self) -> WorkspaceError {
        self.into()
    }

    fn to_bead_error(self) -> BeadError {
        self.into()
    }
}

// ============================================================================
// AGGREGATE ERROR EXT
// ============================================================================

/// Extension trait for adding context to aggregate errors.
pub trait AggregateErrorExt {
    /// Convert to `RepositoryError` with entity and operation context.
    fn in_context(self, entity: &str, operation: &str) -> RepositoryError;

    /// Convert to `RepositoryError` for load operations.
    fn on_load(self, entity: &str) -> RepositoryError;

    /// Convert to `RepositoryError` for save operations.
    fn on_save(self, entity: &str) -> RepositoryError;

    /// Convert to `RepositoryError` for delete operations.
    fn on_delete(self, entity: &str) -> RepositoryError;
}

impl<E> AggregateErrorExt for E
where
    E: IntoRepositoryError,
{
    fn in_context(self, entity: &str, operation: &str) -> RepositoryError {
        self.into_repository_error(entity, operation)
    }

    fn on_load(self, entity: &str) -> RepositoryError {
        self.into_repository_error(entity, "load")
    }

    fn on_save(self, entity: &str) -> RepositoryError {
        self.into_repository_error(entity, "save")
    }

    fn on_delete(self, entity: &str) -> RepositoryError {
        self.into_repository_error(entity, "delete")
    }
}
