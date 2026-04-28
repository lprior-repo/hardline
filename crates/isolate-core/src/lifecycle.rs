//! Shared lifecycle state machine contract and conformance tests
//!
//! This module defines the `LifecycleState` trait which all state machine
//! enums must implement to ensure consistent behavior across modules.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

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
