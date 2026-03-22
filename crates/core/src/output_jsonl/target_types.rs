//! Target reference types
//!
//! # Design Principles
//!
//! Following Scott Wlaschin's DDD principles:
//! - **Parse at boundaries, validate once** - Validate targets at construction
//! - **Make illegal states unrepresentable** - Newtypes prevent invalid values

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};

use super::OutputLineError;

/// A validated action target
///
/// # Invariants
///
/// - Must be non-empty after trimming
/// - Maximum length of 1000 characters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionTarget(String);

impl ActionTarget {
    /// Maximum length for action target
    pub const MAX_LENGTH: usize = 1000;

    /// Create a new action target, validating format
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if target is empty
    /// Returns `OutputLineError::InvalidActionTarget` if target exceeds max length
    pub fn new(target: impl Into<String>) -> Result<Self, OutputLineError> {
        let target = target.into();

        let trimmed = target.trim();
        if trimmed.is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }

        if trimmed.len() > Self::MAX_LENGTH {
            return Err(OutputLineError::InvalidActionTarget(format!(
                "action target exceeds maximum length of {} characters",
                Self::MAX_LENGTH
            )));
        }

        Ok(Self(trimmed.to_string()))
    }

    /// Get the action target as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ActionTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ActionTarget {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated base reference (git branch name)
///
/// Base refs are less constrained - they can be any string
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseRef(String);

impl BaseRef {
    /// Create a new base reference (no validation required)
    #[must_use]
    pub fn new(base_ref: impl Into<String>) -> Self {
        Self(base_ref.into())
    }

    /// Get the base reference as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BaseRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for BaseRef {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated command string
///
/// Commands are less constrained - they can be any string (including empty for manual steps)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command(String);

impl Command {
    /// Create a new command (no validation required)
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self(command.into())
    }

    /// Get the command as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if the command is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Command {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
