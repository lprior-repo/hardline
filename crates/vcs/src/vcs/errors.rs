//! Error types for VCS operations
//!
//! This module provides:
//! - `VcsError` - Main error type for VCS operations
//! - `ParseError` - Errors when parsing `ChangeId` from string
//! - `ChangeError` - Errors when creating or manipulating `Change`

use std::path::PathBuf;

use thiserror::Error;

// ============================================================================
// VcsError
// ============================================================================

/// VCS-specific errors
#[derive(Debug, Error)]
pub enum VcsError {
    /// Path does not exist on filesystem
    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    /// Path exists but is not a directory
    #[error("Path is not a directory: {0}")]
    PathNotDirectory(PathBuf),

    /// No VCS backend detected (neither .git nor .jj found)
    #[error("No VCS backend found at path: {0}")]
    NoVcsFound(PathBuf),

    /// Invalid branch name (empty or contains illegal characters)
    #[error("Invalid branch name: {0}")]
    InvalidBranchName(String),

    /// Invalid commit ID (empty or malformed)
    #[error("Invalid commit ID: {0}")]
    InvalidCommitId(String),

    /// Requested backend type is not supported
    #[error("Backend type not supported: {0:?}")]
    BackendNotSupported(super::BackendType),

    /// VCS command execution failed
    #[error("VCS command failed: {message}")]
    CommandFailed {
        /// Error message
        message: String,
        /// Source error
        #[source]
        source: Option<std::io::Error>,
    },

    /// Repository is in an invalid state
    #[error("Repository is in invalid state: {0}")]
    InvalidState(String),

    /// Operation requires a clean working directory
    #[error("Working directory has uncommitted changes")]
    DirtyWorkingDirectory,

    /// Branch or commit not found
    #[error("{entity} not found: {id}")]
    NotFound {
        /// Entity type (e.g., "Branch", "Commit")
        entity: &'static str,
        /// Entity identifier
        id: String,
    },

    /// Failed to open Git repository
    #[error("Failed to open Git repository at {path}: {message}")]
    GitOpenFailed {
        /// Path to the repository
        path: PathBuf,
        /// Error message
        message: String,
        /// Source error from gix
        #[source]
        source: Option<gix::discover::Error>,
    },

    /// Repository is bare (no working tree) - stacking requires working tree
    #[error("Bare repository not supported: {0}")]
    BareRepositoryNotSupported(PathBuf),

    /// Git reference operation failed
    #[error("Git reference error: {0}")]
    GitReferenceError(String),

    /// Git CLI command failed (for rebase operations)
    #[error("Git CLI command failed: {command}")]
    GitCliFailed {
        /// The command that failed
        command: String,
        /// Source error
        #[source]
        source: Option<std::io::Error>,
    },

    /// Git CLI version too old
    #[error("Git CLI version too old: {found}, requires 2.38+")]
    GitCliVersionTooOld {
        /// The version that was found
        found: String,
    },

    /// Failed to parse Git CLI output
    #[error("Failed to parse Git output: {0}")]
    GitParseError(String),

    /// Failed to open JJ workspace
    #[error("Failed to open JJ workspace at {path}: {message}")]
    JjOpenFailed {
        /// Path to the workspace
        path: PathBuf,
        /// Error message
        message: String,
        /// Source error
        #[source]
        source: Option<anyhow::Error>,
    },

    /// Path is not a JJ workspace
    #[error("Not a JJ workspace: {0}")]
    NotAWorkspace(PathBuf),

    /// Invalid change ID format
    #[error("Invalid JJ change ID: {0}")]
    InvalidChangeId(String),

    /// Change not found in JJ workspace
    #[error("JJ change not found: {id}")]
    ChangeNotFound {
        /// Change ID
        id: String,
    },

    /// Ambiguous change ID
    #[error("Ambiguous JJ change ID: {id}")]
    AmbiguousChangeId {
        /// Change ID
        id: String,
    },

    /// Bookmark not found
    #[error("JJ bookmark not found: {name}")]
    BookmarkNotFound {
        /// Bookmark name
        name: String,
    },

    /// Bookmark already exists
    #[error("JJ bookmark already exists: {name}")]
    BookmarkAlreadyExists {
        /// Bookmark name
        name: String,
    },

    /// Failed to acquire workspace lock
    #[error("Failed to acquire JJ workspace lock: {0}")]
    LockAcquisitionFailed(String),

