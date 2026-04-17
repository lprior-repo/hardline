// //! SessionOutput builder
//!
//! Builder for `SessionOutput` with compile-time required field tracking.

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::{
    domain::session::BranchState, output_jsonl::SessionOutput,
    types::SessionStatus as TypesSessionStatus, WorkspaceState as TypesWorkspaceState,
};

/// Builder for `SessionOutput` with compile-time required field tracking
///
/// # Required Fields
/// - `name`: Session name
/// - `status`: Session status
/// - `state`: Workspace state
/// - `workspace_path`: Absolute path to workspace
///
/// # Optional Fields
/// - `branch`: Git branch information
/// - `created_at`: Creation timestamp (defaults to now)
/// - `updated_at`: Update timestamp (defaults to now)
#[derive(Debug, Clone)]
pub struct SessionOutputBuilder {
    // Required fields (Option to track presence)
    name: Option<String>,
    status: Option<TypesSessionStatus>,
    state: Option<TypesWorkspaceState>,
    workspace_path: Option<PathBuf>,

    // Optional fields
    branch: Option<BranchState>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

impl Default for SessionOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionOutputBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            name: None,
            status: None,
            state: None,
            workspace_path: None,
            branch: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Set the session name (required)
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::InvalidValue` if the name is empty.
    pub fn name(mut self, name: impl Into<String>) -> Result<Self, super::errors::BuilderError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(super::errors::BuilderError::InvalidValue {
                field: "name",
                reason: "session name cannot be empty".to_string(),
            });
        }
        self.name = Some(name);
        Ok(self)
    }

    /// Set the session status (required)
    #[must_use]
    pub const fn status(mut self, status: TypesSessionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the workspace state (required)
    #[must_use]
    pub const fn state(mut self, state: TypesWorkspaceState) -> Self {
        self.state = Some(state);
        self
    }

    /// Set the workspace path (required)
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::InvalidValue` if the path is not absolute.
    pub fn workspace_path(
        mut self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, super::errors::BuilderError> {
        let path = path.into();
        if !path.is_absolute() {
            return Err(super::errors::BuilderError::InvalidValue {
                field: "workspace_path",
                reason: "workspace path must be absolute".to_string(),
            });
        }
        self.workspace_path = Some(path);
        Ok(self)
    }

    /// Set the branch state (optional)
    #[must_use]
    pub fn branch(mut self, branch: BranchState) -> Self {
        self.branch = Some(branch);
        self
    }

    /// Set the creation timestamp (optional, defaults to now)
    #[must_use]
    pub const fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the update timestamp (optional, defaults to now)
    #[must_use]
    pub const fn updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Build the `SessionOutput`
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    /// Returns `BuilderError::InvalidValue` if validation fails.
    pub fn build(self) -> Result<SessionOutput, super::errors::BuilderError> {
        // Validate required fields
        let name = self
            .name
            .ok_or(super::errors::BuilderError::MissingRequired { field: "name" })?;
        let status = self
            .status
            .ok_or(super::errors::BuilderError::MissingRequired { field: "status" })?;
        let state = self
            .state
            .ok_or(super::errors::BuilderError::MissingRequired { field: "state" })?;
        let workspace_path =
            self.workspace_path
                .ok_or(super::errors::BuilderError::MissingRequired {
                    field: "workspace_path",
                })?;

        // Convert status to the output type
        let output_status = convert_session_status(status);

        // Convert state to the output type
        let output_state = convert_workspace_state(state);

        let now = self.created_at.unwrap_or_else(Utc::now);
        let updated = self.updated_at.unwrap_or(now);

        Ok(SessionOutput {
            name,
            status: output_status,
            state: output_state,
            workspace_path,
            branch: self.branch.map(|b: BranchState| b.to_string()),
            metadata: None,
            created_at: now,
            updated_at: updated,
        })
    }
}

const fn convert_session_status(status: TypesSessionStatus) -> crate::types::SessionStatus {
    match status {
        TypesSessionStatus::Creating => crate::types::SessionStatus::Creating,
        TypesSessionStatus::Active => crate::types::SessionStatus::Active,
        TypesSessionStatus::Paused => crate::types::SessionStatus::Paused,
        TypesSessionStatus::Completed => crate::types::SessionStatus::Completed,
        TypesSessionStatus::Failed => crate::types::SessionStatus::Failed,
    }
}

const fn convert_workspace_state(state: TypesWorkspaceState) -> crate::WorkspaceState {
    // WorkspaceState is the same type, just use it directly
    state
}
