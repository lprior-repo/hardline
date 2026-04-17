use crate::domain::entities::WorkspaceState;
use crate::error::WorkspaceError;

pub struct WorkspaceStateMachine;

impl WorkspaceStateMachine {
    pub fn can_transition(from: WorkspaceState, to: WorkspaceState) -> bool {
        matches!(
            (from, to),
            (WorkspaceState::Initializing, WorkspaceState::Active)
                | (WorkspaceState::Active, WorkspaceState::Locked)
                | (WorkspaceState::Locked, WorkspaceState::Active)
                | (WorkspaceState::Active, WorkspaceState::Corrupted)
                | (WorkspaceState::Locked, WorkspaceState::Corrupted)
                | (_, WorkspaceState::Deleted)
        )
    }

    pub fn validate_transition(
        from: WorkspaceState,
        to: WorkspaceState,
    ) -> Result<(), WorkspaceError> {
        if Self::can_transition(from, to) {
            Ok(())
        } else {
            Err(WorkspaceError::InvalidStateTransition {
                from: format!("{:?}", from),
                to: format!("{:?}", to),
            })
        }
    }

    pub fn is_terminal(state: WorkspaceState) -> bool {
        matches!(state, WorkspaceState::Deleted | WorkspaceState::Corrupted)
    }

    pub fn is_lockable(state: WorkspaceState) -> bool {
        state == WorkspaceState::Active
    }

    pub fn is_deletable(state: WorkspaceState) -> bool {
        !Self::is_terminal(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_machine_initializing_to_active_is_valid() {
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Initializing,
            WorkspaceState::Active
        ));
    }

