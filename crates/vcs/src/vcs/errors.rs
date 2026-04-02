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

    /// No VCS backend detected (.git not found)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vcs::types::BackendType;

    #[test]
    fn path_not_found_display() {
        let err = VcsError::PathNotFound("/missing".into());
        let msg = format!("{err}");
        assert!(msg.contains("/missing"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn path_not_directory_display() {
        let err = VcsError::PathNotDirectory("/file.txt".into());
        let msg = format!("{err}");
        assert!(msg.contains("/file.txt"));
        assert!(msg.contains("not a directory"));
    }

    #[test]
    fn no_vcs_found_display() {
        let err = VcsError::NoVcsFound("/repo".into());
        let msg = format!("{err}");
        assert!(msg.contains("/repo"));
        assert!(msg.contains("No VCS backend"));
    }

    #[test]
    fn invalid_branch_name_display() {
        let err = VcsError::InvalidBranchName("bad name!".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bad name!"));
        assert!(msg.contains("Invalid branch name"));
    }

    #[test]
    fn invalid_commit_id_display() {
        let err = VcsError::InvalidCommitId("".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Invalid commit ID"));
    }

    #[test]
    fn command_failed_display() {
        let err = VcsError::CommandFailed {
            message: "segfault".to_string(),
            source: None,
        };
        let msg = format!("{err}");
        assert!(msg.contains("segfault"));
        assert!(msg.contains("command failed"));
    }

    #[test]
    fn invalid_state_display() {
        let err = VcsError::InvalidState("corrupt repo".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("corrupt repo"));
        assert!(msg.contains("invalid state"));
    }

    #[test]
    fn dirty_working_directory_display() {
        let err = VcsError::DirtyWorkingDirectory;
        let msg = format!("{err}");
        assert!(msg.contains("uncommitted changes"));
    }

    #[test]
    fn not_found_display() {
        let err = VcsError::NotFound { entity: "Branch", id: "missing".to_string() };
        let msg = format!("{err}");
        assert!(msg.contains("Branch"));
        assert!(msg.contains("missing"));
    }

    #[test]
    fn git_open_failed_display() {
        let err = VcsError::GitOpenFailed {
            path: "/repo".into(),
            message: "not a repo".to_string(),
            source: None,
        };
        let msg = format!("{err}");
        assert!(msg.contains("/repo"));
        assert!(msg.contains("not a repo"));
    }

    #[test]
    fn bare_repository_not_supported_display() {
        let err = VcsError::BareRepositoryNotSupported("/bare.git".into());
        let msg = format!("{err}");
        assert!(msg.contains("/bare.git"));
        assert!(msg.contains("Bare"));
    }

    #[test]
    fn git_reference_error_display() {
        let err = VcsError::GitReferenceError("bad ref".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bad ref"));
    }

    #[test]
    fn git_cli_failed_display() {
        let err = VcsError::GitCliFailed { command: "rebase".to_string(), source: None };
        let msg = format!("{err}");
        assert!(msg.contains("rebase"));
        assert!(msg.contains("Git CLI"));
    }

    #[test]
    fn git_cli_version_too_old_display() {
        let err = VcsError::GitCliVersionTooOld { found: "2.30".to_string() };
        let msg = format!("{err}");
        assert!(msg.contains("2.30"));
        assert!(msg.contains("too old"));
    }

    #[test]
    fn git_parse_error_display() {
        let err = VcsError::GitParseError("bad json".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bad json"));
        assert!(msg.contains("parse"));
    }

    // -- ParseError Display tests --

    #[test]
    fn parse_error_empty_display() {
        let err = ParseError::Empty;
        let msg = format!("{err}");
        assert!(msg.contains("cannot be empty"));
    }

    #[test]
    fn parse_error_invalid_characters_display() {
        let err = ParseError::InvalidCharacters("@#$".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("@#$"));
        assert!(msg.contains("Invalid characters"));
    }

    // -- ChangeError Display tests --

    #[test]
    fn change_error_empty_message_display() {
        let err = ChangeError::EmptyMessage;
        let msg = format!("{err}");
        assert!(msg.contains("message cannot be empty"));
    }

    #[test]
    fn change_error_empty_author_display() {
        let err = ChangeError::EmptyAuthor;
        let msg = format!("{err}");
        assert!(msg.contains("author cannot be empty"));
    }

    // -- VcsError PartialEq tests --

    #[test]
    fn vcs_error_eq_path_not_found() {
        let a = VcsError::PathNotFound("/a".into());
        let b = VcsError::PathNotFound("/a".into());
        let c = VcsError::PathNotFound("/b".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn vcs_error_eq_path_not_directory() {
        let a = VcsError::PathNotDirectory("/file".into());
        let b = VcsError::PathNotDirectory("/file".into());
        assert_eq!(a, b);
    }

    #[test]
    fn vcs_error_eq_no_vcs_found() {
        let a = VcsError::NoVcsFound("/repo".into());
        let b = VcsError::NoVcsFound("/repo".into());
        assert_eq!(a, b);
    }

    #[test]
    fn vcs_error_eq_invalid_branch_name() {
        let a = VcsError::InvalidBranchName("bad".to_string());
        let b = VcsError::InvalidBranchName("bad".to_string());
        let c = VcsError::InvalidBranchName("other".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn vcs_error_eq_backend_not_supported() {
        let a = VcsError::BackendNotSupported(BackendType::Git);
        let b = VcsError::BackendNotSupported(BackendType::Git);
        assert_eq!(a, b);
    }

    #[test]
    fn vcs_error_eq_command_failed() {
        let a = VcsError::CommandFailed { message: "err".to_string(), source: None };
        let b = VcsError::CommandFailed { message: "err".to_string(), source: None };
        assert_eq!(a, b);
    }

    #[test]
    fn vcs_error_eq_dirty_working_directory() {
        assert_eq!(VcsError::DirtyWorkingDirectory, VcsError::DirtyWorkingDirectory);
    }

    #[test]
    fn vcs_error_eq_not_found() {
        let a = VcsError::NotFound { entity: "Branch", id: "x".to_string() };
        let b = VcsError::NotFound { entity: "Branch", id: "x".to_string() };
        assert_eq!(a, b);
    }

    #[test]
    fn vcs_error_neq_different_variants() {
        let variants = [
            VcsError::PathNotFound("/a".into()),
            VcsError::PathNotDirectory("/a".into()),
            VcsError::NoVcsFound("/a".into()),
            VcsError::InvalidBranchName("a".to_string()),
            VcsError::InvalidCommitId("a".to_string()),
            VcsError::DirtyWorkingDirectory,
        ];
        // All variants should be pairwise unequal
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j], "Variants {i} and {j} should not be equal");
            }
        }
    }

    // -- ChangeError PartialEq tests --

    #[test]
    fn change_error_eq() {
        assert_eq!(ChangeError::EmptyMessage, ChangeError::EmptyMessage);
        assert_eq!(ChangeError::EmptyAuthor, ChangeError::EmptyAuthor);
        assert_ne!(ChangeError::EmptyMessage, ChangeError::EmptyAuthor);
    }

    #[test]
    fn change_error_clone() {
        let a = ChangeError::EmptyMessage.clone();
        assert_eq!(a, ChangeError::EmptyMessage);
        let b = ChangeError::EmptyAuthor.clone();
        assert_eq!(b, ChangeError::EmptyAuthor);
    }

}
