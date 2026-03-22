//! Session output types
//!
//! Provides session state information for the AI control plane.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::output_jsonl::errors::OutputLineError;
use crate::{types::SessionStatus, WorkspaceState};

/// Session output line containing session state and metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionOutput {
    pub name: String,
    pub status: SessionStatus,
    pub state: WorkspaceState,
    pub workspace_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

impl SessionOutput {
    /// Create a new session output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptySessionName` if `name` is blank.
    /// Returns `OutputLineError::RelativePath` if `workspace_path` is not absolute.
    pub fn new(
        name: String,
        status: SessionStatus,
        state: WorkspaceState,
        workspace_path: PathBuf,
    ) -> Result<Self, OutputLineError> {
        if name.trim().is_empty() {
            return Err(OutputLineError::EmptySessionName);
        }
        if !workspace_path.is_absolute() {
            return Err(OutputLineError::RelativePath);
        }
        let now = Utc::now();
        Ok(Self {
            name,
            status,
            state,
            workspace_path,
            branch: None,
            metadata: None,
            created_at: now,
            updated_at: now,
        })
    }

    #[must_use]
    pub fn with_branch(self, branch: String) -> Self {
        Self {
            branch: Some(branch),
            ..self
        }
    }

    #[must_use]
    pub fn with_metadata(self, metadata: serde_json::Value) -> Self {
        Self {
            metadata: Some(metadata),
            ..self
        }
    }
}

/// Session state for output (mirrors `SessionStatus` for JSON output)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Active,
    Paused,
    Creating,
    Completed,
    Failed,
}

/// Type alias for backward compatibility
pub type Session = SessionOutput;
