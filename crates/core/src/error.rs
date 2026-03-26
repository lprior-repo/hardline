//! Unified error types for Source Control Plane.
//!
//! All errors return Result<T, Error> - zero panic, zero unwrap.

use thiserror::Error;

// Re-export types from sub-modules
pub use super::error_types::JjConflictType;
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
    // JJ-specific Errors (3xxx)
    // ========================================================================
    /// JJ-specific errors
    #[error(transparent)]
    Jj(#[from] super::error_jj::JjError),

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

    /// Creates a JJ command error
    #[inline]
    pub fn jj_command_error(
        operation: impl Into<String>,
        msg: impl Into<String>,
        is_not_found: bool,
    ) -> Self {
        use super::error_jj::JjErrorKind;
        JjErrorKind::CommandError {
            operation: operation.into(),
            msg: msg.into(),
            is_not_found,
        }
        .into()
    }

    /// Creates a JJ workspace conflict error
    #[inline]
    pub fn jj_workspace_conflict(
        conflict_type: super::error_types::JjConflictType,
        workspace_name: impl Into<String>,
        msg: impl Into<String>,
        recovery_hint: impl Into<String>,
    ) -> Self {
        use super::error_jj::JjErrorKind;
        JjErrorKind::WorkspaceConflict {
            conflict_type,
            workspace_name: workspace_name.into(),
            msg: msg.into(),
            recovery_hint: recovery_hint.into(),
        }
        .into()
    }

    /// Creates a JJ lock timeout error
    #[inline]
    pub fn jj_lock_timeout(operation: impl Into<String>, timeout_ms: u64, retries: usize) -> Self {
        use super::error_jj::JjErrorKind;
        JjErrorKind::LockTimeout {
            operation: operation.into(),
            timeout_ms,
            retries,
        }
        .into()
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
            Error::State(_) => None,
            Error::Internal(_) => None,
            Error::Jj(_) => None,
            Error::Task(_) => None,
            Error::Wait(_) => None,
            Error::Lock(_) => None,
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
            Error::Jj(e) => e.exit_code(),
            Error::Task(e) => e.exit_code(),
            Error::Wait(e) => e.exit_code(),
            Error::Lock(_) => 90,
        }
    }
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_queue::{QueueError, QueueErrorKind};
    use crate::error_vcs::{VcsError, VcsErrorKind};
    use crate::error_workspace::{WorkspaceError, WorkspaceErrorKind};

    #[test]
    fn test_error_suggestions() {
        let err = Error::Workspace(WorkspaceError {
            inner: WorkspaceErrorKind::NotFound("test".to_string()),
        });
        assert!(err.suggestion().is_some());

        let err = Error::Queue(QueueError {
            inner: QueueErrorKind::Empty,
        });
        assert!(err.suggestion().is_some());
    }

    #[test]
    fn test_exit_codes() {
        let err = Error::Workspace(WorkspaceError {
            inner: WorkspaceErrorKind::NotFound("x".to_string()),
        });
        assert_eq!(err.exit_code(), 10);

        let err = Error::Queue(QueueError {
            inner: QueueErrorKind::Empty,
        });
        assert_eq!(err.exit_code(), 20);

        let err = Error::Vcs(VcsError {
            inner: VcsErrorKind::NotInitialized,
        });
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
}
