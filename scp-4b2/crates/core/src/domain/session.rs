//! Session domain types
//!
//! Provides types for session state and operations.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Session branch state - replaces Option<String> for branch
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchState {
    /// Session is detached (no branch)
    Detached,
    /// Session is on a specific branch
    OnBranch { name: String },
}

impl BranchState {
    #[must_use]
    pub fn branch_name(&self) -> Option<&str> {
        match self {
            Self::Detached => None,
            Self::OnBranch { name } => Some(name),
        }
    }

    #[must_use]
    pub const fn is_detached(&self) -> bool {
        matches!(self, Self::Detached)
    }

    /// Check if a transition from self to target is valid
    #[must_use]
    pub const fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            // Detached can switch to any branch, and OnBranch can become detached
            (Self::Detached, Self::OnBranch { .. })
            | (Self::OnBranch { .. }, Self::Detached)
            | (Self::OnBranch { .. }, Self::OnBranch { .. }) => true,

            // Detached staying Detached is not a transition
            (Self::Detached, Self::Detached) => false,
        }
    }
}

impl std::fmt::Display for BranchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Detached => write!(f, "detached"),
            Self::OnBranch { name } => write!(f, "{name}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_state() {
        let detached = BranchState::Detached;
        assert!(detached.is_detached());
        assert!(detached.branch_name().is_none());

        let on_branch = BranchState::OnBranch {
            name: "main".to_string(),
        };
        assert!(!on_branch.is_detached());
        assert_eq!(on_branch.branch_name(), Some("main"));
    }
}
