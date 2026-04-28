//! Bead state enumeration and state machine logic.
//!
//! Lifecycle: Open → InProgress → Blocked/Deferred → Closed

use serde::{Deserialize, Serialize};

/// Bead state enumeration.
///
/// Lifecycle: Open → InProgress → Blocked → Deferred → Closed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum BeadState {
    /// Bead is open and available to be worked on
    #[default]
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

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[test]
    fn bead_state_display_open() {
        assert_eq!(format!("{}", BeadState::Open), "open");
    }

    #[test]
    fn bead_state_display_in_progress() {
        assert_eq!(format!("{}", BeadState::InProgress), "in_progress");
    }

    #[test]
    fn bead_state_display_blocked() {
        assert_eq!(format!("{}", BeadState::Blocked), "blocked");
    }

    #[test]
    fn bead_state_display_deferred() {
        assert_eq!(format!("{}", BeadState::Deferred), "deferred");
    }

    #[test]
    fn bead_state_display_closed() {
        assert_eq!(format!("{}", BeadState::Closed), "closed");
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn bead_state_serde_roundtrip_all_variants() {
        let states = BeadState::all();
        for state in states {
            let json = serde_json::to_string(&state).expect("serialize");
            let parsed: BeadState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(state, parsed, "Roundtrip failed for {:?}", state);
        }
    }

    #[test]
    fn bead_state_serde_json_output() {
        assert_eq!(
            serde_json::to_string(&BeadState::Open).expect("serialize"),
            "\"Open\""
        );
        assert_eq!(
            serde_json::to_string(&BeadState::Closed).expect("serialize"),
            "\"Closed\""
        );
    }

    // =========================================================================
    // all() and is_terminal Additional Tests
    // =========================================================================

    #[test]
    fn bead_state_all_returns_five_variants() {
        assert_eq!(BeadState::all().len(), 5);
    }

    #[test]
    fn bead_state_all_contains_all_variants() {
        let all = BeadState::all();
        assert!(all.contains(&BeadState::Open));
        assert!(all.contains(&BeadState::InProgress));
        assert!(all.contains(&BeadState::Blocked));
        assert!(all.contains(&BeadState::Deferred));
        assert!(all.contains(&BeadState::Closed));
    }

    #[test]
    fn bead_state_only_closed_is_terminal() {
        for state in BeadState::all() {
            assert_eq!(
                state.is_terminal(),
                state == BeadState::Closed,
                "Only Closed should be terminal, got {:?}",
                state
            );
        }
    }

    #[test]
    fn bead_state_blocked_to_deferred_is_valid() {
        assert!(BeadState::Blocked.can_transition_to(BeadState::Deferred));
    }

    #[test]
    fn bead_state_deferred_to_blocked_is_invalid() {
        assert!(!BeadState::Deferred.can_transition_to(BeadState::Blocked));
    }

    #[test]
    fn bead_state_deferred_to_closed_is_valid() {
        assert!(BeadState::Deferred.can_transition_to(BeadState::Closed));
    }

    #[test]
    fn bead_state_blocked_to_closed_is_valid() {
        assert!(BeadState::Blocked.can_transition_to(BeadState::Closed));
    }

    #[test]
    fn bead_state_valid_transitions_from_blocked() {
        let transitions = BeadState::Blocked.valid_transitions();
        assert_eq!(transitions.len(), 3);
        assert!(transitions.contains(&BeadState::InProgress));
        assert!(transitions.contains(&BeadState::Deferred));
        assert!(transitions.contains(&BeadState::Closed));
    }

    #[test]
    fn bead_state_valid_transitions_from_deferred() {
        let transitions = BeadState::Deferred.valid_transitions();
        assert_eq!(transitions.len(), 2);
        assert!(transitions.contains(&BeadState::InProgress));
        assert!(transitions.contains(&BeadState::Closed));
    }

    // =========================================================================
    // BeadState Proptests
    // =========================================================================

    mod bead_state_proptests {
        use proptest::{prop_assert, prop_assert_eq, proptest};

        use super::*;

        proptest! {
            /// can_transition_to is reflexive-free: no state can transition to itself
            #[test]
            fn prop_no_self_transitions(state_idx in 0u8..5u8) {
                let states = BeadState::all();
                let state = states[state_idx as usize];
                prop_assert!(!state.can_transition_to(state));
            }

            /// Closed state cannot transition to any state
            #[test]
            fn prop_closed_no_transitions(target_idx in 0u8..5u8) {
                let states = BeadState::all();
                let target = states[target_idx as usize];
                prop_assert!(!BeadState::Closed.can_transition_to(target));
            }

            /// Terminal states have empty valid_transitions
            #[test]
            fn prop_terminal_empty_transitions(state_idx in 0u8..5u8) {
                let states = BeadState::all();
                let state = states[state_idx as usize];
                if state.is_terminal() {
                    prop_assert!(state.valid_transitions().is_empty());
                }
            }

            /// can_transition_to matches valid_transitions containment
            #[test]
            fn prop_can_transition_matches_valid_transitions(
                from_idx in 0u8..5u8,
                to_idx in 0u8..5u8
            ) {
                let states = BeadState::all();
                let from = states[from_idx as usize];
                let to = states[to_idx as usize];
                let can = from.can_transition_to(to);
                let in_valid = from.valid_transitions().contains(&to);
                prop_assert_eq!(can, in_valid);
            }

            /// Display is always lowercase ascii
            #[test]
            fn prop_display_is_lowercase_ascii(state_idx in 0u8..5u8) {
                let states = BeadState::all();
                let state = states[state_idx as usize];
                let display = format!("{state}");
                prop_assert!(!display.is_empty());
                prop_assert!(display.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
            }

            /// Serde roundtrip preserves equality
            #[test]
            fn prop_serde_roundtrip(state_idx in 0u8..5u8) {
                let states = BeadState::all();
                let state = states[state_idx as usize];
                let json = serde_json::to_string(&state).unwrap();
                let parsed: BeadState = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(state, parsed);
            }

            /// all() contains every display-able variant
            #[test]
            fn prop_all_has_five_variants(_ in 0u8..1u8) {
                let all = BeadState::all();
                prop_assert_eq!(all.len(), 5);
                // Each variant must be unique
                let mut seen = std::collections::HashSet::new();
                for state in all {
                    prop_assert!(seen.insert(state));
                }
            }
        }
    }
}
