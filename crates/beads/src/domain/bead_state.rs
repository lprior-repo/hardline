use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

use crate::error::{BeadError, Result};

/// BeadState - lifecycle state of a bead following strict state machine rules:
/// Open -> Claimed -> InProgress -> Ready -> (Merged | Abandoned)
#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, EnumIter, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BeadState {
    Open,
    Claimed,
    InProgress,
    Ready,
    Merged,
    Abandoned,
}

impl BeadState {
    /// Check if state can transition to target state
    #[must_use]
    pub fn can_transition_to(&self, target: &BeadState) -> bool {
        // No self-loops allowed
        if self == target {
            return false;
        }

        // Terminal states cannot transition
        if self.is_terminal() {
            return false;
        }

        // Valid transitions following: Open -> Claimed -> InProgress -> Ready -> (Merged | Abandoned)
        match (self, target) {
            (BeadState::Open, BeadState::Claimed) => true,
            (BeadState::Claimed, BeadState::InProgress) => true,
            (BeadState::InProgress, BeadState::Ready) => true,
            (BeadState::Ready, BeadState::Merged) => true,
            (BeadState::Ready, BeadState::Abandoned) => true,
            _ => false,
        }
    }

    /// Check if state is terminal (Merged or Abandoned)
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, BeadState::Merged | BeadState::Abandoned)
    }

    /// Get valid transitions from current state
    #[must_use]
    pub fn valid_transitions(&self) -> Vec<BeadState> {
        BeadState::iter()
            .filter(|target| self.can_transition_to(target))
            .collect()
    }

    /// Perform state transition, returning error for invalid transitions
    pub fn transition_to(&self, target: BeadState) -> Result<BeadState> {
        if !self.can_transition_to(&target) {
            return Err(BeadError::InvalidStateTransition {
                from: self.clone(),
                to: target,
            });
        }
        Ok(target)
    }
}

/// Priority level for beads
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
    P4,
}

impl Priority {
    #[must_use]
    pub fn value(&self) -> u8 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
            Self::P4 => 4,
        }
    }

    pub fn from_value(value: u8) -> Self {
        match value {
            0 => Self::P0,
            1 => Self::P1,
            2 => Self::P2,
            3 => Self::P3,
            _ => Self::P4,
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.value())
    }
}

/// Type of bead
#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BeadType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
}
