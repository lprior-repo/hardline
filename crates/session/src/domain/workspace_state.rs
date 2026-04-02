//! Workspace state machine for session-based workspaces.
//!
//! This module provides the WorkspaceState enum and WorkspaceStateMachine
//! for managing the lifecycle of session workspaces.
//!
//! State lifecycle: Created → Working → Ready → Merged | Conflict | Abandoned

use serde::{Deserialize, Serialize};

/// Workspace state for session-based workspaces.
///
/// Lifecycle: Created → Working → Ready → Merged | Conflict | Abandoned
///
/// - Created: Workspace has been created
/// - Working: Workspace is being actively worked on
/// - Ready: Workspace is ready for review/merge
/// - Merged: Workspace has been merged (terminal state)
/// - Conflict: Workspace has merge conflicts (terminal state)
/// - Abandoned: Workspace was abandoned (terminal state)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WorkspaceState {
    /// Workspace has been created
    #[default]
    Created,
    /// Workspace is being actively worked on
    Working,
    /// Workspace is ready for review/merge
    Ready,
    /// Workspace has been merged (terminal state)
    Merged,
    /// Workspace has merge conflicts (terminal state)
    Conflict,
    /// Workspace was abandoned (terminal state)
    Abandoned,
}

impl WorkspaceState {
    /// All possible workspace states
    pub const fn all() -> [Self; 6] {
        [
            Self::Created,
            Self::Working,
            Self::Ready,
            Self::Merged,
            Self::Conflict,
            Self::Abandoned,
        ]
    }

    /// Check if this state is terminal (no further transitions possible)
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Merged | Self::Conflict | Self::Abandoned)
    }

    /// Check if this state is ready (ready for merge/review)
    #[must_use]
    pub fn is_ready(self) -> bool {
        self == Self::Ready
    }

    /// Check if this state is working (active work)
    #[must_use]
    pub fn is_working(self) -> bool {
        self == Self::Working
    }

    /// Check if a transition from this state to target is valid
    /// State machine: Created → Working → Ready → Merged | Conflict | Abandoned
    #[must_use]
    pub const fn can_transition_to(self, target: Self) -> bool {
        match (self, target) {
            // Created → Working: start working on workspace
            (Self::Created, Self::Working) => true,
            // Working → Ready: mark as ready for review
            (Self::Working, Self::Ready) => true,
            // Working → Abandoned: abandon during work
            (Self::Working, Self::Abandoned) => true,
            // Ready → Merged: merge successful
            (Self::Ready, Self::Merged) => true,
            // Ready → Conflict: merge conflicts detected
            (Self::Ready, Self::Conflict) => true,
            // Ready → Abandoned: abandon after conflicts
            (Self::Ready, Self::Abandoned) => true,
            // Self-loops are not allowed
            _ => false,
        }
    }

    /// Get all valid target states from this state
    #[must_use]
    pub fn valid_transitions(self) -> Vec<Self> {
        Self::all()
            .into_iter()
            .filter(|&target| self.can_transition_to(target))
            .collect()
    }
}

impl std::fmt::Display for WorkspaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Working => write!(f, "working"),
            Self::Ready => write!(f, "ready"),
            Self::Merged => write!(f, "merged"),
            Self::Conflict => write!(f, "conflict"),
            Self::Abandoned => write!(f, "abandoned"),
        }
    }
}

/// Workspace state machine for managing state transitions
pub struct WorkspaceStateMachine;

impl WorkspaceStateMachine {
    /// Attempt to transition from one state to another
    pub fn transition(
        from: WorkspaceState,
        to: WorkspaceState,
    ) -> Result<WorkspaceState, crate::error::SessionError> {
        if from.can_transition_to(to) {
            Ok(to)
        } else {
            Err(crate::error::SessionError::InvalidTransition { from, to })
        }
    }

    /// Check if a transition is valid without performing it
    #[must_use]
    pub fn can_transition(from: WorkspaceState, to: WorkspaceState) -> bool {
        from.can_transition_to(to)
    }

