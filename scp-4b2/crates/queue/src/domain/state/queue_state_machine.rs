use crate::domain::entities::QueueStatus;
use crate::error::QueueError;

pub struct QueueStateMachine;

impl QueueStateMachine {
    pub fn can_transition(from: QueueStatus, to: QueueStatus) -> bool {
        match (from, to) {
            (QueueStatus::Pending, QueueStatus::Claimed) => true,
            (QueueStatus::Claimed, QueueStatus::Rebasing) => true,
            (QueueStatus::Rebasing, QueueStatus::Testing) => true,
            (QueueStatus::Testing, QueueStatus::ReadyToMerge) => true,
            (QueueStatus::Testing, QueueStatus::FailedRetryable) => true,
            (QueueStatus::Testing, QueueStatus::FailedTerminal) => true,
            (QueueStatus::ReadyToMerge, QueueStatus::Merging) => true,
            (QueueStatus::Merging, QueueStatus::Merged) => true,
            (_, QueueStatus::Cancelled) => true,
            _ => false,
        }
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
}
