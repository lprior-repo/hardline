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

/// Result type alias for operations that can fail with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type for Source Control Plane.
///
/// Error codes:
/// - 1xxx: Workspace/Session/Bead errors
/// - 2xxx: Queue errors
/// - 3xxx: VCS errors
/// - 4xxx: Configuration errors
/// - 5xxx: Agent errors
/// - 6xxx: State/Conflict errors
/// - 7xxx: Validation errors
/// - 8xxx: IO/Storage/Orchestration errors
/// - 9xxx: Internal errors
#[derive(Error, Debug, Serialize)]
#[non_exhaustive]
pub enum Error {
    // ========================================================================
    // Workspace/Session Errors (1xxx)
    // ========================================================================
    /// Workspace not found in the database.
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    /// Workspace already exists (duplicate creation attempt).
    #[error("Workspace already exists: {0}")]
    WorkspaceExists(String),

    /// Workspace is locked by another agent.
    #[error("Workspace '{0}' is locked by '{1}'")]
    WorkspaceLocked(String, String),

    /// Workspace has an irreconcilable conflict.
    #[error("Workspace conflict: {0}")]
    WorkspaceConflict(String),

    /// Session not found in the database.
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Session already exists (duplicate creation attempt).
    #[error("Session already exists: {0}")]
    SessionExists(String),

    /// Session is locked by another agent.
    #[error("Session '{0}' is locked by '{1}'")]
    SessionLocked(String, String),

    /// Agent attempted an operation without holding the session lock.
    #[error("Agent '{1}' does not hold lock on session '{0}'")]
    NotLockHolder(String, String),

    /// Session is in an unexpected state for the requested operation.
    #[error("Session '{0}' is {1}, expected {2}")]
    SessionInvalidState(String, String, String),

    // ========================================================================
    // Bead Errors (1xxx - extended)
    // ========================================================================
    /// Bead (issue) not found in the database.
    #[error("Bead not found: {0}")]
    BeadNotFound(String),

    /// Bead already exists (duplicate creation attempt).
    #[error("Bead already exists: {0}")]
    BeadAlreadyExists(String),

    /// Bead ID does not match the expected format.
    #[error("Invalid bead ID: {0}")]
    InvalidBeadId(String),

    /// Bead title fails validation rules.
    #[error("Invalid bead title: {0}")]
    InvalidBeadTitle(String),

    /// Attempted a state transition that is not allowed by the state machine.
    #[error("Invalid bead state transition: {from} -> {to}")]
    BeadInvalidStateTransition { from: String, to: String },

    /// A dependency would create a cycle in the bead dependency graph.
    #[error("Dependency cycle detected: {0}")]
    BeadDependencyCycle(String),

    /// Bead cannot proceed because its dependencies are not satisfied.
    #[error("Bead is blocked by: [{0}]")]
    BeadBlockedBy(String),

    /// Referenced bead dependency does not exist.
    #[error("Invalid bead dependency: {0}")]
    BeadInvalidDependency(String),

    // ========================================================================
    // Queue Errors (2xxx)
    // ========================================================================
    /// Queue has no items to dequeue or peek.
    #[error("Queue is empty")]
    QueueEmpty,

    /// Queue item with the given identifier not found.
    #[error("Queue item not found: {0}")]
    QueueItemNotFound(String),

    /// Queue is locked by another agent for exclusive access.
    #[error("Queue is locked by '{0}'")]
    QueueLocked(String),

    /// A queue operation cannot proceed because another is already in progress.
    #[error("Queue operation already in progress")]
    QueueProcessing,

    /// Position index is out of range for the queue.
    #[error("Invalid queue position: {0}")]
    QueueInvalidPosition(usize),

    /// Queue has reached its maximum capacity.
    #[error("Queue is full (max: {0})")]
    QueueFull(usize),

    // ========================================================================
    // VCS Errors (3xxx)
    // ========================================================================
    /// No VCS repository is initialized in the current directory.
    #[error("VCS not initialized in this directory")]
    VcsNotInitialized,

    /// VCS merge or rebase conflict that requires resolution.
    #[error("VCS conflict in {0}: {1}")]
    VcsConflict(String, String),

    /// Git push to remote failed.
    #[error("Failed to push: {0}")]
    VcsPushFailed(String),

    /// Git pull from remote failed.
    #[error("Failed to pull: {0}")]
    VcsPullFailed(String),

    /// Git rebase failed.
    #[error("Failed to rebase: {0}")]
    VcsRebaseFailed(String),

    /// Git branch does not exist.
    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    /// Git branch already exists (duplicate creation attempt).
    #[error("Branch already exists: {0}")]
    BranchExists(String),

