//! Bead error types.

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::beads::{DomainError, IssueState};

/// Bead state matches the beads `IssueState`.
///
/// Closed state requires a timestamp inline.
pub type BeadState = IssueState;

/// Errors that can occur during bead operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BeadError {
    /// Invalid title
    #[error("invalid title: {0}")]
    InvalidTitle(String),

    /// Invalid description
    #[error("invalid description: {0}")]
    InvalidDescription(String),

    /// Invalid state transition
    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition { from: BeadState, to: BeadState },

    /// Cannot modify closed bead
    #[error("cannot modify closed bead")]
    CannotModifyClosed,

    /// Timestamps are not monotonic
    #[error("timestamps must be monotonic: updated_at ({updated_at}) < created_at ({created_at})")]
    NonMonotonicTimestamps {
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    },

    /// Title is required
    #[error("title is required")]
    TitleRequired,

    /// Domain error from beads module
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),
}
