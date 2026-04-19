//! Workspace state machine for the isolate domain.

use super::types::WorkspaceState;
use crate::error::{IsolateError, Result};

/// State machine governing workspace lifecycle transitions.
///
/// ```text
/// Created -> Working -> Ready -> Merged
///                   |-> Conflict -> Working
///                   |-> Abandoned
///                   `-> (Ready can also: Working, Merged, Conflict, Abandoned)
/// ```
pub struct WorkspaceStateMachine;

impl WorkspaceStateMachine {
    /// Attempt a state transition, returning the new state or an error.
    pub fn transition(from: WorkspaceState, to: WorkspaceState) -> Result<WorkspaceState> {
        if from.can_transition_to(to) {
            Ok(to)
        } else {
            Err(IsolateError::InvalidTransition {
                from: from.to_string(),
                to: to.to_string(),
            })
        }
    }

    /// Check whether a transition is valid without performing it.
    #[must_use]
    pub fn can_transition(from: WorkspaceState, to: WorkspaceState) -> bool {
        from.can_transition_to(to)
    }

    /// Check whether a state is terminal (no further transitions).
    #[must_use]
    pub fn is_terminal(state: WorkspaceState) -> bool {
        state.is_terminal()
    }

    /// Check whether a state indicates active work.
    #[must_use]
    pub fn is_active(state: WorkspaceState) -> bool {
        state.is_active()
    }

    /// Check whether a state indicates completion.
    #[must_use]
    pub fn is_complete(state: WorkspaceState) -> bool {
        state.is_complete()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_created_to_merged() {
        let s = WorkspaceState::Created;
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
        assert_eq!(s, WorkspaceState::Working);
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
        assert_eq!(s, WorkspaceState::Ready);
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Merged).unwrap();
        assert!(s.is_terminal());
    }

    #[test]
    fn conflict_path() {
        let s = WorkspaceState::Created;
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Conflict).unwrap();
        assert!(s.is_active());
        assert!(!s.is_terminal());
    }

    #[test]
    fn early_abandon_path() {
        let s = WorkspaceState::Created;
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Abandoned).unwrap();
        assert!(s.is_terminal());
    }

    #[test]
    fn created_to_ready_rejected() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Ready);
        assert!(result.is_err());
    }

    #[test]
    fn merged_to_any_rejected() {
        for target in WorkspaceState::all() {
            let result = WorkspaceStateMachine::transition(WorkspaceState::Merged, *target);
            assert!(result.is_err(), "Merged -> {target:?} should fail");
        }
    }

    #[test]
    fn abandoned_to_any_rejected() {
        for target in WorkspaceState::all() {
            let result = WorkspaceStateMachine::transition(WorkspaceState::Abandoned, *target);
            assert!(result.is_err(), "Abandoned -> {target:?} should fail");
        }
    }

    #[test]
    fn conflict_to_working_recovers() {
        let s = WorkspaceState::Created;
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Conflict).unwrap();
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
        assert!(s.is_active());
    }

    #[test]
    fn ready_to_working_rework() {
        let s = WorkspaceState::Created;
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
        let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
        assert!(s.is_active());
    }

    #[test]
    fn transition_error_has_context() {
        let err =
            WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Merged)
                .err()
                .unwrap();
        let msg = format!("{err}");
        assert!(msg.contains("created"));
        assert!(msg.contains("merged"));
    }

    #[test]
    fn helper_consistency_checks() {
        for state in WorkspaceState::all() {
            assert_eq!(
                WorkspaceStateMachine::is_terminal(*state),
                state.is_terminal()
            );
            assert_eq!(WorkspaceStateMachine::is_active(*state), state.is_active());
            assert_eq!(
                WorkspaceStateMachine::is_complete(*state),
                state.is_complete()
            );
        }
    }
}
