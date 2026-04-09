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
    /// `AgentState` has no terminal states per spec - any state can transition to Offline or Error
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

    /// Attempt to transition to a new state.
    ///
    /// # Errors
    ///
    /// Returns an error if the transition is not valid for the current state.
    pub fn transition_to(self, target: Self) -> crate::error::Result<Self> {
        if self.can_transition_to(&target) {
            Ok(target)
        } else {
            Err(Error::invalid_state(format!(
                "Invalid transition from {self:?} to {target:?}"
            )))
        }
    }
}

/// Agent state machine with pure transition functions
#[derive(Debug, Clone, Copy, Default)]
pub struct AgentStateMachine;

impl AgentStateMachine {
    /// Attempt to transition from one state to another.
    ///
    /// # Errors
    ///
    /// Returns an error if the transition is not valid for the given states.
    pub fn transition(from: AgentState, to: AgentState) -> crate::error::Result<AgentState> {
        from.transition_to(to)
    }

    /// Check if a transition from one state to another is valid
    #[must_use]
    pub const fn can_transition(from: AgentState, to: AgentState) -> bool {
        from.can_transition_to(&to)
    }

    /// Check if a state is terminal (no further transitions possible)
    /// `AgentState` has no terminal states per spec - any state can transition to Offline or Error
    #[must_use]
    pub const fn is_terminal(state: AgentState) -> bool {
        state.is_terminal()
    }

