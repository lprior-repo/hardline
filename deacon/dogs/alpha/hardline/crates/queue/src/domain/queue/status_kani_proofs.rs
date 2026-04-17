//! Kani proofs for QueueStatus state machine invariants.
//!
//! # Invariants Proven
//!
//! 1. Terminal states are correctly identified
//! 2. Failed states are correctly identified
//! 3. Valid transitions are accepted
//! 4. Invalid transitions are rejected

#[cfg(kani)]
mod proof {
    use crate::domain::queue::status::QueueStatus;

    #[kani::proof]
    fn verify_terminal_states() {
        let terminal_statuses = [
            QueueStatus::Merged,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ];

        for status in terminal_statuses {
            assert!(status.is_terminal());
        }
    }

    #[kani::proof]
    fn verify_non_terminal_states() {
        let non_terminal = [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::FailedRetryable,
        ];

        for status in non_terminal {
            assert!(!status.is_terminal());
        }
    }

    #[kani::proof]
    fn verify_failed_states() {
        let failed_statuses = [QueueStatus::FailedRetryable, QueueStatus::FailedTerminal];

        for status in failed_statuses {
            assert!(status.is_failed());
        }
    }

    #[kani::proof]
    fn verify_pending_valid_transitions() {
        let pending = QueueStatus::Pending;

        assert!(pending.transition_to(QueueStatus::Claimed).is_ok());
        assert!(pending.transition_to(QueueStatus::Cancelled).is_ok());
    }

    #[kani::proof]
    fn verify_pending_invalid_transitions() {
        let pending = QueueStatus::Pending;

        assert!(pending.transition_to(QueueStatus::Merged).is_err());
        assert!(pending.transition_to(QueueStatus::Pending).is_err());
        assert!(pending.transition_to(QueueStatus::Testing).is_err());
    }

    #[kani::proof]
    fn verify_claimed_valid_transitions() {
        let claimed = QueueStatus::Claimed;

        assert!(claimed.transition_to(QueueStatus::Rebasing).is_ok());
        assert!(claimed.transition_to(QueueStatus::Cancelled).is_ok());
    }

    #[kani::proof]
    fn verify_rebasing_valid_transitions() {
        let rebasing = QueueStatus::Rebasing;

        assert!(rebasing.transition_to(QueueStatus::Testing).is_ok());
        assert!(rebasing.transition_to(QueueStatus::FailedRetryable).is_ok());
    }

    #[kani::proof]
    fn verify_testing_valid_transitions() {
        let testing = QueueStatus::Testing;

        assert!(testing.transition_to(QueueStatus::ReadyToMerge).is_ok());
        assert!(testing.transition_to(QueueStatus::FailedRetryable).is_ok());
        assert!(testing.transition_to(QueueStatus::FailedTerminal).is_ok());
    }

    #[kani::proof]
    fn verify_ready_to_merge_valid_transitions() {
        let ready = QueueStatus::ReadyToMerge;

        assert!(ready.transition_to(QueueStatus::Merging).is_ok());
        assert!(ready.transition_to(QueueStatus::FailedRetryable).is_ok());
    }

    #[kani::proof]
    fn verify_merging_valid_transitions() {
        let merging = QueueStatus::Merging;

        assert!(merging.transition_to(QueueStatus::Merged).is_ok());
        assert!(merging.transition_to(QueueStatus::FailedRetryable).is_ok());
    }

    #[kani::proof]
    fn verify_failed_retryable_valid_transitions() {
        let failed = QueueStatus::FailedRetryable;

        assert!(failed.transition_to(QueueStatus::Pending).is_ok());
        assert!(failed.transition_to(QueueStatus::Cancelled).is_ok());
    }

    #[kani::proof]
    fn verify_terminal_states_reject_all_transitions() {
        let terminal = kani::any::<QueueStatus>();
        kani::assume(matches!(
            terminal,
            QueueStatus::Merged | QueueStatus::FailedTerminal | QueueStatus::Cancelled
        ));

        let target = kani::any::<QueueStatus>();
        assert!(terminal.transition_to(target).is_err());
    }

    #[kani::proof]
    fn verify_valid_transitions_are_deterministic() {
        let from = kani::any::<QueueStatus>();
        let to = kani::any::<QueueStatus>();

        let result1 = from.transition_to(to);
        let result2 = from.transition_to(to);

        assert_eq!(result1.is_ok(), result2.is_ok());
    }
}
