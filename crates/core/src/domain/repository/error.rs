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
