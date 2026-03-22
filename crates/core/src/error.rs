//! Unified error types for Source Control Plane.
//!
//! All errors return Result<T, Error> - zero panic, zero unwrap.

use thiserror::Error;

// Re-export types from sub-modules
pub use super::error_types::JjConflictType;
pub use super::error_types::Result;

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
        }
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
        }
    }
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_workspace::{WorkspaceError, WorkspaceErrorKind};
    use crate::error_queue::{QueueError, QueueErrorKind};
    use crate::error_vcs::{VcsError, VcsErrorKind};

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
