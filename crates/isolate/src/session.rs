//! Session data structures and utilities for isolate workspaces.
//!
//! This module provides the Session types which represent isolated git-clone
//! workspaces managed by the isolate system.

use std::{fmt, str::FromStr};

use isolate_core::workspace_state::WorkspaceState;
use serde::{Deserialize, Serialize};

use crate::{IsolateError, Result};

/// Session status representing the lifecycle state of a session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is being created
    #[default]
    Creating,
    /// Session is active and ready for use
    Active,
    /// Session is temporarily paused
    Paused,
    /// Session work is completed
    Completed,
    /// Session creation or operation failed
    Failed,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Active => write!(f, "active"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl FromStr for SessionStatus {
    type Err = IsolateError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "creating" => Ok(Self::Creating),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(IsolateError::InvalidState(s.to_string())),
        }
    }
}

/// A session representing an isolated workspace
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub name: String,
    pub status: SessionStatus,
    #[serde(default)]
    pub state: WorkspaceState,
    pub workspace_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Session {
    /// Create a new session with the given name and workspace path
    #[cfg(test)]
    pub fn new(name: &str, workspace_path: &str) -> Result<Self> {
        validate_session_name(name)?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_err(|e| IsolateError::OperationFailed(format!("system time error: {e}")))?
            .as_secs();

        Ok(Self {
            id: None,
            name: name.to_string(),
            status: SessionStatus::Creating,
            state: WorkspaceState::Created,
            workspace_path: workspace_path.to_string(),
            branch: None,
            created_at: now,
            updated_at: now,
            last_synced: None,
            metadata: None,
        })
    }
}

/// Fields that can be updated on an existing session
#[derive(Debug, Clone, Default)]
pub struct SessionUpdate {
    /// Update the session status
    pub status: Option<SessionStatus>,
    /// Update the workspace state
    pub state: Option<WorkspaceState>,
    /// Update the branch
    pub branch: Option<String>,
    /// Update the last synced timestamp
    pub last_synced: Option<u64>,
    /// Update the metadata
    pub metadata: Option<serde_json::Value>,
}

/// Reserved keywords that cannot be used as session names
const RESERVED_SESSION_NAMES: &[&str] =
    &["null", "undefined", "true", "false", "none", "nil", "void"];

/// Validate a session name
///
/// Session names must:
/// - Not be empty
/// - Not exceed 64 characters
/// - Only contain ASCII alphanumeric characters, dashes, and underscores
/// - Start with a letter (a-z, A-Z)
/// - Not be a reserved keyword
pub fn validate_session_name(name: &str) -> Result<()> {
    // Check if name is empty
    if name.is_empty() {
        return Err(IsolateError::OperationFailed(
            "session name cannot be empty".to_string(),
        ));
    }

    // Check for non-ASCII characters
    if !name.is_ascii() {
        return Err(IsolateError::OperationFailed(
            "session name must contain only ASCII characters".to_string(),
        ));
    }

    // Check length
    if name.len() > 64 {
        return Err(IsolateError::OperationFailed(
            "session name cannot exceed 64 characters".to_string(),
        ));
    }

    // Only allow ASCII alphanumeric, dash, and underscore
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(IsolateError::OperationFailed(
            "session name can only contain ASCII alphanumeric characters, dashes, and underscores"
                .to_string(),
        ));
    }

    // Must start with a letter
    let first_char = name.chars().next();
    if first_char.map(|c| !c.is_ascii_alphabetic()).unwrap_or(true) {
        return Err(IsolateError::OperationFailed(
            "session name must start with a letter (a-z, A-Z)".to_string(),
        ));
    }

    // Check for reserved keywords
    let lower = name.to_lowercase();
    if RESERVED_SESSION_NAMES
        .iter()
        .any(|&keyword| keyword == lower)
    {
        return Err(IsolateError::OperationFailed(format!(
            "session name '{name}' is a reserved keyword"
        )));
    }

    Ok(())
}

