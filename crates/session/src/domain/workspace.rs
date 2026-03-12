//! Workspace aggregate for session-based workspaces.
//!
//! This module provides the Workspace aggregate with full lifecycle management:
//! - States: Created → Working → Ready → Merged | Conflict | Abandoned
//! - Invariants enforced via type system and runtime checks

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::workspace_state::WorkspaceState;
use crate::error::SessionError;

// Re-export for convenience
use std::result::Result;

/// Unique workspace identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn new(id: impl Into<String>) -> Result<Self, SessionError> {
        let id = id.into();
        if id.is_empty() {
            return Err(SessionError::InvalidWorkspaceId(
                "ID cannot be empty".into(),
            ));
        }
        Ok(Self(id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated workspace name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    pub const MAX_LENGTH: usize = 100;

    pub fn new(name: impl Into<String>) -> Result<Self, SessionError> {
        let name = name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(SessionError::InvalidWorkspaceName(
                "Name cannot be empty".into(),
            ));
        }
        if trimmed.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidWorkspaceName(format!(
                "Name exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated workspace path
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspacePath(String);

impl WorkspacePath {
    pub fn new(path: impl Into<String>) -> Result<Self, SessionError> {
        let path = path.into();
        if path.is_empty() {
            return Err(SessionError::InvalidWorkspacePath(
                "Path cannot be empty".into(),
            ));
        }
        // Basic path validation - must start with / or be a relative valid path
        if !path.starts_with('/') && !path.starts_with('.') {
            return Err(SessionError::InvalidWorkspacePath(
                "Path must be absolute or relative".into(),
            ));
        }
        Ok(Self(path))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspacePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Workspace aggregate representing an isolated execution environment.
///
/// # State Machine
/// - Created: Workspace has been created
/// - Working: Workspace is being actively worked on
/// - Ready: Workspace is ready for review/merge
/// - Merged: Workspace has been merged (terminal)
/// - Conflict: Workspace has merge conflicts (terminal)
/// - Abandoned: Workspace was abandoned (terminal)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    id: WorkspaceId,
    name: WorkspaceName,
    path: WorkspacePath,
    state: WorkspaceState,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Workspace {
    /// Create a new workspace in Created state.
    ///
    /// # Preconditions (P1)
    /// - name must be non-empty
    /// - path must be non-empty and valid
    ///
    /// # Postconditions (Q1)
    /// - state = Created
    /// - created_at = updated_at
    pub fn create(name: WorkspaceName, path: WorkspacePath) -> Result<Self, SessionError> {
        let now = Utc::now();
        Ok(Self {
            id: WorkspaceId::new(format!("ws-{}", uuid::Uuid::new_v4()))?,
            name,
            path,
            state: WorkspaceState::Created,
            created_at: now,
            updated_at: now,
        })
    }

    /// Start working on the workspace (transition from Created to Working).
    ///
    /// # Preconditions (P2)
    /// - workspace must be in Created state
    ///
    /// # Postconditions (Q2)
    /// - state = Working
    pub fn start_working(&self) -> Result<Self, SessionError> {
        if self.state != WorkspaceState::Created {
            return Err(SessionError::InvalidTransition {
                from: self.state,
                to: WorkspaceState::Working,
            });
        }

        let mut new_state = self.clone();
        new_state.state = WorkspaceState::Working;
        new_state.updated_at = Utc::now();
        Ok(new_state)
    }

    /// Mark workspace as ready for review (transition from Working to Ready).
    ///
    /// # Preconditions (P3)
    /// - workspace must be in Working state
    ///
    /// # Postconditions (Q3)
    /// - state = Ready
    pub fn mark_ready(&self) -> Result<Self, SessionError> {
        if self.state != WorkspaceState::Working {
            return Err(SessionError::InvalidTransition {
                from: self.state,
                to: WorkspaceState::Ready,
            });
        }

        let mut new_state = self.clone();
        new_state.state = WorkspaceState::Ready;
        new_state.updated_at = Utc::now();
        Ok(new_state)
    }

    /// Merge the workspace successfully.
    ///
    /// # Preconditions (P4)
    /// - workspace must be in Ready state
    ///
    /// # Postconditions (Q4)
    /// - state = Merged (terminal)
    pub fn merge(&self) -> Result<Self, SessionError> {
        if self.state != WorkspaceState::Ready {
            return Err(SessionError::InvalidTransition {
                from: self.state,
                to: WorkspaceState::Merged,
            });
        }

        let mut new_state = self.clone();
        new_state.state = WorkspaceState::Merged;
        new_state.updated_at = Utc::now();
        Ok(new_state)
    }

    /// Mark the workspace as having merge conflicts.
    ///
    /// # Preconditions (P5)
    /// - workspace must be in Ready state
    ///
    /// # Postconditions (Q5)
    /// - state = Conflict (terminal)
    pub fn mark_conflict(&self) -> Result<Self, SessionError> {
        if self.state != WorkspaceState::Ready {
            return Err(SessionError::InvalidTransition {
                from: self.state,
                to: WorkspaceState::Conflict,
            });
        }

        let mut new_state = self.clone();
        new_state.state = WorkspaceState::Conflict;
        new_state.updated_at = Utc::now();
        Ok(new_state)
    }

    /// Abandon the workspace.
    ///
    /// # Preconditions (P6)
    /// - workspace must NOT be in a terminal state (Merged, Conflict, Abandoned)
    ///
    /// # Postconditions (Q6)
    /// - state = Abandoned (terminal)
    pub fn abandon(&self) -> Result<Self, SessionError> {
        if self.state.is_terminal() {
            return Err(SessionError::InvalidTransition {
                from: self.state,
                to: WorkspaceState::Abandoned,
            });
        }

        let mut new_state = self.clone();
        new_state.state = WorkspaceState::Abandoned;
        new_state.updated_at = Utc::now();
        Ok(new_state)
    }

    /// Check if workspace is ready.
    ///
    /// # Postconditions (Q8)
    /// - returns true iff state == Ready
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.state.is_ready()
    }

    /// Check if workspace is working.
    ///
    /// # Postconditions (Q9)
    /// - returns true iff state == Working
    #[must_use]
    pub fn is_working(&self) -> bool {
        self.state.is_working()
    }

    /// Check if workspace is in a terminal state.
    ///
    /// # Postconditions (Q10)
    /// - returns true iff state ∈ {Merged, Conflict, Abandoned}
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Get the workspace ID
    #[must_use]
    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    /// Get the workspace name
    #[must_use]
    pub fn name(&self) -> &WorkspaceName {
        &self.name
    }

    /// Get the workspace path
    #[must_use]
    pub fn path(&self) -> &WorkspacePath {
        &self.path
    }

    /// Get the current state
    #[must_use]
    pub fn state(&self) -> WorkspaceState {
        self.state
    }

    /// Get creation timestamp
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Get last update timestamp
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_create_sets_created_state() {
        let name = WorkspaceName::new("test-workspace").unwrap();
        let path = WorkspacePath::new("/tmp/test").unwrap();
        let workspace = Workspace::create(name, path).unwrap();

        assert_eq!(workspace.state(), WorkspaceState::Created);
        assert!(!workspace.is_working());
        assert!(!workspace.is_ready());
        assert!(!workspace.is_terminal());
    }

    #[test]
    fn workspace_start_working_transitions_to_working() {
        let name = WorkspaceName::new("test-workspace").unwrap();
        let path = WorkspacePath::new("/tmp/test").unwrap();
        let workspace = Workspace::create(name, path).unwrap();
        let working = workspace.start_working().unwrap();

        assert_eq!(working.state(), WorkspaceState::Working);
        assert!(working.is_working());
    }

    #[test]
    fn workspace_mark_ready_transitions_to_ready() {
        let name = WorkspaceName::new("test-workspace").unwrap();
        let path = WorkspacePath::new("/tmp/test").unwrap();
        let workspace = Workspace::create(name, path).unwrap();
        let working = workspace.start_working().unwrap();
        let ready = working.mark_ready().unwrap();

        assert_eq!(ready.state(), WorkspaceState::Ready);
        assert!(ready.is_ready());
    }

    #[test]
    fn workspace_merge_transitions_to_merged() {
        let name = WorkspaceName::new("test-workspace").unwrap();
        let path = WorkspacePath::new("/tmp/test").unwrap();
        let workspace = Workspace::create(name, path).unwrap();
        let working = workspace.start_working().unwrap();
        let ready = working.mark_ready().unwrap();
        let merged = ready.merge().unwrap();

        assert_eq!(merged.state(), WorkspaceState::Merged);
        assert!(merged.is_terminal());
    }

    #[test]
    fn workspace_mark_conflict_transitions_to_conflict() {
        let name = WorkspaceName::new("test-workspace").unwrap();
        let path = WorkspacePath::new("/tmp/test").unwrap();
        let workspace = Workspace::create(name, path).unwrap();
        let working = workspace.start_working().unwrap();
        let ready = working.mark_ready().unwrap();
        let conflict = ready.mark_conflict().unwrap();

        assert_eq!(conflict.state(), WorkspaceState::Conflict);
        assert!(conflict.is_terminal());
    }

    #[test]
    fn workspace_abandon_transitions_to_abandoned() {
        let name = WorkspaceName::new("test-workspace").unwrap();
        let path = WorkspacePath::new("/tmp/test").unwrap();
        let workspace = Workspace::create(name, path).unwrap();
        let abandoned = workspace.abandon().unwrap();

        assert_eq!(abandoned.state(), WorkspaceState::Abandoned);
        assert!(abandoned.is_terminal());
    }

    #[test]
    fn workspace_cannot_start_working_from_working() {
        let name = WorkspaceName::new("test-workspace").unwrap();
        let path = WorkspacePath::new("/tmp/test").unwrap();
        let workspace = Workspace::create(name, path).unwrap();
        let working = workspace.start_working().unwrap();
        let result = working.start_working();

        assert!(result.is_err());
    }

    #[test]
    fn workspace_cannot_mark_ready_from_created() {
        let name = WorkspaceName::new("test-workspace").unwrap();
        let path = WorkspacePath::new("/tmp/test").unwrap();
        let workspace = Workspace::create(name, path).unwrap();
        let result = workspace.mark_ready();

        assert!(result.is_err());
    }

    #[test]
    fn workspace_cannot_abandon_from_terminal_state() {
        let name = WorkspaceName::new("test-workspace").unwrap();
        let path = WorkspacePath::new("/tmp/test").unwrap();
        let workspace = Workspace::create(name, path).unwrap();
        let working = workspace.start_working().unwrap();
        let ready = working.mark_ready().unwrap();
        let merged = ready.merge().unwrap();
        let result = merged.abandon();

        assert!(result.is_err());
    }
}
