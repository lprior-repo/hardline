use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::error::{BeadError, Result};

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BeadState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed {
        closed_at: DateTime<Utc>,
    },
}

impl Default for BeadState {
    fn default() -> Self {
        Self::Open
    }
}

impl BeadState {
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Open | Self::InProgress)
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }

    #[must_use]
    pub fn closed_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Closed { closed_at } => Some(*closed_at),
            _ => None,
        }
    }

    pub fn transition_to(&self, new_state: Self) -> Result<Self> {
        let is_valid = match (self, &new_state) {
            (Self::Closed { .. }, _) => false,
            (Self::Open, Self::InProgress) => true,
            (Self::InProgress, Self::Blocked) => true,
            (Self::InProgress, Self::Deferred) => true,
            (Self::InProgress, Self::Closed { .. }) => true,
            (Self::Blocked, Self::InProgress) => true,
            (Self::Blocked, Self::Deferred) => true,
            (Self::Blocked, Self::Closed { .. }) => true,
            (Self::Deferred, Self::InProgress) => true,
            (Self::Deferred, Self::Closed { .. }) => true,
            (current, _) => {
                std::ptr::eq(current, &new_state) || *current == new_state
            }
        };

        if !is_valid {
            return Err(BeadError::InvalidStateTransition {
                from: format!("{self}"),
                to: format!("{new_state}"),
            });
        }

        match new_state {
            Self::Closed { .. } => Ok(Self::Closed {
                closed_at: Utc::now(),
            }),
            other => Ok(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_open() {
        assert_eq!(BeadState::default(), BeadState::Open);
    }

    #[test]
    fn open_is_active() {
        assert!(BeadState::Open.is_active());
    }

    #[test]
    fn in_progress_is_active() {
        assert!(BeadState::InProgress.is_active());
    }

    #[test]
    fn blocked_is_not_active() {
        assert!(!BeadState::Blocked.is_active());
    }

    #[test]
    fn deferred_is_not_active() {
        assert!(!BeadState::Deferred.is_active());
    }

    #[test]
    fn closed_is_not_active() {
        let closed = BeadState::Closed {
            closed_at: Utc::now(),
        };
        assert!(!closed.is_active());
    }

    #[test]
    fn is_blocked_only_for_blocked() {
        assert!(BeadState::Blocked.is_blocked());
        assert!(!BeadState::Open.is_blocked());
        assert!(!BeadState::InProgress.is_blocked());
        assert!(!BeadState::Deferred.is_blocked());
        let closed = BeadState::Closed {
            closed_at: Utc::now(),
        };
        assert!(!closed.is_blocked());
    }

    #[test]
    fn is_closed_only_for_closed_variant() {
        assert!(BeadState::Closed {
            closed_at: Utc::now()
        }
        .is_closed());
        assert!(!BeadState::Open.is_closed());
        assert!(!BeadState::InProgress.is_closed());
        assert!(!BeadState::Blocked.is_closed());
        assert!(!BeadState::Deferred.is_closed());
    }

    #[test]
    fn closed_at_returns_some_for_closed() {
        let now = Utc::now();
        let state = BeadState::Closed { closed_at: now };
        assert_eq!(state.closed_at(), Some(now));
    }

    #[test]
    fn closed_at_returns_none_for_non_closed() {
        assert_eq!(BeadState::Open.closed_at(), None);
        assert_eq!(BeadState::InProgress.closed_at(), None);
        assert_eq!(BeadState::Blocked.closed_at(), None);
        assert_eq!(BeadState::Deferred.closed_at(), None);
    }

    #[test]
    fn transition_to_closed_from_open_rejected() {
        let result = BeadState::Open.transition_to(BeadState::Closed {
            closed_at: Utc::now(),
        });
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::InvalidStateTransition { from, to } => {
                assert_eq!(from, "open");
                assert_eq!(to, "closed");
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[test]
    fn transition_to_same_state_succeeds() {
        let result = BeadState::Open.transition_to(BeadState::Open);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BeadState::Open);
    }

    #[test]
    fn display_open() {
        assert_eq!(format!("{}", BeadState::Open), "open");
    }

    #[test]
    fn display_in_progress() {
        assert_eq!(format!("{}", BeadState::InProgress), "inprogress");
    }

    #[test]
    fn display_blocked() {
        assert_eq!(format!("{}", BeadState::Blocked), "blocked");
    }

    #[test]
    fn display_deferred() {
        assert_eq!(format!("{}", BeadState::Deferred), "deferred");
    }

    #[test]
    fn display_closed() {
        let state = BeadState::Closed {
            closed_at: Utc::now(),
        };
        assert_eq!(format!("{state}"), "closed");
    }

    #[test]
    fn serde_roundtrip_open() {
        let json = serde_json::to_string(&BeadState::Open).unwrap();
        let parsed: BeadState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, BeadState::Open);
    }

    #[test]
    fn serde_roundtrip_in_progress() {
        let json = serde_json::to_string(&BeadState::InProgress).unwrap();
        let parsed: BeadState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, BeadState::InProgress);
    }

    #[test]
    fn serde_roundtrip_closed() {
        let state = BeadState::Closed {
            closed_at: Utc::now(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: BeadState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, parsed);
    }

    #[test]
    fn from_str_parses_open() {
        let state: BeadState = "open".parse().unwrap();
        assert_eq!(state, BeadState::Open);
    }

    #[test]
    fn from_str_parses_inprogress() {
        let state: BeadState = "inprogress".parse().unwrap();
        assert_eq!(state, BeadState::InProgress);
    }

    #[test]
    fn from_str_parses_blocked() {
        let state: BeadState = "blocked".parse().unwrap();
        assert_eq!(state, BeadState::Blocked);
    }

    #[test]
    fn from_str_parses_deferred() {
        let state: BeadState = "deferred".parse().unwrap();
        assert_eq!(state, BeadState::Deferred);
    }

    #[test]
    fn from_str_parses_closed() {
        let state: BeadState = "closed".parse().unwrap();
        assert!(state.is_closed());
    }

    #[test]
    fn from_str_rejects_invalid() {
        let result: std::result::Result<BeadState, _> = "invalid_state".parse();
        assert!(result.is_err());
    }

    #[test]
    fn serde_roundtrip_blocked() {
        let json = serde_json::to_string(&BeadState::Blocked).unwrap();
        let parsed: BeadState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, BeadState::Blocked);
    }

    #[test]
    fn serde_roundtrip_deferred() {
        let json = serde_json::to_string(&BeadState::Deferred).unwrap();
        let parsed: BeadState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, BeadState::Deferred);
    }

    #[test]
    fn closed_state_equality_with_different_timestamps() {
        let now = Utc::now();
        let state1 = BeadState::Closed { closed_at: now };
        let state2 = BeadState::Closed { closed_at: now };
        assert_eq!(state1, state2);
    }

    #[test]
    fn closed_state_inequality_with_different_timestamps() {
        let state1 = BeadState::Closed {
            closed_at: Utc::now(),
        };
        let state2 = BeadState::Closed {
            closed_at: Utc::now() + chrono::Duration::seconds(60),
        };
        assert_ne!(state1, state2);
    }

    #[test]
    fn hash_works() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BeadState::Open);
        assert!(set.contains(&BeadState::Open));
        assert!(!set.contains(&BeadState::Blocked));
    }

    #[test]
    fn transition_open_to_blocked_rejected() {
        let result = BeadState::Open.transition_to(BeadState::Blocked);
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::InvalidStateTransition { from, to } => {
                assert_eq!(from, "open");
                assert_eq!(to, "blocked");
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[test]
    fn transition_in_progress_to_open_rejected() {
        let result = BeadState::InProgress.transition_to(BeadState::Open);
        assert!(result.is_err());
    }

    #[test]
    fn transition_blocked_to_open_rejected() {
        let result = BeadState::Blocked.transition_to(BeadState::Open);
        assert!(result.is_err());
    }

    #[test]
    fn transition_deferred_to_blocked_rejected() {
        let result = BeadState::Deferred.transition_to(BeadState::Blocked);
        assert!(result.is_err());
    }

    #[test]
    fn transition_deferred_to_deferred_succeeds() {
        let result = BeadState::Deferred.transition_to(BeadState::Deferred);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BeadState::Deferred);
    }

    #[test]
    fn transition_blocked_to_blocked_succeeds() {
        let result = BeadState::Blocked.transition_to(BeadState::Blocked);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BeadState::Blocked);
    }

    #[test]
    fn transition_to_closed_from_in_progress_succeeds() {
        let result = BeadState::InProgress.transition_to(BeadState::Closed {
            closed_at: Utc::now(),
        });
        assert!(result.is_ok());
        let state = result.unwrap();
        assert!(state.is_closed());
        assert!(state.closed_at().is_some());
    }

    #[test]
    fn transition_to_closed_from_blocked_succeeds() {
        let result = BeadState::Blocked.transition_to(BeadState::Closed {
            closed_at: Utc::now(),
        });
        assert!(result.is_ok());
        assert!(result.unwrap().is_closed());
    }

    #[test]
    fn transition_to_closed_from_deferred_succeeds() {
        let result = BeadState::Deferred.transition_to(BeadState::Closed {
            closed_at: Utc::now(),
        });
        assert!(result.is_ok());
        assert!(result.unwrap().is_closed());
    }

    #[test]
    fn transition_from_closed_to_any_rejected() {
        let past = Utc::now() - chrono::Duration::days(1);
        let closed = BeadState::Closed { closed_at: past };
        assert!(closed.transition_to(BeadState::Open).is_err());
        assert!(closed.transition_to(BeadState::InProgress).is_err());
        assert!(closed.transition_to(BeadState::Blocked).is_err());
        assert!(closed.transition_to(BeadState::Deferred).is_err());
        assert!(closed
            .transition_to(BeadState::Closed {
                closed_at: Utc::now(),
            })
            .is_err());
    }

    #[test]
    fn chain_open_to_in_progress_to_closed() {
        let state = BeadState::Open;

        let in_progress = state
            .transition_to(BeadState::InProgress)
            .expect("Open→InProgress should succeed");
        assert_eq!(in_progress, BeadState::InProgress);
        assert!(in_progress.is_active());
        assert!(!in_progress.is_closed());

        let closed = in_progress
            .transition_to(BeadState::Closed {
                closed_at: Utc::now(),
            })
            .expect("InProgress→Closed should succeed");
        assert!(closed.is_closed());
        assert!(closed.closed_at().is_some());
        assert!(!closed.is_active());
    }

    #[test]
    fn chain_open_to_blocked_to_open() {
        let state = BeadState::Open;

        let blocked = state
            .transition_to(BeadState::Blocked)
            .expect("Open→Blocked should succeed");
        assert_eq!(blocked, BeadState::Blocked);
        assert!(blocked.is_blocked());
        assert!(!blocked.is_active());
        assert!(!blocked.is_closed());

        let back_to_open = blocked
            .transition_to(BeadState::Open)
            .expect("Blocked→Open should succeed");
        assert_eq!(back_to_open, BeadState::Open);
        assert!(back_to_open.is_active());
        assert!(!back_to_open.is_blocked());
        assert!(!back_to_open.is_closed());
    }

    #[test]
    fn chain_open_to_deferred_to_open() {
        let state = BeadState::Open;

        let deferred = state
            .transition_to(BeadState::Deferred)
            .expect("Open→Deferred should succeed");
        assert_eq!(deferred, BeadState::Deferred);
        assert!(!deferred.is_active());
        assert!(!deferred.is_closed());

        let back_to_open = deferred
            .transition_to(BeadState::Open)
            .expect("Deferred→Open should succeed");
        assert_eq!(back_to_open, BeadState::Open);
        assert!(back_to_open.is_active());
        assert!(!back_to_open.is_closed());
    }

    mod proptest_bead_state {
        use super::*;
        use proptest::proptest;

        proptest! {
            #[test]
            fn active_implies_not_closed(state_seed in 0u8..=4) {
                let state = match state_seed {
                    0 => BeadState::Open,
                    1 => BeadState::InProgress,
                    2 => BeadState::Blocked,
                    3 => BeadState::Deferred,
                    _ => BeadState::Closed { closed_at: Utc::now() },
                };
                if state.is_active() {
                    assert!(!state.is_closed());
                }
                if state.is_closed() {
                    assert!(!state.is_active());
                }
            }

            #[test]
            fn closed_state_always_has_timestamp(state_seed in 0u8..=4) {
                let state = match state_seed {
                    0 => BeadState::Open,
                    1 => BeadState::InProgress,
                    2 => BeadState::Blocked,
                    3 => BeadState::Deferred,
                    _ => BeadState::Closed { closed_at: Utc::now() },
                };
                assert_eq!(state.is_closed(), state.closed_at().is_some());
            }
        }
    }
}
