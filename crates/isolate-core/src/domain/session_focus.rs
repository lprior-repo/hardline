//! Session focus domain logic.
//!
//! Provides pure domain logic for focusing sessions with:
//! - Precondition validation (session exists, session is active)
//! - Focus state tracking (current focused session)
//! - Railway-oriented error handling with typed domain errors
//!
//! # Architecture
//!
//! This module follows the **Data → Calculations → Actions** pattern:
//!
//! 1. **Data**: `SessionFocusInput` contains the input data
//! 2. **Calculations**: Pure functions validate preconditions and determine focus state
//! 3. **Actions**: Shell handles I/O (workspace switch, Zellij tab focus, environment update)
//!
//! # Contract
//!
//! - **Preconditions**: session must exist, session must be in active/paused state
//! - **Postconditions**: session is marked as focused, working directory changed
//! - **Errors**: `SessionNotFound`, `SessionNotActive`, `SessionAlreadyFocused`

#![cfg_attr(test, allow(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::panic))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::identifiers::SessionName;

// ============================================================================
// DATA: INPUT TYPES
// ============================================================================

/// Input for session focus operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFocusInput {
    /// The session name to focus
    pub session_name: SessionName,
    /// Whether to force focus even if already focused
    pub force: bool,
}

impl SessionFocusInput {
    /// Create a new focus input.
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

/// Output from successful session focus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFocusOutput {
    /// The name of the focused session
    pub session_name: SessionName,
    /// The path to the workspace
    pub workspace_path: PathBuf,
    /// The previous session that was focused (if any)
    pub previous_session: Option<SessionName>,
    /// Whether the workspace was actually switched
    pub workspace_switched: bool,
}

impl SessionFocusOutput {
    /// Create a new focus output.
    #[must_use]
    pub fn new(
        session_name: SessionName,
        workspace_path: PathBuf,
        previous_session: Option<SessionName>,
        workspace_switched: bool,
    ) -> Self {
        Self {
            session_name,
            workspace_path,
            previous_session,
            workspace_switched,
        }
    }
}

/// Represents the current focus state of sessions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SessionFocusState {
    /// The currently focused session name (if any)
    pub focused_session: Option<SessionName>,
    /// Session names ordered by recent focus
    pub focus_order: Vec<SessionName>,
}

impl SessionFocusState {
    /// Create a new empty focus state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create focus state with an initial focused session.
    #[must_use]
    pub fn with_focused(session_name: SessionName) -> Self {
        Self {
            focused_session: Some(session_name.clone()),
            focus_order: vec![session_name],
        }
    }

    /// Check if a session is currently focused.
    #[must_use]
    pub fn is_focused(&self, session_name: &SessionName) -> bool {
        self.focused_session
            .as_ref()
            .is_some_and(|s| s == session_name)
    }

    /// Get the currently focused session name.
    #[must_use]
    pub fn focused(&self) -> Option<&SessionName> {
        self.focused_session.as_ref()
    }
}

// ============================================================================
// CALCULATIONS: DOMAIN ERRORS
// ============================================================================

/// Errors that can occur during session focus.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionFocusError {
    /// Session not found in the database
    #[error("session '{0}' not found")]
    SessionNotFound(String),

    /// Session is not in a focusable state (must be active or paused)
    #[error("session '{0}' is not active or paused, cannot focus")]
    SessionNotActive(String),

    /// Session is already the focused session
    #[error("session '{0}' is already focused")]
    SessionAlreadyFocused(String),

    /// Workspace path does not exist
    #[error("workspace path does not exist: {0}")]
    WorkspaceNotFound(PathBuf),

    /// Failed to switch workspace
    #[error("failed to switch workspace: {0}")]
    WorkspaceSwitchError(String),

    /// Failed to update focus state
    #[error("failed to update focus state: {0}")]
    StateUpdateError(String),
}

// ============================================================================
// CALCULATIONS: VALIDATION FUNCTIONS
// ============================================================================

/// Possible session states that can be focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionFocusableState {
    /// Session is active and ready for work
    Active,
    /// Session is paused but can be resumed
    Paused,
}

impl SessionFocusableState {
    /// Check if a session state string represents a focusable state.
    #[must_use]
    pub fn from_state_str(state: &str) -> Option<Self> {
        match state.to_lowercase().as_str() {
            "active" => Some(Self::Active),
            "paused" => Some(Self::Paused),
            _ => None,
        }
    }

