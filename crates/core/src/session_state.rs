//! Session State Tracking Infrastructure
//!
//! Provides a type-safe state machine for session lifecycle management using:
//! - State Transition enums for valid state changes
//! - `SessionStateManager` for managing state transitions
//! - Type State Pattern with Phantom Types for compile-time safety
//! - `SessionBeadsContext` for beads integration
//! - State history tracking and validation
//! - Railway-Oriented error handling with zero panics

use std::{collections::HashMap, marker::PhantomData};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;

// ═══════════════════════════════════════════════════════════════════════════
// STATE TYPES & TRANSITIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Compile-time state marker for Created sessions
#[derive(Debug, Clone, Copy)]
pub struct Created;

/// Compile-time state marker for Active sessions
#[derive(Debug, Clone, Copy)]
pub struct Active;

/// Compile-time state marker for Syncing sessions
#[derive(Debug, Clone, Copy)]
pub struct Syncing;

/// Compile-time state marker for Synced sessions
#[derive(Debug, Clone, Copy)]
pub struct Synced;

/// Compile-time state marker for Paused sessions
#[derive(Debug, Clone, Copy)]
pub struct Paused;

/// Compile-time state marker for Completed sessions
#[derive(Debug, Clone, Copy)]
pub struct Completed;

/// Compile-time state marker for Failed sessions
#[derive(Debug, Clone, Copy)]
pub struct Failed;

/// Runtime state enumeration for storage and serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    /// Session created but not yet activated
    Created,
    /// Session is active and ready for work
    Active,
    /// Session is being synced with main branch
    Syncing,
    /// Session sync completed
    Synced,
    /// Session is paused
    Paused,
    /// Session work completed
    Completed,
    /// Session creation or operation failed
    Failed,
}

impl SessionState {
    /// Returns true if this state allows transition to next state using exhaustive matching.
    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self.valid_next_states().contains(&next)
    }

    /// Returns all valid next states from current state.
    #[must_use]
    pub fn valid_next_states(self) -> Vec<Self> {
        match self {
            Self::Created => vec![Self::Active, Self::Failed],
            Self::Active => vec![Self::Syncing, Self::Paused, Self::Completed],
            Self::Syncing => vec![Self::Synced, Self::Failed],
            Self::Synced => vec![Self::Active, Self::Paused, Self::Completed],
            Self::Paused => vec![Self::Active, Self::Completed],
            Self::Completed | Self::Failed => vec![Self::Created],
        }
    }

    /// Returns true if this is an active (non-dormant, non-terminal) state.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active | Self::Syncing | Self::Synced)
    }

    /// Returns true if this is a terminal state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    /// Returns all possible session states as a slice.
    #[must_use]
    pub const fn all_states() -> &'static [Self] {
        &[
            Self::Created,
            Self::Active,
            Self::Syncing,
            Self::Synced,
            Self::Paused,
            Self::Completed,
            Self::Failed,
        ]
    }
}

/// State transition event with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Source state
    pub from: SessionState,
    /// Target state
    pub to: SessionState,
    /// Timestamp of transition
    pub timestamp: DateTime<Utc>,
    /// Reason for transition (metadata)
    pub reason: String,
}

impl StateTransition {
    /// Create a new state transition
    pub fn new(from: SessionState, to: SessionState, reason: impl Into<String>) -> Self {
        Self {
            from,
            to,
            timestamp: Utc::now(),
            reason: reason.into(),
        }
    }

