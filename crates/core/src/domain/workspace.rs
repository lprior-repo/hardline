//! Workspace domain types
//!
//! Provides types for workspace state and operations.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

/// Workspace state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceState {
    /// Workspace is being created
    Creating,
    /// Workspace is ready for use
    Ready,
    /// Workspace is in use
    Active,
    /// Workspace is being cleaned up
    Cleaning,
    /// Workspace has been removed
    Removed,
}

impl WorkspaceState {
    /// All valid workspace states
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Creating,
            Self::Ready,
            Self::Active,
            Self::Cleaning,
            Self::Removed,
        ]
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready | Self::Active)
    }

    #[must_use]
    pub const fn is_removed(&self) -> bool {
        matches!(self, Self::Removed)
    }

    /// Check if a transition from self to target is valid
    #[must_use]
    #[allow(clippy::match_same_arms)] // More readable as explicit patterns
    pub const fn can_transition_to(self, target: &Self) -> bool {
        match (self, target) {
            // Creation workflow
            (Self::Creating, Self::Ready | Self::Removed) => true,
            // Ready becomes Active when used
            (Self::Ready, Self::Active | Self::Cleaning | Self::Removed) => true,
            // Active can be cleaned or removed
            (Self::Active, Self::Cleaning | Self::Removed) => true,
            // Cleaning always goes to Removed
            (Self::Cleaning, Self::Removed) => true,
            // Removed is terminal, no self-loops or other transitions
            _ => false,
        }
    }

    /// Get all valid target states from this state
    #[must_use]
    pub fn valid_transitions(&self) -> Vec<Self> {
        Self::all()
            .iter()
            .filter(|&target| self.can_transition_to(target))
            .copied()
            .collect()
    }

    /// Check if this is a terminal state
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Removed)
    }
}

impl std::fmt::Display for WorkspaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Ready => write!(f, "ready"),
            Self::Active => write!(f, "active"),
            Self::Cleaning => write!(f, "cleaning"),
            Self::Removed => write!(f, "removed"),
        }
    }
}

/// Workspace information
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub path: PathBuf,
    pub state: WorkspaceState,
}

