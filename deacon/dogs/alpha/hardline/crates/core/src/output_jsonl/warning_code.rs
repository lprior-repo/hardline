//! Warning code enumeration
//!
//! # Design Principles
//!
//! Following Scott Wlaschin's DDD principles:
//! - **Make illegal states unrepresentable** - Enum prevents arbitrary string codes
//! - **Parse at boundaries** - Validate warning codes at construction

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};

use super::OutputLineError;

/// Known warning codes in the system
///
/// These are predefined warning codes that have specific meanings.
/// Custom codes can be added via the `Custom` variant for extensibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarningCode {
    /// Configuration file not found, using defaults
    ConfigNotFound,
    /// Invalid configuration value
    ConfigInvalid,
    /// Session limit reached
    SessionLimitReached,
    /// Workspace path not found
    WorkspaceNotFound,
    /// Git operation failed
    GitOperationFailed,
    /// Merge conflict detected
    MergeConflict,
    /// Agent not available
    AgentUnavailable,
    /// Custom warning code with string value
    #[serde(untagged)]
    Custom(String),
}

impl WarningCode {
    /// Create a warning code from known codes or validate custom format
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::InvalidWarningCode` if custom code doesn't
    /// follow the pattern: letter followed by alphanumeric (e.g., "W001", "E123")
    pub fn new(code: impl Into<String>) -> Result<Self, OutputLineError> {
        let code = code.into();

        // Match against known codes
        match code.as_str() {
            "CONFIG_NOT_FOUND" => Ok(Self::ConfigNotFound),
            "CONFIG_INVALID" => Ok(Self::ConfigInvalid),
            "SESSION_LIMIT_REACHED" => Ok(Self::SessionLimitReached),
            "WORKSPACE_NOT_FOUND" => Ok(Self::WorkspaceNotFound),
            "GIT_OPERATION_FAILED" => Ok(Self::GitOperationFailed),
            "MERGE_CONFLICT" => Ok(Self::MergeConflict),
            "AGENT_UNAVAILABLE" => Ok(Self::AgentUnavailable),
            custom => {
                // Validate custom code format: letter followed by alphanumeric
                if custom.is_empty() {
                    return Err(OutputLineError::InvalidWarningCode(
                        "warning code cannot be empty".to_string(),
                    ));
                }

                // Must start with a letter
                if !custom
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                {
                    return Err(OutputLineError::InvalidWarningCode(format!(
                        "warning code must start with a letter, got: {custom}"
                    )));
                }

                // All characters must be alphanumeric or underscore
                if !custom
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    return Err(OutputLineError::InvalidWarningCode(format!(
                        "warning code must be alphanumeric or underscore, got: {custom}"
                    )));
                }

                Ok(Self::Custom(custom.to_string()))
            }
        }
    }

    /// Get the warning code as a string slice
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::ConfigNotFound => "CONFIG_NOT_FOUND",
            Self::ConfigInvalid => "CONFIG_INVALID",
            Self::SessionLimitReached => "SESSION_LIMIT_REACHED",
            Self::WorkspaceNotFound => "WORKSPACE_NOT_FOUND",
            Self::GitOperationFailed => "GIT_OPERATION_FAILED",
            Self::MergeConflict => "MERGE_CONFLICT",
            Self::AgentUnavailable => "AGENT_UNAVAILABLE",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Check if this is a custom warning code
    #[must_use]
    pub const fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }
}

impl fmt::Display for WarningCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AsRef<str> for WarningCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
