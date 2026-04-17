//! Repository error types and result alias.
//!
//! Provides a unified error taxonomy for all repository operations.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use thiserror::Error;

/// Common errors across all repository operations.
///
/// This error type covers expected failures in repository operations:
/// - **Not found**: Requested entity doesn't exist (informational, not exceptional)
/// - **Conflict**: Operation would violate constraints (duplicate IDs, etc.)
/// - **Invalid input**: Domain validation failed
/// - **Storage failure**: Underlying storage error (corruption, permissions, etc.)
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// Entity not found in repository
    #[error("entity not found: {0}")]
    NotFound(String),

    /// Conflict with existing data (duplicate, constraint violation)
    #[error("conflict: {0}")]
    Conflict(String),

    /// Invalid input for domain operation
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Underlying storage failure
    #[error("storage error: {0}")]
    StorageError(String),

    /// Operation not supported by repository
    #[error("operation not supported: {0}")]
    NotSupported(String),

    /// Concurrent modification conflict
    #[error("concurrent modification: {0}")]
    ConcurrentModification(String),
}

impl RepositoryError {
    /// Create a not found error
    #[must_use]
    pub fn not_found(entity: &str, id: impl std::fmt::Display) -> Self {
        Self::NotFound(format!("{entity} '{id}'"))
    }

    /// Create a conflict error
    #[must_use]
    pub fn conflict(reason: impl Into<String>) -> Self {
        Self::Conflict(reason.into())
    }

    /// Create an invalid input error
    #[must_use]
    pub fn invalid_input(reason: impl Into<String>) -> Self {
        Self::InvalidInput(reason.into())
    }

    /// Create a storage error
    #[must_use]
    pub fn storage_error(reason: impl Into<String>) -> Self {
        Self::StorageError(reason.into())
    }
}

/// Result type alias for repository operations
pub type RepositoryResult<T> = Result<T, RepositoryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_display() {
        let err = RepositoryError::NotFound("session 'abc'".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("session 'abc'"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn conflict_display() {
        let err = RepositoryError::Conflict("duplicate key".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("duplicate key"));
        assert!(msg.contains("conflict"));
    }

    #[test]
    fn invalid_input_display() {
        let err = RepositoryError::InvalidInput("empty name".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("empty name"));
        assert!(msg.contains("invalid input"));
    }

    #[test]
    fn storage_error_display() {
        let err = RepositoryError::StorageError("disk full".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("disk full"));
        assert!(msg.contains("storage error"));
    }

    #[test]
    fn not_supported_display() {
        let err = RepositoryError::NotSupported("batch delete".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("batch delete"));
        assert!(msg.contains("not supported"));
    }

    #[test]
    fn concurrent_modification_display() {
        let err = RepositoryError::ConcurrentModification("race condition".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("race condition"));
        assert!(msg.contains("concurrent"));
    }

    #[test]
    fn helper_constructors() {
        let err = RepositoryError::not_found("Session", "id-1");
        assert!(matches!(err, RepositoryError::NotFound(_)));
        assert!(err.to_string().contains("Session 'id-1'"));

        let err = RepositoryError::conflict("duplicate");
        assert!(matches!(err, RepositoryError::Conflict(_)));

        let err = RepositoryError::invalid_input("bad");
        assert!(matches!(err, RepositoryError::InvalidInput(_)));

        let err = RepositoryError::storage_error("fail");
        assert!(matches!(err, RepositoryError::StorageError(_)));
    }

    #[test]
    fn all_variants_are_exhaustive() {
        let _ = RepositoryError::NotFound(String::new());
        let _ = RepositoryError::Conflict(String::new());
        let _ = RepositoryError::InvalidInput(String::new());
        let _ = RepositoryError::StorageError(String::new());
        let _ = RepositoryError::NotSupported(String::new());
        let _ = RepositoryError::ConcurrentModification(String::new());
    }

    #[test]
    fn result_type_alias() {
        let ok: RepositoryResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: RepositoryResult<i32> = Err(RepositoryError::NotFound("x".into()));
        assert!(err.is_err());
    }
}
