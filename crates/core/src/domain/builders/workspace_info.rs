// //! Workspace info builder
//!
//! Builder for `WorkspaceInfo` with fluent API.

use std::path::PathBuf;

use crate::domain::workspace::{WorkspaceInfo, WorkspaceState as OutputWorkspaceState};

/// Builder for `WorkspaceInfo` with fluent API
///
/// # Required Fields
/// - `path`: Workspace path
/// - `state`: Workspace state
#[derive(Debug, Clone)]
pub struct WorkspaceInfoBuilder {
    // Required fields
    path: Option<PathBuf>,
    state: Option<WorkspaceInfoState>,
}

/// Workspace info state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceInfoState {
    Creating,
    Ready,
    Active,
    Cleaning,
    Removed,
}

impl Default for WorkspaceInfoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceInfoBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            path: None,
            state: None,
        }
    }

    /// Set the workspace path (required)
    #[must_use]
    pub fn path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Set the workspace state (required)
    #[must_use]
    pub const fn state(mut self, state: WorkspaceInfoState) -> Self {
        self.state = Some(state);
        self
    }

    /// Build the `WorkspaceInfo`
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<WorkspaceInfo, super::errors::BuilderError> {
        let path = self
            .path
            .ok_or(super::errors::BuilderError::MissingRequired { field: "path" })?;
        let state = self
            .state
            .ok_or(super::errors::BuilderError::MissingRequired { field: "state" })?;

        Ok(WorkspaceInfo {
            path,
            state: convert_workspace_info_state(state),
        })
    }
}

const fn convert_workspace_info_state(state: WorkspaceInfoState) -> OutputWorkspaceState {
    match state {
        WorkspaceInfoState::Creating => OutputWorkspaceState::Creating,
        WorkspaceInfoState::Ready => OutputWorkspaceState::Ready,
        WorkspaceInfoState::Active => OutputWorkspaceState::Active,
        WorkspaceInfoState::Cleaning => OutputWorkspaceState::Cleaning,
        WorkspaceInfoState::Removed => OutputWorkspaceState::Removed,
    }
}
