use crate::{domain::entities::WorkspaceState, error::WorkspaceError};

pub struct WorkspaceStateMachine;

impl WorkspaceStateMachine {
    pub const fn can_transition(from: WorkspaceState, to: WorkspaceState) -> bool {
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

    pub const fn is_terminal(state: WorkspaceState) -> bool {
        matches!(state, WorkspaceState::Deleted | WorkspaceState::Corrupted)
    }

    pub fn is_lockable(state: WorkspaceState) -> bool {
        state == WorkspaceState::Active
    }

    pub const fn is_deletable(state: WorkspaceState) -> bool {
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

    // --- Table-driven tests for all valid and invalid transitions ---

    /// Struct representing a transition test case: given a from-state, apply a transition
    /// and verify the to-state (or expect failure).
    struct TransitionCase {
        label: &'static str,
        from: WorkspaceState,
        to: WorkspaceState,
        expect_valid: bool,
    }

    /// Exhaustive table of ALL state machine transitions.
    /// Every (from, to) pair from the 5-state enum is covered.
    fn all_transition_cases() -> Vec<TransitionCase> {
        let states = [
            WorkspaceState::Initializing,
            WorkspaceState::Active,
            WorkspaceState::Locked,
            WorkspaceState::Corrupted,
            WorkspaceState::Deleted,
        ];
        // Valid transitions from the can_transition matches:
        // Init→Active, Active→Locked, Locked→Active, Active→Corrupted,
        // Locked→Corrupted, _→Deleted
        let valid_set: Vec<(WorkspaceState, WorkspaceState)> = vec![
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
        let mut cases = Vec::new();
        for &from in &states {
            for &to in &states {
                let expect_valid = valid_set.iter().any(|&(v, t)| v == from && t == to);
                cases.push(TransitionCase {
                    label: Box::leak(format!("{from:?}→{to:?}").into_boxed_str()),
                    from,
                    to,
                    expect_valid,
                });
            }
        }
        cases
    }

    #[test]
    fn table_driven_all_transitions_can_transition() {
        for case in all_transition_cases() {
            let result = WorkspaceStateMachine::can_transition(case.from, case.to);
            assert_eq!(
                result, case.expect_valid,
                "can_transition({}): expected {}, got {}",
                case.label, case.expect_valid, result
            );
        }
    }

    #[test]
    fn table_driven_all_transitions_validate_transition() {
        for case in all_transition_cases() {
            let result = WorkspaceStateMachine::validate_transition(case.from, case.to);
            if case.expect_valid {
                assert!(
                    result.is_ok(),
                    "validate_transition({}): expected Ok, got {:?}",
                    case.label,
                    result
                );
            } else {
                assert!(
                    result.is_err(),
                    "validate_transition({}): expected Err, got {:?}",
                    case.label,
                    result
                );
                // Verify error contains from/to state names
                let err = result.err().unwrap();
                let msg = format!("{err}");
                assert!(
                    msg.contains(&format!("{:?}", case.from)),
                    "error message should contain from state: {msg}"
                );
                assert!(
                    msg.contains(&format!("{:?}", case.to)),
                    "error message should contain to state: {msg}"
                );
            }
        }
    }

    #[test]
    fn table_driven_valid_transitions_return_ok_and_update_state() {
        use crate::{domain::entities::workspace::Workspace, WorkspaceName, WorkspacePath};

        // Table: (label, setup_fn that produces a workspace at the 'from' state, expected_to)
        // We test the actual entity transitions, not just the boolean state machine.
        let cases: Vec<(&str, WorkspaceState, WorkspaceState)> = vec![
            (
                "Init→Active",
                WorkspaceState::Initializing,
                WorkspaceState::Active,
            ),
            (
                "Active→Locked",
                WorkspaceState::Active,
                WorkspaceState::Locked,
            ),
            (
                "Locked→Active",
                WorkspaceState::Locked,
                WorkspaceState::Active,
            ),
            (
                "Active→Deleted",
                WorkspaceState::Active,
                WorkspaceState::Deleted,
            ),
            (
                "Active→Corrupted",
                WorkspaceState::Active,
                WorkspaceState::Corrupted,
            ),
            (
                "Locked→Corrupted",
                WorkspaceState::Locked,
                WorkspaceState::Corrupted,
            ),
            (
                "Locked→Deleted",
                WorkspaceState::Locked,
                WorkspaceState::Deleted,
            ),
            (
                "Corrupted→Deleted",
                WorkspaceState::Corrupted,
                WorkspaceState::Deleted,
            ),
        ];

        for (label, from, expected_to) in cases {
            // Verify the state machine agrees this is valid
            assert!(
                WorkspaceStateMachine::can_transition(from, expected_to),
                "{label}: state machine should allow {from:?}→{expected_to:?}"
            );

            // Build a workspace at the 'from' state and perform the transition
            match (from, expected_to) {
                (WorkspaceState::Initializing, WorkspaceState::Active) => {
                    let ws = Workspace::create(
                        WorkspaceName::new("t".into()).unwrap(),
                        WorkspacePath::new("/t".into()).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(ws.state, WorkspaceState::Initializing, "{label}: before");
                    let active = ws.activate().unwrap();
                    assert_eq!(active.state, WorkspaceState::Active, "{label}: after");
                }
                (WorkspaceState::Active, WorkspaceState::Locked) => {
                    let ws = Workspace::create(
                        WorkspaceName::new("t".into()).unwrap(),
                        WorkspacePath::new("/t".into()).unwrap(),
                    )
                    .unwrap()
                    .activate()
                    .unwrap();
                    assert_eq!(ws.state, WorkspaceState::Active, "{label}: before");
                    let locked = ws.lock("test-agent".into()).unwrap();
                    assert_eq!(locked.state, WorkspaceState::Locked, "{label}: after");
                    assert_eq!(locked.lock_holder(), Some("test-agent"));
                }
                (WorkspaceState::Locked, WorkspaceState::Active) => {
                    let ws = Workspace::create(
                        WorkspaceName::new("t".into()).unwrap(),
                        WorkspacePath::new("/t".into()).unwrap(),
                    )
                    .unwrap()
                    .activate()
                    .unwrap()
                    .lock("test-agent".into())
                    .unwrap();
                    assert_eq!(ws.state, WorkspaceState::Locked, "{label}: before");
                    let active = ws.unlock().unwrap();
                    assert_eq!(active.state, WorkspaceState::Active, "{label}: after");
                    assert!(active.lock_holder().is_none());
                }
                (WorkspaceState::Active, WorkspaceState::Deleted) => {
                    let ws = Workspace::create(
                        WorkspaceName::new("t".into()).unwrap(),
                        WorkspacePath::new("/t".into()).unwrap(),
                    )
                    .unwrap()
                    .activate()
                    .unwrap();
                    assert_eq!(ws.state, WorkspaceState::Active, "{label}: before");
                    let deleted = ws.delete().unwrap();
                    assert_eq!(deleted.state, WorkspaceState::Deleted, "{label}: after");
                    assert!(deleted.is_terminal());
                }
                (WorkspaceState::Active, WorkspaceState::Corrupted) => {
                    let ws = Workspace::create(
                        WorkspaceName::new("t".into()).unwrap(),
                        WorkspacePath::new("/t".into()).unwrap(),
                    )
                    .unwrap()
                    .activate()
                    .unwrap();
                    assert_eq!(ws.state, WorkspaceState::Active, "{label}: before");
                    let corrupted = ws.mark_corrupted().unwrap();
                    assert_eq!(corrupted.state, WorkspaceState::Corrupted, "{label}: after");
                    assert!(corrupted.is_terminal());
                    assert!(corrupted.lock_holder().is_none());
                }
                (WorkspaceState::Locked, WorkspaceState::Corrupted) => {
                    let ws = Workspace::create(
                        WorkspaceName::new("t".into()).unwrap(),
                        WorkspacePath::new("/t".into()).unwrap(),
                    )
                    .unwrap()
                    .activate()
                    .unwrap()
                    .lock("test-agent".into())
                    .unwrap();
                    assert_eq!(ws.state, WorkspaceState::Locked, "{label}: before");
                    let corrupted = ws.mark_corrupted().unwrap();
                    assert_eq!(corrupted.state, WorkspaceState::Corrupted, "{label}: after");
                    assert!(corrupted.is_terminal());
                }
                (WorkspaceState::Locked, WorkspaceState::Deleted) => {
                    let ws = Workspace::create(
                        WorkspaceName::new("t".into()).unwrap(),
                        WorkspacePath::new("/t".into()).unwrap(),
                    )
                    .unwrap()
                    .activate()
                    .unwrap()
                    .lock("test-agent".into())
                    .unwrap();
                    assert_eq!(ws.state, WorkspaceState::Locked, "{label}: before");
                    let deleted = ws.delete().unwrap();
                    assert_eq!(deleted.state, WorkspaceState::Deleted, "{label}: after");
                    assert!(deleted.is_terminal());
                }
                (WorkspaceState::Corrupted, WorkspaceState::Deleted) => {
                    let ws = Workspace::create(
                        WorkspaceName::new("t".into()).unwrap(),
                        WorkspacePath::new("/t".into()).unwrap(),
                    )
                    .unwrap()
                    .activate()
                    .unwrap()
                    .mark_corrupted()
                    .unwrap();
                    assert_eq!(ws.state, WorkspaceState::Corrupted, "{label}: before");
                    let deleted = ws.delete().unwrap();
                    assert_eq!(deleted.state, WorkspaceState::Deleted, "{label}: after");
                }
                _ => panic!("unhandled case: {label}"),
            }
        }
    }

    #[test]
    fn table_driven_recover_locked_to_active_via_service() {
        use crate::{
            application::workspace_service::WorkspaceService, WorkspaceName, WorkspacePath,
        };

        // The "recover" operation in the domain: Locked→Active via WorkspaceService
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("recover-test".into()).unwrap(),
            WorkspacePath::new("/tmp/recover-test".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "stuck-agent".into()).unwrap();
        assert_eq!(
            locked.state,
            WorkspaceState::Locked,
            "before recover: Locked"
        );
        assert_eq!(locked.lock_holder(), Some("stuck-agent"));

        let recovered = WorkspaceService::recover_workspace(locked).unwrap();
        assert_eq!(
            recovered.state,
            WorkspaceState::Active,
            "after recover: Active"
        );
        assert!(recovered.lock_holder().is_none(), "lock holder cleared");
        assert!(recovered.is_active());
    }

    #[test]
    fn table_driven_no_panic_on_invalid_transitions() {
        // Verify no panic occurs when attempting invalid transitions via validate_transition
        for case in all_transition_cases() {
            if !case.expect_valid {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = WorkspaceStateMachine::validate_transition(case.from, case.to);
                }));
                assert!(
                    result.is_ok(),
                    "PANIC on invalid transition {}: should return Err, not panic",
                    case.label
                );
            }
        }
    }

    #[test]
    fn table_driven_terminal_state_properties() {
        // Verify terminal state invariants hold for all states
        let cases = vec![
            (WorkspaceState::Initializing, false, true, false),
            (WorkspaceState::Active, false, true, true),
            (WorkspaceState::Locked, false, true, false),
            (WorkspaceState::Corrupted, true, false, false),
            (WorkspaceState::Deleted, true, false, false),
        ];
        for (state, expect_terminal, expect_deletable, expect_lockable) in cases {
            assert_eq!(
                WorkspaceStateMachine::is_terminal(state),
                expect_terminal,
                "is_terminal({state:?})"
            );
            assert_eq!(
                WorkspaceStateMachine::is_deletable(state),
                expect_deletable,
                "is_deletable({state:?})"
            );
            assert_eq!(
                WorkspaceStateMachine::is_lockable(state),
                expect_lockable,
                "is_lockable({state:?})"
            );
        }
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use proptest::{prelude::*, prop_assert, prop_assert_eq};

        use super::*;

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