    /// Check if a state is terminal
    #[must_use]
    pub fn is_terminal(state: WorkspaceState) -> bool {
        state.is_terminal()
    }

    /// Check if a state is working
    #[must_use]
    pub fn is_working(state: WorkspaceState) -> bool {
        state.is_working()
    }

    /// Check if a state is ready
    #[must_use]
    pub fn is_ready(state: WorkspaceState) -> bool {
        state.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_state_created_to_working_transition_succeeds() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Working);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), WorkspaceState::Working);
    }

    #[test]
    fn workspace_state_working_to_ready_transition_succeeds() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Working, WorkspaceState::Ready);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), WorkspaceState::Ready);
    }

    #[test]
    fn workspace_state_ready_to_merged_transition_succeeds() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Merged);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), WorkspaceState::Merged);
    }

    #[test]
    fn workspace_state_ready_to_conflict_transition_succeeds() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Conflict);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), WorkspaceState::Conflict);
    }

    #[test]
    fn workspace_state_working_to_abandoned_transition_succeeds() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Working, WorkspaceState::Abandoned);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), WorkspaceState::Abandoned);
    }

    #[test]
    fn workspace_state_invalid_created_to_ready_fails() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Ready);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_state_invalid_merged_to_working_fails() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Merged, WorkspaceState::Working);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_state_terminal_states_cannot_transition() {
        for terminal in [
            WorkspaceState::Merged,
            WorkspaceState::Conflict,
            WorkspaceState::Abandoned,
        ] {
            let result = WorkspaceStateMachine::transition(terminal, WorkspaceState::Working);
            assert!(
                result.is_err(),
                "Terminal state {:?} should not transition",
                terminal
            );
        }
    }

    #[test]
    fn workspace_state_can_transition_returns_correct_values() {
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Created,
            WorkspaceState::Working
        ));
        assert!(!WorkspaceStateMachine::can_transition(
            WorkspaceState::Created,
            WorkspaceState::Ready
        ));
    }

    #[test]
    fn workspace_state_is_terminal_identifies_terminal_states() {
        assert!(WorkspaceStateMachine::is_terminal(WorkspaceState::Merged));
        assert!(WorkspaceStateMachine::is_terminal(WorkspaceState::Conflict));
        assert!(WorkspaceStateMachine::is_terminal(
            WorkspaceState::Abandoned
        ));
        assert!(!WorkspaceStateMachine::is_terminal(WorkspaceState::Created));
        assert!(!WorkspaceStateMachine::is_terminal(WorkspaceState::Working));
        assert!(!WorkspaceStateMachine::is_terminal(WorkspaceState::Ready));
    }

    #[test]
    fn workspace_state_is_ready_identifies_ready_state() {
        assert!(WorkspaceStateMachine::is_ready(WorkspaceState::Ready));
        assert!(!WorkspaceStateMachine::is_ready(WorkspaceState::Created));
        assert!(!WorkspaceStateMachine::is_ready(WorkspaceState::Working));
    }

    #[test]
    fn workspace_state_is_working_identifies_working_state() {
        assert!(WorkspaceStateMachine::is_working(WorkspaceState::Working));
        assert!(!WorkspaceStateMachine::is_working(WorkspaceState::Created));
        assert!(!WorkspaceStateMachine::is_working(WorkspaceState::Ready));
    }

    #[test]
    fn workspace_state_valid_transitions_lists_correct_targets() {
        let created_targets = WorkspaceState::Created.valid_transitions();
        assert_eq!(created_targets, vec![WorkspaceState::Working]);

        let working_targets = WorkspaceState::Working.valid_transitions();
        assert_eq!(
            working_targets,
            vec![WorkspaceState::Ready, WorkspaceState::Abandoned]
        );

        let ready_targets = WorkspaceState::Ready.valid_transitions();
        assert_eq!(
            ready_targets,
            vec![
                WorkspaceState::Merged,
                WorkspaceState::Conflict,
                WorkspaceState::Abandoned
            ]
        );
    }
}

#[cfg(test)]
mod workspace_state_machine_tests {
    use super::{WorkspaceState, WorkspaceStateMachine};

