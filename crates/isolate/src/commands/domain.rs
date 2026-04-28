#![allow(
    clippy::manual_string_new,
    clippy::redundant_closure_for_method_calls,
    clippy::unnecessary_wraps,
    clippy::uninlined_format_args
)]
//! Domain types for CLI handlers
//!
//! This module provides semantic newtypes that make illegal states unrepresentable.
//! All parsing and validation happens at the boundary, converting raw strings into
//! validated domain types.
//!
//! # Single Source of Truth
//!
//! This module re-exports `BeadId` and identifier types from `isolate_core::domain::identifiers`
//! to maintain consistency across the codebase.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{
    fmt::{self, Display},
    str::FromStr,
};

// Re-export identifier types from isolate_core (single source of truth)
pub use isolate_core::domain::identifiers::{AgentId, BeadId, IdentifierError, SessionName};
use thiserror::Error;

// ═══════════════════════════════════════════════════════════════════════════
// DOMAIN ERRORS
// ═══════════════════════════════════════════════════════════════════════════

/// Domain errors for CLI handler validation
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("session name '{0}' is invalid: {1}")]
    InvalidSessionName(String, String),

    #[error("bead ID '{0}' is invalid: {1}")]
    InvalidBeadId(String, String),

    #[error("agent ID '{0}' is invalid: {1}")]
    InvalidAgentId(String, String),

    #[error("workspace name '{0}' is invalid: {1}")]
    InvalidWorkspace(String, String),

    #[error("queue entry ID {0} is invalid: must be positive")]
    InvalidQueueId(i64),

    #[error("priority {0} is invalid: must be between 0 and 10")]
    InvalidPriority(i32),

    #[error("required field '{0}' is missing")]
    RequiredFieldMissing(String),
}

impl DomainError {
    /// Create a validation error with context
    #[must_use]
    pub const fn invalid_session_name(name: String, reason: String) -> Self {
        Self::InvalidSessionName(name, reason)
    }

    /// Create a bead ID validation error
    #[must_use]
    pub const fn invalid_bead_id(id: String, reason: String) -> Self {
        Self::InvalidBeadId(id, reason)
    }

    /// Create an agent ID validation error
    #[must_use]
    pub const fn invalid_agent_id(id: String, reason: String) -> Self {
        Self::InvalidAgentId(id, reason)
    }

    /// Create a workspace validation error
    #[must_use]
    pub const fn invalid_workspace(name: String, reason: String) -> Self {
        Self::InvalidWorkspace(name, reason)
    }
}