impl WorkspaceInfo {
    #[must_use]
    pub const fn new(path: PathBuf, state: WorkspaceState) -> Self {
        Self { path, state }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- all() --

    #[test]
    fn test_all_returns_five_states() {
        assert_eq!(WorkspaceState::all().len(), 5);
    }

    #[test]
    fn test_all_is_exhaustive() {
        let all = WorkspaceState::all();
        assert!(all.contains(&WorkspaceState::Creating));
        assert!(all.contains(&WorkspaceState::Ready));
        assert!(all.contains(&WorkspaceState::Active));
        assert!(all.contains(&WorkspaceState::Cleaning));
        assert!(all.contains(&WorkspaceState::Removed));
    }

    // -- is_active / is_ready / is_removed --

    #[test]
    fn test_is_active_only_active() {
        assert!(WorkspaceState::Active.is_active());
        assert!(!WorkspaceState::Ready.is_active());
        assert!(!WorkspaceState::Creating.is_active());
        assert!(!WorkspaceState::Cleaning.is_active());
        assert!(!WorkspaceState::Removed.is_active());
    }

    #[test]
    fn test_is_ready() {
        assert!(WorkspaceState::Ready.is_ready());
        assert!(WorkspaceState::Active.is_ready());
        assert!(!WorkspaceState::Creating.is_ready());
        assert!(!WorkspaceState::Cleaning.is_ready());
        assert!(!WorkspaceState::Removed.is_ready());
    }

    #[test]
    fn test_is_removed() {
        assert!(WorkspaceState::Removed.is_removed());
        assert!(!WorkspaceState::Active.is_removed());
    }

    // -- is_terminal --

    #[test]
    fn test_removed_is_terminal() {
        assert!(WorkspaceState::Removed.is_terminal());
        assert!(!WorkspaceState::Creating.is_terminal());
        assert!(!WorkspaceState::Ready.is_terminal());
        assert!(!WorkspaceState::Active.is_terminal());
        assert!(!WorkspaceState::Cleaning.is_terminal());
    }

    // -- Transitions --

    #[test]
    fn test_creating_to_ready() {
        assert!(WorkspaceState::Creating.can_transition_to(&WorkspaceState::Ready));
    }

    #[test]
    fn test_creating_to_removed() {
        assert!(WorkspaceState::Creating.can_transition_to(&WorkspaceState::Removed));
    }

    #[test]
    fn test_creating_to_active_rejected() {
        assert!(!WorkspaceState::Creating.can_transition_to(&WorkspaceState::Active));
    }

    #[test]
    fn test_ready_to_active() {
        assert!(WorkspaceState::Ready.can_transition_to(&WorkspaceState::Active));
    }

    #[test]
    fn test_ready_to_cleaning() {
        assert!(WorkspaceState::Ready.can_transition_to(&WorkspaceState::Cleaning));
    }

    #[test]
    fn test_ready_to_removed() {
        assert!(WorkspaceState::Ready.can_transition_to(&WorkspaceState::Removed));
    }

    #[test]
    fn test_ready_to_creating_rejected() {
        assert!(!WorkspaceState::Ready.can_transition_to(&WorkspaceState::Creating));
    }

    #[test]
    fn test_active_to_cleaning() {
        assert!(WorkspaceState::Active.can_transition_to(&WorkspaceState::Cleaning));
    }

    #[test]
    fn test_active_to_removed() {
        assert!(WorkspaceState::Active.can_transition_to(&WorkspaceState::Removed));
    }

    #[test]
    fn test_cleaning_to_removed() {
        assert!(WorkspaceState::Cleaning.can_transition_to(&WorkspaceState::Removed));
    }

    #[test]
    fn test_cleaning_to_active_rejected() {
        assert!(!WorkspaceState::Cleaning.can_transition_to(&WorkspaceState::Active));
    }

    #[test]
    fn test_removed_rejects_all() {
        for &state in &WorkspaceState::all() {
            assert!(
                !WorkspaceState::Removed.can_transition_to(&state),
                "Removed should reject transition to {:?}",
                state
            );
        }
    }

    // -- valid_transitions --

    #[test]
    fn test_valid_transitions_creating() {
        let transitions = WorkspaceState::Creating.valid_transitions();
        assert_eq!(transitions.len(), 2);
        assert!(transitions.contains(&WorkspaceState::Ready));
        assert!(transitions.contains(&WorkspaceState::Removed));
    }

    #[test]
    fn test_valid_transitions_removed_is_empty() {
        assert!(WorkspaceState::Removed.valid_transitions().is_empty());
    }

    // -- Display --

    #[test]
    fn test_display_all_variants() {
        assert_eq!(format!("{}", WorkspaceState::Creating), "creating");
        assert_eq!(format!("{}", WorkspaceState::Ready), "ready");
        assert_eq!(format!("{}", WorkspaceState::Active), "active");
        assert_eq!(format!("{}", WorkspaceState::Cleaning), "cleaning");
        assert_eq!(format!("{}", WorkspaceState::Removed), "removed");
    }

    // -- PartialEq --

    #[test]
    fn test_state_equality() {
        assert_eq!(WorkspaceState::Active, WorkspaceState::Active);
        assert_ne!(WorkspaceState::Active, WorkspaceState::Ready);
    }

    // -- Copy --

    #[test]
    fn test_copy() {
        let state = WorkspaceState::Active;
        let copied = state;
        assert_eq!(state, copied);
    }

    // -- WorkspaceInfo --

    #[test]
    fn test_workspace_info_new() {
        let info = WorkspaceInfo::new(PathBuf::from("/tmp/ws"), WorkspaceState::Ready);
        assert_eq!(info.path, PathBuf::from("/tmp/ws"));
        assert_eq!(info.state, WorkspaceState::Ready);
    }

    #[test]
    fn test_workspace_info_clone() {
        let info = WorkspaceInfo::new(PathBuf::from("/tmp/ws"), WorkspaceState::Active);
        let cloned = info.clone();
        assert_eq!(info.path, cloned.path);
        assert_eq!(info.state, cloned.state);
    }
}
