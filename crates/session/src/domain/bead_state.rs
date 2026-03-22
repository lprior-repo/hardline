//! Bead state enumeration and state machine logic.
//!
//! Lifecycle: Open → InProgress → Blocked/Deferred → Closed

use serde::{Deserialize, Serialize};

/// Bead state enumeration.
///
/// Lifecycle: Open → InProgress → Blocked → Deferred → Closed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BeadState {
    /// Bead is open and available to be worked on
    Open,
    /// Bead is actively being worked on
    InProgress,
    /// Bead is blocked by dependencies
    Blocked,
    /// Bead has been deferred
    Deferred,
    /// Bead is closed/done (terminal state)
    Closed,
}

impl BeadState {
    /// All possible bead states
    pub const fn all() -> [Self; 5] {
        [
            Self::Open,
            Self::InProgress,
            Self::Blocked,
            Self::Deferred,
            Self::Closed,
        ]
    }

    /// Check if this state is terminal (no further transitions possible)
    #[must_use]
    pub fn is_terminal(self) -> bool {
        self == Self::Closed
    }

    /// Check if a transition from this state to target is valid
    /// State machine: Open → InProgress → Blocked/Deferred → Closed
    #[must_use]
    pub const fn can_transition_to(self, target: Self) -> bool {
        match (self, target) {
            // Open can go to InProgress
            (Self::Open, Self::InProgress) => true,
            // InProgress can go to Blocked, Deferred, or Closed
            (Self::InProgress, Self::Blocked) => true,
            (Self::InProgress, Self::Deferred) => true,
            (Self::InProgress, Self::Closed) => true,
            // Blocked can go back to InProgress, Deferred, or Closed
            (Self::Blocked, Self::InProgress) => true,
            (Self::Blocked, Self::Deferred) => true,
            (Self::Blocked, Self::Closed) => true,
            // Deferred can go back to InProgress or Closed
            (Self::Deferred, Self::InProgress) => true,
            (Self::Deferred, Self::Closed) => true,
            // Closed is terminal - cannot transition to any other state
            (Self::Closed, _) => false,
            // Self-loops are not allowed
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

impl Default for BeadState {
    fn default() -> Self {
        Self::Open
    }
}

impl std::fmt::Display for BeadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Blocked => write!(f, "blocked"),
            Self::Deferred => write!(f, "deferred"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bead_state_default_is_open() {
        assert_eq!(BeadState::default(), BeadState::Open);
    }

    #[test]
    fn bead_state_is_terminal() {
        assert!(BeadState::Closed.is_terminal());
        assert!(!BeadState::Open.is_terminal());
        assert!(!BeadState::InProgress.is_terminal());
    }

    #[test]
    fn bead_state_transitions() {
        // Valid transitions
        assert!(BeadState::Open.can_transition_to(BeadState::InProgress));
        assert!(BeadState::InProgress.can_transition_to(BeadState::Blocked));
        assert!(BeadState::InProgress.can_transition_to(BeadState::Deferred));
        assert!(BeadState::InProgress.can_transition_to(BeadState::Closed));
        assert!(BeadState::Blocked.can_transition_to(BeadState::InProgress));
        assert!(BeadState::Deferred.can_transition_to(BeadState::InProgress));

        // Invalid transitions
        assert!(!BeadState::Open.can_transition_to(BeadState::Blocked));
        assert!(!BeadState::Closed.can_transition_to(BeadState::Open));
        assert!(!BeadState::Open.can_transition_to(BeadState::Open)); // No self-loops
    }

    #[test]
    fn bead_state_valid_transitions() {
        let from_open = BeadState::Open.valid_transitions();
        assert_eq!(from_open, vec![BeadState::InProgress]);

        let from_in_progress = BeadState::InProgress.valid_transitions();
        assert_eq!(
            from_in_progress,
            vec![BeadState::Blocked, BeadState::Deferred, BeadState::Closed]
        );

        let from_closed = BeadState::Closed.valid_transitions();
        assert!(from_closed.is_empty());
    }
}