    #[test]
    fn state_machine_active_to_locked_is_valid() {
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Active,
            WorkspaceState::Locked
        ));
    }

    #[test]
    fn state_machine_initializing_to_deleted_is_valid() {
        // The wildcard (_, Deleted) pattern means any state can transition to Deleted
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Initializing,
            WorkspaceState::Deleted
        ));
    }

    #[test]
    fn state_machine_deleted_is_terminal() {
        assert!(WorkspaceStateMachine::is_terminal(WorkspaceState::Deleted));
    }

    #[test]
    fn state_machine_active_is_lockable() {
        assert!(WorkspaceStateMachine::is_lockable(WorkspaceState::Active));
    }

    #[test]
    fn state_machine_locked_to_active_is_valid() {
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Locked,
            WorkspaceState::Active
        ));
    }

    #[test]
    fn state_machine_active_to_corrupted_is_valid() {
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Active,
            WorkspaceState::Corrupted
        ));
    }

    #[test]
    fn state_machine_locked_to_corrupted_is_valid() {
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Locked,
            WorkspaceState::Corrupted
        ));
    }

    #[test]
    fn state_machine_any_to_deleted_except_initializing() {
        // Active -> Deleted should be valid
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Active,
            WorkspaceState::Deleted
        ));
    }

    #[test]
    fn state_machine_corrupted_is_terminal() {
        assert!(WorkspaceStateMachine::is_terminal(
            WorkspaceState::Corrupted
        ));
    }

    #[test]
    fn state_machine_initializing_is_not_terminal() {
        assert!(!WorkspaceStateMachine::is_terminal(
            WorkspaceState::Initializing
        ));
    }

    #[test]
    fn state_machine_active_is_not_terminal() {
        assert!(!WorkspaceStateMachine::is_terminal(WorkspaceState::Active));
    }

    #[test]
    fn state_machine_locked_is_not_terminal() {
        assert!(!WorkspaceStateMachine::is_terminal(WorkspaceState::Locked));
    }

    #[test]
    fn state_machine_locked_is_not_lockable() {
        assert!(!WorkspaceStateMachine::is_lockable(WorkspaceState::Locked));
    }

    #[test]
    fn state_machine_initializing_is_not_lockable() {
        assert!(!WorkspaceStateMachine::is_lockable(
            WorkspaceState::Initializing
        ));
    }

    #[test]
    fn state_machine_deleted_is_not_lockable() {
        assert!(!WorkspaceStateMachine::is_lockable(WorkspaceState::Deleted));
    }

    #[test]
    fn state_machine_corrupted_is_not_lockable() {
        assert!(!WorkspaceStateMachine::is_lockable(
            WorkspaceState::Corrupted
        ));
    }

    #[test]
    fn state_machine_deletable_states() {
        assert!(WorkspaceStateMachine::is_deletable(
            WorkspaceState::Initializing
        ));
        assert!(WorkspaceStateMachine::is_deletable(WorkspaceState::Active));
        assert!(WorkspaceStateMachine::is_deletable(WorkspaceState::Locked));
    }

    #[test]
    fn state_machine_terminal_states_not_deletable() {
        assert!(!WorkspaceStateMachine::is_deletable(
            WorkspaceState::Deleted
        ));
        assert!(!WorkspaceStateMachine::is_deletable(
            WorkspaceState::Corrupted
        ));
    }

    #[test]
    fn state_machine_validate_transition_success() {
        assert!(WorkspaceStateMachine::validate_transition(
            WorkspaceState::Initializing,
            WorkspaceState::Active
        )
        .is_ok());
    }

    #[test]
    fn state_machine_validate_transition_failure() {
        let result = WorkspaceStateMachine::validate_transition(
            WorkspaceState::Initializing,
            WorkspaceState::Initializing,
        );
        assert!(result.is_err());
        match result.err() {
            Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                assert_eq!(from, "Initializing");
                assert_eq!(to, "Initializing");
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[test]
    fn state_machine_validate_transition_deleted_from_any() {
        // The wildcard (_, Deleted) means any state can transition to Deleted
        assert!(WorkspaceStateMachine::validate_transition(
            WorkspaceState::Active,
            WorkspaceState::Deleted
        )
        .is_ok());
        assert!(WorkspaceStateMachine::validate_transition(
            WorkspaceState::Initializing,
            WorkspaceState::Deleted
        )
        .is_ok());
    }

    #[test]
    fn state_machine_invalid_transitions_matrix() {
        let invalid = vec![
            (WorkspaceState::Initializing, WorkspaceState::Initializing),
            (WorkspaceState::Initializing, WorkspaceState::Locked),
            (WorkspaceState::Initializing, WorkspaceState::Corrupted),
            (WorkspaceState::Active, WorkspaceState::Initializing),
            (WorkspaceState::Active, WorkspaceState::Active),
            (WorkspaceState::Locked, WorkspaceState::Initializing),
            (WorkspaceState::Locked, WorkspaceState::Locked),
            (WorkspaceState::Corrupted, WorkspaceState::Active),
            (WorkspaceState::Corrupted, WorkspaceState::Locked),
            (WorkspaceState::Corrupted, WorkspaceState::Initializing),
            (WorkspaceState::Deleted, WorkspaceState::Active),
            (WorkspaceState::Deleted, WorkspaceState::Initializing),
            (WorkspaceState::Deleted, WorkspaceState::Locked),
        ];
        for (from, to) in invalid {
            assert!(
                !WorkspaceStateMachine::can_transition(from, to),
                "expected {from:?} -> {to:?} to be invalid"
            );
        }
    }

    #[test]
    fn state_machine_valid_transitions_matrix() {
        let valid = vec![
            (WorkspaceState::Initializing, WorkspaceState::Active),
            (WorkspaceState::Initializing, WorkspaceState::Deleted),
            (WorkspaceState::Active, WorkspaceState::Locked),
            (WorkspaceState::Active, WorkspaceState::Corrupted),
            (WorkspaceState::Active, WorkspaceState::Deleted),
            (WorkspaceState::Locked, WorkspaceState::Active),
            (WorkspaceState::Locked, WorkspaceState::Corrupted),
            (WorkspaceState::Locked, WorkspaceState::Deleted),
            (WorkspaceState::Corrupted, WorkspaceState::Deleted),
            (WorkspaceState::Deleted, WorkspaceState::Deleted),
        ];
        for (from, to) in valid {
            assert!(
                WorkspaceStateMachine::can_transition(from, to),
                "expected {from:?} -> {to:?} to be valid"
            );
        }
    }

    // --- Additional unit tests ---

    #[test]
    fn state_machine_validate_transition_err_message_format() {
        let result = WorkspaceStateMachine::validate_transition(
            WorkspaceState::Active,
            WorkspaceState::Initializing,
        );
        assert!(result.is_err());
        let err = result.err().unwrap();
        let msg = format!("{err}");
        assert!(msg.contains("Active"));
        assert!(msg.contains("Initializing"));
        assert!(msg.contains("Invalid state transition"));
    }

    #[test]
    fn state_machine_deleted_can_transition_to_deleted() {
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Deleted,
            WorkspaceState::Deleted
        ));
    }

    #[test]
    fn state_machine_corrupted_can_transition_to_deleted() {
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Corrupted,
            WorkspaceState::Deleted
        ));
    }

    #[test]
    fn state_machine_corrupted_cannot_transition_to_active() {
        assert!(!WorkspaceStateMachine::can_transition(
            WorkspaceState::Corrupted,
            WorkspaceState::Active
        ));
    }

    #[test]
    fn state_machine_corrupted_cannot_transition_to_locked() {
        assert!(!WorkspaceStateMachine::can_transition(
            WorkspaceState::Corrupted,
            WorkspaceState::Locked
        ));
    }

    #[test]
    fn state_machine_corrupted_cannot_transition_to_initializing() {
        assert!(!WorkspaceStateMachine::can_transition(
            WorkspaceState::Corrupted,
            WorkspaceState::Initializing
        ));
    }

    #[test]
    fn state_machine_deleted_cannot_transition_to_active() {
        assert!(!WorkspaceStateMachine::can_transition(
            WorkspaceState::Deleted,
            WorkspaceState::Active
        ));
    }

    #[test]
    fn state_machine_deleted_cannot_transition_to_locked() {
        assert!(!WorkspaceStateMachine::can_transition(
            WorkspaceState::Deleted,
            WorkspaceState::Locked
        ));
    }

    #[test]
    fn state_machine_deleted_cannot_transition_to_initializing() {
        assert!(!WorkspaceStateMachine::can_transition(
            WorkspaceState::Deleted,
            WorkspaceState::Initializing
        ));
    }

    #[test]
    fn state_machine_deleted_cannot_transition_to_corrupted() {
        assert!(!WorkspaceStateMachine::can_transition(
            WorkspaceState::Deleted,
            WorkspaceState::Corrupted
        ));
    }

    #[test]
    fn state_machine_is_lockable_only_for_active() {
        assert!(WorkspaceStateMachine::is_lockable(WorkspaceState::Active));
        assert!(!WorkspaceStateMachine::is_lockable(
            WorkspaceState::Initializing
        ));
        assert!(!WorkspaceStateMachine::is_lockable(WorkspaceState::Locked));
        assert!(!WorkspaceStateMachine::is_lockable(
            WorkspaceState::Corrupted
        ));
        assert!(!WorkspaceStateMachine::is_lockable(WorkspaceState::Deleted));
    }

    #[test]
    fn state_machine_is_deletable_for_non_terminal() {
        assert!(WorkspaceStateMachine::is_deletable(
            WorkspaceState::Initializing
        ));
        assert!(WorkspaceStateMachine::is_deletable(WorkspaceState::Active));
        assert!(WorkspaceStateMachine::is_deletable(WorkspaceState::Locked));
        assert!(!WorkspaceStateMachine::is_deletable(
            WorkspaceState::Deleted
        ));
        assert!(!WorkspaceStateMachine::is_deletable(
            WorkspaceState::Corrupted
        ));
    }

    #[test]
    fn state_machine_validate_all_valid_transitions_succeed() {
        let valid = vec![
            (WorkspaceState::Initializing, WorkspaceState::Active),
            (WorkspaceState::Initializing, WorkspaceState::Deleted),
            (WorkspaceState::Active, WorkspaceState::Locked),
            (WorkspaceState::Active, WorkspaceState::Corrupted),
            (WorkspaceState::Active, WorkspaceState::Deleted),
            (WorkspaceState::Locked, WorkspaceState::Active),
            (WorkspaceState::Locked, WorkspaceState::Corrupted),
            (WorkspaceState::Locked, WorkspaceState::Deleted),
            (WorkspaceState::Corrupted, WorkspaceState::Deleted),
            (WorkspaceState::Deleted, WorkspaceState::Deleted),
        ];
        for (from, to) in valid {
            assert!(
                WorkspaceStateMachine::validate_transition(from, to).is_ok(),
                "expected {from:?} -> {to:?} to be valid"
            );
        }
    }

    #[test]
    fn state_machine_validate_all_invalid_transitions_fail() {
        let invalid = vec![
            (WorkspaceState::Initializing, WorkspaceState::Initializing),
            (WorkspaceState::Initializing, WorkspaceState::Locked),
            (WorkspaceState::Initializing, WorkspaceState::Corrupted),
            (WorkspaceState::Active, WorkspaceState::Initializing),
            (WorkspaceState::Active, WorkspaceState::Active),
            (WorkspaceState::Locked, WorkspaceState::Initializing),
            (WorkspaceState::Locked, WorkspaceState::Locked),
            (WorkspaceState::Corrupted, WorkspaceState::Active),
            (WorkspaceState::Corrupted, WorkspaceState::Locked),
            (WorkspaceState::Corrupted, WorkspaceState::Initializing),
            (WorkspaceState::Deleted, WorkspaceState::Active),
            (WorkspaceState::Deleted, WorkspaceState::Initializing),
            (WorkspaceState::Deleted, WorkspaceState::Locked),
            (WorkspaceState::Deleted, WorkspaceState::Corrupted),
        ];
        for (from, to) in invalid {
            assert!(
                WorkspaceStateMachine::validate_transition(from, to).is_err(),
                "expected {from:?} -> {to:?} to be invalid"
            );
        }
    }

    #[test]
    fn state_machine_is_terminal_consistent_with_state_enum() {
        // Corrupted and Deleted are terminal; others are not
        let terminal = [WorkspaceState::Corrupted, WorkspaceState::Deleted];
        let non_terminal = [
            WorkspaceState::Initializing,
            WorkspaceState::Active,
            WorkspaceState::Locked,
        ];
        for state in &terminal {
            assert!(WorkspaceStateMachine::is_terminal(*state));
            assert!(!WorkspaceStateMachine::is_deletable(*state));
        }
        for state in &non_terminal {
            assert!(!WorkspaceStateMachine::is_terminal(*state));
            assert!(WorkspaceStateMachine::is_deletable(*state));
        }
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;
        use proptest::{prop_assert, prop_assert_eq};

        proptest! {
            #[test]
            fn state_machine_can_transition_matches_validate(from_idx in 0usize..5, to_idx in 0usize..5) {
                let all_states = [
                    WorkspaceState::Initializing,
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    WorkspaceState::Corrupted,
                    WorkspaceState::Deleted,
                ];
                let from = all_states[from_idx];
                let to = all_states[to_idx];
                let can = WorkspaceStateMachine::can_transition(from, to);
                let result = WorkspaceStateMachine::validate_transition(from, to);
                prop_assert_eq!(can, result.is_ok());
            }

            #[test]
            fn state_machine_terminal_states_are_never_lockable(idx in 0usize..5) {
                let all_states = [
                    WorkspaceState::Initializing,
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    WorkspaceState::Corrupted,
                    WorkspaceState::Deleted,
                ];
                let state = all_states[idx];
                if WorkspaceStateMachine::is_terminal(state) {
                    prop_assert!(!WorkspaceStateMachine::is_lockable(state));
                }
            }

            #[test]
            fn state_machine_terminal_states_are_never_deletable(idx in 0usize..5) {
                let all_states = [
                    WorkspaceState::Initializing,
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    WorkspaceState::Corrupted,
                    WorkspaceState::Deleted,
                ];
                let state = all_states[idx];
                // is_terminal and is_deletable are mutually exclusive
                prop_assert_eq!(
                    WorkspaceStateMachine::is_terminal(state),
                    !WorkspaceStateMachine::is_deletable(state)
                );
            }

            #[test]
            fn state_machine_any_state_can_transition_to_deleted(idx in 0usize..5) {
                let all_states = [
                    WorkspaceState::Initializing,
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    WorkspaceState::Corrupted,
                    WorkspaceState::Deleted,
                ];
                let state = all_states[idx];
                prop_assert!(WorkspaceStateMachine::can_transition(state, WorkspaceState::Deleted));
            }

            #[test]
            fn state_machine_deleted_only_transitions_to_deleted(to_idx in 0usize..5) {
                let all_states = [
                    WorkspaceState::Initializing,
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    WorkspaceState::Corrupted,
                    WorkspaceState::Deleted,
                ];
                let to = all_states[to_idx];
                prop_assert_eq!(
                    WorkspaceStateMachine::can_transition(WorkspaceState::Deleted, to),
                    to == WorkspaceState::Deleted
                );
            }

            #[test]
            fn state_machine_locked_cannot_transition_to_itself(
                to_idx in 0usize..5
            ) {
                let all_states = [
                    WorkspaceState::Initializing,
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    WorkspaceState::Corrupted,
                    WorkspaceState::Deleted,
                ];
                let to = all_states[to_idx];
                let can = WorkspaceStateMachine::can_transition(WorkspaceState::Locked, to);
                if to == WorkspaceState::Locked {
                    prop_assert!(!can);
                }
            }
        }
    }
}
