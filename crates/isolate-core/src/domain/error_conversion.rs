//! Error conversion traits and implementations for the domain layer.
//!
//! This module provides comprehensive error conversion between domain error types,
//! improving ergonomics while maintaining error context. Following DDD principles,
//! errors are categorized and converted with clear preservation of information.
//!
//! # Error Conversion Hierarchy
//!
//! 1. `IdentifierError` → Aggregate errors (`SessionError`, `BeadError`, etc.)
//! 2. Aggregate errors → `RepositoryError`
//! 3. Domain errors → `anyhow::Error` (for shell/imperative layer)
//! 4. `BuilderError` → Aggregate errors
//!
//! # Design Principles
//!
//! - **Context preservation**: Error conversions retain original information
//! - **Ergonomic conversions**: From impls where lossless conversion is possible
//! - **Explicit conversions**: TryFrom/TryInto where validation is needed
//! - **Clear error messages**: Converted errors explain what went wrong
//!
//! # Example
//!
//! ```rust,ignore
//! use isolate_core::domain::{SessionName, identifiers::IdentifierError};
//! use isolate_core::domain::aggregates::session::SessionError;
//!
//! // IdentifierError converts to aggregate-specific errors
//! let name_result: Result<SessionName, IdentifierError> = SessionName::parse("invalid!");
//! // Can now use `?` to convert to SessionError
//! let session = Session::new_root(id, name?, branch, path)?;
//! ```

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::domain::{
    aggregates::{bead::BeadError, session::SessionError, workspace::WorkspaceError},
    builders::BuilderError,
    identifiers::IdentifierError,
    repository::RepositoryError,
};

// ============================================================================
// IDENTIFIER ERROR CONVERSIONS
// ============================================================================

impl From<IdentifierError> for SessionError {
    fn from(_err: IdentifierError) -> Self {
        Self::CannotActivate
    }
}

impl From<IdentifierError> for WorkspaceError {
    fn from(_err: IdentifierError) -> Self {
        Self::CannotUse(crate::domain::workspace::WorkspaceState::Creating)
    }
}

impl From<IdentifierError> for BeadError {
    fn from(err: IdentifierError) -> Self {
        match err {
            IdentifierError::Empty => Self::TitleRequired,
            _ => Self::InvalidTitle(err.to_string()),
        }
    }
}

// ============================================================================
// AGGREGATE ERROR TO REPOSITORY ERROR CONVERSIONS
// ============================================================================

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

// ============================================================================
// EXTENSION TRAIT IMPLEMENTATIONS
// ============================================================================
// BUILDER ERROR CONVERSIONS
// ============================================================================

impl From<BuilderError> for SessionError {
    fn from(err: BuilderError) -> Self {
        match &err {
            BuilderError::MissingRequired { field: _ }
            | BuilderError::InvalidValue {
                field: _,
                reason: _,
            }
            | BuilderError::Overflow {
                field: _,
                capacity: _,
            }
            | BuilderError::InvalidTransition {
                from: _,
                to: _,
                reason: _,
            } => Self::CannotActivate,
        }
    }
}

impl From<BuilderError> for WorkspaceError {
    fn from(err: BuilderError) -> Self {
        match &err {
            BuilderError::MissingRequired { .. }
            | BuilderError::InvalidValue { .. }
            | BuilderError::Overflow { .. }
            | BuilderError::InvalidTransition { .. } => {
                Self::CannotUse(crate::domain::workspace::WorkspaceState::Creating)
            }
        }
    }
}

impl From<BuilderError> for BeadError {
    fn from(err: BuilderError) -> Self {
        match &err {
            BuilderError::MissingRequired { field } => match *field {
                "title" => Self::TitleRequired,
                _ => Self::InvalidTitle(format!("missing required field: {field}")),
            },
            BuilderError::InvalidValue { field, reason } => {
                if *field == "title" {
                    Self::InvalidTitle(reason.clone())
                } else {
                    Self::InvalidTitle(format!("invalid {field}: {reason}"))
                }
            }
            BuilderError::Overflow { field, capacity } => {
                Self::InvalidTitle(format!("field {field} exceeds capacity {capacity}"))
            }
            BuilderError::InvalidTransition { from, to, reason } => {
                Self::InvalidTitle(format!("invalid transition from {from} to {to}: {reason}"))
            }
        }
    }
}

impl From<BuilderError> for RepositoryError {
    fn from(err: BuilderError) -> Self {
        match &err {
            BuilderError::MissingRequired { field } => {
                Self::InvalidInput(format!("missing required field: {field}"))
            }
            BuilderError::InvalidValue { field, reason } => {
                Self::InvalidInput(format!("invalid value for {field}: {reason}"))
            }
            BuilderError::Overflow { field, capacity } => {
                Self::InvalidInput(format!("field {field} exceeds capacity of {capacity}"))
            }
            BuilderError::InvalidTransition { from, to, reason } => {
                Self::InvalidInput(format!("invalid transition from {from} to {to}: {reason}",))
            }
        }
    }
}

