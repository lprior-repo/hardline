//! Data layer for stack navigate - inert, serializable types.
//!
//! No business logic. Types only.

use serde::{Deserialize, Serialize};

/// Direction for stack navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigateDirection {
    /// Move to parent branch (toward trunk).
    Up,
    /// Move to first child branch (away from trunk).
    Down,
    /// Move to the trunk branch (root of the stack).
    Top,
    /// Move to the deepest descendant branch (leaf of the stack).
    Bottom,
    /// Move to the previous sibling branch (same parent, alphabetically before).
    Prev,
}

impl std::fmt::Display for NavigateDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Up => write!(f, "up"),
            Self::Down => write!(f, "down"),
            Self::Top => write!(f, "top"),
            Self::Bottom => write!(f, "bottom"),
            Self::Prev => write!(f, "prev"),
        }
    }
}

/// Options for the stack navigate command.
#[derive(Debug, Clone)]
pub struct StackNavigateOptions {
    /// Direction to navigate.
    pub direction: NavigateDirection,
}

/// Result of a stack navigate operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackNavigateResult {
    /// Branch we navigated from.
    pub from_branch: String,
    /// Branch we navigated to (None if already at boundary).
    pub to_branch: Option<String>,
    /// Whether the git checkout was performed.
    pub checked_out: bool,
}

/// Error taxonomy for stack navigate operations.
#[derive(Debug, thiserror::Error)]
pub enum NavigateError {
    #[error("Not on a branch (detached HEAD)")]
    DetachedHead,
    #[error("Current branch '{0}' is not in the stack")]
    NotInStack(String),
    #[error("Already at {direction} — no branch to navigate to")]
    AtBoundary { direction: String },
    #[error("Workspace has uncommitted changes. Stash or commit first.")]
    DirtyWorkspace,
    #[error("No stack branches tracked")]
    EmptyStack,
    #[error("Failed to get current branch: {0}")]
    CurrentBranchFailed(String),
    #[error("Failed to checkout branch '{branch}': {stderr}")]
    CheckoutFailed { branch: String, stderr: String },
    #[error("VCS error: {0}")]
    VcsError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigate_direction_display() {
        assert_eq!(NavigateDirection::Up.to_string(), "up");
        assert_eq!(NavigateDirection::Down.to_string(), "down");
        assert_eq!(NavigateDirection::Top.to_string(), "top");
        assert_eq!(NavigateDirection::Bottom.to_string(), "bottom");
        assert_eq!(NavigateDirection::Prev.to_string(), "prev");
    }

    #[test]
    fn navigate_direction_serde_roundtrip() {
        for dir in [
            NavigateDirection::Up,
            NavigateDirection::Down,
            NavigateDirection::Top,
            NavigateDirection::Bottom,
            NavigateDirection::Prev,
        ] {
            let json = serde_json::to_string(&dir).expect("serialize");
            let back: NavigateDirection = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, dir);
        }
    }

    #[test]
    fn navigate_result_serialization() {
        let result = StackNavigateResult {
            from_branch: "feature-a".to_string(),
            to_branch: Some("main".to_string()),
            checked_out: true,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: StackNavigateResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.from_branch, "feature-a");
        assert_eq!(back.to_branch, Some("main".to_string()));
        assert!(back.checked_out);
    }

    #[test]
    fn navigate_error_display_boundary() {
        let err = NavigateError::AtBoundary {
            direction: "top".to_string(),
        };
        assert!(err.to_string().contains("top"));
    }

    #[test]
    fn navigate_error_display_dirty() {
        let err = NavigateError::DirtyWorkspace;
        assert!(err.to_string().contains("uncommitted"));
    }
}