/// Validate a status transition
///
/// Enforces valid state transitions in the session lifecycle:
/// - Creating -> Active, Failed
/// - Active -> Paused, Completed, Failed
/// - Paused -> Active, Failed
/// - Failed -> Creating (retry)
/// - Completed -> Active (reopen)
#[allow(dead_code)]
pub fn validate_status_transition(from: SessionStatus, to: SessionStatus) -> Result<()> {
    use SessionStatus::{Active, Completed, Creating, Failed, Paused};

    let valid = matches!(
        (from, to),
        (Creating | Paused | Completed, Active)
            | (Creating | Active | Paused, Failed)
            | (Active, Paused | Completed)
            | (Failed, Creating) // Can retry failed session
    );

    if valid {
        Ok(())
    } else {
        Err(IsolateError::InvalidTransition {
            from: from.to_string(),
            to: to.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new_valid() -> Result<()> {
        let session = Session::new("my-session", "/path/to/workspace")?;
        assert_eq!(session.name, "my-session");
        assert_eq!(session.status, SessionStatus::Creating);
        assert!(session.id.is_none());
        assert!(session.created_at > 0);
        assert_eq!(session.created_at, session.updated_at);
        Ok(())
    }

    #[test]
    fn test_session_name_empty() {
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_too_long() {
        let long_name = "a".repeat(65);
        let result = validate_session_name(&long_name);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_invalid_chars() {
        let result = validate_session_name("my session");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_starts_with_dash() {
        let result = validate_session_name("-session");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_valid_with_underscore() {
        let result = validate_session_name("my_session");
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_name_starts_with_underscore_rejected() {
        let result = validate_session_name("_session");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_starts_with_digit_rejected() {
        let result = validate_session_name("123session");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_rejects_unicode() {
        let unicode_cases = vec!["中文名字", "日本語", "café", "Ñoño", "naïve"];

        for name in unicode_cases {
            let result = validate_session_name(name);
            assert!(result.is_err(), "Should reject unicode name: {name}");
        }
    }

    #[test]
    fn test_session_name_accepts_valid_names() {
        let valid_cases = vec![
            "name",
            "my-name",
            "myName",
            "MyName123",
            "name123",
            "n-a-m-e",
            "feature-branch-123",
            "UPPERCASE",
            "a",
        ];

        for name in valid_cases {
            let result = validate_session_name(name);
            assert!(result.is_ok(), "Should accept valid name: {name}");
        }
    }

    #[test]
    fn test_status_display() {
        assert_eq!(SessionStatus::Creating.to_string(), "creating");
        assert_eq!(SessionStatus::Active.to_string(), "active");
        assert_eq!(SessionStatus::Paused.to_string(), "paused");
        assert_eq!(SessionStatus::Completed.to_string(), "completed");
        assert_eq!(SessionStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_status_from_str() -> Result<()> {
        assert_eq!(
            SessionStatus::from_str("creating")?,
            SessionStatus::Creating
        );
        assert_eq!(SessionStatus::from_str("active")?, SessionStatus::Active);
        assert_eq!(SessionStatus::from_str("paused")?, SessionStatus::Paused);
        assert_eq!(
            SessionStatus::from_str("completed")?,
            SessionStatus::Completed
        );
        assert_eq!(SessionStatus::from_str("failed")?, SessionStatus::Failed);
        Ok(())
    }

    #[test]
    fn test_status_from_str_invalid() {
        let result = SessionStatus::from_str("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_transition_creating_to_active() {
        let result = validate_status_transition(SessionStatus::Creating, SessionStatus::Active);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transition_creating_to_failed() {
        let result = validate_status_transition(SessionStatus::Creating, SessionStatus::Failed);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transition_active_to_paused() {
        let result = validate_status_transition(SessionStatus::Active, SessionStatus::Paused);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transition_invalid() {
        let result = validate_status_transition(SessionStatus::Completed, SessionStatus::Paused);
        assert!(result.is_err());
    }
}