    #[test]
    fn test_workspace_state_sm_happy_path() {
        let state = WorkspaceState::Created;
        let result = WorkspaceStateMachine::transition(state, WorkspaceState::Working);
        assert!(result.is_ok());
        let state = result.unwrap();
        let result = WorkspaceStateMachine::transition(state, WorkspaceState::Ready);
        assert!(result.is_ok());
        let state = result.unwrap();
        let result = WorkspaceStateMachine::transition(state, WorkspaceState::Merged);
        assert!(result.is_ok());
        assert!(result.unwrap().is_terminal());
    }

    #[test]
    fn test_workspace_state_sm_conflict_path() {
        let state = WorkspaceState::Created;
        let result = WorkspaceStateMachine::transition(state, WorkspaceState::Working);
        assert!(result.is_ok());
        let state = result.unwrap();
        let result = WorkspaceStateMachine::transition(state, WorkspaceState::Ready);
        assert!(result.is_ok());
        let state = result.unwrap();
        let result = WorkspaceStateMachine::transition(state, WorkspaceState::Conflict);
        assert!(result.is_ok());
        assert!(result.unwrap().is_terminal());
    }

    #[test]
    fn test_workspace_state_sm_abandon_early_path() {
        let state = WorkspaceState::Created;
        let result = WorkspaceStateMachine::transition(state, WorkspaceState::Working);
        assert!(result.is_ok());
        let state = result.unwrap();
        let result = WorkspaceStateMachine::transition(state, WorkspaceState::Abandoned);
        assert!(result.is_ok());
        assert!(result.unwrap().is_terminal());
    }

    // =========================================================================
    // WorkspaceState Display Tests
    // =========================================================================

    mod workspace_state_display_tests {
        use super::*;

        #[test]
        fn workspace_state_display_created() {
            assert_eq!(format!("{}", WorkspaceState::Created), "created");
        }

        #[test]
        fn workspace_state_display_working() {
            assert_eq!(format!("{}", WorkspaceState::Working), "working");
        }

        #[test]
        fn workspace_state_display_ready() {
            assert_eq!(format!("{}", WorkspaceState::Ready), "ready");
        }

        #[test]
        fn workspace_state_display_merged() {
            assert_eq!(format!("{}", WorkspaceState::Merged), "merged");
        }

        #[test]
        fn workspace_state_display_conflict() {
            assert_eq!(format!("{}", WorkspaceState::Conflict), "conflict");
        }

        #[test]
        fn workspace_state_display_abandoned() {
            assert_eq!(format!("{}", WorkspaceState::Abandoned), "abandoned");
        }
    }

    // =========================================================================
    // WorkspaceState Serialization Tests
    // =========================================================================

    mod workspace_state_serde_tests {
        use super::*;

        #[test]
        fn workspace_state_serde_roundtrip_all_variants() {
            for state in WorkspaceState::all() {
                let json = serde_json::to_string(&state).expect("serialize");
                let parsed: WorkspaceState = serde_json::from_str(&json).expect("deserialize");
                assert_eq!(state, parsed, "Roundtrip failed for {:?}", state);
            }
        }
    }

    // =========================================================================
    // WorkspaceState Additional Transition Tests
    // =========================================================================

    mod workspace_state_extended_tests {
        use super::*;

        #[test]
        fn workspace_state_default_is_created() {
            assert_eq!(WorkspaceState::default(), WorkspaceState::Created);
        }

        #[test]
        fn workspace_state_all_returns_six_variants() {
            assert_eq!(WorkspaceState::all().len(), 6);
        }

