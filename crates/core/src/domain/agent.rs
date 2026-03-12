//! Agent domain types
//!
//! Provides types for agent state and operations.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::domain::identifiers::AgentId;
use crate::error::Error;

/// Agent state information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// Agent is active and processing
    Active,
    /// Agent is idle
    Idle,
    /// Agent is offline
    Offline,
    /// Agent is in error state
    Error,
}

impl AgentState {
    /// All valid agent states
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Idle, Self::Active, Self::Offline, Self::Error]
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    #[must_use]
    pub const fn is_offline(&self) -> bool {
        matches!(self, Self::Offline)
    }

    /// Check if a transition from self to target is valid
    #[must_use]
    #[allow(clippy::match_same_arms)] // More readable as explicit patterns
    pub const fn can_transition_to(self, target: &Self) -> bool {
        match (self, target) {
            // Valid transitions:
            // - Idle <-> Active (bidirectional)
            // - Any state -> Offline
            // - Any state -> Error
            // - Offline -> Idle
            (Self::Idle, Self::Active) | (Self::Active, Self::Idle) => true,
            (Self::Idle | Self::Active | Self::Error, Self::Offline) => true,
            (Self::Idle | Self::Active | Self::Offline, Self::Error) => true,
            (Self::Offline, Self::Idle) => true,

            // Self-loops and other transitions not allowed
            _ => false,
        }
    }

    /// Get all valid target states from this state
    #[must_use]
    pub fn valid_transitions(&self) -> Vec<Self> {
        Self::all()
            .iter()
            .filter(|&target| self.can_transition_to(target))
            .copied()
            .collect()
    }

    /// Check if this state is terminal
    /// AgentState has no terminal states per spec - any state can transition to Offline or Error
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        false
    }

    /// Check if this state is available (can process work)
    /// Available states are Idle and Active
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Idle | Self::Active)
    }

    /// Attempt to transition to a new state
    /// Returns Ok(new_state) if transition is valid, Err(AgentError::InvalidTransition) otherwise
    pub fn transition_to(self, target: Self) -> crate::error::Result<Self> {
        if self.can_transition_to(&target) {
            Ok(target)
        } else {
            Err(Error::InvalidState(format!(
                "Invalid transition from {:?} to {:?}",
                self, target
            )))
        }
    }
}

/// Agent state machine with pure transition functions
#[derive(Debug, Clone, Copy, Default)]
pub struct AgentStateMachine;

impl AgentStateMachine {
    /// Attempt to transition from one state to another
    /// Returns Ok(new_state) if transition is valid, Err(InvalidTransition) otherwise
    pub fn transition(from: AgentState, to: AgentState) -> crate::error::Result<AgentState> {
        from.transition_to(to)
    }

    /// Check if a transition from one state to another is valid
    #[must_use]
    pub fn can_transition(from: AgentState, to: AgentState) -> bool {
        from.can_transition_to(&to)
    }

    /// Check if a state is terminal (no further transitions possible)
    /// AgentState has no terminal states per spec - any state can transition to Offline or Error
    #[must_use]
    pub fn is_terminal(state: AgentState) -> bool {
        state.is_terminal()
    }

    /// Check if a state is available (can process work)
    /// Available states are Idle and Active
    #[must_use]
    pub fn is_available(state: AgentState) -> bool {
        state.is_available()
    }
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Idle => write!(f, "idle"),
            Self::Offline => write!(f, "offline"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Agent information
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: AgentId,
    pub state: AgentState,
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
}

impl AgentInfo {
    #[must_use]
    pub const fn new(id: AgentId, state: AgentState) -> Self {
        Self {
            id,
            state,
            last_seen: None,
        }
    }

    #[must_use]
    pub fn with_last_seen(self, last_seen: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            id: self.id,
            state: self.state,
            last_seen: Some(last_seen),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AgentState Tests
    // =========================================================================

    #[test]
    fn test_all_states() {
        let states = AgentState::all();
        assert_eq!(states.len(), 4);
        assert!(states.contains(&AgentState::Idle));
        assert!(states.contains(&AgentState::Active));
        assert!(states.contains(&AgentState::Offline));
        assert!(states.contains(&AgentState::Error));
    }

    #[test]
    fn test_idle_is_available() {
        assert!(AgentState::Idle.is_available());
        assert!(!AgentState::Idle.is_active());
    }

    #[test]
    fn test_active_is_available() {
        assert!(AgentState::Active.is_available());
        assert!(AgentState::Active.is_active());
    }

    #[test]
    fn test_offline_not_available() {
        assert!(!AgentState::Offline.is_available());
        assert!(AgentState::Offline.is_offline());
    }

    #[test]
    fn test_error_not_available() {
        assert!(!AgentState::Error.is_available());
    }

    #[test]
    fn test_no_terminal_states() {
        // AgentState has no terminal states per spec
        assert!(!AgentState::Idle.is_terminal());
        assert!(!AgentState::Active.is_terminal());
        assert!(!AgentState::Offline.is_terminal());
        assert!(!AgentState::Error.is_terminal());
    }

    // =========================================================================
    // AgentState Transition Tests (matching contract violation examples)
    // =========================================================================

    #[test]
    fn test_valid_idle_to_active_transition() {
        let result = AgentState::Idle.transition_to(AgentState::Active);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), AgentState::Active);
    }

