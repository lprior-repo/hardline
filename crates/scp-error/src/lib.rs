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

impl Error {
    #[must_use]
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Self::WorkspaceNotFound(_) => {
                Some("Try 'scp workspace list' to see available workspaces".into())
            }
            Self::SessionNotFound(_) => {
                Some("Try 'scp session list' to see available sessions".into())
            }
            Self::QueueEmpty => {
                Some("No items in queue. Use 'scp queue enqueue <branch>' to add one".into())
            }
            Self::WorkspaceLocked(_, holder) => {
                Some(format!("Use 'scp agent kill {holder}' to force release"))
            }
            Self::VcsNotInitialized => Some("Run 'scp init' to initialize VCS".into()),
            Self::WorkingCopyDirty => Some("Commit or stash your changes before continuing".into()),
            _ => None,
        }
    }

    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::WorkspaceNotFound(_) => 10,
            Self::WorkspaceExists(_) => 11,
            Self::WorkspaceLocked(_, _) => 12,
            Self::WorkspaceConflict(_) => 13,
            Self::SessionNotFound(_) => 14,
            Self::SessionExists(_) => 15,
            Self::SessionLocked(_, _) => 16,
            Self::NotLockHolder(_, _) => 17,
            Self::SessionInvalidState(_, _, _) => 18,
            Self::BeadNotFound(_) => 19,
            Self::BeadAlreadyExists(_) => 20,
            Self::QueueEmpty => 30,
            Self::QueueItemNotFound(_) => 31,
            Self::QueueLocked(_) => 32,
            Self::QueueProcessing => 33,
            Self::QueueInvalidPosition(_) => 34,
            Self::QueueFull(_) => 35,
            Self::VcsNotInitialized => 40,
            Self::VcsConflict(_, _) => 41,
            Self::VcsPushFailed(_) => 42,
            Self::VcsPullFailed(_) => 43,
            Self::VcsRebaseFailed(_) => 44,
            Self::BranchNotFound(_) => 45,
            Self::BranchExists(_) => 46,
            Self::CommitNotFound(_) => 47,
            Self::WorkingCopyDirty => 48,
            Self::ConfigNotFound(_) => 60,
            Self::ConfigInvalid(_) => 61,
            Self::ConfigPermission(_) => 62,
            Self::InvalidConfig(_) => 63,
            Self::InvalidRepoUrl(_) => 64,
            Self::AgentNotFound(_) => 70,
            Self::AgentExists(_) => 71,
            Self::AgentTimeout(_) => 72,
            Self::InvalidState(_) => 80,
            Self::NotFound(_) => 81,
            Self::InvalidOperation(_) => 82,
            Self::ValidationError(_) => 90,
            Self::ValidationFieldError { .. } => 91,
            Self::InvalidIdentifier(_) => 92,
            Self::IoError(_) => 100,
            Self::JsonParseError(_) => 102,
            Self::YamlParseError(_) => 103,
            Self::Database(_) => 104,
            Self::Serialization(_) => 105,
            Self::LockTimeout { .. } => 110,
            Self::CloneFailed(_) => 111,
            Self::RecordFailed(_) => 112,
            Self::Persistence(_) => 113,
            Self::StateTransition(_) => 114,
            Self::ScenarioError(_) => 120,
            Self::RunnerError(_) => 121,
            Self::DefinitionError(_) => 122,
            Self::ServerError(_) => 123,
            Self::SyncError(_) => 124,
            Self::Internal(_) => 130,
            Self::Unimplemented(_) => 131,
            Self::InvariantViolation(_) => 132,
            Self::InvalidBeadId(_) => 133,
            Self::InvalidBeadTitle(_) => 134,
            Self::BeadInvalidStateTransition { .. } => 135,
            Self::BeadDependencyCycle(_) => 136,
            Self::BeadBlockedBy(_) => 137,
            Self::BeadInvalidDependency(_) => 138,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // CLAIM 1: Display output for every variant
    // =========================================================================

    // --- Workspace/Session ---
    #[test]
    fn display_workspace_not_found() {
        assert_eq!(
            Error::WorkspaceNotFound("my-ws".into()).to_string(),
            "Workspace not found: my-ws"
        );
    }

    #[test]
    fn display_workspace_exists() {
        assert_eq!(
            Error::WorkspaceExists("my-ws".into()).to_string(),
            "Workspace already exists: my-ws"
        );
    }

    #[test]
    fn display_workspace_locked() {
        assert_eq!(
            Error::WorkspaceLocked("my-ws".into(), "agent-1".into()).to_string(),
            "Workspace 'my-ws' is locked by 'agent-1'"
        );
    }

    #[test]
    fn display_workspace_conflict() {
        assert_eq!(
            Error::WorkspaceConflict("overlap".into()).to_string(),
            "Workspace conflict: overlap"
        );
    }

    #[test]
    fn display_session_not_found() {
        assert_eq!(
            Error::SessionNotFound("s1".into()).to_string(),
            "Session not found: s1"
        );
    }

    #[test]
    fn display_session_exists() {
        assert_eq!(
            Error::SessionExists("s1".into()).to_string(),
            "Session already exists: s1"
        );
    }

    #[test]
    fn display_session_locked() {
        assert_eq!(
            Error::SessionLocked("s1".into(), "agent-2".into()).to_string(),
            "Session 's1' is locked by 'agent-2'"
        );
    }

    #[test]
    fn display_not_lock_holder() {
        assert_eq!(
            Error::NotLockHolder("s1".into(), "imposter".into()).to_string(),
            "Agent 'imposter' does not hold lock on session 's1'"
        );
    }

    #[test]
    fn display_session_invalid_state() {
        assert_eq!(
            Error::SessionInvalidState("s1".into(), "closed".into(), "open".into()).to_string(),
            "Session 's1' is closed, expected open"
        );
    }

    // --- Bead ---
    #[test]
    fn display_bead_not_found() {
        assert_eq!(
            Error::BeadNotFound("ha-123".into()).to_string(),
            "Bead not found: ha-123"
        );
    }

    #[test]
    fn display_bead_already_exists() {
        assert_eq!(
            Error::BeadAlreadyExists("ha-123".into()).to_string(),
            "Bead already exists: ha-123"
        );
    }

    #[test]
    fn display_invalid_bead_id() {
        assert_eq!(
            Error::InvalidBeadId("bad id!".into()).to_string(),
            "Invalid bead ID: bad id!"
        );
    }

    #[test]
    fn display_invalid_bead_title() {
        assert_eq!(
            Error::InvalidBeadTitle("".into()).to_string(),
            "Invalid bead title: "
        );
    }

    #[test]
    fn display_bead_invalid_state_transition() {
        assert_eq!(
            Error::BeadInvalidStateTransition {
                from: "open".into(),
                to: "closed".into()
            }
            .to_string(),
            "Invalid bead state transition: open -> closed"
        );
    }

    #[test]
    fn display_bead_dependency_cycle() {
        assert_eq!(
            Error::BeadDependencyCycle("ha-1 -> ha-2 -> ha-1".into()).to_string(),
            "Dependency cycle detected: ha-1 -> ha-2 -> ha-1"
        );
    }

    #[test]
    fn display_bead_blocked_by() {
        assert_eq!(
            Error::BeadBlockedBy("ha-5, ha-6".into()).to_string(),
            "Bead is blocked by: [ha-5, ha-6]"
        );
    }

    #[test]
    fn display_bead_invalid_dependency() {
        assert_eq!(
            Error::BeadInvalidDependency("self-ref".into()).to_string(),
            "Invalid bead dependency: self-ref"
        );
    }

    // --- Queue ---
    #[test]
    fn display_queue_empty() {
        assert_eq!(Error::QueueEmpty.to_string(), "Queue is empty");
    }

    #[test]
    fn display_queue_item_not_found() {
        assert_eq!(
            Error::QueueItemNotFound("item-1".into()).to_string(),
            "Queue item not found: item-1"
        );
    }

    #[test]
    fn display_queue_locked() {
        assert_eq!(
            Error::QueueLocked("agent-3".into()).to_string(),
            "Queue is locked by 'agent-3'"
        );
    }

    #[test]
    fn display_queue_processing() {
        assert_eq!(
            Error::QueueProcessing.to_string(),
            "Queue operation already in progress"
        );
    }

    #[test]
    fn display_queue_invalid_position() {
        assert_eq!(
            Error::QueueInvalidPosition(999).to_string(),
            "Invalid queue position: 999"
        );
    }

    #[test]
    fn display_queue_full() {
        assert_eq!(
            Error::QueueFull(50).to_string(),
            "Queue is full (max: 50)"
        );
    }

    // --- VCS ---
    #[test]
    fn display_vcs_not_initialized() {
        assert_eq!(
            Error::VcsNotInitialized.to_string(),
            "VCS not initialized in this directory"
        );
    }

    #[test]
    fn display_vcs_conflict() {
        assert_eq!(
            Error::VcsConflict("file.rs".into(), "merge conflict".into()).to_string(),
            "VCS conflict in file.rs: merge conflict"
        );
    }

    #[test]
    fn display_vcs_push_failed() {
        assert_eq!(
            Error::VcsPushFailed("rejected".into()).to_string(),
            "Failed to push: rejected"
        );
    }

    #[test]
    fn display_vcs_pull_failed() {
        assert_eq!(
            Error::VcsPullFailed("network error".into()).to_string(),
            "Failed to pull: network error"
        );
    }

    #[test]
    fn display_vcs_rebase_failed() {
        assert_eq!(
            Error::VcsRebaseFailed("conflict in main.rs".into()).to_string(),
            "Failed to rebase: conflict in main.rs"
        );
    }

    #[test]
    fn display_branch_not_found() {
        assert_eq!(
            Error::BranchNotFound("feature/x".into()).to_string(),
            "Branch not found: feature/x"
        );
    }

    #[test]
    fn display_branch_exists() {
        assert_eq!(
            Error::BranchExists("main".into()).to_string(),
            "Branch already exists: main"
        );
    }

    #[test]
    fn display_commit_not_found() {
        assert_eq!(
            Error::CommitNotFound("abc1234".into()).to_string(),
            "Commit not found: abc1234"
        );
    }

    #[test]
    fn display_working_copy_dirty() {
        assert_eq!(
            Error::WorkingCopyDirty.to_string(),
            "Working copy has uncommitted changes"
        );
    }

    // --- Config ---
    #[test]
    fn display_config_not_found() {
        assert_eq!(
            Error::ConfigNotFound("settings.toml".into()).to_string(),
            "Configuration not found: settings.toml"
        );
    }

    #[test]
    fn display_config_invalid() {
        assert_eq!(
            Error::ConfigInvalid("bad format".into()).to_string(),
            "Configuration invalid: bad format"
        );
    }

    #[test]
    fn display_config_permission() {
        assert_eq!(
            Error::ConfigPermission("/etc/scp/config".into()).to_string(),
            "Configuration permission denied: /etc/scp/config"
        );
    }

    #[test]
    fn display_invalid_config() {
        assert_eq!(
            Error::InvalidConfig("missing field 'name'".into()).to_string(),
            "Invalid configuration: missing field 'name'"
        );
    }

    #[test]
    fn display_invalid_repo_url() {
        assert_eq!(
            Error::InvalidRepoUrl("not-a-url".into()).to_string(),
            "Invalid repository URL: not-a-url"
        );
    }

    // --- Agent ---
    #[test]
    fn display_agent_not_found() {
        assert_eq!(
            Error::AgentNotFound("alpha".into()).to_string(),
            "Agent not found: alpha"
        );
    }

    #[test]
    fn display_agent_exists() {
        assert_eq!(
            Error::AgentExists("alpha".into()).to_string(),
            "Agent already registered: alpha"
        );
    }

    #[test]
    fn display_agent_timeout() {
        assert_eq!(
            Error::AgentTimeout("beta".into()).to_string(),
            "Agent 'beta' heartbeat timeout"
        );
    }

    // --- State/Conflict ---
    #[test]
    fn display_invalid_state() {
        assert_eq!(
            Error::InvalidState("corrupted".into()).to_string(),
            "Invalid state: corrupted"
        );
    }

    #[test]
    fn display_not_found() {
        assert_eq!(
            Error::NotFound("resource".into()).to_string(),
            "Not found: resource"
        );
    }

    #[test]
    fn display_invalid_operation() {
        assert_eq!(
            Error::InvalidOperation("delete on read-only".into()).to_string(),
            "Invalid operation: delete on read-only"
        );
    }

    // --- Validation ---
    #[test]
    fn display_validation_error() {
        assert_eq!(
            Error::ValidationError("field required".into()).to_string(),
            "Validation error: field required"
        );
    }

    #[test]
    fn display_validation_field_error() {
        let err = Error::ValidationFieldError {
            message: "must be positive".into(),
            field: "age".into(),
            value: Some("-5".into()),
        };
        assert_eq!(err.to_string(), "Validation error on 'age': must be positive");
    }

    #[test]
    fn display_validation_field_error_no_value() {
        let err = Error::ValidationFieldError {
            message: "required".into(),
            field: "name".into(),
            value: None,
        };
        assert_eq!(err.to_string(), "Validation error on 'name': required");
    }

    #[test]
    fn display_invalid_identifier() {
        assert_eq!(
            Error::InvalidIdentifier("123bad".into()).to_string(),
            "Invalid identifier: 123bad"
        );
    }

    // --- IO/Storage ---
    #[test]
    fn display_io_error() {
        assert_eq!(
            Error::IoError("permission denied".into()).to_string(),
            "IO error: permission denied"
        );
    }

    #[test]
    fn display_json_parse_error() {
        assert_eq!(
            Error::JsonParseError("unexpected token".into()).to_string(),
            "JSON parse error: unexpected token"
        );
    }

    #[test]
    fn display_yaml_parse_error() {
        assert_eq!(
            Error::YamlParseError("invalid mapping".into()).to_string(),
            "YAML parse error: invalid mapping"
        );
    }

    #[test]
    fn display_database() {
        assert_eq!(
            Error::Database("connection refused".into()).to_string(),
            "Database error: connection refused"
        );
    }

    #[test]
    fn display_serialization() {
        assert_eq!(
            Error::Serialization("buffer overflow".into()).to_string(),
            "Serialization error: buffer overflow"
        );
    }

    // --- Orchestration/Workflow ---
    #[test]
    fn display_lock_timeout() {
        let err = Error::LockTimeout {
            operation: "acquire workspace".into(),
            timeout_ms: 5000,
            retries: 3,
        };
        assert_eq!(
            err.to_string(),
            "Lock acquisition timeout for 'acquire workspace' after 5000ms (3 retries)"
        );
    }

    #[test]
    fn display_clone_failed() {
        assert_eq!(
            Error::CloneFailed("repo unreachable".into()).to_string(),
            "Clone failed: repo unreachable"
        );
    }

    #[test]
    fn display_record_failed() {
        assert_eq!(
            Error::RecordFailed("write failed".into()).to_string(),
            "Record failed: write failed"
        );
    }

    #[test]
    fn display_persistence() {
        assert_eq!(
            Error::Persistence("disk full".into()).to_string(),
            "Persistence error: disk full"
        );
    }

    #[test]
    fn display_state_transition() {
        assert_eq!(
            Error::StateTransition("open -> locked invalid".into()).to_string(),
            "State transition error: open -> locked invalid"
        );
    }

    // --- Scenario/Execution ---
    #[test]
    fn display_scenario_error() {
        assert_eq!(
            Error::ScenarioError("step 3 failed".into()).to_string(),
            "Scenario error: step 3 failed"
        );
    }

    #[test]
    fn display_runner_error() {
        assert_eq!(
            Error::RunnerError("binary not found".into()).to_string(),
            "Runner error: binary not found"
        );
    }

    #[test]
    fn display_definition_error() {
        assert_eq!(
            Error::DefinitionError("missing required field".into()).to_string(),
            "Definition error: missing required field"
        );
    }

    #[test]
    fn display_server_error() {
        assert_eq!(
            Error::ServerError("port 8080 in use".into()).to_string(),
            "Server error: port 8080 in use"
        );
    }

    #[test]
    fn display_sync_error() {
        assert_eq!(
            Error::SyncError("remote diverged".into()).to_string(),
            "Sync error: remote diverged"
        );
    }

    // --- Internal ---
    #[test]
    fn display_internal() {
        assert_eq!(
            Error::Internal("unexpected null".into()).to_string(),
            "Internal error: unexpected null"
        );
    }

    #[test]
    fn display_unimplemented() {
        assert_eq!(
            Error::Unimplemented("feature X".into()).to_string(),
            "Not implemented: feature X"
        );
    }

    #[test]
    fn display_invariant_violation() {
        assert_eq!(
            Error::InvariantViolation("assumption broken".into()).to_string(),
            "Invariant violation: assumption broken"
        );
    }

    // =========================================================================
    // CLAIM 2: suggestion() returns correct values
    // =========================================================================

    #[test]
    fn suggestion_workspace_not_found() {
        let err = Error::WorkspaceNotFound("x".into());
        assert_eq!(
            err.suggestion(),
            Some("Try 'scp workspace list' to see available workspaces".into())
        );
    }

    #[test]
    fn suggestion_session_not_found() {
        let err = Error::SessionNotFound("x".into());
        assert_eq!(
            err.suggestion(),
            Some("Try 'scp session list' to see available sessions".into())
        );
    }

    #[test]
    fn suggestion_queue_empty() {
        assert_eq!(
            Error::QueueEmpty.suggestion(),
            Some("No items in queue. Use 'scp queue enqueue <branch>' to add one".into())
        );
    }

    #[test]
    fn suggestion_workspace_locked_includes_holder() {
        let err = Error::WorkspaceLocked("ws".into(), "agent-x".into());
        assert_eq!(
            err.suggestion(),
            Some("Use 'scp agent kill agent-x' to force release".into())
        );
    }

    #[test]
    fn suggestion_vcs_not_initialized() {
        assert_eq!(
            Error::VcsNotInitialized.suggestion(),
            Some("Run 'scp init' to initialize VCS".into())
        );
    }

    #[test]
    fn suggestion_working_copy_dirty() {
        assert_eq!(
            Error::WorkingCopyDirty.suggestion(),
            Some("Commit or stash your changes before continuing".into())
        );
    }

    #[test]
    fn suggestion_none_for_internal() {
        assert!(Error::Internal("x".into()).suggestion().is_none());
    }

    #[test]
    fn suggestion_none_for_all_other_variants() {
        // Sample non-suggestion variants to confirm they return None
        let no_suggestion = [
            Error::WorkspaceExists("x".into()),
            Error::WorkspaceConflict("x".into()),
            Error::SessionExists("x".into()),
            Error::SessionLocked("x".into(), "y".into()),
            Error::NotLockHolder("x".into(), "y".into()),
            Error::SessionInvalidState("x".into(), "y".into(), "z".into()),
            Error::BeadNotFound("x".into()),
            Error::BeadAlreadyExists("x".into()),
            Error::QueueItemNotFound("x".into()),
            Error::QueueLocked("x".into()),
            Error::QueueProcessing,
            Error::QueueInvalidPosition(0),
            Error::QueueFull(10),
            Error::VcsConflict("x".into(), "y".into()),
            Error::VcsPushFailed("x".into()),
            Error::VcsPullFailed("x".into()),
            Error::VcsRebaseFailed("x".into()),
            Error::BranchNotFound("x".into()),
            Error::BranchExists("x".into()),
            Error::CommitNotFound("x".into()),
            Error::ConfigNotFound("x".into()),
            Error::ConfigInvalid("x".into()),
            Error::ConfigPermission("x".into()),
            Error::InvalidConfig("x".into()),
            Error::InvalidRepoUrl("x".into()),
            Error::AgentNotFound("x".into()),
            Error::AgentExists("x".into()),
            Error::AgentTimeout("x".into()),
            Error::InvalidState("x".into()),
            Error::NotFound("x".into()),
            Error::InvalidOperation("x".into()),
            Error::ValidationError("x".into()),
            Error::ValidationFieldError {
                message: "x".into(),
                field: "y".into(),
                value: None,
            },
            Error::InvalidIdentifier("x".into()),
            Error::IoError("x".into()),
            Error::JsonParseError("x".into()),
            Error::YamlParseError("x".into()),
            Error::Database("x".into()),
            Error::Serialization("x".into()),
            Error::LockTimeout {
                operation: "x".into(),
                timeout_ms: 0,
                retries: 0,
            },
            Error::CloneFailed("x".into()),
            Error::RecordFailed("x".into()),
            Error::Persistence("x".into()),
            Error::StateTransition("x".into()),
            Error::ScenarioError("x".into()),
            Error::RunnerError("x".into()),
            Error::DefinitionError("x".into()),
            Error::ServerError("x".into()),
            Error::SyncError("x".into()),
            Error::Unimplemented("x".into()),
            Error::InvariantViolation("x".into()),
            Error::InvalidBeadId("x".into()),
            Error::InvalidBeadTitle("x".into()),
            Error::BeadInvalidStateTransition {
                from: "x".into(),
                to: "y".into(),
            },
            Error::BeadDependencyCycle("x".into()),
            Error::BeadBlockedBy("x".into()),
            Error::BeadInvalidDependency("x".into()),
        ];
        for err in no_suggestion {
            assert!(err.suggestion().is_none(), "Expected no suggestion for: {err}");
        }
    }

    // =========================================================================
    // CLAIM 3: exit_code() for every variant
    // =========================================================================

    #[test]
    fn exit_codes_workspace_session() {
        assert_eq!(Error::WorkspaceNotFound("x".into()).exit_code(), 10);
        assert_eq!(Error::WorkspaceExists("x".into()).exit_code(), 11);
        assert_eq!(Error::WorkspaceLocked("x".into(), "y".into()).exit_code(), 12);
        assert_eq!(Error::WorkspaceConflict("x".into()).exit_code(), 13);
        assert_eq!(Error::SessionNotFound("x".into()).exit_code(), 14);
        assert_eq!(Error::SessionExists("x".into()).exit_code(), 15);
        assert_eq!(Error::SessionLocked("x".into(), "y".into()).exit_code(), 16);
        assert_eq!(Error::NotLockHolder("x".into(), "y".into()).exit_code(), 17);
        assert_eq!(Error::SessionInvalidState("a".into(), "b".into(), "c".into()).exit_code(), 18);
    }

    #[test]
    fn exit_codes_bead() {
        assert_eq!(Error::BeadNotFound("x".into()).exit_code(), 19);
        assert_eq!(Error::BeadAlreadyExists("x".into()).exit_code(), 20);
    }

    #[test]
    fn exit_codes_bead_extended() {
        assert_eq!(Error::InvalidBeadId("x".into()).exit_code(), 133);
        assert_eq!(Error::InvalidBeadTitle("x".into()).exit_code(), 134);
        assert_eq!(
            Error::BeadInvalidStateTransition {
                from: "a".into(),
                to: "b".into()
            }
            .exit_code(),
            135
        );
        assert_eq!(Error::BeadDependencyCycle("x".into()).exit_code(), 136);
        assert_eq!(Error::BeadBlockedBy("x".into()).exit_code(), 137);
        assert_eq!(Error::BeadInvalidDependency("x".into()).exit_code(), 138);
    }

    #[test]
    fn exit_codes_queue() {
        assert_eq!(Error::QueueEmpty.exit_code(), 30);
        assert_eq!(Error::QueueItemNotFound("x".into()).exit_code(), 31);
        assert_eq!(Error::QueueLocked("x".into()).exit_code(), 32);
        assert_eq!(Error::QueueProcessing.exit_code(), 33);
        assert_eq!(Error::QueueInvalidPosition(0).exit_code(), 34);
        assert_eq!(Error::QueueFull(10).exit_code(), 35);
    }

    #[test]
    fn exit_codes_vcs() {
        assert_eq!(Error::VcsNotInitialized.exit_code(), 40);
        assert_eq!(Error::VcsConflict("a".into(), "b".into()).exit_code(), 41);
        assert_eq!(Error::VcsPushFailed("x".into()).exit_code(), 42);
        assert_eq!(Error::VcsPullFailed("x".into()).exit_code(), 43);
        assert_eq!(Error::VcsRebaseFailed("x".into()).exit_code(), 44);
        assert_eq!(Error::BranchNotFound("x".into()).exit_code(), 45);
        assert_eq!(Error::BranchExists("x".into()).exit_code(), 46);
        assert_eq!(Error::CommitNotFound("x".into()).exit_code(), 47);
        assert_eq!(Error::WorkingCopyDirty.exit_code(), 48);
    }

    #[test]
    fn exit_codes_config() {
        assert_eq!(Error::ConfigNotFound("x".into()).exit_code(), 60);
        assert_eq!(Error::ConfigInvalid("x".into()).exit_code(), 61);
        assert_eq!(Error::ConfigPermission("x".into()).exit_code(), 62);
        assert_eq!(Error::InvalidConfig("x".into()).exit_code(), 63);
        assert_eq!(Error::InvalidRepoUrl("x".into()).exit_code(), 64);
    }

    #[test]
    fn exit_codes_agent() {
        assert_eq!(Error::AgentNotFound("x".into()).exit_code(), 70);
        assert_eq!(Error::AgentExists("x".into()).exit_code(), 71);
        assert_eq!(Error::AgentTimeout("x".into()).exit_code(), 72);
    }

    #[test]
    fn exit_codes_state_conflict() {
        assert_eq!(Error::InvalidState("x".into()).exit_code(), 80);
        assert_eq!(Error::NotFound("x".into()).exit_code(), 81);
        assert_eq!(Error::InvalidOperation("x".into()).exit_code(), 82);
    }

    #[test]
    fn exit_codes_validation() {
        assert_eq!(Error::ValidationError("x".into()).exit_code(), 90);
        assert_eq!(
            Error::ValidationFieldError {
                message: "x".into(),
                field: "y".into(),
                value: None,
            }
            .exit_code(),
            91
        );
        assert_eq!(Error::InvalidIdentifier("x".into()).exit_code(), 92);
    }

    #[test]
    fn exit_codes_io_storage() {
        assert_eq!(Error::IoError("x".into()).exit_code(), 100);
        assert_eq!(Error::JsonParseError("x".into()).exit_code(), 102);
        assert_eq!(Error::YamlParseError("x".into()).exit_code(), 103);
        assert_eq!(Error::Database("x".into()).exit_code(), 104);
        assert_eq!(Error::Serialization("x".into()).exit_code(), 105);
    }

    #[test]
    fn exit_codes_orchestration() {
        assert_eq!(
            Error::LockTimeout {
                operation: "x".into(),
                timeout_ms: 0,
                retries: 0,
            }
            .exit_code(),
            110
        );
        assert_eq!(Error::CloneFailed("x".into()).exit_code(), 111);
        assert_eq!(Error::RecordFailed("x".into()).exit_code(), 112);
        assert_eq!(Error::Persistence("x".into()).exit_code(), 113);
        assert_eq!(Error::StateTransition("x".into()).exit_code(), 114);
    }

    #[test]
    fn exit_codes_scenario_execution() {
        assert_eq!(Error::ScenarioError("x".into()).exit_code(), 120);
        assert_eq!(Error::RunnerError("x".into()).exit_code(), 121);
        assert_eq!(Error::DefinitionError("x".into()).exit_code(), 122);
        assert_eq!(Error::ServerError("x".into()).exit_code(), 123);
        assert_eq!(Error::SyncError("x".into()).exit_code(), 124);
    }

    #[test]
    fn exit_codes_internal() {
        assert_eq!(Error::Internal("x".into()).exit_code(), 130);
        assert_eq!(Error::Unimplemented("x".into()).exit_code(), 131);
        assert_eq!(Error::InvariantViolation("x".into()).exit_code(), 132);
    }

    // =========================================================================
    // CLAIM 4: Serialization (JSON)
    // =========================================================================

    #[test]
    fn serialize_unit_variant() {
        let json = serde_json::to_string(&Error::QueueEmpty).unwrap();
        assert!(json.contains("QueueEmpty"));
    }

    #[test]
    fn serialize_tuple_variant() {
        let json = serde_json::to_string(&Error::WorkspaceNotFound("ws-1".into())).unwrap();
        assert!(json.contains("WorkspaceNotFound"));
        assert!(json.contains("ws-1"));
    }

    #[test]
    fn serialize_struct_variant() {
        let err = Error::LockTimeout {
            operation: "lock".into(),
            timeout_ms: 5000,
            retries: 3,
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("LockTimeout"));
        assert!(json.contains("lock"));
        assert!(json.contains("5000"));
        assert!(json.contains("3"));
    }

    #[test]
    fn serialize_validation_field_error_with_none_value() {
        let err = Error::ValidationFieldError {
            message: "required".into(),
            field: "name".into(),
            value: None,
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("ValidationFieldError"));
        assert!(json.contains("required"));
        assert!(json.contains("name"));
    }

    #[test]
    fn serialize_usize_variant() {
        let json = serde_json::to_string(&Error::QueueFull(42)).unwrap();
        assert!(json.contains("QueueFull"));
        assert!(json.contains("42"));
    }

    // =========================================================================
    // CLAIM 5: Result<T> type alias
    // =========================================================================

    #[test]
    fn result_alias_ok() {
        fn returns_result() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(returns_result().unwrap(), 42);
    }

    #[test]
    fn result_alias_err() {
        fn returns_result() -> Result<i32> {
            Err(Error::NotFound("thing".into()))
        }
        let err = returns_result().unwrap_err();
        assert_eq!(err.to_string(), "Not found: thing");
    }

    // =========================================================================
    // CLAIM 6: Non-exhaustive forces wildcard in downstream match
    // =========================================================================

    #[test]
    fn non_exhaustive_allows_wildcard() {
        // The #[non_exhaustive] attribute means downstream crates must use _
        let err = Error::Internal("test".into());
        match err {
            Error::Internal(_) => {}  // known arm
            _ => {}                   // wildcard required
        }
    }

    // =========================================================================
    // CLAIM 7: Unit variants construct without arguments
    // =========================================================================

    #[test]
    fn unit_variant_construction() {
        let _ = Error::QueueEmpty;
        let _ = Error::QueueProcessing;
        let _ = Error::VcsNotInitialized;
        let _ = Error::WorkingCopyDirty;
    }

    // =========================================================================
    // CLAIM 8: Struct variants expose named fields
    // =========================================================================

    #[test]
    fn struct_variant_field_access_bead_transition() {
        let err = Error::BeadInvalidStateTransition {
            from: "open".into(),
            to: "closed".into(),
        };
        if let Error::BeadInvalidStateTransition { from, to } = err {
            assert_eq!(from, "open");
            assert_eq!(to, "closed");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn struct_variant_field_access_lock_timeout() {
        let err = Error::LockTimeout {
            operation: "acquire".into(),
            timeout_ms: 3000,
            retries: 5,
        };
        if let Error::LockTimeout {
            operation,
            timeout_ms,
            retries,
        } = err
        {
            assert_eq!(operation, "acquire");
            assert_eq!(timeout_ms, 3000);
            assert_eq!(retries, 5);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn struct_variant_field_access_validation_field_error() {
        let err = Error::ValidationFieldError {
            message: "too short".into(),
            field: "title".into(),
            value: Some("ab".into()),
        };
        if let Error::ValidationFieldError {
            message,
            field,
            value,
        } = err
        {
            assert_eq!(message, "too short");
            assert_eq!(field, "title");
            assert_eq!(value.as_deref(), Some("ab"));
        } else {
            panic!("wrong variant");
        }
    }

    // =========================================================================
    // CLAIM 9: Tuple variants destructure correctly
    // =========================================================================

    #[test]
    fn tuple_variant_destructure_single() {
        let err = Error::IoError("disk full".into());
        if let Error::IoError(msg) = err {
            assert_eq!(msg, "disk full");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn tuple_variant_destructure_pair() {
        let err = Error::WorkspaceLocked("ws".into(), "agent".into());
        if let Error::WorkspaceLocked(ws, agent) = err {
            assert_eq!(ws, "ws");
            assert_eq!(agent, "agent");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn tuple_variant_destructure_triple() {
        let err = Error::SessionInvalidState("s1".into(), "closed".into(), "open".into());
        if let Error::SessionInvalidState(id, actual, expected) = err {
            assert_eq!(id, "s1");
            assert_eq!(actual, "closed");
            assert_eq!(expected, "open");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn usize_variant_destructure() {
        let err = Error::QueueInvalidPosition(42);
        if let Error::QueueInvalidPosition(pos) = err {
            assert_eq!(pos, 42);
        } else {
            panic!("wrong variant");
        }
    }

    // =========================================================================
    // CLAIM 10: Edge cases — empty strings, special characters, unicode
    // =========================================================================

    #[test]
    fn empty_string_in_display() {
        assert_eq!(Error::NotFound("".into()).to_string(), "Not found: ");
        assert_eq!(Error::InvalidBeadTitle("".into()).to_string(), "Invalid bead title: ");
    }

    #[test]
    fn unicode_in_display() {
        assert_eq!(
            Error::WorkspaceNotFound("ワークスペース".into()).to_string(),
            "Workspace not found: ワークスペース"
        );
    }

    #[test]
    fn special_characters_in_display() {
        assert_eq!(
            Error::IoError("error: \n\t\r\"'\\{}".into()).to_string(),
            "IO error: error: \n\t\r\"'\\{}"
        );
    }

    #[test]
    fn very_long_string_in_display() {
        let long = "x".repeat(10_000);
        let result = Error::Internal(long.clone()).to_string();
        assert!(result.starts_with("Internal error: "));
        assert!(result.len() > 10_000);
    }

    #[test]
    fn zero_usize_in_queue_position() {
        assert_eq!(Error::QueueInvalidPosition(0).to_string(), "Invalid queue position: 0");
    }

    #[test]
    fn max_usize_in_queue_full() {
        assert_eq!(
            Error::QueueFull(usize::MAX).to_string(),
            format!("Queue is full (max: {})", usize::MAX)
        );
    }

    #[test]
    fn zero_timeout_in_lock_timeout() {
        let err = Error::LockTimeout {
            operation: "x".into(),
            timeout_ms: 0,
            retries: 0,
        };
        assert_eq!(err.exit_code(), 110);
        assert!(err.to_string().contains("0ms"));
        assert!(err.to_string().contains("0 retries"));
    }

    // =========================================================================
    // CLAIM 11: std::error::Error trait is implemented
    // =========================================================================

    #[test]
    fn error_trait_is_object_safe() {
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&Error::Internal("test".into()));
    }

    // =========================================================================
    // CLAIM 12: Debug trait works
    // =========================================================================

    #[test]
    fn debug_format_contains_variant_name() {
        let err = Error::QueueEmpty;
        let debug = format!("{err:?}");
        assert!(debug.contains("QueueEmpty"));
    }
}