impl From<IdentifierError> for DomainError {
    fn from(err: IdentifierError) -> Self {
        match err {
            IdentifierError::Empty => {
                DomainError::InvalidSessionName(String::new(), "cannot be empty".to_string())
            }
            IdentifierError::TooLong { max, actual } => DomainError::InvalidSessionName(
                format!("({actual} chars)"),
                format!("too long (max {max} characters)"),
            ),
            IdentifierError::InvalidCharacters { details } => {
                DomainError::InvalidSessionName(String::new(), details)
            }
            IdentifierError::InvalidFormat { details } => {
                DomainError::InvalidSessionName(String::new(), details)
            }
            IdentifierError::InvalidStart { expected: _ } => DomainError::InvalidSessionName(
                String::new(),
                "must start with a letter".to_string(),
            ),
            IdentifierError::InvalidPrefix { prefix, value } => {
                DomainError::InvalidBeadId(value, format!("must have prefix '{prefix}'"))
            }
            IdentifierError::InvalidHex { value } => {
                DomainError::InvalidBeadId(value, "invalid hexadecimal format".to_string())
            }
            IdentifierError::NotAbsolutePath { value } => {
                DomainError::InvalidWorkspace(value, "path must be absolute".to_string())
            }
            IdentifierError::NullBytesInPath => {
                DomainError::InvalidWorkspace(String::new(), "path contains null bytes".to_string())
            }
            IdentifierError::NotAscii { value } => {
                DomainError::InvalidSessionName(value, "must be ASCII only".to_string())
            }
            IdentifierError::ContainsPathSeparators => DomainError::InvalidWorkspace(
                String::new(),
                "cannot contain path separators".to_string(),
            ),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// QUEUE ID NEWTYPE
// ═══════════════════════════════════════════════════════════════════════════

/// A validated queue entry ID
///
/// Queue IDs must be positive integers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueueId(i64);

impl QueueId {
    /// Validate and create a new `QueueId`
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidQueueId` if the ID is not positive.
    pub const fn new(id: i64) -> Result<Self, DomainError> {
        if id <= 0 {
            return Err(DomainError::InvalidQueueId(id));
        }
        Ok(Self(id))
    }

    /// Get the underlying numeric value
    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }
}

impl FromStr for QueueId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = s
            .parse::<i64>()
            .map_err(|_| DomainError::InvalidQueueId(0))?;
        Self::new(id)
    }
}

impl Display for QueueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i64> for QueueId {
    type Error = DomainError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PRIORITY NEWTYPE
// ═══════════════════════════════════════════════════════════════════════════

/// A validated priority value
///
/// Priorities must be between 0 and 10 inclusive
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Priority(i32);

impl Priority {
    /// Validate and create a new `Priority`
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidPriority` if the priority is outside the range 0-10.
    pub const fn new(priority: i32) -> Result<Self, DomainError> {
        if priority < 0 || priority > 10 {
            return Err(DomainError::InvalidPriority(priority));
        }
        Ok(Self(priority))
    }

    /// Get the underlying numeric value
    #[must_use]
    pub const fn value(self) -> i32 {
        self.0
    }

    /// Default priority (5)
    #[must_use]
    pub const fn default() -> Self {
        Self(5)
    }
}

impl FromStr for Priority {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let priority = s
            .parse::<i32>()
            .map_err(|_| DomainError::InvalidPriority(-1))?;
        Self::new(priority)
    }
}

impl Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::default()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// QUEUE ACTION ENUM (makes illegal states unrepresentable)
// ═══════════════════════════════════════════════════════════════════════════

/// Queue command action - enum instead of bool flags
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueAction {
    /// List all queue entries
    List,

    /// Add a session to the queue
    Add {
        session: SessionName,
        bead: Option<BeadId>,
        priority: Priority,
        agent: Option<AgentId>,
    },

    /// Remove a session from the queue
    Remove { session: SessionName },

    /// Show status for a specific session
    Status { session: Option<SessionName> },

    /// Show queue statistics
    Stats,

    /// Get next entry to process
    Next,

    /// Process the queue
    Process,

    /// Retry a failed entry
    Retry { id: QueueId },

    /// Cancel a pending entry
    Cancel { id: QueueId },

    /// Reclaim stale entries
    ReclaimStale { id: QueueId },

    /// Show status for a specific entry ID
    StatusId { id: QueueId },
}

impl QueueAction {
    /// Check if this is a list operation
    #[must_use]
    pub const fn is_list(&self) -> bool {
        matches!(self, Self::List)
    }

    /// Check if this is a status operation
    #[must_use]
    pub const fn is_status(&self) -> bool {
        matches!(self, Self::Status { .. } | Self::StatusId { .. })
    }

    /// Check if this is a process operation
    #[must_use]
    pub const fn is_process(&self) -> bool {
        matches!(self, Self::Process | Self::Next)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // SessionName tests (from isolate_core::domain::identifiers)
    #[test]
    fn test_session_name_valid() {
        assert!(SessionName::parse("valid-session").is_ok());
        assert!(SessionName::parse("ValidSession123").is_ok());
        assert!(SessionName::parse("_underscore").is_ok());
    }

    #[test]
    fn test_session_name_empty() {
        let result = SessionName::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_invalid_start() {
        assert!(SessionName::parse("123session").is_err());
        assert!(SessionName::parse("-session").is_err());
    }

    #[test]
    fn test_session_name_invalid_chars() {
        assert!(SessionName::parse("session with spaces").is_err());
        assert!(SessionName::parse("session.with.dots").is_err());
    }

    // BeadId tests (from isolate_core::domain::identifiers)
    #[test]
    fn test_bead_id_valid() {
        assert!(BeadId::parse("bd-123").is_ok());
        assert!(BeadId::parse("bd-abc123").is_ok());
    }

    #[test]
    fn test_bead_id_no_prefix() {
        let result = BeadId::parse("123");
        assert!(result.is_err());
    }

    #[test]
    fn test_bead_id_invalid_suffix() {
        assert!(BeadId::parse("bd-123-456").is_err()); // Hyphens not allowed in hex part
        assert!(BeadId::parse("bd-abc_def").is_err()); // Underscores not allowed in hex part
    }

    // AgentId tests (from isolate_core::domain::identifiers)
    #[test]
    fn test_agent_id_valid() {
        assert!(AgentId::parse("agent-1").is_ok());
        assert!(AgentId::parse("test_agent").is_ok());
    }

    #[test]
    fn test_agent_id_empty() {
        assert!(AgentId::parse("").is_err());
    }

    // QueueId tests
    #[test]
    fn test_queue_id_valid() {
        assert!(QueueId::new(1).is_ok());
        assert!(QueueId::new(100).is_ok());
    }

    #[test]
    fn test_queue_id_invalid() {
        assert_eq!(QueueId::new(0), Err(DomainError::InvalidQueueId(0)));
        assert_eq!(QueueId::new(-1), Err(DomainError::InvalidQueueId(-1)));
    }

    // Priority tests
    #[test]
    fn test_priority_valid() {
        assert!(Priority::new(0).is_ok());
        assert!(Priority::new(5).is_ok());
        assert!(Priority::new(10).is_ok());
    }

    #[test]
    fn test_priority_invalid() {
        assert_eq!(Priority::new(-1), Err(DomainError::InvalidPriority(-1)));
        assert_eq!(Priority::new(11), Err(DomainError::InvalidPriority(11)));
    }

    // QueueAction tests
    #[test]
    fn test_queue_action_list() {
        let action = QueueAction::List;
        assert!(action.is_list());
        assert!(!action.is_status());
        assert!(!action.is_process());
    }

    #[test]
    fn test_queue_action_status() {
        let session = SessionName::parse("test").expect("valid");
        let action = QueueAction::Status {
            session: Some(session),
        };
        assert!(!action.is_list());
        assert!(action.is_status());
        assert!(!action.is_process());
    }
}