    /// Git commit does not exist.
    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    /// Working copy has uncommitted changes that block the operation.
    #[error("Working copy has uncommitted changes")]
    WorkingCopyDirty,

    // ========================================================================
    // Configuration Errors (4xxx)
    // ========================================================================
    /// Configuration file or key not found.
    #[error("Configuration not found: {0}")]
    ConfigNotFound(String),

    /// Configuration value fails validation or parsing.
    #[error("Configuration invalid: {0}")]
    ConfigInvalid(String),

    /// Insufficient permissions to read or write configuration.
    #[error("Configuration permission denied: {0}")]
    ConfigPermission(String),

    /// Configuration is structurally invalid.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Repository URL is malformed or unsupported.
    #[error("Invalid repository URL: {0}")]
    InvalidRepoUrl(String),

    // ========================================================================
    // Agent Errors (5xxx)
    // ========================================================================
    /// Agent with the given ID is not registered.
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Agent with the given ID is already registered.
    #[error("Agent already registered: {0}")]
    AgentExists(String),

    /// Agent heartbeat was not received within the timeout window.
    #[error("Agent '{0}' heartbeat timeout")]
    AgentTimeout(String),

    // ========================================================================
    // State/Conflict Errors (6xxx)
    // ========================================================================
    /// An operation was attempted in an invalid state.
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// A requested resource was not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// The requested operation is not valid for the current context.
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    // ========================================================================
    // Validation Errors (7xxx)
    // ========================================================================
    /// General input validation failure.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Validation failure on a specific field with structured context.
    #[error("Validation error on '{field}': {message}")]
    ValidationFieldError {
        message: String,
        field: String,
        value: Option<String>,
    },

    /// An identifier does not match the required format.
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),

    // ========================================================================
    // IO/Storage Errors (8xxx)
    // ========================================================================
    /// Filesystem I/O operation failed.
    #[error("IO error: {0}")]
    IoError(String),

    /// JSON deserialization failed.
    #[error("JSON parse error: {0}")]
    JsonParseError(String),

    /// YAML deserialization failed.
    #[error("YAML parse error: {0}")]
    YamlParseError(String),

    /// Database operation failed.
    #[error("Database error: {0}")]
    Database(String),

    /// Serialization of data to a wire or storage format failed.
    #[error("Serialization error: {0}")]
    Serialization(String),

    // ========================================================================
    // Orchestration/Workflow Errors (8xxx - extended)
    // ========================================================================
    /// Failed to acquire a lock within the timeout window.
    #[error("Lock acquisition timeout for '{operation}' after {timeout_ms}ms ({retries} retries)")]
    LockTimeout {
        operation: String,
        timeout_ms: u64,
        retries: usize,
    },

    /// Git clone operation failed.
    #[error("Clone failed: {0}")]
    CloneFailed(String),

    /// Recording an event or operation to the database failed.
    #[error("Record failed: {0}")]
    RecordFailed(String),

    /// Persisting state to disk or database failed.
    #[error("Persistence error: {0}")]
    Persistence(String),

    /// State machine transition was rejected.
    #[error("State transition error: {0}")]
    StateTransition(String),

    // ========================================================================
    // Scenario/Execution Errors (8xxx - extended)
    // ========================================================================
    /// Scenario definition or execution failed.
    #[error("Scenario error: {0}")]
    ScenarioError(String),

    /// Test runner or execution engine failed.
    #[error("Runner error: {0}")]
    RunnerError(String),

    /// Scenario or workflow definition is invalid.
    #[error("Definition error: {0}")]
    DefinitionError(String),

    /// Server-side operation failed.
    #[error("Server error: {0}")]
    ServerError(String),

    /// Synchronization between components failed.
    #[error("Sync error: {0}")]
    SyncError(String),

    // ========================================================================
    // Internal Errors (9xxx)
    // ========================================================================
    /// An unexpected internal error occurred.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Feature or code path is not yet implemented.
    #[error("Not implemented: {0}")]
    Unimplemented(String),

    /// A programming invariant was violated (indicates a bug).
    #[error("Invariant violation: {0}")]
    InvariantViolation(String),
}

impl Error {
    /// Returns a human-readable suggestion for fixing the error.
    ///
    /// Returns `None` for errors where no actionable suggestion exists.
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

