//! Session status state machine and operations
//!
//! Session lifecycle: Creating -> Active -> Paused/Completed
//!                     Creating -> Failed

use serde::{Deserialize, Serialize};

use crate::lifecycle::LifecycleState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Creating,
    Active,
    Paused,
    Completed,
    Failed,
}

impl SessionStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Creating | Self::Paused, Self::Active)
                | (Self::Creating, Self::Failed)
                | (Self::Active, Self::Paused | Self::Completed)
                | (Self::Paused, Self::Completed)
        )
    }

    #[must_use]
    pub fn valid_next_states(self) -> Vec<Self> {
        match self {
            Self::Creating => vec![Self::Active, Self::Failed],
            Self::Active => vec![Self::Paused, Self::Completed],
            Self::Paused => vec![Self::Active, Self::Completed],
            Self::Completed | Self::Failed => vec![],
        }
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    #[must_use]
    pub fn all_states() -> &'static [Self] {
        &[
            Self::Creating,
            Self::Active,
            Self::Paused,
            Self::Completed,
            Self::Failed,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    Status,
    Diff,
    Focus,
    Remove,
}

impl SessionStatus {
    pub fn allowed_operations(self) -> Vec<Operation> {
        match self {
            Self::Creating => vec![],
            Self::Active => vec![
                Operation::Status,
                Operation::Diff,
                Operation::Focus,
                Operation::Remove,
            ],
            Self::Paused => vec![Operation::Status, Operation::Focus, Operation::Remove],
            Self::Completed | Self::Failed => vec![Operation::Remove],
        }
    }

    #[must_use]
    pub fn allows_operation(self, op: Operation) -> bool {
        self.allowed_operations().contains(&op)
    }
}

impl LifecycleState for SessionStatus {
    fn can_transition_to(self, next: Self) -> bool {
        self.can_transition_to(next)
    }

    fn valid_next_states(self) -> Vec<Self> {
        self.valid_next_states()
    }

    fn is_terminal(self) -> bool {
        self.is_terminal()
    }

    fn all_states() -> &'static [Self] {
        Self::all_states()
    }
}
