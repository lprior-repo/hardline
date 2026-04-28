//! Unified error types for Source Control Plane.
//!
//! All errors return Result<T, Error> - zero panic, zero unwrap.

use thiserror::Error;

// Re-export types from sub-modules
pub use super::error_types::Result;
// Forward declaration for lock errors
pub use crate::coordination::locks::errors::LockError;

// ========================================================================
// Unified Error Type
// ========================================================================

/// Unified error type for SCP (Source Control Plane).
///
/// Error codes:
/// - 1xxx: Workspace/Session errors
/// - 2xxx: Queue errors
/// - 3xxx: VCS errors
/// - 4xxx: Configuration errors
/// - 5xxx: Agent errors
/// - 6xxx: IO errors
/// - 7xxx: State/Conflict errors
/// - 8xxx: Validation errors
/// - 9xxx: Internal errors
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    // ========================================================================
    // Workspace/Session Errors (1xxx)
    // ========================================================================
    /// Workspace errors
    #[error(transparent)]
    Workspace(#[from] super::error_workspace::WorkspaceError),

    /// Session errors
    #[error(transparent)]
    Session(#[from] super::error_workspace::SessionError),

    // ========================================================================
    // Queue Errors (2xxx)
    // ========================================================================
    /// Queue errors
    #[error(transparent)]
    Queue(#[from] super::error_queue::QueueError),

    // ========================================================================
    // VCS Errors (3xxx)
    // ========================================================================
    /// VCS errors
    #[error(transparent)]
    Vcs(#[from] super::error_vcs::VcsError),

    // ========================================================================
    // Configuration Errors (4xxx)
    // ========================================================================
    /// Configuration errors
    #[error(transparent)]
    Config(#[from] super::error_config::ConfigError),

    // ========================================================================
    // Agent Errors (5xxx)
    // ========================================================================
    /// Agent errors
    #[error(transparent)]
    Agent(#[from] super::error_agent::AgentError),

    // ========================================================================
    // IO Errors (6xxx)
    // ========================================================================
    /// IO errors
    #[error(transparent)]
    Io(#[from] super::error_io::IoError),

    // ========================================================================
    // State/Conflict Errors (7xxx)
    // ========================================================================
    /// State and validation errors
    #[error(transparent)]
    State(#[from] super::error_state::StateError),

    // ========================================================================
    // Internal Errors (9xxx)
    // ========================================================================
    /// Internal errors
    #[error(transparent)]
    Internal(#[from] super::error_internal::InternalError),

    // ========================================================================
    // Task Errors (6xxx)
    // ========================================================================
    /// Task errors
    #[error(transparent)]
    Task(#[from] super::error_task::TaskError),

    // ========================================================================
    // Wait/Batch Errors (5xxx, 8xxx)
    // ========================================================================
    /// Wait and batch errors
    #[error(transparent)]
    Wait(#[from] super::error_wait::WaitError),

    // ========================================================================
    // Lock Errors (9xxx)
    // ========================================================================
    /// Lock manager errors (session locking with TTL/heartbeat)
    #[error(transparent)]
    Lock(#[from] super::coordination::locks::errors::LockError),
}

// ========================================================================
// Error conversions
// ========================================================================

/// Convert std::io::Error to Error::Io(IoError(IoErrorKind::Io(io_error)))
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        use super::error_io::IoErrorKind;
        IoErrorKind::Io(e).into()
    }
}

/// Convert serde_json::Error to Error::Io
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        use super::error_io::IoErrorKind;
        IoErrorKind::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("JSON error: {e}"),
        ))
        .into()
    }
}

// ========================================================================
// Constructors for backwards compatibility
// ========================================================================

impl Error {
    /// Creates a Database error
    #[inline]
    pub fn database(msg: impl Into<String>) -> Self {
        use super::error_io::IoErrorKind;
        IoErrorKind::Database(msg.into()).into()
    }

    /// Creates an InvalidIdentifier error
    #[inline]
    pub fn invalid_identifier(msg: impl Into<String>) -> Self {
        use super::error_state::StateErrorKind;
        StateErrorKind::InvalidIdentifier(msg.into()).into()
    }

    /// Creates a ValidationFieldError
    #[inline]
    pub fn validation_field_error(
        field: impl Into<String>,
        message: impl Into<String>,
        value: Option<String>,
    ) -> Self {
        use super::error_state::StateErrorKind;
        StateErrorKind::ValidationFieldError {
            field: field.into(),
            message: message.into(),
            value,
        }
        .into()
    }

    /// Creates an IoError
    #[inline]
    pub fn io_error(msg: impl Into<String>) -> Self {
        use super::error_io::IoErrorKind;
        IoErrorKind::IoError(msg.into()).into()
    }

    /// Creates an InvalidState error
    #[inline]
    pub fn invalid_state(msg: impl Into<String>) -> Self {
        use super::error_state::StateErrorKind;
        StateErrorKind::InvalidState(msg.into()).into()
    }

    /// Creates a NotFound error
    #[inline]
    pub fn not_found(msg: impl Into<String>) -> Self {
        use super::error_state::StateErrorKind;
        StateErrorKind::NotFound(msg.into()).into()
    }

    /// Creates a ValidationError
    #[inline]
    pub fn validation_error(msg: impl Into<String>) -> Self {
        use super::error_state::StateErrorKind;
        StateErrorKind::ValidationError(msg.into()).into()
    }

    /// Creates a SessionLocked error
    #[inline]
    pub fn session_locked(session: impl Into<String>, holder: impl Into<String>) -> Self {
        use super::error_workspace::SessionErrorKind;
        SessionErrorKind::Locked(session.into(), holder.into()).into()
    }

    /// Creates a SessionNotFound error
    #[inline]
    pub fn session(msg: impl Into<String>) -> Self {
        use super::error_workspace::SessionErrorKind;
        SessionErrorKind::NotFound(msg.into()).into()
    }

    /// Creates a WorkspaceConflict error
    #[inline]
    pub fn workspace_conflict(msg: impl Into<String>) -> Self {
        use super::error_workspace::WorkspaceErrorKind;
        WorkspaceErrorKind::Conflict(msg.into()).into()
    }

    /// Creates a NotLockHolder error
    #[inline]
    pub fn not_lock_holder(session: impl Into<String>, agent: impl Into<String>) -> Self {
        use super::error_workspace::SessionErrorKind;
        SessionErrorKind::NotLockHolder(session.into(), agent.into()).into()
    }

    /// Creates a ConfigNotFound error
    #[inline]
    pub fn config_not_found(msg: impl Into<String>) -> Self {
        use super::error_config::ConfigErrorKind;
        ConfigErrorKind::NotFound(msg.into()).into()
    }

    /// Creates a ConfigInvalid error
    #[inline]
    pub fn config_invalid(msg: impl Into<String>) -> Self {
        use super::error_config::ConfigErrorKind;
        ConfigErrorKind::Invalid(msg.into()).into()
    }

    /// Creates a ConfigPermission error
    #[inline]
    pub fn config_permission(msg: impl Into<String>) -> Self {
        use super::error_config::ConfigErrorKind;
        ConfigErrorKind::Permission(msg.into()).into()
    }

    /// Creates an AgentNotFound error
    #[inline]
    pub fn agent_not_found(id: impl Into<String>) -> Self {
        use super::error_agent::AgentErrorKind;
        AgentErrorKind::NotFound(id.into()).into()
    }

    /// Creates an AgentExists error
    #[inline]
    pub fn agent_exists(id: impl Into<String>) -> Self {
        use super::error_agent::AgentErrorKind;
        AgentErrorKind::Exists(id.into()).into()
    }

    /// Creates a VcsNotInitialized error
    #[inline]
    pub fn vcs_not_initialized() -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::NotInitialized.into()
    }

    /// Creates a VcsInitFailed error
    #[inline]
    pub fn vcs_init_failed(
        vcs_type: impl Into<String>,
        directory: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::InitFailed {
            vcs_type: vcs_type.into(),
            directory: directory.into(),
            reason: reason.into(),
        }
        .into()
    }

    /// Creates a VcsConflict error
    #[inline]
    pub fn vcs_conflict(repo: impl Into<String>, msg: impl Into<String>) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::Conflict(repo.into(), msg.into()).into()
    }

    /// Creates a VcsPushFailed error
    #[inline]
    pub fn vcs_push_failed(msg: impl Into<String>) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::PushFailed(msg.into()).into()
    }

    /// Creates a VcsPullFailed error
    #[inline]
    pub fn vcs_pull_failed(msg: impl Into<String>) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::PullFailed(msg.into()).into()
    }

    /// Creates a VcsRebaseFailed error
    #[inline]
    pub fn vcs_rebase_failed(msg: impl Into<String>) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::RebaseFailed(msg.into()).into()
    }

    /// Creates a BranchNotFound error
    #[inline]
    pub fn branch_not_found(branch: impl Into<String>) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::BranchNotFound(branch.into()).into()
    }

    /// Creates a BranchExists error
    #[inline]
    pub fn branch_exists(branch: impl Into<String>) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::BranchExists(branch.into()).into()
    }

    /// Creates a CommitNotFound error
    #[inline]
    pub fn commit_not_found(commit: impl Into<String>) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::CommitNotFound(commit.into()).into()
    }

    /// Creates a WorkingCopyDirty error
    #[inline]
    pub fn working_copy_dirty() -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::WorkingCopyDirty.into()
    }

    /// Creates a CommitFailed error
    #[inline]
    pub fn vcs_commit_failed(msg: impl Into<String>) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::CommitFailed(msg.into()).into()
    }

    /// Creates a CheckoutFailed error
    #[inline]
    pub fn vcs_checkout_failed(msg: impl Into<String>) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::CheckoutFailed(msg.into()).into()
    }

    /// Creates a DiffFailed error
    #[inline]
    pub fn vcs_diff_failed(msg: impl Into<String>) -> Self {
        use super::error_vcs::VcsErrorKind;
        VcsErrorKind::DiffFailed(msg.into()).into()
    }

    /// Creates a WorkspaceNotFound error
    #[inline]
    pub fn workspace_not_found(name: impl Into<String>) -> Self {
        use super::error_workspace::WorkspaceErrorKind;
        WorkspaceErrorKind::NotFound(name.into()).into()
    }

    /// Creates a WorkspaceExists error
    #[inline]
    pub fn workspace_exists(name: impl Into<String>) -> Self {
        use super::error_workspace::WorkspaceErrorKind;
        WorkspaceErrorKind::Exists(name.into()).into()
    }

    /// Creates a WorkspaceLocked error
    #[inline]
    pub fn workspace_locked(name: impl Into<String>, holder: impl Into<String>) -> Self {
        use super::error_workspace::WorkspaceErrorKind;
        WorkspaceErrorKind::Locked(name.into(), holder.into()).into()
    }

    /// Creates a SessionExists error
    #[inline]
    pub fn session_exists(name: impl Into<String>) -> Self {
        use super::error_workspace::SessionErrorKind;
        SessionErrorKind::Exists(name.into()).into()
    }

    /// Creates a SessionInvalidState error
    #[inline]
    pub fn session_invalid_state(
        session: impl Into<String>,
        state: impl Into<String>,
        expected: impl Into<String>,
    ) -> Self {
        use super::error_workspace::SessionErrorKind;
        SessionErrorKind::InvalidState(session.into(), state.into(), expected.into()).into()
    }

    /// Creates a QueueEmpty error
    #[inline]
    pub fn queue_empty() -> Self {
        use super::error_queue::QueueErrorKind;
        QueueErrorKind::Empty.into()
    }

    /// Creates a QueueItemNotFound error
    #[inline]
    pub fn queue_item_not_found(item: impl Into<String>) -> Self {
        use super::error_queue::QueueErrorKind;
        QueueErrorKind::ItemNotFound(item.into()).into()
    }

    /// Creates a QueueLocked error
    #[inline]
    pub fn queue_locked(holder: impl Into<String>) -> Self {
        use super::error_queue::QueueErrorKind;
        QueueErrorKind::Locked(holder.into()).into()
    }

    /// Creates a QueueProcessing error
    #[inline]
    pub fn queue_processing() -> Self {
        use super::error_queue::QueueErrorKind;
        QueueErrorKind::Processing.into()
    }

    /// Creates a QueueInvalidPosition error
    #[inline]
    pub fn queue_invalid_position(pos: usize) -> Self {
        use super::error_queue::QueueErrorKind;
        QueueErrorKind::InvalidPosition(pos).into()
    }

    /// Creates a QueueFull error
    #[inline]
    pub fn queue_full(size: usize) -> Self {
        use super::error_queue::QueueErrorKind;
        QueueErrorKind::Full(size).into()
    }

    /// Creates an Internal error
    #[inline]
    pub fn internal(msg: impl Into<String>) -> Self {
        use super::error_internal::InternalErrorKind;
        InternalErrorKind::Internal(msg.into()).into()
    }

    /// Creates an Unimplemented error
    #[inline]
    pub fn unimplemented(feature: impl Into<String>) -> Self {
        use super::error_internal::InternalErrorKind;
        InternalErrorKind::Unimplemented(feature.into()).into()
    }

    /// Creates a BatchEmpty error
    #[inline]
    pub fn batch_empty() -> Self {
        use super::error_wait::WaitErrorKind;
        WaitErrorKind::BatchEmpty.into()
    }

    /// Creates a BatchCommandFailed error
    #[inline]
    pub fn batch_command_failed(msg: impl Into<String>) -> Self {
        use super::error_wait::WaitErrorKind;
        WaitErrorKind::BatchCommandFailed(msg.into()).into()
    }

    /// Creates a BatchRollbackFailed error
    #[inline]
    pub fn batch_rollback_failed(msg: impl Into<String>) -> Self {
        use super::error_wait::WaitErrorKind;
        WaitErrorKind::BatchRollbackFailed(msg.into()).into()
    }

    /// Creates a BatchSizeExceeded error
    #[inline]
    pub fn batch_size_exceeded(size: usize) -> Self {
        use super::error_wait::WaitErrorKind;
        WaitErrorKind::BatchSizeExceeded(size).into()
    }

    /// Creates a CheckpointError
    #[inline]
    pub fn checkpoint_error(msg: impl Into<String>) -> Self {
        use super::error_wait::WaitErrorKind;
        WaitErrorKind::CheckpointError(msg.into()).into()
    }
}

