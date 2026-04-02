//! Agent-related errors.
//!
//! Error codes: 5xxx

use crate::error::Error;
use thiserror::Error;

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
        assert_eq!(AgentError::from(AgentErrorKind::NotFound("x".into())).exit_code(), 50);
        assert_eq!(AgentError::from(AgentErrorKind::Exists("x".into())).exit_code(), 51);
        assert_eq!(AgentError::from(AgentErrorKind::Timeout("x".into())).exit_code(), 52);
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
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            AgentErrorKind::NotFound(_) => 50,
            AgentErrorKind::Exists(_) => 51,
            AgentErrorKind::Timeout(_) => 52,
        }
    }
}
