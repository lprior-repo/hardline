//! Session status state machine and operations
//!
//! Session lifecycle: Creating -> Active -> Paused/Completed
//!                     Creating -> Failed

use serde::{Deserialize, Serialize};

use crate::lifecycle::LifecycleState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Creating,
    Active,
    Paused,
    Completed,
    Failed,
}

impl SessionStatus {
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Creating | Self::Paused, Self::Active)
                | (Self::Creating, Self::Failed)
                | (Self::Active, Self::Paused | Self::Completed)
                | (Self::Paused, Self::Completed)
        )
    }

    #[must_use]
    pub fn valid_next_states(self) -> Vec<Self> {
        match self {
            Self::Creating => vec![Self::Active, Self::Failed],
            Self::Active => vec![Self::Paused, Self::Completed],
            Self::Paused => vec![Self::Active, Self::Completed],
            Self::Completed | Self::Failed => vec![],
        }
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    #[must_use]
    pub const fn all_states() -> &'static [Self] {
        &[
            Self::Creating,
            Self::Active,
            Self::Paused,
            Self::Completed,
            Self::Failed,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    Status,
    Diff,
    Focus,
    Remove,
}

impl SessionStatus {
    pub fn allowed_operations(self) -> Vec<Operation> {
        match self {
            Self::Creating => vec![],
            Self::Active => vec![
                Operation::Status,
                Operation::Diff,
                Operation::Focus,
                Operation::Remove,
            ],
            Self::Paused => vec![Operation::Status, Operation::Focus, Operation::Remove],
            Self::Completed | Self::Failed => vec![Operation::Remove],
        }
    }

    #[must_use]
    pub fn allows_operation(self, op: Operation) -> bool {
        self.allowed_operations().contains(&op)
    }
}

impl LifecycleState for SessionStatus {
    fn can_transition_to(self, next: Self) -> bool {
        self.can_transition_to(next)
    }

    fn valid_next_states(self) -> Vec<Self> {
        self.valid_next_states()
    }

    fn is_terminal(self) -> bool {
        self.is_terminal()
    }

    fn all_states() -> &'static [Self] {
        Self::all_states()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- is_terminal --

    #[test]
    fn test_completed_is_terminal() {
        assert!(SessionStatus::Completed.is_terminal());
    }

    #[test]
    fn test_failed_is_terminal() {
        assert!(SessionStatus::Failed.is_terminal());
    }

    #[test]
    fn test_creating_not_terminal() {
        assert!(!SessionStatus::Creating.is_terminal());
    }

    #[test]
    fn test_active_not_terminal() {
        assert!(!SessionStatus::Active.is_terminal());
    }

    #[test]
    fn test_paused_not_terminal() {
        assert!(!SessionStatus::Paused.is_terminal());
    }

    // -- Valid transitions from Creating --

    #[test]
    fn test_creating_to_active() {
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
    }

    #[test]
    fn test_creating_to_failed() {
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Failed));
    }

    #[test]
    fn test_creating_to_paused_rejected() {
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));
    }

    #[test]
    fn test_creating_to_completed_rejected() {
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Completed));
    }

    #[test]
    fn test_creating_to_creating_rejected() {
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Creating));
    }

    // -- Valid transitions from Active --

    #[test]
    fn test_active_to_paused() {
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
    }

    #[test]
    fn test_active_to_completed() {
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Completed));
    }

    #[test]
    fn test_active_to_creating_rejected() {
        assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Creating));
    }

    #[test]
    fn test_active_to_active_rejected() {
        assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Active));
    }

    // -- Valid transitions from Paused --

    #[test]
    fn test_paused_to_active() {
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
    }

    #[test]
    fn test_paused_to_completed() {
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Completed));
    }

    #[test]
    fn test_paused_to_creating_rejected() {
        assert!(!SessionStatus::Paused.can_transition_to(SessionStatus::Creating));
    }

    // -- Terminal state transitions --

    #[test]
    fn test_completed_rejects_all_transitions() {
        for &next in SessionStatus::all_states() {
            assert!(
                !SessionStatus::Completed.can_transition_to(next),
                "Completed should not allow transition to {:?}",
                next
            );
        }
    }

    #[test]
    fn test_failed_rejects_all_transitions() {
        for &next in SessionStatus::all_states() {
            assert!(
                !SessionStatus::Failed.can_transition_to(next),
                "Failed should not allow transition to {:?}",
                next
            );
        }
    }

    // -- valid_next_states --

    #[test]
    fn test_creating_valid_next_states() {
        let next = SessionStatus::Creating.valid_next_states();
        assert_eq!(next.len(), 2);
        assert!(next.contains(&SessionStatus::Active));
        assert!(next.contains(&SessionStatus::Failed));
    }

    #[test]
    fn test_active_valid_next_states() {
        let next = SessionStatus::Active.valid_next_states();
        assert_eq!(next.len(), 2);
        assert!(next.contains(&SessionStatus::Paused));
        assert!(next.contains(&SessionStatus::Completed));
    }

    #[test]
    fn test_paused_valid_next_states() {
        let next = SessionStatus::Paused.valid_next_states();
        assert_eq!(next.len(), 2);
        assert!(next.contains(&SessionStatus::Active));
        assert!(next.contains(&SessionStatus::Completed));
    }

    #[test]
    fn test_completed_valid_next_states_empty() {
        assert!(SessionStatus::Completed.valid_next_states().is_empty());
    }

    #[test]
    fn test_failed_valid_next_states_empty() {
        assert!(SessionStatus::Failed.valid_next_states().is_empty());
    }

    // -- all_states --

    #[test]
    fn test_all_states_returns_five_variants() {
        assert_eq!(SessionStatus::all_states().len(), 5);
    }

    #[test]
    fn test_all_states_is_exhaustive() {
        let all = SessionStatus::all_states();
        assert!(all.contains(&SessionStatus::Creating));
        assert!(all.contains(&SessionStatus::Active));
        assert!(all.contains(&SessionStatus::Paused));
        assert!(all.contains(&SessionStatus::Completed));
        assert!(all.contains(&SessionStatus::Failed));
    }

    // -- Transition consistency: can_transition_to matches valid_next_states --

    #[test]
    fn test_transition_consistency() {
        for &from_state in SessionStatus::all_states() {
            let valid_nexts = from_state.valid_next_states();
            for &to_state in SessionStatus::all_states() {
                let can = from_state.can_transition_to(to_state);
                let in_list = valid_nexts.contains(&to_state);
                assert_eq!(
                    can, in_list,
                    "Inconsistency: {:?} -> {:?}: can_transition={can}, in_valid_list={in_list}",
                    from_state, to_state
                );
            }
        }
    }

    // -- allowed_operations --

    #[test]
    fn test_creating_allows_no_operations() {
        assert!(SessionStatus::Creating.allowed_operations().is_empty());
    }

    #[test]
    fn test_active_allows_all_operations() {
        let ops = SessionStatus::Active.allowed_operations();
        assert_eq!(ops.len(), 4);
        assert!(ops.contains(&Operation::Status));
        assert!(ops.contains(&Operation::Diff));
        assert!(ops.contains(&Operation::Focus));
        assert!(ops.contains(&Operation::Remove));
    }

    #[test]
    fn test_paused_allows_status_focus_remove() {
        let ops = SessionStatus::Paused.allowed_operations();
        assert_eq!(ops.len(), 3);
        assert!(ops.contains(&Operation::Status));
        assert!(ops.contains(&Operation::Focus));
        assert!(ops.contains(&Operation::Remove));
        assert!(!ops.contains(&Operation::Diff));
    }

    #[test]
    fn test_completed_allows_only_remove() {
        let ops = SessionStatus::Completed.allowed_operations();
        assert_eq!(ops.len(), 1);
        assert!(ops.contains(&Operation::Remove));
    }

    #[test]
    fn test_failed_allows_only_remove() {
        let ops = SessionStatus::Failed.allowed_operations();
        assert_eq!(ops.len(), 1);
        assert!(ops.contains(&Operation::Remove));
    }

    // -- allows_operation --

    #[test]
    fn test_allows_operation_active() {
        assert!(SessionStatus::Active.allows_operation(Operation::Status));
        assert!(SessionStatus::Active.allows_operation(Operation::Diff));
        assert!(SessionStatus::Active.allows_operation(Operation::Focus));
        assert!(SessionStatus::Active.allows_operation(Operation::Remove));
    }

    #[test]
    fn test_allows_operation_creating_none() {
        assert!(!SessionStatus::Creating.allows_operation(Operation::Status));
        assert!(!SessionStatus::Creating.allows_operation(Operation::Diff));
    }

    #[test]
    fn test_allows_operation_paused_no_diff() {
        assert!(SessionStatus::Paused.allows_operation(Operation::Status));
        assert!(!SessionStatus::Paused.allows_operation(Operation::Diff));
    }

    // -- PartialEq --

    #[test]
    fn test_status_equality() {
        assert_eq!(SessionStatus::Active, SessionStatus::Active);
        assert_ne!(SessionStatus::Active, SessionStatus::Paused);
    }

    // -- Copy --

    #[test]
    fn test_copy() {
        let status = SessionStatus::Active;
        let copied = status;
        assert_eq!(status, copied);
    }
}