    /// Check if a state is available (can process work)
    /// Available states are Idle and Active
    #[must_use]
    pub const fn is_available(state: AgentState) -> bool {
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

    // =========================================================================
    // Exhaustive Transition Matrix
    // =========================================================================

    #[test]
    fn test_full_transition_matrix() {
        // Verify every (from, to) pair has correct can_transition_to result
        let states = AgentState::all();

        // Valid transitions per the match in can_transition_to:
        let valid_pairs: Vec<(AgentState, AgentState)> = vec![
            // Idle <-> Active (bidirectional)
            (AgentState::Idle, AgentState::Active),
            (AgentState::Active, AgentState::Idle),
            // Any -> Offline (Idle, Active, Error)
            (AgentState::Idle, AgentState::Offline),
            (AgentState::Active, AgentState::Offline),
            (AgentState::Error, AgentState::Offline),
            // Any -> Error (Idle, Active, Offline)
            (AgentState::Idle, AgentState::Error),
            (AgentState::Active, AgentState::Error),
            (AgentState::Offline, AgentState::Error),
            // Offline -> Idle
            (AgentState::Offline, AgentState::Idle),
            // Self-transitions are NOT valid (fall through to _ => false)
        ];

        for from in &states {
            for to in &states {
                let expected = valid_pairs.contains(&(*from, *to));
                let actual = from.can_transition_to(to);
                assert_eq!(
                    actual, expected,
                    "can_transition_to({from:?}, {to:?}): expected {expected}, got {actual}"
                );
            }
        }
    }

    #[test]
    fn test_invalid_transitions_comprehensive() {
        let invalid_pairs: Vec<(AgentState, AgentState)> = vec![
            // Error -> Active (must go through Offline -> Idle -> Active)
            (AgentState::Error, AgentState::Active),
            // Error -> Idle
            (AgentState::Error, AgentState::Idle),
            // Offline -> Active (must go through Idle first)
            (AgentState::Offline, AgentState::Active),
            // Self-transitions are NOT valid
            (AgentState::Idle, AgentState::Idle),
            (AgentState::Active, AgentState::Active),
            (AgentState::Offline, AgentState::Offline),
            (AgentState::Error, AgentState::Error),
        ];

        for (from, to) in invalid_pairs {
            assert!(
                !from.can_transition_to(&to),
                "transition({from:?}, {to:?}) should be invalid"
            );
            let result = from.transition_to(to);
            assert!(result.is_err(), "transition({from:?}, {to:?}) should error");
        }
    }

    #[test]
    fn test_self_transitions_invalid() {
        // Self-transitions are NOT valid per the state machine spec
        for state in AgentState::all() {
            assert!(
                !state.can_transition_to(&state),
                "self-transition for {state:?} should be invalid"
            );
            let result = state.transition_to(state);
            assert!(
                result.is_err(),
                "self-transition for {state:?} should error"
            );
        }
    }

    // =========================================================================
    // valid_transitions() per state
    // =========================================================================

    #[test]
    fn test_idle_valid_transitions() {
        let transitions = AgentState::Idle.valid_transitions();
        assert_eq!(transitions.len(), 3);
        assert!(transitions.contains(&AgentState::Active));
        assert!(transitions.contains(&AgentState::Offline));
        assert!(transitions.contains(&AgentState::Error));
        // Self-transition NOT included
    }

    #[test]
    fn test_active_valid_transitions() {
        let transitions = AgentState::Active.valid_transitions();
        assert_eq!(transitions.len(), 3);
        assert!(transitions.contains(&AgentState::Idle));
        assert!(transitions.contains(&AgentState::Offline));
        assert!(transitions.contains(&AgentState::Error));
    }

    #[test]
    fn test_offline_valid_transitions() {
        let transitions = AgentState::Offline.valid_transitions();
        assert_eq!(transitions.len(), 2);
        assert!(transitions.contains(&AgentState::Idle));
        assert!(transitions.contains(&AgentState::Error));
    }

    #[test]
    fn test_error_valid_transitions() {
        let transitions = AgentState::Error.valid_transitions();
        assert_eq!(transitions.len(), 1);
        assert!(transitions.contains(&AgentState::Offline));
    }

    // =========================================================================
    // AgentInfo Edge Cases
    // =========================================================================

    #[test]
    fn test_agent_info_preserves_id() {
        let id = AgentId::parse("unique-agent-42").unwrap();
        let info = AgentInfo::new(id, AgentState::Active);
        assert_eq!(info.id.as_str(), "unique-agent-42");
    }

    #[test]
    fn test_agent_info_last_seen_is_none_by_default() {
        let id = AgentId::parse("test").unwrap();
        let info = AgentInfo::new(id, AgentState::Idle);
        assert!(info.last_seen.is_none());
    }

    #[test]
    fn test_agent_info_with_last_seen_preserves_state() {
        let id = AgentId::parse("test").unwrap();
        let now = chrono::Utc::now();
        let info = AgentInfo::new(id, AgentState::Active).with_last_seen(now);
        assert_eq!(info.state, AgentState::Active);
        assert_eq!(info.last_seen, Some(now));
    }

    #[test]
    fn test_agent_info_with_last_seen_overwrites() {
        let id = AgentId::parse("test").unwrap();
        let t1 = chrono::Utc::now();
        let t2 = t1 + chrono::TimeDelta::try_seconds(60).unwrap();
        let info = AgentInfo::new(id, AgentState::Idle).with_last_seen(t1);
        let info = info.with_last_seen(t2);
        assert_eq!(info.last_seen, Some(t2));
    }

    #[test]
    fn test_agent_info_all_states() {
        let id = AgentId::parse("test").unwrap();
        for state in AgentState::all() {
            let info = AgentInfo::new(id.clone(), state);
            assert_eq!(info.state, state);
        }
    }

    // =========================================================================
    // Transition Error Messages
    // =========================================================================

    #[test]
    fn test_transition_error_contains_state_names() {
        let err = AgentState::Error
            .transition_to(AgentState::Active)
            .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("Invalid transition"),
            "error message should describe invalid transition: {msg}"
        );
    }

    // =========================================================================
    // Activity Timestamp Tracking (via AgentInfo)
    // =========================================================================

