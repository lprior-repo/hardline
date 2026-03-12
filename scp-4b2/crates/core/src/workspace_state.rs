//! Workspace Lifecycle State Machine
//!
//! Provides a type-safe state machine for workspace lifecycle management:
//! - `WorkspaceState` enum for runtime state representation
//! - Valid state transitions with exhaustive pattern matching
//! - Atomic state transition support for concurrent agents
//! - Railway-Oriented error handling with zero panics

use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// WORKSPACE STATE ENUM
// ═══════════════════════════════════════════════════════════════════════════

/// Workspace lifecycle states for parallel agent coordination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceState {
    /// Workspace created, not yet actively worked on
    #[default]
    Created,
    /// Actively being worked on by an agent
    Working,
    /// Work complete, ready for merge review
    Ready,
    /// Successfully merged to main branch
    Merged,
    /// Manually abandoned by agent
    Abandoned,
    /// Merge conflict detected, needs resolution
    Conflict,
}

impl WorkspaceState {
    /// Returns all valid next states from current state.
    #[must_use]
    pub fn valid_next_states(self) -> Vec<Self> {
        match self {
            Self::Created => vec![Self::Working],
            Self::Working => vec![Self::Ready, Self::Conflict, Self::Abandoned],
            Self::Ready => vec![Self::Working, Self::Merged, Self::Conflict, Self::Abandoned],
            Self::Conflict => vec![Self::Working, Self::Abandoned],
            Self::Merged | Self::Abandoned => vec![],
        }
    }

    /// Returns true if this state can transition to the next state.
    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self.valid_next_states().contains(&next)
    }

    /// Returns true if this is a terminal state (no further transitions possible).
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Merged | Self::Abandoned)
    }

    /// Returns true if this state indicates active work is happening.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Working | Self::Conflict)
    }

    /// Returns true if this state indicates work is complete (ready or merged).
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Ready | Self::Merged)
    }

    /// Returns all possible workspace states as a slice.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Created,
            Self::Working,
            Self::Ready,
            Self::Merged,
            Self::Abandoned,
            Self::Conflict,
        ]
    }
}

impl fmt::Display for WorkspaceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Working => write!(f, "working"),
            Self::Ready => write!(f, "ready"),
            Self::Merged => write!(f, "merged"),
            Self::Abandoned => write!(f, "abandoned"),
            Self::Conflict => write!(f, "conflict"),
        }
    }
}

impl FromStr for WorkspaceState {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "created" => Ok(Self::Created),
            "working" => Ok(Self::Working),
            "ready" => Ok(Self::Ready),
            "merged" => Ok(Self::Merged),
            "abandoned" => Ok(Self::Abandoned),
            "conflict" => Ok(Self::Conflict),
            _ => Err(Error::WorkspaceConflict(format!(
                "Invalid workspace state: '{}'. Valid states: created, working, ready, merged, abandoned, conflict",
                s
            ))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE TRANSITION
// ═══════════════════════════════════════════════════════════════════════════

/// A workspace state transition event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStateTransition {
    /// Source state
    pub from: WorkspaceState,
    /// Target state
    pub to: WorkspaceState,
    /// Timestamp of transition (UTC)
    pub timestamp: DateTime<Utc>,
    /// Reason for transition (human-readable)
    pub reason: String,
    /// Agent ID that performed the transition (for audit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

impl WorkspaceStateTransition {
    /// Create a new state transition
    #[must_use]
    pub fn new(from: WorkspaceState, to: WorkspaceState, reason: impl Into<String>) -> Self {
        Self {
            from,
            to,
            timestamp: Utc::now(),
            reason: reason.into(),
            agent_id: None,
        }
    }

    /// Create a new state transition with agent ID
    #[must_use]
    pub fn with_agent(
        from: WorkspaceState,
        to: WorkspaceState,
        reason: impl Into<String>,
        agent_id: impl Into<String>,
    ) -> Self {
        Self {
            from,
            to,
            timestamp: Utc::now(),
            reason: reason.into(),
            agent_id: Some(agent_id.into()),
        }
    }

    /// Validate that the transition is allowed
    pub fn validate(&self) -> Result<()> {
        if self.from.can_transition_to(self.to) {
            Ok(())
        } else {
            Err(Error::SessionInvalidState(
                self.from.to_string(),
                self.to.to_string(),
                "Invalid workspace state transition".to_string(),
            ))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE QUERY HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Filter predicate for workspace states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceStateFilter {
    /// Match a specific state
    State(WorkspaceState),
    /// Match any active state (Working, Conflict)
    Active,
    /// Match any complete state (Ready, Merged)
    Complete,
    /// Match any terminal state (Merged, Abandoned)
    Terminal,
    /// Match any non-terminal state
    NonTerminal,
    /// Match all states
    All,
}

impl WorkspaceStateFilter {
    /// Check if a workspace state matches this filter
    #[must_use]
    pub fn matches(self, state: WorkspaceState) -> bool {
        match self {
            Self::State(s) => state == s,
            Self::Active => state.is_active(),
            Self::Complete => state.is_complete(),
            Self::Terminal => state.is_terminal(),
            Self::NonTerminal => !state.is_terminal(),
            Self::All => true,
        }
    }
}

impl FromStr for WorkspaceStateFilter {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "all" => Ok(Self::All),
            "active" => Ok(Self::Active),
            "complete" => Ok(Self::Complete),
            "terminal" => Ok(Self::Terminal),
            "non-terminal" | "nonterminal" => Ok(Self::NonTerminal),
            _ => WorkspaceState::from_str(s).map(Self::State),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_valid_transitions_succeed() {
        assert!(WorkspaceState::Created.can_transition_to(WorkspaceState::Working));
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Ready));
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Conflict));
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Abandoned));
    }

    #[test]
    fn test_invalid_transition_returns_error() {
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Ready));
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Merged));
    }

    #[test]
    fn test_terminal_states_reject_transitions() {
        assert!(WorkspaceState::Merged.is_terminal());
        assert!(WorkspaceState::Abandoned.is_terminal());
        assert!(WorkspaceState::Merged.valid_next_states().is_empty());
    }

    #[test]
    fn test_state_display() {
        assert_eq!(WorkspaceState::Created.to_string(), "created");
        assert_eq!(WorkspaceState::Working.to_string(), "working");
    }

    #[test]
    fn test_state_from_str() {
        assert_eq!(
            WorkspaceState::from_str("created").ok(),
            Some(WorkspaceState::Created)
        );
    }

    #[test]
    fn test_transition_validate_valid() {
        let transition = WorkspaceStateTransition::new(
            WorkspaceState::Created,
            WorkspaceState::Working,
            "start work",
        );
        assert!(transition.validate().is_ok());
    }

    #[test]
    fn test_transition_validate_invalid() {
        let transition = WorkspaceStateTransition::new(
            WorkspaceState::Created,
            WorkspaceState::Merged,
            "skip everything",
        );
        assert!(transition.validate().is_err());
    }
}
