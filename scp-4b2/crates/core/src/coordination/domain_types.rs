//! Domain primitive types (semantic newtypes).
//!
//! This module provides type-safe identifiers and domain primitives
//! to prevent primitive obsession and make illegal states unrepresentable.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use thiserror::Error;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DOMAIN ERRORS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Error type for domain primitive validation.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DomainError {
    /// Empty string provided where non-empty was required.
    #[error("empty string not allowed for {field}")]
    Empty { field: &'static str },

    /// Invalid format for domain primitive.
    #[error("invalid format for {field}: {value}")]
    InvalidFormat { field: &'static str, value: String },

    /// ID cannot be parsed.
    #[error("cannot parse {field} from '{value}'")]
    ParseError { field: &'static str, value: String },
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// WORKSPACE NAME - RE-EXPORTED FROM DOMAIN
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// AGENT ID (Re-export from domain layer)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BEAD ID (Re-export from domain layer)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pub use crate::domain::identifiers::{AgentId, BeadId, WorkspaceName};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_name_valid() {
        match WorkspaceName::parse("test-workspace") {
            Ok(name) => {
                assert_eq!(name.as_str(), "test-workspace");
            }
            Err(e) => panic!("Failed to create WorkspaceName: {e}"),
        }
    }

    #[test]
    fn test_workspace_name_empty_is_invalid() {
        let name = WorkspaceName::parse("");
        assert!(matches!(
            name,
            Err(crate::domain::identifiers::IdError::Empty)
        ));
    }

    #[test]
    fn test_workspace_name_from_str() {
        match WorkspaceName::parse("my-workspace") {
            Ok(name) => {
                assert_eq!(name.as_str(), "my-workspace");
            }
            Err(e) => panic!("Failed to parse WorkspaceName: {e}"),
        }
    }
}
