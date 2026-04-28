//! Exhaustive state transition matrix tests for all lifecycle state machines
//!
//! This test module provides comprehensive coverage for:
//! - `SessionStatus` (5 states)
//! - `SessionState` (7 states)
//! - `WorkspaceState` (6 states)
//!
//! All tests verify:
//! - Every legal transition is allowed
//! - Every illegal transition is rejected
//! - Terminal state behavior
//! - Edge cases and boundary conditions

// Integration tests have relaxed clippy settings for test infrastructure.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use isolate_core::{
    lifecycle::LifecycleState, session_state::SessionState, types::SessionStatus, WorkspaceState,
};
use itertools::Itertools;

// ═══════════════════════════════════════════════════════════════════════════
// TRANSITION MATRIX GENERATION
// ═══════════════════════════════════════════════════════════════════════════

/// Represents a single state transition test case
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TransitionTestCase<S> {
    from: S,
    to: S,
    expected_allowed: bool,
}

/// Generate all possible transition pairs for a state machine
fn generate_all_transition_pairs<S: LifecycleState + Copy>() -> Vec<TransitionTestCase<S>> {
    S::all_states()
        .iter()
        .cartesian_product(S::all_states().iter())
        .map(|(&from, &to)| {
            let expected_allowed = from.valid_next_states().contains(&to);
            TransitionTestCase {
                from,
                to,
                expected_allowed,
            }
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION STATUS TRANSITION MATRIX TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod session_status_matrix {
    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // EXHAUSTIVE TRANSITION MATRIX TEST
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_all_transition_pairs_exhaustively() {
        let test_cases = generate_all_transition_pairs::<SessionStatus>();

        // There are 5 states, so 5 * 5 = 25 possible transitions
        assert_eq!(test_cases.len(), 25);

        for case in test_cases {
            let actual = case.from.can_transition_to(case.to);
            assert_eq!(
                actual, case.expected_allowed,
                "SessionStatus transition {:?} -> {:?}: expected {}, got {}",
                case.from, case.to, case.expected_allowed, actual
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SPECIFIC ALLOWED TRANSITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_creating_transitions() {
        // Creating -> Active (allowed)
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        // Creating -> Failed (allowed)
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Failed));

        // Creating -> Paused (forbidden)
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));
        // Creating -> Completed (forbidden)
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Completed));
        // Creating -> Creating (self-transition forbidden)
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Creating));
    }

    #[test]
    fn test_active_transitions() {
        // Active -> Paused (allowed)
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        // Active -> Completed (allowed)
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Completed));

        // Active -> Creating (forbidden)
        assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Creating));
        // Active -> Failed (forbidden)
        assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Failed));
        // Active -> Active (self-transition forbidden)
        assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Active));
    }

    #[test]
    fn test_paused_transitions() {
        // Paused -> Active (allowed)
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
        // Paused -> Completed (allowed)
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Completed));

        // Paused -> Creating (forbidden)
        assert!(!SessionStatus::Paused.can_transition_to(SessionStatus::Creating));
        // Paused -> Failed (forbidden)
        assert!(!SessionStatus::Paused.can_transition_to(SessionStatus::Failed));
        // Paused -> Paused (self-transition forbidden)
        assert!(!SessionStatus::Paused.can_transition_to(SessionStatus::Paused));
    }

    #[test]
    fn test_completed_is_terminal() {
        assert!(SessionStatus::Completed.is_terminal());
        assert!(SessionStatus::Completed.valid_next_states().is_empty());

        // Cannot transition to any state
        for &target in SessionStatus::all_states() {
            assert!(
                !SessionStatus::Completed.can_transition_to(target),
                "Completed should not transition to {target:?}"
            );
        }
    }

    #[test]
    fn test_failed_is_terminal() {
        assert!(SessionStatus::Failed.is_terminal());
        assert!(SessionStatus::Failed.valid_next_states().is_empty());

        // Cannot transition to any state
        for &target in SessionStatus::all_states() {
            assert!(
                !SessionStatus::Failed.can_transition_to(target),
                "Failed should not transition to {target:?}"
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALID NEXT STATES COUNTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_valid_next_states_counts() {
        use std::collections::HashMap;

        let expected_counts: HashMap<SessionStatus, usize> = [
            (SessionStatus::Creating, 2),  // Active, Failed
            (SessionStatus::Active, 2),    // Paused, Completed
            (SessionStatus::Paused, 2),    // Active, Completed
            (SessionStatus::Completed, 0), // Terminal
            (SessionStatus::Failed, 0),    // Terminal
        ]
        .into_iter()
        .collect();

        for &state in SessionStatus::all_states() {
            let actual_count = state.valid_next_states().len();
            let expected_count = expected_counts[&state];
            assert_eq!(
                actual_count, expected_count,
                "SessionStatus::{state:?} should have {expected_count} valid next states, got {actual_count}"
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // TRANSITION CONFORMANCE WITH LIFECYCLE TRAIT
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_lifecycle_trait_conformance() {
        // Ensure can_transition_to is consistent with valid_next_states
        for &from in SessionStatus::all_states() {
            let valid_nexts = from.valid_next_states();

            for &to in SessionStatus::all_states() {
                let can_transition = from.can_transition_to(to);
                let in_valid_list = valid_nexts.contains(&to);

                assert_eq!(
                    can_transition, in_valid_list,
                    "SessionStatus: can_transition_to({to:?}) = {can_transition}, but in valid_next_states = {in_valid_list}"
                );
            }
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SELF-TRANSITIONS ARE FORBIDDEN
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_no_self_transitions() {
        for &state in SessionStatus::all_states() {
            assert!(
                !state.can_transition_to(state),
                "SessionStatus::{state:?} should not allow self-transition"
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION STATE TRANSITION MATRIX TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod session_state_matrix {
    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // EXHAUSTIVE TRANSITION MATRIX TEST
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_all_transition_pairs_exhaustively() {
        let test_cases = generate_all_transition_pairs::<SessionState>();

        // There are 7 states, so 7 * 7 = 49 possible transitions
        assert_eq!(test_cases.len(), 49);

        for case in test_cases {
            let actual = case.from.can_transition_to(case.to);
            assert_eq!(
                actual, case.expected_allowed,
                "SessionState transition {:?} -> {:?}: expected {}, got {}",
                case.from, case.to, case.expected_allowed, actual
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SPECIFIC ALLOWED TRANSITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_created_transitions() {
        // Created -> Active (allowed)
        assert!(SessionState::Created.can_transition_to(SessionState::Active));
        // Created -> Failed (allowed)
        assert!(SessionState::Created.can_transition_to(SessionState::Failed));

        // Created -> Syncing (forbidden - must go through Active first)
        assert!(!SessionState::Created.can_transition_to(SessionState::Syncing));
        // Created -> Synced (forbidden)
        assert!(!SessionState::Created.can_transition_to(SessionState::Synced));
        // Created -> Paused (forbidden)
        assert!(!SessionState::Created.can_transition_to(SessionState::Paused));
        // Created -> Completed (forbidden)
        assert!(!SessionState::Created.can_transition_to(SessionState::Completed));
    }

    #[test]
    fn test_active_transitions() {
        // Active -> Syncing (allowed)
        assert!(SessionState::Active.can_transition_to(SessionState::Syncing));
        // Active -> Paused (allowed)
        assert!(SessionState::Active.can_transition_to(SessionState::Paused));
        // Active -> Completed (allowed)
        assert!(SessionState::Active.can_transition_to(SessionState::Completed));

        // Active -> Created (forbidden - no going back)
        assert!(!SessionState::Active.can_transition_to(SessionState::Created));
        // Active -> Synced (forbidden - must go through Syncing first)
        assert!(!SessionState::Active.can_transition_to(SessionState::Synced));
        // Active -> Failed (forbidden)
        assert!(!SessionState::Active.can_transition_to(SessionState::Failed));
    }

    #[test]
    fn test_syncing_transitions() {
        // Syncing -> Synced (allowed)
        assert!(SessionState::Syncing.can_transition_to(SessionState::Synced));
        // Syncing -> Failed (allowed)
        assert!(SessionState::Syncing.can_transition_to(SessionState::Failed));

        // Syncing -> Created (forbidden)
        assert!(!SessionState::Syncing.can_transition_to(SessionState::Created));
        // Syncing -> Active (forbidden - must complete sync first)
        assert!(!SessionState::Syncing.can_transition_to(SessionState::Active));
        // Syncing -> Paused (forbidden)
        assert!(!SessionState::Syncing.can_transition_to(SessionState::Paused));
        // Syncing -> Completed (forbidden)
        assert!(!SessionState::Syncing.can_transition_to(SessionState::Completed));
    }

    #[test]
    fn test_synced_transitions() {
        // Synced -> Active (allowed - can do more work)
        assert!(SessionState::Synced.can_transition_to(SessionState::Active));
        // Synced -> Paused (allowed)
        assert!(SessionState::Synced.can_transition_to(SessionState::Paused));
        // Synced -> Completed (allowed)
        assert!(SessionState::Synced.can_transition_to(SessionState::Completed));

        // Synced -> Created (forbidden)
        assert!(!SessionState::Synced.can_transition_to(SessionState::Created));
        // Synced -> Syncing (forbidden - must go back to Active first)
        assert!(!SessionState::Synced.can_transition_to(SessionState::Syncing));
        // Synced -> Failed (forbidden)
        assert!(!SessionState::Synced.can_transition_to(SessionState::Failed));
    }

    #[test]
    fn test_paused_transitions() {
        // Paused -> Active (allowed)
        assert!(SessionState::Paused.can_transition_to(SessionState::Active));
        // Paused -> Completed (allowed)
        assert!(SessionState::Paused.can_transition_to(SessionState::Completed));

        // Paused -> Created (forbidden)
        assert!(!SessionState::Paused.can_transition_to(SessionState::Created));
        // Paused -> Syncing (forbidden - must resume to Active first)
        assert!(!SessionState::Paused.can_transition_to(SessionState::Syncing));
        // Paused -> Synced (forbidden)
        assert!(!SessionState::Paused.can_transition_to(SessionState::Synced));
        // Paused -> Failed (forbidden)
        assert!(!SessionState::Paused.can_transition_to(SessionState::Failed));
    }

    #[test]
    fn test_completed_transitions() {
        // Note: SessionState.Completed is NOT terminal - it can go back to Created
        // This differs from SessionStatus.Completed which IS terminal

        // Completed -> Created (allowed - for undo/restart)
        assert!(SessionState::Completed.can_transition_to(SessionState::Created));

        // Completed -> everything else (forbidden)
        assert!(!SessionState::Completed.can_transition_to(SessionState::Active));
        assert!(!SessionState::Completed.can_transition_to(SessionState::Syncing));
        assert!(!SessionState::Completed.can_transition_to(SessionState::Synced));
        assert!(!SessionState::Completed.can_transition_to(SessionState::Paused));
        assert!(!SessionState::Completed.can_transition_to(SessionState::Failed));
    }

    #[test]
    fn test_failed_transitions() {
        // Note: SessionState.Failed is NOT terminal - it can go back to Created
        // This differs from SessionStatus.Failed which IS terminal

        // Failed -> Created (allowed - for retry)
        assert!(SessionState::Failed.can_transition_to(SessionState::Created));

        // Failed -> everything else (forbidden)
        assert!(!SessionState::Failed.can_transition_to(SessionState::Active));
        assert!(!SessionState::Failed.can_transition_to(SessionState::Syncing));
        assert!(!SessionState::Failed.can_transition_to(SessionState::Synced));
        assert!(!SessionState::Failed.can_transition_to(SessionState::Paused));
        assert!(!SessionState::Failed.can_transition_to(SessionState::Completed));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // NO TERMINAL STATES PROPERTY
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_no_terminal_states() {
        // SessionState intentionally has no terminal states - all can transition
        for &state in SessionState::all_states() {
            assert!(
                !state.is_terminal(),
                "SessionState::{state:?} should not be terminal"
            );
            assert!(
                !state.valid_next_states().is_empty(),
                "SessionState::{state:?} should have at least one valid transition"
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALID NEXT STATES COUNTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_valid_next_states_counts() {
        use std::collections::HashMap;

        let expected_counts: HashMap<SessionState, usize> = [
            (SessionState::Created, 2),   // Active, Failed
            (SessionState::Active, 3),    // Syncing, Paused, Completed
            (SessionState::Syncing, 2),   // Synced, Failed
            (SessionState::Synced, 3),    // Active, Paused, Completed
            (SessionState::Paused, 2),    // Active, Completed
            (SessionState::Completed, 1), // Created
            (SessionState::Failed, 1),    // Created
        ]
        .into_iter()
        .collect();

        for &state in SessionState::all_states() {
            let actual_count = state.valid_next_states().len();
            let expected_count = expected_counts[&state];
            assert_eq!(
                actual_count, expected_count,
                "SessionState::{state:?} should have {expected_count} valid next states, got {actual_count}"
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SELF-TRANSITIONS ARE FORBIDDEN
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_no_self_transitions() {
        for &state in SessionState::all_states() {
            assert!(
                !state.can_transition_to(state),
                "SessionState::{state:?} should not allow self-transition"
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// WORKSPACE STATE TRANSITION MATRIX TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod workspace_state_matrix {
    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // EXHAUSTIVE TRANSITION MATRIX TEST
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_all_transition_pairs_exhaustively() {
        let test_cases = generate_all_transition_pairs::<WorkspaceState>();

        // There are 6 states, so 6 * 6 = 36 possible transitions
        assert_eq!(test_cases.len(), 36);

        for case in test_cases {
            let actual = case.from.can_transition_to(case.to);
            assert_eq!(
                actual, case.expected_allowed,
                "WorkspaceState transition {:?} -> {:?}: expected {}, got {}",
                case.from, case.to, case.expected_allowed, actual
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SPECIFIC ALLOWED TRANSITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_created_transitions() {
        // Created -> Working (allowed - only valid transition)
        assert!(WorkspaceState::Created.can_transition_to(WorkspaceState::Working));

        // Created -> everything else (forbidden)
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Ready));
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Merged));
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Abandoned));
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Conflict));
        // Self-transition forbidden
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Created));
    }

    #[test]
    fn test_working_transitions() {
        // Working -> Ready (allowed)
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Ready));
        // Working -> Conflict (allowed)
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Conflict));
        // Working -> Abandoned (allowed)
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Abandoned));

        // Working -> Created (forbidden - no going back)
        assert!(!WorkspaceState::Working.can_transition_to(WorkspaceState::Created));
        // Working -> Merged (forbidden - must go through Ready first)
        assert!(!WorkspaceState::Working.can_transition_to(WorkspaceState::Merged));
        // Self-transition forbidden
        assert!(!WorkspaceState::Working.can_transition_to(WorkspaceState::Working));
    }

    #[test]
    fn test_ready_transitions() {
        // Ready -> Working (allowed - needs more work)
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Working));
        // Ready -> Merged (allowed - successful merge)
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Merged));
        // Ready -> Conflict (allowed - merge conflict on merge attempt)
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Conflict));
        // Ready -> Abandoned (allowed - decided not to merge)
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Abandoned));

        // Ready -> Created (forbidden)
        assert!(!WorkspaceState::Ready.can_transition_to(WorkspaceState::Created));
        // Self-transition forbidden
        assert!(!WorkspaceState::Ready.can_transition_to(WorkspaceState::Ready));
    }

    #[test]
    fn test_conflict_transitions() {
        // Conflict -> Working (allowed - conflict resolved)
        assert!(WorkspaceState::Conflict.can_transition_to(WorkspaceState::Working));
        // Conflict -> Abandoned (allowed - give up on conflict)
        assert!(WorkspaceState::Conflict.can_transition_to(WorkspaceState::Abandoned));

        // Conflict -> Created (forbidden)
        assert!(!WorkspaceState::Conflict.can_transition_to(WorkspaceState::Created));
        // Conflict -> Ready (forbidden - must resolve conflict first)
        assert!(!WorkspaceState::Conflict.can_transition_to(WorkspaceState::Ready));
        // Conflict -> Merged (forbidden - must resolve and go through Ready)
        assert!(!WorkspaceState::Conflict.can_transition_to(WorkspaceState::Merged));
        // Self-transition forbidden
        assert!(!WorkspaceState::Conflict.can_transition_to(WorkspaceState::Conflict));
    }

    #[test]
    fn test_merged_is_terminal() {
        assert!(WorkspaceState::Merged.is_terminal());
        assert!(WorkspaceState::Merged.valid_next_states().is_empty());

        // Cannot transition to any state
        for &target in WorkspaceState::all_states() {
            assert!(
                !WorkspaceState::Merged.can_transition_to(target),
                "Merged should not transition to {target:?}"
            );
        }
    }

    #[test]
    fn test_abandoned_is_terminal() {
        assert!(WorkspaceState::Abandoned.is_terminal());
        assert!(WorkspaceState::Abandoned.valid_next_states().is_empty());

        // Cannot transition to any state
        for &target in WorkspaceState::all_states() {
            assert!(
                !WorkspaceState::Abandoned.can_transition_to(target),
                "Abandoned should not transition to {target:?}"
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALID NEXT STATES COUNTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_valid_next_states_counts() {
        use std::collections::HashMap;

        let expected_counts: HashMap<WorkspaceState, usize> = [
            (WorkspaceState::Created, 1),   // Working
            (WorkspaceState::Working, 3),   // Ready, Conflict, Abandoned
            (WorkspaceState::Ready, 4),     // Working, Merged, Conflict, Abandoned
            (WorkspaceState::Conflict, 2),  // Working, Abandoned
            (WorkspaceState::Merged, 0),    // Terminal
            (WorkspaceState::Abandoned, 0), // Terminal
        ]
        .into_iter()
        .collect();

        for &state in WorkspaceState::all_states() {
            let actual_count = state.valid_next_states().len();
            let expected_count = expected_counts[&state];
            assert_eq!(
                actual_count, expected_count,
                "WorkspaceState::{state:?} should have {expected_count} valid next states, got {actual_count}"
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SELF-TRANSITIONS ARE FORBIDDEN
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_no_self_transitions() {
        for &state in WorkspaceState::all_states() {
            assert!(
                !state.can_transition_to(state),
                "WorkspaceState::{state:?} should not allow self-transition"
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // TERMINAL STATES VERIFICATION
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_only_merged_and_abandoned_are_terminal() {
        for &state in WorkspaceState::all_states() {
            let is_terminal = matches!(state, WorkspaceState::Merged | WorkspaceState::Abandoned);
            assert_eq!(
                state.is_terminal(),
                is_terminal,
                "WorkspaceState::{state:?}.is_terminal() should be {is_terminal}"
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CROSS-STATE-MACHINE INVARIANT TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod cross_machine_invariants {
    use super::*;

    /// Verify that terminal states have empty `valid_next_states` for ALL state machines
    #[test]
    fn test_terminal_states_have_no_valid_transitions() {
        // SessionStatus terminal states
        for &state in SessionStatus::all_states() {
            if state.is_terminal() {
                assert!(
                    state.valid_next_states().is_empty(),
                    "Terminal SessionStatus::{state:?} should have empty valid_next_states"
                );
            }
        }

        // WorkspaceState terminal states
        for &state in WorkspaceState::all_states() {
            if state.is_terminal() {
                assert!(
                    state.valid_next_states().is_empty(),
                    "Terminal WorkspaceState::{state:?} should have empty valid_next_states"
                );
            }
        }
    }

    /// Verify that non-terminal states have at least one valid transition
    #[test]
    fn test_non_terminal_states_have_valid_transitions() {
        // SessionStatus non-terminal states
        for &state in SessionStatus::all_states() {
            if !state.is_terminal() {
                assert!(
                    !state.valid_next_states().is_empty(),
                    "Non-terminal SessionStatus::{state:?} should have valid transitions"
                );
            }
        }

        // SessionState (no terminal states - all should have transitions)
        for &state in SessionState::all_states() {
            assert!(
                !state.valid_next_states().is_empty(),
                "SessionState::{state:?} should have valid transitions"
            );
        }

        // WorkspaceState non-terminal states
        for &state in WorkspaceState::all_states() {
            if !state.is_terminal() {
                assert!(
                    !state.valid_next_states().is_empty(),
                    "Non-terminal WorkspaceState::{state:?} should have valid transitions"
                );
            }
        }
    }

    /// Verify `can_transition_to` is consistent with `valid_next_states` for all machines
    #[test]
    fn test_can_transition_consistency() {
        // SessionStatus
        for &from in SessionStatus::all_states() {
            let valid_nexts = from.valid_next_states();
            for &to in SessionStatus::all_states() {
                assert_eq!(
                    from.can_transition_to(to),
                    valid_nexts.contains(&to),
                    "SessionStatus: can_transition_to inconsistency for {from:?} -> {to:?}"
                );
            }
        }

        // SessionState
        for &from in SessionState::all_states() {
            let valid_nexts = from.valid_next_states();
            for &to in SessionState::all_states() {
                assert_eq!(
                    from.can_transition_to(to),
                    valid_nexts.contains(&to),
                    "SessionState: can_transition_to inconsistency for {from:?} -> {to:?}"
                );
            }
        }

        // WorkspaceState
        for &from in WorkspaceState::all_states() {
            let valid_nexts = from.valid_next_states();
            for &to in WorkspaceState::all_states() {
                assert_eq!(
                    from.can_transition_to(to),
                    valid_nexts.contains(&to),
                    "WorkspaceState: can_transition_to inconsistency for {from:?} -> {to:?}"
                );
            }
        }
    }

    /// Verify `all_states` returns unique values
    #[test]
    fn test_all_states_are_unique() {
        // SessionStatus
        let all: Vec<_> = SessionStatus::all_states().iter().collect();
        let unique: std::collections::HashSet<_> = all.iter().collect();
        assert_eq!(
            all.len(),
            unique.len(),
            "SessionStatus::all_states() contains duplicates"
        );

        // SessionState
        let all: Vec<_> = SessionState::all_states().iter().collect();
        let unique: std::collections::HashSet<_> = all.iter().collect();
        assert_eq!(
            all.len(),
            unique.len(),
            "SessionState::all_states() contains duplicates"
        );

        // WorkspaceState
        let all: Vec<_> = WorkspaceState::all_states().iter().collect();
        let unique: std::collections::HashSet<_> = all.iter().collect();
        assert_eq!(
            all.len(),
            unique.len(),
            "WorkspaceState::all_states() contains duplicates"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// EDGE CASE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod edge_cases {
    use super::*;

    /// Test that state counts are correct
    #[test]
    fn test_state_counts() {
        assert_eq!(SessionStatus::all_states().len(), 5);
        assert_eq!(SessionState::all_states().len(), 7);
        assert_eq!(WorkspaceState::all_states().len(), 6);
    }

    /// Test that the transition matrix is correct for common workflows
    #[test]
    fn test_common_session_status_workflow() {
        // Typical workflow: Creating -> Active -> Paused -> Active -> Completed
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Completed));
    }

    #[test]
    fn test_common_session_state_workflow() {
        // Typical workflow: Created -> Active -> Syncing -> Synced -> Completed
        assert!(SessionState::Created.can_transition_to(SessionState::Active));
        assert!(SessionState::Active.can_transition_to(SessionState::Syncing));
        assert!(SessionState::Syncing.can_transition_to(SessionState::Synced));
        assert!(SessionState::Synced.can_transition_to(SessionState::Completed));
    }

    #[test]
    fn test_common_workspace_state_workflow() {
        // Typical workflow: Created -> Working -> Ready -> Merged
        assert!(WorkspaceState::Created.can_transition_to(WorkspaceState::Working));
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Ready));
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Merged));
    }

    #[test]
    fn test_conflict_recovery_workflow() {
        // Conflict recovery: Working -> Conflict -> Working -> Ready -> Merged
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Conflict));
        assert!(WorkspaceState::Conflict.can_transition_to(WorkspaceState::Working));
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Ready));
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Merged));
    }

    #[test]
    fn test_retry_workflow() {
        // Retry workflow: Created -> Active -> ... -> Failed -> Created
        assert!(SessionState::Created.can_transition_to(SessionState::Failed));
        assert!(SessionState::Failed.can_transition_to(SessionState::Created));

        // Sync failure: Active -> Syncing -> Failed -> Created
        assert!(SessionState::Active.can_transition_to(SessionState::Syncing));
        assert!(SessionState::Syncing.can_transition_to(SessionState::Failed));
    }

    /// Test that there are no "orphan" states (states that no other state can transition to)
    #[test]
    fn test_no_orphan_states() {
        // For SessionStatus, verify all non-Creating states are reachable
        let reachable_from_creating = SessionStatus::Creating.valid_next_states();
        assert!(reachable_from_creating.contains(&SessionStatus::Active));

        let reachable_from_active = SessionStatus::Active.valid_next_states();
        assert!(reachable_from_active.contains(&SessionStatus::Paused));
        assert!(reachable_from_active.contains(&SessionStatus::Completed));

        let reachable_from_paused = SessionStatus::Paused.valid_next_states();
        assert!(reachable_from_paused.contains(&SessionStatus::Active));

        // For WorkspaceState, verify all non-Created states are reachable
        let reachable_from_created = WorkspaceState::Created.valid_next_states();
        assert!(reachable_from_created.contains(&WorkspaceState::Working));

        let reachable_from_working = WorkspaceState::Working.valid_next_states();
        assert!(reachable_from_working.contains(&WorkspaceState::Ready));
        assert!(reachable_from_working.contains(&WorkspaceState::Conflict));

        let reachable_from_ready = WorkspaceState::Ready.valid_next_states();
        assert!(reachable_from_ready.contains(&WorkspaceState::Merged));
    }

    /// Test using itertools to verify transition counts
    #[test]
    fn test_total_transition_counts_with_itertools() {
        // Count total allowed transitions for each state machine
        let session_status_allowed: usize = SessionStatus::all_states()
            .iter()
            .map(|s| s.valid_next_states().len())
            .sum();

        let session_state_allowed: usize = SessionState::all_states()
            .iter()
            .map(|s| s.valid_next_states().len())
            .sum();

        let workspace_state_allowed: usize = WorkspaceState::all_states()
            .iter()
            .map(|s| s.valid_next_states().len())
            .sum();

        // SessionStatus: Creating(2) + Active(2) + Paused(2) + Completed(0) + Failed(0) = 6
        assert_eq!(session_status_allowed, 6);

        // SessionState: Created(2) + Active(3) + Syncing(2) + Synced(3) + Paused(2) + Completed(1)
        // + Failed(1) = 14
        assert_eq!(session_state_allowed, 14);

        // WorkspaceState: Created(1) + Working(3) + Ready(4) + Conflict(2) + Merged(0) +
        // Abandoned(0) = 10
        assert_eq!(workspace_state_allowed, 10);
    }

    /// Test transition symmetry properties
    #[test]
    fn test_no_symmetric_transitions_where_not_expected() {
        // Verify that certain transitions are NOT symmetric

        // SessionStatus: Created -> Active is allowed, but Active -> Created is not
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Creating));

        // WorkspaceState: Created -> Working is allowed, but Working -> Created is not
        assert!(WorkspaceState::Created.can_transition_to(WorkspaceState::Working));
        assert!(!WorkspaceState::Working.can_transition_to(WorkspaceState::Created));

        // SessionState: Created -> Active is allowed, but Active -> Created is not
        assert!(SessionState::Created.can_transition_to(SessionState::Active));
        assert!(!SessionState::Active.can_transition_to(SessionState::Created));
    }

    /// Test bidirectional transitions where they exist
    #[test]
    fn test_bidirectional_transitions() {
        // SessionStatus: Paused <-> Active
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));

        // SessionState: Active <-> Synced (via Syncing)
        // Note: Direct Active <-> Synced is NOT bidirectional
        assert!(!SessionState::Active.can_transition_to(SessionState::Synced));
        assert!(SessionState::Synced.can_transition_to(SessionState::Active));

        // WorkspaceState: Working <-> Ready
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Ready));
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Working));

        // WorkspaceState: Working <-> Conflict
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Conflict));
        assert!(WorkspaceState::Conflict.can_transition_to(WorkspaceState::Working));
    }
}
