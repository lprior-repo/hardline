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
            _ => Err(Error::workspace_conflict(format!(
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
            Err(Error::invalid_state(format!(
                "Invalid state transition: '{}' -> '{}'",
                self.from, self.to
            )))
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

    // ── WorkspaceState construction & enum variants ──────────────────────────

    #[test]
    fn test_all_enum_variants_exist() {
        let states = WorkspaceState::all();
        assert_eq!(states.len(), 6);
        assert!(states.contains(&WorkspaceState::Created));
        assert!(states.contains(&WorkspaceState::Working));
        assert!(states.contains(&WorkspaceState::Ready));
        assert!(states.contains(&WorkspaceState::Merged));
        assert!(states.contains(&WorkspaceState::Abandoned));
        assert!(states.contains(&WorkspaceState::Conflict));
    }

    #[test]
    fn test_default_is_created() {
        assert_eq!(WorkspaceState::default(), WorkspaceState::Created);
    }

    #[test]
    fn test_all_variants_are_distinct() {
        let mut set = std::collections::HashSet::new();
        for state in WorkspaceState::all() {
            assert!(set.insert(state), "Duplicate variant found: {state:?}");
        }
    }

    #[test]
    fn test_is_active() {
        assert!(!WorkspaceState::Created.is_active());
        assert!(WorkspaceState::Working.is_active());
        assert!(!WorkspaceState::Ready.is_active());
        assert!(!WorkspaceState::Merged.is_active());
        assert!(!WorkspaceState::Abandoned.is_active());
        assert!(WorkspaceState::Conflict.is_active());
    }

    #[test]
    fn test_is_complete() {
        assert!(!WorkspaceState::Created.is_complete());
        assert!(!WorkspaceState::Working.is_complete());
        assert!(WorkspaceState::Ready.is_complete());
        assert!(WorkspaceState::Merged.is_complete());
        assert!(!WorkspaceState::Abandoned.is_complete());
        assert!(!WorkspaceState::Conflict.is_complete());
    }

    #[test]
    fn test_is_terminal() {
        assert!(!WorkspaceState::Created.is_terminal());
        assert!(!WorkspaceState::Working.is_terminal());
        assert!(!WorkspaceState::Ready.is_terminal());
        assert!(WorkspaceState::Merged.is_terminal());
        assert!(WorkspaceState::Abandoned.is_terminal());
        assert!(!WorkspaceState::Conflict.is_terminal());
    }

    // ── Display ──────────────────────────────────────────────────────────────

    #[test]
    fn test_state_display_all_variants() {
        assert_eq!(WorkspaceState::Created.to_string(), "created");
        assert_eq!(WorkspaceState::Working.to_string(), "working");
        assert_eq!(WorkspaceState::Ready.to_string(), "ready");
        assert_eq!(WorkspaceState::Merged.to_string(), "merged");
        assert_eq!(WorkspaceState::Abandoned.to_string(), "abandoned");
        assert_eq!(WorkspaceState::Conflict.to_string(), "conflict");
    }

    // ── FromStr ──────────────────────────────────────────────────────────────

    #[test]
    fn test_state_from_str_all_valid() {
        assert_eq!(
            WorkspaceState::from_str("created").ok(),
            Some(WorkspaceState::Created)
        );
        assert_eq!(
            WorkspaceState::from_str("working").ok(),
            Some(WorkspaceState::Working)
        );
        assert_eq!(
            WorkspaceState::from_str("ready").ok(),
            Some(WorkspaceState::Ready)
        );
        assert_eq!(
            WorkspaceState::from_str("merged").ok(),
            Some(WorkspaceState::Merged)
        );
        assert_eq!(
            WorkspaceState::from_str("abandoned").ok(),
            Some(WorkspaceState::Abandoned)
        );
        assert_eq!(
            WorkspaceState::from_str("conflict").ok(),
            Some(WorkspaceState::Conflict)
        );
    }

    #[test]
    fn test_state_from_str_case_insensitive() {
        assert_eq!(
            WorkspaceState::from_str("CREATED").ok(),
            Some(WorkspaceState::Created)
        );
        assert_eq!(
            WorkspaceState::from_str("Working").ok(),
            Some(WorkspaceState::Working)
        );
        assert_eq!(
            WorkspaceState::from_str("MERGED").ok(),
            Some(WorkspaceState::Merged)
        );
    }

    #[test]
    fn test_state_from_str_invalid() {
        assert!(WorkspaceState::from_str("invalid").is_err());
        assert!(WorkspaceState::from_str("").is_err());
        assert!(WorkspaceState::from_str("123").is_err());
    }

    // ── Valid transitions ────────────────────────────────────────────────────

    #[test]
    fn test_all_valid_transitions_succeed() {
        assert!(WorkspaceState::Created.can_transition_to(WorkspaceState::Working));
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Ready));
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Conflict));
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Abandoned));
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Working));
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Merged));
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Conflict));
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Abandoned));
        assert!(WorkspaceState::Conflict.can_transition_to(WorkspaceState::Working));
        assert!(WorkspaceState::Conflict.can_transition_to(WorkspaceState::Abandoned));
    }

    #[test]
    fn test_invalid_transition_returns_error() {
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Ready));
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Merged));
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Abandoned));
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Conflict));
    }

    #[test]
    fn test_terminal_states_reject_transitions() {
        assert!(WorkspaceState::Merged.is_terminal());
        assert!(WorkspaceState::Abandoned.is_terminal());
        assert!(WorkspaceState::Merged.valid_next_states().is_empty());
        assert!(WorkspaceState::Abandoned.valid_next_states().is_empty());

        // No state can transition to itself
        for state in WorkspaceState::all() {
            assert!(!state.can_transition_to(*state));
        }
    }

    #[test]
    fn test_valid_next_states_completeness() {
        // Every non-terminal state should have at least one valid transition
        for state in WorkspaceState::all() {
            if !state.is_terminal() {
                assert!(
                    !state.valid_next_states().is_empty(),
                    "Non-terminal state {state:?} should have valid transitions"
                );
            }
        }
    }

    // ── WorkspaceStateTransition construction ────────────────────────────────

    #[test]
    fn test_transition_new() {
        let transition = WorkspaceStateTransition::new(
            WorkspaceState::Created,
            WorkspaceState::Working,
            "start work",
        );
        assert_eq!(transition.from, WorkspaceState::Created);
        assert_eq!(transition.to, WorkspaceState::Working);
        assert_eq!(transition.reason, "start work");
        assert!(transition.agent_id.is_none());
    }

    #[test]
    fn test_transition_with_agent() {
        let transition = WorkspaceStateTransition::with_agent(
            WorkspaceState::Created,
            WorkspaceState::Working,
            "start work",
            "agent-42",
        );
        assert_eq!(transition.agent_id.as_deref(), Some("agent-42"));
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

    #[test]
    fn test_transition_validate_terminal_to_any() {
        // Terminal states can't transition to anything
        let t1 = WorkspaceStateTransition::new(
            WorkspaceState::Merged,
            WorkspaceState::Working,
            "re-open",
        );
        let t2 = WorkspaceStateTransition::new(
            WorkspaceState::Abandoned,
            WorkspaceState::Created,
            "restart",
        );
        assert!(t1.validate().is_err());
        assert!(t2.validate().is_err());
    }

    // ── WorkspaceStateFilter ─────────────────────────────────────────────────

    #[test]
    fn test_filter_specific_state() {
        let filter = WorkspaceStateFilter::State(WorkspaceState::Working);
        assert!(filter.matches(WorkspaceState::Working));
        assert!(!filter.matches(WorkspaceState::Created));
        assert!(!filter.matches(WorkspaceState::Ready));
    }

    #[test]
    fn test_filter_active() {
        let filter = WorkspaceStateFilter::Active;
        assert!(filter.matches(WorkspaceState::Working));
        assert!(filter.matches(WorkspaceState::Conflict));
        assert!(!filter.matches(WorkspaceState::Created));
        assert!(!filter.matches(WorkspaceState::Ready));
        assert!(!filter.matches(WorkspaceState::Merged));
        assert!(!filter.matches(WorkspaceState::Abandoned));
    }

    #[test]
    fn test_filter_complete() {
        let filter = WorkspaceStateFilter::Complete;
        assert!(!filter.matches(WorkspaceState::Created));
        assert!(!filter.matches(WorkspaceState::Working));
        assert!(filter.matches(WorkspaceState::Ready));
        assert!(filter.matches(WorkspaceState::Merged));
        assert!(!filter.matches(WorkspaceState::Abandoned));
        assert!(!filter.matches(WorkspaceState::Conflict));
    }

    #[test]
    fn test_filter_terminal() {
        let filter = WorkspaceStateFilter::Terminal;
        assert!(!filter.matches(WorkspaceState::Created));
        assert!(!filter.matches(WorkspaceState::Working));
        assert!(!filter.matches(WorkspaceState::Ready));
        assert!(filter.matches(WorkspaceState::Merged));
        assert!(filter.matches(WorkspaceState::Abandoned));
        assert!(!filter.matches(WorkspaceState::Conflict));
    }

    #[test]
    fn test_filter_non_terminal() {
        let filter = WorkspaceStateFilter::NonTerminal;
        assert!(filter.matches(WorkspaceState::Created));
        assert!(filter.matches(WorkspaceState::Working));
        assert!(filter.matches(WorkspaceState::Ready));
        assert!(!filter.matches(WorkspaceState::Merged));
        assert!(!filter.matches(WorkspaceState::Abandoned));
        assert!(filter.matches(WorkspaceState::Conflict));
    }

    #[test]
    fn test_filter_all() {
        let filter = WorkspaceStateFilter::All;
        for state in WorkspaceState::all() {
            assert!(filter.matches(*state));
        }
    }

    #[test]
    fn test_filter_from_str() {
        assert_eq!(
            WorkspaceStateFilter::from_str("all").ok(),
            Some(WorkspaceStateFilter::All)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("active").ok(),
            Some(WorkspaceStateFilter::Active)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("complete").ok(),
            Some(WorkspaceStateFilter::Complete)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("terminal").ok(),
            Some(WorkspaceStateFilter::Terminal)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("non-terminal").ok(),
            Some(WorkspaceStateFilter::NonTerminal)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("nonterminal").ok(),
            Some(WorkspaceStateFilter::NonTerminal)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("working").ok(),
            Some(WorkspaceStateFilter::State(WorkspaceState::Working))
        );
    }

    #[test]
    fn test_filter_matches_exhaustive() {
        // Every state matches exactly one of Active/Complete/Terminal or is NonTerminal
        for state in WorkspaceState::all() {
            if state.is_terminal() {
                assert!(WorkspaceStateFilter::Terminal.matches(*state));
                assert!(!WorkspaceStateFilter::NonTerminal.matches(*state));
            } else {
                assert!(!WorkspaceStateFilter::Terminal.matches(*state));
                assert!(WorkspaceStateFilter::NonTerminal.matches(*state));
            }
        }
    }

    // =========================================================================
    // Serde roundtrip tests
    // =========================================================================

    #[test]
    fn test_workspace_state_serde_roundtrip_all_variants() {
        for state in WorkspaceState::all() {
            let json = serde_json::to_string(&state).expect("serialize ok");
            let deserialized: WorkspaceState = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(*state, deserialized, "Roundtrip failed for {state:?}");
        }
    }

    #[test]
    fn test_workspace_state_serde_lowercase() {
        assert_eq!(serde_json::to_string(&WorkspaceState::Created).expect("ok"), "\"created\"");
        assert_eq!(serde_json::to_string(&WorkspaceState::Working).expect("ok"), "\"working\"");
        assert_eq!(serde_json::to_string(&WorkspaceState::Ready).expect("ok"), "\"ready\"");
        assert_eq!(serde_json::to_string(&WorkspaceState::Merged).expect("ok"), "\"merged\"");
        assert_eq!(serde_json::to_string(&WorkspaceState::Abandoned).expect("ok"), "\"abandoned\"");
    }

    #[test]
    fn test_workspace_state_transition_serde_roundtrip() {
        let transition = WorkspaceStateTransition {
            from: WorkspaceState::Created,
            to: WorkspaceState::Working,
            timestamp: Utc::now(),
            reason: "agent started work".to_string(),
            agent_id: Some("agent-1".to_string()),
        };
        let json = serde_json::to_string(&transition).expect("serialize ok");
        let deserialized: WorkspaceStateTransition = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(transition.from, deserialized.from);
        assert_eq!(transition.to, deserialized.to);
        assert_eq!(transition.reason, deserialized.reason);
    }

    #[test]
    fn test_workspace_state_transition_serde_with_none_optional() {
        let transition = WorkspaceStateTransition {
            from: WorkspaceState::Working,
            to: WorkspaceState::Ready,
            timestamp: Utc::now(),
            reason: "paused".to_string(),
            agent_id: None,
        };
        let json = serde_json::to_string(&transition).expect("serialize ok");
        let deserialized: WorkspaceStateTransition = serde_json::from_str(&json).expect("deserialize ok");
        assert!(deserialized.agent_id.is_none());
    }
}
