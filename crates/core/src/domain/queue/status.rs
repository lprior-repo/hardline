//! Queue status - State machine for queue entry lifecycle
//!
//! Represents all possible states a queue entry can be in through its lifecycle.
//! All state transitions are validated via `transition_to`.

use crate::domain::validation::{ValidationError, ValidationResult};

/// Maximum priority value for queue entries
pub const MAX_PRIORITY: u32 = 100;

/// Status of a queue entry
///
/// Represents the state machine for a queue entry through its lifecycle.
/// All state transitions are validated via `transition_to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum QueueStatus {
    /// Waiting to be processed
    Pending,
    /// Claimed by an agent
    Claimed,
    /// Currently being rebased
    Rebasing,
    /// Running tests
    Testing,
    /// Ready to merge
    ReadyToMerge,
    /// Currently merging
    Merging,
    /// Successfully merged
    Merged,
    /// Failed with retryable error
    FailedRetryable,
    /// Failed terminally
    FailedTerminal,
    /// Cancelled
    Cancelled,
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Claimed => write!(f, "claimed"),
            Self::Rebasing => write!(f, "rebasing"),
            Self::Testing => write!(f, "testing"),
            Self::ReadyToMerge => write!(f, "ready_to_merge"),
            Self::Merging => write!(f, "merging"),
            Self::Merged => write!(f, "merged"),
            Self::FailedRetryable => write!(f, "failed_retryable"),
            Self::FailedTerminal => write!(f, "failed_terminal"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl QueueStatus {
    /// Check if this is a terminal state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Merged | Self::FailedTerminal | Self::Cancelled)
    }

    /// Check if this is a failed state.
    #[must_use]
    pub const fn is_failed(self) -> bool {
        matches!(self, Self::FailedRetryable | Self::FailedTerminal)
    }

    /// Try to transition to a new state using Railway-Oriented Programming.
    ///
    /// # Errors
    /// Returns `ValidationError::InvalidStateTransition` if the transition is not allowed.
    pub fn transition_to(self, new_status: Self) -> ValidationResult<Self> {
        match (self, new_status) {
            // Valid transitions - all lead to Ok(new_status)
            (Self::Pending, Self::Claimed | Self::Cancelled)
            | (Self::Claimed, Self::Rebasing | Self::Cancelled)
            | (Self::Rebasing, Self::Testing | Self::FailedRetryable)
            | (Self::Testing, Self::ReadyToMerge | Self::FailedRetryable | Self::FailedTerminal)
            | (Self::ReadyToMerge, Self::Merging | Self::FailedRetryable)
            | (Self::Merging, Self::Merged | Self::FailedRetryable)
            | (Self::FailedRetryable, Self::Pending | Self::Cancelled) => Ok(new_status),

            // Terminal states and invalid transitions
            _ => Err(ValidationError::InvalidStateTransition {
                from: self.to_string(),
                to: new_status.to_string(),
            }),
        }
    }
}
