//! Workspace state machine for session-based workspaces.
//!
//! This module provides the WorkspaceState enum and WorkspaceStateMachine
//! for managing the lifecycle of session workspaces.

use serde::{Deserialize, Serialize};

/// Workspace state for session-based workspaces.
///
/// Lifecycle: Created → Working → Ready → Merged/Conflict/Abandoned
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkspaceState {
    /// Workspace has been created but work hasn't started
    Created,
    /// Workspace is actively being worked on
    Working,
    /// Workspace work is complete and ready for merge
    Ready,
    /// Workspace has been successfully merged
    Merged,
    /// Workspace has merge conflicts
    Conflict,
    /// Workspace was abandoned
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

    /// Check if this state is ready (ready for merge decision)
    #[must_use]
    pub const fn is_ready(self) -> bool {
        self == Self::Ready
    }

    /// Check if a transition from this state to target is valid
    #[must_use]
    pub const fn can_transition_to(self, target: Self) -> bool {
        match (self, target) {
            // Forward progression: Created → Working → Ready
            (Self::Created, Self::Working) => true,
            (Self::Working, Self::Ready) => true,
            // From Ready: can go to terminal states
            (Self::Ready, Self::Merged) => true,
            (Self::Ready, Self::Conflict) => true,
            (Self::Ready, Self::Abandoned) => true,
            // Self-loops are not allowed (use explicit no-op if needed)
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

impl Default for WorkspaceState {
    fn default() -> Self {
        Self::Created
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
            Err(crate::error::SessionError::InvalidTransition {
                from: format!("{:?}", from),
                to: format!("{:?}", to),
            })
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
    fn workspace_state_ready_to_abandoned_transition_succeeds() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Abandoned);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), WorkspaceState::Abandoned);
    }

    #[test]
    fn workspace_state_invalid_created_to_merged_fails() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Merged);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_state_invalid_ready_to_created_fails() {
        let result =
            WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Created);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_state_terminal_states_cannot_transition() {
        for terminal in [
            WorkspaceState::Merged,
            WorkspaceState::Conflict,
            WorkspaceState::Abandoned,
        ] {
            let result = WorkspaceStateMachine::transition(terminal, WorkspaceState::Created);
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
            WorkspaceState::Merged
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
        assert!(!WorkspaceStateMachine::is_ready(WorkspaceState::Merged));
    }

    #[test]
    fn workspace_state_valid_transitions_lists_correct_targets() {
        let created_targets = WorkspaceState::Created.valid_transitions();
        assert_eq!(created_targets, vec![WorkspaceState::Working]);

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
