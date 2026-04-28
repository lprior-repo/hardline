use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// Value object representing the state of a worktree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WorktreeState {
    /// Worktree is being created
    Creating = 0,

    /// Worktree exists but is not fully initialized
    Incomplete = 1,

    /// Worktree is ready and usable
    Active = 2,

    /// Worktree is temporarily unavailable
    Suspended = 3,

    /// Worktree is being removed
    Removing = 4,

    /// Worktree has been removed
    Removed = 5,
}

impl WorktreeState {
    /// Create a worktree state from a numeric value
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(WorktreeState::Creating),
            1 => Some(WorktreeState::Incomplete),
            2 => Some(WorktreeState::Active),
            3 => Some(WorktreeState::Suspended),
            4 => Some(WorktreeState::Removing),
            5 => Some(WorktreeState::Removed),
            _ => None,
        }
    }

    /// Convert to numeric value
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Get a human-readable name for the state
    pub fn name(&self) -> &'static str {
        match self {
            WorktreeState::Creating => "Creating",
            WorktreeState::Incomplete => "Incomplete",
            WorktreeState::Active => "Active",
            WorktreeState::Suspended => "Suspended",
            WorktreeState::Removing => "Removing",
            WorktreeState::Removed => "Removed",
        }
    }

    /// Check if state is terminal (no further transitions allowed)
    pub fn is_terminal(self) -> bool {
        matches!(self, WorktreeState::Removed)
    }

    /// Check if state is active (worktree can be used)
    pub fn is_active(self) -> bool {
        matches!(self, WorktreeState::Active)
    }

    /// Check if state is transient (intermediate states)
    pub fn is_transient(self) -> bool {
        matches!(
            self,
            WorktreeState::Creating | WorktreeState::Incomplete | WorktreeState::Removing
        )
    }

    /// Get valid next states from this state
    pub fn valid_next_states(self) -> Vec<WorktreeState> {
        match self {
            WorktreeState::Creating => vec![WorktreeState::Active, WorktreeState::Removed],
            WorktreeState::Incomplete => vec![
                WorktreeState::Active,
                WorktreeState::Suspended,
                WorktreeState::Removed,
            ],
            WorktreeState::Active => vec![WorktreeState::Suspended, WorktreeState::Removing],
            WorktreeState::Suspended => vec![WorktreeState::Active, WorktreeState::Removing],
            WorktreeState::Removing => vec![WorktreeState::Removed],
            WorktreeState::Removed => vec![], // Terminal state
        }
    }

    /// Check if state transition is valid
    pub fn can_transition_to(self, target: WorktreeState) -> bool {
        self.valid_next_states().contains(&target)
    }
}

impl Display for WorktreeState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.name(), f)
    }
}

impl From<WorktreeState> for u8 {
    fn from(state: WorktreeState) -> Self {
        state.as_u8()
    }
}

impl TryFrom<u8> for WorktreeState {
    type Error = super::WorktreeDomainError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        WorktreeState::from_u8(value).ok_or({
            super::WorktreeDomainError::InvalidStateTransition(
                WorktreeState::Active,
                WorktreeState::Active,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_state_from_u8_returns_correct_state_for_valid_values() {
        assert_eq!(WorktreeState::from_u8(0), Some(WorktreeState::Creating));
        assert_eq!(WorktreeState::from_u8(2), Some(WorktreeState::Active));
        assert_eq!(WorktreeState::from_u8(5), Some(WorktreeState::Removed));
        assert_eq!(WorktreeState::from_u8(99), None);
    }

    #[test]
    fn worktree_state_as_u8_returns_correct_value() {
        assert_eq!(WorktreeState::Active.as_u8(), 2);
        assert_eq!(WorktreeState::Removed.as_u8(), 5);
    }

    #[test]
    fn worktree_state_name_returns_human_readable_name() {
        assert_eq!(WorktreeState::Active.name(), "Active");
        assert_eq!(WorktreeState::Removed.name(), "Removed");
    }

    #[test]
    fn worktree_state_is_terminal_returns_true_for_removed() {
        assert!(WorktreeState::Removed.is_terminal());
        assert!(!WorktreeState::Active.is_terminal());
    }

    #[test]
    fn worktree_state_is_active_returns_true_for_active() {
        assert!(WorktreeState::Active.is_active());
        assert!(!WorktreeState::Suspended.is_active());
    }

    #[test]
    fn worktree_state_is_transient_returns_true_for_intermediate_states() {
        assert!(WorktreeState::Creating.is_transient());
        assert!(!WorktreeState::Active.is_transient());
        assert!(!WorktreeState::Removed.is_transient());
    }

    #[test]
    fn worktree_state_valid_next_states_returns_correct_transitions() {
        assert_eq!(
            WorktreeState::Creating.valid_next_states(),
            vec![WorktreeState::Active, WorktreeState::Removed]
        );

        assert_eq!(
            WorktreeState::Active.valid_next_states(),
            vec![WorktreeState::Suspended, WorktreeState::Removing]
        );

        assert_eq!(WorktreeState::Removed.valid_next_states(), vec![]);
    }

    #[test]
    fn worktree_state_can_transition_to_returns_true_for_valid_transitions() {
        assert!(WorktreeState::Creating.can_transition_to(WorktreeState::Active));
        assert!(!WorktreeState::Creating.can_transition_to(WorktreeState::Suspended));
        assert!(!WorktreeState::Removed.can_transition_to(WorktreeState::Active));
    }

    #[test]
    fn worktree_state_try_from_u8_returns_correct_state() {
        let state: WorktreeState = WorktreeState::Active.as_u8().try_into().unwrap();
        assert_eq!(state, WorktreeState::Active);
    }
}

// Note: State machine tests require proptest-state-machine crate which is not in workspace
// The state machine tests were removed due to proptest version incompatibility