    /// Returns a numeric exit code for CLI use.
    ///
    /// Codes follow the error category scheme: 1xxx workspace, 2xxx queue,
    /// 3xxx VCS, 4xxx config, 5xxx agent, 6xxx state, 7xxx validation,
    /// 8xxx IO/orchestration, 9xxx internal. Always non-zero.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self {
            // Workspace/Session: 10-18 (matches core error_workspace)
            Self::WorkspaceNotFound(_) => 10,
            Self::WorkspaceExists(_) => 11,
            Self::WorkspaceLocked(_, _) => 12,
            Self::WorkspaceConflict(_) => 13,
            Self::SessionNotFound(_) => 14,
            Self::SessionExists(_) => 15,
            Self::SessionLocked(_, _) => 16,
            Self::NotLockHolder(_, _) => 17,
            Self::SessionInvalidState(_, _, _) => 18,
            // Bead primary: 19, 27-29 (no core equivalent, placed in gaps)
            Self::BeadNotFound(_) => 19,
            Self::BeadAlreadyExists(_) => 26,
            Self::InvalidBeadId(_) => 27,
            Self::InvalidBeadTitle(_) => 28,
            Self::BeadInvalidStateTransition { .. } => 29,
            // Queue: 20-25 (matches core error_queue)
            Self::QueueEmpty => 20,
            Self::QueueItemNotFound(_) => 21,
            Self::QueueLocked(_) => 22,
            Self::QueueProcessing => 23,
            Self::QueueInvalidPosition(_) => 24,
            Self::QueueFull(_) => 25,
            // VCS: 30-38 (matches core error_vcs)
            Self::VcsNotInitialized => 30,
            Self::VcsConflict(_, _) => 31,
            Self::VcsPushFailed(_) => 32,
            Self::VcsPullFailed(_) => 33,
            Self::VcsRebaseFailed(_) => 34,
            Self::BranchNotFound(_) => 35,
            Self::BranchExists(_) => 36,
            Self::CommitNotFound(_) => 37,
            Self::WorkingCopyDirty => 38,
            // Config: 40-42 (matches core error_config)
            Self::ConfigNotFound(_) => 40,
            Self::ConfigInvalid(_) => 41,
            Self::ConfigPermission(_) => 42,
            // Agent: 50-52 (matches core error_agent)
            Self::AgentNotFound(_) => 50,
            Self::AgentExists(_) => 51,
            Self::AgentTimeout(_) => 52,
            // Bead extended: 66-68 (no core equivalent)
            Self::BeadDependencyCycle(_) => 66,
            Self::BeadBlockedBy(_) => 67,
            Self::BeadInvalidDependency(_) => 68,
            // IO/Storage: 60-63, 69 (matches core error_io, 69 for Serialization)
            Self::IoError(_) => 60,
            Self::JsonParseError(_) => 61,
            Self::YamlParseError(_) => 62,
            Self::Database(_) => 63,
            Self::Serialization(_) => 69,
            // State: 70-71 (matches core error_state)
            Self::InvalidState(_) => 70,
            Self::NotFound(_) => 71,
            // Orchestration: 72-74 (no core equivalent)
            Self::LockTimeout { .. } => 72,
            Self::Persistence(_) => 73,
            Self::StateTransition(_) => 74,
            // Scenario/Execution: 75-79 (no core equivalent)
            Self::ScenarioError(_) => 75,
            Self::RunnerError(_) => 76,
            Self::DefinitionError(_) => 77,
            Self::ServerError(_) => 78,
            Self::SyncError(_) => 79,
            // Validation: 80-82 (matches core error_state validation variants)
            Self::ValidationError(_) => 80,
            Self::ValidationFieldError { .. } => 81,
            Self::InvalidIdentifier(_) => 82,
            // Internal: 90-91, 93-97 (matches core error_internal + extensions)
            Self::Internal(_) => 90,
            Self::Unimplemented(_) => 91,
            Self::CloneFailed(_) => 93,
            Self::RecordFailed(_) => 94,
            Self::InvalidConfig(_) => 92,
            Self::InvalidRepoUrl(_) => 95,
            Self::InvalidOperation(_) => 96,
            Self::InvariantViolation(_) => 97,
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
        assert_eq!(Error::BeadAlreadyExists("x".into()).exit_code(), 26);
    }

    #[test]
    fn exit_codes_bead_extended() {
        assert_eq!(Error::InvalidBeadId("x".into()).exit_code(), 27);
        assert_eq!(Error::InvalidBeadTitle("x".into()).exit_code(), 28);
        assert_eq!(
            Error::BeadInvalidStateTransition {
                from: "a".into(),
                to: "b".into()
            }
            .exit_code(),
            29
        );
        assert_eq!(Error::BeadDependencyCycle("x".into()).exit_code(), 66);
        assert_eq!(Error::BeadBlockedBy("x".into()).exit_code(), 67);
        assert_eq!(Error::BeadInvalidDependency("x".into()).exit_code(), 68);
    }

    #[test]
    fn exit_codes_queue() {
        assert_eq!(Error::QueueEmpty.exit_code(), 20);
        assert_eq!(Error::QueueItemNotFound("x".into()).exit_code(), 21);
        assert_eq!(Error::QueueLocked("x".into()).exit_code(), 22);
        assert_eq!(Error::QueueProcessing.exit_code(), 23);
        assert_eq!(Error::QueueInvalidPosition(0).exit_code(), 24);
        assert_eq!(Error::QueueFull(10).exit_code(), 25);
    }

    #[test]
    fn exit_codes_vcs() {
        assert_eq!(Error::VcsNotInitialized.exit_code(), 30);
        assert_eq!(Error::VcsConflict("a".into(), "b".into()).exit_code(), 31);
        assert_eq!(Error::VcsPushFailed("x".into()).exit_code(), 32);
        assert_eq!(Error::VcsPullFailed("x".into()).exit_code(), 33);
        assert_eq!(Error::VcsRebaseFailed("x".into()).exit_code(), 34);
        assert_eq!(Error::BranchNotFound("x".into()).exit_code(), 35);
        assert_eq!(Error::BranchExists("x".into()).exit_code(), 36);
        assert_eq!(Error::CommitNotFound("x".into()).exit_code(), 37);
        assert_eq!(Error::WorkingCopyDirty.exit_code(), 38);
    }

    #[test]
    fn exit_codes_config() {
        assert_eq!(Error::ConfigNotFound("x".into()).exit_code(), 40);
        assert_eq!(Error::ConfigInvalid("x".into()).exit_code(), 41);
        assert_eq!(Error::ConfigPermission("x".into()).exit_code(), 42);
        assert_eq!(Error::InvalidConfig("x".into()).exit_code(), 92);
        assert_eq!(Error::InvalidRepoUrl("x".into()).exit_code(), 95);
    }

    #[test]
    fn exit_codes_agent() {
        assert_eq!(Error::AgentNotFound("x".into()).exit_code(), 50);
        assert_eq!(Error::AgentExists("x".into()).exit_code(), 51);
        assert_eq!(Error::AgentTimeout("x".into()).exit_code(), 52);
    }

    #[test]
    fn exit_codes_state_conflict() {
        assert_eq!(Error::InvalidState("x".into()).exit_code(), 70);
        assert_eq!(Error::NotFound("x".into()).exit_code(), 71);
        assert_eq!(Error::InvalidOperation("x".into()).exit_code(), 96);
    }

    #[test]
    fn exit_codes_validation() {
        assert_eq!(Error::ValidationError("x".into()).exit_code(), 80);
        assert_eq!(
            Error::ValidationFieldError {
                message: "x".into(),
                field: "y".into(),
                value: None,
            }
            .exit_code(),
            81
        );
        assert_eq!(Error::InvalidIdentifier("x".into()).exit_code(), 82);
    }

    #[test]
    fn exit_codes_io_storage() {
        assert_eq!(Error::IoError("x".into()).exit_code(), 60);
        assert_eq!(Error::JsonParseError("x".into()).exit_code(), 61);
        assert_eq!(Error::YamlParseError("x".into()).exit_code(), 62);
        assert_eq!(Error::Database("x".into()).exit_code(), 63);
        assert_eq!(Error::Serialization("x".into()).exit_code(), 69);
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
            72
        );
        assert_eq!(Error::CloneFailed("x".into()).exit_code(), 93);
        assert_eq!(Error::RecordFailed("x".into()).exit_code(), 94);
        assert_eq!(Error::Persistence("x".into()).exit_code(), 73);
        assert_eq!(Error::StateTransition("x".into()).exit_code(), 74);
    }

    #[test]
    fn exit_codes_scenario_execution() {
        assert_eq!(Error::ScenarioError("x".into()).exit_code(), 75);
        assert_eq!(Error::RunnerError("x".into()).exit_code(), 76);
        assert_eq!(Error::DefinitionError("x".into()).exit_code(), 77);
        assert_eq!(Error::ServerError("x".into()).exit_code(), 78);
        assert_eq!(Error::SyncError("x".into()).exit_code(), 79);
    }

    #[test]
    fn exit_codes_internal() {
        assert_eq!(Error::Internal("x".into()).exit_code(), 90);
        assert_eq!(Error::Unimplemented("x".into()).exit_code(), 91);
        assert_eq!(Error::InvariantViolation("x".into()).exit_code(), 97);
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
        assert_eq!(err.exit_code(), 72);
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
