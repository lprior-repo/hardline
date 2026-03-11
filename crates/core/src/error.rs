//! Unified error types for Source Control Plane.
//!
//! All errors return Result<T, Error> - zero panic, zero unwrap.

use serde::Serialize;
use thiserror::Error;

/// Unified error type for SCP (Source Control Plane).
///
/// Error codes:
/// - 1xxx: Workspace/Session errors
/// - 2xxx: Queue errors  
/// - 3xxx: VCS errors
/// - 4xxx: Configuration errors
/// - 5xxx: Agent errors
/// - 9xxx: Internal errors
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    // ========================================================================
    // Workspace/Session Errors (1xxx)
    // ========================================================================
    /// Workspace not found
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    /// Workspace already exists
    #[error("Workspace already exists: {0}")]
    WorkspaceExists(String),

    /// Workspace is locked by another process
    #[error("Workspace '{0}' is locked by '{1}'")]
    WorkspaceLocked(String, String),

    /// Workspace conflict during operation
    #[error("Workspace conflict: {0}")]
    WorkspaceConflict(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Session already exists
    #[error("Session already exists: {0}")]
    SessionExists(String),

    /// Session is locked
    #[error("Session '{0}' is locked by '{1}'")]
    SessionLocked(String, String),

    /// Not the lock holder
    #[error("Agent '{1}' does not hold lock on session '{0}'")]
    NotLockHolder(String, String),

    /// Session in invalid state for operation
    #[error("Session '{0}' is {1}, expected {2}")]
    SessionInvalidState(String, String, String),

    // ========================================================================
    // Queue Errors (2xxx)
    // ========================================================================
    /// Queue is empty
    #[error("Queue is empty")]
    QueueEmpty,

    /// Queue item not found
    #[error("Queue item not found: {0}")]
    QueueItemNotFound(String),

    /// Queue is locked
    #[error("Queue is locked by '{0}'")]
    QueueLocked(String),

    /// Queue operation already in progress
    #[error("Queue operation already in progress")]
    QueueProcessing,

    /// Invalid queue position
    #[error("Invalid queue position: {0}")]
    QueueInvalidPosition(usize),

    /// Queue full (if there's a max size)
    #[error("Queue is full (max: {0})")]
    QueueFull(usize),

    // ========================================================================
    // VCS Errors (3xxx)
    // ========================================================================
    /// VCS not initialized
    #[error("VCS not initialized in this directory")]
    VcsNotInitialized,

    /// VCS conflict detected
    #[error("VCS conflict in {0}: {1}")]
    VcsConflict(String, String),

    /// Push failed
    #[error("Failed to push: {0}")]
    VcsPushFailed(String),

    /// Pull failed
    #[error("Failed to pull: {0}")]
    VcsPullFailed(String),

    /// Rebase failed
    #[error("Failed to rebase: {0}")]
    VcsRebaseFailed(String),

    /// Branch not found
    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    /// Branch already exists
    #[error("Branch already exists: {0}")]
    BranchExists(String),

    /// Commit not found
    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    /// Working copy is dirty
    #[error("Working copy has uncommitted changes")]
    WorkingCopyDirty,

    // ========================================================================
    // Configuration Errors (4xxx)
    // ========================================================================
    /// Configuration not found
    #[error("Configuration not found: {0}")]
    ConfigNotFound(String),

    /// Configuration is invalid
    #[error("Configuration invalid: {0}")]
    ConfigInvalid(String),

    /// Configuration permission denied
    #[error("Configuration permission denied: {0}")]
    ConfigPermission(String),

    // ========================================================================
    // Agent Errors (5xxx)
    // ========================================================================
    /// Agent not found
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Agent already registered
    #[error("Agent already registered: {0}")]
    AgentExists(String),

    /// Agent heartbeat timeout
    #[error("Agent '{0}' heartbeat timeout")]
    AgentTimeout(String),

    // ========================================================================
    // State/Conflict Errors (7xxx)
    // ========================================================================
    /// Invalid state for operation
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    // ========================================================================
    // Validation Errors (8xxx)
    // ========================================================================
    /// Validation failed
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Validation failed with field context
    #[error("Validation error on '{field}': {message}")]
    ValidationFieldError {
        /// Human-readable error message
        message: String,
        /// Field name that failed validation
        field: String,
        /// Invalid value provided
        value: Option<String>,
    },

    /// Invalid identifier format
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),

    /// IO error with context
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// IO error with custom message
    #[error("IO error: {0}")]
    IoError(String),

    /// JSON parse error
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// YAML parse error
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    // ========================================================================
    // Internal Errors (9xxx)
    // ========================================================================
    /// Internal invariant violation
    #[error("Internal error: {0}")]
    Internal(String),

    /// Unimplemented feature
    #[error("Not implemented: {0}")]
    Unimplemented(String),

    // ========================================================================
    // JJ-specific Errors (3xxx - VCS)
    // ========================================================================
    /// JJ command execution failed
    #[error("JJ command '{operation}' failed: {msg}")]
    JjCommandError {
        /// Operation that was attempted
        operation: String,
        /// Error message from JJ
        msg: String,
        /// Whether JJ binary was not found
        is_not_found: bool,
    },

    /// JJ workspace conflict detected
    #[error("JJ workspace conflict: {conflict_type:?} for '{workspace_name}': {msg}")]
    JjWorkspaceConflict {
        /// Type of conflict detected
        conflict_type: JjConflictType,
        /// Workspace name
        workspace_name: String,
        /// Raw error output
        msg: String,
        /// Recovery hint
        recovery_hint: String,
    },

    /// Lock acquisition timeout
    #[error("Lock acquisition timeout for '{operation}' after {timeout_ms}ms ({retries} retries)")]
    LockTimeout {
        /// Operation that was being locked
        operation: String,
        /// Timeout in milliseconds
        timeout_ms: u64,
        /// Number of retry attempts
        retries: usize,
    },

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Clone operation failed
    #[error("Clone failed: {0}")]
    CloneFailed(String),

    /// Record operation failed
    #[error("Record failed: {0}")]
    RecordFailed(String),

    /// Invalid repository URL
    #[error("Invalid repository URL: {0}")]
    InvalidRepoUrl(String),
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::WorkspaceNotFound(s) => Error::WorkspaceNotFound(s.clone()),
            Error::WorkspaceExists(s) => Error::WorkspaceExists(s.clone()),
            Error::WorkspaceLocked(s, s2) => Error::WorkspaceLocked(s.clone(), s2.clone()),
            Error::WorkspaceConflict(s) => Error::WorkspaceConflict(s.clone()),
            Error::SessionNotFound(s) => Error::SessionNotFound(s.clone()),
            Error::SessionExists(s) => Error::SessionExists(s.clone()),
            Error::SessionLocked(s, s2) => Error::SessionLocked(s.clone(), s2.clone()),
            Error::NotLockHolder(s, s2) => Error::NotLockHolder(s.clone(), s2.clone()),
            Error::SessionInvalidState(s, s2, s3) => {
                Error::SessionInvalidState(s.clone(), s2.clone(), s3.clone())
            }
            Error::QueueEmpty => Error::QueueEmpty,
            Error::QueueItemNotFound(s) => Error::QueueItemNotFound(s.clone()),
            Error::QueueLocked(s) => Error::QueueLocked(s.clone()),
            Error::QueueProcessing => Error::QueueProcessing,
            Error::QueueInvalidPosition(s) => Error::QueueInvalidPosition(s.clone()),
            Error::QueueFull(n) => Error::QueueFull(*n),
            Error::VcsNotInitialized => Error::VcsNotInitialized,
            Error::VcsConflict(s, s2) => Error::VcsConflict(s.clone(), s2.clone()),
            Error::VcsPushFailed(s) => Error::VcsPushFailed(s.clone()),
            Error::VcsPullFailed(s) => Error::VcsPullFailed(s.clone()),
            Error::VcsRebaseFailed(s) => Error::VcsRebaseFailed(s.clone()),
            Error::BranchNotFound(s) => Error::BranchNotFound(s.clone()),
            Error::BranchExists(s) => Error::BranchExists(s.clone()),
            Error::CommitNotFound(s) => Error::CommitNotFound(s.clone()),
            Error::WorkingCopyDirty => Error::WorkingCopyDirty,
            Error::ConfigNotFound(s) => Error::ConfigNotFound(s.clone()),
            Error::ConfigInvalid(s) => Error::ConfigInvalid(s.clone()),
            Error::ConfigPermission(s) => Error::ConfigPermission(s.clone()),
            Error::AgentNotFound(s) => Error::AgentNotFound(s.clone()),
            Error::AgentExists(s) => Error::AgentExists(s.clone()),
            Error::AgentTimeout(s) => Error::AgentTimeout(s.clone()),
            Error::InvalidState(s) => Error::InvalidState(s.clone()),
            Error::NotFound(s) => Error::NotFound(s.clone()),
            Error::ValidationError(s) => Error::ValidationError(s.clone()),
            Error::ValidationFieldError {
                field,
                message,
                value,
            } => Error::ValidationFieldError {
                field: field.clone(),
                message: message.clone(),
                value: value.clone(),
            },
            Error::InvalidIdentifier(s) => Error::InvalidIdentifier(s.clone()),
            Error::Io(_) => Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                self.to_string(),
            )),
            Error::IoError(s) => Error::IoError(s.clone()),
            Error::JsonParse(_) => {
                Error::JsonParse(serde_json::from_str::<serde_json::Value>("{}").unwrap_err())
            }
            Error::YamlParse(_) => {
                Error::YamlParse(serde_yaml::from_str::<serde_yaml::Value>(":").unwrap_err())
            }
            Error::Database(s) => Error::Database(s.clone()),
            Error::Internal(s) => Error::Internal(s.clone()),
            Error::Unimplemented(s) => Error::Unimplemented(s.clone()),
            Error::JjCommandError {
                operation,
                msg,
                is_not_found,
            } => Error::JjCommandError {
                operation: operation.clone(),
                msg: msg.clone(),
                is_not_found: *is_not_found,
            },
            Error::JjWorkspaceConflict {
                conflict_type,
                workspace_name,
                msg,
                recovery_hint,
            } => Error::JjWorkspaceConflict {
                conflict_type: *conflict_type,
                workspace_name: workspace_name.clone(),
                msg: msg.clone(),
                recovery_hint: recovery_hint.clone(),
            },
            Error::LockTimeout {
                operation,
                timeout_ms,
                retries,
            } => Error::LockTimeout {
                operation: operation.clone(),
                timeout_ms: *timeout_ms,
                retries: *retries,
            },
            Error::InvalidConfig(s) => Error::InvalidConfig(s.clone()),
            Error::CloneFailed(s) => Error::CloneFailed(s.clone()),
            Error::RecordFailed(s) => Error::RecordFailed(s.clone()),
            Error::InvalidRepoUrl(s) => Error::InvalidRepoUrl(s.clone()),
        }
    }
}