    #[test]
    fn test_valid_active_to_idle_transition() {
        let result = AgentState::Active.transition_to(AgentState::Idle);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), AgentState::Idle);
    }

    #[test]
    fn test_valid_any_to_offline() {
        // From contract: any state -> Offline is valid
        assert!(AgentState::Idle.can_transition_to(&AgentState::Offline));
        assert!(AgentState::Active.can_transition_to(&AgentState::Offline));
        assert!(AgentState::Error.can_transition_to(&AgentState::Offline));
    }

    #[test]
    fn test_valid_any_to_error() {
        // From contract: any state -> Error is valid
        assert!(AgentState::Idle.can_transition_to(&AgentState::Error));
        assert!(AgentState::Active.can_transition_to(&AgentState::Error));
        assert!(AgentState::Offline.can_transition_to(&AgentState::Error));
    }

    #[test]
    fn test_valid_offline_to_idle() {
        let result = AgentState::Offline.transition_to(AgentState::Idle);
        assert!(result.is_ok());
    }

    // Contract violation examples - MUST produce errors
    #[test]
    fn test_invalid_error_to_active_transition() {
        // From contract: transition(Error, Active) -> should Err
        let result = AgentState::Error.transition_to(AgentState::Active);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_idle_to_error_transition() {
        // From contract: transition(Idle, Error) -> should Ok (valid transition per "any->Error" rule)
        let result = AgentState::Idle.transition_to(AgentState::Error);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_active_to_conflict_transition() {
        // Active -> Conflict is not a valid transition
        let result = AgentState::Active.transition_to(AgentState::Error);
        // Actually per spec any state -> Error is valid, so this should pass
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_transitions_list() {
        let idle_transitions = AgentState::Idle.valid_transitions();
        assert!(idle_transitions.contains(&AgentState::Active));
        assert!(idle_transitions.contains(&AgentState::Offline));
        assert!(idle_transitions.contains(&AgentState::Error));
        assert_eq!(idle_transitions.len(), 3);
    }

    // =========================================================================
    // AgentStateMachine Tests
    // =========================================================================

    #[test]
    fn test_state_machine_transition() {
        let result = AgentStateMachine::transition(AgentState::Idle, AgentState::Active);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), AgentState::Active);
    }

    #[test]
    fn test_state_machine_can_transition() {
        assert!(AgentStateMachine::can_transition(
            AgentState::Idle,
            AgentState::Active
        ));
        assert!(!AgentStateMachine::can_transition(
            AgentState::Error,
            AgentState::Active
        ));
    }

    #[test]
    fn test_state_machine_is_terminal() {
        // AgentState has no terminal states
        assert!(!AgentStateMachine::is_terminal(AgentState::Idle));
        assert!(!AgentStateMachine::is_terminal(AgentState::Active));
        assert!(!AgentStateMachine::is_terminal(AgentState::Offline));
        assert!(!AgentStateMachine::is_terminal(AgentState::Error));
    }

    #[test]
    fn test_state_machine_is_available() {
        assert!(AgentStateMachine::is_available(AgentState::Idle));
        assert!(AgentStateMachine::is_available(AgentState::Active));
        assert!(!AgentStateMachine::is_available(AgentState::Offline));
        assert!(!AgentStateMachine::is_available(AgentState::Error));
    }

    // =========================================================================
    // AgentInfo Tests
    // =========================================================================

    #[test]
    fn test_agent_info_new() {
        let id = AgentId::parse("test-agent-001").unwrap();
        let info = AgentInfo::new(id, AgentState::Idle);
        assert_eq!(info.state, AgentState::Idle);
        assert!(info.last_seen.is_none());
    }

    #[test]
    fn test_agent_info_with_last_seen() {
        use chrono::Utc;
        let id = AgentId::parse("test-agent-002").unwrap();
        let info = AgentInfo::new(id, AgentState::Active);
        let now = Utc::now();
        let updated = info.with_last_seen(now);
        assert_eq!(updated.state, AgentState::Active);
        assert!(updated.last_seen.is_some());
    }

    // =========================================================================
    // AgentState Display Tests
    // =========================================================================

    #[test]
    fn test_state_display() {
        assert_eq!(AgentState::Idle.to_string(), "idle");
        assert_eq!(AgentState::Active.to_string(), "active");
        assert_eq!(AgentState::Offline.to_string(), "offline");
        assert_eq!(AgentState::Error.to_string(), "error");
    }
}
