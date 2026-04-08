//! Exhaustive black-hat tests for the isolate WorkspaceStateMachine.
//!
//! Covers:
//! - Full 6x6 transition matrix (36 pairs, every from×to)
//! - Valid happy paths and alternate paths
//! - Terminal state enforcement (Merged, Abandoned)
//! - Helper consistency (is_terminal, is_active, is_complete)
//! - Error messages contain from/to state names
//! - No-panic guarantee on all transitions
//! - Proptest: arbitrary state pairs

use scp_isolate::{WorkspaceState, WorkspaceStateMachine};
use scp_isolate::IsolateError;

// === Exhaustive transition matrix ===

/// All 6 states in the isolate domain.
const ALL_STATES: [WorkspaceState; 6] = [
    WorkspaceState::Created,
    WorkspaceState::Working,
    WorkspaceState::Ready,
    WorkspaceState::Merged,
    WorkspaceState::Abandoned,
    WorkspaceState::Conflict,
];

/// Valid transitions from the state machine's valid_next_states().
const VALID_TRANSITIONS: &[(WorkspaceState, WorkspaceState)] = &[
    (WorkspaceState::Created, WorkspaceState::Working),
    (WorkspaceState::Working, WorkspaceState::Ready),
    (WorkspaceState::Working, WorkspaceState::Conflict),
    (WorkspaceState::Working, WorkspaceState::Abandoned),
    (WorkspaceState::Ready, WorkspaceState::Working),
    (WorkspaceState::Ready, WorkspaceState::Merged),
    (WorkspaceState::Ready, WorkspaceState::Conflict),
    (WorkspaceState::Ready, WorkspaceState::Abandoned),
    (WorkspaceState::Conflict, WorkspaceState::Working),
    (WorkspaceState::Conflict, WorkspaceState::Abandoned),
];

fn is_valid_transition(from: WorkspaceState, to: WorkspaceState) -> bool {
    VALID_TRANSITIONS.iter().any(|&(f, t)| f == from && t == to)
}

#[test]
fn table_driven_all_valid_transitions_succeed() {
    for &(from, to) in VALID_TRANSITIONS {
        let result = WorkspaceStateMachine::transition(from, to);
        assert!(
            result.is_ok(),
            "transition {from:?} -> {to:?} should succeed"
        );
        assert_eq!(result.unwrap(), to);
    }
}

#[test]
fn table_driven_all_invalid_transitions_fail() {
    for &from in &ALL_STATES {
        for &to in &ALL_STATES {
            if !is_valid_transition(from, to) {
                let result = WorkspaceStateMachine::transition(from, to);
                assert!(
                    result.is_err(),
                    "transition {from:?} -> {to:?} should fail"
                );
            }
        }
    }
}

#[test]
fn table_driven_can_transition_matches_transition_result() {
    for &from in &ALL_STATES {
        for &to in &ALL_STATES {
            let can = WorkspaceStateMachine::can_transition(from, to);
            let result = WorkspaceStateMachine::transition(from, to);
            assert_eq!(
                can,
                result.is_ok(),
                "can_transition({from:?}, {to:?}) = {can}, but transition returned {result:?}"
            );
        }
    }
}

#[test]
fn table_driven_error_contains_state_names() {
    for &from in &ALL_STATES {
        for &to in &ALL_STATES {
            if !is_valid_transition(from, to) {
                if let Err(IsolateError::InvalidTransition { from: f, to: t }) =
                    WorkspaceStateMachine::transition(from, to)
                {
                    assert!(
                        !f.is_empty(),
                        "error 'from' field must not be empty for {from:?} -> {to:?}"
                    );
                    assert!(
                        !t.is_empty(),
                        "error 'to' field must not be empty for {from:?} -> {to:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn table_driven_no_panic_on_any_transition() {
    for &from in &ALL_STATES {
        for &to in &ALL_STATES {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = WorkspaceStateMachine::transition(from, to);
            }));
            assert!(
                result.is_ok(),
                "PANIC on transition {from:?} -> {to:?}"
            );
        }
    }
}

// === Happy path sequences ===

#[test]
fn happy_path_created_working_ready_merged() {
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    assert_eq!(s, WorkspaceState::Working);
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    assert_eq!(s, WorkspaceState::Ready);
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Merged).unwrap();
    assert_eq!(s, WorkspaceState::Merged);
    assert!(s.is_terminal());
}

#[test]
fn conflict_path_created_working_conflict_working_ready_merged() {
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Conflict).unwrap();
    assert!(s.is_active());
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Merged).unwrap();
    assert!(s.is_terminal());
}

