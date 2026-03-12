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

use thiserror::Error;

use crate::domain::{identifiers::WorkspaceName, workspace::WorkspaceState};

// ============================================================================
// DOMAIN ERRORS
// ============================================================================

/// Errors that can occur during workspace operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkspaceError {
    /// Invalid state transition
    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition {
        from: WorkspaceState,
        to: WorkspaceState,
    },

    /// Workspace path does not exist
    #[error("workspace path does not exist: {0}")]
    PathNotFound(PathBuf),

    /// Workspace is not in a ready state
    #[error("workspace is not ready: {0:?}")]
    NotReady(WorkspaceState),

    /// Workspace is not active
    #[error("workspace is not active: {0:?}")]
    NotActive(WorkspaceState),

    /// Workspace has been removed
    #[error("workspace has been removed")]
    Removed,

    /// Cannot use workspace in current state
    #[error("cannot use workspace in state: {0:?}")]
    CannotUse(WorkspaceState),

    /// Workspace name already exists
    #[error("workspace name already exists: {0}")]
    NameAlreadyExists(WorkspaceName),
}

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
    pub name: WorkspaceName,
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
    pub fn create(name: WorkspaceName, path: PathBuf) -> Result<Self, WorkspaceError> {
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
        name: WorkspaceName,
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
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError::InvalidStateTransition` if transition is invalid.
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

// ============================================================================
// WORKSPACE BUILDER
// ============================================================================

/// Builder for constructing workspaces.
///
/// Provides a fluent interface for workspace creation with validation.
#[derive(Debug, Default)]
pub struct WorkspaceBuilder {
    name: Option<WorkspaceName>,
    path: Option<PathBuf>,
    state: Option<WorkspaceState>,
}

impl WorkspaceBuilder {
    /// Create a new workspace builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the workspace name.
    #[must_use]
    pub fn name(mut self, name: WorkspaceName) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the workspace path.
    #[must_use]
    pub fn path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Set the workspace state.
    #[must_use]
    pub const fn state(mut self, state: WorkspaceState) -> Self {
        self.state = Some(state);
        self
    }

    /// Build the workspace.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceError` if:
    /// - Required fields are missing
    /// - Path doesn't exist
    pub fn build(self) -> Result<Workspace, WorkspaceError> {
        let name = self.name.ok_or(WorkspaceError::CannotUse(
            WorkspaceState::Creating, // Using existing error for missing name
        ))?;
        let path = self.path.ok_or(WorkspaceError::CannotUse(
            WorkspaceState::Creating, // Using existing error for missing path
        ))?;

        match self.state {
            Some(state) => Workspace::reconstruct(name, path, state),
            None => Workspace::create(name, path),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_workspace() -> Workspace {
        let name = WorkspaceName::parse("test-workspace").expect("valid name");
        let path = PathBuf::from("/tmp"); // Assume exists for test

        Workspace::create(name, path).expect("workspace created")
    }

    #[test]
    fn test_create_workspace() {
        let workspace = create_test_workspace();

        assert!(workspace.is_creating());
        assert!(!workspace.is_ready());
        assert!(!workspace.is_active());
        assert_eq!(workspace.name.as_str(), "test-workspace");
    }

    #[test]
    fn test_creating_to_ready() {
        let workspace = create_test_workspace();

        let ready = workspace.mark_ready().expect("transition valid");

        assert!(ready.is_ready());
        assert!(!ready.is_creating());
    }

    #[test]
    fn test_ready_to_active() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");

        let active = ready.mark_active().expect("transition valid");

        assert!(active.is_active());
        assert!(!active.is_ready());
    }

    #[test]
    fn test_active_to_cleaning() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");
        let active = ready.mark_active().expect("transition valid");

        let cleaning = active.start_cleaning().expect("transition valid");

        assert!(cleaning.is_cleaning());
        assert!(!cleaning.is_active());
    }

    #[test]
    fn test_cleaning_to_removed() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");
        let active = ready.mark_active().expect("transition valid");
        let cleaning = active.start_cleaning().expect("transition valid");

        let removed = cleaning.mark_removed().expect("transition valid");

        assert!(removed.is_removed());
        assert!(removed.is_terminal());
    }

    #[test]
    fn test_ready_to_removed() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");

        let removed = ready.mark_removed().expect("transition valid");

        assert!(removed.is_removed());
    }

    #[test]
    fn test_creating_to_removed() {
        let workspace = create_test_workspace();

        let removed = workspace.mark_removed().expect("transition valid");

        assert!(removed.is_removed());
    }

    #[test]
    fn test_invalid_state_transition() {
        let workspace = create_test_workspace();

        // Cannot go from Creating to Active directly
        let result = workspace.transition_to(WorkspaceState::Active);
        assert!(matches!(
            result,
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));

        // Removed is terminal
        let removed = workspace.mark_removed().expect("transition valid");
        let result = removed.transition_to(WorkspaceState::Creating);
        assert!(matches!(
            result,
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
    }

    #[test]
    fn test_validate_ready() {
        let workspace = create_test_workspace();

        // Creating state is not ready
        let result = workspace.validate_ready();
        assert!(matches!(result, Err(WorkspaceError::NotReady(_))));

        let ready = workspace.mark_ready().expect("transition valid");
        assert!(ready.validate_ready().is_ok());
    }

    #[test]
    fn test_validate_active() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");

        // Ready is not active
        let result = ready.validate_active();
        assert!(matches!(result, Err(WorkspaceError::NotActive(_))));

        let active = ready.mark_active().expect("transition valid");
        assert!(active.validate_active().is_ok());
    }

    #[test]
    fn test_validate_can_use() {
        let workspace = create_test_workspace();

        // Creating cannot be used
        let result = workspace.validate_can_use();
        assert!(matches!(result, Err(WorkspaceError::CannotUse(_))));

        let ready = workspace.mark_ready().expect("transition valid");
        assert!(ready.validate_can_use().is_ok());

        let active = ready.mark_active().expect("transition valid");
        assert!(active.validate_can_use().is_ok());
    }

    #[test]
    fn test_path_not_found() {
        let name = WorkspaceName::parse("test").expect("valid name");
        let path = PathBuf::from("/nonexistent/path");

        let result = Workspace::create(name, path);
        assert!(matches!(result, Err(WorkspaceError::PathNotFound(_))));
    }

    #[test]
    fn test_change_path() {
        let workspace = create_test_workspace();
        let new_path = PathBuf::from("/var/tmp");

        let changed = workspace
            .change_path(new_path.clone())
            .expect("path changed");
        assert_eq!(changed.path, new_path);
    }

    #[test]
    fn test_builder() {
        let name = WorkspaceName::parse("builder-test").expect("valid name");
        let path = PathBuf::from("/tmp");

        let workspace = Workspace::builder()
            .name(name.clone())
            .path(path)
            .build()
            .expect("builder works");

        assert_eq!(workspace.name, name);
        assert!(workspace.is_creating());
    }

    #[test]
    fn test_builder_with_state() {
        let name = WorkspaceName::parse("builder-state").expect("valid name");
        let path = PathBuf::from("/tmp");

        let workspace = Workspace::builder()
            .name(name)
            .path(path)
            .state(WorkspaceState::Ready)
            .build()
            .expect("builder works");

        assert!(workspace.is_ready());
    }
}
