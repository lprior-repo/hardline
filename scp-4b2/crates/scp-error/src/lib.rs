#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Central error types for Source Control Plane.
//!
//! This crate provides the unified error types used across the SCP workspace.
//! All other crates should depend on this crate for error handling.

use serde::Serialize;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug, Serialize)]
#[non_exhaustive]
pub enum Error {
    // ========================================================================
    // Workspace/Session Errors (1xxx)
    // ========================================================================
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Workspace already exists: {0}")]
    WorkspaceExists(String),

    #[error("Workspace '{0}' is locked by '{1}'")]
    WorkspaceLocked(String, String),

    #[error("Workspace conflict: {0}")]
    WorkspaceConflict(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session already exists: {0}")]
    SessionExists(String),

    #[error("Session '{0}' is locked by '{1}'")]
    SessionLocked(String, String),

    #[error("Agent '{1}' does not hold lock on session '{0}'")]
    NotLockHolder(String, String),

    #[error("Session '{0}' is {1}, expected {2}")]
    SessionInvalidState(String, String, String),

    // ========================================================================
    // Bead Errors (1xxx - extended)
    // ========================================================================
    #[error("Bead not found: {0}")]
    BeadNotFound(String),

    #[error("Bead already exists: {0}")]
    BeadAlreadyExists(String),

    #[error("Invalid bead ID: {0}")]
    InvalidBeadId(String),

    #[error("Invalid bead title: {0}")]
    InvalidBeadTitle(String),

    #[error("Invalid bead state transition: {from} -> {to}")]
    BeadInvalidStateTransition { from: String, to: String },

    #[error("Dependency cycle detected: {0}")]
    BeadDependencyCycle(String),

    #[error("Bead is blocked by: [{0}]")]
    BeadBlockedBy(String),

    #[error("Invalid bead dependency: {0}")]
    BeadInvalidDependency(String),

    // ========================================================================
    // Queue Errors (2xxx)
    // ========================================================================
    #[error("Queue is empty")]
    QueueEmpty,

    #[error("Queue item not found: {0}")]
    QueueItemNotFound(String),

    #[error("Queue is locked by '{0}'")]
    QueueLocked(String),

    #[error("Queue operation already in progress")]
    QueueProcessing,

    #[error("Invalid queue position: {0}")]
    QueueInvalidPosition(usize),

    #[error("Queue is full (max: {0})")]
    QueueFull(usize),

    // ========================================================================
    // VCS Errors (3xxx)
    // ========================================================================
    #[error("VCS not initialized in this directory")]
    VcsNotInitialized,

    #[error("VCS conflict in {0}: {1}")]
    VcsConflict(String, String),

    #[error("Failed to push: {0}")]
    VcsPushFailed(String),

    #[error("Failed to pull: {0}")]
    VcsPullFailed(String),

    #[error("Failed to rebase: {0}")]
    VcsRebaseFailed(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Branch already exists: {0}")]
    BranchExists(String),

    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    #[error("Working copy has uncommitted changes")]
    WorkingCopyDirty,

    #[error("JJ command '{operation}' failed: {msg}")]
    JjCommandError {
        operation: String,
        msg: String,
        is_not_found: bool,
    },

    #[error("JJ workspace conflict: {conflict_type:?} for '{workspace_name}': {msg}")]
    JjWorkspaceConflict {
        conflict_type: JjConflictType,
        workspace_name: String,
        msg: String,
        recovery_hint: String,
    },

    // ========================================================================
    // Configuration Errors (4xxx)
    // ========================================================================
    #[error("Configuration not found: {0}")]
    ConfigNotFound(String),

    #[error("Configuration invalid: {0}")]
    ConfigInvalid(String),

    #[error("Configuration permission denied: {0}")]
    ConfigPermission(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Invalid repository URL: {0}")]
    InvalidRepoUrl(String),

    // ========================================================================
    // Agent Errors (5xxx)
    // ========================================================================
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Agent already registered: {0}")]
    AgentExists(String),

    #[error("Agent '{0}' heartbeat timeout")]
    AgentTimeout(String),

    // ========================================================================
    // State/Conflict Errors (6xxx)
    // ========================================================================
    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    // ========================================================================
    // Validation Errors (7xxx)
    // ========================================================================
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Validation error on '{field}': {message}")]
    ValidationFieldError {
        message: String,
        field: String,
        value: Option<String>,
    },

    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),

    // ========================================================================
    // IO/Storage Errors (8xxx)
    // ========================================================================
    #[error("IO error: {0}")]
    IoError(String),

    #[error("JSON parse error: {0}")]
    JsonParseError(String),

    #[error("YAML parse error: {0}")]
    YamlParseError(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    // ========================================================================
    // Orchestration/Workflow Errors (8xxx - extended)
    // ========================================================================
    #[error("Lock acquisition timeout for '{operation}' after {timeout_ms}ms ({retries} retries)")]
    LockTimeout {
        operation: String,
        timeout_ms: u64,
        retries: usize,
    },

    #[error("Clone failed: {0}")]
    CloneFailed(String),

    #[error("Record failed: {0}")]
    RecordFailed(String),

    #[error("Persistence error: {0}")]
    Persistence(String),

    #[error("State transition error: {0}")]
    StateTransition(String),

    // ========================================================================
    // Scenario/Execution Errors (8xxx - extended)
    // ========================================================================
    #[error("Scenario error: {0}")]
    ScenarioError(String),

    #[error("Runner error: {0}")]
    RunnerError(String),

    #[error("Definition error: {0}")]
    DefinitionError(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Sync error: {0}")]
    SyncError(String),

    // ========================================================================
    // Internal Errors (9xxx)
    // ========================================================================
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Not implemented: {0}")]
    Unimplemented(String),

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum JjConflictType {
    AlreadyExists,
    ConcurrentModification,
    Abandoned,
    Stale,
}

impl Error {
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Error::WorkspaceNotFound(_) => {
                Some("Try 'scp workspace list' to see available workspaces".into())
            }
            Error::SessionNotFound(_) => {
                Some("Try 'scp session list' to see available sessions".into())
            }
            Error::QueueEmpty => {
                Some("No items in queue. Use 'scp queue enqueue <branch>' to add one".into())
            }
            Error::WorkspaceLocked(_, holder) => {
                Some(format!("Use 'scp agent kill {}' to force release", holder))
            }
            Error::VcsNotInitialized => Some("Run 'scp init' to initialize VCS".into()),
            Error::WorkingCopyDirty => {
                Some("Commit or stash your changes before continuing".into())
            }
            _ => None,
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Error::WorkspaceNotFound(_) => 10,
            Error::WorkspaceExists(_) => 11,
            Error::WorkspaceLocked(_, _) => 12,
            Error::WorkspaceConflict(_) => 13,
            Error::SessionNotFound(_) => 14,
            Error::SessionExists(_) => 15,
            Error::SessionLocked(_, _) => 16,
            Error::NotLockHolder(_, _) => 17,
            Error::SessionInvalidState(_, _, _) => 18,
            Error::BeadNotFound(_) => 19,
            Error::BeadAlreadyExists(_) => 20,
            Error::QueueEmpty => 30,
            Error::QueueItemNotFound(_) => 31,
            Error::QueueLocked(_) => 32,
            Error::QueueProcessing => 33,
            Error::QueueInvalidPosition(_) => 34,
            Error::QueueFull(_) => 35,
            Error::VcsNotInitialized => 40,
            Error::VcsConflict(_, _) => 41,
            Error::VcsPushFailed(_) => 42,
            Error::VcsPullFailed(_) => 43,
            Error::VcsRebaseFailed(_) => 44,
            Error::BranchNotFound(_) => 45,
            Error::BranchExists(_) => 46,
            Error::CommitNotFound(_) => 47,
            Error::WorkingCopyDirty => 48,
            Error::JjCommandError { .. } => 49,
            Error::JjWorkspaceConflict { .. } => 50,
            Error::ConfigNotFound(_) => 60,
            Error::ConfigInvalid(_) => 61,
            Error::ConfigPermission(_) => 62,
            Error::InvalidConfig(_) => 63,
            Error::InvalidRepoUrl(_) => 64,
            Error::AgentNotFound(_) => 70,
            Error::AgentExists(_) => 71,
            Error::AgentTimeout(_) => 72,
            Error::InvalidState(_) => 80,
            Error::NotFound(_) => 81,
            Error::InvalidOperation(_) => 82,
            Error::ValidationError(_) => 90,
            Error::ValidationFieldError { .. } => 91,
            Error::InvalidIdentifier(_) => 92,
            Error::IoError(_) => 100,
            Error::JsonParseError(_) => 102,
            Error::YamlParseError(_) => 103,
            Error::Database(_) => 104,
            Error::Serialization(_) => 105,
            Error::LockTimeout { .. } => 110,
            Error::CloneFailed(_) => 111,
            Error::RecordFailed(_) => 112,
            Error::Persistence(_) => 113,
            Error::StateTransition(_) => 114,
            Error::ScenarioError(_) => 120,
            Error::RunnerError(_) => 121,
            Error::DefinitionError(_) => 122,
            Error::ServerError(_) => 123,
            Error::SyncError(_) => 124,
            Error::Internal(_) => 130,
            Error::Unimplemented(_) => 131,
            Error::InvariantViolation(_) => 132,
            Error::InvalidBeadId(_) => 133,
            Error::InvalidBeadTitle(_) => 134,
            Error::BeadInvalidStateTransition { .. } => 135,
            Error::BeadDependencyCycle(_) => 136,
            Error::BeadBlockedBy(_) => 137,
            Error::BeadInvalidDependency(_) => 138,
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

        let err = Error::Internal("test".into());
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(Error::WorkspaceNotFound("x".into()).exit_code(), 10);
        assert_eq!(Error::QueueEmpty.exit_code(), 30);
        assert_eq!(Error::VcsNotInitialized.exit_code(), 40);
    }

    #[test]
    fn test_bead_blocked_by_display() {
        let err = Error::BeadBlockedBy("bead1, bead2".into());
        assert_eq!(err.to_string(), "Bead is blocked by: [bead1, bead2]");
    }
}
