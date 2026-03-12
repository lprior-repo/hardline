//! Agent domain types
//!
//! Provides types for agent state and operations.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::domain::identifiers::AgentId;

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
    pub const fn with_last_seen(mut self, last_seen: chrono::DateTime<chrono::Utc>) -> Self {
        self.last_seen = Some(last_seen);
        self
    }
}
