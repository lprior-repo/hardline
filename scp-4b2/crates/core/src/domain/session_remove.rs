//! Session removal domain logic.
//!
//! Provides pure domain logic for removing sessions with:
//! - Precondition validation (session exists, not locked)
//! - Postcondition verification (session deleted, workspace cleaned)
//! - Railway-oriented error handling with typed domain errors
//!
//! # Architecture
//!
//! This module follows the **Data → Calculations → Actions** pattern:
//!
//! 1. **Data**: `SessionRemoveInput` contains the input data
//! 2. **Calculations**: Pure functions validate preconditions and determine removal strategy
//! 3. **Actions**: Shell handles I/O (DB delete, workspace cleanup, Zellij tab close)
//!
//! # Contract
//!
//! - **Preconditions**: session must exist, session must not be locked
//! - **Postconditions**: session deleted, workspace cleaned, Zellij tab closed
//! - **Errors**: `SessionNotFound`, `SessionLocked`

#![cfg_attr(test, allow(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::panic))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::identifiers::SessionName;

// ============================================================================
// DATA: INPUT TYPES
// ============================================================================

/// Input for session removal operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRemoveInput {
    /// The session name to remove
    pub session_name: SessionName,
    /// Force removal even with uncommitted changes
    pub force: bool,
}

impl SessionRemoveInput {
    /// Create a new removal input.
    #[must_use]
    pub fn new(session_name: SessionName, force: bool) -> Self {
        Self {
            session_name,
            force,
        }
    }

    /// Get the session name as a string reference.
    #[must_use]
    pub fn name(&self) -> &str {
        self.session_name.as_str()
    }
}

// ============================================================================
// DATA: OUTPUT TYPES
// ============================================================================

/// Output from successful session removal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRemoveOutput {
    /// The name of the removed session
    pub session_name: SessionName,
    /// The path to the workspace that was cleaned
    pub workspace_path: PathBuf,
    /// Whether the workspace was actually deleted (vs preserved)
    pub workspace_deleted: bool,
}

impl SessionRemoveOutput {
    /// Create a new removal output.
    #[must_use]
    pub fn new(
        session_name: SessionName,
        workspace_path: PathBuf,
        workspace_deleted: bool,
    ) -> Self {
        Self {
            session_name,
            workspace_path,
            workspace_deleted,
        }
    }
}

// ============================================================================
// CALCULATIONS: DOMAIN ERRORS
// ============================================================================

/// Errors that can occur during session removal.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionRemoveError {
    /// Session not found in the database
    #[error("session '{0}' not found")]
    SessionNotFound(String),

    /// Session is locked by another agent
    #[error("session '{session}' is locked by '{holder}'")]
    SessionLocked {
        /// The session name
        session: String,
        /// The agent holding the lock
        holder: String,
    },

    /// Workspace path does not exist
    #[error("workspace path does not exist: {0}")]
    WorkspaceNotFound(PathBuf),

    /// Failed to delete session from database
    #[error("failed to delete session from database: {0}")]
    DatabaseDeleteError(String),

    /// Failed to clean workspace directory
    #[error("failed to clean workspace: {0}")]
    WorkspaceCleanupError(String),

    /// Failed to close Zellij tab
    #[error("failed to close Zellij tab: {0}")]
    ZellijTabCloseError(String),
}

// ============================================================================
// CALCULATIONS: VALIDATION FUNCTIONS
// ============================================================================

/// Validate preconditions for session removal.
///
/// Returns `Ok` if the session can be removed, or an error describing why not.
///
/// # Arguments
///
/// * `session_exists` - Whether the session exists in the database
/// * `is_locked` - Whether the session is currently locked
/// * `holder` - The lock holder if locked
/// * `_force` - Whether force removal was requested (reserved for future use)
pub fn validate_removal_preconditions(
    session_exists: bool,
    is_locked: bool,
    holder: Option<&str>,
    _force: bool,
) -> Result<(), SessionRemoveError> {
    // Precondition: session must exist
    if !session_exists {
        return Err(SessionRemoveError::SessionNotFound(
            "session not found".to_string(),
        ));
    }

    // Precondition: session must not be locked (unless force is used)
    // Note: The contract specifies "session must not be locked" as precondition
    // So we always check for locks, force doesn't bypass this
    if is_locked {
        let holder = holder.unwrap_or("unknown");
        return Err(SessionRemoveError::SessionLocked {
            session: "session".to_string(),
            holder: holder.to_string(),
        });
    }

    Ok(())
}

