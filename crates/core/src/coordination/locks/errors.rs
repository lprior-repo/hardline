//! Lock manager error types.
//!
//! Error codes: 9xxx

use thiserror::Error;

/// Lock manager errors.
///
/// All fallible functions return `Result<T, Error>`.
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct LockError(#[from] LockErrorKind);

#[derive(Error, Debug, Clone)]
pub enum LockErrorKind {
    /// Session does not exist in sessions table
    /// Used when lock/heartbeat/unlock is attempted on non-existent session
    #[error("Session not found: {session}")]
    SessionNotFound { session: String },

    /// Session is already locked by another agent
    /// Contains the holder's agent_id for client-side retry logic
    #[error("Session '{session}' is locked by '{holder}'")]
    SessionLocked { session: String, holder: String },

    /// Agent attempted operation on lock held by different agent
    /// Used for unlock() and heartbeat() when agent_id != holder
    #[error("Agent '{agent_id}' does not hold lock on session '{session}'")]
    NotLockHolder { session: String, agent_id: String },

    /// No active lock exists for session
    /// Used for heartbeat() when lock is missing or expired
    #[error("{0}")]
    NotFound(String),

    /// Database operation failed (connection, query, transaction)
    /// Wraps sqlx::Error with context
    #[error("{0}")]
    DatabaseError(String),

    /// Failed to parse timestamp or other format
    /// Used for RFC3339 timestamp parsing failures
    #[error("{0}")]
    ParseError(String),

    /// Unknown/unexpected error with context
    /// Catch-all for errors not covered by specific variants
    #[error("{0}")]
    Unknown(String),

    /// TTL value outside valid range [0, 86400]
    /// Used when ttl_seconds parameter is < 0 or > 86400 (24 hours)
    #[error("{0}")]
    TtlOutOfRange(String),

    /// Session name is empty string
    /// Used when session parameter is ""
    #[error("{0}")]
    EmptySessionName(String),

    /// Agent ID is empty string
    /// Used when agent_id parameter is ""
    #[error("{0}")]
    EmptyAgentId(String),

    /// TTL value would overflow u64::MAX
    /// Used when ttl_seconds = u64::MAX or would overflow on arithmetic
    #[error("{0}")]
    TtlOverflow(String),

    /// Session name exceeds 255 character limit
    /// Used when session.len() > 255 (SQLite TEXT limit)
    #[error("{0}")]
    SessionNameTooLong(String),
}

impl From<LockErrorKind> for crate::error::Error {
    fn from(e: LockErrorKind) -> Self {
        crate::error::Error::Lock(LockError::new(e))
    }
}

impl LockError {
    /// Returns the error code for telemetry.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match &self.inner {
            LockErrorKind::SessionNotFound { .. } => "SESSION_NOT_FOUND",
            LockErrorKind::SessionLocked { .. } => "SESSION_LOCKED",
            LockErrorKind::NotLockHolder { .. } => "NOT_LOCK_HOLDER",
            LockErrorKind::NotFound(_) => "NOT_FOUND",
            LockErrorKind::DatabaseError(_) => "DATABASE_ERROR",
            LockErrorKind::ParseError(_) => "PARSE_ERROR",
            LockErrorKind::Unknown(_) => "UNKNOWN",
            LockErrorKind::TtlOutOfRange(_) => "TTL_OUT_OF_RANGE",
            LockErrorKind::EmptySessionName(_) => "EMPTY_SESSION_NAME",
            LockErrorKind::EmptyAgentId(_) => "EMPTY_AGENT_ID",
            LockErrorKind::TtlOverflow(_) => "TTL_OVERFLOW",
            LockErrorKind::SessionNameTooLong(_) => "SESSION_NAME_TOO_LONG",
        }
    }

    /// Returns a human-readable suggestion for fixing the error.
    #[must_use]
    pub fn suggestion(&self) -> Option<String> {
        match &self.inner {
            LockErrorKind::SessionLocked { holder, .. } => {
                Some(format!("Use 'scp agent kill {holder}' to force release"))
            }
            LockErrorKind::SessionNotFound { .. } => {
                Some("Try 'scp session list' to see available sessions".to_string())
            }
            _ => None,
        }
    }

    /// Returns exit code for CLI.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match &self.inner {
            LockErrorKind::SessionNotFound { .. } => 14,
            LockErrorKind::SessionLocked { .. } => 16,
            LockErrorKind::NotLockHolder { .. } => 17,
            LockErrorKind::NotFound(_) => 71,
            LockErrorKind::DatabaseError(_) => 63,
            LockErrorKind::ParseError(_) => 80,
            LockErrorKind::Unknown(_) => 90,
            LockErrorKind::TtlOutOfRange(_) => 80,
            LockErrorKind::EmptySessionName(_) => 80,
            LockErrorKind::EmptyAgentId(_) => 80,
            LockErrorKind::TtlOverflow(_) => 80,
            LockErrorKind::SessionNameTooLong(_) => 80,
        }
    }
}
