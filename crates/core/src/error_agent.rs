//! Agent-related errors.
//!
//! Error codes: 9xxx infrastructure (ADR-007)

use thiserror::Error;

use crate::error::Error;

/// Agent-related errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct AgentError {
    #[from]
    inner: AgentErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum AgentErrorKind {
    /// Agent not found
    #[error("Agent not found: {0}")]
    NotFound(String),

    /// Agent already registered
    #[error("Agent already registered: {0}")]
    Exists(String),

    /// Agent heartbeat timeout
    #[error("Agent '{0}' heartbeat timeout")]
    Timeout(String),
}

impl From<AgentErrorKind> for Error {
    fn from(e: AgentErrorKind) -> Self {
        Error::Agent(e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_error_kind_not_found_display() {
        let err = AgentErrorKind::NotFound("agent-42".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("agent-42"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn agent_error_kind_exists_display() {
        let err = AgentErrorKind::Exists("agent-42".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("agent-42"));
        assert!(msg.contains("already registered"));
    }

    #[test]
    fn agent_error_kind_timeout_display() {
        let err = AgentErrorKind::Timeout("agent-42".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("agent-42"));
        assert!(msg.contains("heartbeat timeout"));
    }

    #[test]
    fn agent_error_exit_codes() {
        assert_eq!(
            AgentError::from(AgentErrorKind::NotFound("x".into())).exit_code(),
            100
        );
        assert_eq!(
            AgentError::from(AgentErrorKind::Exists("x".into())).exit_code(),
            101
        );
        assert_eq!(
            AgentError::from(AgentErrorKind::Timeout("x".into())).exit_code(),
            102
        );
    }

    #[test]
    fn agent_error_kind_accessor() {
        let err = AgentError::from(AgentErrorKind::NotFound("agent-1".to_string()));
        assert!(matches!(err.kind(), AgentErrorKind::NotFound(_)));
    }

    #[test]
    fn from_agent_error_kind_to_error() {
        let err: Error = AgentErrorKind::Timeout("agent-1".to_string()).into();
        assert!(matches!(err, Error::Agent(_)));
    }
}

// ========================================================================
// Exit Code
// ========================================================================

impl AgentError {
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub fn kind(&self) -> &AgentErrorKind {
        &self.inner
    }

    /// Returns exit code for CLI.
    /// Agent errors use range 100-102 (ADR-007: 9xxx infrastructure).
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            AgentErrorKind::NotFound(_) => 100,
            AgentErrorKind::Exists(_) => 101,
            AgentErrorKind::Timeout(_) => 102,
        }
    }
}
