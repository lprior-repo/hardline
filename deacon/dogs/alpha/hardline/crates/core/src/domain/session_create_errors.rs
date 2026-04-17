//! Session creation errors - Error domain
//!
//! Defines all errors that can occur during session creation.
//!
//! # Error Taxonomy
//!
//! - `Error::ValidationError` for name/workspace validation
//! - `Error::SessionAlreadyExists` for duplicate names
//! - `Error::MaxSessionsExceeded` for limit reached

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::domain::{
    identifiers::SessionName,
    repository::{RepositoryError, SessionRepository},
};

/// Errors that can occur during session creation
///
/// Follows the error taxonomy from the contract:
/// - `Error::ValidationError` for name/workspace validation
/// - `Error::SessionAlreadyExists` for duplicate names
/// - `Error::MaxSessionsExceeded` for limit reached
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCreateError {
    /// Workspace path does not exist (P5)
    ///
    /// The provided workspace path must exist on the filesystem.
    /// This is a runtime validation because it requires I/O.
    WorkspaceNotFound {
        /// The path that was provided
        path: PathBuf,
    },

    /// Session name already exists (P6)
    ///
    /// Each session must have a unique name within the system.
    /// This requires checking the repository for existing sessions.
    SessionAlreadyExists {
        /// The name that already exists
        name: SessionName,
    },

    /// Maximum session limit exceeded (P7)
    ///
    /// The system has reached its maximum capacity for sessions.
    MaxSessionsExceeded {
        /// The maximum number of sessions allowed
        max: usize,
        /// The current number of sessions
        current: usize,
    },

    /// Repository operation failed
    ///
    /// Underlying repository error (connection, corruption, etc.)
    RepositoryError(String),
}

impl std::fmt::Display for SessionCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceNotFound { path } => {
                write!(f, "workspace path does not exist: {}", path.display())
            }
            Self::SessionAlreadyExists { name } => {
                write!(f, "session name already exists: {}", name.as_str())
            }
            Self::MaxSessionsExceeded { max, current } => {
                write!(f, "max sessions exceeded: {current} of {max}")
            }
            Self::RepositoryError(msg) => {
                write!(f, "repository error: {msg}")
            }
        }
    }
}

impl std::error::Error for SessionCreateError {}

impl From<RepositoryError> for SessionCreateError {
    fn from(err: RepositoryError) -> Self {
        Self::RepositoryError(err.to_string())
    }
}
