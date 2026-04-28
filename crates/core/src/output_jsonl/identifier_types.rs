//! Identifier newtypes - parse at boundaries, validate once
//!
//! # Design Principles
//!
//! Following Scott Wlaschin's DDD principles:
//! - **Parse at boundaries, validate once** - Validate identifiers at construction
//! - **Make illegal states unrepresentable** - Newtypes prevent invalid values

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};

use super::OutputLineError;
// Re-export from domain (single source of truth)
pub use crate::domain::{BeadId, SessionName};

/// A validated issue identifier
///
/// # Invariants
///
/// - Must be non-empty after trimming
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IssueId(String);

impl IssueId {
    /// Create a new issue ID, validating it's non-empty
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if the ID is empty
    pub fn new(id: impl Into<String>) -> Result<Self, OutputLineError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }
        Ok(Self(id))
    }

    /// Get the issue ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IssueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for IssueId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