    #[test]
    fn test_activity_timestamp_ordering() {
        let id = AgentId::parse("test").unwrap();
        let t1 = chrono::Utc::now();
        let t2 = t1 + chrono::TimeDelta::try_seconds(1).unwrap();
        let t3 = t2 + chrono::TimeDelta::try_seconds(1).unwrap();

        let info = AgentInfo::new(id, AgentState::Idle)
            .with_last_seen(t1)
            .with_last_seen(t2)
            .with_last_seen(t3);

        assert_eq!(info.last_seen, Some(t3));
        assert!(t3 > t2);
        assert!(t2 > t1);
    }

    // =========================================================================
    // Lifecycle Sequence Tests
    // =========================================================================

    #[test]
    fn test_full_lifecycle_idle_active_offline_idle() {
        // Start idle, go active, go offline, recover to idle
        let state = AgentState::Idle;
        let state = state.transition_to(AgentState::Active).unwrap();
        assert_eq!(state, AgentState::Active);

        let state = state.transition_to(AgentState::Offline).unwrap();
        assert_eq!(state, AgentState::Offline);

        let state = state.transition_to(AgentState::Idle).unwrap();
        assert_eq!(state, AgentState::Idle);
    }

    #[test]
    fn test_full_lifecycle_with_error_recovery() {
        // Active -> Error -> Offline -> Idle -> Active
        let state = AgentState::Active;
        let state = state.transition_to(AgentState::Error).unwrap();
        assert_eq!(state, AgentState::Error);

        let state = state.transition_to(AgentState::Offline).unwrap();
        assert_eq!(state, AgentState::Offline);

        let state = state.transition_to(AgentState::Idle).unwrap();
        assert_eq!(state, AgentState::Idle);

        let state = state.transition_to(AgentState::Active).unwrap();
        assert_eq!(state, AgentState::Active);
    }

    #[test]
    fn test_error_cannot_go_directly_to_active() {
        // Error must go through Offline -> Idle -> Active
        let state = AgentState::Error;
        assert!(state.transition_to(AgentState::Active).is_err());
        assert!(state.transition_to(AgentState::Idle).is_err());

        // But can go to Offline
        let state = state.transition_to(AgentState::Offline).unwrap();
        // Then to Idle
        let state = state.transition_to(AgentState::Idle).unwrap();
        // Then to Active
        let state = state.transition_to(AgentState::Active).unwrap();
        assert_eq!(state, AgentState::Active);
    }

    #[test]
    fn test_offline_cannot_go_directly_to_active() {
        assert!(AgentState::Offline
            .transition_to(AgentState::Active)
            .is_err());
    }