// ========================================================================
// Error Context & Suggestions
// ========================================================================

impl Error {
    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Error::Workspace(e) => e.suggestion(),
            Error::Session(e) => e.suggestion(),
            Error::Queue(e) => e.suggestion(),
            Error::Vcs(e) => e.suggestion(),
            Error::Config(_) => None,
            Error::Agent(_) => None,
            Error::Io(_) => None,
            Error::State(e) => e.suggestion(),
            Error::Internal(_) => None,
            Error::Task(_) => None,
            Error::Wait(_) => None,
            Error::Lock(e) => e.suggestion(),
        }
    }

    /// Returns a human-readable suggestion for SessionLocked error.
    pub fn session_locked_suggestion(holder: &str) -> Option<String> {
        Some(format!("Use 'scp agent kill {holder}' to force release"))
    }

    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Workspace(e) => e.exit_code(),
            Error::Session(e) => e.exit_code(),
            Error::Queue(e) => e.exit_code(),
            Error::Vcs(e) => e.exit_code(),
            Error::Config(e) => e.exit_code(),
            Error::Agent(e) => e.exit_code(),
            Error::Io(e) => e.exit_code(),
            Error::State(e) => e.exit_code(),
            Error::Internal(e) => e.exit_code(),
            Error::Task(e) => e.exit_code(),
            Error::Wait(e) => e.exit_code(),
            Error::Lock(e) => e.exit_code(),
        }
    }

    /// Returns a SCREAMING_SNAKE_CASE machine-readable error code.
    ///
    /// This code identifies the error category and variant for programmatic
    /// consumption by AI agents and tooling.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Error::Workspace(e) => workspace_error_code(e),
            Error::Session(e) => session_error_code(e),
            Error::Queue(e) => queue_error_code(e),
            Error::Vcs(e) => vcs_error_code(e),
            Error::Config(e) => config_error_code(e),
            Error::Agent(e) => agent_error_code(e),
            Error::Io(e) => io_error_code(e),
            Error::State(e) => state_error_code(e),
            Error::Internal(e) => internal_error_code(e),
            Error::Task(e) => task_error_code(e),
            Error::Wait(e) => wait_error_code(e),
            Error::Lock(e) => e.code(),
        }
    }

    /// Returns structured context information for this error as a JSON value.
    ///
    /// This provides machine-readable context that can be used by AI agents
    /// or tools to understand the error in detail. Each error variant exposes
    /// its relevant fields in a structured map.
    #[must_use]
    pub fn context_map(&self) -> Option<serde_json::Value> {
        match self {
            Error::Workspace(e) => workspace_context_map(e),
            Error::Session(e) => session_context_map(e),
            Error::Queue(e) => queue_context_map(e),
            Error::Vcs(e) => vcs_context_map(e),
            Error::Config(e) => config_context_map(e),
            Error::Agent(e) => agent_context_map(e),
            Error::Io(e) => io_context_map(e),
            Error::State(e) => state_context_map(e),
            Error::Internal(e) => internal_context_map(e),
            Error::Task(e) => task_context_map(e),
            Error::Wait(e) => wait_context_map(e),
            Error::Lock(e) => lock_context_map(e),
        }
    }
}

