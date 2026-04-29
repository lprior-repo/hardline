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
//!
//! # Error Code Ranges (ADR-007)
//!
//! | Range | Category     | Description                          |
//! |-------|-------------|--------------------------------------|
//! | 1xxx  | Workspace    | Workspace creation, management, state |
//! | 2xxx  | Session      | Session lifecycle, bead claiming       |
//! | 3xxx  | Bead         | Task/bead operations, dependencies     |
//! | 4xxx  | Queue        | Queue management, priority, ordering   |
//! | 5xxx  | VCS          | Git operations, conflicts             |
//! | 6xxx  | Stack        | Stacked PRs, branch stacks            |
//! | 7xxx  | GitHub       | GitHub API, PRs, CI status            |
//! | 8xxx  | Snapshot     | Backup/restore, checkpoints           |
//! | 9xxx  | Internal     | System errors, database, infra        |

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

// ========================================================================
// Error Category (ADR-007)
// ========================================================================

/// Categorizes errors into subsystem-level groups per ADR-007.
///
/// Each category maps to a numeric range (e.g. Workspace = 1xxx).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorCategory {
    /// Workspace creation, management, state (1xxx)
    Workspace,
    /// Session lifecycle, bead claiming (2xxx)
    Session,
    /// Task/bead operations, dependencies (3xxx)
    Bead,
    /// Queue management, priority, ordering (4xxx)
    Queue,
    /// Git operations, conflicts (5xxx)
    Vcs,
    /// Stacked PRs, branch stacks (6xxx)
    Stack,
    /// GitHub API, PRs, CI status (7xxx)
    GitHub,
    /// Backup/restore, checkpoints (8xxx)
    Snapshot,
    /// System errors, database, infrastructure (9xxx)
    Internal,
}

impl ErrorCategory {
    /// Returns the base numeric range for this category (e.g. 1000 for Workspace).
    #[must_use]
    pub const fn base(&self) -> u16 {
        match self {
            Self::Workspace => 1000,
            Self::Session => 2000,
            Self::Bead => 3000,
            Self::Queue => 4000,
            Self::Vcs => 5000,
            Self::Stack => 6000,
            Self::GitHub => 7000,
            Self::Snapshot => 8000,
            Self::Internal => 9000,
        }
    }

    /// Returns the inclusive upper bound for this category (e.g. 1999 for Workspace).
    #[must_use]
    pub const fn max(&self) -> u16 {
        match self {
            Self::Workspace => 1999,
            Self::Session => 2999,
            Self::Bead => 3999,
            Self::Queue => 4999,
            Self::Vcs => 5999,
            Self::Stack => 6999,
            Self::GitHub => 7999,
            Self::Snapshot => 8999,
            Self::Internal => 9999,
        }
    }
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Workspace => write!(f, "workspace"),
            Self::Session => write!(f, "session"),
            Self::Bead => write!(f, "bead"),
            Self::Queue => write!(f, "queue"),
            Self::Vcs => write!(f, "vcs"),
            Self::Stack => write!(f, "stack"),
            Self::GitHub => write!(f, "github"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Internal => write!(f, "internal"),
        }
    }
}

// ========================================================================
// Fix Suggestion (ADR-007)
// ========================================================================

/// Risk level of a suggested fix command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixRisk {
    /// Read-only or easily reversible.
    Safe,
    /// Modifies state but recoverable.
    Moderate,
    /// Potentially destructive.
    Dangerous,
}

impl std::fmt::Display for FixRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Safe => write!(f, "safe"),
            Self::Moderate => write!(f, "moderate"),
            Self::Dangerous => write!(f, "dangerous"),
        }
    }
}

/// A suggested fix for an error, with a command the user can run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorFix {
    /// Shell command the user can run to resolve the error.
    pub command: String,
    /// Human-readable description of what the fix does.
    pub description: String,
    /// Risk level of running this fix.
    pub risk: FixRisk,
}

impl ErrorFix {
    /// Creates a new fix suggestion.
    pub fn new(
        command: impl Into<String>,
        description: impl Into<String>,
        risk: FixRisk,
    ) -> Self {
        Self {
            command: command.into(),
            description: description.into(),
            risk,
        }
    }

    /// Convenience: creates a safe fix suggestion.
    pub fn safe(command: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(command, description, FixRisk::Safe)
    }
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Error {
    // ========================================================================
    // WORKSPACE ERRORS (1xxx)
    // ========================================================================
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Workspace already exists: {0}")]
    WorkspaceExists(String),

    #[error("Workspace '{0}' is locked by '{1}'")]
    WorkspaceLocked(String, String),

    #[error("Workspace conflict: {0}")]
    WorkspaceConflict(String),

    // ========================================================================
    // SESSION ERRORS (2xxx)
    // ========================================================================
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
    // BEAD ERRORS (3xxx)
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
    // QUEUE ERRORS (4xxx)
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
    // VCS ERRORS (5xxx)
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
    // STACK ERRORS (6xxx)
    // ========================================================================
    #[error("Stack not found: {0}")]
    StackNotFound(String),

    #[error("Stack orphaned: parent {0} not found")]
    StackOrphaned(String),

    #[error("Stack cyclic dependency detected")]
    StackCyclicDependency,

    #[error("Stack in invalid state: {0}")]
    StackInvalidState(String),

    #[error("Stack PR not found: {0}")]
    StackPrNotFound(String),

    // ========================================================================
    // GITHUB ERRORS (7xxx)
    // ========================================================================
    #[error("GitHub authentication failed: {0}")]
    GitHubAuthFailed(String),

    #[error("GitHub token expired")]
    GitHubTokenExpired,

    #[error("GitHub rate limited: retry after {0}")]
    GitHubRateLimited(String),

    #[error("GitHub PR closed: {0}")]
    GitHubPrClosed(String),

    #[error("GitHub PR not found: {0}")]
    GitHubPrNotFound(String),

    #[error("GitHub API error: {status} - {message}")]
    GitHubApiError { status: u16, message: String },

    #[error("GitHub CI status check failed: {0:?}")]
    GitHubCiFailed(Vec<String>),

    // ========================================================================
    // SNAPSHOT ERRORS (8xxx)
    // ========================================================================
    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),

    #[error("Snapshot corrupted: {0}")]
    SnapshotCorrupted(String),

    #[error("Snapshot expired: {0}")]
    SnapshotExpired(String),

    #[error("Snapshot limit exceeded: {0}")]
    SnapshotLimitExceeded(String),

    #[error("Snapshot restore failed: {0}")]
    SnapshotRestoreFailed(String),

    // ========================================================================
    // CONFIGURATION ERRORS (9xxx - infrastructure)
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
    // AGENT ERRORS (9xxx - infrastructure)
    // ========================================================================
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Agent already registered: {0}")]
    AgentExists(String),

    #[error("Agent '{0}' heartbeat timeout")]
    AgentTimeout(String),

