//! Session aggregate root with business rules and invariants.
//!
//! The Session aggregate represents a development session with:
//! - Unique identity (`SessionId`)
//! - Human-readable name (`SessionName`)
//! - Branch state (detached or on branch)
//! - Workspace location
//!
//! # Invariants
//!
//! 1. Session names must be unique within a workspace
//! 2. Active sessions must have valid workspace paths
//! 3. Branch transitions must be valid (detached <-> branch, branch <-> branch)

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use thiserror::Error;

use crate::domain::{
    identifiers::{SessionId, SessionName},
    session::BranchState,
};

// ============================================================================
// DOMAIN ERRORS
// ============================================================================

/// Errors that can occur during session operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    /// Invalid branch transition
    #[error("invalid branch transition: {from:?} -> {to:?}")]
    InvalidBranchTransition { from: BranchState, to: BranchState },

    /// Workspace path does not exist
    #[error("workspace path does not exist: {0}")]
    WorkspaceNotFound(PathBuf),

    /// Session is not active
    #[error("session is not active")]
    NotActive,

    /// Cannot activate session with invalid state
    #[error("cannot activate session: invalid state")]
    CannotActivate,

    /// Session name conflicts with existing session
    #[error("session name already exists: {0}")]
    NameAlreadyExists(SessionName),
}

// ============================================================================
// SESSION AGGREGATE ROOT
// ============================================================================

/// Session aggregate root.
///
/// Enforces all business rules and invariants for sessions.
/// All state transitions go through validated methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// Human-readable session name
    pub name: SessionName,
    /// Branch state (detached or on branch)
    pub branch: BranchState,
    /// Absolute path to workspace root
    pub workspace_path: PathBuf,
}

impl Session {
    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Create a new session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::WorkspaceNotFound` if workspace path doesn't exist.
    pub fn new(
        id: SessionId,
        name: SessionName,
        branch: BranchState,
        workspace_path: PathBuf,
    ) -> Result<Self, SessionError> {
        // Validate workspace exists
        if !workspace_path.exists() {
            return Err(SessionError::WorkspaceNotFound(workspace_path));
        }

        Ok(Self {
            id,
            name,
            branch,
            workspace_path,
        })
    }

    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// Check if session is active (has valid branch and workspace).
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.branch.is_detached() && self.workspace_path.exists()
    }

    /// Get the branch name if on a branch.
    #[must_use]
    pub fn branch_name(&self) -> Option<&str> {
        self.branch.branch_name()
    }

    // ========================================================================
    // STATE TRANSITION METHODS
    // ========================================================================

    /// Transition to a new branch state.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InvalidBranchTransition` if transition is invalid.
    pub fn transition_branch(&self, new_branch: BranchState) -> Result<Self, SessionError> {
        if !self.branch.can_transition_to(&new_branch) {
            return Err(SessionError::InvalidBranchTransition {
                from: self.branch.clone(),
                to: new_branch,
            });
        }

        Ok(Self {
            branch: new_branch,
            ..self.clone()
        })
    }

    /// Change the workspace path.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::WorkspaceNotFound` if new path doesn't exist.
    pub fn change_workspace(&self, new_path: PathBuf) -> Result<Self, SessionError> {
        if !new_path.exists() {
            return Err(SessionError::WorkspaceNotFound(new_path));
        }

        Ok(Self {
            workspace_path: new_path,
            ..self.clone()
        })
    }

    /// Rename the session.
    ///
    /// Note: The caller is responsible for ensuring name uniqueness.
    #[must_use]
    pub fn rename(&self, new_name: SessionName) -> Self {
        Self {
            name: new_name,
            ..self.clone()
        }
    }

    // ========================================================================
    // VALIDATION METHODS
    // ========================================================================

    /// Validate that the session can be activated.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::CannotActivate` if session cannot be activated.
    pub fn validate_can_activate(&self) -> Result<(), SessionError> {
        if !self.is_active() {
            return Err(SessionError::CannotActivate);
        }
        Ok(())
    }

    /// Validate that the session is in a valid state.
    ///
    /// A session is valid if:
    /// - Workspace path exists
    /// - Branch state is valid
    ///
    /// # Errors
    ///
    /// Returns `SessionError::WorkspaceNotFound` if workspace path doesn't exist.
    pub fn validate(&self) -> Result<(), SessionError> {
        if !self.workspace_path.exists() {
            return Err(SessionError::WorkspaceNotFound(self.workspace_path.clone()));
        }
        Ok(())
    }

    // ========================================================================
    // BUILDER PATTERN
    // ========================================================================

    /// Create a builder for constructing or modifying sessions.
    #[must_use]
    pub fn builder() -> SessionBuilder {
        SessionBuilder::new()
    }
}