// ========================================================================
// Error Code Helpers (for code() method)
// ========================================================================

fn workspace_error_code(e: &super::error_workspace::WorkspaceError) -> &'static str {
    use super::error_workspace::WorkspaceErrorKind;
    match e.kind() {
        WorkspaceErrorKind::NotFound(_) => "WORKSPACE_NOT_FOUND",
        WorkspaceErrorKind::Exists(_) => "WORKSPACE_EXISTS",
        WorkspaceErrorKind::Locked(_, _) => "WORKSPACE_LOCKED",
        WorkspaceErrorKind::Conflict(_) => "WORKSPACE_CONFLICT",
    }
}

fn session_error_code(e: &super::error_workspace::SessionError) -> &'static str {
    use super::error_workspace::SessionErrorKind;
    match e.kind() {
        SessionErrorKind::NotFound(_) => "SESSION_NOT_FOUND",
        SessionErrorKind::Exists(_) => "SESSION_EXISTS",
        SessionErrorKind::Locked(_, _) => "SESSION_LOCKED",
        SessionErrorKind::NotLockHolder(_, _) => "NOT_LOCK_HOLDER",
        SessionErrorKind::InvalidState(_, _, _) => "SESSION_INVALID_STATE",
    }
}

fn queue_error_code(e: &super::error_queue::QueueError) -> &'static str {
    use super::error_queue::QueueErrorKind;
    match e.kind() {
        QueueErrorKind::Empty => "QUEUE_EMPTY",
        QueueErrorKind::ItemNotFound(_) => "QUEUE_ITEM_NOT_FOUND",
        QueueErrorKind::Locked(_) => "QUEUE_LOCKED",
        QueueErrorKind::Processing => "QUEUE_PROCESSING",
        QueueErrorKind::InvalidPosition(_) => "QUEUE_INVALID_POSITION",
        QueueErrorKind::Full(_) => "QUEUE_FULL",
    }
}

fn vcs_error_code(e: &super::error_vcs::VcsError) -> &'static str {
    use super::error_vcs::VcsErrorKind;
    match e.kind() {
        VcsErrorKind::NotInitialized => "VCS_NOT_INITIALIZED",
        VcsErrorKind::Conflict(_, _) => "VCS_CONFLICT",
        VcsErrorKind::PushFailed(_) => "VCS_PUSH_FAILED",
        VcsErrorKind::PullFailed(_) => "VCS_PULL_FAILED",
        VcsErrorKind::RebaseFailed(_) => "VCS_REBASE_FAILED",
        VcsErrorKind::BranchNotFound(_) => "BRANCH_NOT_FOUND",
        VcsErrorKind::BranchExists(_) => "BRANCH_EXISTS",
        VcsErrorKind::CommitNotFound(_) => "COMMIT_NOT_FOUND",
        VcsErrorKind::WorkingCopyDirty => "WORKING_COPY_DIRTY",
        VcsErrorKind::CommitFailed(_) => "VCS_COMMIT_FAILED",
        VcsErrorKind::CheckoutFailed(_) => "VCS_CHECKOUT_FAILED",
        VcsErrorKind::DiffFailed(_) => "VCS_DIFF_FAILED",
        VcsErrorKind::MergeNoCommitId => "VCS_MERGE_NO_COMMIT_ID",
        VcsErrorKind::InitFailed { .. } => "VCS_INIT_FAILED",
    }
}

fn config_error_code(e: &super::error_config::ConfigError) -> &'static str {
    use super::error_config::ConfigErrorKind;
    match e.kind() {
        ConfigErrorKind::ConfigKeyNotFound(_) => "CONFIG_KEY_NOT_FOUND",
        ConfigErrorKind::ConfigParseError(_) => "CONFIG_PARSE_ERROR",
        ConfigErrorKind::ConfigWriteError(_) => "CONFIG_WRITE_ERROR",
        ConfigErrorKind::ConfigScopeError(_) => "CONFIG_SCOPE_ERROR",
        ConfigErrorKind::ConfigLockError(_) => "CONFIG_LOCK_ERROR",
        ConfigErrorKind::NotFound(_) => "CONFIG_NOT_FOUND",
        ConfigErrorKind::Invalid(_) => "CONFIG_INVALID",
        ConfigErrorKind::Permission(_) => "CONFIG_PERMISSION",
        ConfigErrorKind::SecuritySymlink(_) => "CONFIG_SECURITY_SYMLINK",
        ConfigErrorKind::FileTooLarge(_) => "CONFIG_FILE_TOO_LARGE",
        ConfigErrorKind::WatcherError(_) => "CONFIG_WATCHER_ERROR",
        ConfigErrorKind::DeadSymlink(_) => "CONFIG_DEAD_SYMLINK",
    }
}

fn agent_error_code(e: &super::error_agent::AgentError) -> &'static str {
    use super::error_agent::AgentErrorKind;
    match e.kind() {
        AgentErrorKind::NotFound(_) => "AGENT_NOT_FOUND",
        AgentErrorKind::Exists(_) => "AGENT_EXISTS",
        AgentErrorKind::Timeout(_) => "AGENT_TIMEOUT",
    }
}

fn io_error_code(e: &super::error_io::IoError) -> &'static str {
    use super::error_io::IoErrorKind;
    match e.kind() {
        IoErrorKind::Io(_) => "IO_ERROR",
        IoErrorKind::IoError(_) => "IO_ERROR",
        IoErrorKind::JsonParse(_) => "JSON_PARSE_ERROR",
        IoErrorKind::YamlParse(_) => "YAML_PARSE_ERROR",
        IoErrorKind::Database(_) => "DATABASE_ERROR",
    }
}

fn state_error_code(e: &super::error_state::StateError) -> &'static str {
    use super::error_state::StateErrorKind;
    match e.kind() {
        StateErrorKind::InvalidState(_) => "INVALID_STATE",
        StateErrorKind::NotFound(_) => "NOT_FOUND",
        StateErrorKind::ValidationError(_) => "VALIDATION_ERROR",
        StateErrorKind::ValidationFieldError { .. } => "VALIDATION_FIELD_ERROR",
        StateErrorKind::InvalidIdentifier(_) => "INVALID_IDENTIFIER",
    }
}

fn internal_error_code(e: &super::error_internal::InternalError) -> &'static str {
    use super::error_internal::InternalErrorKind;
    match e.kind() {
        InternalErrorKind::Internal(_) => "INTERNAL_ERROR",
        InternalErrorKind::Unimplemented(_) => "UNIMPLEMENTED",
        InternalErrorKind::InvalidConfig(_) => "INVALID_CONFIG",
        InternalErrorKind::CloneFailed(_) => "CLONE_FAILED",
        InternalErrorKind::RecordFailed(_) => "RECORD_FAILED",
        InternalErrorKind::InvalidRepoUrl(_) => "INVALID_REPO_URL",
        InternalErrorKind::InvalidOperation(_) => "INVALID_OPERATION",
    }
}

fn task_error_code(e: &super::error_task::TaskError) -> &'static str {
    use super::error_task::TaskErrorKind;
    match e.kind() {
        TaskErrorKind::NotFound(_) => "TASK_NOT_FOUND",
        TaskErrorKind::AlreadyClaimed(_, _) => "TASK_ALREADY_CLAIMED",
        TaskErrorKind::NotClaimed(_) => "TASK_NOT_CLAIMED",
        TaskErrorKind::Locked(_) => "TASK_LOCKED",
        TaskErrorKind::InvalidId(_) => "TASK_INVALID_ID",
        TaskErrorKind::InvalidStateTransition(_, _) => "TASK_INVALID_STATE_TRANSITION",
    }
}

fn wait_error_code(e: &super::error_wait::WaitError) -> &'static str {
    use super::error_wait::WaitErrorKind;
    match e.kind() {
        WaitErrorKind::Timeout(_, _) => "WAIT_TIMEOUT",
        WaitErrorKind::InvalidWaitMode(_) => "INVALID_WAIT_MODE",
        WaitErrorKind::InvalidSessionName(_) => "INVALID_SESSION_NAME",
        WaitErrorKind::BatchEmpty => "BATCH_EMPTY",
        WaitErrorKind::BatchCommandFailed(_) => "BATCH_COMMAND_FAILED",
        WaitErrorKind::BatchRollbackFailed(_) => "BATCH_ROLLBACK_FAILED",
        WaitErrorKind::CheckpointError(_) => "CHECKPOINT_ERROR",
        WaitErrorKind::BatchSizeExceeded(_) => "BATCH_SIZE_EXCEEDED",
    }
}