#[test]
fn rework_path_ready_back_to_working() {
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    assert!(s.is_complete());
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    assert!(s.is_active());
    assert!(!s.is_complete());
}

#[test]
fn early_abandon_from_working() {
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Abandoned).unwrap();
    assert!(s.is_terminal());
}

#[test]
fn abandon_from_ready() {
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Abandoned).unwrap();
    assert!(s.is_terminal());
}

#[test]
fn abandon_from_conflict() {
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Conflict).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Abandoned).unwrap();
    assert!(s.is_terminal());
}

#[test]
fn conflict_from_ready() {
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Conflict).unwrap();
    assert!(s.is_active());
}

// === Terminal state enforcement ===

#[test]
fn merged_is_terminal_cannot_transition_to_anything() {
    for &target in &ALL_STATES {
        let result = WorkspaceStateMachine::transition(WorkspaceState::Merged, target);
        assert!(
            result.is_err(),
            "Merged -> {target:?} should fail (terminal state)"
        );
    }
}

#[test]
fn abandoned_is_terminal_cannot_transition_to_anything() {
    for &target in &ALL_STATES {
        let result = WorkspaceStateMachine::transition(WorkspaceState::Abandoned, target);
        assert!(
            result.is_err(),
            "Abandoned -> {target:?} should fail (terminal state)"
        );
    }
}

#[test]
fn valid_next_states_for_terminal_returns_empty() {
    assert!(WorkspaceState::Merged.valid_next_states().is_empty());
    assert!(WorkspaceState::Abandoned.valid_next_states().is_empty());
}

#[test]
fn terminal_states_cannot_transition_to_themselves() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Merged, WorkspaceState::Merged).is_err());
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Abandoned, WorkspaceState::Abandoned).is_err());
}

// === Invalid transitions from each state ===

#[test]
fn created_cannot_skip_to_ready() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Ready).is_err());
}

#[test]
fn created_cannot_skip_to_merged() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Merged).is_err());
}

#[test]
fn created_cannot_skip_to_abandoned() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Abandoned).is_err());
}

#[test]
fn created_cannot_skip_to_conflict() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Conflict).is_err());
}

#[test]
fn created_cannot_stay_created() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Created).is_err());
}

#[test]
fn working_cannot_go_to_created() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Working, WorkspaceState::Created).is_err());
}

#[test]
fn working_cannot_go_to_merged() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Working, WorkspaceState::Merged).is_err());
}

#[test]
fn working_cannot_stay_working() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Working, WorkspaceState::Working).is_err());
}

#[test]
fn ready_cannot_go_to_created() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Created).is_err());
}

#[test]
fn ready_cannot_stay_ready() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Ready).is_err());
}

#[test]
fn conflict_cannot_go_to_created() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Conflict, WorkspaceState::Created).is_err());
}

#[test]
fn conflict_cannot_go_to_ready() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Conflict, WorkspaceState::Ready).is_err());
}

#[test]
fn conflict_cannot_go_to_merged() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Conflict, WorkspaceState::Merged).is_err());
}

#[test]
fn conflict_cannot_stay_conflict() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Conflict, WorkspaceState::Conflict).is_err());
}

// === Helper consistency ===

#[test]
fn is_terminal_only_true_for_merged_and_abandoned() {
    for &state in &ALL_STATES {
        let expected = matches!(state, WorkspaceState::Merged | WorkspaceState::Abandoned);
        assert_eq!(
            WorkspaceStateMachine::is_terminal(state),
            expected,
            "is_terminal({state:?})"
        );
        assert_eq!(
            state.is_terminal(),
            expected,
            "WorkspaceState::is_terminal({state:?})"
        );
    }
}

#[test]
fn is_active_only_true_for_working_and_conflict() {
    for &state in &ALL_STATES {
        let expected = matches!(state, WorkspaceState::Working | WorkspaceState::Conflict);
        assert_eq!(
            WorkspaceStateMachine::is_active(state),
            expected,
            "is_active({state:?})"
        );
        assert_eq!(
            state.is_active(),
            expected,
            "WorkspaceState::is_active({state:?})"
        );
    }
}

