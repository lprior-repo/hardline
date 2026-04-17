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

    /// Session name contains invalid characters (control characters, newlines, etc.)
    /// Used when session name fails sanitization checks
    #[error("{0}")]
    InvalidSessionName(String),
}

impl From<LockErrorKind> for crate::error::Error {
    fn from(e: LockErrorKind) -> Self {
        crate::error::Error::Lock(LockError(e))
    }
}

impl LockError {
    /// Returns a reference to the underlying error kind.
    #[must_use]
    pub const fn kind(&self) -> &LockErrorKind {
        &self.0
    }

    /// Returns the error code for telemetry.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match &self.0 {
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
            LockErrorKind::InvalidSessionName(_) => "INVALID_SESSION_NAME",
        }
    }

    /// Returns a human-readable suggestion for fixing the error.
    #[must_use]
    pub fn suggestion(&self) -> Option<String> {
        match &self.0 {
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
        match &self.0 {
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
            LockErrorKind::InvalidSessionName(_) => 80,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_not_found_display() {
        let err = LockErrorKind::SessionNotFound {
            session: "sess-1".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("sess-1"));
        assert!(msg.contains("Session not found"));
    }

    #[test]
    fn session_locked_display() {
        let err = LockErrorKind::SessionLocked {
            session: "sess-1".to_string(),
            holder: "agent-1".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("sess-1"));
        assert!(msg.contains("agent-1"));
        assert!(msg.contains("locked"));
    }

    #[test]
    fn not_lock_holder_display() {
        let err = LockErrorKind::NotLockHolder {
            session: "sess-1".to_string(),
            agent_id: "agent-2".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("agent-2"));
        assert!(msg.contains("does not hold lock"));
    }

    #[test]
    fn not_found_display() {
        let err = LockErrorKind::NotFound("no active lock".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("no active lock"));
    }

    #[test]
    fn database_error_display() {
        let err = LockErrorKind::DatabaseError("connection refused".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn parse_error_display() {
        let err = LockErrorKind::ParseError("invalid timestamp".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("invalid timestamp"));
    }

    #[test]
    fn unknown_display() {
        let err = LockErrorKind::Unknown("unexpected error".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("unexpected error"));
    }

    #[test]
    fn ttl_out_of_range_display() {
        let err = LockErrorKind::TtlOutOfRange("TTL must be 0-86400".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("TTL must be 0-86400"));
    }

    #[test]
    fn empty_session_name_display() {
        let err = LockErrorKind::EmptySessionName("session name cannot be empty".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("session name cannot be empty"));
    }

    #[test]
    fn empty_agent_id_display() {
        let err = LockErrorKind::EmptyAgentId("agent ID cannot be empty".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("agent ID cannot be empty"));
    }

    #[test]
    fn ttl_overflow_display() {
        let err = LockErrorKind::TtlOverflow("TTL value overflow".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("TTL value overflow"));
    }

    #[test]
    fn session_name_too_long_display() {
        let err = LockErrorKind::SessionNameTooLong("name exceeds 255 chars".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("name exceeds 255 chars"));
    }

    #[test]
    fn invalid_session_name_display() {
        let err = LockErrorKind::InvalidSessionName("contains null bytes".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("contains null bytes"));
    }

    #[test]
    fn lock_error_kind_accessor() {
        let err = LockError(LockErrorKind::SessionNotFound {
            session: "s".to_string(),
        });
        assert!(matches!(err.kind(), LockErrorKind::SessionNotFound { .. }));
    }

    #[test]
    fn lock_error_code_all_variants() {
        assert_eq!(
            LockError(LockErrorKind::SessionNotFound {
                session: "s".into()
            })
            .code(),
            "SESSION_NOT_FOUND"
        );
        assert_eq!(
            LockError(LockErrorKind::SessionLocked {
                session: "s".into(),
                holder: "h".into()
            })
            .code(),
            "SESSION_LOCKED"
        );
        assert_eq!(
            LockError(LockErrorKind::NotLockHolder {
                session: "s".into(),
                agent_id: "a".into()
            })
            .code(),
            "NOT_LOCK_HOLDER"
        );
        assert_eq!(
            LockError(LockErrorKind::NotFound("x".into())).code(),
            "NOT_FOUND"
        );
        assert_eq!(
            LockError(LockErrorKind::DatabaseError("x".into())).code(),
            "DATABASE_ERROR"
        );
        assert_eq!(
            LockError(LockErrorKind::ParseError("x".into())).code(),
            "PARSE_ERROR"
        );
        assert_eq!(
            LockError(LockErrorKind::Unknown("x".into())).code(),
            "UNKNOWN"
        );
        assert_eq!(
            LockError(LockErrorKind::TtlOutOfRange("x".into())).code(),
            "TTL_OUT_OF_RANGE"
        );
        assert_eq!(
            LockError(LockErrorKind::EmptySessionName("x".into())).code(),
            "EMPTY_SESSION_NAME"
        );
        assert_eq!(
            LockError(LockErrorKind::EmptyAgentId("x".into())).code(),
            "EMPTY_AGENT_ID"
        );
        assert_eq!(
            LockError(LockErrorKind::TtlOverflow("x".into())).code(),
            "TTL_OVERFLOW"
        );
        assert_eq!(
            LockError(LockErrorKind::SessionNameTooLong("x".into())).code(),
            "SESSION_NAME_TOO_LONG"
        );
        assert_eq!(
            LockError(LockErrorKind::InvalidSessionName("x".into())).code(),
            "INVALID_SESSION_NAME"
        );
    }

    #[test]
    fn lock_error_suggestion_session_locked() {
        let err = LockError(LockErrorKind::SessionLocked {
            session: "sess".into(),
            holder: "agent-1".into(),
        });
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("agent kill agent-1"));
    }

    #[test]
    fn lock_error_suggestion_session_not_found() {
        let err = LockError(LockErrorKind::SessionNotFound {
            session: "sess".into(),
        });
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("session list"));
    }

    #[test]
    fn lock_error_suggestion_none_for_database_error() {
        let err = LockError(LockErrorKind::DatabaseError("fail".into()));
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn lock_error_exit_codes_all_variants() {
        assert_eq!(
            LockError(LockErrorKind::SessionNotFound {
                session: "s".into()
            })
            .exit_code(),
            14
        );
        assert_eq!(
            LockError(LockErrorKind::SessionLocked {
                session: "s".into(),
                holder: "h".into()
            })
            .exit_code(),
            16
        );
        assert_eq!(
            LockError(LockErrorKind::NotLockHolder {
                session: "s".into(),
                agent_id: "a".into()
            })
            .exit_code(),
            17
        );
        assert_eq!(
            LockError(LockErrorKind::NotFound("x".into())).exit_code(),
            71
        );
        assert_eq!(
            LockError(LockErrorKind::DatabaseError("x".into())).exit_code(),
            63
        );
        assert_eq!(
            LockError(LockErrorKind::ParseError("x".into())).exit_code(),
            80
        );
        assert_eq!(
            LockError(LockErrorKind::Unknown("x".into())).exit_code(),
            90
        );
        assert_eq!(
            LockError(LockErrorKind::TtlOutOfRange("x".into())).exit_code(),
            80
        );
        assert_eq!(
            LockError(LockErrorKind::EmptySessionName("x".into())).exit_code(),
            80
        );
        assert_eq!(
            LockError(LockErrorKind::EmptyAgentId("x".into())).exit_code(),
            80
        );
        assert_eq!(
            LockError(LockErrorKind::TtlOverflow("x".into())).exit_code(),
            80
        );
        assert_eq!(
            LockError(LockErrorKind::SessionNameTooLong("x".into())).exit_code(),
            80
        );
        assert_eq!(
            LockError(LockErrorKind::InvalidSessionName("x".into())).exit_code(),
            80
        );
    }

    #[test]
    fn from_lock_error_kind_to_error() {
        let err: crate::error::Error = LockErrorKind::SessionNotFound {
            session: "s".into(),
        }
        .into();
        assert!(matches!(err, crate::error::Error::Lock(_)));
    }
}
