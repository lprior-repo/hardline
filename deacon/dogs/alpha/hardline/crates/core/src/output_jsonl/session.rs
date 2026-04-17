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

#[cfg(test)]
mod tests {
    use super::*;

    // ── SessionState enum ────────────────────────────────────────────────────

    #[test]
    fn test_session_state_all_variants() {
        let variants = [
            SessionState::Active,
            SessionState::Paused,
            SessionState::Creating,
            SessionState::Completed,
            SessionState::Failed,
        ];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn test_session_state_serde_roundtrip() {
        for state in [
            SessionState::Active,
            SessionState::Paused,
            SessionState::Creating,
            SessionState::Completed,
            SessionState::Failed,
        ] {
            let json = serde_json::to_string(&state).expect("serialize ok");
            let deserialized: SessionState = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(state, deserialized, "Roundtrip failed for {state:?}");
        }
    }

    #[test]
    fn test_session_state_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&SessionState::Active).expect("ok"),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&SessionState::Failed).expect("ok"),
            "\"failed\""
        );
    }

    // ── SessionOutput::new ───────────────────────────────────────────────────

    #[test]
    fn test_session_output_new() {
        let session = SessionOutput::new(
            "test-session".to_string(),
            crate::types::SessionStatus::Active,
            crate::WorkspaceState::Working,
            PathBuf::from("/tmp/workspace"),
        )
        .expect("valid");
        assert_eq!(session.name, "test-session");
        assert!(session.branch.is_none());
        assert!(session.metadata.is_none());
    }

    #[test]
    fn test_session_output_new_rejects_empty_name() {
        let result = SessionOutput::new(
            "".to_string(),
            crate::types::SessionStatus::Active,
            crate::WorkspaceState::Working,
            PathBuf::from("/tmp/workspace"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_session_output_new_rejects_whitespace_name() {
        let result = SessionOutput::new(
            "   ".to_string(),
            crate::types::SessionStatus::Active,
            crate::WorkspaceState::Working,
            PathBuf::from("/tmp/workspace"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_session_output_new_rejects_relative_path() {
        let result = SessionOutput::new(
            "test".to_string(),
            crate::types::SessionStatus::Active,
            crate::WorkspaceState::Working,
            PathBuf::from("relative/path"),
        );
        assert!(result.is_err());
    }

    // ── with_branch ──────────────────────────────────────────────────────────

    #[test]
    fn test_session_output_with_branch() {
        let session = SessionOutput::new(
            "test".to_string(),
            crate::types::SessionStatus::Active,
            crate::WorkspaceState::Working,
            PathBuf::from("/tmp/ws"),
        )
        .expect("valid")
        .with_branch("main".to_string());
        assert_eq!(session.branch.as_deref(), Some("main"));
    }

    // ── with_metadata ────────────────────────────────────────────────────────

    #[test]
    fn test_session_output_with_metadata() {
        let meta = serde_json::json!({"key": "value"});
        let session = SessionOutput::new(
            "test".to_string(),
            crate::types::SessionStatus::Active,
            crate::WorkspaceState::Working,
            PathBuf::from("/tmp/ws"),
        )
        .expect("valid")
        .with_metadata(meta);
        assert!(session.metadata.is_some());
        assert_eq!(session.metadata.as_ref().expect("meta")["key"], "value");
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn test_session_output_serde_roundtrip_minimal() {
        let session = SessionOutput::new(
            "serde-test".to_string(),
            crate::types::SessionStatus::Active,
            crate::WorkspaceState::Working,
            PathBuf::from("/tmp/ws"),
        )
        .expect("valid");

        let json = serde_json::to_string(&session).expect("serialize ok");
        let deserialized: SessionOutput = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(session.name, deserialized.name);
        assert!(deserialized.branch.is_none());
        assert!(deserialized.metadata.is_none());
    }

    #[test]
    fn test_session_output_serde_roundtrip_full() {
        let session = SessionOutput::new(
            "full-test".to_string(),
            crate::types::SessionStatus::Paused,
            crate::WorkspaceState::Created,
            PathBuf::from("/home/user/ws"),
        )
        .expect("valid")
        .with_branch("feature".to_string())
        .with_metadata(serde_json::json!({"agent": "claude"}));

        let json = serde_json::to_string(&session).expect("serialize ok");
        let deserialized: SessionOutput = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.name, "full-test");
        assert_eq!(deserialized.branch.as_deref(), Some("feature"));
    }

    #[test]
    fn test_session_output_serde_skips_none_fields() {
        let session = SessionOutput::new(
            "test".to_string(),
            crate::types::SessionStatus::Active,
            crate::WorkspaceState::Working,
            PathBuf::from("/tmp/ws"),
        )
        .expect("valid");

        let json_val = serde_json::to_value(&session).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(!obj.contains_key("branch"));
        assert!(!obj.contains_key("metadata"));
    }

    // ── Clone / Debug ────────────────────────────────────────────────────────

    #[test]
    fn test_session_output_clone() {
        let session = SessionOutput::new(
            "clone-test".to_string(),
            crate::types::SessionStatus::Active,
            crate::WorkspaceState::Working,
            PathBuf::from("/tmp/ws"),
        )
        .expect("valid");
        let cloned = session.clone();
        assert_eq!(session.name, cloned.name);
    }

    #[test]
    fn test_session_output_debug() {
        let session = SessionOutput::new(
            "debug-test".to_string(),
            crate::types::SessionStatus::Active,
            crate::WorkspaceState::Working,
            PathBuf::from("/tmp/ws"),
        )
        .expect("valid");
        let debug = format!("{session:?}");
        assert!(debug.contains("debug-test"));
    }
}
