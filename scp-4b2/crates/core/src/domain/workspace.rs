//! Workspace domain types
//!
//! Provides types for workspace state and operations.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

/// Workspace state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
