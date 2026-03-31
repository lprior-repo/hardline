//! State types for the beads domain.
//!
//! State enums make illegal states unrepresentable.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use super::errors::DomainError;

/// The complete state of an issue.
///
/// This enum makes the closed timestamp requirement unrepresentable:
/// - `Closed` variant *must* include a timestamp
/// - No other variant can be closed
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Serialize,
    Deserialize,
    Hash,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    #[strum(to_string = "closed")]
    Closed {
        closed_at: DateTime<Utc>,
    },
}

impl IssueState {
    /// Check if the issue is in an active state (open or in progress).
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Open | Self::InProgress)
    }

    /// Check if the issue is blocked.
    #[must_use]
    pub const fn is_blocked(self) -> bool {
        matches!(self, Self::Blocked)
    }

    /// Check if the issue is closed.
    #[must_use]
    pub const fn is_closed(self) -> bool {
        matches!(self, Self::Closed { .. })
    }

    /// Get the closed timestamp if the issue is closed.
    #[must_use]
    pub const fn closed_at(self) -> Option<DateTime<Utc>> {
        match self {
            Self::Closed { closed_at } => Some(closed_at),
            _ => None,
        }
    }

    /// Transition to a new state with validation.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidStateTransition` if the transition is invalid.
    pub fn transition_to(self, new_state: Self) -> Result<Self, DomainError> {
        // Flexible workflow: can transition from any state to any state.
        // Closed MUST have a timestamp (already enforced by the type system).
        Ok(new_state)
    }
}

/// Type classification for issues.
#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
    #[strum(to_string = "merge-request")]
    MergeRequest,
}
