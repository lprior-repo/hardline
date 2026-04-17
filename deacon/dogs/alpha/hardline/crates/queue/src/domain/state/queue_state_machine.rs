use crate::domain::entities::QueueStatus;
use crate::domain::queue::status::QueueStatus as CanonicalQueueStatus;
use crate::error::QueueError;

/// Queue state machine validation utilities.
///
/// **DEPRECATED**: All transition logic now lives in `QueueStatus::transition_to()`
/// (in `domain::queue::status`). This struct delegates to that single source of truth.
/// Prefer calling `QueueStatus::transition_to()` directly for new code.
pub struct QueueStateMachine;

/// Convert between the two identical-but-separate `QueueStatus` enums used in this crate.
/// TODO(#unify-queue-status): The crate has two `QueueStatus` enums (entities vs queue module).
///       This bridge exists only until they are unified into one.
fn to_canonical(status: QueueStatus) -> CanonicalQueueStatus {
    match status {
        QueueStatus::Pending => CanonicalQueueStatus::Pending,
        QueueStatus::Claimed => CanonicalQueueStatus::Claimed,
        QueueStatus::Rebasing => CanonicalQueueStatus::Rebasing,
        QueueStatus::Testing => CanonicalQueueStatus::Testing,
        QueueStatus::ReadyToMerge => CanonicalQueueStatus::ReadyToMerge,
        QueueStatus::Merging => CanonicalQueueStatus::Merging,
        QueueStatus::Merged => CanonicalQueueStatus::Merged,
        QueueStatus::FailedRetryable => CanonicalQueueStatus::FailedRetryable,
        QueueStatus::FailedTerminal => CanonicalQueueStatus::FailedTerminal,
        QueueStatus::Cancelled => CanonicalQueueStatus::Cancelled,
    }
}

impl QueueStateMachine {
    /// Check if a transition is valid by delegating to `QueueStatus::transition_to()`.
    ///
    /// This is the single source of truth -- all callers converge here.
    pub fn can_transition(from: QueueStatus, to: QueueStatus) -> bool {
        to_canonical(from).transition_to(to_canonical(to)).is_ok()
    }

