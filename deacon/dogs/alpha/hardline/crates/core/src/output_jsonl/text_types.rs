//! Text newtypes - validate once, use everywhere
//!
//! # Design Principles
//!
//! Following Scott Wlaschin's DDD principles:
//! - **Parse at boundaries, validate once** - Validate text at construction
//! - **Make illegal states unrepresentable** - Newtypes prevent invalid values

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};

use super::OutputLineError;

/// A validated issue title
///
/// # Invariants
///
/// - Must be non-empty after trimming
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueTitle(String);

impl IssueTitle {
    /// Create a new issue title, validating it's non-empty
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyTitle` if the title is empty
    pub fn new(title: impl Into<String>) -> Result<Self, OutputLineError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(OutputLineError::EmptyTitle);
        }
        Ok(Self(title))
    }

    /// Get the issue title as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IssueTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for IssueTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated plan title
///
/// # Invariants
///
/// - Must be non-empty after trimming
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTitle(String);

impl PlanTitle {
    /// Create a new plan title, validating it's non-empty
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyTitle` if the title is empty
    pub fn new(title: impl Into<String>) -> Result<Self, OutputLineError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(OutputLineError::EmptyTitle);
        }
        Ok(Self(title))
    }

    /// Get the plan title as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlanTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for PlanTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated plan description
///
/// # Invariants
///
/// - Must be non-empty after trimming
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanDescription(String);

impl PlanDescription {
    /// Create a new plan description, validating it's non-empty
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyDescription` if the description is empty
    pub fn new(desc: impl Into<String>) -> Result<Self, OutputLineError> {
        let desc = desc.into();
        if desc.trim().is_empty() {
            return Err(OutputLineError::EmptyDescription);
        }
        Ok(Self(desc))
    }

    /// Get the plan description as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlanDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for PlanDescription {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated message content
///
/// # Invariants
///
/// - Must be non-empty after trimming
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message(String);

impl Message {
    /// Create a new message, validating it's non-empty
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if the message is empty
    pub fn new(msg: impl Into<String>) -> Result<Self, OutputLineError> {
        let msg = msg.into();
        if msg.trim().is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }
        Ok(Self(msg))
    }

    /// Get the message as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Message {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