/// Types of JJ workspace conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JjConflictType {
    /// Workspace already exists
    AlreadyExists,
    /// Concurrent modification detected
    ConcurrentModification,
    /// Workspace has been abandoned
    Abandoned,
    /// Working copy is stale
    Stale,
}

/// Result type alias using our custom error
pub type Result<T> = std::result::Result<T, Error>;

// ========================================================================
// Error Context & Suggestions
// ========================================================================

impl Error {
    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Error::WorkspaceNotFound(_name) => Some(format!(
                "Try 'scp workspace list' to see available workspaces"
            )),
            Error::SessionNotFound(_name) => {
                Some(format!("Try 'scp session list' to see available sessions"))
            }
            Error::QueueEmpty => {
                Some("No items in queue. Use 'scp queue enqueue <branch>' to add one".to_string())
            }
            Error::WorkspaceLocked(_, holder) => {
                Some(format!("Use 'scp agent kill {}' to force release", holder))
            }
            Error::VcsNotInitialized => Some("Run 'scp init' to initialize VCS".to_string()),
            Error::WorkingCopyDirty => {
                Some("Commit or stash your changes before continuing".to_string())
            }
            _ => None,
        }
    }

    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self {
            // Workspace/Session errors
            Error::WorkspaceNotFound(_) => 10,
            Error::WorkspaceExists(_) => 11,
            Error::WorkspaceLocked(_, _) => 12,
            Error::WorkspaceConflict(_) => 13,
            Error::SessionNotFound(_) => 14,
            Error::SessionExists(_) => 15,
            Error::SessionLocked(_, _) => 16,
            Error::NotLockHolder(_, _) => 17,
            Error::SessionInvalidState(_, _, _) => 18,

            // Queue errors
            Error::QueueEmpty => 20,
            Error::QueueItemNotFound(_) => 21,
            Error::QueueLocked(_) => 22,
            Error::QueueProcessing => 23,
            Error::QueueInvalidPosition(_) => 24,
            Error::QueueFull(_) => 25,

            // VCS errors
            Error::VcsNotInitialized => 30,
            Error::VcsConflict(_, _) => 31,
            Error::VcsPushFailed(_) => 32,
            Error::VcsPullFailed(_) => 33,
            Error::VcsRebaseFailed(_) => 34,
            Error::BranchNotFound(_) => 35,
            Error::BranchExists(_) => 36,
            Error::CommitNotFound(_) => 37,
            Error::WorkingCopyDirty => 38,

            // Config errors
            Error::ConfigNotFound(_) => 40,
            Error::ConfigInvalid(_) => 41,
            Error::ConfigPermission(_) => 42,

            // Agent errors
            Error::AgentNotFound(_) => 50,
            Error::AgentExists(_) => 51,
            Error::AgentTimeout(_) => 52,

            // State/Conflict errors
            Error::InvalidState(_) => 70,
            Error::NotFound(_) => 71,

            // Validation errors
            Error::ValidationError(_) => 80,
            Error::ValidationFieldError { .. } => 81,
            Error::InvalidIdentifier(_) => 82,

            // IO errors
            Error::Io(_) => 60,
            Error::IoError(_) => 64,
            Error::JsonParse(_) => 61,
            Error::YamlParse(_) => 62,
            Error::Database(_) => 63,

            // JJ errors
            Error::JjCommandError { .. } => 39,
            Error::JjWorkspaceConflict { .. } => 39,
            Error::LockTimeout { .. } => 37,

            // Internal
            Error::Internal(_) => 90,
            Error::Unimplemented(_) => 91,
            Error::InvalidConfig(_) => 92,
            Error::CloneFailed(_) => 93,
            Error::RecordFailed(_) => 94,
            Error::InvalidRepoUrl(_) => 95,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_suggestions() {
        let err = Error::WorkspaceNotFound("test".into());
        assert!(err.suggestion().is_some());

        let err = Error::QueueEmpty;
        assert!(err.suggestion().is_some());
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(Error::WorkspaceNotFound("x".into()).exit_code(), 10);
        assert_eq!(Error::QueueEmpty.exit_code(), 20);
        assert_eq!(Error::VcsNotInitialized.exit_code(), 30);
    }
}
