//! Agent-related errors.
//!
//! Error codes: 5xxx

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

// ========================================================================
// Exit Code
// ========================================================================

impl AgentError {
    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            AgentErrorKind::NotFound(_) => 50,
            AgentErrorKind::Exists(_) => 51,
            AgentErrorKind::Timeout(_) => 52,
        }
    }
}
