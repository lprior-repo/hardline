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

use crate::error::{Error, Result};

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
            Err(Error::SessionInvalidState(
                format!("{:?}", self.from),
                format!("{:?}", self.to),
                "Invalid state transition".to_string(),
            ))
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

    #[test]
    fn test_session_state_manager_type_exists() {
        let manager = SessionStateManager::new("test-session");
        assert_eq!(manager.session_id(), "test-session");
        assert_eq!(manager.current_state(), SessionState::Created);
    }

    #[test]
    fn test_state_transition_created_to_active() {
        let transition =
            StateTransition::new(SessionState::Created, SessionState::Active, "activation");
        assert_eq!(transition.from, SessionState::Created);
        assert_eq!(transition.to, SessionState::Active);
    }

    #[test]
    fn test_state_validation_prevents_invalid_created_to_paused() {
        let transition =
            StateTransition::new(SessionState::Created, SessionState::Paused, "invalid");
        assert!(transition.validate().is_err());
    }

    #[test]
    fn test_state_validation_allows_valid_created_to_active() {
        let transition = StateTransition::new(SessionState::Created, SessionState::Active, "valid");
        assert!(transition.validate().is_ok());
    }
}
