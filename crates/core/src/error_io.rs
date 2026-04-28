//! IO-related errors.
//!
//! Error codes: 6xxx

use thiserror::Error;

use crate::error::Error;

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
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub fn kind(&self) -> &IoErrorKind {
        &self.inner
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_kind_io_error_display() {
        let err = IoErrorKind::IoError("file not found".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("file not found"));
        assert!(msg.contains("IO error"));
    }

    #[test]
    fn io_error_kind_database_display() {
        let err = IoErrorKind::Database("connection refused".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("connection refused"));
        assert!(msg.contains("Database error"));
    }

    #[test]
    fn io_error_exit_codes() {
        assert_eq!(
            IoError::from(IoErrorKind::IoError("x".into())).exit_code(),
            64
        );
        assert_eq!(
            IoError::from(IoErrorKind::Database("x".into())).exit_code(),
            63
        );
        assert_eq!(
            IoError::from(IoErrorKind::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "missing",
            )))
            .exit_code(),
            60
        );
        assert_eq!(
            IoError::from(IoErrorKind::JsonParse(
                serde_json::from_str::<serde_json::Value>("bad").expect_err("parse err"),
            ))
            .exit_code(),
            61
        );
        assert_eq!(
            IoError::from(IoErrorKind::YamlParse(
                serde_yaml::from_str::<serde_yaml::Value>(": bad").expect_err("parse err"),
            ))
            .exit_code(),
            62
        );
    }

    #[test]
    fn io_error_kind_io_display() {
        let err = IoErrorKind::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        let msg = format!("{err}");
        assert!(msg.contains("denied"));
        assert!(msg.contains("IO error"));
    }

    #[test]
    fn io_error_kind_accessor() {
        let err = IoError::from(IoErrorKind::IoError("test".to_string()));
        assert!(matches!(err.kind(), IoErrorKind::IoError(_)));
    }

    #[test]
    fn from_io_error_kind_to_error() {
        let err: Error = IoErrorKind::IoError("test".to_string()).into();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn io_error_json_parse_display() {
        let err = IoErrorKind::JsonParse(
            serde_json::from_str::<serde_json::Value>("invalid").expect_err("parse err"),
        );
        let msg = format!("{err}");
        assert!(msg.contains("JSON parse error"));
    }

    #[test]
    fn io_error_yaml_parse_display() {
        let err = IoErrorKind::YamlParse(
            serde_yaml::from_str::<serde_yaml::Value>(": invalid").expect_err("parse err"),
        );
        let msg = format!("{err}");
        assert!(msg.contains("YAML parse error"));
    }
}
