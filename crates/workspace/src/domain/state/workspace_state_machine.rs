use crate::domain::entities::WorkspaceState;
use crate::error::WorkspaceError;

pub struct WorkspaceStateMachine;

impl WorkspaceStateMachine {
    pub fn can_transition(from: WorkspaceState, to: WorkspaceState) -> bool {
        match (from, to) {
            (WorkspaceState::Initializing, WorkspaceState::Active) => true,
            (WorkspaceState::Active, WorkspaceState::Locked) => true,
            (WorkspaceState::Locked, WorkspaceState::Active) => true,
            (WorkspaceState::Active, WorkspaceState::Corrupted) => true,
            (WorkspaceState::Locked, WorkspaceState::Corrupted) => true,
            (_, WorkspaceState::Deleted) => true,
            _ => false,
        }
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
    fn state_machine_initializing_to_deleted_is_invalid() {
        assert!(!WorkspaceStateMachine::can_transition(
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
}
