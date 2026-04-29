//! Workspace aggregate for session-based workspaces.
//!
//! This module provides the Workspace aggregate with full lifecycle management:
//! - States: Created → Working → Ready → Merged | Conflict | Abandoned
//! - Invariants enforced via type system and runtime checks

// Re-export for convenience
use std::result::Result;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{domain::workspace_state::WorkspaceState, error::SessionError};

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
    pub const fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Get the workspace ID
    #[must_use]
    pub const fn id(&self) -> &WorkspaceId {
        &self.id
    }

    /// Get the workspace name
    #[must_use]
    pub const fn name(&self) -> &WorkspaceName {
        &self.name
    }

    /// Get the workspace path
    #[must_use]
    pub const fn path(&self) -> &WorkspacePath {
        &self.path
    }

    /// Get the current state
    #[must_use]
    pub const fn state(&self) -> WorkspaceState {
        self.state
    }

    /// Get creation timestamp
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Get last update timestamp
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
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

    // =========================================================================
    // WorkspaceId Tests
    // =========================================================================

    mod workspace_id_tests {
        use super::*;

        #[test]
        fn workspace_id_valid() {
            let id = WorkspaceId::new("ws-001").expect("valid");
            assert_eq!(id.as_str(), "ws-001");
        }

        #[test]
        fn workspace_id_empty_rejects() {
            let result = WorkspaceId::new("");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidWorkspaceId(_)
            ));
        }

        #[test]
        fn workspace_id_display() {
            let id = WorkspaceId::new("ws-test").expect("valid");
            assert_eq!(format!("{id}"), "ws-test");
        }
    }

    // =========================================================================
    // WorkspaceName Tests (workspace variant)
    // =========================================================================

    mod workspace_name_tests {
        use super::*;

        #[test]
        fn workspace_name_valid() {
            let name = WorkspaceName::new("my-workspace").expect("valid");
            assert_eq!(name.as_str(), "my-workspace");
        }

        #[test]
        fn workspace_name_trims_whitespace() {
            let name = WorkspaceName::new("  padded  ").expect("valid");
            assert_eq!(name.as_str(), "padded");
        }

        #[test]
        fn workspace_name_empty_rejects() {
            let result = WorkspaceName::new("");
            assert!(result.is_err());
        }

        #[test]
        fn workspace_name_whitespace_only_rejects() {
            let result = WorkspaceName::new("   ");
            assert!(result.is_err());
        }

        #[test]
        fn workspace_name_max_length_boundary() {
            let max_name = "w".repeat(WorkspaceName::MAX_LENGTH);
            let name = WorkspaceName::new(max_name).expect("at max");
            assert_eq!(name.as_str().len(), WorkspaceName::MAX_LENGTH);
        }

        #[test]
        fn workspace_name_exceeds_max_rejects() {
            let too_long = "w".repeat(WorkspaceName::MAX_LENGTH + 1);
            let result = WorkspaceName::new(too_long);
            assert!(result.is_err());
        }

        #[test]
        fn workspace_name_display() {
            let name = WorkspaceName::new("test").expect("valid");
            assert_eq!(format!("{name}"), "test");
        }
    }

    // =========================================================================
    // WorkspacePath Tests
    // =========================================================================

    mod workspace_path_tests {
        use super::*;

        #[test]
        fn workspace_path_absolute_valid() {
            let path = WorkspacePath::new("/tmp/test-workspace").expect("valid");
            assert_eq!(path.as_str(), "/tmp/test-workspace");
        }

        #[test]
        fn workspace_path_relative_with_dot_valid() {
            let path = WorkspacePath::new("./relative/path").expect("valid");
            assert_eq!(path.as_str(), "./relative/path");
        }

        #[test]
        fn workspace_path_empty_rejects() {
            let result = WorkspacePath::new("");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidWorkspacePath(_)
            ));
        }

        #[test]
        fn workspace_path_bare_name_rejects() {
            let result = WorkspacePath::new("just-a-name");
            assert!(result.is_err());
        }

        #[test]
        fn workspace_path_display() {
            let path = WorkspacePath::new("/home/user/ws").expect("valid");
            assert_eq!(format!("{path}"), "/home/user/ws");
        }
    }

    // =========================================================================
    // Workspace Aggregate Additional Tests
    // =========================================================================

    mod workspace_aggregate_tests {
        use super::*;

        #[test]
        fn workspace_create_generates_id_with_prefix() {
            let name = WorkspaceName::new("test").expect("valid");
            let path = WorkspacePath::new("/tmp/test").expect("valid");
            let workspace = Workspace::create(name, path).expect("created");
            assert!(workspace.id().as_str().starts_with("ws-"));
        }

        #[test]
        fn workspace_create_sets_created_at_equals_updated_at() {
            let name = WorkspaceName::new("test").expect("valid");
            let path = WorkspacePath::new("/tmp/test").expect("valid");
            let workspace = Workspace::create(name, path).expect("created");
            assert_eq!(workspace.created_at(), workspace.updated_at());
        }

        #[test]
        fn workspace_getters_return_correct_values() {
            let name = WorkspaceName::new("my-ws").expect("valid");
            let path = WorkspacePath::new("/tmp/my-ws").expect("valid");
            let workspace = Workspace::create(name, path).expect("created");

            assert_eq!(workspace.name().as_str(), "my-ws");
            assert_eq!(workspace.path().as_str(), "/tmp/my-ws");
            assert_eq!(workspace.state(), WorkspaceState::Created);
        }

        #[test]
        fn workspace_abandon_from_ready_succeeds() {
            let name = WorkspaceName::new("test").expect("valid");
            let path = WorkspacePath::new("/tmp/test").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let working = ws.start_working().expect("working");
            let ready = working.mark_ready().expect("ready");
            let abandoned = ready.abandon().expect("abandon");
            assert_eq!(abandoned.state(), WorkspaceState::Abandoned);
            assert!(abandoned.is_terminal());
        }

        #[test]
        fn workspace_transition_updates_updated_at() {
            let name = WorkspaceName::new("test").expect("valid");
            let path = WorkspacePath::new("/tmp/test").expect("valid");
            let workspace = Workspace::create(name, path).expect("created");

            // Small delay to ensure time difference
            std::thread::sleep(std::time::Duration::from_millis(2));
            let working = workspace.start_working().expect("working");

            assert!(working.updated_at() >= workspace.created_at());
        }

        #[test]
        fn workspace_cannot_mark_conflict_from_working() {
            let name = WorkspaceName::new("test").expect("valid");
            let path = WorkspacePath::new("/tmp/test").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let working = ws.start_working().expect("working");
            let result = working.mark_conflict();
            assert!(result.is_err());
        }

        #[test]
        fn workspace_cannot_merge_from_created() {
            let name = WorkspaceName::new("test").expect("valid");
            let path = WorkspacePath::new("/tmp/test").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let result = ws.merge();
            assert!(result.is_err());
        }

        #[test]
        fn workspace_cannot_merge_from_working() {
            let name = WorkspaceName::new("test").expect("valid");
            let path = WorkspacePath::new("/tmp/test").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let working = ws.start_working().expect("working");
            let result = working.merge();
            assert!(result.is_err());
        }

        #[test]
        fn workspace_conflict_path() {
            let name = WorkspaceName::new("test").expect("valid");
            let path = WorkspacePath::new("/tmp/test").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let working = ws.start_working().expect("working");
            let ready = working.mark_ready().expect("ready");
            let conflict = ready.mark_conflict().expect("conflict");
            assert_eq!(conflict.state(), WorkspaceState::Conflict);
            assert!(conflict.is_terminal());
        }

        #[test]
        fn workspace_abandon_from_created_succeeds() {
            let name = WorkspaceName::new("test").expect("valid");
            let path = WorkspacePath::new("/tmp/test").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let abandoned = ws.abandon().expect("abandon");
            assert_eq!(abandoned.state(), WorkspaceState::Abandoned);
            assert!(abandoned.is_terminal());
        }
    }

    // =========================================================================
    // Workspace Serde and Lifecycle Edge-Case Tests
    // =========================================================================

    mod workspace_serde_and_lifecycle_tests {
        use super::*;

        #[test]
        fn workspace_serde_roundtrip_created() {
            let name = WorkspaceName::new("serde-ws").expect("valid");
            let path = WorkspacePath::new("/tmp/serde").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let json = serde_json::to_string(&ws).expect("serialize");
            let parsed: Workspace = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(ws.id(), parsed.id());
            assert_eq!(ws.state(), parsed.state());
        }

        #[test]
        fn workspace_serde_roundtrip_merged() {
            let name = WorkspaceName::new("serde-merged").expect("valid");
            let path = WorkspacePath::new("/tmp/serde-merged").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let working = ws.start_working().expect("working");
            let ready = working.mark_ready().expect("ready");
            let merged = ready.merge().expect("merged");
            let json = serde_json::to_string(&merged).expect("serialize");
            let parsed: Workspace = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(merged.state(), parsed.state());
        }

        #[test]
        fn workspace_full_happy_path() {
            let name = WorkspaceName::new("full-path").expect("valid");
            let path = WorkspacePath::new("/tmp/full-path").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            assert_eq!(ws.state(), WorkspaceState::Created);

            let working = ws.start_working().expect("working");
            assert_eq!(working.state(), WorkspaceState::Working);
            assert!(working.is_working());
            assert!(!working.is_ready());
            assert!(!working.is_terminal());

            let ready = working.mark_ready().expect("ready");
            assert_eq!(ready.state(), WorkspaceState::Ready);
            assert!(ready.is_ready());
            assert!(!ready.is_working());

            let merged = ready.merge().expect("merged");
            assert_eq!(merged.state(), WorkspaceState::Merged);
            assert!(merged.is_terminal());
            assert!(!merged.is_ready());
            assert!(!merged.is_working());
        }

        #[test]
        fn workspace_full_abandon_from_ready() {
            let name = WorkspaceName::new("abandon-ready").expect("valid");
            let path = WorkspacePath::new("/tmp/abandon-ready").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let working = ws.start_working().expect("working");
            let ready = working.mark_ready().expect("ready");
            let abandoned = ready.abandon().expect("abandon");
            assert_eq!(abandoned.state(), WorkspaceState::Abandoned);
            assert!(abandoned.is_terminal());
        }

        #[test]
        fn workspace_id_preserved_through_transitions() {
            let name = WorkspaceName::new("id-persist").expect("valid");
            let path = WorkspacePath::new("/tmp/id-persist").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let id_before = ws.id().as_str().to_string();

            let working = ws.start_working().expect("working");
            let ready = working.mark_ready().expect("ready");
            let merged = ready.merge().expect("merged");

            assert_eq!(merged.id().as_str(), id_before);
        }

        #[test]
        fn workspace_name_and_path_preserved_through_transitions() {
            let name = WorkspaceName::new("field-persist").expect("valid");
            let path = WorkspacePath::new("/tmp/field-persist").expect("valid");
            let ws = Workspace::create(name, path).expect("created");

            let working = ws.start_working().expect("working");
            assert_eq!(working.name().as_str(), "field-persist");
            assert_eq!(working.path().as_str(), "/tmp/field-persist");
        }

        #[test]
        fn workspace_abandon_from_conflict_fails() {
            let name = WorkspaceName::new("abandon-conflict").expect("valid");
            let path = WorkspacePath::new("/tmp/abandon-conflict").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let working = ws.start_working().expect("working");
            let ready = working.mark_ready().expect("ready");
            let conflict = ready.mark_conflict().expect("conflict");
            let result = conflict.abandon();
            assert!(result.is_err());
        }

        #[test]
        fn workspace_start_working_from_merged_fails() {
            let name = WorkspaceName::new("start-merged").expect("valid");
            let path = WorkspacePath::new("/tmp/start-merged").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let working = ws.start_working().expect("working");
            let ready = working.mark_ready().expect("ready");
            let merged = ready.merge().expect("merged");
            let result = merged.start_working();
            assert!(result.is_err());
        }
    }
}
