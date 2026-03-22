//! Domain errors for the beads issue tracker.
//!
//! Structured errors using thiserror for explicit error handling.

use thiserror::Error;

/// Errors that can occur in the beads domain.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("ID cannot be empty")]
    EmptyId,

    #[error("ID must match pattern: {0}")]
    InvalidIdPattern(String),

    #[error("Title cannot be empty")]
    EmptyTitle,

    #[error("Title exceeds maximum length of {max} characters (got {got})")]
    TitleTooLong { max: usize, got: usize },

    #[error("Description exceeds maximum length of {max} characters")]
    DescriptionTooLong { max: usize },

    #[error("Invalid datetime format: {0}")]
    InvalidDatetime(String),

    #[error("Issue not found: {0}")]
    NotFound(String),

    #[error("Duplicate issue ID: {0}")]
    DuplicateId(String),

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Closed issues must have a closed_at timestamp")]
    ClosedWithoutTimestamp,

    #[error("Invalid filter criteria: {0}")]
    InvalidFilter(String),
}