// ============================================================================
// CONTEXT-PRESERVING CONVERSION TRAITS
// ============================================================================

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

// ============================================================================
// EXTENSION TRAITS FOR ERGONOMIC ERROR HANDLING
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

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::domain::{
        identifiers::{SessionName, WorkspaceName},
        workspace::WorkspaceState,
    };

    // ===== IdentifierError to AggregateError conversions =====

    #[test]
    fn test_identifier_error_to_session_error() {
        let err = IdentifierError::Empty;
        let session_err: SessionError = err.into();
        assert!(matches!(session_err, SessionError::CannotActivate));

        let err = IdentifierError::TooLong {
            max: 63,
            actual: 100,
        };
        let session_err: SessionError = err.into();
        assert!(matches!(session_err, SessionError::CannotActivate));
    }

    #[test]
    fn test_identifier_error_to_workspace_error() {
        let err = IdentifierError::ContainsPathSeparators;
        let workspace_err: WorkspaceError = err.into();
        assert!(matches!(
            workspace_err,
            WorkspaceError::CannotUse(WorkspaceState::Creating)
        ));
    }

    #[test]
    fn test_identifier_error_to_bead_error() {
        let err = IdentifierError::Empty;
        let bead_err: BeadError = err.into();
        assert!(matches!(bead_err, BeadError::TitleRequired));

        let err = IdentifierError::InvalidFormat {
            details: "test".to_string(),
        };
        let bead_err: BeadError = err.into();
        assert!(matches!(bead_err, BeadError::InvalidTitle(_)));
    }

    #[test]
    fn test_identifier_error_ext() {
        let err = IdentifierError::Empty;
        let session_err = err.to_session_error();
        assert!(matches!(session_err, SessionError::CannotActivate));

        let err = IdentifierError::ContainsPathSeparators;
        let workspace_err = err.to_workspace_error();
        assert!(matches!(workspace_err, WorkspaceError::CannotUse(_)));
    }

    // ===== AggregateError to RepositoryError conversions =====

    #[test]
    fn test_session_error_to_repository_error() {
        let err = SessionError::NameAlreadyExists(SessionName::parse("test").expect("valid name"));
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::Conflict(_)));

        let err = SessionError::WorkspaceNotFound(PathBuf::from("/test"));
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::NotFound(_)));
    }

    #[test]
    fn test_workspace_error_to_repository_error() {
        let err = WorkspaceError::PathNotFound(PathBuf::from("/test"));
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::NotFound(_)));

        let err = WorkspaceError::Removed;
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::NotFound(_)));

        let err =
            WorkspaceError::NameAlreadyExists(WorkspaceName::parse("test").expect("valid name"));
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::Conflict(_)));
    }

    #[test]
    fn test_bead_error_to_repository_error() {
        let err = BeadError::CannotModifyClosed;
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::InvalidInput(_)));

        let err = BeadError::TitleRequired;
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::InvalidInput(_)));
    }

    // ===== Context-preserving conversions =====

    #[test]
    fn test_into_repository_error_with_context() {
        let err = SessionError::NameAlreadyExists(SessionName::parse("test").expect("valid name"));
        let repo_err = err.in_context("session", "create");
        assert!(matches!(repo_err, RepositoryError::Conflict(_)));
        assert!(repo_err.to_string().contains("session"));

        let err = WorkspaceError::PathNotFound(PathBuf::from("/test"));
        let repo_err = err.on_load("workspace");
        assert!(matches!(repo_err, RepositoryError::NotFound(_)));
        assert!(repo_err.to_string().contains("load"));

        let err = BeadError::InvalidTitle("test".to_string());
        let repo_err = err.on_save("bead");
        assert!(matches!(repo_err, RepositoryError::InvalidInput(_)));
        assert!(repo_err.to_string().contains("save"));
    }

    // ===== BuilderError conversions =====

    #[test]
    fn test_builder_error_to_session_error() {
        let err = BuilderError::MissingRequired { field: "name" };
        let session_err: SessionError = err.into();
        assert!(matches!(session_err, SessionError::CannotActivate));
    }

    #[test]
    fn test_builder_error_to_bead_error() {
        let err = BuilderError::MissingRequired { field: "title" };
        let bead_err: BeadError = err.into();
        assert!(matches!(bead_err, BeadError::TitleRequired));

        let err = BuilderError::InvalidValue {
            field: "title",
            reason: "too long".to_string(),
        };
        let bead_err: BeadError = err.into();
        assert!(matches!(bead_err, BeadError::InvalidTitle(_)));
    }

    #[test]
    fn test_builder_error_to_repository_error() {
        let err = BuilderError::MissingRequired { field: "id" };
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::InvalidInput(_)));

        let err = BuilderError::InvalidValue {
            field: "name",
            reason: "empty".to_string(),
        };
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::InvalidInput(_)));
    }
}
