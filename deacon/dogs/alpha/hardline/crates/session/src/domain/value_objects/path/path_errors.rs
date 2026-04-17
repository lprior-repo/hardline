//! Error types for AbsolutePath validation.

use thiserror::Error;

/// Errors that can occur when validating a path.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PathValidationError {
    /// Path is relative (does not start with / or is empty)
    #[error("Path is not absolute: {input}")]
    NotAbsolute { input: String },
    /// Path contains invalid UTF-8 bytes
    #[error("Path contains invalid UTF-8: {invalid_bytes:?}")]
    InvalidUtf8 { invalid_bytes: Vec<u8> },
}

/// Errors from shell metacharacter detection.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ShellMetacharacterError {
    #[error("Path contains $ at position {position}")]
    ContainsDollar { position: usize },
    #[error("Path contains backtick at position {position}")]
    ContainsBacktick { position: usize },
    #[error("Path contains ; at position {position}")]
    ContainsSemicolon { position: usize },
    #[error("Path contains | at position {position}")]
    ContainsPipe { position: usize },
    #[error("Path contains & at position {position}")]
    ContainsAmpersand { position: usize },
}

/// Errors that can occur when constructing an AbsolutePath.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AbsolutePathError {
    #[error(transparent)]
    PathValidation(#[from] PathValidationError),
    #[error(transparent)]
    ShellMetacharacter(#[from] ShellMetacharacterError),
}