    /// Returns true if this state is focusable.
    #[must_use]
    pub const fn is_focusable(self) -> bool {
        matches!(self, Self::Active | Self::Paused)
    }
}

/// Validate preconditions for session focus.
///
/// Returns `Ok` if the session can be focused, or an error describing why not.
///
/// # Arguments
///
/// * `session_exists` - Whether the session exists in the database
/// * `session_state` - The current state of the session as a string
/// * `is_already_focused` - Whether the session is already the focused session
/// * `force` - Whether force focus was requested
pub fn validate_focus_preconditions(
    session_exists: bool,
    session_state: Option<&str>,
    is_already_focused: bool,
    force: bool,
) -> Result<(), SessionFocusError> {
    // Precondition: session must exist
    if !session_exists {
        return Err(SessionFocusError::SessionNotFound(
            "session not found".to_string(),
        ));
    }

    // Precondition: session must be in a focusable state (active or paused)
    let state = session_state
        .and_then(SessionFocusableState::from_state_str)
        .ok_or_else(|| SessionFocusError::SessionNotActive("unknown state".to_string()))?;

    if !state.is_focusable() {
        return Err(SessionFocusError::SessionNotActive(
            session_state.unwrap_or("unknown").to_string(),
        ));
    }

    // Precondition: session must not already be focused (unless force is used)
    if is_already_focused && !force {
        return Err(SessionFocusError::SessionAlreadyFocused(
            "session already focused".to_string(),
        ));
    }

    Ok(())
}

/// Determine if workspace should be switched.
///
/// Returns whether the workspace directory needs to be changed.
#[must_use]
pub fn should_switch_workspace(
    current_focus: Option<&SessionName>,
    target_session: &SessionName,
) -> bool {
    // Always switch if no session is currently focused
    if current_focus.is_none() {
        return true;
    }

    // Switch if the target is different from current
    current_focus.is_some_and(|current| current != target_session)
}

/// Update focus state after successful focus operation.
///
/// Returns a new focus state with the session added to focus order.
#[must_use]
pub fn update_focus_state(
    state: &SessionFocusState,
    session_name: SessionName,
) -> SessionFocusState {
    // Remove session from existing position in focus order
    let mut new_order: Vec<SessionName> = state
        .focus_order
        .iter()
        .filter(|s| *s != &session_name)
        .cloned()
        .collect();

    // Add session to the front (most recent)
    new_order.insert(0, session_name.clone());

    // Keep only the last 10 sessions in focus order
    let focus_order = new_order.into_iter().take(10).collect();

    SessionFocusState {
        focused_session: Some(session_name),
        focus_order,
    }
}

// ============================================================================
// CALCULATIONS: FOCUS RESULT
// ============================================================================