// ========================================================================
// Context Map Helpers (for context_map() method)
// ========================================================================

fn workspace_context_map(e: &super::error_workspace::WorkspaceError) -> Option<serde_json::Value> {
    use super::error_workspace::WorkspaceErrorKind;
    match e.kind() {
        WorkspaceErrorKind::NotFound(name) => Some(serde_json::json!({
            "resource_type": "workspace",
            "workspace_name": name,
            "searched_in": "database",
        })),
        WorkspaceErrorKind::Exists(name) => Some(serde_json::json!({
            "resource_type": "workspace",
            "workspace_name": name,
        })),
        WorkspaceErrorKind::Locked(name, holder) => Some(serde_json::json!({
            "workspace_name": name,
            "holder": holder,
        })),
        WorkspaceErrorKind::Conflict(msg) => Some(serde_json::json!({
            "message": msg,
        })),
    }
}

fn session_context_map(e: &super::error_workspace::SessionError) -> Option<serde_json::Value> {
    use super::error_workspace::SessionErrorKind;
    match e.kind() {
        SessionErrorKind::NotFound(name) => Some(serde_json::json!({
            "resource_type": "session",
            "session_name": name,
            "searched_in": "database",
        })),
        SessionErrorKind::Exists(name) => Some(serde_json::json!({
            "resource_type": "session",
            "session_name": name,
        })),
        SessionErrorKind::Locked(session, holder) => Some(serde_json::json!({
            "session": session,
            "holder": holder,
        })),
        SessionErrorKind::NotLockHolder(session, agent_id) => Some(serde_json::json!({
            "session": session,
            "agent_id": agent_id,
        })),
        SessionErrorKind::InvalidState(session, state, expected) => Some(serde_json::json!({
            "session": session,
            "actual_state": state,
            "expected_state": expected,
        })),
    }
}

fn queue_context_map(e: &super::error_queue::QueueError) -> Option<serde_json::Value> {
    use super::error_queue::QueueErrorKind;
    match e.kind() {
        QueueErrorKind::Empty => Some(serde_json::json!({
            "error_type": "queue_empty",
        })),
        QueueErrorKind::ItemNotFound(item) => Some(serde_json::json!({
            "error_type": "queue_item_not_found",
            "item": item,
        })),
        QueueErrorKind::Locked(holder) => Some(serde_json::json!({
            "error_type": "queue_locked",
            "holder": holder,
        })),
        QueueErrorKind::Processing => Some(serde_json::json!({
            "error_type": "queue_processing",
        })),
        QueueErrorKind::InvalidPosition(pos) => Some(serde_json::json!({
            "error_type": "queue_invalid_position",
            "position": pos,
        })),
        QueueErrorKind::Full(size) => Some(serde_json::json!({
            "error_type": "queue_full",
            "max_size": size,
        })),
    }
}

fn vcs_context_map(e: &super::error_vcs::VcsError) -> Option<serde_json::Value> {
    use super::error_vcs::VcsErrorKind;
    match e.kind() {
        VcsErrorKind::NotInitialized => Some(serde_json::json!({
            "error_type": "vcs_not_initialized",
        })),
        VcsErrorKind::Conflict(repo, msg) => Some(serde_json::json!({
            "repo": repo,
            "message": msg,
        })),
        VcsErrorKind::PushFailed(msg)
        | VcsErrorKind::PullFailed(msg)
        | VcsErrorKind::RebaseFailed(msg) => Some(serde_json::json!({
            "operation": "vcs",
            "error": msg,
        })),
        VcsErrorKind::BranchNotFound(branch) => Some(serde_json::json!({
            "resource_type": "branch",
            "branch": branch,
        })),
        VcsErrorKind::BranchExists(branch) => Some(serde_json::json!({
            "resource_type": "branch",
            "branch": branch,
        })),
        VcsErrorKind::CommitNotFound(commit) => Some(serde_json::json!({
            "resource_type": "commit",
            "commit": commit,
        })),
        VcsErrorKind::WorkingCopyDirty => Some(serde_json::json!({
            "error_type": "working_copy_dirty",
        })),
        VcsErrorKind::CommitFailed(msg)
        | VcsErrorKind::CheckoutFailed(msg)
        | VcsErrorKind::DiffFailed(msg) => Some(serde_json::json!({
            "operation": "vcs",
            "error": msg,
        })),
        VcsErrorKind::MergeNoCommitId => Some(serde_json::json!({
            "error_type": "merge_no_commit_id",
        })),
        VcsErrorKind::InitFailed {
            vcs_type,
            directory,
            reason,
        } => Some(serde_json::json!({
            "vcs_type": vcs_type,
            "directory": directory,
            "reason": reason,
        })),
    }
}

fn config_context_map(e: &super::error_config::ConfigError) -> Option<serde_json::Value> {
    use super::error_config::ConfigErrorKind;
    match e.kind() {
        ConfigErrorKind::ConfigKeyNotFound(key)
        | ConfigErrorKind::ConfigParseError(key)
        | ConfigErrorKind::ConfigWriteError(key)
        | ConfigErrorKind::ConfigScopeError(key)
        | ConfigErrorKind::ConfigLockError(key)
        | ConfigErrorKind::NotFound(key)
        | ConfigErrorKind::Invalid(key)
        | ConfigErrorKind::Permission(key)
        | ConfigErrorKind::SecuritySymlink(key)
        | ConfigErrorKind::FileTooLarge(key)
        | ConfigErrorKind::WatcherError(key)
        | ConfigErrorKind::DeadSymlink(key) => Some(serde_json::json!({
            "message": key,
        })),
    }
}

fn agent_context_map(e: &super::error_agent::AgentError) -> Option<serde_json::Value> {
    use super::error_agent::AgentErrorKind;
    match e.kind() {
        AgentErrorKind::NotFound(id) | AgentErrorKind::Exists(id) | AgentErrorKind::Timeout(id) => {
            Some(serde_json::json!({
                "agent_id": id,
            }))
        }
    }
}

fn io_context_map(e: &super::error_io::IoError) -> Option<serde_json::Value> {
    use super::error_io::IoErrorKind;
    match e.kind() {
        IoErrorKind::Io(err) => Some(serde_json::json!({
            "operation": "file_io",
            "error": err.to_string(),
        })),
        IoErrorKind::IoError(msg) | IoErrorKind::Database(msg) => Some(serde_json::json!({
            "error_type": "io_error",
            "message": msg,
        })),
        IoErrorKind::JsonParse(err) => Some(serde_json::json!({
            "operation": "json_parse",
            "error": err.to_string(),
        })),
        IoErrorKind::YamlParse(err) => Some(serde_json::json!({
            "operation": "yaml_parse",
            "error": err.to_string(),
        })),
    }
}

