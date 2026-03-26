//! IO-related errors.
//!
//! Error codes: 6xxx

use crate::error::Error;
use thiserror::Error;

/// IO-related errors
#[derive(Error, Debug)]
#[error(transparent)]
pub struct IoError {
    #[from]
    inner: IoErrorKind,
}

#[derive(Error, Debug)]
pub enum IoErrorKind {
    /// IO error with context
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// IO error with custom message
    #[error("IO error: {0}")]
    IoError(String),

    /// JSON parse error
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// YAML parse error
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),
}

impl From<IoErrorKind> for Error {
    fn from(e: IoErrorKind) -> Self {
        Error::Io(e.into())
    }
}

// ========================================================================
// Exit Code
// ========================================================================

impl IoError {
    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            IoErrorKind::Io(_) => 60,
            IoErrorKind::IoError(_) => 64,
            IoErrorKind::JsonParse(_) => 61,
            IoErrorKind::YamlParse(_) => 62,
            IoErrorKind::Database(_) => 63,
        }
    }
}
