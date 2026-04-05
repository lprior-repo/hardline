//! Workspace state types for the isolate domain.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::IsolateError;

/// Lifecycle states for an isolated git-clone workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceState {
    #[default]
    Created,
    Working,
    Ready,
    Merged,
    Abandoned,
    Conflict,
}

impl WorkspaceState {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Created, Self::Working, Self::Ready, Self::Merged, Self::Abandoned, Self::Conflict]
    }

    #[must_use]
    pub fn valid_next_states(self) -> &'static [Self] {
        match self {
            Self::Created => &[Self::Working],
            Self::Working => &[Self::Ready, Self::Conflict, Self::Abandoned],
            Self::Ready => &[Self::Working, Self::Merged, Self::Conflict, Self::Abandoned],
            Self::Conflict => &[Self::Working, Self::Abandoned],
            Self::Merged | Self::Abandoned => &[],
        }
    }

    #[must_use]
    pub fn can_transition_to(self, target: Self) -> bool {
        self.valid_next_states().contains(&target)
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Merged | Self::Abandoned)
    }

    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Working | Self::Conflict)
    }

    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Ready | Self::Merged)
    }
}

impl fmt::Display for WorkspaceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Created => "created",
            Self::Working => "working",
            Self::Ready => "ready",
            Self::Merged => "merged",
            Self::Abandoned => "abandoned",
            Self::Conflict => "conflict",
        })
    }
}

impl FromStr for WorkspaceState {
    type Err = IsolateError;
    fn from_str(s: &str) -> std::result::Result<Self, IsolateError> {
        match s.to_lowercase().as_str() {
            "created" => Ok(Self::Created),
            "working" => Ok(Self::Working),
            "ready" => Ok(Self::Ready),
            "merged" => Ok(Self::Merged),
            "abandoned" => Ok(Self::Abandoned),
            "conflict" => Ok(Self::Conflict),
            _ => Err(IsolateError::InvalidState(s.to_string())),
        }
    }
}
