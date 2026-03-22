//! Identifier error conversion implementations.
//!
//! This module provides `From<IdentifierError>` implementations
//! for aggregate errors and validation errors.

use crate::domain::{
    aggregates::{bead::BeadError, session::SessionError, workspace::WorkspaceError},
    identifiers::IdentifierError,
    validation::ValidationError,
};

impl From<IdentifierError> for SessionError {
    fn from(_err: IdentifierError) -> Self {
        Self::CannotActivate
    }
}

impl From<IdentifierError> for WorkspaceError {
    fn from(_err: IdentifierError) -> Self {
        Self::CannotUse(crate::domain::workspace::WorkspaceState::Creating)
    }
}

impl From<IdentifierError> for BeadError {
    fn from(err: IdentifierError) -> Self {
        match err {
            IdentifierError::Empty => Self::TitleRequired,
            _ => Self::InvalidTitle(err.to_string()),
        }
    }
}

impl From<IdentifierError> for ValidationError {
    fn from(err: IdentifierError) -> Self {
        match err {
            IdentifierError::Empty => Self::EmptyValue("identifier".to_string()),
            IdentifierError::TooLong { max, actual } => Self::ExceedsMaximum {
                field: "value".to_string(),
                value: actual as u32,
                max: max as u32,
            },
            IdentifierError::InvalidCharacters { details } => Self::InvalidCharacters {
                field: "value".to_string(),
                found: details,
            },
            IdentifierError::InvalidFormat { details } => Self::InvalidCharacters {
                field: "value".to_string(),
                found: details,
            },
            IdentifierError::InvalidStart { .. } => Self::InvalidCharacters {
                field: "value".to_string(),
                found: "invalid start character".to_string(),
            },
            IdentifierError::InvalidPrefix { .. } => Self::InvalidCharacters {
                field: "value".to_string(),
                found: "invalid prefix".to_string(),
            },
            IdentifierError::InvalidHex { .. } => Self::InvalidCharacters {
                field: "value".to_string(),
                found: "invalid hex".to_string(),
            },
            IdentifierError::NotAbsolutePath { .. } => Self::InvalidCharacters {
                field: "path".to_string(),
                found: "not absolute".to_string(),
            },
            IdentifierError::NullBytesInPath => Self::InvalidCharacters {
                field: "path".to_string(),
                found: "null byte".to_string(),
            },
            IdentifierError::NotAscii { .. } => Self::InvalidCharacters {
                field: "value".to_string(),
                found: "non-ASCII".to_string(),
            },
            IdentifierError::ContainsPathSeparators => Self::InvalidCharacters {
                field: "value".to_string(),
                found: "path separator".to_string(),
            },
        }
    }
}