    /// Validate that the transition is allowed
    pub fn validate(&self) -> Result<()> {
        if self.from.can_transition_to(self.to) {
            Ok(())
        } else {
            Err(crate::error::Error::invalid_state(format!(
                "Session {:?} cannot transition to {:?}",
                self.from, self.to
            )))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION STATE MANAGER
// ═══════════════════════════════════════════════════════════════════════════

/// Session state manager with type-safe state machine.
pub struct SessionStateManager<S = Created> {
    session_id: String,
    current_state: SessionState,
    history: Vec<StateTransition>,
    metadata: HashMap<String, String>,
    _state: PhantomData<S>,
}

impl SessionStateManager<Created> {
    /// Create a new session state manager in Created state
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            current_state: SessionState::Created,
            history: Vec::new(),
            metadata: HashMap::new(),
            _state: PhantomData,
        }
    }
}

impl<S> SessionStateManager<S> {
    /// Get current session ID
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get current state
    #[must_use]
    pub const fn current_state(&self) -> SessionState {
        self.current_state
    }

    /// Get state history
    #[must_use]
    pub fn history(&self) -> &[StateTransition] {
        &self.history
    }

    /// Get metadata
    #[must_use]
    pub const fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Set metadata value
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Record a state transition
    fn record_transition(&mut self, transition: &StateTransition) -> Result<()> {
        transition.validate()?;
        self.history.push(transition.clone());
        self.current_state = transition.to;
        Ok(())
    }
}

impl SessionStateManager<Created> {
    /// Transition from Created to Active
    pub fn activate(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Active>> {
        let transition = StateTransition::new(SessionState::Created, SessionState::Active, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Created to Failed
    pub fn fail(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Failed>> {
        let transition = StateTransition::new(SessionState::Created, SessionState::Failed, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Active> {
    /// Transition from Active to Syncing
    pub fn sync(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Syncing>> {
        let transition = StateTransition::new(SessionState::Active, SessionState::Syncing, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Active to Paused
    pub fn pause(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Paused>> {
        let transition = StateTransition::new(SessionState::Active, SessionState::Paused, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Active to Completed
    pub fn complete(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Completed>> {
        let transition =
            StateTransition::new(SessionState::Active, SessionState::Completed, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Syncing> {
    /// Transition from Syncing to Synced
    pub fn sync_complete(
        mut self,
        reason: impl Into<String>,
    ) -> Result<SessionStateManager<Synced>> {
        let transition = StateTransition::new(SessionState::Syncing, SessionState::Synced, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Syncing to Failed
    pub fn fail(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Failed>> {
        let transition = StateTransition::new(SessionState::Syncing, SessionState::Failed, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Synced> {
    /// Transition from Synced to Active
    pub fn reactivate(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Active>> {
        let transition = StateTransition::new(SessionState::Synced, SessionState::Active, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Synced to Paused
    pub fn pause(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Paused>> {
        let transition = StateTransition::new(SessionState::Synced, SessionState::Paused, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Synced to Completed
    pub fn complete(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Completed>> {
        let transition =
            StateTransition::new(SessionState::Synced, SessionState::Completed, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Paused> {
    /// Transition from Paused to Active
    pub fn resume(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Active>> {
        let transition = StateTransition::new(SessionState::Paused, SessionState::Active, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Paused to Completed
    pub fn complete(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Completed>> {
        let transition =
            StateTransition::new(SessionState::Paused, SessionState::Completed, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Completed> {
    /// Transition from Completed to Created to allow restart
    pub fn restart(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Created>> {
        let transition =
            StateTransition::new(SessionState::Completed, SessionState::Created, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Failed> {
    /// Transition from Failed to Created to allow retry
    pub fn retry(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Created>> {
        let transition = StateTransition::new(SessionState::Failed, SessionState::Created, reason);
        self.record_transition(&transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ── Helpers ───────────────────────────────────────────────────────────

    /// All 14 valid (from, to) transition pairs from `valid_next_states()`.
    const VALID_TRANSITIONS: &[(SessionState, SessionState)] = &[
        (SessionState::Created, SessionState::Active),
        (SessionState::Created, SessionState::Failed),
        (SessionState::Active, SessionState::Syncing),
        (SessionState::Active, SessionState::Paused),
        (SessionState::Active, SessionState::Completed),
        (SessionState::Syncing, SessionState::Synced),
        (SessionState::Syncing, SessionState::Failed),
        (SessionState::Synced, SessionState::Active),
        (SessionState::Synced, SessionState::Paused),
        (SessionState::Synced, SessionState::Completed),
        (SessionState::Paused, SessionState::Active),
        (SessionState::Paused, SessionState::Completed),
        (SessionState::Completed, SessionState::Created),
        (SessionState::Failed, SessionState::Created),
    ];

    /// Every (from, to) pair that is NOT in the valid set (35 invalid transitions).
    fn invalid_transitions() -> Vec<(SessionState, SessionState)> {
        let valid: std::collections::HashSet<_> = VALID_TRANSITIONS.iter().copied().collect();
        SessionState::all_states()
            .iter()
            .flat_map(|&from| SessionState::all_states().iter().map(move |&to| (from, to)))
            .filter(|pair| !valid.contains(pair))
            .collect()
    }

    // ========================================================================
    // 1. Valid Transition Matrix
    // ========================================================================

    #[test]
    fn valid_transitions_pass_can_transition_to() {
        for &(from, to) in VALID_TRANSITIONS {
            assert!(
                from.can_transition_to(to),
                "can_transition_to: expected {from:?} -> {to:?} to be valid"
            );
        }
    }

    #[test]
    fn valid_transitions_pass_validate() {
        for &(from, to) in VALID_TRANSITIONS {
            let t = StateTransition::new(from, to, "ok");
            assert!(
                t.validate().is_ok(),
                "validate: expected {from:?} -> {to:?} to be valid"
            );
        }
    }

    #[test]
    fn valid_next_states_count_matches_matrix() {
        let expected_counts: [(SessionState, usize); 7] = [
            (SessionState::Created, 2),
            (SessionState::Active, 3),
            (SessionState::Syncing, 2),
            (SessionState::Synced, 3),
            (SessionState::Paused, 2),
            (SessionState::Completed, 1),
            (SessionState::Failed, 1),
        ];
        for (state, expected) in expected_counts {
            assert_eq!(
                state.valid_next_states().len(),
                expected,
                "{state:?}.valid_next_states() count"
            );
        }
    }

    #[test]
    fn valid_next_states_are_unique() {
        for &state in SessionState::all_states() {
            let next = state.valid_next_states();
            let mut seen = std::collections::HashSet::new();
            for &s in &next {
                assert!(
                    seen.insert(s),
                    "{state:?}.valid_next_states() contains duplicate: {s:?}"
                );
            }
        }
    }

    #[test]
    fn total_valid_transition_count_is_14() {
        assert_eq!(
            VALID_TRANSITIONS.len(),
            14,
            "should have exactly 14 valid transitions"
        );
    }

    // ========================================================================
    // 2. Rejected Transitions — Descriptive Errors
    // ========================================================================

    #[test]
    fn all_invalid_transitions_fail_can_transition_to() {
        for (from, to) in invalid_transitions() {
            assert!(
                !from.can_transition_to(to),
                "can_transition_to: expected {from:?} -> {to:?} to be INVALID"
            );
        }
    }

    #[test]
    fn all_invalid_transitions_fail_validate() {
        for (from, to) in invalid_transitions() {
            let t = StateTransition::new(from, to, "bad");
            assert!(
                t.validate().is_err(),
                "validate: expected {from:?} -> {to:?} to fail"
            );
        }
    }

    #[test]
    fn invalid_transition_error_is_descriptive() {
        let t = StateTransition::new(SessionState::Created, SessionState::Paused, "bad");
        let err = t.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Created") || msg.contains("created"),
            "error should name source state: {msg}"
        );
        assert!(
            msg.contains("Paused") || msg.contains("paused"),
            "error should name target state: {msg}"
        );
    }

    #[test]
    fn invalid_transition_error_contains_both_states() {
        for (from, to) in invalid_transitions() {
            let t = StateTransition::new(from, to, "x");
            if let Err(err) = t.validate() {
                let msg = err.to_string().to_lowercase();
                let from_str = format!("{from:?}").to_lowercase();
                let to_str = format!("{to:?}").to_lowercase();
                assert!(
                    msg.contains(&from_str),
                    "error for {from:?}->{to:?} should mention source: {msg}"
                );
                assert!(
                    msg.contains(&to_str),
                    "error for {from:?}->{to:?} should mention target: {msg}"
                );
            }
        }
    }

    #[test]
    fn invalid_count_is_35() {
        assert_eq!(
            invalid_transitions().len(),
            35,
            "7 states x 7 states = 49 total, minus 14 valid = 35 invalid"
        );
    }

    // ========================================================================
    // 3. Same-State Transitions (X -> X is always invalid)
    // ========================================================================

    #[test]
    fn same_state_transitions_are_invalid() {
        for &state in SessionState::all_states() {
            assert!(
                !state.can_transition_to(state),
                "{state:?} -> {state:?} should be invalid"
            );
            let t = StateTransition::new(state, state, "self");
            assert!(
                t.validate().is_err(),
                "{state:?} -> {state:?} should fail validation"
            );
        }
    }

    // ========================================================================
    // 4. Transition Metadata — timestamp and reason
    // ========================================================================

    #[test]
    fn transition_timestamp_is_set_to_now() {
        let before = Utc::now();
        let t = StateTransition::new(SessionState::Created, SessionState::Active, "reason");
        let after = Utc::now();

        assert!(t.timestamp >= before, "timestamp should be >= before");
        assert!(t.timestamp <= after, "timestamp should be <= after");
    }

    #[test]
    fn transition_reason_is_preserved() {
        let t = StateTransition::new(SessionState::Created, SessionState::Active, "test reason");
        assert_eq!(t.reason, "test reason");
    }

    #[test]
    fn transition_reason_accepts_empty_string() {
        let t = StateTransition::new(SessionState::Created, SessionState::Active, "");
        assert_eq!(t.reason, "");
    }

    #[test]
    fn transition_from_and_to_are_correct() {
        let t = StateTransition::new(SessionState::Active, SessionState::Syncing, "sync");
        assert_eq!(t.from, SessionState::Active);
        assert_eq!(t.to, SessionState::Syncing);
    }

    #[test]
    fn transition_is_cloneable() {
        let t = StateTransition::new(SessionState::Created, SessionState::Active, "clone me");
        let cloned = t.clone();
        assert_eq!(cloned.from, t.from);
        assert_eq!(cloned.to, t.to);
        assert_eq!(cloned.timestamp, t.timestamp);
        assert_eq!(cloned.reason, t.reason);
    }

    #[test]
    fn transition_is_serializable() {
        let t = StateTransition::new(SessionState::Active, SessionState::Paused, "pause");
        let json = serde_json::to_string(&t).expect("should serialize");
        let back: StateTransition = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(back.from, t.from);
        assert_eq!(back.to, t.to);
        assert_eq!(back.reason, t.reason);
    }

    // ========================================================================
    // 5. Transition Recording / History
    // ========================================================================

    #[test]
    fn manager_starts_with_empty_history() {
        let mgr = SessionStateManager::new("s1");
        assert!(mgr.history().is_empty());
    }

    #[test]
    fn activate_records_history() {
        let mgr = SessionStateManager::new("s1");
        let mgr = mgr.activate("go").expect("should activate");
        assert_eq!(mgr.history().len(), 1);
        assert_eq!(mgr.history()[0].from, SessionState::Created);
        assert_eq!(mgr.history()[0].to, SessionState::Active);
        assert_eq!(mgr.history()[0].reason, "go");
    }

    #[test]
    fn full_lifecycle_records_all_transitions() {
        let mgr = SessionStateManager::new("lifecycle");
        let mgr = mgr.activate("activate").expect("activate");
        let mgr = mgr.sync("sync").expect("sync");
        let mgr = mgr.sync_complete("done").expect("sync_complete");
        let mgr = mgr.pause("pause").expect("pause");
        let mgr = mgr.resume("resume").expect("resume");
        let mgr = mgr.complete("done").expect("complete");

        assert_eq!(mgr.history().len(), 6);

        let expected = [
            (SessionState::Created, SessionState::Active),
            (SessionState::Active, SessionState::Syncing),
            (SessionState::Syncing, SessionState::Synced),
            (SessionState::Synced, SessionState::Paused),
            (SessionState::Paused, SessionState::Active),
            (SessionState::Active, SessionState::Completed),
        ];
        for (i, (from, to)) in expected.iter().enumerate() {
            assert_eq!(mgr.history()[i].from, *from, "step {i} from");
            assert_eq!(mgr.history()[i].to, *to, "step {i} to");
        }
    }

    #[test]
    fn history_timestamps_are_monotonically_non_decreasing() {
        let mgr = SessionStateManager::new("ts-test");
        let mgr = mgr.activate("a").expect("activate");
        let mgr = mgr.sync("s").expect("sync");
        let mgr = mgr.sync_complete("sc").expect("sync_complete");

        for window in mgr.history().windows(2) {
            assert!(
                window[0].timestamp <= window[1].timestamp,
                "timestamps should be non-decreasing"
            );
        }
    }

    #[test]
    fn manager_tracks_current_state_through_transitions() {
        let mgr = SessionStateManager::new("state-track");
        assert_eq!(mgr.current_state(), SessionState::Created);

        let mgr = mgr.activate("go").expect("activate");
        assert_eq!(mgr.current_state(), SessionState::Active);

        let mgr = mgr.pause("wait").expect("pause");
        assert_eq!(mgr.current_state(), SessionState::Paused);

        let mgr = mgr.resume("back").expect("resume");
        assert_eq!(mgr.current_state(), SessionState::Active);
    }

    #[test]
    fn metadata_survives_transitions() {
        let mgr = SessionStateManager::new("meta");
        let mut mgr = mgr.activate("go").expect("activate");
        mgr.set_metadata("key1", "value1");
        let mgr = mgr.pause("pause").expect("pause");

        assert_eq!(mgr.metadata().get("key1"), Some(&"value1".to_string()));
    }

    // ========================================================================
    // 6. Rollback Transitions (restart/retry)
    // ========================================================================

    #[test]
    fn restart_from_completed_goes_to_created() {
        let mgr = SessionStateManager::new("restart");
        let mgr = mgr.activate("go").expect("activate");
        let mgr = mgr.complete("done").expect("complete");
        assert_eq!(mgr.current_state(), SessionState::Completed);

        let mgr = mgr.restart("restart").expect("restart");
        assert_eq!(mgr.current_state(), SessionState::Created);
    }

    #[test]
    fn retry_from_failed_goes_to_created() {
        let mgr = SessionStateManager::new("retry");
        let mgr = mgr.fail("error").expect("fail");
        assert_eq!(mgr.current_state(), SessionState::Failed);

        let mgr = mgr.retry("retry").expect("retry");
        assert_eq!(mgr.current_state(), SessionState::Created);
    }

    #[test]
    fn restart_preserves_history() {
        let mgr = SessionStateManager::new("restart-hist");
        let mgr = mgr.activate("go").expect("activate");
        let mgr = mgr.complete("done").expect("complete");
        let mgr = mgr.restart("restart").expect("restart");

        assert_eq!(mgr.history().len(), 3);
        assert_eq!(mgr.history()[2].from, SessionState::Completed);
        assert_eq!(mgr.history()[2].to, SessionState::Created);
    }

    #[test]
    fn retry_preserves_history() {
        let mgr = SessionStateManager::new("retry-hist");
        let mgr = mgr.fail("error").expect("fail");
        let mgr = mgr.retry("retry").expect("retry");

        assert_eq!(mgr.history().len(), 2);
        assert_eq!(mgr.history()[1].from, SessionState::Failed);
        assert_eq!(mgr.history()[1].to, SessionState::Created);
    }

    #[test]
    fn full_cycle_with_restart() {
        let mgr = SessionStateManager::new("cycle");
        let mgr = mgr.activate("go").expect("activate");
        let mgr = mgr.complete("done").expect("complete");
        let mgr = mgr.restart("restart").expect("restart");
        let mgr = mgr.activate("go2").expect("activate2");
        let mgr = mgr.complete("done2").expect("complete2");

        assert_eq!(mgr.history().len(), 5);
        assert_eq!(mgr.current_state(), SessionState::Completed);
    }

    #[test]
    fn full_cycle_with_retry() {
        let mgr = SessionStateManager::new("cycle");
        let mgr = mgr.fail("error").expect("fail");
        let mgr = mgr.retry("retry").expect("retry");
        let mgr = mgr.activate("go").expect("activate");
        let mgr = mgr.complete("done").expect("complete");

        assert_eq!(mgr.history().len(), 4);
        assert_eq!(mgr.current_state(), SessionState::Completed);
    }

    // ========================================================================
    // 7. State Query Predicates — is_active, is_terminal
    // ========================================================================

    #[test]
    fn active_state_is_active() {
        assert!(SessionState::Active.is_active());
    }

    #[test]
    fn syncing_state_is_active() {
        assert!(SessionState::Syncing.is_active());
    }

    #[test]
    fn synced_state_is_active() {
        assert!(SessionState::Synced.is_active());
    }

    #[test]
    fn created_state_is_not_active() {
        assert!(!SessionState::Created.is_active());
    }

    #[test]
    fn paused_state_is_not_active() {
        assert!(!SessionState::Paused.is_active());
    }

    #[test]
    fn completed_state_is_not_active() {
        assert!(!SessionState::Completed.is_active());
    }

    #[test]
    fn failed_state_is_not_active() {
        assert!(!SessionState::Failed.is_active());
    }

    #[test]
    fn is_active_and_is_terminal_are_disjoint() {
        // No state should be both active and terminal
        for &state in SessionState::all_states() {
            assert!(
                !(state.is_active() && state.is_terminal()),
                "{state:?} should not be both active and terminal"
            );
        }
    }

    #[test]
    fn is_active_or_is_terminal_covers_all_states() {
        // Every state is either active, terminal, or dormant (Created/Paused)
        for &state in SessionState::all_states() {
            let active = state.is_active();
            let terminal = state.is_terminal();
            assert!(
                active || terminal || matches!(state, SessionState::Created | SessionState::Paused),
                "{state:?} should be active, terminal, or dormant"
            );
        }
    }

    // ========================================================================
    // 7b. Terminal State Invariants
    // ========================================================================

    #[test]
    fn completed_is_terminal() {
        assert!(SessionState::Completed.is_terminal());
    }

    #[test]
    fn failed_is_terminal() {
        assert!(SessionState::Failed.is_terminal());
    }

    #[test]
    fn non_terminal_states() {
        for &state in &[
            SessionState::Created,
            SessionState::Active,
            SessionState::Syncing,
            SessionState::Synced,
            SessionState::Paused,
        ] {
            assert!(!state.is_terminal(), "{state:?} should not be terminal");
        }
    }

    #[test]
    fn terminal_states_can_only_restart_or_retry() {
        // Completed can only go to Created
        let next = SessionState::Completed.valid_next_states();
        assert_eq!(next, vec![SessionState::Created]);

        // Failed can only go to Created
        let next = SessionState::Failed.valid_next_states();
        assert_eq!(next, vec![SessionState::Created]);
    }

    #[test]
    fn all_states_list_has_7_variants() {
        assert_eq!(SessionState::all_states().len(), 7);
    }

    #[test]
    fn all_states_are_distinct() {
        let states = SessionState::all_states();
        for i in 0..states.len() {
            for j in (i + 1)..states.len() {
                assert_ne!(states[i], states[j], "duplicate state at {i} and {j}");
            }
        }
    }

    // ========================================================================
    // 8. SessionState serialization
    // ========================================================================

    #[test]
    fn session_state_serializes_to_lowercase() {
        let json = serde_json::to_string(&SessionState::Active).expect("serialize");
        assert_eq!(json, "\"active\"");
    }

    #[test]
    fn all_session_states_roundtrip() {
        for &state in SessionState::all_states() {
            let json = serde_json::to_string(&state).expect("serialize");
            let back: SessionState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(state, back, "roundtrip for {state:?}");
        }
    }

    // ========================================================================
    // 9. Proptests
    // ========================================================================

    prop_compose! {
        fn arb_session_state()(idx in 0..7usize) -> SessionState {
            SessionState::all_states()[idx]
        }
    }

    proptest! {
        #[test]
        fn prop_can_transition_agrees_with_validate(
            from in arb_session_state(),
            to in arb_session_state()
        ) {
            let t = StateTransition::new(from, to, "prop");
            let can = from.can_transition_to(to);
            let valid = t.validate().is_ok();
            prop_assert_eq!(
                can, valid,
                "can_transition_to({:?}->{:?}) = {}, validate = {}",
                from, to, can, valid
            );
        }

        #[test]
        fn prop_valid_transitions_never_fail_validate(
            from in arb_session_state(),
            to in arb_session_state()
        ) {
            if from.can_transition_to(to) {
                let t = StateTransition::new(from, to, "valid");
                prop_assert!(t.validate().is_ok(), "{from:?}->{to:?} should validate");
            }
        }

        #[test]
        fn prop_invalid_transitions_always_fail_validate(
            from in arb_session_state(),
            to in arb_session_state()
        ) {
            if !from.can_transition_to(to) {
                let t = StateTransition::new(from, to, "invalid");
                prop_assert!(t.validate().is_err(), "{from:?}->{to:?} should fail");
            }
        }

        #[test]
        fn prop_terminal_states_reject_most_transitions(
            to in arb_session_state()
        ) {
            for &terminal in &[SessionState::Completed, SessionState::Failed] {
                if to == SessionState::Created {
                    prop_assert!(terminal.can_transition_to(to));
                } else {
                    prop_assert!(!terminal.can_transition_to(to));
                }
            }
        }

        #[test]
        fn prop_valid_next_states_subset_of_all_states(
            from in arb_session_state()
        ) {
            let next = from.valid_next_states();
            for &s in &next {
                prop_assert!(SessionState::all_states().contains(&s));
            }
        }

        #[test]
        fn prop_transition_preserves_reason(
            from in arb_session_state(),
            to in arb_session_state(),
            reason in ".*"
        ) {
            let t = StateTransition::new(from, to, &reason);
            prop_assert_eq!(t.reason, reason);
        }

        #[test]
        fn prop_transition_fields_match_constructor(
            from in arb_session_state(),
            to in arb_session_state(),
            reason in ".*"
        ) {
            let t = StateTransition::new(from, to, &reason);
            prop_assert_eq!(t.from, from);
            prop_assert_eq!(t.to, to);
        }

        #[test]
        fn prop_is_active_never_true_for_terminal_states(
            state in arb_session_state()
        ) {
            if state.is_terminal() {
                prop_assert!(!state.is_active());
            }
        }

        #[test]
        fn prop_valid_transitions_never_cross_terminal_to_active(
            from in arb_session_state(),
            to in arb_session_state()
        ) {
            if from.is_terminal() && from.can_transition_to(to) {
                // Terminal states can only go to Created
                prop_assert_eq!(to, SessionState::Created);
            }
        }
    }
}
