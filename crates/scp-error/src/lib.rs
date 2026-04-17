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