fn state_context_map(e: &super::error_state::StateError) -> Option<serde_json::Value> {
    use super::error_state::StateErrorKind;
    match e.kind() {
        StateErrorKind::InvalidState(msg) => Some(serde_json::json!({
            "error_type": "invalid_state",
            "message": msg,
        })),
        StateErrorKind::NotFound(msg) => Some(serde_json::json!({
            "resource_type": "resource",
            "resource_id": msg,
            "searched_in": "database",
        })),
        StateErrorKind::ValidationError(msg) => Some(serde_json::json!({
            "error_type": "validation_error",
            "message": msg,
        })),
        StateErrorKind::ValidationFieldError {
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
        StateErrorKind::InvalidIdentifier(msg) => Some(serde_json::json!({
            "error_type": "invalid_identifier",
            "message": msg,
        })),
    }
}

fn internal_context_map(e: &super::error_internal::InternalError) -> Option<serde_json::Value> {
    use super::error_internal::InternalErrorKind;
    match e.kind() {
        InternalErrorKind::Internal(msg)
        | InternalErrorKind::Unimplemented(msg)
        | InternalErrorKind::CloneFailed(msg)
        | InternalErrorKind::RecordFailed(msg)
        | InternalErrorKind::InvalidRepoUrl(msg)
        | InternalErrorKind::InvalidOperation(msg)
        | InternalErrorKind::InvalidConfig(msg) => Some(serde_json::json!({
            "error_type": "internal_error",
            "message": msg,
        })),
    }
}

fn task_context_map(e: &super::error_task::TaskError) -> Option<serde_json::Value> {
    use super::error_task::TaskErrorKind;
    match e.kind() {
        TaskErrorKind::NotFound(id)
        | TaskErrorKind::Locked(id)
        | TaskErrorKind::InvalidId(id)
        | TaskErrorKind::NotClaimed(id) => Some(serde_json::json!({
            "task_id": id,
        })),
        TaskErrorKind::AlreadyClaimed(id, agent) => Some(serde_json::json!({
            "task_id": id,
            "claimed_by": agent,
        })),
        TaskErrorKind::InvalidStateTransition(id, transition) => Some(serde_json::json!({
            "task_id": id,
            "transition": transition,
        })),
    }
}

fn wait_context_map(e: &super::error_wait::WaitError) -> Option<serde_json::Value> {
    use super::error_wait::WaitErrorKind;
    match e.kind() {
        WaitErrorKind::Timeout(session, waiting_for) => Some(serde_json::json!({
            "session": session,
            "waiting_for": waiting_for,
        })),
        WaitErrorKind::InvalidWaitMode(mode) => Some(serde_json::json!({
            "message": mode,
        })),
        WaitErrorKind::InvalidSessionName(name) => Some(serde_json::json!({
            "message": name,
        })),
        WaitErrorKind::BatchEmpty => Some(serde_json::json!({
            "error_type": "batch_empty",
        })),
        WaitErrorKind::BatchCommandFailed(msg)
        | WaitErrorKind::BatchRollbackFailed(msg)
        | WaitErrorKind::CheckpointError(msg) => Some(serde_json::json!({
            "message": msg,
        })),
        WaitErrorKind::BatchSizeExceeded(size) => Some(serde_json::json!({
            "max_size": size,
        })),
    }
}

fn lock_context_map(
    e: &super::coordination::locks::errors::LockError,
) -> Option<serde_json::Value> {
    use super::coordination::locks::errors::LockErrorKind;
    match e.kind() {
        LockErrorKind::SessionNotFound { session } => Some(serde_json::json!({
            "resource_type": "session",
            "session_name": session,
        })),
        LockErrorKind::SessionLocked { session, holder } => Some(serde_json::json!({
            "session": session,
            "holder": holder,
        })),
        LockErrorKind::NotLockHolder { session, agent_id } => Some(serde_json::json!({
            "session": session,
            "agent_id": agent_id,
        })),
        LockErrorKind::NotFound(msg)
        | LockErrorKind::DatabaseError(msg)
        | LockErrorKind::ParseError(msg)
        | LockErrorKind::Unknown(msg)
        | LockErrorKind::TtlOutOfRange(msg)
        | LockErrorKind::EmptySessionName(msg)
        | LockErrorKind::EmptyAgentId(msg)
        | LockErrorKind::TtlOverflow(msg)
        | LockErrorKind::SessionNameTooLong(msg)
        | LockErrorKind::InvalidSessionName(msg) => Some(serde_json::json!({
            "message": msg,
        })),
    }
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error_queue::QueueErrorKind, error_vcs::VcsErrorKind, error_workspace::WorkspaceErrorKind,
    };

    #[test]
    fn test_error_suggestions() {
        let err = Error::from(WorkspaceErrorKind::NotFound("test".to_string()));
        assert!(err.suggestion().is_some());

        let err = Error::from(QueueErrorKind::Empty);
        assert!(err.suggestion().is_some());
    }

    #[test]
    fn test_exit_codes() {
        let err = Error::from(WorkspaceErrorKind::NotFound("x".to_string()));
        assert_eq!(err.exit_code(), 10);

        let err = Error::from(QueueErrorKind::Empty);
        assert_eq!(err.exit_code(), 20);

        let err = Error::from(VcsErrorKind::NotInitialized);
        assert_eq!(err.exit_code(), 30);
    }

    #[test]
    fn test_backwards_compat_constructors() {
        let err = Error::database("test");
        assert_eq!(err.exit_code(), 63);

        let err = Error::invalid_identifier("test");
        assert_eq!(err.exit_code(), 82);

        let err = Error::validation_field_error("field", "msg", None);
        assert_eq!(err.exit_code(), 81);
    }

    #[test]
    fn test_error_code_workspace() {
        let err = Error::workspace_not_found("test-workspace");
        assert_eq!(err.code(), "WORKSPACE_NOT_FOUND");

        let err = Error::workspace_exists("test-workspace");
        assert_eq!(err.code(), "WORKSPACE_EXISTS");

        let err = Error::workspace_locked("ws", "holder");
        assert_eq!(err.code(), "WORKSPACE_LOCKED");
    }

    #[test]
    fn test_error_code_session() {
        let err = Error::session("test-session");
        assert_eq!(err.code(), "SESSION_NOT_FOUND");

        let err = Error::session_locked("s", "h");
        assert_eq!(err.code(), "SESSION_LOCKED");

        let err = Error::session_invalid_state("s", "active", "inactive");
        assert_eq!(err.code(), "SESSION_INVALID_STATE");
    }

    #[test]
    fn test_error_code_vcs() {
        let err = Error::vcs_not_initialized();
        assert_eq!(err.code(), "VCS_NOT_INITIALIZED");

        let err = Error::working_copy_dirty();
        assert_eq!(err.code(), "WORKING_COPY_DIRTY");
    }

    #[test]
    fn test_error_code_io() {
        let err = Error::io_error("disk full");
        assert_eq!(err.code(), "IO_ERROR");

        let err = Error::database("db corrupt");
        assert_eq!(err.code(), "DATABASE_ERROR");
    }

    #[test]
    fn test_error_code_state() {
        let err = Error::not_found("missing resource");
        assert_eq!(err.code(), "NOT_FOUND");

        let err = Error::validation_error("bad input");
        assert_eq!(err.code(), "VALIDATION_ERROR");

        let err = Error::invalid_identifier("bad-id!");
        assert_eq!(err.code(), "INVALID_IDENTIFIER");
    }

    #[test]
    fn test_context_map_workspace_not_found() {
        let err = Error::workspace_not_found("my-workspace");
        let ctx = err.context_map().expect("should have context");

        assert_eq!(ctx["resource_type"], "workspace");
        assert_eq!(ctx["workspace_name"], "my-workspace");
        assert_eq!(ctx["searched_in"], "database");
    }

    #[test]
    fn test_context_map_session_locked() {
        let err = Error::session_locked("my-session", "agent-001");
        let ctx = err.context_map().expect("should have context");

        assert_eq!(ctx["session"], "my-session");
        assert_eq!(ctx["holder"], "agent-001");
    }

    #[test]
    fn test_context_map_vcs_not_initialized() {
        let err = Error::vcs_not_initialized();
        let ctx = err.context_map().expect("should have context");

        assert_eq!(ctx["error_type"], "vcs_not_initialized");
    }

    #[test]
    fn test_context_map_validation_field_error() {
        let err = Error::validation_field_error("name", "too short", Some("ab".to_string()));
        let ctx = err.context_map().expect("should have context");

        assert_eq!(ctx["field"], "name");
        assert_eq!(ctx["message"], "too short");
        assert_eq!(ctx["value"], "ab");
    }

    #[test]
    fn test_context_map_validation_field_error_no_value() {
        let err = Error::validation_field_error("name", "required", None);
        let ctx = err.context_map().expect("should have context");

        assert_eq!(ctx["field"], "name");
        assert!(!ctx.as_object().map_or(true, |o| o.contains_key("value")));
    }

    #[test]
    fn test_context_map_not_found() {
        let err = Error::not_found("session-xyz");
        let ctx = err.context_map().expect("should have context");

        assert_eq!(ctx["resource_type"], "resource");
        assert_eq!(ctx["resource_id"], "session-xyz");
        assert_eq!(ctx["searched_in"], "database");
    }

    // ── Comprehensive Error construction for each variant ─────────────

    #[test]
    fn test_error_construction_workspace_variants() {
        let err = Error::workspace_not_found("ws1");
        assert!(matches!(err, Error::Workspace(_)));
        assert!(err.to_string().contains("ws1"));

        let err = Error::workspace_exists("ws2");
        assert!(matches!(err, Error::Workspace(_)));

        let err = Error::workspace_locked("ws3", "holder1");
        assert!(matches!(err, Error::Workspace(_)));
        assert!(err.to_string().contains("ws3"));
        assert!(err.to_string().contains("holder1"));

        let err = Error::workspace_conflict("merge conflict");
        assert!(matches!(err, Error::Workspace(_)));
    }

    #[test]
    fn test_error_construction_session_variants() {
        let err = Error::session("s1");
        assert!(matches!(err, Error::Session(_)));

        let err = Error::session_exists("s2");
        assert!(matches!(err, Error::Session(_)));

        let err = Error::session_locked("s3", "agent1");
        assert!(matches!(err, Error::Session(_)));

        let err = Error::not_lock_holder("s4", "agent2");
        assert!(matches!(err, Error::Session(_)));

        let err = Error::session_invalid_state("s5", "active", "idle");
        assert!(matches!(err, Error::Session(_)));
    }

    #[test]
    fn test_error_construction_queue_variants() {
        assert!(matches!(Error::queue_empty(), Error::Queue(_)));
        assert!(matches!(Error::queue_item_not_found("i1"), Error::Queue(_)));
        assert!(matches!(Error::queue_locked("h1"), Error::Queue(_)));
        assert!(matches!(Error::queue_processing(), Error::Queue(_)));
        assert!(matches!(Error::queue_invalid_position(5), Error::Queue(_)));
        assert!(matches!(Error::queue_full(100), Error::Queue(_)));
    }

    #[test]
    fn test_error_construction_vcs_variants() {
        assert!(matches!(Error::vcs_not_initialized(), Error::Vcs(_)));
        assert!(matches!(Error::vcs_conflict("repo", "msg"), Error::Vcs(_)));
        assert!(matches!(Error::vcs_push_failed("msg"), Error::Vcs(_)));
        assert!(matches!(Error::vcs_pull_failed("msg"), Error::Vcs(_)));
        assert!(matches!(Error::vcs_rebase_failed("msg"), Error::Vcs(_)));
        assert!(matches!(Error::branch_not_found("b1"), Error::Vcs(_)));
        assert!(matches!(Error::branch_exists("b2"), Error::Vcs(_)));
        assert!(matches!(Error::commit_not_found("c1"), Error::Vcs(_)));
        assert!(matches!(Error::working_copy_dirty(), Error::Vcs(_)));
        assert!(matches!(Error::vcs_commit_failed("msg"), Error::Vcs(_)));
        assert!(matches!(Error::vcs_checkout_failed("msg"), Error::Vcs(_)));
        assert!(matches!(Error::vcs_diff_failed("msg"), Error::Vcs(_)));
        assert!(matches!(
            Error::vcs_init_failed("git", "/tmp", "err"),
            Error::Vcs(_)
        ));
    }

    #[test]
    fn test_error_construction_config_variants() {
        assert!(matches!(Error::config_not_found("msg"), Error::Config(_)));
        assert!(matches!(Error::config_invalid("msg"), Error::Config(_)));
        assert!(matches!(Error::config_permission("msg"), Error::Config(_)));
    }

    #[test]
    fn test_error_construction_agent_variants() {
        assert!(matches!(Error::agent_not_found("a1"), Error::Agent(_)));
        assert!(matches!(Error::agent_exists("a2"), Error::Agent(_)));
    }

    #[test]
    fn test_error_construction_io_variants() {
        assert!(matches!(Error::io_error("disk full"), Error::Io(_)));
        assert!(matches!(Error::database("corrupt"), Error::Io(_)));
    }

    #[test]
    fn test_error_construction_state_variants() {
        assert!(matches!(Error::invalid_state("bad state"), Error::State(_)));
        assert!(matches!(Error::not_found("missing"), Error::State(_)));
        assert!(matches!(
            Error::validation_error("bad input"),
            Error::State(_)
        ));
        assert!(matches!(
            Error::validation_field_error("f", "m", Some("v".into())),
            Error::State(_)
        ));
        assert!(matches!(
            Error::invalid_identifier("bad-id"),
            Error::State(_)
        ));
    }

    #[test]
    fn test_error_construction_internal_variants() {
        assert!(matches!(Error::internal("oops"), Error::Internal(_)));
        assert!(matches!(Error::unimplemented("todo"), Error::Internal(_)));
    }

    #[test]
    fn test_error_construction_wait_variants() {
        assert!(matches!(Error::batch_empty(), Error::Wait(_)));
        assert!(matches!(Error::batch_command_failed("msg"), Error::Wait(_)));
        assert!(matches!(
            Error::batch_rollback_failed("msg"),
            Error::Wait(_)
        ));
        assert!(matches!(Error::batch_size_exceeded(10), Error::Wait(_)));
        assert!(matches!(Error::checkpoint_error("msg"), Error::Wait(_)));
    }

    #[test]
    fn test_error_construction_lock_variant() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: Error = LockErrorKind::SessionNotFound {
            session: "s1".to_string(),
        }
        .into();
        assert!(matches!(err, Error::Lock(_)));

        let err: Error = LockErrorKind::SessionLocked {
            session: "s2".to_string(),
            holder: "h1".to_string(),
        }
        .into();
        assert!(matches!(err, Error::Lock(_)));
    }

    // ── error_code() returns non-zero for all variants ────────────────

    #[test]
    fn test_error_code_non_zero_all_variants() {
        // Workspace
        assert_ne!(Error::workspace_not_found("w").exit_code(), 0);
        assert_ne!(Error::workspace_exists("w").exit_code(), 0);
        assert_ne!(Error::workspace_locked("w", "h").exit_code(), 0);
        assert_ne!(Error::workspace_conflict("m").exit_code(), 0);

        // Session
        assert_ne!(Error::session("s").exit_code(), 0);
        assert_ne!(Error::session_exists("s").exit_code(), 0);
        assert_ne!(Error::session_locked("s", "h").exit_code(), 0);
        assert_ne!(Error::not_lock_holder("s", "a").exit_code(), 0);
        assert_ne!(Error::session_invalid_state("s", "a", "b").exit_code(), 0);

        // Queue
        assert_ne!(Error::queue_empty().exit_code(), 0);
        assert_ne!(Error::queue_item_not_found("i").exit_code(), 0);
        assert_ne!(Error::queue_locked("h").exit_code(), 0);
        assert_ne!(Error::queue_processing().exit_code(), 0);
        assert_ne!(Error::queue_invalid_position(1).exit_code(), 0);
        assert_ne!(Error::queue_full(10).exit_code(), 0);

        // VCS
        assert_ne!(Error::vcs_not_initialized().exit_code(), 0);
        assert_ne!(Error::vcs_conflict("r", "m").exit_code(), 0);
        assert_ne!(Error::vcs_push_failed("m").exit_code(), 0);
        assert_ne!(Error::vcs_pull_failed("m").exit_code(), 0);
        assert_ne!(Error::vcs_rebase_failed("m").exit_code(), 0);
        assert_ne!(Error::branch_not_found("b").exit_code(), 0);
        assert_ne!(Error::branch_exists("b").exit_code(), 0);
        assert_ne!(Error::commit_not_found("c").exit_code(), 0);
        assert_ne!(Error::working_copy_dirty().exit_code(), 0);
        assert_ne!(Error::vcs_commit_failed("m").exit_code(), 0);
        assert_ne!(Error::vcs_checkout_failed("m").exit_code(), 0);
        assert_ne!(Error::vcs_diff_failed("m").exit_code(), 0);
        assert_ne!(Error::vcs_init_failed("git", "/tmp", "err").exit_code(), 0);

        // Config
        assert_ne!(Error::config_not_found("m").exit_code(), 0);
        assert_ne!(Error::config_invalid("m").exit_code(), 0);
        assert_ne!(Error::config_permission("m").exit_code(), 0);

        // Agent
        assert_ne!(Error::agent_not_found("a").exit_code(), 0);
        assert_ne!(Error::agent_exists("a").exit_code(), 0);

        // IO
        assert_ne!(Error::io_error("m").exit_code(), 0);
        assert_ne!(Error::database("m").exit_code(), 0);

        // State
        assert_ne!(Error::invalid_state("m").exit_code(), 0);
        assert_ne!(Error::not_found("m").exit_code(), 0);
        assert_ne!(Error::validation_error("m").exit_code(), 0);
        assert_ne!(Error::validation_field_error("f", "m", None).exit_code(), 0);
        assert_ne!(Error::invalid_identifier("m").exit_code(), 0);

        // Internal
        assert_ne!(Error::internal("m").exit_code(), 0);
        assert_ne!(Error::unimplemented("m").exit_code(), 0);

        // Wait
        assert_ne!(Error::batch_empty().exit_code(), 0);
        assert_ne!(Error::batch_command_failed("m").exit_code(), 0);
        assert_ne!(Error::batch_rollback_failed("m").exit_code(), 0);
        assert_ne!(Error::batch_size_exceeded(10).exit_code(), 0);
        assert_ne!(Error::checkpoint_error("m").exit_code(), 0);

        // Lock
        use crate::coordination::locks::errors::LockErrorKind;
        assert_ne!(
            Error::from(LockErrorKind::SessionNotFound {
                session: "s".into()
            })
            .exit_code(),
            0
        );
    }

    // ── context_map() returns Some for all variants ───────────────────

    #[test]
    fn test_context_map_returns_some_all_variants() {
        // Workspace
        assert!(Error::workspace_not_found("w").context_map().is_some());
        assert!(Error::workspace_exists("w").context_map().is_some());
        assert!(Error::workspace_locked("w", "h").context_map().is_some());
        assert!(Error::workspace_conflict("m").context_map().is_some());

        // Session
        assert!(Error::session("s").context_map().is_some());
        assert!(Error::session_exists("s").context_map().is_some());
        assert!(Error::session_locked("s", "h").context_map().is_some());
        assert!(Error::not_lock_holder("s", "a").context_map().is_some());
        assert!(Error::session_invalid_state("s", "a", "b")
            .context_map()
            .is_some());

        // Queue
        assert!(Error::queue_empty().context_map().is_some());
        assert!(Error::queue_item_not_found("i").context_map().is_some());
        assert!(Error::queue_locked("h").context_map().is_some());
        assert!(Error::queue_processing().context_map().is_some());
        assert!(Error::queue_invalid_position(1).context_map().is_some());
        assert!(Error::queue_full(10).context_map().is_some());

        // VCS
        assert!(Error::vcs_not_initialized().context_map().is_some());
        assert!(Error::vcs_conflict("r", "m").context_map().is_some());
        assert!(Error::vcs_push_failed("m").context_map().is_some());
        assert!(Error::branch_not_found("b").context_map().is_some());
        assert!(Error::working_copy_dirty().context_map().is_some());
        assert!(Error::vcs_commit_failed("m").context_map().is_some());
        assert!(Error::vcs_init_failed("git", "/tmp", "err")
            .context_map()
            .is_some());

        // Config
        assert!(Error::config_not_found("m").context_map().is_some());
        assert!(Error::config_invalid("m").context_map().is_some());

        // Agent
        assert!(Error::agent_not_found("a").context_map().is_some());
        assert!(Error::agent_exists("a").context_map().is_some());

        // IO
        assert!(Error::io_error("m").context_map().is_some());
        assert!(Error::database("m").context_map().is_some());

        // State
        assert!(Error::invalid_state("m").context_map().is_some());
        assert!(Error::not_found("m").context_map().is_some());
        assert!(Error::validation_error("m").context_map().is_some());
        assert!(Error::validation_field_error("f", "m", None)
            .context_map()
            .is_some());
        assert!(Error::invalid_identifier("m").context_map().is_some());

        // Internal
        assert!(Error::internal("m").context_map().is_some());
        assert!(Error::unimplemented("m").context_map().is_some());

        // Wait
        assert!(Error::batch_empty().context_map().is_some());
        assert!(Error::batch_command_failed("m").context_map().is_some());
        assert!(Error::checkpoint_error("m").context_map().is_some());

        // Lock
        use crate::coordination::locks::errors::LockErrorKind;
        assert!(Error::from(LockErrorKind::SessionNotFound {
            session: "s".into()
        })
        .context_map()
        .is_some());
        assert!(Error::from(LockErrorKind::DatabaseError("db".into()))
            .context_map()
            .is_some());
    }

    // ── suggestion() returns appropriate suggestions ──────────────────

    #[test]
    fn test_suggestion_workspace_not_found() {
        let err = Error::workspace_not_found("ws");
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("scp workspace list"));
    }

    #[test]
    fn test_suggestion_workspace_locked() {
        let err = Error::workspace_locked("ws", "holder1");
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("holder1"));
        assert!(s.contains("kill"));
    }

    #[test]
    fn test_suggestion_session_not_found() {
        let err = Error::session("s");
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("scp session list"));
    }

    #[test]
    fn test_suggestion_session_locked() {
        let err = Error::session_locked("s", "holder1");
        assert!(err.suggestion().is_none()); // Session locked has no generic suggestion
    }

    #[test]
    fn test_suggestion_queue_empty() {
        let err = Error::queue_empty();
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("queue enqueue"));
    }

    #[test]
    fn test_suggestion_vcs_not_initialized() {
        let err = Error::vcs_not_initialized();
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("scp init"));
    }

    #[test]
    fn test_suggestion_working_copy_dirty() {
        let err = Error::working_copy_dirty();
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("commit") || s.contains("stash"));
    }

    #[test]
    fn test_suggestion_state_not_found() {
        let err = Error::not_found("resource-xyz");
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("scp session list"));
    }

    #[test]
    fn test_suggestion_none_for_variants_without_suggestions() {
        // Config errors have no suggestion
        assert!(Error::config_not_found("x").suggestion().is_none());
        assert!(Error::config_invalid("x").suggestion().is_none());

        // Agent errors have no suggestion
        assert!(Error::agent_not_found("x").suggestion().is_none());
        assert!(Error::agent_exists("x").suggestion().is_none());

        // IO errors have no suggestion
        assert!(Error::io_error("x").suggestion().is_none());
        assert!(Error::database("x").suggestion().is_none());

        // Internal errors have no suggestion
        assert!(Error::internal("x").suggestion().is_none());
        assert!(Error::unimplemented("x").suggestion().is_none());

        // Task errors have no suggestion
        // (can't construct directly, but tested via kind)

        // Wait errors have no suggestion
        assert!(Error::batch_empty().suggestion().is_none());
        assert!(Error::batch_command_failed("x").suggestion().is_none());

        // State validation errors (except NotFound) have no suggestion
        assert!(Error::invalid_state("x").suggestion().is_none());
        assert!(Error::validation_error("x").suggestion().is_none());
        assert!(Error::validation_field_error("f", "m", None)
            .suggestion()
            .is_none());
        assert!(Error::invalid_identifier("x").suggestion().is_none());
    }

    #[test]
    fn test_suggestion_lock_session_locked() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: Error = LockErrorKind::SessionLocked {
            session: "s".to_string(),
            holder: "h1".to_string(),
        }
        .into();
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("h1"));
        assert!(s.contains("kill"));
    }

    #[test]
    fn test_suggestion_lock_session_not_found() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: Error = LockErrorKind::SessionNotFound {
            session: "s".to_string(),
        }
        .into();
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("scp session list"));
    }

    // ── Display impl correctness ──────────────────────────────────────

    #[test]
    fn test_display_workspace_errors() {
        assert!(Error::workspace_not_found("ws")
            .to_string()
            .contains("Workspace not found"));
        assert!(Error::workspace_not_found("ws").to_string().contains("ws"));
        assert!(Error::workspace_exists("ws")
            .to_string()
            .contains("already exists"));
        assert!(Error::workspace_locked("ws", "h")
            .to_string()
            .contains("locked"));
        assert!(Error::workspace_conflict("msg")
            .to_string()
            .contains("conflict"));
    }

    #[test]
    fn test_display_session_errors() {
        assert!(Error::session("s")
            .to_string()
            .contains("Session not found"));
        assert!(Error::session_exists("s")
            .to_string()
            .contains("already exists"));
        assert!(Error::session_locked("s", "h")
            .to_string()
            .contains("locked"));
        assert!(Error::not_lock_holder("s", "a")
            .to_string()
            .contains("does not hold lock"));
        assert!(Error::session_invalid_state("s", "active", "idle")
            .to_string()
            .contains("expected idle"));
    }

    #[test]
    fn test_display_queue_errors() {
        assert!(Error::queue_empty().to_string().contains("empty"));
        assert!(Error::queue_item_not_found("i")
            .to_string()
            .contains("not found"));
        assert!(Error::queue_locked("h").to_string().contains("locked"));
        assert!(Error::queue_processing()
            .to_string()
            .contains("in progress"));
        assert!(Error::queue_full(10).to_string().contains("10"));
    }

    #[test]
    fn test_display_vcs_errors() {
        assert!(Error::vcs_not_initialized()
            .to_string()
            .contains("not initialized"));
        assert!(Error::vcs_conflict("repo", "msg")
            .to_string()
            .contains("conflict"));
        assert!(Error::vcs_push_failed("msg").to_string().contains("push"));
        assert!(Error::working_copy_dirty()
            .to_string()
            .contains("uncommitted"));
        assert!(Error::vcs_init_failed("git", "/tmp", "err")
            .to_string()
            .contains("git"));
        assert!(Error::vcs_init_failed("git", "/tmp", "err")
            .to_string()
            .contains("/tmp"));
    }

    #[test]
    fn test_display_state_errors() {
        assert!(Error::invalid_state("msg")
            .to_string()
            .contains("Invalid state"));
        assert!(Error::not_found("x").to_string().contains("Not found"));
        assert!(Error::validation_error("x")
            .to_string()
            .contains("Validation"));
        assert!(Error::invalid_identifier("x")
            .to_string()
            .contains("Invalid identifier"));
        let err = Error::validation_field_error("name", "too short", Some("ab".into()));
        let display = err.to_string();
        assert!(display.contains("name"));
        assert!(display.contains("too short"));
    }

    #[test]
    fn test_display_internal_errors() {
        assert!(Error::internal("oops")
            .to_string()
            .contains("Internal error"));
        assert!(Error::unimplemented("todo")
            .to_string()
            .contains("Not implemented"));
    }

    #[test]
    fn test_display_config_errors() {
        assert!(Error::config_not_found("file.toml")
            .to_string()
            .contains("not found"));
        assert!(Error::config_invalid("bad").to_string().contains("invalid"));
        assert!(Error::config_permission("denied")
            .to_string()
            .contains("permission"));
    }

    #[test]
    fn test_display_agent_errors() {
        assert!(Error::agent_not_found("a1")
            .to_string()
            .contains("not found"));
        assert!(Error::agent_exists("a2")
            .to_string()
            .contains("already registered"));
    }

    #[test]
    fn test_display_io_errors() {
        assert!(Error::io_error("disk full")
            .to_string()
            .contains("IO error"));
        assert!(Error::database("corrupt")
            .to_string()
            .contains("Database error"));
    }

    #[test]
    fn test_display_wait_errors() {
        assert!(Error::batch_empty().to_string().contains("empty"));
        assert!(Error::batch_command_failed("err")
            .to_string()
            .contains("failed"));
        assert!(Error::batch_size_exceeded(10).to_string().contains("10"));
        assert!(Error::checkpoint_error("err")
            .to_string()
            .contains("Checkpoint"));
    }

    #[test]
    fn test_display_lock_errors() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: Error = LockErrorKind::SessionLocked {
            session: "s".into(),
            holder: "h".into(),
        }
        .into();
        assert!(err.to_string().contains("locked"));
        assert!(err.to_string().contains("s"));
        assert!(err.to_string().contains("h"));
    }

    // ── error_code() SCREAMING_SNAKE_CASE correctness ────────────────

    #[test]
    fn test_error_code_queue_variants() {
        assert_eq!(Error::queue_empty().code(), "QUEUE_EMPTY");
        assert_eq!(
            Error::queue_item_not_found("i").code(),
            "QUEUE_ITEM_NOT_FOUND"
        );
        assert_eq!(Error::queue_locked("h").code(), "QUEUE_LOCKED");
        assert_eq!(Error::queue_processing().code(), "QUEUE_PROCESSING");
        assert_eq!(
            Error::queue_invalid_position(1).code(),
            "QUEUE_INVALID_POSITION"
        );
        assert_eq!(Error::queue_full(10).code(), "QUEUE_FULL");
    }

    #[test]
    fn test_error_code_config_variants() {
        assert_eq!(Error::config_not_found("x").code(), "CONFIG_NOT_FOUND");
        assert_eq!(Error::config_invalid("x").code(), "CONFIG_INVALID");
        assert_eq!(Error::config_permission("x").code(), "CONFIG_PERMISSION");
    }

    #[test]
    fn test_error_code_agent_variants() {
        assert_eq!(Error::agent_not_found("a").code(), "AGENT_NOT_FOUND");
        assert_eq!(Error::agent_exists("a").code(), "AGENT_EXISTS");
    }

    #[test]
    fn test_error_code_state_variants() {
        assert_eq!(Error::invalid_state("m").code(), "INVALID_STATE");
        assert_eq!(Error::not_found("m").code(), "NOT_FOUND");
        assert_eq!(Error::validation_error("m").code(), "VALIDATION_ERROR");
        assert_eq!(
            Error::validation_field_error("f", "m", None).code(),
            "VALIDATION_FIELD_ERROR"
        );
        assert_eq!(Error::invalid_identifier("m").code(), "INVALID_IDENTIFIER");
    }

    #[test]
    fn test_error_code_internal_variants() {
        assert_eq!(Error::internal("m").code(), "INTERNAL_ERROR");
        assert_eq!(Error::unimplemented("m").code(), "UNIMPLEMENTED");
    }

    #[test]
    fn test_error_code_task_variants() {
        use crate::error_task::TaskErrorKind;
        assert_eq!(
            Error::from(TaskErrorKind::NotFound("t".into())).code(),
            "TASK_NOT_FOUND"
        );
        assert_eq!(
            Error::from(TaskErrorKind::AlreadyClaimed("t".into(), "a".into())).code(),
            "TASK_ALREADY_CLAIMED"
        );
        assert_eq!(
            Error::from(TaskErrorKind::Locked("t".into())).code(),
            "TASK_LOCKED"
        );
        assert_eq!(
            Error::from(TaskErrorKind::InvalidId("t".into())).code(),
            "TASK_INVALID_ID"
        );
    }

    #[test]
    fn test_error_code_wait_variants() {
        use crate::error_wait::WaitErrorKind;
        assert_eq!(Error::from(WaitErrorKind::BatchEmpty).code(), "BATCH_EMPTY");
        assert_eq!(
            Error::from(WaitErrorKind::BatchCommandFailed("m".into())).code(),
            "BATCH_COMMAND_FAILED"
        );
        assert_eq!(
            Error::from(WaitErrorKind::BatchRollbackFailed("m".into())).code(),
            "BATCH_ROLLBACK_FAILED"
        );
        assert_eq!(
            Error::from(WaitErrorKind::CheckpointError("m".into())).code(),
            "CHECKPOINT_ERROR"
        );
        assert_eq!(
            Error::from(WaitErrorKind::BatchSizeExceeded(10)).code(),
            "BATCH_SIZE_EXCEEDED"
        );
    }

    #[test]
    fn test_error_code_lock_variants() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: Error = LockErrorKind::SessionNotFound {
            session: "s".into(),
        }
        .into();
        assert_eq!(err.code(), "SESSION_NOT_FOUND");

        let err: Error = LockErrorKind::SessionLocked {
            session: "s".into(),
            holder: "h".into(),
        }
        .into();
        assert_eq!(err.code(), "SESSION_LOCKED");

        let err: Error = LockErrorKind::NotLockHolder {
            session: "s".into(),
            agent_id: "a".into(),
        }
        .into();
        assert_eq!(err.code(), "NOT_LOCK_HOLDER");

        let err: Error = LockErrorKind::DatabaseError("db".into()).into();
        assert_eq!(err.code(), "DATABASE_ERROR");
    }

    // ── context_map field correctness for additional variants ─────────

    #[test]
    fn test_context_map_queue_item_not_found() {
        let err = Error::queue_item_not_found("item-1");
        let ctx = err.context_map().expect("should have context");
        assert_eq!(ctx["error_type"], "queue_item_not_found");
        assert_eq!(ctx["item"], "item-1");
    }

    #[test]
    fn test_context_map_queue_full() {
        let err = Error::queue_full(100);
        let ctx = err.context_map().expect("should have context");
        assert_eq!(ctx["error_type"], "queue_full");
        assert_eq!(ctx["max_size"], 100);
    }

    #[test]
    fn test_context_map_vcs_branch_not_found() {
        let err = Error::branch_not_found("feat");
        let ctx = err.context_map().expect("should have context");
        assert_eq!(ctx["resource_type"], "branch");
        assert_eq!(ctx["branch"], "feat");
    }

    #[test]
    fn test_context_map_vcs_init_failed() {
        let err = Error::vcs_init_failed("git", "/home/project", "permission denied");
        let ctx = err.context_map().expect("should have context");
        assert_eq!(ctx["vcs_type"], "git");
        assert_eq!(ctx["directory"], "/home/project");
        assert_eq!(ctx["reason"], "permission denied");
    }

    #[test]
    fn test_context_map_agent_not_found() {
        let err = Error::agent_not_found("agent-1");
        let ctx = err.context_map().expect("should have context");
        assert_eq!(ctx["agent_id"], "agent-1");
    }

    #[test]
    fn test_context_map_io_error() {
        let err = Error::io_error("disk full");
        let ctx = err.context_map().expect("should have context");
        assert_eq!(ctx["error_type"], "io_error");
        assert_eq!(ctx["message"], "disk full");
    }

    #[test]
    fn test_context_map_database_error() {
        let err = Error::database("connection lost");
        let ctx = err.context_map().expect("should have context");
        assert_eq!(ctx["error_type"], "io_error");
        assert_eq!(ctx["message"], "connection lost");
    }

    #[test]
    fn test_context_map_internal_error() {
        let err = Error::internal("invariant violated");
        let ctx = err.context_map().expect("should have context");
        assert_eq!(ctx["error_type"], "internal_error");
        assert_eq!(ctx["message"], "invariant violated");
    }

    #[test]
    fn test_context_map_lock_session_locked() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: Error = LockErrorKind::SessionLocked {
            session: "s".into(),
            holder: "h".into(),
        }
        .into();
        let ctx = err.context_map().expect("should have context");
        assert_eq!(ctx["session"], "s");
        assert_eq!(ctx["holder"], "h");
    }

    #[test]
    fn test_context_map_wait_batch_size_exceeded() {
        let err = Error::batch_size_exceeded(50);
        let ctx = err.context_map().expect("should have context");
        assert_eq!(ctx["max_size"], 50);
    }
}