/// Build focus output from operation details.
#[must_use]
pub fn build_focus_output(
    session_name: SessionName,
    workspace_path: PathBuf,
    previous_focus: Option<SessionName>,
    workspace_switched: bool,
) -> SessionFocusOutput {
    SessionFocusOutput::new(
        session_name,
        workspace_path,
        previous_focus,
        workspace_switched,
    )
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
        let result = validate_focus_preconditions(
            false,          // session doesn't exist
            Some("active"), // state would be active
            false,          // not already focused
            false,
        );

        assert!(matches!(result, Err(SessionFocusError::SessionNotFound(_))));
    }

    #[test]
    fn test_validate_preconditions_session_not_active() {
        let result = validate_focus_preconditions(
            true,              // session exists
            Some("completed"), // state is completed
            false,             // not already focused
            false,
        );

        assert!(matches!(
            result,
            Err(SessionFocusError::SessionNotActive(_))
        ));
    }

    #[test]
    fn test_validate_preconditions_session_already_focused() {
        let result = validate_focus_preconditions(
            true,           // session exists
            Some("active"), // state is active
            true,           // already focused
            false,          // no force
        );

        assert!(matches!(
            result,
            Err(SessionFocusError::SessionAlreadyFocused(_))
        ));
    }

    #[test]
    fn test_validate_preconditions_success() {
        let result = validate_focus_preconditions(
            true,           // session exists
            Some("active"), // state is active
            false,          // not already focused
            false,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_preconditions_force_overrides_already_focused() {
        let result = validate_focus_preconditions(
            true,           // session exists
            Some("active"), // state is active
            true,           // already focused
            true,           // force
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_preconditions_paused_is_focusable() {
        let result = validate_focus_preconditions(
            true,           // session exists
            Some("paused"), // state is paused
            false,          // not already focused
            false,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_should_switch_workspace_no_current_focus() {
        let session = SessionName::parse("test-session").expect("valid name");
        let should_switch = should_switch_workspace(None, &session);

        assert!(should_switch);
    }

    #[test]
    fn test_should_switch_workspace_different_session() {
        let current = SessionName::parse("current-session").expect("valid name");
        let target = SessionName::parse("target-session").expect("valid name");
        let should_switch = should_switch_workspace(Some(&current), &target);

        assert!(should_switch);
    }

    #[test]
    fn test_should_switch_workspace_same_session() {
        let session = SessionName::parse("test-session").expect("valid name");
        let should_switch = should_switch_workspace(Some(&session), &session);

        assert!(!should_switch);
    }

    #[test]
    fn test_session_focus_state_new() {
        let state = SessionFocusState::new();
        assert!(state.focused_session.is_none());
        assert!(state.focus_order.is_empty());
    }

    #[test]
    fn test_session_focus_state_with_focused() {
        let name = SessionName::parse("test-session").expect("valid name");
        let state = SessionFocusState::with_focused(name.clone());

        assert!(state.is_focused(&name));
        assert_eq!(state.focused(), Some(&name));
    }

    #[test]
    fn test_update_focus_state() {
        let name1 = SessionName::parse("session-1").expect("valid name");
        let name2 = SessionName::parse("session-2").expect("valid name");
        let name3 = SessionName::parse("session-3").expect("valid name");

        // Start with session-1 focused
        let state = SessionFocusState::with_focused(name1.clone());

        // Focus session-2
        let new_state = update_focus_state(&state, name2.clone());

        assert!(new_state.is_focused(&name2));
        assert_eq!(new_state.focused(), Some(&name2));
        // session-2 should be first, session-1 second
        assert_eq!(new_state.focus_order.first(), Some(&name2));
    }

    #[test]
    fn test_update_focus_state_maintains_order() {
        let name1 = SessionName::parse("session-1").expect("valid name");
        let name2 = SessionName::parse("session-2").expect("valid name");
        let name3 = SessionName::parse("session-3").expect("valid name");

        let state = SessionFocusState {
            focused_session: Some(name1.clone()),
            focus_order: vec![name1.clone(), name2.clone()],
        };

        // Focus session-3
        let new_state = update_focus_state(&state, name3.clone());

        // Should be: session-3, session-1, session-2
        assert_eq!(new_state.focus_order.len(), 3);
        assert_eq!(new_state.focus_order[0], name3);
        assert_eq!(new_state.focus_order[1], name1);
        assert_eq!(new_state.focus_order[2], name2);
    }

    #[test]
    fn test_session_focus_input_new() {
        let name = SessionName::parse("test-session").expect("valid name");
        let input = SessionFocusInput::new(name.clone(), false);

        assert_eq!(input.name(), "test-session");
        assert!(!input.force);
    }

    #[test]
    fn test_session_focus_output_new() {
        let name = SessionName::parse("test-session").expect("valid name");
        let path = PathBuf::from("/tmp/test-workspace");
        let output = SessionFocusOutput::new(name.clone(), path.clone(), None, true);

        assert_eq!(output.session_name, name);
        assert_eq!(output.workspace_path, path);
        assert!(output.previous_session.is_none());
        assert!(output.workspace_switched);
    }

    #[test]
    fn test_session_focusable_state_from_state_str() {
        assert_eq!(
            SessionFocusableState::from_state_str("active"),
            Some(SessionFocusableState::Active)
        );
        assert_eq!(
            SessionFocusableState::from_state_str("paused"),
            Some(SessionFocusableState::Paused)
        );
        assert_eq!(SessionFocusableState::from_state_str("completed"), None);
        assert_eq!(SessionFocusableState::from_state_str("invalid"), None);
    }

    #[test]
    fn test_build_focus_output() {
        let name = SessionName::parse("test-session").expect("valid name");
        let prev = SessionName::parse("previous-session").expect("valid name");
        let path = PathBuf::from("/tmp/test");

        let output = build_focus_output(name.clone(), path.clone(), Some(prev.clone()), true);

        assert_eq!(output.session_name, name);
        assert_eq!(output.workspace_path, path);
        assert_eq!(output.previous_session, Some(prev));
        assert!(output.workspace_switched);
    }
}