        #[test]
        fn workspace_state_ready_to_abandoned_is_valid() {
            assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Abandoned));
        }

        #[test]
        fn workspace_state_created_to_created_is_invalid() {
            assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Created));
        }

        #[test]
        fn workspace_state_working_to_working_is_invalid() {
            assert!(!WorkspaceState::Working.can_transition_to(WorkspaceState::Working));
        }

        #[test]
        fn workspace_state_ready_to_ready_is_invalid() {
            assert!(!WorkspaceState::Ready.can_transition_to(WorkspaceState::Ready));
        }

        #[test]
        fn workspace_state_created_to_merged_is_invalid() {
            assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Merged));
        }

        #[test]
        fn workspace_state_valid_transitions_from_created() {
            let transitions = WorkspaceState::Created.valid_transitions();
            assert_eq!(transitions, vec![WorkspaceState::Working]);
        }

        #[test]
        fn workspace_state_valid_transitions_from_merged_is_empty() {
            let transitions = WorkspaceState::Merged.valid_transitions();
            assert!(transitions.is_empty());
        }

        #[test]
        fn workspace_state_valid_transitions_from_conflict_is_empty() {
            let transitions = WorkspaceState::Conflict.valid_transitions();
            assert!(transitions.is_empty());
        }

        #[test]
        fn workspace_state_valid_transitions_from_abandoned_is_empty() {
            let transitions = WorkspaceState::Abandoned.valid_transitions();
            assert!(transitions.is_empty());
        }

        // =========================================================================
        // WorkspaceState Proptests
        // =========================================================================

        mod workspace_state_proptests {
            use super::*;
            use proptest::proptest;
            use proptest::{prop_assert, prop_assert_eq};

            proptest! {
                /// No self-transitions are allowed
                #[test]
                fn prop_no_self_transitions(state_idx in 0u8..6u8) {
                    let states = WorkspaceState::all();
                    let state = states[state_idx as usize];
                    prop_assert!(!state.can_transition_to(state));
                }

                /// Terminal states have empty valid_transitions
                #[test]
                fn prop_terminal_empty_transitions(state_idx in 0u8..6u8) {
                    let states = WorkspaceState::all();
                    let state = states[state_idx as usize];
                    if state.is_terminal() {
                        prop_assert!(state.valid_transitions().is_empty());
                    } else {
                        prop_assert!(!state.valid_transitions().is_empty());
                    }
                }

                /// can_transition_to matches valid_transitions containment
                #[test]
                fn prop_can_transition_matches_valid_transitions(
                    from_idx in 0u8..6u8,
                    to_idx in 0u8..6u8
                ) {
                    let states = WorkspaceState::all();
                    let from = states[from_idx as usize];
                    let to = states[to_idx as usize];
                    let can = from.can_transition_to(to);
                    let in_valid = from.valid_transitions().contains(&to);
                    prop_assert_eq!(can, in_valid);
                }

                /// Display is always lowercase ascii
                #[test]
                fn prop_display_is_lowercase_ascii(state_idx in 0u8..6u8) {
                    let states = WorkspaceState::all();
                    let state = states[state_idx as usize];
                    let display = format!("{state}");
                    prop_assert!(!display.is_empty());
                    prop_assert!(display.chars().all(|c| c.is_ascii_lowercase()));
                }

                /// Serde roundtrip preserves equality
                #[test]
                fn prop_serde_roundtrip(state_idx in 0u8..6u8) {
                    let states = WorkspaceState::all();
                    let state = states[state_idx as usize];
                    let json = serde_json::to_string(&state).unwrap();
                    let parsed: WorkspaceState = serde_json::from_str(&json).unwrap();
                    prop_assert_eq!(state, parsed);
                }

                /// all() has exactly 6 unique variants
                #[test]
                fn prop_all_has_six_unique_variants(_ in 0u8..1u8) {
                    let all = WorkspaceState::all();
                    prop_assert_eq!(all.len(), 6);
                    let mut seen = std::collections::HashSet::new();
                    for state in all {
                        prop_assert!(seen.insert(state));
                    }
                }

                /// Transition function matches can_transition_to
                #[test]
                fn prop_machine_transition_matches_can_transition(
                    from_idx in 0u8..6u8,
                    to_idx in 0u8..6u8
                ) {
                    let states = WorkspaceState::all();
                    let from = states[from_idx as usize];
                    let to = states[to_idx as usize];
                    let machine_result = WorkspaceStateMachine::transition(from, to);
                    let can = from.can_transition_to(to);
                    prop_assert_eq!(machine_result.is_ok(), can);
                }
            }
        }
    }
}