    pub fn validate_transition(from: QueueStatus, to: QueueStatus) -> Result<(), QueueError> {
        if Self::can_transition(from, to) {
            Ok(())
        } else {
            Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", from),
                to: format!("{:?}", to),
            })
        }
    }

    pub fn is_terminal(status: QueueStatus) -> bool {
        matches!(
            status,
            QueueStatus::Merged | QueueStatus::FailedTerminal | QueueStatus::Cancelled
        )
    }

    pub fn is_active(status: QueueStatus) -> bool {
        matches!(
            status,
            QueueStatus::Claimed
                | QueueStatus::Rebasing
                | QueueStatus::Testing
                | QueueStatus::ReadyToMerge
                | QueueStatus::Merging
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_machine_pending_to_claimed_is_valid() {
        assert!(QueueStateMachine::can_transition(
            QueueStatus::Pending,
            QueueStatus::Claimed
        ));
    }

    #[test]
    fn state_machine_pending_to_merged_is_invalid() {
        assert!(!QueueStateMachine::can_transition(
            QueueStatus::Pending,
            QueueStatus::Merged
        ));
    }

    #[test]
    fn state_machine_cancelled_is_terminal() {
        assert!(QueueStateMachine::is_terminal(QueueStatus::Cancelled));
    }

    #[test]
    fn state_machine_claimed_is_active() {
        assert!(QueueStateMachine::is_active(QueueStatus::Claimed));
    }

    #[test]
    fn state_machine_pending_is_not_active() {
        assert!(!QueueStateMachine::is_active(QueueStatus::Pending));
    }

    #[test]
    fn state_machine_merged_is_terminal() {
        assert!(QueueStateMachine::is_terminal(QueueStatus::Merged));
    }

    #[test]
    fn state_machine_failed_terminal_is_terminal() {
        assert!(QueueStateMachine::is_terminal(QueueStatus::FailedTerminal));
    }

    #[test]
    fn state_machine_pending_is_not_terminal() {
        assert!(!QueueStateMachine::is_terminal(QueueStatus::Pending));
    }

    #[test]
    fn state_machine_claimed_is_not_terminal() {
        assert!(!QueueStateMachine::is_terminal(QueueStatus::Claimed));
    }

    #[test]
    fn state_machine_failed_retryable_is_not_terminal() {
        assert!(!QueueStateMachine::is_terminal(
            QueueStatus::FailedRetryable
        ));
    }

    #[test]
    fn state_machine_validate_transition_valid() {
        assert!(
            QueueStateMachine::validate_transition(QueueStatus::Pending, QueueStatus::Claimed)
                .is_ok()
        );
    }

    #[test]
    fn state_machine_validate_transition_invalid() {
        let result =
            QueueStateMachine::validate_transition(QueueStatus::Pending, QueueStatus::Merged);
        assert!(result.is_err());
        if let Err(QueueError::InvalidStateTransition { from, to }) = result {
            assert!(from.contains("Pending"));
            assert!(to.contains("Merged"));
        }
    }

    #[test]
    fn state_machine_rebasing_is_active() {
        assert!(QueueStateMachine::is_active(QueueStatus::Rebasing));
    }

    #[test]
    fn state_machine_testing_is_active() {
        assert!(QueueStateMachine::is_active(QueueStatus::Testing));
    }

    #[test]
    fn state_machine_ready_to_merge_is_active() {
        assert!(QueueStateMachine::is_active(QueueStatus::ReadyToMerge));
    }

    #[test]
    fn state_machine_merging_is_active() {
        assert!(QueueStateMachine::is_active(QueueStatus::Merging));
    }

    #[test]
    fn state_machine_merged_is_not_active() {
        assert!(!QueueStateMachine::is_active(QueueStatus::Merged));
    }

    #[test]
    fn state_machine_failed_terminal_is_not_active() {
        assert!(!QueueStateMachine::is_active(QueueStatus::FailedTerminal));
    }

    #[test]
    fn state_machine_cancelled_is_not_active() {
        assert!(!QueueStateMachine::is_active(QueueStatus::Cancelled));
    }

    #[test]
    fn state_machine_all_cancel_transitions_valid() {
        let statuses = [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::FailedRetryable,
        ];
        for status in &statuses {
            assert!(
                QueueStateMachine::can_transition(*status, QueueStatus::Cancelled),
                "Cancel from {:?} should be allowed",
                status
            );
        }
    }

    #[test]
    fn state_machine_terminal_to_any_rejected() {
        let terminal = [
            QueueStatus::Merged,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ];
        let targets = [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
        ];
        for from in &terminal {
            for to in &targets {
                assert!(
                    !QueueStateMachine::can_transition(*from, *to),
                    "Transition from {:?} to {:?} should be rejected",
                    from,
                    to
                );
            }
        }
    }

    #[test]
    fn state_machine_happy_path_all_valid() {
        let path = [
            (QueueStatus::Pending, QueueStatus::Claimed),
            (QueueStatus::Claimed, QueueStatus::Rebasing),
            (QueueStatus::Rebasing, QueueStatus::Testing),
            (QueueStatus::Testing, QueueStatus::ReadyToMerge),
            (QueueStatus::ReadyToMerge, QueueStatus::Merging),
            (QueueStatus::Merging, QueueStatus::Merged),
        ];
        for (from, to) in &path {
            assert!(
                QueueStateMachine::can_transition(*from, *to),
                "{:?} -> {:?} should be valid",
                from,
                to
            );
        }
    }

    #[test]
    fn state_machine_failure_paths_valid() {
        let failure_paths = [
            (QueueStatus::Testing, QueueStatus::FailedRetryable),
            (QueueStatus::Testing, QueueStatus::FailedTerminal),
        ];
        for (from, to) in &failure_paths {
            assert!(
                QueueStateMachine::can_transition(*from, *to),
                "{:?} -> {:?} should be valid",
                from,
                to
            );
        }
    }

    #[test]
    fn state_machine_failed_retryable_not_from_ready_to_merge() {
        assert!(
            !QueueStateMachine::can_transition(
                QueueStatus::ReadyToMerge,
                QueueStatus::FailedRetryable
            ),
            "ReadyToMerge -> FailedRetryable should be rejected (entity does not support it)"
        );
    }

    #[test]
    fn state_machine_failed_retryable_not_from_merging() {
        assert!(
            !QueueStateMachine::can_transition(QueueStatus::Merging, QueueStatus::FailedRetryable),
            "Merging -> FailedRetryable should be rejected (entity does not support it)"
        );
    }

    #[test]
    fn state_machine_failed_retryable_not_from_rebasing() {
        assert!(
            !QueueStateMachine::can_transition(QueueStatus::Rebasing, QueueStatus::FailedRetryable),
            "Rebasing -> FailedRetryable should be rejected (entity does not support it)"
        );
    }

    #[test]
    fn state_machine_retryable_to_pending() {
        assert!(QueueStateMachine::can_transition(
            QueueStatus::FailedRetryable,
            QueueStatus::Pending
        ));
    }

    /// Exhaustive consistency check: every possible (from, to) pair must produce
    /// the same result from both `QueueStateMachine::can_transition` and
    /// `QueueStatus::transition_to` (the canonical source of truth).
    #[test]
    fn state_machine_consistent_with_queue_status_transition_to() {
        use crate::domain::queue::status::QueueStatus as CanonicalStatus;
        use crate::domain::validation::ValidationResult;

        fn to_canonical(status: QueueStatus) -> CanonicalStatus {
            match status {
                QueueStatus::Pending => CanonicalStatus::Pending,
                QueueStatus::Claimed => CanonicalStatus::Claimed,
                QueueStatus::Rebasing => CanonicalStatus::Rebasing,
                QueueStatus::Testing => CanonicalStatus::Testing,
                QueueStatus::ReadyToMerge => CanonicalStatus::ReadyToMerge,
                QueueStatus::Merging => CanonicalStatus::Merging,
                QueueStatus::Merged => CanonicalStatus::Merged,
                QueueStatus::FailedRetryable => CanonicalStatus::FailedRetryable,
                QueueStatus::FailedTerminal => CanonicalStatus::FailedTerminal,
                QueueStatus::Cancelled => CanonicalStatus::Cancelled,
            }
        }

        let all_statuses = [
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

        for from in &all_statuses {
            for to in &all_statuses {
                let sm_result = QueueStateMachine::can_transition(*from, *to);
                let canonical_result: ValidationResult<_> =
                    to_canonical(*from).transition_to(to_canonical(*to));
                let canonical_ok = canonical_result.is_ok();

                assert_eq!(
                    sm_result, canonical_ok,
                    "Inconsistency: QueueStateMachine says {:?} -> {:?} is {}, \
                     but QueueStatus::transition_to says {}",
                    from, to, sm_result, canonical_ok
                );
            }
        }
    }

    /// Verify `is_terminal` is consistent with the canonical `QueueStatus::is_terminal`.
    #[test]
    fn state_machine_is_terminal_consistent_with_queue_status() {
        use crate::domain::queue::status::QueueStatus as CanonicalStatus;

        let all = [
            (QueueStatus::Pending, CanonicalStatus::Pending),
            (QueueStatus::Claimed, CanonicalStatus::Claimed),
            (QueueStatus::Rebasing, CanonicalStatus::Rebasing),
            (QueueStatus::Testing, CanonicalStatus::Testing),
            (QueueStatus::ReadyToMerge, CanonicalStatus::ReadyToMerge),
            (QueueStatus::Merging, CanonicalStatus::Merging),
            (QueueStatus::Merged, CanonicalStatus::Merged),
            (
                QueueStatus::FailedRetryable,
                CanonicalStatus::FailedRetryable,
            ),
            (QueueStatus::FailedTerminal, CanonicalStatus::FailedTerminal),
            (QueueStatus::Cancelled, CanonicalStatus::Cancelled),
        ];

        for (entity_status, canonical_status) in &all {
            assert_eq!(
                QueueStateMachine::is_terminal(*entity_status),
                canonical_status.is_terminal(),
                "is_terminal mismatch for {:?}",
                entity_status
            );
        }
    }

    /// Verify `is_active` is the complement of `is_terminal` (excluding Pending and FailedRetryable).
    #[test]
    fn state_machine_is_active_is_complement_of_terminal() {
        let all_statuses = [
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

        for status in &all_statuses {
            let is_active = QueueStateMachine::is_active(*status);
            let is_terminal = QueueStateMachine::is_terminal(*status);

            // Active statuses are neither terminal nor Pending nor FailedRetryable
            let expected_active = !is_terminal
                && !matches!(status, QueueStatus::Pending | QueueStatus::FailedRetryable);

            assert_eq!(
                is_active, expected_active,
                "is_active mismatch for {:?}: got {}, expected {}",
                status, is_active, expected_active
            );
        }
    }
}
