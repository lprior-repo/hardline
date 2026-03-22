//! Configuration-related errors.
//!
//! Error codes: 4xxx

use thiserror::Error;
use crate::error::Error;

/// Configuration-related errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct ConfigError {
    #[from]
    inner: ConfigErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum ConfigErrorKind {
    /// Configuration not found
    #[error("Configuration not found: {0}")]
    NotFound(String),

    /// Configuration is invalid
    #[error("Configuration invalid: {0}")]
    Invalid(String),

    /// Configuration permission denied
    #[error("Configuration permission denied: {0}")]
    Permission(String),
}

impl From<ConfigErrorKind> for Error {
    fn from(e: ConfigErrorKind) -> Self {
        Error::Config(e.into())
    }
}

// ========================================================================
// Exit Code
// ========================================================================

impl ConfigError {
    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            ConfigErrorKind::NotFound(_) => 40,
            ConfigErrorKind::Invalid(_) => 41,
            ConfigErrorKind::Permission(_) => 42,
        }
    }
}
