//! Workspace-related domain events
//!
//! Events emitted during workspace lifecycle:
//! - [`WorkspaceCreatedEvent`] - A workspace was created
//! - [`WorkspaceRemovedEvent`] - A workspace was removed

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::identifiers::WorkspaceName;

/// Event emitted when a workspace is created
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCreatedEvent {
    /// Name of the workspace
    pub workspace_name: WorkspaceName,
    /// Path to the workspace on disk
    pub path: PathBuf,
    /// When the workspace was created
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a workspace is removed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRemovedEvent {
    /// Name of the workspace
    pub workspace_name: WorkspaceName,
    /// Path where the workspace was located
    pub path: PathBuf,
    /// When the workspace was removed
    pub timestamp: DateTime<Utc>,
}
