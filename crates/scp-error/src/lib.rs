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

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
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

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}

impl Error {
    /// Returns a SCREAMING_SNAKE_CASE machine-readable error code for this error.
    ///
    /// Useful for programmatic error handling, structured logging, and
    /// machine-readable output in CLI `--json` mode.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            // Workspace/Session (1xxx)
            Self::WorkspaceNotFound(_) => "WORKSPACE_NOT_FOUND",
            Self::WorkspaceExists(_) => "WORKSPACE_EXISTS",
            Self::WorkspaceLocked(_, _) => "WORKSPACE_LOCKED",
            Self::WorkspaceConflict(_) => "WORKSPACE_CONFLICT",
            Self::SessionNotFound(_) => "SESSION_NOT_FOUND",
            Self::SessionExists(_) => "SESSION_EXISTS",
            Self::SessionLocked(_, _) => "SESSION_LOCKED",
            Self::NotLockHolder(_, _) => "NOT_LOCK_HOLDER",
            Self::SessionInvalidState(_, _, _) => "SESSION_INVALID_STATE",
            // Bead (1xxx extended)
            Self::BeadNotFound(_) => "BEAD_NOT_FOUND",
            Self::BeadAlreadyExists(_) => "BEAD_ALREADY_EXISTS",
            Self::InvalidBeadId(_) => "INVALID_BEAD_ID",
            Self::InvalidBeadTitle(_) => "INVALID_BEAD_TITLE",
            Self::BeadInvalidStateTransition { .. } => "BEAD_INVALID_STATE_TRANSITION",
            Self::BeadDependencyCycle(_) => "BEAD_DEPENDENCY_CYCLE",
            Self::BeadBlockedBy(_) => "BEAD_BLOCKED_BY",
            Self::BeadInvalidDependency(_) => "BEAD_INVALID_DEPENDENCY",
            // Queue (2xxx)
            Self::QueueEmpty => "QUEUE_EMPTY",
            Self::QueueItemNotFound(_) => "QUEUE_ITEM_NOT_FOUND",
            Self::QueueLocked(_) => "QUEUE_LOCKED",
            Self::QueueProcessing => "QUEUE_PROCESSING",
            Self::QueueInvalidPosition(_) => "QUEUE_INVALID_POSITION",
            Self::QueueFull(_) => "QUEUE_FULL",
            // VCS (3xxx)
            Self::VcsNotInitialized => "VCS_NOT_INITIALIZED",
            Self::VcsConflict(_, _) => "VCS_CONFLICT",
            Self::VcsPushFailed(_) => "VCS_PUSH_FAILED",
            Self::VcsPullFailed(_) => "VCS_PULL_FAILED",
            Self::VcsRebaseFailed(_) => "VCS_REBASE_FAILED",
            Self::BranchNotFound(_) => "BRANCH_NOT_FOUND",
            Self::BranchExists(_) => "BRANCH_EXISTS",
            Self::CommitNotFound(_) => "COMMIT_NOT_FOUND",
            Self::WorkingCopyDirty => "WORKING_COPY_DIRTY",
            // Config (4xxx)
            Self::ConfigNotFound(_) => "CONFIG_NOT_FOUND",
            Self::ConfigInvalid(_) => "CONFIG_INVALID",
            Self::ConfigPermission(_) => "CONFIG_PERMISSION",
            Self::InvalidConfig(_) => "INVALID_CONFIG",
            Self::InvalidRepoUrl(_) => "INVALID_REPO_URL",
            // Agent (5xxx)
            Self::AgentNotFound(_) => "AGENT_NOT_FOUND",
            Self::AgentExists(_) => "AGENT_EXISTS",
            Self::AgentTimeout(_) => "AGENT_TIMEOUT",
            // State/Conflict (6xxx)
            Self::InvalidState(_) => "INVALID_STATE",
            Self::NotFound(_) => "NOT_FOUND",
            Self::InvalidOperation(_) => "INVALID_OPERATION",
            // Validation (7xxx)
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::ValidationFieldError { .. } => "VALIDATION_FIELD_ERROR",
            Self::InvalidIdentifier(_) => "INVALID_IDENTIFIER",
            // IO/Storage (8xxx)
            Self::IoError(_) => "IO_ERROR",
            Self::JsonParseError(_) => "JSON_PARSE_ERROR",
            Self::YamlParseError(_) => "YAML_PARSE_ERROR",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Serialization(_) => "SERIALIZATION_ERROR",
            // Orchestration/Workflow (8xxx extended)
            Self::LockTimeout { .. } => "LOCK_TIMEOUT",
            Self::CloneFailed(_) => "CLONE_FAILED",
            Self::RecordFailed(_) => "RECORD_FAILED",
            Self::Persistence(_) => "PERSISTENCE_ERROR",
            Self::StateTransition(_) => "STATE_TRANSITION_ERROR",
            // Scenario/Execution (8xxx extended)
            Self::ScenarioError(_) => "SCENARIO_ERROR",
            Self::RunnerError(_) => "RUNNER_ERROR",
            Self::DefinitionError(_) => "DEFINITION_ERROR",
            Self::ServerError(_) => "SERVER_ERROR",
            Self::SyncError(_) => "SYNC_ERROR",
            // Internal (9xxx)
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Unimplemented(_) => "NOT_IMPLEMENTED",
            Self::InvariantViolation(_) => "INVARIANT_VIOLATION",
        }
    }

    /// Returns structured context information for this error as a JSON value.
    ///
    /// Provides machine-readable context for AI agents and tooling to understand
    /// the error in detail. Each variant exposes its relevant fields.
    #[must_use]
    pub fn context_map(&self) -> Option<serde_json::Value> {
        match self {
            // Workspace/Session
            Self::WorkspaceNotFound(name) => Some(serde_json::json!({
                "resource_type": "workspace",
                "workspace_name": name,
            })),
            Self::WorkspaceExists(name) => Some(serde_json::json!({
                "resource_type": "workspace",
                "workspace_name": name,
            })),
            Self::WorkspaceLocked(name, holder) => Some(serde_json::json!({
                "workspace_name": name,
                "holder": holder,
            })),
            Self::WorkspaceConflict(msg) => Some(serde_json::json!({
                "message": msg,
            })),
            Self::SessionNotFound(name) => Some(serde_json::json!({
                "resource_type": "session",
                "session_name": name,
            })),
            Self::SessionExists(name) => Some(serde_json::json!({
                "resource_type": "session",
                "session_name": name,
            })),
            Self::SessionLocked(session, holder) => Some(serde_json::json!({
                "session": session,
                "holder": holder,
            })),
            Self::NotLockHolder(session, agent_id) => Some(serde_json::json!({
                "session": session,
                "agent_id": agent_id,
            })),
            Self::SessionInvalidState(session, actual, expected) => Some(serde_json::json!({
                "session": session,
                "actual_state": actual,
                "expected_state": expected,
            })),
            // Bead
            Self::BeadNotFound(id) => Some(serde_json::json!({
                "resource_type": "bead",
                "bead_id": id,
            })),
            Self::BeadAlreadyExists(id) => Some(serde_json::json!({
                "resource_type": "bead",
                "bead_id": id,
            })),
            Self::InvalidBeadId(id) => Some(serde_json::json!({
                "bead_id": id,
            })),
            Self::InvalidBeadTitle(title) => Some(serde_json::json!({
                "title": title,
            })),
            Self::BeadInvalidStateTransition { from, to } => Some(serde_json::json!({
                "from_state": from,
                "to_state": to,
            })),
            Self::BeadDependencyCycle(path) => Some(serde_json::json!({
                "cycle_path": path,
            })),
            Self::BeadBlockedBy(blockers) => Some(serde_json::json!({
                "blockers": blockers,
            })),
            Self::BeadInvalidDependency(dep) => Some(serde_json::json!({
                "dependency": dep,
            })),
            // Queue
            Self::QueueEmpty => Some(serde_json::json!({
                "error_type": "queue_empty",
            })),
            Self::QueueItemNotFound(item) => Some(serde_json::json!({
                "item": item,
            })),
            Self::QueueLocked(holder) => Some(serde_json::json!({
                "holder": holder,
            })),
            Self::QueueProcessing => Some(serde_json::json!({
                "error_type": "queue_processing",
            })),
            Self::QueueInvalidPosition(pos) => Some(serde_json::json!({
                "position": pos,
            })),
            Self::QueueFull(max) => Some(serde_json::json!({
                "max_size": max,
            })),
            // VCS
            Self::VcsNotInitialized => Some(serde_json::json!({
                "error_type": "vcs_not_initialized",
            })),
            Self::VcsConflict(repo, msg) => Some(serde_json::json!({
                "repo": repo,
                "message": msg,
            })),
            Self::VcsPushFailed(msg) => Some(serde_json::json!({
                "operation": "push",
                "error": msg,
            })),
            Self::VcsPullFailed(msg) => Some(serde_json::json!({
                "operation": "pull",
                "error": msg,
            })),
            Self::VcsRebaseFailed(msg) => Some(serde_json::json!({
                "operation": "rebase",
                "error": msg,
            })),
            Self::BranchNotFound(branch) => Some(serde_json::json!({
                "resource_type": "branch",
                "branch_name": branch,
            })),
            Self::BranchExists(branch) => Some(serde_json::json!({
                "resource_type": "branch",
                "branch_name": branch,
            })),
            Self::CommitNotFound(commit) => Some(serde_json::json!({
                "resource_type": "commit",
                "commit_id": commit,
            })),
            Self::WorkingCopyDirty => Some(serde_json::json!({
                "error_type": "working_copy_dirty",
            })),
            // Config
            Self::ConfigNotFound(key) => Some(serde_json::json!({
                "resource_type": "config",
                "key": key,
            })),
            Self::ConfigInvalid(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::ConfigPermission(path) => Some(serde_json::json!({
                "path": path,
            })),
            Self::InvalidConfig(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::InvalidRepoUrl(url) => Some(serde_json::json!({
                "url": url,
            })),
            // Agent
            Self::AgentNotFound(id) => Some(serde_json::json!({
                "resource_type": "agent",
                "agent_id": id,
            })),
            Self::AgentExists(id) => Some(serde_json::json!({
                "resource_type": "agent",
                "agent_id": id,
            })),
            Self::AgentTimeout(id) => Some(serde_json::json!({
                "agent_id": id,
            })),
            // State/Conflict
            Self::InvalidState(msg) => Some(serde_json::json!({
                "state": msg,
            })),
            Self::NotFound(resource) => Some(serde_json::json!({
                "resource": resource,
            })),
            Self::InvalidOperation(op) => Some(serde_json::json!({
                "operation": op,
            })),
            // Validation
            Self::ValidationError(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::ValidationFieldError {
                message,
                field,
                value,
            } => {
                let mut map = serde_json::json!({
                    "field": field,
                    "message": message,
                });
                if let Some(v) = value {
                    map["value"] = serde_json::json!(v);
                }
                Some(map)
            }
            Self::InvalidIdentifier(id) => Some(serde_json::json!({
                "identifier": id,
            })),
            // IO/Storage
            Self::IoError(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::JsonParseError(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::YamlParseError(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::Database(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::Serialization(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            // Orchestration/Workflow
            Self::LockTimeout {
                operation,
                timeout_ms,
                retries,
            } => Some(serde_json::json!({
                "operation": operation,
                "timeout_ms": timeout_ms,
                "retries": retries,
            })),
            Self::CloneFailed(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::RecordFailed(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::Persistence(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::StateTransition(msg) => Some(serde_json::json!({
                "transition": msg,
            })),
            // Scenario/Execution
            Self::ScenarioError(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::RunnerError(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::DefinitionError(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::ServerError(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::SyncError(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            // Internal
            Self::Internal(msg) => Some(serde_json::json!({
                "error": msg,
            })),
            Self::Unimplemented(feature) => Some(serde_json::json!({
                "feature": feature,
            })),
            Self::InvariantViolation(msg) => Some(serde_json::json!({
                "error": msg,
            })),
        }
    }

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

    // Helper to build one of every variant for exhaustive iteration.
    fn all_variants() -> Vec<Error> {
        vec![
            Error::WorkspaceNotFound("ws".into()),
            Error::WorkspaceExists("ws".into()),
            Error::WorkspaceLocked("ws".into(), "agent".into()),
            Error::WorkspaceConflict("msg".into()),
            Error::SessionNotFound("s".into()),
            Error::SessionExists("s".into()),
            Error::SessionLocked("s".into(), "agent".into()),
            Error::NotLockHolder("s".into(), "agent".into()),
            Error::SessionInvalidState("s".into(), "old".into(), "new".into()),
            Error::BeadNotFound("b".into()),
            Error::BeadAlreadyExists("b".into()),
            Error::InvalidBeadId("b".into()),
            Error::InvalidBeadTitle("".into()),
            Error::BeadInvalidStateTransition { from: "a".into(), to: "b".into() },
            Error::BeadDependencyCycle("b".into()),
            Error::BeadBlockedBy("b".into()),
            Error::BeadInvalidDependency("b".into()),
            Error::QueueEmpty,
            Error::QueueItemNotFound("q".into()),
            Error::QueueLocked("agent".into()),
            Error::QueueProcessing,
            Error::QueueInvalidPosition(99),
            Error::QueueFull(10),
            Error::VcsNotInitialized,
            Error::VcsConflict("file".into(), "msg".into()),
            Error::VcsPushFailed("msg".into()),
            Error::VcsPullFailed("msg".into()),
            Error::VcsRebaseFailed("msg".into()),
            Error::BranchNotFound("b".into()),
            Error::BranchExists("b".into()),
            Error::CommitNotFound("c".into()),
            Error::WorkingCopyDirty,
            Error::ConfigNotFound("k".into()),
            Error::ConfigInvalid("msg".into()),
            Error::ConfigPermission("k".into()),
            Error::InvalidConfig("msg".into()),
            Error::InvalidRepoUrl("url".into()),
            Error::AgentNotFound("a".into()),
            Error::AgentExists("a".into()),
            Error::AgentTimeout("a".into()),
            Error::InvalidState("msg".into()),
            Error::NotFound("res".into()),
            Error::InvalidOperation("op".into()),
            Error::ValidationError("msg".into()),
            Error::ValidationFieldError { message: "m".into(), field: "f".into(), value: Some("v".into()) },
            Error::InvalidIdentifier("id".into()),
            Error::IoError("msg".into()),
            Error::JsonParseError("msg".into()),
            Error::YamlParseError("msg".into()),
            Error::Database("msg".into()),
            Error::Serialization("msg".into()),
            Error::LockTimeout { operation: "op".into(), timeout_ms: 5000, retries: 3 },
            Error::CloneFailed("msg".into()),
            Error::RecordFailed("msg".into()),
            Error::Persistence("msg".into()),
            Error::StateTransition("msg".into()),
            Error::ScenarioError("msg".into()),
            Error::RunnerError("msg".into()),
            Error::DefinitionError("msg".into()),
            Error::ServerError("msg".into()),
            Error::SyncError("msg".into()),
            Error::Internal("msg".into()),
            Error::Unimplemented("feat".into()),
            Error::InvariantViolation("msg".into()),
        ]
    }

    // ── Display / Debug ──────────────────────────────────────────────────

    #[test]
    fn test_display_all_variants() {
        let variants = all_variants();
        for v in &variants {
            let display = v.to_string();
            assert!(!display.is_empty(), "Display should not be empty for {:?}", v);
        }
    }

    #[test]
    fn test_debug_all_variants() {
        let variants = all_variants();
        for v in &variants {
            let debug = format!("{:?}", v);
            assert!(!debug.is_empty(), "Debug should not be empty for {:?}", v);
        }
    }

    #[test]
    fn test_display_contains_key_fields() {
        assert!(Error::WorkspaceNotFound("my-ws".into()).to_string().contains("my-ws"));
        assert!(Error::WorkspaceLocked("ws".into(), "alice".into()).to_string().contains("alice"));
        assert!(Error::SessionInvalidState("s".into(), "open".into(), "closed".into())
            .to_string()
            .contains("open"));
        assert!(Error::BeadInvalidStateTransition { from: "a".into(), to: "b".into() }
            .to_string()
            .contains("a"));
    }

    // ── Suggestions ──────────────────────────────────────────────────────

    #[test]
    fn test_suggestion_bearing_variants() {
        let suggesters = [
            Error::WorkspaceNotFound("x".into()),
            Error::SessionNotFound("x".into()),
            Error::QueueEmpty,
            Error::WorkspaceLocked("x".into(), "holder".into()),
            Error::VcsNotInitialized,
            Error::WorkingCopyDirty,
        ];
        for err in &suggesters {
            assert!(err.suggestion().is_some(), "Expected suggestion for {:?}", err);
        }
    }

    #[test]
    fn test_no_suggestion_variants() {
        let non_suggesters = [
            Error::Internal("x".into()),
            Error::InvariantViolation("x".into()),
            Error::Unimplemented("x".into()),
            Error::BeadNotFound("x".into()),
            Error::QueueItemNotFound("x".into()),
            Error::AgentNotFound("x".into()),
            Error::ConfigNotFound("x".into()),
            Error::IoError("x".into()),
            Error::LockTimeout { operation: "op".into(), timeout_ms: 1000, retries: 0 },
        ];
        for err in &non_suggesters {
            assert!(err.suggestion().is_none(), "Expected no suggestion for {:?}", err);
        }
    }

    #[test]
    fn test_suggestion_workspace_locked_includes_holder() {
        let err = Error::WorkspaceLocked("ws".into(), "alice".into());
        let suggestion = err.suggestion().unwrap();
        assert!(suggestion.contains("alice"));
    }

    // ── Exit codes ───────────────────────────────────────────────────────

    #[test]
    fn test_exit_codes_all_nonzero() {
        for v in all_variants() {
            assert!(v.exit_code() > 0, "Exit code must be nonzero for {:?}", v);
        }
    }

    #[test]
    fn test_exit_codes_unique() {
        let variants = all_variants();
        let codes: Vec<i32> = variants.iter().map(|v| v.exit_code()).collect();
        let mut sorted = codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(codes.len(), sorted.len(), "Exit codes must be unique — found duplicates");
    }

    #[test]
    fn test_exit_codes_match_documented_block() {
        // Each error group has a documented comment block (e.g. "Workspace/Session Errors (1xxx)").
        // The actual codes use compact blocks: workspace 10-18, beads 19-20+,
        // queue 30-35, vcs 40-48, config 60-64, agent 70-72, state 80-82,
        // validation 90-92, io/storage 100-105, orchestration 110-114,
        // scenario 120-124, internal 130-138.
        // Bead codes (133-138) share the 13x range with internal codes.
        // We verify each variant's code is in the correct documented block.
        for v in all_variants() {
            let code = v.exit_code();
            let (lo, hi, name) = match &v {
                Error::WorkspaceNotFound(_) | Error::WorkspaceExists(_)
                | Error::WorkspaceLocked(_, _) | Error::WorkspaceConflict(_)
                | Error::SessionNotFound(_) | Error::SessionExists(_)
                | Error::SessionLocked(_, _) | Error::NotLockHolder(_, _)
                | Error::SessionInvalidState(_, _, _) => (10, 19, "workspace/session"),
                Error::BeadNotFound(_) | Error::BeadAlreadyExists(_) => (19, 21, "bead"),
                Error::InvalidBeadId(_) | Error::InvalidBeadTitle(_)
                | Error::BeadInvalidStateTransition { .. }
                | Error::BeadDependencyCycle(_) | Error::BeadBlockedBy(_)
                | Error::BeadInvalidDependency(_) => (133, 139, "bead-extended"),
                Error::QueueEmpty | Error::QueueItemNotFound(_)
                | Error::QueueLocked(_) | Error::QueueProcessing
                | Error::QueueInvalidPosition(_) | Error::QueueFull(_) => (30, 36, "queue"),
                Error::VcsNotInitialized | Error::VcsConflict(_, _)
                | Error::VcsPushFailed(_) | Error::VcsPullFailed(_)
                | Error::VcsRebaseFailed(_) | Error::BranchNotFound(_)
                | Error::BranchExists(_) | Error::CommitNotFound(_)
                | Error::WorkingCopyDirty => (40, 49, "vcs"),
                Error::ConfigNotFound(_) | Error::ConfigInvalid(_)
                | Error::ConfigPermission(_) | Error::InvalidConfig(_)
                | Error::InvalidRepoUrl(_) => (60, 65, "config"),
                Error::AgentNotFound(_) | Error::AgentExists(_)
                | Error::AgentTimeout(_) => (70, 73, "agent"),
                Error::InvalidState(_) | Error::NotFound(_)
                | Error::InvalidOperation(_) => (80, 83, "state/conflict"),
                Error::ValidationError(_) | Error::ValidationFieldError { .. }
                | Error::InvalidIdentifier(_) => (90, 93, "validation"),
                Error::IoError(_) | Error::JsonParseError(_)
                | Error::YamlParseError(_) | Error::Database(_)
                | Error::Serialization(_) => (100, 106, "io/storage"),
                Error::LockTimeout { .. } | Error::CloneFailed(_)
                | Error::RecordFailed(_) | Error::Persistence(_)
                | Error::StateTransition(_) => (110, 115, "orchestration"),
                Error::ScenarioError(_) | Error::RunnerError(_)
                | Error::DefinitionError(_) | Error::ServerError(_)
                | Error::SyncError(_) => (120, 125, "scenario"),
                Error::Internal(_) | Error::Unimplemented(_)
                | Error::InvariantViolation(_) => (130, 133, "internal"),
            };
            assert!(
                code >= lo && code < hi,
                "Exit code {code} for {:?} ({name}) outside range {lo}-{hi}",
                v
            );
        }
    }

    #[test]
    fn test_exit_codes_spot_checks() {
        assert_eq!(Error::WorkspaceNotFound("x".into()).exit_code(), 10);
        assert_eq!(Error::QueueEmpty.exit_code(), 30);
        assert_eq!(Error::VcsNotInitialized.exit_code(), 40);
        assert_eq!(Error::ConfigNotFound("x".into()).exit_code(), 60);
        assert_eq!(Error::AgentNotFound("x".into()).exit_code(), 70);
        assert_eq!(Error::IoError("x".into()).exit_code(), 100);
        assert_eq!(Error::Internal("x".into()).exit_code(), 130);
    }

    // ── Serialization ────────────────────────────────────────────────────

    #[test]
    fn test_serialize_all_variants_produces_valid_json() {
        for v in all_variants() {
            let json = serde_json::to_string(&v).unwrap_or_else(|e| panic!("Serialize failed for {:?}: {e}", v));
            let _: serde_json::Value = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("Invalid JSON for {:?}: {e}\nJSON: {json}", v));
        }
    }

    #[test]
    fn test_serialize_struct_variants_includes_fields() {
        let err = Error::BeadInvalidStateTransition { from: "open".into(), to: "closed".into() };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("open"));
        assert!(json.contains("closed"));

        let err = Error::ValidationFieldError {
            message: "required".into(),
            field: "title".into(),
            value: Some("".into()),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("title"));
        assert!(json.contains("required"));

        let err = Error::LockTimeout { operation: "write".into(), timeout_ms: 5000, retries: 3 };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("write"));
        assert!(json.contains("5000"));
    }

    // ── Trait bounds ─────────────────────────────────────────────────────

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Error>();
    }

    #[test]
    fn test_error_implements_std_error() {
        fn assert_std_error<T: std::error::Error>() {}
        assert_std_error::<Error>();
    }

    #[test]
    fn test_error_source_is_none() {
        // All variants are plain data — no wrapped std::error::Error sources.
        for v in all_variants() {
            assert!(std::error::Error::source(&v).is_none(), "Expected no source for {:?}", v);
        }
    }
}
