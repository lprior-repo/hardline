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

    // -- Display --

    #[test]
    fn test_display_detached() {
        assert_eq!(format!("{}", BranchState::Detached), "detached");
    }

    #[test]
    fn test_display_on_branch() {
        let branch = BranchState::OnBranch {
            name: "feature-xyz".to_string(),
        };
        assert_eq!(format!("{branch}"), "feature-xyz");
    }

    // -- Transitions --

    #[test]
    fn test_detached_to_on_branch() {
        let target = BranchState::OnBranch {
            name: "main".to_string(),
        };
        assert!(BranchState::Detached.can_transition_to(&target));
    }

    #[test]
    fn test_on_branch_to_detached() {
        let branch = BranchState::OnBranch {
            name: "main".to_string(),
        };
        assert!(branch.can_transition_to(&BranchState::Detached));
    }

    #[test]
    fn test_on_branch_to_on_branch_same() {
        let branch = BranchState::OnBranch {
            name: "main".to_string(),
        };
        let target = BranchState::OnBranch {
            name: "main".to_string(),
        };
        assert!(branch.can_transition_to(&target));
    }

    #[test]
    fn test_on_branch_to_different_branch() {
        let branch = BranchState::OnBranch {
            name: "main".to_string(),
        };
        let target = BranchState::OnBranch {
            name: "feature".to_string(),
        };
        assert!(branch.can_transition_to(&target));
    }

    #[test]
    fn test_detached_to_detached_rejected() {
        assert!(!BranchState::Detached.can_transition_to(&BranchState::Detached));
    }

    // -- PartialEq --

    #[test]
    fn test_branch_equality() {
        let a = BranchState::OnBranch {
            name: "main".to_string(),
        };
        let b = BranchState::OnBranch {
            name: "main".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_branch_inequality() {
        let a = BranchState::OnBranch {
            name: "main".to_string(),
        };
        let b = BranchState::OnBranch {
            name: "feature".to_string(),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_detached_ne_on_branch() {
        assert_ne!(
            BranchState::Detached,
            BranchState::OnBranch {
                name: "x".to_string()
            }
        );
    }

    // -- branch_name edge cases --

    #[test]
    fn test_branch_name_empty_string() {
        let branch = BranchState::OnBranch {
            name: String::new(),
        };
        assert_eq!(branch.branch_name(), Some(""));
        assert!(!branch.is_detached());
    }
}