    // ========================================================================
    // STATE/CONFLICT ERRORS (9xxx - infrastructure)
    // ========================================================================
    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    // ========================================================================
    // VALIDATION ERRORS (9xxx - infrastructure)
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
    // IO/STORAGE ERRORS (9xxx - infrastructure)
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
    // ORCHESTRATION/WORKFLOW ERRORS (9xxx - infrastructure)
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
    // SCENARIO/EXECUTION ERRORS (9xxx - infrastructure)
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
    // INTERNAL ERRORS (9xxx)
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
    /// Returns a `SCREAMING_SNAKE_CASE` machine-readable error code for this error.
    ///
    /// Useful for programmatic error handling, structured logging, and
    /// machine-readable output in CLI `--json` mode.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::WorkspaceNotFound(_)
            | Self::WorkspaceExists(_)
            | Self::WorkspaceLocked(_, _)
            | Self::WorkspaceConflict(_) => self.code_workspace(),
            Self::SessionNotFound(_)
            | Self::SessionExists(_)
            | Self::SessionLocked(_, _)
            | Self::NotLockHolder(_, _)
            | Self::SessionInvalidState(_, _, _) => self.code_session(),
            Self::BeadNotFound(_)
            | Self::BeadAlreadyExists(_)
            | Self::InvalidBeadId(_)
            | Self::InvalidBeadTitle(_)
            | Self::BeadInvalidStateTransition { .. }
            | Self::BeadDependencyCycle(_)
            | Self::BeadBlockedBy(_)
            | Self::BeadInvalidDependency(_) => self.code_bead(),
            Self::QueueEmpty
            | Self::QueueItemNotFound(_)
            | Self::QueueLocked(_)
            | Self::QueueProcessing
            | Self::QueueInvalidPosition(_)
            | Self::QueueFull(_) => self.code_queue(),
            Self::VcsNotInitialized
            | Self::VcsConflict(_, _)
            | Self::VcsPushFailed(_)
            | Self::VcsPullFailed(_)
            | Self::VcsRebaseFailed(_)
            | Self::BranchNotFound(_)
            | Self::BranchExists(_)
            | Self::CommitNotFound(_)
            | Self::WorkingCopyDirty => self.code_vcs(),
            Self::StackNotFound(_)
            | Self::StackOrphaned(_)
            | Self::StackCyclicDependency
            | Self::StackInvalidState(_)
            | Self::StackPrNotFound(_) => self.code_stack(),
            Self::GitHubAuthFailed(_)
            | Self::GitHubTokenExpired
            | Self::GitHubRateLimited(_)
            | Self::GitHubPrClosed(_)
            | Self::GitHubPrNotFound(_)
            | Self::GitHubApiError { .. }
            | Self::GitHubCiFailed(_) => self.code_github(),
            Self::SnapshotNotFound(_)
            | Self::SnapshotCorrupted(_)
            | Self::SnapshotExpired(_)
            | Self::SnapshotLimitExceeded(_)
            | Self::SnapshotRestoreFailed(_) => self.code_snapshot(),
            _ => self.code_infrastructure(),
        }
    }

    #[allow(clippy::panic)]
    const fn code_workspace(&self) -> &'static str {
        match self {
            Self::WorkspaceNotFound(_) => "WORKSPACE_NOT_FOUND",
            Self::WorkspaceExists(_) => "WORKSPACE_EXISTS",
            Self::WorkspaceLocked(_, _) => "WORKSPACE_LOCKED",
            Self::WorkspaceConflict(_) => "WORKSPACE_CONFLICT",
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn code_session(&self) -> &'static str {
        match self {
            Self::SessionNotFound(_) => "SESSION_NOT_FOUND",
            Self::SessionExists(_) => "SESSION_EXISTS",
            Self::SessionLocked(_, _) => "SESSION_LOCKED",
            Self::NotLockHolder(_, _) => "NOT_LOCK_HOLDER",
            Self::SessionInvalidState(_, _, _) => "SESSION_INVALID_STATE",
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn code_bead(&self) -> &'static str {
        match self {
            Self::BeadNotFound(_) => "BEAD_NOT_FOUND",
            Self::BeadAlreadyExists(_) => "BEAD_ALREADY_EXISTS",
            Self::InvalidBeadId(_) => "INVALID_BEAD_ID",
            Self::InvalidBeadTitle(_) => "INVALID_BEAD_TITLE",
            Self::BeadInvalidStateTransition { .. } => "BEAD_INVALID_STATE_TRANSITION",
            Self::BeadDependencyCycle(_) => "BEAD_DEPENDENCY_CYCLE",
            Self::BeadBlockedBy(_) => "BEAD_BLOCKED_BY",
            Self::BeadInvalidDependency(_) => "BEAD_INVALID_DEPENDENCY",
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn code_queue(&self) -> &'static str {
        match self {
            Self::QueueEmpty => "QUEUE_EMPTY",
            Self::QueueItemNotFound(_) => "QUEUE_ITEM_NOT_FOUND",
            Self::QueueLocked(_) => "QUEUE_LOCKED",
            Self::QueueProcessing => "QUEUE_PROCESSING",
            Self::QueueInvalidPosition(_) => "QUEUE_INVALID_POSITION",
            Self::QueueFull(_) => "QUEUE_FULL",
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn code_vcs(&self) -> &'static str {
        match self {
            Self::VcsNotInitialized => "VCS_NOT_INITIALIZED",
            Self::VcsConflict(_, _) => "VCS_CONFLICT",
            Self::VcsPushFailed(_) => "VCS_PUSH_FAILED",
            Self::VcsPullFailed(_) => "VCS_PULL_FAILED",
            Self::VcsRebaseFailed(_) => "VCS_REBASE_FAILED",
            Self::BranchNotFound(_) => "BRANCH_NOT_FOUND",
            Self::BranchExists(_) => "BRANCH_EXISTS",
            Self::CommitNotFound(_) => "COMMIT_NOT_FOUND",
            Self::WorkingCopyDirty => "WORKING_COPY_DIRTY",
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn code_stack(&self) -> &'static str {
        match self {
            Self::StackNotFound(_) => "STACK_NOT_FOUND",
            Self::StackOrphaned(_) => "STACK_ORPHANED",
            Self::StackCyclicDependency => "STACK_CYCLIC_DEPENDENCY",
            Self::StackInvalidState(_) => "STACK_INVALID_STATE",
            Self::StackPrNotFound(_) => "STACK_PR_NOT_FOUND",
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn code_github(&self) -> &'static str {
        match self {
            Self::GitHubAuthFailed(_) => "GITHUB_AUTH_FAILED",
            Self::GitHubTokenExpired => "GITHUB_TOKEN_EXPIRED",
            Self::GitHubRateLimited(_) => "GITHUB_RATE_LIMITED",
            Self::GitHubPrClosed(_) => "GITHUB_PR_CLOSED",
            Self::GitHubPrNotFound(_) => "GITHUB_PR_NOT_FOUND",
            Self::GitHubApiError { .. } => "GITHUB_API_ERROR",
            Self::GitHubCiFailed(_) => "GITHUB_CI_FAILED",
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn code_snapshot(&self) -> &'static str {
        match self {
            Self::SnapshotNotFound(_) => "SNAPSHOT_NOT_FOUND",
            Self::SnapshotCorrupted(_) => "SNAPSHOT_CORRUPTED",
            Self::SnapshotExpired(_) => "SNAPSHOT_EXPIRED",
            Self::SnapshotLimitExceeded(_) => "SNAPSHOT_LIMIT_EXCEEDED",
            Self::SnapshotRestoreFailed(_) => "SNAPSHOT_RESTORE_FAILED",
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn code_infrastructure(&self) -> &'static str {
        match self {
            Self::ConfigNotFound(_) => "CONFIG_NOT_FOUND",
            Self::ConfigInvalid(_) => "CONFIG_INVALID",
            Self::ConfigPermission(_) => "CONFIG_PERMISSION",
            Self::InvalidConfig(_) => "INVALID_CONFIG",
            Self::InvalidRepoUrl(_) => "INVALID_REPO_URL",
            Self::AgentNotFound(_) => "AGENT_NOT_FOUND",
            Self::AgentExists(_) => "AGENT_EXISTS",
            Self::AgentTimeout(_) => "AGENT_TIMEOUT",
            Self::InvalidState(_) => "INVALID_STATE",
            Self::NotFound(_) => "NOT_FOUND",
            Self::InvalidOperation(_) => "INVALID_OPERATION",
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::ValidationFieldError { .. } => "VALIDATION_FIELD_ERROR",
            Self::InvalidIdentifier(_) => "INVALID_IDENTIFIER",
            Self::IoError(_) => "IO_ERROR",
            Self::JsonParseError(_) => "JSON_PARSE_ERROR",
            Self::YamlParseError(_) => "YAML_PARSE_ERROR",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Serialization(_) => "SERIALIZATION_ERROR",
            Self::LockTimeout { .. } => "LOCK_TIMEOUT",
            Self::CloneFailed(_) => "CLONE_FAILED",
            Self::RecordFailed(_) => "RECORD_FAILED",
            Self::Persistence(_) => "PERSISTENCE_ERROR",
            Self::StateTransition(_) => "STATE_TRANSITION_ERROR",
            Self::ScenarioError(_) => "SCENARIO_ERROR",
            Self::RunnerError(_) => "RUNNER_ERROR",
            Self::DefinitionError(_) => "DEFINITION_ERROR",
            Self::ServerError(_) => "SERVER_ERROR",
            Self::SyncError(_) => "SYNC_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Unimplemented(_) => "NOT_IMPLEMENTED",
            Self::InvariantViolation(_) => "INVARIANT_VIOLATION",
            _ => panic!("unhandled variant"),
        }
    }

    /// Returns the hierarchical numeric error code per ADR-007.
    ///
    /// Code ranges:
    /// - 1xxx: Workspace errors
    /// - 2xxx: Session errors
    /// - 3xxx: Bead errors
    /// - 4xxx: Queue errors
    /// - 5xxx: VCS errors
    /// - 6xxx: Stack errors
    /// - 7xxx: GitHub errors
    /// - 8xxx: Snapshot errors
    /// - 9xxx: Internal/infrastructure errors
    #[must_use]
    pub const fn numeric_code(&self) -> u16 {
        match self {
            Self::WorkspaceNotFound(_)
            | Self::WorkspaceExists(_)
            | Self::WorkspaceLocked(_, _)
            | Self::WorkspaceConflict(_) => self.numeric_code_workspace(),
            Self::SessionNotFound(_)
            | Self::SessionExists(_)
            | Self::SessionLocked(_, _)
            | Self::NotLockHolder(_, _)
            | Self::SessionInvalidState(_, _, _) => self.numeric_code_session(),
            Self::BeadNotFound(_)
            | Self::BeadAlreadyExists(_)
            | Self::InvalidBeadId(_)
            | Self::InvalidBeadTitle(_)
            | Self::BeadInvalidStateTransition { .. }
            | Self::BeadDependencyCycle(_)
            | Self::BeadBlockedBy(_)
            | Self::BeadInvalidDependency(_) => self.numeric_code_bead(),
            Self::QueueEmpty
            | Self::QueueItemNotFound(_)
            | Self::QueueLocked(_)
            | Self::QueueProcessing
            | Self::QueueInvalidPosition(_)
            | Self::QueueFull(_) => self.numeric_code_queue(),
            Self::VcsNotInitialized
            | Self::VcsConflict(_, _)
            | Self::VcsPushFailed(_)
            | Self::VcsPullFailed(_)
            | Self::VcsRebaseFailed(_)
            | Self::BranchNotFound(_)
            | Self::BranchExists(_)
            | Self::CommitNotFound(_)
            | Self::WorkingCopyDirty => self.numeric_code_vcs(),
            Self::StackNotFound(_)
            | Self::StackOrphaned(_)
            | Self::StackCyclicDependency
            | Self::StackInvalidState(_)
            | Self::StackPrNotFound(_) => self.numeric_code_stack(),
            Self::GitHubAuthFailed(_)
            | Self::GitHubTokenExpired
            | Self::GitHubRateLimited(_)
            | Self::GitHubPrClosed(_)
            | Self::GitHubPrNotFound(_)
            | Self::GitHubApiError { .. }
            | Self::GitHubCiFailed(_) => self.numeric_code_github(),
            Self::SnapshotNotFound(_)
            | Self::SnapshotCorrupted(_)
            | Self::SnapshotExpired(_)
            | Self::SnapshotLimitExceeded(_)
            | Self::SnapshotRestoreFailed(_) => self.numeric_code_snapshot(),
            _ => self.numeric_code_infrastructure(),
        }
    }

    #[allow(clippy::panic)]
    const fn numeric_code_workspace(&self) -> u16 {
        match self {
            Self::WorkspaceNotFound(_) => 1001,
            Self::WorkspaceExists(_) => 1002,
            Self::WorkspaceLocked(_, _) => 1003,
            Self::WorkspaceConflict(_) => 1004,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn numeric_code_session(&self) -> u16 {
        match self {
            Self::SessionNotFound(_) => 2001,
            Self::SessionExists(_) => 2002,
            Self::SessionLocked(_, _) => 2003,
            Self::NotLockHolder(_, _) => 2004,
            Self::SessionInvalidState(_, _, _) => 2005,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn numeric_code_bead(&self) -> u16 {
        match self {
            Self::BeadNotFound(_) => 3001,
            Self::BeadAlreadyExists(_) => 3002,
            Self::InvalidBeadId(_) => 3003,
            Self::InvalidBeadTitle(_) => 3004,
            Self::BeadInvalidStateTransition { .. } => 3005,
            Self::BeadDependencyCycle(_) => 3006,
            Self::BeadBlockedBy(_) => 3007,
            Self::BeadInvalidDependency(_) => 3008,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn numeric_code_queue(&self) -> u16 {
        match self {
            Self::QueueEmpty => 4001,
            Self::QueueItemNotFound(_) => 4002,
            Self::QueueLocked(_) => 4003,
            Self::QueueProcessing => 4004,
            Self::QueueInvalidPosition(_) => 4005,
            Self::QueueFull(_) => 4006,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn numeric_code_vcs(&self) -> u16 {
        match self {
            Self::VcsNotInitialized => 5001,
            Self::VcsConflict(_, _) => 5002,
            Self::VcsPushFailed(_) => 5003,
            Self::VcsPullFailed(_) => 5004,
            Self::VcsRebaseFailed(_) => 5005,
            Self::BranchNotFound(_) => 5006,
            Self::BranchExists(_) => 5007,
            Self::CommitNotFound(_) => 5008,
            Self::WorkingCopyDirty => 5009,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn numeric_code_stack(&self) -> u16 {
        match self {
            Self::StackNotFound(_) => 6001,
            Self::StackOrphaned(_) => 6002,
            Self::StackCyclicDependency => 6003,
            Self::StackInvalidState(_) => 6004,
            Self::StackPrNotFound(_) => 6005,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn numeric_code_github(&self) -> u16 {
        match self {
            Self::GitHubAuthFailed(_) => 7001,
            Self::GitHubTokenExpired => 7002,
            Self::GitHubRateLimited(_) => 7003,
            Self::GitHubPrClosed(_) => 7004,
            Self::GitHubPrNotFound(_) => 7005,
            Self::GitHubApiError { .. } => 7006,
            Self::GitHubCiFailed(_) => 7007,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn numeric_code_snapshot(&self) -> u16 {
        match self {
            Self::SnapshotNotFound(_) => 8001,
            Self::SnapshotCorrupted(_) => 8002,
            Self::SnapshotExpired(_) => 8003,
            Self::SnapshotLimitExceeded(_) => 8004,
            Self::SnapshotRestoreFailed(_) => 8005,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn numeric_code_infrastructure(&self) -> u16 {
        match self {
            Self::ConfigNotFound(_) => 9101,
            Self::ConfigInvalid(_) => 9102,
            Self::ConfigPermission(_) => 9103,
            Self::InvalidConfig(_) => 9104,
            Self::InvalidRepoUrl(_) => 9105,
            Self::AgentNotFound(_) => 9201,
            Self::AgentExists(_) => 9202,
            Self::AgentTimeout(_) => 9203,
            Self::InvalidState(_) => 9301,
            Self::NotFound(_) => 9302,
            Self::InvalidOperation(_) => 9303,
            Self::ValidationError(_) => 9401,
            Self::ValidationFieldError { .. } => 9402,
            Self::InvalidIdentifier(_) => 9403,
            Self::IoError(_) => 9501,
            Self::JsonParseError(_) => 9502,
            Self::YamlParseError(_) => 9503,
            Self::Database(_) => 9504,
            Self::Serialization(_) => 9505,
            Self::LockTimeout { .. } => 9601,
            Self::CloneFailed(_) => 9602,
            Self::RecordFailed(_) => 9603,
            Self::Persistence(_) => 9604,
            Self::StateTransition(_) => 9605,
            Self::ScenarioError(_) => 9701,
            Self::RunnerError(_) => 9702,
            Self::DefinitionError(_) => 9703,
            Self::ServerError(_) => 9704,
            Self::SyncError(_) => 9705,
            Self::Internal(_) => 9001,
            Self::Unimplemented(_) => 9002,
            Self::InvariantViolation(_) => 9003,
            _ => panic!("unhandled variant"),
        }
    }

    /// Returns the error category per ADR-007.
    #[must_use]
    pub const fn category(&self) -> ErrorCategory {
        match self {
            Self::WorkspaceNotFound(_)
            | Self::WorkspaceExists(_)
            | Self::WorkspaceLocked(_, _)
            | Self::WorkspaceConflict(_) => ErrorCategory::Workspace,

            Self::SessionNotFound(_)
            | Self::SessionExists(_)
            | Self::SessionLocked(_, _)
            | Self::NotLockHolder(_, _)
            | Self::SessionInvalidState(_, _, _) => ErrorCategory::Session,

            Self::BeadNotFound(_)
            | Self::BeadAlreadyExists(_)
            | Self::InvalidBeadId(_)
            | Self::InvalidBeadTitle(_)
            | Self::BeadInvalidStateTransition { .. }
            | Self::BeadDependencyCycle(_)
            | Self::BeadBlockedBy(_)
            | Self::BeadInvalidDependency(_) => ErrorCategory::Bead,

            Self::QueueEmpty
            | Self::QueueItemNotFound(_)
            | Self::QueueLocked(_)
            | Self::QueueProcessing
            | Self::QueueInvalidPosition(_)
            | Self::QueueFull(_) => ErrorCategory::Queue,

            Self::VcsNotInitialized
            | Self::VcsConflict(_, _)
            | Self::VcsPushFailed(_)
            | Self::VcsPullFailed(_)
            | Self::VcsRebaseFailed(_)
            | Self::BranchNotFound(_)
            | Self::BranchExists(_)
            | Self::CommitNotFound(_)
            | Self::WorkingCopyDirty => ErrorCategory::Vcs,

            Self::StackNotFound(_)
            | Self::StackOrphaned(_)
            | Self::StackCyclicDependency
            | Self::StackInvalidState(_)
            | Self::StackPrNotFound(_) => ErrorCategory::Stack,

            Self::GitHubAuthFailed(_)
            | Self::GitHubTokenExpired
            | Self::GitHubRateLimited(_)
            | Self::GitHubPrClosed(_)
            | Self::GitHubPrNotFound(_)
            | Self::GitHubApiError { .. }
            | Self::GitHubCiFailed(_) => ErrorCategory::GitHub,

            Self::SnapshotNotFound(_)
            | Self::SnapshotCorrupted(_)
            | Self::SnapshotExpired(_)
            | Self::SnapshotLimitExceeded(_)
            | Self::SnapshotRestoreFailed(_) => ErrorCategory::Snapshot,

            _ => self.category_infrastructure(),
        }
    }

    #[allow(clippy::panic)]
    const fn category_infrastructure(&self) -> ErrorCategory {
        match self {
            Self::ConfigNotFound(_)
            | Self::ConfigInvalid(_)
            | Self::ConfigPermission(_)
            | Self::InvalidConfig(_)
            | Self::InvalidRepoUrl(_)
            | Self::AgentNotFound(_)
            | Self::AgentExists(_)
            | Self::AgentTimeout(_)
            | Self::InvalidState(_)
            | Self::NotFound(_)
            | Self::InvalidOperation(_)
            | Self::ValidationError(_)
            | Self::ValidationFieldError { .. }
            | Self::InvalidIdentifier(_)
            | Self::IoError(_)
            | Self::JsonParseError(_)
            | Self::YamlParseError(_)
            | Self::Database(_)
            | Self::Serialization(_)
            | Self::LockTimeout { .. }
            | Self::CloneFailed(_)
            | Self::RecordFailed(_)
            | Self::Persistence(_)
            | Self::StateTransition(_)
            | Self::ScenarioError(_)
            | Self::RunnerError(_)
            | Self::DefinitionError(_)
            | Self::ServerError(_)
            | Self::SyncError(_)
            | Self::Internal(_)
            | Self::Unimplemented(_)
            | Self::InvariantViolation(_) => ErrorCategory::Internal,
            _ => panic!("unhandled variant"),
        }
    }

    /// Returns whether this error is retryable.
    ///
    /// Network and transient errors are retryable; state violations and
    /// not-found errors are terminal.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::VcsPushFailed(_)
            | Self::VcsPullFailed(_)
            | Self::GitHubRateLimited(_)
            | Self::LockTimeout { .. } => true,
            // All other errors are terminal
            _ => false,
        }
    }

    /// Returns a suggested fix for this error, if available.
    #[must_use]
    pub fn fix(&self) -> Option<ErrorFix> {
        match self {
            Self::WorkspaceNotFound(_) => Some(ErrorFix::safe(
                "scp workspace list",
                "List available workspaces",
            )),
            Self::SessionNotFound(_) => Some(ErrorFix::safe(
                "scp session list",
                "List available sessions",
            )),
            Self::QueueEmpty => Some(ErrorFix::safe(
                "scp queue enqueue <branch>",
                "Add an item to the queue",
            )),
            Self::WorkspaceLocked(_, holder) => Some(ErrorFix::new(
                format!("scp agent kill {holder}"),
                "Force release the workspace lock",
                FixRisk::Moderate,
            )),
            Self::VcsNotInitialized => Some(ErrorFix::safe(
                "scp init",
                "Initialize VCS in this directory",
            )),
            Self::WorkingCopyDirty => Some(ErrorFix::safe(
                "git stash",
                "Stash uncommitted changes before continuing",
            )),
            _ => None,
        }
    }

    /// Returns structured context information for this error as a JSON value.
    ///
    /// Provides machine-readable context for AI agents and tooling to understand
    /// the error in detail. Each variant exposes its relevant fields.
    #[must_use]
    pub fn context_map(&self) -> Option<serde_json::Value> {
        match self {
            Self::WorkspaceNotFound(_)
            | Self::WorkspaceExists(_)
            | Self::WorkspaceLocked(_, _)
            | Self::WorkspaceConflict(_) => Some(self.context_workspace()),
            Self::SessionNotFound(_)
            | Self::SessionExists(_)
            | Self::SessionLocked(_, _)
            | Self::NotLockHolder(_, _)
            | Self::SessionInvalidState(_, _, _) => Some(self.context_session()),
            Self::BeadNotFound(_)
            | Self::BeadAlreadyExists(_)
            | Self::InvalidBeadId(_)
            | Self::InvalidBeadTitle(_)
            | Self::BeadInvalidStateTransition { .. }
            | Self::BeadDependencyCycle(_)
            | Self::BeadBlockedBy(_)
            | Self::BeadInvalidDependency(_) => Some(self.context_bead()),
            Self::QueueEmpty
            | Self::QueueItemNotFound(_)
            | Self::QueueLocked(_)
            | Self::QueueProcessing
            | Self::QueueInvalidPosition(_)
            | Self::QueueFull(_) => Some(self.context_queue()),
            Self::VcsNotInitialized
            | Self::VcsConflict(_, _)
            | Self::VcsPushFailed(_)
            | Self::VcsPullFailed(_)
            | Self::VcsRebaseFailed(_)
            | Self::BranchNotFound(_)
            | Self::BranchExists(_)
            | Self::CommitNotFound(_)
            | Self::WorkingCopyDirty => Some(self.context_vcs()),
            Self::StackNotFound(_)
            | Self::StackOrphaned(_)
            | Self::StackCyclicDependency
            | Self::StackInvalidState(_)
            | Self::StackPrNotFound(_) => Some(self.context_stack()),
            Self::GitHubAuthFailed(_)
            | Self::GitHubTokenExpired
            | Self::GitHubRateLimited(_)
            | Self::GitHubPrClosed(_)
            | Self::GitHubPrNotFound(_)
            | Self::GitHubApiError { .. }
            | Self::GitHubCiFailed(_) => Some(self.context_github()),
            _ => Some(self.context_map_infrastructure()),
        }
    }

    /// Dispatches infrastructure (9xxx) context mapping.
    fn context_map_infrastructure(&self) -> serde_json::Value {
        match self {
            Self::SnapshotNotFound(_)
            | Self::SnapshotCorrupted(_)
            | Self::SnapshotExpired(_)
            | Self::SnapshotLimitExceeded(_)
            | Self::SnapshotRestoreFailed(_) => self.context_snapshot(),
            Self::ConfigNotFound(_)
            | Self::ConfigInvalid(_)
            | Self::ConfigPermission(_)
            | Self::InvalidConfig(_)
            | Self::InvalidRepoUrl(_) => self.context_config(),
            Self::AgentNotFound(_)
            | Self::AgentExists(_)
            | Self::AgentTimeout(_) => self.context_agent(),
            Self::InvalidState(_)
            | Self::NotFound(_)
            | Self::InvalidOperation(_)
            | Self::ValidationError(_)
            | Self::ValidationFieldError { .. }
            | Self::InvalidIdentifier(_) => self.context_state_validation(),
            Self::IoError(_)
            | Self::JsonParseError(_)
            | Self::YamlParseError(_)
            | Self::Database(_)
            | Self::Serialization(_) => self.context_io_storage(),
            Self::LockTimeout { .. }
            | Self::CloneFailed(_)
            | Self::RecordFailed(_)
            | Self::Persistence(_)
            | Self::StateTransition(_)
            | Self::Unimplemented(_) => self.context_orchestration(),
            Self::ScenarioError(_)
            | Self::RunnerError(_)
            | Self::DefinitionError(_)
            | Self::ServerError(_)
            | Self::SyncError(_)
            | Self::Internal(_)
            | Self::InvariantViolation(_) => self.context_scenario(),
            _ => unreachable!("context_map_infrastructure: unhandled variant"),
        }
    }

    fn context_workspace(&self) -> serde_json::Value {
        match self {
            Self::WorkspaceNotFound(name) | Self::WorkspaceExists(name) => {
                serde_json::json!({
                    "resource_type": "workspace",
                    "workspace_name": name,
                })
            }
            Self::WorkspaceLocked(name, holder) => serde_json::json!({
                "workspace_name": name,
                "holder": holder,
            }),
            Self::WorkspaceConflict(msg) => serde_json::json!({
                "message": msg,
            }),
            _ => unreachable!("context_workspace called on non-workspace variant"),
        }
    }

    fn context_session(&self) -> serde_json::Value {
        match self {
            Self::SessionNotFound(name) | Self::SessionExists(name) => serde_json::json!({
                "resource_type": "session",
                "session_name": name,
            }),
            Self::SessionLocked(session, holder) => serde_json::json!({
                "session": session,
                "holder": holder,
            }),
            Self::NotLockHolder(session, agent_id) => serde_json::json!({
                "session": session,
                "agent_id": agent_id,
            }),
            Self::SessionInvalidState(session, actual, expected) => serde_json::json!({
                "session": session,
                "actual_state": actual,
                "expected_state": expected,
            }),
            _ => unreachable!("context_session called on non-session variant"),
        }
    }

    fn context_bead(&self) -> serde_json::Value {
        match self {
            Self::BeadNotFound(id) | Self::BeadAlreadyExists(id) => serde_json::json!({
                "resource_type": "bead",
                "bead_id": id,
            }),
            Self::InvalidBeadId(id) => serde_json::json!({
                "bead_id": id,
            }),
            Self::InvalidBeadTitle(title) => serde_json::json!({
                "title": title,
            }),
            Self::BeadInvalidStateTransition { from, to } => serde_json::json!({
                "from_state": from,
                "to_state": to,
            }),
            Self::BeadDependencyCycle(path) => serde_json::json!({
                "cycle_path": path,
            }),
            Self::BeadBlockedBy(blockers) => serde_json::json!({
                "blockers": blockers,
            }),
            Self::BeadInvalidDependency(dep) => serde_json::json!({
                "dependency": dep,
            }),
            _ => unreachable!("context_bead called on non-bead variant"),
        }
    }

    fn context_queue(&self) -> serde_json::Value {
        match self {
            Self::QueueEmpty => serde_json::json!({
                "error_type": "queue_empty",
            }),
            Self::QueueItemNotFound(item) => serde_json::json!({
                "item": item,
            }),
            Self::QueueLocked(holder) => serde_json::json!({
                "holder": holder,
            }),
            Self::QueueProcessing => serde_json::json!({
                "error_type": "queue_processing",
            }),
            Self::QueueInvalidPosition(pos) => serde_json::json!({
                "position": pos,
            }),
            Self::QueueFull(max) => serde_json::json!({
                "max_size": max,
            }),
            _ => unreachable!("context_queue called on non-queue variant"),
        }
    }

    fn context_vcs(&self) -> serde_json::Value {
        match self {
            Self::VcsNotInitialized => serde_json::json!({
                "error_type": "vcs_not_initialized",
            }),
            Self::VcsConflict(repo, msg) => serde_json::json!({
                "repo": repo,
                "message": msg,
            }),
            Self::VcsPushFailed(msg) => serde_json::json!({
                "operation": "push",
                "error": msg,
            }),
            Self::VcsPullFailed(msg) => serde_json::json!({
                "operation": "pull",
                "error": msg,
            }),
            Self::VcsRebaseFailed(msg) => serde_json::json!({
                "operation": "rebase",
                "error": msg,
            }),
            Self::BranchNotFound(branch) | Self::BranchExists(branch) => serde_json::json!({
                "resource_type": "branch",
                "branch_name": branch,
            }),
            Self::CommitNotFound(commit) => serde_json::json!({
                "resource_type": "commit",
                "commit_id": commit,
            }),
            Self::WorkingCopyDirty => serde_json::json!({
                "error_type": "working_copy_dirty",
            }),
            _ => unreachable!("context_vcs called on non-vcs variant"),
        }
    }

    fn context_stack(&self) -> serde_json::Value {
        match self {
            Self::StackNotFound(name) => serde_json::json!({
                "resource_type": "stack",
                "stack_name": name,
            }),
            Self::StackOrphaned(parent) => serde_json::json!({
                "parent": parent,
            }),
            Self::StackCyclicDependency => serde_json::json!({
                "error_type": "stack_cyclic_dependency",
            }),
            Self::StackInvalidState(state) => serde_json::json!({
                "state": state,
            }),
            Self::StackPrNotFound(pr) => serde_json::json!({
                "pr": pr,
            }),
            _ => unreachable!("context_stack called on non-stack variant"),
        }
    }

    fn context_github(&self) -> serde_json::Value {
        match self {
            Self::GitHubAuthFailed(msg) => serde_json::json!({
                "error": msg,
            }),
            Self::GitHubTokenExpired => serde_json::json!({
                "error_type": "github_token_expired",
            }),
            Self::GitHubRateLimited(retry_after) => serde_json::json!({
                "retry_after": retry_after,
            }),
            Self::GitHubPrClosed(pr) | Self::GitHubPrNotFound(pr) => serde_json::json!({
                "pr": pr,
            }),
            Self::GitHubApiError { status, message } => serde_json::json!({
                "status": status,
                "message": message,
            }),
            Self::GitHubCiFailed(checks) => serde_json::json!({
                "checks": checks,
            }),
            _ => unreachable!("context_github called on non-github variant"),
        }
    }

    fn context_snapshot(&self) -> serde_json::Value {
        match self {
            Self::SnapshotNotFound(id) => serde_json::json!({
                "resource_type": "snapshot",
                "snapshot_id": id,
            }),
            Self::SnapshotCorrupted(details) => serde_json::json!({
                "error": details,
            }),
            Self::SnapshotExpired(msg)
            | Self::SnapshotLimitExceeded(msg)
            | Self::SnapshotRestoreFailed(msg) => serde_json::json!({
                "error": msg,
            }),
            _ => unreachable!("context_snapshot called on non-snapshot variant"),
        }
    }

    fn context_config(&self) -> serde_json::Value {
        match self {
            Self::ConfigNotFound(key) => serde_json::json!({
                "resource_type": "config",
                "key": key,
            }),
            Self::ConfigInvalid(msg) | Self::InvalidConfig(msg) => serde_json::json!({
                "error": msg,
            }),
            Self::ConfigPermission(path) => serde_json::json!({
                "path": path,
            }),
            Self::InvalidRepoUrl(url) => serde_json::json!({
                "url": url,
            }),
            _ => unreachable!("context_config called on non-config variant"),
        }
    }

    fn context_agent(&self) -> serde_json::Value {
        match self {
            Self::AgentNotFound(id) | Self::AgentExists(id) => serde_json::json!({
                "resource_type": "agent",
                "agent_id": id,
            }),
            Self::AgentTimeout(id) => serde_json::json!({
                "agent_id": id,
            }),
            _ => unreachable!("context_agent called on non-agent variant"),
        }
    }

    fn context_state_validation(&self) -> serde_json::Value {
        match self {
            Self::InvalidState(msg) => serde_json::json!({
                "state": msg,
            }),
            Self::NotFound(resource) => serde_json::json!({
                "resource": resource,
            }),
            Self::InvalidOperation(op) => serde_json::json!({
                "operation": op,
            }),
            Self::ValidationError(msg) => serde_json::json!({
                "error": msg,
            }),
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
                map
            }
            Self::InvalidIdentifier(id) => serde_json::json!({
                "identifier": id,
            }),
            _ => unreachable!("context_state_validation called on non-state/validation variant"),
        }
    }

    fn context_io_storage(&self) -> serde_json::Value {
        match self {
            Self::IoError(msg)
            | Self::JsonParseError(msg)
            | Self::YamlParseError(msg)
            | Self::Database(msg)
            | Self::Serialization(msg) => serde_json::json!({
                "error": msg,
            }),
            _ => unreachable!("context_io_storage called on non-io/storage variant"),
        }
    }

    fn context_orchestration(&self) -> serde_json::Value {
        match self {
            Self::LockTimeout {
                operation,
                timeout_ms,
                retries,
            } => serde_json::json!({
                "operation": operation,
                "timeout_ms": timeout_ms,
                "retries": retries,
            }),
            Self::CloneFailed(msg)
            | Self::RecordFailed(msg)
            | Self::Persistence(msg) => serde_json::json!({
                "error": msg,
            }),
            Self::StateTransition(msg) => serde_json::json!({
                "transition": msg,
            }),
            Self::Unimplemented(feature) => serde_json::json!({
                "feature": feature,
            }),
            _ => unreachable!("context_orchestration called on non-orchestration variant"),
        }
    }

    fn context_scenario(&self) -> serde_json::Value {
        match self {
            Self::ScenarioError(msg)
            | Self::RunnerError(msg)
            | Self::DefinitionError(msg)
            | Self::ServerError(msg)
            | Self::SyncError(msg)
            | Self::Internal(msg)
            | Self::InvariantViolation(msg) => serde_json::json!({
                "error": msg,
            }),
            _ => unreachable!("context_scenario called on non-scenario/internal variant"),
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

    /// Returns a CLI exit code.
    ///
    /// Uses the high byte of `numeric_code()` for categorization while
    /// fitting in a `u8` range (1-255). The low byte provides uniqueness
    /// within each category.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::WorkspaceNotFound(_)
            | Self::WorkspaceExists(_)
            | Self::WorkspaceLocked(_, _)
            | Self::WorkspaceConflict(_) => self.exit_code_workspace(),
            Self::SessionNotFound(_)
            | Self::SessionExists(_)
            | Self::SessionLocked(_, _)
            | Self::NotLockHolder(_, _)
            | Self::SessionInvalidState(_, _, _) => self.exit_code_session(),
            Self::BeadNotFound(_)
            | Self::BeadAlreadyExists(_)
            | Self::InvalidBeadId(_)
            | Self::InvalidBeadTitle(_)
            | Self::BeadInvalidStateTransition { .. }
            | Self::BeadDependencyCycle(_)
            | Self::BeadBlockedBy(_)
            | Self::BeadInvalidDependency(_) => self.exit_code_bead(),
            Self::QueueEmpty
            | Self::QueueItemNotFound(_)
            | Self::QueueLocked(_)
            | Self::QueueProcessing
            | Self::QueueInvalidPosition(_)
            | Self::QueueFull(_) => self.exit_code_queue(),
            Self::VcsNotInitialized
            | Self::VcsConflict(_, _)
            | Self::VcsPushFailed(_)
            | Self::VcsPullFailed(_)
            | Self::VcsRebaseFailed(_)
            | Self::BranchNotFound(_)
            | Self::BranchExists(_)
            | Self::CommitNotFound(_)
            | Self::WorkingCopyDirty => self.exit_code_vcs(),
            Self::StackNotFound(_)
            | Self::StackOrphaned(_)
            | Self::StackCyclicDependency
            | Self::StackInvalidState(_)
            | Self::StackPrNotFound(_) => self.exit_code_stack(),
            Self::GitHubAuthFailed(_)
            | Self::GitHubTokenExpired
            | Self::GitHubRateLimited(_)
            | Self::GitHubPrClosed(_)
            | Self::GitHubPrNotFound(_)
            | Self::GitHubApiError { .. }
            | Self::GitHubCiFailed(_) => self.exit_code_github(),
            Self::SnapshotNotFound(_)
            | Self::SnapshotCorrupted(_)
            | Self::SnapshotExpired(_)
            | Self::SnapshotLimitExceeded(_)
            | Self::SnapshotRestoreFailed(_) => self.exit_code_snapshot(),
            _ => self.exit_code_infrastructure(),
        }
    }

    #[allow(clippy::panic)]
    const fn exit_code_workspace(&self) -> i32 {
        match self {
            Self::WorkspaceNotFound(_) => 10,
            Self::WorkspaceExists(_) => 11,
            Self::WorkspaceLocked(_, _) => 12,
            Self::WorkspaceConflict(_) => 13,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn exit_code_session(&self) -> i32 {
        match self {
            Self::SessionNotFound(_) => 20,
            Self::SessionExists(_) => 21,
            Self::SessionLocked(_, _) => 22,
            Self::NotLockHolder(_, _) => 23,
            Self::SessionInvalidState(_, _, _) => 24,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn exit_code_bead(&self) -> i32 {
        match self {
            Self::BeadNotFound(_) => 30,
            Self::BeadAlreadyExists(_) => 31,
            Self::InvalidBeadId(_) => 32,
            Self::InvalidBeadTitle(_) => 33,
            Self::BeadInvalidStateTransition { .. } => 34,
            Self::BeadDependencyCycle(_) => 35,
            Self::BeadBlockedBy(_) => 36,
            Self::BeadInvalidDependency(_) => 37,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn exit_code_queue(&self) -> i32 {
        match self {
            Self::QueueEmpty => 40,
            Self::QueueItemNotFound(_) => 41,
            Self::QueueLocked(_) => 42,
            Self::QueueProcessing => 43,
            Self::QueueInvalidPosition(_) => 44,
            Self::QueueFull(_) => 45,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn exit_code_vcs(&self) -> i32 {
        match self {
            Self::VcsNotInitialized => 50,
            Self::VcsConflict(_, _) => 51,
            Self::VcsPushFailed(_) => 52,
            Self::VcsPullFailed(_) => 53,
            Self::VcsRebaseFailed(_) => 54,
            Self::BranchNotFound(_) => 55,
            Self::BranchExists(_) => 56,
            Self::CommitNotFound(_) => 57,
            Self::WorkingCopyDirty => 58,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn exit_code_stack(&self) -> i32 {
        match self {
            Self::StackNotFound(_) => 60,
            Self::StackOrphaned(_) => 61,
            Self::StackCyclicDependency => 62,
            Self::StackInvalidState(_) => 63,
            Self::StackPrNotFound(_) => 64,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn exit_code_github(&self) -> i32 {
        match self {
            Self::GitHubAuthFailed(_) => 70,
            Self::GitHubTokenExpired => 71,
            Self::GitHubRateLimited(_) => 72,
            Self::GitHubPrClosed(_) => 73,
            Self::GitHubPrNotFound(_) => 74,
            Self::GitHubApiError { .. } => 75,
            Self::GitHubCiFailed(_) => 76,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn exit_code_snapshot(&self) -> i32 {
        match self {
            Self::SnapshotNotFound(_) => 80,
            Self::SnapshotCorrupted(_) => 81,
            Self::SnapshotExpired(_) => 82,
            Self::SnapshotLimitExceeded(_) => 83,
            Self::SnapshotRestoreFailed(_) => 84,
            _ => panic!("unhandled variant"),
        }
    }

    #[allow(clippy::panic)]
    const fn exit_code_infrastructure(&self) -> i32 {
        match self {
            Self::ConfigNotFound(_) => 90,
            Self::ConfigInvalid(_) => 91,
            Self::ConfigPermission(_) => 92,
            Self::InvalidConfig(_) => 93,
            Self::InvalidRepoUrl(_) => 94,
            Self::AgentNotFound(_) => 100,
            Self::AgentExists(_) => 101,
            Self::AgentTimeout(_) => 102,
            Self::InvalidState(_) => 110,
            Self::NotFound(_) => 111,
            Self::InvalidOperation(_) => 112,
            Self::ValidationError(_) => 120,
            Self::ValidationFieldError { .. } => 121,
            Self::InvalidIdentifier(_) => 122,
            Self::IoError(_) => 130,
            Self::JsonParseError(_) => 131,
            Self::YamlParseError(_) => 132,
            Self::Database(_) => 133,
            Self::Serialization(_) => 134,
            Self::LockTimeout { .. } => 140,
            Self::CloneFailed(_) => 141,
            Self::RecordFailed(_) => 142,
            Self::Persistence(_) => 143,
            Self::StateTransition(_) => 144,
            Self::ScenarioError(_) => 150,
            Self::RunnerError(_) => 151,
            Self::DefinitionError(_) => 152,
            Self::ServerError(_) => 153,
            Self::SyncError(_) => 154,
            Self::Internal(_) => 200,
            Self::Unimplemented(_) => 201,
            Self::InvariantViolation(_) => 202,
            _ => panic!("unhandled variant"),
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
            Error::BeadInvalidStateTransition {
                from: "a".into(),
                to: "b".into(),
            },
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
            Error::StackNotFound("stack".into()),
            Error::StackOrphaned("parent".into()),
            Error::StackCyclicDependency,
            Error::StackInvalidState("bad".into()),
            Error::StackPrNotFound("pr".into()),
            Error::GitHubAuthFailed("fail".into()),
            Error::GitHubTokenExpired,
            Error::GitHubRateLimited("60s".into()),
            Error::GitHubPrClosed("123".into()),
            Error::GitHubPrNotFound("123".into()),
            Error::GitHubApiError {
                status: 502,
                message: "bad gateway".into(),
            },
            Error::GitHubCiFailed(vec!["ci".into()]),
            Error::SnapshotNotFound("snap".into()),
            Error::SnapshotCorrupted("bad".into()),
            Error::SnapshotExpired("old".into()),
            Error::SnapshotLimitExceeded("max".into()),
            Error::SnapshotRestoreFailed("err".into()),
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
            Error::ValidationFieldError {
                message: "m".into(),
                field: "f".into(),
                value: Some("v".into()),
            },
            Error::InvalidIdentifier("id".into()),
            Error::IoError("msg".into()),
            Error::JsonParseError("msg".into()),
            Error::YamlParseError("msg".into()),
            Error::Database("msg".into()),
            Error::Serialization("msg".into()),
            Error::LockTimeout {
                operation: "op".into(),
                timeout_ms: 5000,
                retries: 3,
            },
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
            assert!(
                !display.is_empty(),
                "Display should not be empty for {:?}",
                v
            );
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
        assert!(Error::WorkspaceNotFound("my-ws".into())
            .to_string()
            .contains("my-ws"));
        assert!(Error::WorkspaceLocked("ws".into(), "alice".into())
            .to_string()
            .contains("alice"));
        assert!(
            Error::SessionInvalidState("s".into(), "open".into(), "closed".into())
                .to_string()
                .contains("open")
        );
        assert!(Error::BeadInvalidStateTransition {
            from: "a".into(),
            to: "b".into()
        }
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
            assert!(
                err.suggestion().is_some(),
                "Expected suggestion for {:?}",
                err
            );
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
            Error::LockTimeout {
                operation: "op".into(),
                timeout_ms: 1000,
                retries: 0,
            },
        ];
        for err in &non_suggesters {
            assert!(
                err.suggestion().is_none(),
                "Expected no suggestion for {:?}",
                err
            );
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
        assert_eq!(
            codes.len(),
            sorted.len(),
            "Exit codes must be unique — found duplicates"
        );
    }

    #[test]
    fn test_exit_codes_match_documented_block() {
        // Verify each variant's exit code is in the correct documented block
        // per ADR-007 category mapping:
        // workspace 10-13, session 20-24, bead 30-37, queue 40-45,
        // vcs 50-58, stack 60-64, github 70-76, snapshot 80-84,
        // config 90-94, agent 100-102, state 110-112, validation 120-122,
        // io/storage 130-134, orchestration 140-144, scenario 150-154, internal 200-202
        for v in all_variants() {
            let code = v.exit_code();
            let (lo, hi, name) = match &v {
                Error::WorkspaceNotFound(_)
                | Error::WorkspaceExists(_)
                | Error::WorkspaceLocked(_, _)
                | Error::WorkspaceConflict(_) => (10, 14, "workspace"),
                Error::SessionNotFound(_)
                | Error::SessionExists(_)
                | Error::SessionLocked(_, _)
                | Error::NotLockHolder(_, _)
                | Error::SessionInvalidState(_, _, _) => (20, 25, "session"),
                Error::BeadNotFound(_)
                | Error::BeadAlreadyExists(_)
                | Error::InvalidBeadId(_)
                | Error::InvalidBeadTitle(_)
                | Error::BeadInvalidStateTransition { .. }
                | Error::BeadDependencyCycle(_)
                | Error::BeadBlockedBy(_)
                | Error::BeadInvalidDependency(_) => (30, 38, "bead"),
                Error::QueueEmpty
                | Error::QueueItemNotFound(_)
                | Error::QueueLocked(_)
                | Error::QueueProcessing
                | Error::QueueInvalidPosition(_)
                | Error::QueueFull(_) => (40, 46, "queue"),
                Error::VcsNotInitialized
                | Error::VcsConflict(_, _)
                | Error::VcsPushFailed(_)
                | Error::VcsPullFailed(_)
                | Error::VcsRebaseFailed(_)
                | Error::BranchNotFound(_)
                | Error::BranchExists(_)
                | Error::CommitNotFound(_)
                | Error::WorkingCopyDirty => (50, 59, "vcs"),
                Error::StackNotFound(_)
                | Error::StackOrphaned(_)
                | Error::StackCyclicDependency
                | Error::StackInvalidState(_)
                | Error::StackPrNotFound(_) => (60, 65, "stack"),
                Error::GitHubAuthFailed(_)
                | Error::GitHubTokenExpired
                | Error::GitHubRateLimited(_)
                | Error::GitHubPrClosed(_)
                | Error::GitHubPrNotFound(_)
                | Error::GitHubApiError { .. }
                | Error::GitHubCiFailed(_) => (70, 77, "github"),
                Error::SnapshotNotFound(_)
                | Error::SnapshotCorrupted(_)
                | Error::SnapshotExpired(_)
                | Error::SnapshotLimitExceeded(_)
                | Error::SnapshotRestoreFailed(_) => (80, 85, "snapshot"),
                Error::ConfigNotFound(_)
                | Error::ConfigInvalid(_)
                | Error::ConfigPermission(_)
                | Error::InvalidConfig(_)
                | Error::InvalidRepoUrl(_) => (90, 95, "config"),
                Error::AgentNotFound(_) | Error::AgentExists(_) | Error::AgentTimeout(_) => {
                    (100, 103, "agent")
                }
                Error::InvalidState(_) | Error::NotFound(_) | Error::InvalidOperation(_) => {
                    (110, 113, "state/conflict")
                }
                Error::ValidationError(_)
                | Error::ValidationFieldError { .. }
                | Error::InvalidIdentifier(_) => (120, 123, "validation"),
                Error::IoError(_)
                | Error::JsonParseError(_)
                | Error::YamlParseError(_)
                | Error::Database(_)
                | Error::Serialization(_) => (130, 135, "io/storage"),
                Error::LockTimeout { .. }
                | Error::CloneFailed(_)
                | Error::RecordFailed(_)
                | Error::Persistence(_)
                | Error::StateTransition(_) => (140, 145, "orchestration"),
                Error::ScenarioError(_)
                | Error::RunnerError(_)
                | Error::DefinitionError(_)
                | Error::ServerError(_)
                | Error::SyncError(_) => (150, 155, "scenario"),
                Error::Internal(_) | Error::Unimplemented(_) | Error::InvariantViolation(_) => {
                    (200, 203, "internal")
                }
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
        assert_eq!(Error::SessionNotFound("x".into()).exit_code(), 20);
        assert_eq!(Error::BeadNotFound("x".into()).exit_code(), 30);
        assert_eq!(Error::QueueEmpty.exit_code(), 40);
        assert_eq!(Error::VcsNotInitialized.exit_code(), 50);
        assert_eq!(Error::StackNotFound("x".into()).exit_code(), 60);
        assert_eq!(Error::GitHubAuthFailed("x".into()).exit_code(), 70);
        assert_eq!(Error::SnapshotNotFound("x".into()).exit_code(), 80);
        assert_eq!(Error::ConfigNotFound("x".into()).exit_code(), 90);
        assert_eq!(Error::AgentNotFound("x".into()).exit_code(), 100);
        assert_eq!(Error::Internal("x".into()).exit_code(), 200);
    }

    // ── Serialization ────────────────────────────────────────────────────

    #[test]
    fn test_serialize_all_variants_produces_valid_json() {
        for v in all_variants() {
            let json = serde_json::to_string(&v)
                .unwrap_or_else(|e| panic!("Serialize failed for {:?}: {e}", v));
            let _: serde_json::Value = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("Invalid JSON for {:?}: {e}\nJSON: {json}", v));
        }
    }

    #[test]
    fn test_serialize_struct_variants_includes_fields() {
        let err = Error::BeadInvalidStateTransition {
            from: "open".into(),
            to: "closed".into(),
        };
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

        let err = Error::LockTimeout {
            operation: "write".into(),
            timeout_ms: 5000,
            retries: 3,
        };
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
            assert!(
                std::error::Error::source(&v).is_none(),
                "Expected no source for {:?}",
                v
            );
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
        assert_eq!(
            Error::InvalidBeadTitle("".into()).to_string(),
            "Invalid bead title: "
        );
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
        assert_eq!(
            Error::QueueInvalidPosition(0).to_string(),
            "Invalid queue position: 0"
        );
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
        assert_eq!(err.exit_code(), 140);
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

    // =========================================================================
    // CLAIM 13: code() returns SCREAMING_SNAKE_CASE for every variant
    // =========================================================================

    #[test]
    fn code_workspace_session() {
        assert_eq!(
            Error::WorkspaceNotFound("x".into()).code(),
            "WORKSPACE_NOT_FOUND"
        );
        assert_eq!(
            Error::WorkspaceExists("x".into()).code(),
            "WORKSPACE_EXISTS"
        );
        assert_eq!(
            Error::WorkspaceLocked("x".into(), "y".into()).code(),
            "WORKSPACE_LOCKED"
        );
        assert_eq!(
            Error::WorkspaceConflict("x".into()).code(),
            "WORKSPACE_CONFLICT"
        );
        assert_eq!(
            Error::SessionNotFound("x".into()).code(),
            "SESSION_NOT_FOUND"
        );
        assert_eq!(Error::SessionExists("x".into()).code(), "SESSION_EXISTS");
        assert_eq!(
            Error::SessionLocked("x".into(), "y".into()).code(),
            "SESSION_LOCKED"
        );
        assert_eq!(
            Error::NotLockHolder("x".into(), "y".into()).code(),
            "NOT_LOCK_HOLDER"
        );
        assert_eq!(
            Error::SessionInvalidState("a".into(), "b".into(), "c".into()).code(),
            "SESSION_INVALID_STATE"
        );
    }

    #[test]
    fn code_bead() {
        assert_eq!(Error::BeadNotFound("x".into()).code(), "BEAD_NOT_FOUND");
        assert_eq!(
            Error::BeadAlreadyExists("x".into()).code(),
            "BEAD_ALREADY_EXISTS"
        );
        assert_eq!(Error::InvalidBeadId("x".into()).code(), "INVALID_BEAD_ID");
        assert_eq!(
            Error::InvalidBeadTitle("x".into()).code(),
            "INVALID_BEAD_TITLE"
        );
        assert_eq!(
            Error::BeadInvalidStateTransition {
                from: "a".into(),
                to: "b".into()
            }
            .code(),
            "BEAD_INVALID_STATE_TRANSITION"
        );
        assert_eq!(
            Error::BeadDependencyCycle("x".into()).code(),
            "BEAD_DEPENDENCY_CYCLE"
        );
        assert_eq!(Error::BeadBlockedBy("x".into()).code(), "BEAD_BLOCKED_BY");
        assert_eq!(
            Error::BeadInvalidDependency("x".into()).code(),
            "BEAD_INVALID_DEPENDENCY"
        );
    }

    #[test]
    fn code_queue() {
        assert_eq!(Error::QueueEmpty.code(), "QUEUE_EMPTY");
        assert_eq!(
            Error::QueueItemNotFound("x".into()).code(),
            "QUEUE_ITEM_NOT_FOUND"
        );
        assert_eq!(Error::QueueLocked("x".into()).code(), "QUEUE_LOCKED");
        assert_eq!(Error::QueueProcessing.code(), "QUEUE_PROCESSING");
        assert_eq!(
            Error::QueueInvalidPosition(0).code(),
            "QUEUE_INVALID_POSITION"
        );
        assert_eq!(Error::QueueFull(10).code(), "QUEUE_FULL");
    }

    #[test]
    fn code_vcs() {
        assert_eq!(Error::VcsNotInitialized.code(), "VCS_NOT_INITIALIZED");
        assert_eq!(
            Error::VcsConflict("a".into(), "b".into()).code(),
            "VCS_CONFLICT"
        );
        assert_eq!(Error::VcsPushFailed("x".into()).code(), "VCS_PUSH_FAILED");
        assert_eq!(Error::VcsPullFailed("x".into()).code(), "VCS_PULL_FAILED");
        assert_eq!(
            Error::VcsRebaseFailed("x".into()).code(),
            "VCS_REBASE_FAILED"
        );
        assert_eq!(Error::BranchNotFound("x".into()).code(), "BRANCH_NOT_FOUND");
        assert_eq!(Error::BranchExists("x".into()).code(), "BRANCH_EXISTS");
        assert_eq!(Error::CommitNotFound("x".into()).code(), "COMMIT_NOT_FOUND");
        assert_eq!(Error::WorkingCopyDirty.code(), "WORKING_COPY_DIRTY");
    }

    #[test]
    fn code_config() {
        assert_eq!(Error::ConfigNotFound("x".into()).code(), "CONFIG_NOT_FOUND");
        assert_eq!(Error::ConfigInvalid("x".into()).code(), "CONFIG_INVALID");
        assert_eq!(
            Error::ConfigPermission("x".into()).code(),
            "CONFIG_PERMISSION"
        );
        assert_eq!(Error::InvalidConfig("x".into()).code(), "INVALID_CONFIG");
        assert_eq!(Error::InvalidRepoUrl("x".into()).code(), "INVALID_REPO_URL");
    }

    #[test]
    fn code_agent() {
        assert_eq!(Error::AgentNotFound("x".into()).code(), "AGENT_NOT_FOUND");
        assert_eq!(Error::AgentExists("x".into()).code(), "AGENT_EXISTS");
        assert_eq!(Error::AgentTimeout("x".into()).code(), "AGENT_TIMEOUT");
    }

    #[test]
    fn code_state_validation() {
        assert_eq!(Error::InvalidState("x".into()).code(), "INVALID_STATE");
        assert_eq!(Error::NotFound("x".into()).code(), "NOT_FOUND");
        assert_eq!(
            Error::InvalidOperation("x".into()).code(),
            "INVALID_OPERATION"
        );
        assert_eq!(
            Error::ValidationError("x".into()).code(),
            "VALIDATION_ERROR"
        );
        assert_eq!(
            Error::ValidationFieldError {
                message: "x".into(),
                field: "y".into(),
                value: None
            }
            .code(),
            "VALIDATION_FIELD_ERROR"
        );
        assert_eq!(
            Error::InvalidIdentifier("x".into()).code(),
            "INVALID_IDENTIFIER"
        );
    }

    #[test]
    fn code_io_orchestration() {
        assert_eq!(Error::IoError("x".into()).code(), "IO_ERROR");
        assert_eq!(Error::JsonParseError("x".into()).code(), "JSON_PARSE_ERROR");
        assert_eq!(Error::YamlParseError("x".into()).code(), "YAML_PARSE_ERROR");
        assert_eq!(Error::Database("x".into()).code(), "DATABASE_ERROR");
        assert_eq!(
            Error::Serialization("x".into()).code(),
            "SERIALIZATION_ERROR"
        );
        assert_eq!(
            Error::LockTimeout {
                operation: "x".into(),
                timeout_ms: 0,
                retries: 0
            }
            .code(),
            "LOCK_TIMEOUT"
        );
        assert_eq!(Error::CloneFailed("x".into()).code(), "CLONE_FAILED");
        assert_eq!(Error::RecordFailed("x".into()).code(), "RECORD_FAILED");
        assert_eq!(Error::Persistence("x".into()).code(), "PERSISTENCE_ERROR");
        assert_eq!(
            Error::StateTransition("x".into()).code(),
            "STATE_TRANSITION_ERROR"
        );
    }

    #[test]
    fn code_scenario_internal() {
        assert_eq!(Error::ScenarioError("x".into()).code(), "SCENARIO_ERROR");
        assert_eq!(Error::RunnerError("x".into()).code(), "RUNNER_ERROR");
        assert_eq!(
            Error::DefinitionError("x".into()).code(),
            "DEFINITION_ERROR"
        );
        assert_eq!(Error::ServerError("x".into()).code(), "SERVER_ERROR");
        assert_eq!(Error::SyncError("x".into()).code(), "SYNC_ERROR");
        assert_eq!(Error::Internal("x".into()).code(), "INTERNAL_ERROR");
        assert_eq!(Error::Unimplemented("x".into()).code(), "NOT_IMPLEMENTED");
        assert_eq!(
            Error::InvariantViolation("x".into()).code(),
            "INVARIANT_VIOLATION"
        );
    }

    #[test]
    fn code_is_static_lifetime() {
        // code() returns &'static str, so the reference must outlive the error
        let code = {
            let err = Error::QueueEmpty;
            err.code()
        };
        assert_eq!(code, "QUEUE_EMPTY");
    }

    // =========================================================================
    // CLAIM 14: context_map() returns structured JSON for every variant
    // =========================================================================

    #[test]
    fn context_map_workspace_not_found() {
        let ctx = Error::WorkspaceNotFound("my-ws".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["resource_type"], "workspace");
        assert_eq!(ctx["workspace_name"], "my-ws");
    }

    #[test]
    fn context_map_workspace_locked() {
        let ctx = Error::WorkspaceLocked("ws".into(), "agent-1".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["workspace_name"], "ws");
        assert_eq!(ctx["holder"], "agent-1");
    }

    #[test]
    fn context_map_session_invalid_state() {
        let ctx = Error::SessionInvalidState("s1".into(), "closed".into(), "open".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["session"], "s1");
        assert_eq!(ctx["actual_state"], "closed");
        assert_eq!(ctx["expected_state"], "open");
    }

    #[test]
    fn context_map_bead_not_found() {
        let ctx = Error::BeadNotFound("ha-123".into()).context_map().unwrap();
        assert_eq!(ctx["resource_type"], "bead");
        assert_eq!(ctx["bead_id"], "ha-123");
    }

    #[test]
    fn context_map_bead_invalid_state_transition() {
        let ctx = Error::BeadInvalidStateTransition {
            from: "open".into(),
            to: "closed".into(),
        }
        .context_map()
        .unwrap();
        assert_eq!(ctx["from_state"], "open");
        assert_eq!(ctx["to_state"], "closed");
    }

    #[test]
    fn context_map_bead_dependency_cycle() {
        let ctx = Error::BeadDependencyCycle("ha-1 -> ha-2 -> ha-1".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["cycle_path"], "ha-1 -> ha-2 -> ha-1");
    }

    #[test]
    fn context_map_queue_empty() {
        let ctx = Error::QueueEmpty.context_map().unwrap();
        assert_eq!(ctx["error_type"], "queue_empty");
    }

    #[test]
    fn context_map_queue_full() {
        let ctx = Error::QueueFull(50).context_map().unwrap();
        assert_eq!(ctx["max_size"], 50);
    }

    #[test]
    fn context_map_vcs_conflict() {
        let ctx = Error::VcsConflict("file.rs".into(), "merge conflict".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["repo"], "file.rs");
        assert_eq!(ctx["message"], "merge conflict");
    }

    #[test]
    fn context_map_vcs_push_failed() {
        let ctx = Error::VcsPushFailed("rejected".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["operation"], "push");
        assert_eq!(ctx["error"], "rejected");
    }

    #[test]
    fn context_map_branch_not_found() {
        let ctx = Error::BranchNotFound("feature/x".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["resource_type"], "branch");
        assert_eq!(ctx["branch_name"], "feature/x");
    }

    #[test]
    fn context_map_config_not_found() {
        let ctx = Error::ConfigNotFound("settings.toml".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["resource_type"], "config");
        assert_eq!(ctx["key"], "settings.toml");
    }

    #[test]
    fn context_map_agent_not_found() {
        let ctx = Error::AgentNotFound("alpha".into()).context_map().unwrap();
        assert_eq!(ctx["resource_type"], "agent");
        assert_eq!(ctx["agent_id"], "alpha");
    }

    #[test]
    fn context_map_validation_field_error_with_value() {
        let ctx = Error::ValidationFieldError {
            message: "must be positive".into(),
            field: "age".into(),
            value: Some("-5".into()),
        }
        .context_map()
        .unwrap();
        assert_eq!(ctx["field"], "age");
        assert_eq!(ctx["message"], "must be positive");
        assert_eq!(ctx["value"], "-5");
    }

    #[test]
    fn context_map_validation_field_error_without_value() {
        let ctx = Error::ValidationFieldError {
            message: "required".into(),
            field: "name".into(),
            value: None,
        }
        .context_map()
        .unwrap();
        assert_eq!(ctx["field"], "name");
        assert_eq!(ctx["message"], "required");
        assert!(!ctx.as_object().unwrap().contains_key("value"));
    }

    #[test]
    fn context_map_lock_timeout() {
        let ctx = Error::LockTimeout {
            operation: "acquire workspace".into(),
            timeout_ms: 5000,
            retries: 3,
        }
        .context_map()
        .unwrap();
        assert_eq!(ctx["operation"], "acquire workspace");
        assert_eq!(ctx["timeout_ms"], 5000);
        assert_eq!(ctx["retries"], 3);
    }

    #[test]
    fn context_map_io_error() {
        let ctx = Error::IoError("disk full".into()).context_map().unwrap();
        assert_eq!(ctx["error"], "disk full");
    }

    #[test]
    fn context_map_database() {
        let ctx = Error::Database("connection refused".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["error"], "connection refused");
    }

    #[test]
    fn context_map_unimplemented() {
        let ctx = Error::Unimplemented("feature X".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["feature"], "feature X");
    }

    #[test]
    fn context_map_not_found() {
        let ctx = Error::NotFound("resource".into()).context_map().unwrap();
        assert_eq!(ctx["resource"], "resource");
    }

    #[test]
    fn context_map_invalid_operation() {
        let ctx = Error::InvalidOperation("delete on read-only".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["operation"], "delete on read-only");
    }

    #[test]
    fn context_map_invalid_identifier() {
        let ctx = Error::InvalidIdentifier("123bad".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["identifier"], "123bad");
    }

    #[test]
    fn context_map_serialization() {
        let ctx = Error::Serialization("buffer overflow".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["error"], "buffer overflow");
    }

    #[test]
    fn context_map_state_transition() {
        let ctx = Error::StateTransition("open -> locked invalid".into())
            .context_map()
            .unwrap();
        assert_eq!(ctx["transition"], "open -> locked invalid");
    }

    #[test]
    fn context_map_all_variants_return_some() {
        // Exhaustive check: every variant returns Some
        for variant in all_variants() {
            assert!(
                variant.context_map().is_some(),
                "context_map() returned None for: {variant}"
            );
        }
    }

    #[test]
    fn code_all_variants_are_screaming_snake() {
        // Verify all codes match SCREAMING_SNAKE_CASE pattern
        for variant in all_variants() {
            let code = variant.code();
            assert!(
                code.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "Code '{code}' is not SCREAMING_SNAKE_CASE for: {variant}"
            );
            assert!(
                !code.starts_with('_') && !code.ends_with('_') && !code.contains("__"),
                "Code '{code}' has invalid underscores for: {variant}"
            );
        }
    }

    // CLAIM 13: From<std::io::Error> conversion
    // =========================================================================

    #[test]
    fn from_io_error_converts_to_io_error_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::IoError(_)));
        assert_eq!(err.to_string(), "IO error: file not found");
    }

    #[test]
    fn from_io_error_preserves_message() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err: Error = Error::from(io_err);
        assert_eq!(err.to_string(), "IO error: access denied");
    }

    #[test]
    fn from_io_error_works_with_map_err() {
        fn fallible() -> Result<()> {
            std::fs::read_to_string("/nonexistent/path")?;
            Ok(())
        }
        let err = fallible().unwrap_err();
        assert!(matches!(err, Error::IoError(_)));
        assert!(err.to_string().contains("IO error:"));
    }

    // =========================================================================
    // CLAIM 15: ErrorCategory (ADR-007)
    // =========================================================================

    #[test]
    fn error_category_display() {
        assert_eq!(ErrorCategory::Workspace.to_string(), "workspace");
        assert_eq!(ErrorCategory::Session.to_string(), "session");
        assert_eq!(ErrorCategory::Bead.to_string(), "bead");
        assert_eq!(ErrorCategory::Queue.to_string(), "queue");
        assert_eq!(ErrorCategory::Vcs.to_string(), "vcs");
        assert_eq!(ErrorCategory::Stack.to_string(), "stack");
        assert_eq!(ErrorCategory::GitHub.to_string(), "github");
        assert_eq!(ErrorCategory::Snapshot.to_string(), "snapshot");
        assert_eq!(ErrorCategory::Internal.to_string(), "internal");
    }

    #[test]
    fn error_category_base_and_max() {
        assert_eq!(ErrorCategory::Workspace.base(), 1000);
        assert_eq!(ErrorCategory::Workspace.max(), 1999);
        assert_eq!(ErrorCategory::Session.base(), 2000);
        assert_eq!(ErrorCategory::Internal.base(), 9000);
        assert_eq!(ErrorCategory::Internal.max(), 9999);
    }

    // =========================================================================
    // CLAIM 16: FixRisk and ErrorFix (ADR-007)
    // =========================================================================

    #[test]
    fn fix_risk_display() {
        assert_eq!(FixRisk::Safe.to_string(), "safe");
        assert_eq!(FixRisk::Moderate.to_string(), "moderate");
        assert_eq!(FixRisk::Dangerous.to_string(), "dangerous");
    }

    #[test]
    fn error_fix_new_and_safe() {
        let fix = ErrorFix::new("rm -rf /", "Delete everything", FixRisk::Dangerous);
        assert_eq!(fix.command, "rm -rf /");
        assert_eq!(fix.description, "Delete everything");
        assert_eq!(fix.risk, FixRisk::Dangerous);

        let safe_fix = ErrorFix::safe("git status", "Show working tree status");
        assert_eq!(safe_fix.risk, FixRisk::Safe);
    }

    #[test]
    fn error_fix_serialization() {
        let fix = ErrorFix::safe("scp workspace list", "List workspaces");
        let json = serde_json::to_string(&fix).expect("serialize");
        assert!(json.contains("scp workspace list"));
        assert!(json.contains("safe"));
    }

    // =========================================================================
    // CLAIM 17: numeric_code() (ADR-007 hierarchical codes)
    // =========================================================================

    #[test]
    fn numeric_code_workspace_range() {
        assert_eq!(Error::WorkspaceNotFound("x".into()).numeric_code(), 1001);
        assert_eq!(Error::WorkspaceExists("x".into()).numeric_code(), 1002);
        assert_eq!(Error::WorkspaceLocked("x".into(), "y".into()).numeric_code(), 1003);
        assert_eq!(Error::WorkspaceConflict("x".into()).numeric_code(), 1004);
    }

    #[test]
    fn numeric_code_session_range() {
        assert_eq!(Error::SessionNotFound("x".into()).numeric_code(), 2001);
        assert_eq!(Error::SessionExists("x".into()).numeric_code(), 2002);
        assert_eq!(Error::SessionLocked("x".into(), "y".into()).numeric_code(), 2003);
        assert_eq!(Error::NotLockHolder("x".into(), "y".into()).numeric_code(), 2004);
        assert_eq!(
            Error::SessionInvalidState("a".into(), "b".into(), "c".into()).numeric_code(),
            2005
        );
    }

    #[test]
    fn numeric_code_bead_range() {
        assert_eq!(Error::BeadNotFound("x".into()).numeric_code(), 3001);
        assert_eq!(Error::BeadAlreadyExists("x".into()).numeric_code(), 3002);
        assert_eq!(Error::InvalidBeadId("x".into()).numeric_code(), 3003);
        assert_eq!(Error::InvalidBeadTitle("x".into()).numeric_code(), 3004);
        assert_eq!(
            Error::BeadInvalidStateTransition {
                from: "a".into(),
                to: "b".into()
            }
            .numeric_code(),
            3005
        );
        assert_eq!(Error::BeadDependencyCycle("x".into()).numeric_code(), 3006);
        assert_eq!(Error::BeadBlockedBy("x".into()).numeric_code(), 3007);
        assert_eq!(Error::BeadInvalidDependency("x".into()).numeric_code(), 3008);
    }

    #[test]
    fn numeric_code_queue_range() {
        assert_eq!(Error::QueueEmpty.numeric_code(), 4001);
        assert_eq!(Error::QueueItemNotFound("x".into()).numeric_code(), 4002);
        assert_eq!(Error::QueueLocked("x".into()).numeric_code(), 4003);
        assert_eq!(Error::QueueProcessing.numeric_code(), 4004);
        assert_eq!(Error::QueueInvalidPosition(0).numeric_code(), 4005);
        assert_eq!(Error::QueueFull(10).numeric_code(), 4006);
    }

    #[test]
    fn numeric_code_vcs_range() {
        assert_eq!(Error::VcsNotInitialized.numeric_code(), 5001);
        assert_eq!(
            Error::VcsConflict("a".into(), "b".into()).numeric_code(),
            5002
        );
        assert_eq!(Error::VcsPushFailed("x".into()).numeric_code(), 5003);
        assert_eq!(Error::VcsPullFailed("x".into()).numeric_code(), 5004);
        assert_eq!(Error::VcsRebaseFailed("x".into()).numeric_code(), 5005);
        assert_eq!(Error::BranchNotFound("x".into()).numeric_code(), 5006);
        assert_eq!(Error::BranchExists("x".into()).numeric_code(), 5007);
        assert_eq!(Error::CommitNotFound("x".into()).numeric_code(), 5008);
        assert_eq!(Error::WorkingCopyDirty.numeric_code(), 5009);
    }

    #[test]
    fn numeric_code_stack_range() {
        assert_eq!(Error::StackNotFound("x".into()).numeric_code(), 6001);
        assert_eq!(Error::StackOrphaned("x".into()).numeric_code(), 6002);
        assert_eq!(Error::StackCyclicDependency.numeric_code(), 6003);
        assert_eq!(Error::StackInvalidState("x".into()).numeric_code(), 6004);
        assert_eq!(Error::StackPrNotFound("x".into()).numeric_code(), 6005);
    }

    #[test]
    fn numeric_code_github_range() {
        assert_eq!(Error::GitHubAuthFailed("x".into()).numeric_code(), 7001);
        assert_eq!(Error::GitHubTokenExpired.numeric_code(), 7002);
        assert_eq!(Error::GitHubRateLimited("x".into()).numeric_code(), 7003);
        assert_eq!(Error::GitHubPrClosed("x".into()).numeric_code(), 7004);
        assert_eq!(Error::GitHubPrNotFound("x".into()).numeric_code(), 7005);
        assert_eq!(
            Error::GitHubApiError {
                status: 502,
                message: "x".into()
            }
            .numeric_code(),
            7006
        );
        assert_eq!(Error::GitHubCiFailed(vec!["x".into()]).numeric_code(), 7007);
    }

    #[test]
    fn numeric_code_snapshot_range() {
        assert_eq!(Error::SnapshotNotFound("x".into()).numeric_code(), 8001);
        assert_eq!(Error::SnapshotCorrupted("x".into()).numeric_code(), 8002);
        assert_eq!(Error::SnapshotExpired("x".into()).numeric_code(), 8003);
        assert_eq!(Error::SnapshotLimitExceeded("x".into()).numeric_code(), 8004);
        assert_eq!(Error::SnapshotRestoreFailed("x".into()).numeric_code(), 8005);
    }

    #[test]
    fn numeric_code_internal_range() {
        assert_eq!(Error::Internal("x".into()).numeric_code(), 9001);
        assert_eq!(Error::Unimplemented("x".into()).numeric_code(), 9002);
        assert_eq!(Error::InvariantViolation("x".into()).numeric_code(), 9003);
    }

    #[test]
    fn numeric_code_infrastructure_subranges() {
        // Config (91xx)
        assert_eq!(Error::ConfigNotFound("x".into()).numeric_code(), 9101);
        // Agent (92xx)
        assert_eq!(Error::AgentNotFound("x".into()).numeric_code(), 9201);
        // State (93xx)
        assert_eq!(Error::InvalidState("x".into()).numeric_code(), 9301);
        // Validation (94xx)
        assert_eq!(Error::ValidationError("x".into()).numeric_code(), 9401);
        // IO (95xx)
        assert_eq!(Error::IoError("x".into()).numeric_code(), 9501);
        // Orchestration (96xx)
        assert_eq!(
            Error::LockTimeout {
                operation: "x".into(),
                timeout_ms: 0,
                retries: 0
            }
            .numeric_code(),
            9601
        );
        // Scenario (97xx)
        assert_eq!(Error::ScenarioError("x".into()).numeric_code(), 9701);
    }

    #[test]
    fn numeric_code_all_unique() {
        let codes: Vec<u16> = all_variants().iter().map(|v| v.numeric_code()).collect();
        let mut sorted = codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            codes.len(),
            sorted.len(),
            "Numeric codes must be unique -- found duplicates"
        );
    }

    #[test]
    fn numeric_code_in_correct_category_range() {
        for variant in all_variants() {
            let code = variant.numeric_code();
            let cat = variant.category();
            assert!(
                code >= cat.base() && code <= cat.max(),
                "Numeric code {code} for {:?} outside category {:?} range {}-{}",
                variant.code(),
                cat,
                cat.base(),
                cat.max()
            );
        }
    }

    // =========================================================================
    // CLAIM 18: category() (ADR-007)
    // =========================================================================

    #[test]
    fn category_workspace() {
        assert_eq!(Error::WorkspaceNotFound("x".into()).category(), ErrorCategory::Workspace);
        assert_eq!(Error::WorkspaceExists("x".into()).category(), ErrorCategory::Workspace);
        assert_eq!(
            Error::WorkspaceLocked("x".into(), "y".into()).category(),
            ErrorCategory::Workspace
        );
        assert_eq!(Error::WorkspaceConflict("x".into()).category(), ErrorCategory::Workspace);
    }

    #[test]
    fn category_session() {
        assert_eq!(Error::SessionNotFound("x".into()).category(), ErrorCategory::Session);
        assert_eq!(Error::SessionExists("x".into()).category(), ErrorCategory::Session);
    }

    #[test]
    fn category_bead() {
        assert_eq!(Error::BeadNotFound("x".into()).category(), ErrorCategory::Bead);
        assert_eq!(Error::BeadAlreadyExists("x".into()).category(), ErrorCategory::Bead);
    }

    #[test]
    fn category_queue() {
        assert_eq!(Error::QueueEmpty.category(), ErrorCategory::Queue);
    }

    #[test]
    fn category_vcs() {
        assert_eq!(Error::VcsNotInitialized.category(), ErrorCategory::Vcs);
    }

    #[test]
    fn category_stack() {
        assert_eq!(Error::StackNotFound("x".into()).category(), ErrorCategory::Stack);
        assert_eq!(Error::StackCyclicDependency.category(), ErrorCategory::Stack);
    }

    #[test]
    fn category_github() {
        assert_eq!(Error::GitHubAuthFailed("x".into()).category(), ErrorCategory::GitHub);
        assert_eq!(Error::GitHubTokenExpired.category(), ErrorCategory::GitHub);
    }

    #[test]
    fn category_snapshot() {
        assert_eq!(Error::SnapshotNotFound("x".into()).category(), ErrorCategory::Snapshot);
    }

    #[test]
    fn category_internal() {
        assert_eq!(Error::Internal("x".into()).category(), ErrorCategory::Internal);
        assert_eq!(Error::ConfigNotFound("x".into()).category(), ErrorCategory::Internal);
        assert_eq!(Error::AgentNotFound("x".into()).category(), ErrorCategory::Internal);
        assert_eq!(Error::IoError("x".into()).category(), ErrorCategory::Internal);
    }

    // =========================================================================
    // CLAIM 19: is_retryable() (ADR-007)
    // =========================================================================

    #[test]
    fn retryable_errors() {
        assert!(Error::VcsPushFailed("x".into()).is_retryable());
        assert!(Error::VcsPullFailed("x".into()).is_retryable());
        assert!(Error::GitHubRateLimited("60s".into()).is_retryable());
        assert!(
            Error::LockTimeout {
                operation: "x".into(),
                timeout_ms: 5000,
                retries: 3
            }
            .is_retryable()
        );
    }

    #[test]
    fn non_retryable_errors() {
        assert!(!Error::WorkspaceNotFound("x".into()).is_retryable());
        assert!(!Error::SessionNotFound("x".into()).is_retryable());
        assert!(!Error::BeadNotFound("x".into()).is_retryable());
        assert!(!Error::QueueEmpty.is_retryable());
        assert!(!Error::Internal("x".into()).is_retryable());
        assert!(!Error::InvariantViolation("x".into()).is_retryable());
        assert!(!Error::ConfigNotFound("x".into()).is_retryable());
    }

    // =========================================================================
    // CLAIM 20: fix() (ADR-007)
    // =========================================================================

    #[test]
    fn fix_workspace_not_found() {
        let fix = Error::WorkspaceNotFound("x".into()).fix().expect("should have fix");
        assert_eq!(fix.command, "scp workspace list");
        assert_eq!(fix.risk, FixRisk::Safe);
    }

    #[test]
    fn fix_session_not_found() {
        let fix = Error::SessionNotFound("x".into()).fix().expect("should have fix");
        assert_eq!(fix.command, "scp session list");
        assert_eq!(fix.risk, FixRisk::Safe);
    }

    #[test]
    fn fix_queue_empty() {
        let fix = Error::QueueEmpty.fix().expect("should have fix");
        assert!(fix.command.contains("enqueue"));
        assert_eq!(fix.risk, FixRisk::Safe);
    }

    #[test]
    fn fix_workspace_locked() {
        let fix = Error::WorkspaceLocked("ws".into(), "alice".into())
            .fix()
            .expect("should have fix");
        assert!(fix.command.contains("alice"));
        assert!(fix.command.contains("kill"));
        assert_eq!(fix.risk, FixRisk::Moderate);
    }

    #[test]
    fn fix_vcs_not_initialized() {
        let fix = Error::VcsNotInitialized.fix().expect("should have fix");
        assert_eq!(fix.command, "scp init");
    }

    #[test]
    fn fix_working_copy_dirty() {
        let fix = Error::WorkingCopyDirty.fix().expect("should have fix");
        assert_eq!(fix.command, "git stash");
    }

    #[test]
    fn fix_none_for_non_recoverable() {
        assert!(Error::Internal("x".into()).fix().is_none());
        assert!(Error::BeadNotFound("x".into()).fix().is_none());
        assert!(Error::QueueItemNotFound("x".into()).fix().is_none());
        assert!(Error::InvariantViolation("x".into()).fix().is_none());
    }

    // =========================================================================
    // CLAIM 21: New variant display
    // =========================================================================

    #[test]
    fn display_stack_variants() {
        assert!(Error::StackNotFound("s".into()).to_string().contains("Stack not found"));
        assert!(Error::StackOrphaned("p".into()).to_string().contains("orphaned"));
        assert!(Error::StackCyclicDependency.to_string().contains("cyclic"));
        assert!(Error::StackInvalidState("bad".into()).to_string().contains("invalid state"));
        assert!(Error::StackPrNotFound("123".into()).to_string().contains("PR not found"));
    }

    #[test]
    fn display_github_variants() {
        assert!(Error::GitHubAuthFailed("fail".into()).to_string().contains("auth"));
        assert!(Error::GitHubTokenExpired.to_string().contains("expired"));
        assert!(Error::GitHubRateLimited("60s".into()).to_string().contains("rate limited"));
        assert!(Error::GitHubPrClosed("123".into()).to_string().contains("closed"));
        assert!(Error::GitHubPrNotFound("123".into()).to_string().contains("not found"));
        assert!(
            Error::GitHubApiError {
                status: 502,
                message: "bad".into()
            }
            .to_string()
            .contains("502")
        );
        assert!(Error::GitHubCiFailed(vec!["ci".into()]).to_string().contains("CI"));
    }

    #[test]
    fn display_snapshot_variants() {
        assert!(Error::SnapshotNotFound("s".into()).to_string().contains("not found"));
        assert!(Error::SnapshotCorrupted("bad".into()).to_string().contains("corrupted"));
        assert!(Error::SnapshotExpired("old".into()).to_string().contains("expired"));
        assert!(Error::SnapshotLimitExceeded("max".into()).to_string().contains("exceeded"));
        assert!(Error::SnapshotRestoreFailed("err".into()).to_string().contains("restore failed"));
    }
}