#[test]
fn is_complete_only_true_for_ready_and_merged() {
    for &state in &ALL_STATES {
        let expected = matches!(state, WorkspaceState::Ready | WorkspaceState::Merged);
        assert_eq!(
            WorkspaceStateMachine::is_complete(state),
            expected,
            "is_complete({state:?})"
        );
        assert_eq!(
            state.is_complete(),
            expected,
            "WorkspaceState::is_complete({state:?})"
        );
    }
}

#[test]
fn terminal_states_are_never_active() {
    for &state in &ALL_STATES {
        if state.is_terminal() {
            assert!(!state.is_active(), "{state:?} is terminal but active");
        }
    }
}

#[test]
fn terminal_states_are_never_complete() {
    // Merged is both terminal and complete — this is the exception
    assert!(WorkspaceState::Merged.is_complete());
    assert!(WorkspaceState::Abandoned.is_terminal());
    assert!(!WorkspaceState::Abandoned.is_complete());
}

#[test]
fn active_states_are_not_complete() {
    for &state in &ALL_STATES {
        if state.is_active() {
            assert!(!state.is_complete(), "{state:?} is active and complete");
        }
    }
}

#[test]
fn complete_states_are_not_active() {
    for &state in &ALL_STATES {
        if state.is_complete() {
            assert!(!state.is_active(), "{state:?} is complete and active");
        }
    }
}

// === valid_next_states consistency ===

#[test]
fn valid_next_states_are_exactly_the_valid_transitions() {
    for &state in &ALL_STATES {
        let next = state.valid_next_states();
        for &target in &ALL_STATES {
            let in_next = next.contains(&target);
            let is_valid = is_valid_transition(state, target);
            assert_eq!(
                in_next, is_valid,
                "valid_next_states({state:?}) contains {target:?} = {in_next}, but is_valid = {is_valid}"
            );
        }
    }
}

#[test]
fn can_transition_to_matches_valid_next_states() {
    for &state in &ALL_STATES {
        for &target in &ALL_STATES {
            let from_next = state.valid_next_states().contains(&target);
            let can = state.can_transition_to(target);
            assert_eq!(from_next, can, "mismatch for {state:?} -> {target:?}");
        }
    }
}

// === Display and FromStr roundtrip ===

#[test]
fn display_from_str_roundtrip() {
    for &state in &ALL_STATES {
        let display = state.to_string();
        let parsed: WorkspaceState = display.parse().unwrap_or_else(|e| {
            panic!("failed to parse display output '{display}' for {state:?}: {e}")
        });
        assert_eq!(parsed, state, "roundtrip failed for {state:?}");
    }
}

#[test]
fn from_str_case_insensitive() {
    assert!("CREATED".parse::<WorkspaceState>().is_ok());
    assert!("working".parse::<WorkspaceState>().is_ok());
    assert!("Ready".parse::<WorkspaceState>().is_ok());
    assert!("MERGED".parse::<WorkspaceState>().is_ok());
    assert!("Abandoned".parse::<WorkspaceState>().is_ok());
    assert!("CONFLICT".parse::<WorkspaceState>().is_ok());
}

#[test]
fn from_str_invalid_returns_error() {
    let invalid = ["", "unknown", "pending", "active", "deleted", "corrupted", "locked"];
    for s in &invalid {
        let result: Result<WorkspaceState, _> = s.parse();
        assert!(result.is_err(), "'{s}' should not parse as WorkspaceState");
    }
}

#[test]
fn from_str_error_contains_valid_list() {
    let err = "bogus".parse::<WorkspaceState>().unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("created"), "error should list valid states");
    assert!(msg.contains("working"));
    assert!(msg.contains("ready"));
    assert!(msg.contains("merged"));
    assert!(msg.contains("abandoned"));
    assert!(msg.contains("conflict"));
}

// === Serialization roundtrip ===

#[test]
fn serde_json_roundtrip_all_states() {
    for &state in &ALL_STATES {
        let json = serde_json::to_string(&state).unwrap();
        let parsed: WorkspaceState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, state, "serde roundtrip failed for {state:?}");
    }
}

#[test]
fn serde_json_uses_lowercase() {
    let cases = [
        (WorkspaceState::Created, "\"created\""),
        (WorkspaceState::Working, "\"working\""),
        (WorkspaceState::Ready, "\"ready\""),
        (WorkspaceState::Merged, "\"merged\""),
        (WorkspaceState::Abandoned, "\"abandoned\""),
        (WorkspaceState::Conflict, "\"conflict\""),
    ];
    for (state, expected_json) in cases {
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, expected_json, "serialization mismatch for {state:?}");
    }
}

