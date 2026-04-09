//! Workspace navigation data types

use scp_core::vcs::Workspace;

/// Command types for workspace navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceNavCommand {
    /// Create a new workspace
    Spawn,
    /// Switch to a workspace
    Switch,
    /// List all workspaces
    List,
    /// Show workspace status
    Status,
    /// Switch to next workspace
    Next,
    /// Switch to previous workspace
    Prev,
}

/// Output types for workspace navigation
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceNavOutput {
    /// The workspace name involved
    pub workspace: Option<String>,
    /// Success message or details
    pub message: String,
    /// Whether the operation was successful
    pub success: bool,
}

impl WorkspaceNavOutput {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            workspace: None,
            message: message.into(),
            success: true,
        }
    }

    pub fn success_with_workspace(
        workspace: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            workspace: Some(workspace.into()),
            message: message.into(),
            success: true,
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            workspace: None,
            message: message.into(),
            success: false,
        }
    }
}

/// Workspace info for listing
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub name: String,
    pub is_current: bool,
}

impl From<Workspace> for WorkspaceInfo {
    fn from(ws: Workspace) -> Self {
        Self {
            name: ws.name,
            is_current: ws.is_current,
        }
    }
}