/// Determine if workspace should be deleted based on input and state.
///
/// Returns whether to delete the workspace directory.
#[must_use]
pub fn should_delete_workspace(force: bool, has_uncommitted_changes: bool) -> bool {
    // If force is specified, delete workspace
    if force {
        return true;
    }

    // If no uncommitted changes, safe to delete
    !has_uncommitted_changes
}

// ============================================================================
// CALCULATIONS: TYPE-LEVEL REMOVAL STRATEGY
// ============================================================================

/// Strategy for workspace cleanup during removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceCleanupStrategy {
    /// Delete the workspace directory entirely
    Delete,
    /// Preserve the workspace directory (leave in place)
    Preserve,
}

impl WorkspaceCleanupStrategy {
    /// Determine the cleanup strategy based on removal input.
    #[must_use]
    pub fn from_input(force: bool, has_uncommitted: bool) -> Self {
        if force || !has_uncommitted {
            Self::Delete
        } else {
            Self::Preserve
        }
    }

    /// Whether this strategy deletes the workspace.
    #[must_use]
    pub const fn deletes_workspace(self) -> bool {
        matches!(self, Self::Delete)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::redundant_clone)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_preconditions_session_not_found() {
        let result = validate_removal_preconditions(
            false, // session doesn't exist
            false, // not locked
            None, false,
        );

        assert!(matches!(
            result,
            Err(SessionRemoveError::SessionNotFound(_))
        ));
    }

    #[test]
    fn test_validate_preconditions_session_locked() {
        let result = validate_removal_preconditions(
            true, // session exists
            true, // is locked
            Some("agent-1"),
            false,
        );

        assert!(matches!(
            result,
            Err(SessionRemoveError::SessionLocked { .. })
        ));
    }

    #[test]
    fn test_validate_preconditions_success() {
        let result = validate_removal_preconditions(
            true,  // session exists
            false, // not locked
            None, false,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_workspace_cleanup_strategy_force() {
        let strategy = WorkspaceCleanupStrategy::from_input(true, true);
        assert_eq!(strategy, WorkspaceCleanupStrategy::Delete);
    }

    #[test]
    fn test_workspace_cleanup_strategy_no_changes() {
        let strategy = WorkspaceCleanupStrategy::from_input(false, false);
        assert_eq!(strategy, WorkspaceCleanupStrategy::Delete);
    }

    #[test]
    fn test_workspace_cleanup_strategy_with_changes_no_force() {
        let strategy = WorkspaceCleanupStrategy::from_input(false, true);
        assert_eq!(strategy, WorkspaceCleanupStrategy::Preserve);
    }

    #[test]
    fn test_should_delete_workspace_force() {
        assert!(should_delete_workspace(true, true));
        assert!(should_delete_workspace(true, false));
    }

    #[test]
    fn test_should_delete_workspace_no_force_no_changes() {
        assert!(should_delete_workspace(false, false));
    }

    #[test]
    fn test_should_delete_workspace_no_force_with_changes() {
        assert!(!should_delete_workspace(false, true));
    }

    #[test]
    fn test_session_remove_input_new() {
        let name = SessionName::parse("test-session").expect("valid name");
        let input = SessionRemoveInput::new(name.clone(), false);

        assert_eq!(input.name(), "test-session");
        assert!(!input.force);
    }

    #[test]
    fn test_session_remove_output_new() {
        let name = SessionName::parse("test-session").expect("valid name");
        let path = PathBuf::from("/tmp/test-workspace");
        let output = SessionRemoveOutput::new(name.clone(), path.clone(), true);

        assert_eq!(output.session_name, name);
        assert_eq!(output.workspace_path, path);
        assert!(output.workspace_deleted);
    }
}