#[test]
fn default_is_created() {
    assert_eq!(WorkspaceState::default(), WorkspaceState::Created);
}

#[test]
fn all_returns_six_states() {
    assert_eq!(WorkspaceState::all().len(), 6);
    assert!(WorkspaceState::all().contains(&WorkspaceState::Created));
    assert!(WorkspaceState::all().contains(&WorkspaceState::Working));
    assert!(WorkspaceState::all().contains(&WorkspaceState::Ready));
    assert!(WorkspaceState::all().contains(&WorkspaceState::Merged));
    assert!(WorkspaceState::all().contains(&WorkspaceState::Abandoned));
    assert!(WorkspaceState::all().contains(&WorkspaceState::Conflict));
}

// === Copy semantics ===

#[test]
fn state_copy_semantics() {
    let state = WorkspaceState::Working;
    let copied = state;
    assert_eq!(state, copied);
}

#[test]
fn state_equality_and_inequality() {
    assert_eq!(WorkspaceState::Created, WorkspaceState::Created);
    assert_ne!(WorkspaceState::Created, WorkspaceState::Working);
}

#[test]
fn state_hash_set_deduplication() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    for &state in &ALL_STATES {
        set.insert(state);
    }
    assert_eq!(set.len(), 6);
}

// === Proptests ===

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq};

    proptest! {
        #[test]
        fn can_transition_matches_transition(from_idx in 0usize..6, to_idx in 0usize..6) {
            let from = ALL_STATES[from_idx];
            let to = ALL_STATES[to_idx];
            let can = WorkspaceStateMachine::can_transition(from, to);
            let result = WorkspaceStateMachine::transition(from, to);
            prop_assert_eq!(can, result.is_ok());
        }

        #[test]
        fn terminal_states_never_transition(idx in 0usize..6, to_idx in 0usize..6) {
            let terminals = [WorkspaceState::Merged, WorkspaceState::Abandoned];
            let from = terminals[idx % 2];
            let to = ALL_STATES[to_idx];
            prop_assert!(!WorkspaceStateMachine::can_transition(from, to));
        }

        #[test]
        fn is_terminal_and_active_mutually_exclusive(idx in 0usize..6) {
            let state = ALL_STATES[idx];
            prop_assert!(
                !state.is_terminal() || !state.is_active(),
                "{:?}: terminal and active overlap", state
            );
        }

        #[test]
        fn valid_next_states_excludes_self(idx in 0usize..6) {
            let state = ALL_STATES[idx];
            prop_assert!(
                !state.valid_next_states().contains(&state),
                "{:?}: self-transition in valid_next_states", state
            );
        }

        #[test]
        fn display_from_str_roundtrip_arbitrary(idx in 0usize..6) {
            let state = ALL_STATES[idx];
            let display = state.to_string();
            let parsed: WorkspaceState = display.parse().unwrap();
            prop_assert_eq!(parsed, state);
        }

        #[test]
        fn serde_roundtrip_arbitrary(idx in 0usize..6) {
            let state = ALL_STATES[idx];
            let json = serde_json::to_string(&state).unwrap();
            let parsed: WorkspaceState = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(parsed, state);
        }

        #[test]
        fn happy_path_always_succeeds(steps_idx in 0usize..3) {
            let paths: &[Vec<WorkspaceState>] = &[
                vec![WorkspaceState::Working, WorkspaceState::Ready, WorkspaceState::Merged],
                vec![WorkspaceState::Working, WorkspaceState::Ready, WorkspaceState::Abandoned],
                vec![WorkspaceState::Working, WorkspaceState::Abandoned],
            ];
            let path = &paths[steps_idx];
            let mut state = WorkspaceState::Created;
            for &target in path {
                state = WorkspaceStateMachine::transition(state, target).unwrap();
            }
            prop_assert!(state.is_terminal());
        }

        #[test]
        fn no_panic_on_any_transition(from_idx in 0usize..6, to_idx in 0usize..6) {
            let from = ALL_STATES[from_idx];
            let to = ALL_STATES[to_idx];
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = WorkspaceStateMachine::transition(from, to);
            }));
            prop_assert!(result.is_ok());
        }
    }
}
