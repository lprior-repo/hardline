//! Shared lifecycle state machine contract and conformance tests
//!
//! This module defines the `LifecycleState` trait which all state machine
//! enums must implement to ensure consistent behavior across modules.

use crate::error::Result;

/// Shared contract for all lifecycle state machines
///
/// This trait ensures consistent behavior across different state enums:
/// - `SessionStatus`
/// - `SessionState`
/// - `WorkspaceState`
///
/// # Contract Requirements
///
/// 1. **Transition Consistency**: `can_transition_to(next)` must return true if and only if `next`
///    is in `valid_next_states()`
///
/// 2. **Terminal States**: If `is_terminal()` returns true, `valid_next_states()` must return an
///    empty vec
///
/// 3. **Non-Terminal States**: If `is_terminal()` returns false, `valid_next_states()` must return
///    at least one state
///
/// 4. **Exhaustive Matching**: `all_states()` must return all possible enum variants
pub trait LifecycleState: Copy + Eq + Sized + 'static {
    /// Returns true if transition from `self` to `next` is valid
    fn can_transition_to(self, next: Self) -> bool;

    /// Returns all valid next states from current state
    fn valid_next_states(self) -> Vec<Self>;

    /// Returns true if this is a terminal state (no transitions out)
    fn is_terminal(self) -> bool;

    /// Returns all possible states for this state machine
    fn all_states() -> &'static [Self];

    /// Attempt to transition to a new state, returning an error if invalid
    fn try_transition(self, next: Self) -> Result<Self>
    where
        Self: std::fmt::Debug,
    {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(crate::error::Error::invalid_state(format!(
                "Cannot transition from {:?} to {:?}",
                self, next
            )))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SHARED CONFORMANCE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
pub mod conformance_tests {
    use super::*;

    /// Test that can_transition_to matches valid_next_states
    ///
    /// This ensures consistency between the two methods.
    pub fn test_transition_consistency<T: LifecycleState + std::fmt::Debug>() {
        for &from_state in T::all_states() {
            let valid_nexts = from_state.valid_next_states();

            for &to_state in T::all_states() {
                let can_transition = from_state.can_transition_to(to_state);
                let in_valid_list = valid_nexts.contains(&to_state);

                assert_eq!(
                    can_transition, in_valid_list,
                    "Inconsistency for {:?} -> {:?}: can_transition={}, in_valid_list={}",
                    from_state, to_state, can_transition, in_valid_list
                );
            }
        }
    }

    /// Test that terminal states have no valid next states
    pub fn test_terminal_states_no_transitions<T: LifecycleState + std::fmt::Debug>() {
        for &state in T::all_states() {
            if state.is_terminal() {
                assert!(
                    state.valid_next_states().is_empty(),
                    "Terminal state {:?} must have no valid next states, but got: {:?}",
                    state,
                    state.valid_next_states()
                );
            }
        }
    }

    /// Test that non-terminal states have at least one valid next state
    pub fn test_non_terminal_states_have_transitions<T: LifecycleState + std::fmt::Debug>() {
        for &state in T::all_states() {
            if !state.is_terminal() {
                assert!(
                    !state.valid_next_states().is_empty(),
                    "Non-terminal state {:?} must have at least one valid next state",
                    state
                );
            }
        }
    }

    /// Test that terminal states cannot transition to anything
    pub fn test_terminal_states_reject_all_transitions<T: LifecycleState + std::fmt::Debug>() {
        for &from_state in T::all_states() {
            if from_state.is_terminal() {
                for &to_state in T::all_states() {
                    assert!(
                        !from_state.can_transition_to(to_state),
                        "Terminal state {:?} must not allow transition to {:?}",
                        from_state,
                        to_state
                    );
                }
            }
        }
    }

    /// Test that all_states returns unique states
    pub fn test_all_states_unique<T: LifecycleState + std::fmt::Debug>() {
        let all = T::all_states();
        for (i, &state1) in all.iter().enumerate() {
            for (j, &state2) in all.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        state1, state2,
                        "all_states() contains duplicate at indices {} and {}: {:?}",
                        i, j, state1
                    );
                }
            }
        }
    }

    /// Run all conformance tests for a state type
    pub fn run_all_tests<T: LifecycleState + std::fmt::Debug>() {
        test_transition_consistency::<T>();
        test_terminal_states_no_transitions::<T>();
        test_non_terminal_states_have_transitions::<T>();
        test_terminal_states_reject_all_transitions::<T>();
        test_all_states_unique::<T>();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS FOR THE LIFECYCLE TRAIT AND CONFORMANCE HELPERS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_state::{SessionState, StateTransition};
    use crate::type_session_status::SessionStatus;

    // ═══════════════════════════════════════════════════════════════════════
    // Conformance test helper validation (using SessionStatus which implements LifecycleState)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_run_all_conformance_tests_for_session_status() {
        conformance_tests::run_all_tests::<SessionStatus>();
    }

    #[test]
    fn test_transition_consistency_for_session_status() {
        conformance_tests::test_transition_consistency::<SessionStatus>();
    }

    #[test]
    fn test_terminal_states_no_transitions_for_session_status() {
        conformance_tests::test_terminal_states_no_transitions::<SessionStatus>();
    }

    #[test]
    fn test_non_terminal_states_have_transitions_for_session_status() {
        conformance_tests::test_non_terminal_states_have_transitions::<SessionStatus>();
    }

    #[test]
    fn test_terminal_states_reject_all_transitions_for_session_status() {
        conformance_tests::test_terminal_states_reject_all_transitions::<SessionStatus>();
    }

    #[test]
    fn test_all_states_unique_for_session_status() {
        conformance_tests::test_all_states_unique::<SessionStatus>();
    }

    // ═══════════════════════════════════════════════════════════════════════
    // try_transition tests (using SessionStatus which implements LifecycleState)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_try_transition_valid() {
        let result = SessionStatus::Creating.try_transition(SessionStatus::Active);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SessionStatus::Active);
    }

    #[test]
    fn test_try_transition_invalid() {
        let result = SessionStatus::Creating.try_transition(SessionStatus::Completed);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_transition_from_terminal_fails() {
        let result = SessionStatus::Completed.try_transition(SessionStatus::Active);
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // StateTransition type tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_state_transition_creation() {
        let transition = StateTransition::new(
            SessionState::Created,
            SessionState::Active,
            "activation",
        );
        assert_eq!(transition.from, SessionState::Created);
        assert_eq!(transition.to, SessionState::Active);
        assert_eq!(transition.reason, "activation");
    }

    #[test]
    fn test_state_transition_validate_valid() {
        let transition = StateTransition::new(
            SessionState::Created,
            SessionState::Active,
            "test",
        );
        assert!(transition.validate().is_ok());
    }

    #[test]
    fn test_state_transition_validate_invalid() {
        let transition = StateTransition::new(
            SessionState::Completed,
            SessionState::Active,
            "invalid",
        );
        assert!(transition.validate().is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SessionStatus all variants / Debug
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_session_status_all_variants_exhaustive() {
        let all = SessionStatus::all_states();
        assert!(all.contains(&SessionStatus::Creating));
        assert!(all.contains(&SessionStatus::Active));
        assert!(all.contains(&SessionStatus::Paused));
        assert!(all.contains(&SessionStatus::Completed));
        assert!(all.contains(&SessionStatus::Failed));
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_session_status_debug_format() {
        let states = SessionStatus::all_states();
        for &state in states {
            let debug = format!("{state:?}");
            assert!(!debug.is_empty(), "Debug for {:?} should not be empty", state);
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SessionStatus terminal states
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_session_status_terminal_states() {
        assert!(SessionStatus::Completed.is_terminal());
        assert!(SessionStatus::Failed.is_terminal());
        assert!(!SessionStatus::Creating.is_terminal());
        assert!(!SessionStatus::Active.is_terminal());
        assert!(!SessionStatus::Paused.is_terminal());
    }

    #[test]
    fn test_session_status_valid_transitions() {
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Failed));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Completed));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Completed));

        // Invalid transitions
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Completed));
        assert!(!SessionStatus::Completed.can_transition_to(SessionStatus::Active));
        assert!(!SessionStatus::Failed.can_transition_to(SessionStatus::Active));
    }
}
