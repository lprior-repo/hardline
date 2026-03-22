//! Workspace builder for fluent construction.
//!
//! Provides a builder pattern for constructing workspaces with validation.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::domain::identifiers::WorkspaceName;
use crate::domain::workspace::WorkspaceState;

use super::workspace::Workspace;
use super::workspace_error::WorkspaceError;

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
            WorkspaceState::Creating,
        ))?;
        let path = self.path.ok_or(WorkspaceError::CannotUse(
            WorkspaceState::Creating,
        ))?;

        match self.state {
            Some(state) => Workspace::reconstruct(name, path, state),
            None => Workspace::create(name, path),
        }
    }
}
