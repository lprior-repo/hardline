//! Queue status - State machine for queue entry lifecycle

use crate::domain::validation::{ValidationError, ValidationResult};

/// Maximum priority value for queue entries
pub const MAX_PRIORITY: u32 = 100;

/// Status of a queue entry
///
/// Represents the state machine for a queue entry through its lifecycle.
/// All state transitions are validated via `transition_to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum QueueStatus {
    /// Waiting to be processed
    #[default]
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
    /// This is the **single source of truth** for all state transitions.
    /// Every state machine query (`can_transition`, `is_active`, `is_terminal`)
    /// must be consistent with these rules.
    ///
    /// # State transition diagram
    ///
    /// ```text
    ///   Pending ──claim──> Claimed ──rebase──> Rebasing ──test──> Testing
    ///      ^                  |                    |                  |
    ///      |                  |                    |     ready    fail_retry|fail_terminal
    ///      |                  |                    |       v        v        v
    ///      |                  |                    |  ReadyToMerge FailedRetryable
    ///      |                  |                    |       |              |
    ///      |                  |                    |    merge          cancel/retry
    ///      |                  |                    |       v              v
    ///      |                  |                    |    Merging      (Pending or Cancelled)
    ///      |                  |                    |       |
    ///      |                  |                    |    merged
    ///      |                  |                    |       v
    ///      |                  |                    |    Merged (terminal)
    ///      |                  |                    |
    ///      |                  |               failed_terminal
    ///      |                  |                    v
    ///      |                  |           FailedTerminal (terminal)
    ///      |                  |
    ///      +--- retry --------+
    ///      |
    ///   cancel (from ANY non-terminal state)
    ///      v
    ///   Cancelled (terminal)
    /// ```
    ///
    /// # Errors
    /// Returns `ValidationError::InvalidStateTransition` if the transition is not allowed.
    pub fn transition_to(self, new_status: Self) -> ValidationResult<Self> {
        match (self, new_status) {
            // Valid transitions from Pending
            (Self::Pending, Self::Claimed | Self::Cancelled) => Ok(new_status),

            // Valid transitions from Claimed
            (Self::Claimed, Self::Rebasing | Self::Cancelled) => Ok(new_status),

            // Valid transitions from Rebasing
            (Self::Rebasing, Self::Testing | Self::Cancelled) => Ok(new_status),

            // Valid transitions from Testing
            (
                Self::Testing,
                Self::ReadyToMerge | Self::FailedRetryable | Self::FailedTerminal | Self::Cancelled,
            ) => Ok(new_status),

            // Valid transitions from ReadyToMerge
            (Self::ReadyToMerge, Self::Merging | Self::Cancelled) => Ok(new_status),

            // Valid transitions from Merging
            (Self::Merging, Self::Merged | Self::Cancelled) => Ok(new_status),

            // Valid transitions from FailedRetryable
            (Self::FailedRetryable, Self::Pending | Self::Cancelled) => Ok(new_status),

            // Terminal states and invalid transitions
            _ => Err(ValidationError::InvalidStateTransition {
                from: self.to_string(),
                to: new_status.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_status_display_all_variants() {
        assert_eq!(format!("{}", QueueStatus::Pending), "pending");
        assert_eq!(format!("{}", QueueStatus::Claimed), "claimed");
        assert_eq!(format!("{}", QueueStatus::Rebasing), "rebasing");
        assert_eq!(format!("{}", QueueStatus::Testing), "testing");
        assert_eq!(format!("{}", QueueStatus::ReadyToMerge), "ready_to_merge");
        assert_eq!(format!("{}", QueueStatus::Merging), "merging");
        assert_eq!(format!("{}", QueueStatus::Merged), "merged");
        assert_eq!(
            format!("{}", QueueStatus::FailedRetryable),
            "failed_retryable"
        );
        assert_eq!(
            format!("{}", QueueStatus::FailedTerminal),
            "failed_terminal"
        );
        assert_eq!(format!("{}", QueueStatus::Cancelled), "cancelled");
    }

    #[test]
    fn queue_status_all_transitions() {
        // Happy path: Pending -> Claimed -> Rebasing -> Testing -> ReadyToMerge -> Merging -> Merged
        let chain = [
            (QueueStatus::Pending, QueueStatus::Claimed),
            (QueueStatus::Claimed, QueueStatus::Rebasing),
            (QueueStatus::Rebasing, QueueStatus::Testing),
            (QueueStatus::Testing, QueueStatus::ReadyToMerge),
            (QueueStatus::ReadyToMerge, QueueStatus::Merging),
            (QueueStatus::Merging, QueueStatus::Merged),
        ];
        for (from, to) in &chain {
            assert!(
                from.transition_to(*to).is_ok(),
                "Transition from {:?} to {:?} should succeed",
                from,
                to
            );
        }
    }

    #[test]
    fn queue_status_cancel_from_all_non_terminal_states() {
        // Cancel is allowed from any non-terminal state
        let cancellable = [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::FailedRetryable,
        ];
        for status in &cancellable {
            assert!(
                status.transition_to(QueueStatus::Cancelled).is_ok(),
                "Cancel from {:?} should succeed",
                status
            );
        }
    }

    #[test]
    fn queue_status_cancel_from_terminal_rejected() {
        let terminal = [
            QueueStatus::Merged,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ];
        for status in &terminal {
            assert!(
                status.transition_to(QueueStatus::Pending).is_err(),
                "Transition from terminal {:?} should be rejected",
                status
            );
        }
    }

    #[test]
    fn queue_status_is_terminal_comprehensive() {
        assert!(QueueStatus::Merged.is_terminal());
        assert!(QueueStatus::FailedTerminal.is_terminal());
        assert!(QueueStatus::Cancelled.is_terminal());
        assert!(!QueueStatus::Pending.is_terminal());
        assert!(!QueueStatus::Claimed.is_terminal());
        assert!(!QueueStatus::Rebasing.is_terminal());
        assert!(!QueueStatus::Testing.is_terminal());
        assert!(!QueueStatus::ReadyToMerge.is_terminal());
        assert!(!QueueStatus::Merging.is_terminal());
        assert!(!QueueStatus::FailedRetryable.is_terminal());
    }

    #[test]
    fn queue_status_is_failed_comprehensive() {
        assert!(QueueStatus::FailedRetryable.is_failed());
        assert!(QueueStatus::FailedTerminal.is_failed());
        assert!(!QueueStatus::Pending.is_failed());
        assert!(!QueueStatus::Claimed.is_failed());
        assert!(!QueueStatus::Merged.is_failed());
        assert!(!QueueStatus::Cancelled.is_failed());
    }

    #[test]
    fn queue_status_failed_retryable_to_pending() {
        let result = QueueStatus::FailedRetryable.transition_to(QueueStatus::Pending);
        assert!(result.is_ok());
    }

    #[test]
    fn queue_status_invalid_transition_error_message() {
        let result = QueueStatus::Pending.transition_to(QueueStatus::Merged);
        match result {
            Err(ValidationError::InvalidStateTransition { from, to }) => {
                assert_eq!(from, "pending");
                assert_eq!(to, "merged");
            }
            _ => panic!("Expected InvalidStateTransition error"),
        }
    }

    #[test]
    fn queue_status_clone_and_eq() {
        let a = QueueStatus::Pending;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn queue_status_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(QueueStatus::Pending);
        set.insert(QueueStatus::Pending);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn queue_status_serde_roundtrip() {
        let statuses = [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::Merged,
            QueueStatus::FailedRetryable,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ];
        for status in &statuses {
            let json = serde_json::to_string(status).unwrap();
            let back: QueueStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*status, back);
        }
    }

    #[test]
    fn queue_status_serde_uses_pascal_case() {
        let json = serde_json::to_string(&QueueStatus::ReadyToMerge).unwrap();
        assert_eq!(json, "\"ReadyToMerge\"");
    }

    #[test]
    fn max_priority_value() {
        assert_eq!(MAX_PRIORITY, 100);
    }

    #[test]
    fn queue_status_testing_to_failed_retryable() {
        let result = QueueStatus::Testing.transition_to(QueueStatus::FailedRetryable);
        assert!(result.is_ok());
    }

    #[test]
    fn queue_status_testing_to_failed_terminal() {
        let result = QueueStatus::Testing.transition_to(QueueStatus::FailedTerminal);
        assert!(result.is_ok());
    }

    #[test]
    fn queue_status_ready_to_merge_to_failed_retryable_rejected() {
        let result = QueueStatus::ReadyToMerge.transition_to(QueueStatus::FailedRetryable);
        assert!(result.is_err());
    }

    #[test]
    fn queue_status_merging_to_failed_retryable_rejected() {
        let result = QueueStatus::Merging.transition_to(QueueStatus::FailedRetryable);
        assert!(result.is_err());
    }

    #[test]
    fn queue_status_rebasing_to_failed_retryable_rejected() {
        let result = QueueStatus::Rebasing.transition_to(QueueStatus::FailedRetryable);
        assert!(result.is_err());
    }
}