// ============================================================================
// SESSION BUILDER
// ============================================================================

/// Builder for constructing sessions.
///
/// Provides a fluent interface for session creation with validation.
#[derive(Debug, Default)]
pub struct SessionBuilder {
    id: Option<SessionId>,
    name: Option<SessionName>,
    branch: Option<BranchState>,
    workspace_path: Option<PathBuf>,
}

impl SessionBuilder {
    /// Create a new session builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the session ID.
    #[must_use]
    pub fn id(mut self, id: SessionId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the session name.
    #[must_use]
    pub fn name(mut self, name: SessionName) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the branch state.
    #[must_use]
    pub fn branch(mut self, branch: BranchState) -> Self {
        self.branch = Some(branch);
        self
    }

    /// Set the workspace path.
    #[must_use]
    pub fn workspace_path(mut self, path: PathBuf) -> Self {
        self.workspace_path = Some(path);
        self
    }

    /// Build the session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError` if:
    /// - Required fields are missing
    /// - Workspace path doesn't exist
    pub fn build(self) -> Result<Session, SessionError> {
        let id = self.id.ok_or(SessionError::CannotActivate)?;
        let name = self.name.ok_or(SessionError::CannotActivate)?;
        let branch = self.branch.ok_or(SessionError::CannotActivate)?;
        let workspace_path = self.workspace_path.ok_or(SessionError::CannotActivate)?;

        Session::new(id, name, branch, workspace_path)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let id = SessionId::parse("test-session-1").expect("valid id");
        let name = SessionName::parse("test-session").expect("valid name");
        let workspace = PathBuf::from("/tmp"); // Assume exists for test

        let session = Session::new(
            id.clone(),
            name.clone(),
            BranchState::OnBranch {
                name: "main".to_string(),
            },
            workspace,
        )
        .expect("session created");

        assert_eq!(session.id, id);
        assert_eq!(session.name, name);
        assert_eq!(session.branch_name(), Some("main"));
    }

    #[test]
    fn test_branch_transition() {
        let id = SessionId::parse("test-3").expect("valid id");
        let name = SessionName::parse("test").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session =
            Session::new(id, name, BranchState::Detached, workspace).expect("session created");

        // Detached -> OnBranch is valid
        let new_branch = BranchState::OnBranch {
            name: "main".to_string(),
        };
        let session = session
            .transition_branch(new_branch.clone())
            .expect("transition valid");

        assert_eq!(session.branch, new_branch);

        // OnBranch -> OnBranch is valid (switching branches)
        let another_branch = BranchState::OnBranch {
            name: "feature".to_string(),
        };
        let session = session
            .transition_branch(another_branch.clone())
            .expect("switch branches valid");

        assert_eq!(session.branch, another_branch);

        // OnBranch -> Detached is valid
        let session = session
            .transition_branch(BranchState::Detached)
            .expect("detach valid");

        assert!(session.branch.is_detached());
    }

    #[test]
    fn test_invalid_branch_transition() {
        let id = SessionId::parse("test-4").expect("valid id");
        let name = SessionName::parse("test").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session =
            Session::new(id, name, BranchState::Detached, workspace).expect("session created");

        // Detached -> Detached is invalid (no self-loop)
        let result = session.transition_branch(BranchState::Detached);
        assert!(matches!(
            result,
            Err(SessionError::InvalidBranchTransition { .. })
        ));
    }

    #[test]
    fn test_workspace_not_found() {
        let id = SessionId::parse("test-7").expect("valid id");
        let name = SessionName::parse("test").expect("valid name");
        let workspace = PathBuf::from("/nonexistent/path/that/does/not/exist");

        let result = Session::new(id, name, BranchState::Detached, workspace);

        assert!(matches!(result, Err(SessionError::WorkspaceNotFound(_))));
    }

    #[test]
    fn test_builder() {
        let id = SessionId::parse("test-8").expect("valid id");
        let name = SessionName::parse("test").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session = Session::builder()
            .id(id.clone())
            .name(name.clone())
            .branch(BranchState::OnBranch {
                name: "main".to_string(),
            })
            .workspace_path(workspace)
            .build()
            .expect("builder works");

        assert_eq!(session.id, id);
        assert_eq!(session.name, name);
    }

    #[test]
    fn test_rename() {
        let id = SessionId::parse("test-9").expect("valid id");
        let name1 = SessionName::parse("name1").expect("valid name");
        let name2 = SessionName::parse("name2").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session =
            Session::new(id, name1, BranchState::Detached, workspace).expect("session created");

        let renamed = session.rename(name2.clone());
        assert_eq!(renamed.name, name2);
    }
}
