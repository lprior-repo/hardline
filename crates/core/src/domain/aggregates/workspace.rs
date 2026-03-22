//! Workspace aggregate root with business rules and invariants.
//!
//! The Workspace aggregate represents a development workspace with:
//! - Unique identity (`WorkspaceName`)
//! - Filesystem location (`PathBuf`)
//! - Lifecycle state (Creating -> Ready -> Active -> Cleaning -> Removed)
//!
//! # Invariants
//!
//! 1. Workspace names must be unique
//! 2. State transitions follow the lifecycle:
//!    - Creating -> Ready | Removed
//!    - Ready -> Active | Cleaning | Removed
//!    - Active -> Cleaning | Removed
//!    - Cleaning -> Removed
//!    - Removed (terminal)
//! 3. Workspace path must exist for Ready/Active states
//! 4. Only workspaces in Ready/Active state can be used for development

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::domain::workspace::WorkspaceState;

// Re-export from sibling modules (declared in parent)
pub use crate::domain::aggregates::workspace_error::WorkspaceError;
pub use crate::domain::aggregates::workspace_builder::WorkspaceBuilder;

// ============================================================================
// WORKSPACE AGGREGATE ROOT
// ============================================================================

/// Workspace aggregate root.
///
/// Enforces all business rules and invariants for workspaces.
/// All state transitions go through validated methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    /// Workspace name (unique identifier)
    pub name: crate::domain::identifiers::WorkspaceName,
    /// Absolute path to workspace directory
    pub path: PathBuf,
    /// Current workspace state
    pub state: WorkspaceState,
}

impl Workspace {
    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Create a new workspace in Creating state.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::PathNotFound` if path doesn't exist.
    pub fn create(
        name: crate::domain::identifiers::WorkspaceName,
        path: PathBuf,
    ) -> Result<Self, WorkspaceError> {
        if !path.exists() {
            return Err(WorkspaceError::PathNotFound(path));
        }

        Ok(Self {
            name,
            path,
            state: WorkspaceState::Creating,
        })
    }

    /// Create a workspace with a specific state (for reconstruction).
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::PathNotFound` if path doesn't exist.
    pub fn reconstruct(
        name: crate::domain::identifiers::WorkspaceName,
        path: PathBuf,
        state: WorkspaceState,
    ) -> Result<Self, WorkspaceError> {
        if !path.exists() {
            return Err(WorkspaceError::PathNotFound(path));
        }

        Ok(Self { name, path, state })
    }

    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// Check if workspace is in Creating state.
    #[must_use]
    pub const fn is_creating(&self) -> bool {
        matches!(self.state, WorkspaceState::Creating)
    }

    /// Check if workspace is in Ready state.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self.state, WorkspaceState::Ready)
    }

    /// Check if workspace is in Active state.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.state, WorkspaceState::Active)
    }

    /// Check if workspace is in Cleaning state.
    #[must_use]
    pub const fn is_cleaning(&self) -> bool {
        matches!(self.state, WorkspaceState::Cleaning)
    }

    /// Check if workspace has been removed.
    #[must_use]
    pub const fn is_removed(&self) -> bool {
        matches!(self.state, WorkspaceState::Removed)
    }

    /// Check if workspace is ready for use (Ready or Active).
    #[must_use]
    pub const fn can_use(&self) -> bool {
        matches!(self.state, WorkspaceState::Ready | WorkspaceState::Active)
    }

    /// Check if workspace is in a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    // ========================================================================
    // STATE TRANSITION METHODS
    // ========================================================================

    /// Transition to Ready state.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::InvalidStateTransition` if current state is not Creating.
    pub fn mark_ready(&self) -> Result<Self, WorkspaceError> {
        self.transition_to(WorkspaceState::Ready)
    }

    /// Transition to Active state.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::InvalidStateTransition` if current state is not Ready.
    pub fn mark_active(&self) -> Result<Self, WorkspaceError> {
        self.transition_to(WorkspaceState::Active)
    }

    /// Transition to Cleaning state.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::InvalidStateTransition` if current state is not Ready or Active.
    pub fn start_cleaning(&self) -> Result<Self, WorkspaceError> {
        self.transition_to(WorkspaceState::Cleaning)
    }

    /// Transition to Removed state.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::InvalidStateTransition` if current state is terminal.
    pub fn mark_removed(&self) -> Result<Self, WorkspaceError> {
        self.transition_to(WorkspaceState::Removed)
    }

    /// Transition to a new state with validation.
    fn transition_to(&self, new_state: WorkspaceState) -> Result<Self, WorkspaceError> {
        if !self.state.can_transition_to(&new_state) {
            return Err(WorkspaceError::InvalidStateTransition {
                from: self.state,
                to: new_state,
            });
        }

        Ok(Self {
            state: new_state,
            ..self.clone()
        })
    }

    // ========================================================================
    // VALIDATION METHODS
    // ========================================================================

    /// Validate that workspace is ready for use.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::NotReady` if workspace is not in Ready or Active state.
    pub const fn validate_ready(&self) -> Result<(), WorkspaceError> {
        if !self.can_use() {
            return Err(WorkspaceError::NotReady(self.state));
        }
        Ok(())
    }

    /// Validate that workspace is active.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::NotActive` if workspace is not in Active state.
    pub const fn validate_active(&self) -> Result<(), WorkspaceError> {
        if !self.is_active() {
            return Err(WorkspaceError::NotActive(self.state));
        }
        Ok(())
    }

    /// Validate that workspace has not been removed.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::Removed` if workspace is in Removed state.
    pub const fn validate_not_removed(&self) -> Result<(), WorkspaceError> {
        if self.is_removed() {
            return Err(WorkspaceError::Removed);
        }
        Ok(())
    }

    /// Validate that workspace can be used for operations.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::CannotUse` if workspace is not in Ready or Active state.
    pub const fn validate_can_use(&self) -> Result<(), WorkspaceError> {
        if !self.can_use() {
            return Err(WorkspaceError::CannotUse(self.state));
        }
        Ok(())
    }

    // ========================================================================
    // PATH OPERATIONS
    // ========================================================================

    /// Change the workspace path.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::PathNotFound` if new path doesn't exist.
    pub fn change_path(&self, new_path: PathBuf) -> Result<Self, WorkspaceError> {
        if !new_path.exists() {
            return Err(WorkspaceError::PathNotFound(new_path));
        }

        Ok(Self {
            path: new_path,
            ..self.clone()
        })
    }

    // ========================================================================
    // BUILDER PATTERN
    // ========================================================================

    /// Create a builder for constructing workspaces.
    #[must_use]
    pub fn builder() -> WorkspaceBuilder {
        WorkspaceBuilder::new()
    }
}