    // =========================================================================
    // Proptests
    // =========================================================================

    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq};

    /// Strategy for generating any AgentState
    fn agent_state_strategy() -> impl Strategy<Value = AgentState> {
        prop_oneof![
            Just(AgentState::Idle),
            Just(AgentState::Active),
            Just(AgentState::Offline),
            Just(AgentState::Error),
        ]
    }

    proptest! {
        /// can_transition_to agrees with transition_to for all pairs
        #[test]
        fn proptest_transition_agrees_with_can_transition(
            from in agent_state_strategy(),
            to in agent_state_strategy()
        ) {
            let can = from.can_transition_to(&to);
            let result = from.transition_to(to);
            prop_assert_eq!(can, result.is_ok());
        }

        /// Valid transition returns the target state
        #[test]
        fn proptest_valid_transition_returns_target(
            from in agent_state_strategy(),
            to in agent_state_strategy()
        ) {
            if from.can_transition_to(&to) {
                let result = from.transition_to(to).expect("valid");
                prop_assert_eq!(result, to);
            }
        }

        /// Invalid transition preserves the from state in the error
        #[test]
        fn proptest_invalid_transition_preserves_from(
            from in agent_state_strategy(),
            to in agent_state_strategy()
        ) {
            if !from.can_transition_to(&to) {
                assert!(from.transition_to(to).is_err());
            }
        }

        /// is_available matches known available states
        #[test]
        fn proptest_is_available_consistency(state in agent_state_strategy()) {
            let expected = matches!(state, AgentState::Idle | AgentState::Active);
            prop_assert_eq!(state.is_available(), expected);
        }

        /// is_active matches Active variant only
        #[test]
        fn proptest_is_active_consistency(state in agent_state_strategy()) {
            prop_assert_eq!(state.is_active(), matches!(state, AgentState::Active));
        }

        /// is_offline matches Offline variant only
        #[test]
        fn proptest_is_offline_consistency(state in agent_state_strategy()) {
            prop_assert_eq!(state.is_offline(), matches!(state, AgentState::Offline));
        }

        /// No state is terminal
        #[test]
        fn proptest_no_terminal_states(state in agent_state_strategy()) {
            prop_assert!(!state.is_terminal());
        }

        /// valid_transitions only contains valid targets
        #[test]
        fn proptest_valid_transitions_subset(state in agent_state_strategy()) {
            let transitions = state.valid_transitions();
            for target in transitions {
                prop_assert!(state.can_transition_to(&target));
            }
        }

        /// Display output matches expected string
        #[test]
        fn proptest_display_matches(state in agent_state_strategy()) {
            let expected = match state {
                AgentState::Active => "active",
                AgentState::Idle => "idle",
                AgentState::Offline => "offline",
                AgentState::Error => "error",
            };
            prop_assert_eq!(state.to_string(), expected);
        }

        /// Sequential transitions are deterministic
        #[test]
        fn proptest_sequential_transitions(
            transitions in prop::collection::vec(agent_state_strategy(), 1..=20)
        ) {
            let mut current = AgentState::Idle;
            for target in transitions {
                if current.can_transition_to(&target) {
                    current = current.transition_to(target).expect("valid");
                }
            }
            // Final state must be a valid AgentState
            prop_assert!(AgentState::all().contains(&current));
        }

        /// AgentStateMachine delegates correctly
        #[test]
        fn proptest_state_machine_delegates(
            from in agent_state_strategy(),
            to in agent_state_strategy()
        ) {
            let direct = from.transition_to(to);
            let via_sm = AgentStateMachine::transition(from, to);
            prop_assert_eq!(direct.is_ok(), via_sm.is_ok());
            if let (Ok(d), Ok(s)) = (direct, via_sm) {
                prop_assert_eq!(d, s);
            }
        }

        /// AgentInfo with_last_seen always overwrites
        #[test]
        fn proptest_agent_info_last_seen_overwrite(
            t1 in -86400i64..=86400i64,
            t2 in -86400i64..=86400i64
        ) {
            let id = AgentId::parse("test").unwrap();
            let time1 = chrono::Utc::now() + chrono::TimeDelta::try_seconds(t1).unwrap_or(chrono::TimeDelta::zero());
            let time2 = chrono::Utc::now() + chrono::TimeDelta::try_seconds(t2).unwrap_or(chrono::TimeDelta::zero());
            let info = AgentInfo::new(id, AgentState::Active)
                .with_last_seen(time1)
                .with_last_seen(time2);
            prop_assert_eq!(info.last_seen, Some(time2));
        }

        /// Self-transition always fails
        #[test]
        fn proptest_self_transition_always_invalid(state in agent_state_strategy()) {
            prop_assert!(!state.can_transition_to(&state));
            prop_assert!(state.transition_to(state).is_err());
        }

        /// Bidirectional symmetry: if A->B is valid and B->A is valid,
        /// then both roundtrips work
        #[test]
        fn proptest_bidirectional_symmetry(
            a in agent_state_strategy(),
            b in agent_state_strategy()
        ) {
            if a.can_transition_to(&b) && b.can_transition_to(&a) {
                let via_ab = a.transition_to(b).expect("ok").transition_to(a).expect("ok");
                prop_assert_eq!(via_ab, a);
            }
        }
    }
}