    /// Rebase operation failed
    #[error("JJ rebase operation failed: {message}")]
    RebaseFailed {
        /// Error message
        message: String,
        /// Source error
        #[source]
        source: Option<anyhow::Error>,
    },

    /// JJ internal error
    #[error("JJ internal error: {0}")]
    JjInternalError(#[source] anyhow::Error),
}

impl PartialEq for VcsError {
    #[allow(clippy::match_same_arms)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::PathNotFound(a), Self::PathNotFound(b))
            | (Self::PathNotDirectory(a), Self::PathNotDirectory(b))
            | (Self::NoVcsFound(a), Self::NoVcsFound(b))
            | (Self::BareRepositoryNotSupported(a), Self::BareRepositoryNotSupported(b)) => a == b,
            (Self::InvalidBranchName(a), Self::InvalidBranchName(b))
            | (Self::InvalidCommitId(a), Self::InvalidCommitId(b))
            | (Self::InvalidState(a), Self::InvalidState(b))
            | (Self::GitReferenceError(a), Self::GitReferenceError(b))
            | (Self::GitParseError(a), Self::GitParseError(b)) => a == b,
            (Self::BackendNotSupported(a), Self::BackendNotSupported(b)) => a == b,
            (Self::CommandFailed { message: a, .. }, Self::CommandFailed { message: b, .. }) => {
                a == b
            }
            (
                Self::GitOpenFailed {
                    path: p1,
                    message: m1,
                    ..
                },
                Self::GitOpenFailed {
                    path: p2,
                    message: m2,
                    ..
                },
            ) => p1 == p2 && m1 == m2,
            (Self::GitCliFailed { command: a, .. }, Self::GitCliFailed { command: b, .. }) => {
                a == b
            }
            (Self::GitCliVersionTooOld { found: a }, Self::GitCliVersionTooOld { found: b }) => {
                a == b
            }
            (Self::DirtyWorkingDirectory, Self::DirtyWorkingDirectory) => true,
            (Self::NotFound { entity: a1, id: a2 }, Self::NotFound { entity: b1, id: b2 }) => {
                a1 == b1 && a2 == b2
            }
            (Self::NotAWorkspace(a), Self::NotAWorkspace(b)) => a == b,
            (Self::InvalidChangeId(a), Self::InvalidChangeId(b))
            | (Self::LockAcquisitionFailed(a), Self::LockAcquisitionFailed(b)) => a == b,
            (
                Self::JjOpenFailed {
                    path: p1,
                    message: m1,
                    ..
                },
                Self::JjOpenFailed {
                    path: p2,
                    message: m2,
                    ..
                },
            ) => p1 == p2 && m1 == m2,
            (Self::ChangeNotFound { id: a }, Self::ChangeNotFound { id: b }) => a == b,
            (Self::AmbiguousChangeId { id: a }, Self::AmbiguousChangeId { id: b }) => a == b,
            (Self::BookmarkNotFound { name: a }, Self::BookmarkNotFound { name: b }) => a == b,
            (Self::BookmarkAlreadyExists { name: a }, Self::BookmarkAlreadyExists { name: b }) => {
                a == b
            }
            (Self::RebaseFailed { message: a, .. }, Self::RebaseFailed { message: b, .. }) => {
                a == b
            }
            (Self::JjInternalError(a), Self::JjInternalError(b)) => a.to_string() == b.to_string(),
            _ => false,
        }
    }
}

// ============================================================================
// ParseError
// ============================================================================

/// Errors when parsing `ChangeId` from string
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Input string is empty or whitespace-only
    #[error("ChangeId cannot be empty")]
    Empty,

    /// Input contains invalid characters for the format
    #[error("Invalid characters in ChangeId: {0}")]
    InvalidCharacters(String),

    /// Git SHA has invalid length (expected 7-40 characters)
    #[error("Invalid Git SHA length: {0} characters")]
    InvalidGitShaLength(usize),

    /// JJ change ID has invalid length (expected >= 1)
    #[error("Invalid JJ change ID length: {0} characters")]
    InvalidJjLength(usize),
}

// ============================================================================
// ChangeError
// ============================================================================

/// Errors when creating or manipulating Change
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ChangeError {
    /// Message cannot be empty
    #[error("Change message cannot be empty")]
    EmptyMessage,

    /// Author cannot be empty
    #[error("Change author cannot be empty")]
    EmptyAuthor,
}
