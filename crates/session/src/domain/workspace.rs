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

    // =========================================================================
    // WorkspaceId Generation, Uniqueness, and Type-level Tests
    // =========================================================================

    mod workspace_id_type_tests {
        use super::*;

        #[test]
        fn workspace_id_clone_preserves_value() {
            let id = WorkspaceId::new("ws-clone-test").expect("valid");
            let cloned = id.clone();
            assert_eq!(id, cloned);
            assert_eq!(id.as_str(), cloned.as_str());
        }

        #[test]
        fn workspace_id_hash_consistency() {
            use std::collections::HashSet;
            let id1 = WorkspaceId::new("ws-hash-test").expect("valid");
            let id2 = WorkspaceId::new("ws-hash-test").expect("valid");
            let mut set = HashSet::new();
            set.insert(id1);
            assert!(set.contains(&id2));
        }

        #[test]
        fn workspace_id_different_values_hash_differently() {
            use std::collections::HashSet;
            let id1 = WorkspaceId::new("ws-alpha").expect("valid");
            let id2 = WorkspaceId::new("ws-beta").expect("valid");
            let mut set = HashSet::new();
            set.insert(id1.clone());
            assert!(!set.contains(&id2));
            set.insert(id2);
            assert_eq!(set.len(), 2);
        }

        #[test]
        fn workspace_id_serde_roundtrip() {
            let id = WorkspaceId::new("ws-serde-123").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: WorkspaceId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
            assert_eq!(parsed.as_str(), "ws-serde-123");
        }

        #[test]
        fn workspace_id_serde_json_output() {
            let id = WorkspaceId::new("ws-json").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            assert_eq!(json, "\"ws-json\"");
        }

        #[test]
        fn workspace_id_uniqueness_across_workspace_creates() {
            let name = WorkspaceName::new("unique-test").expect("valid");
            let path = WorkspacePath::new("/tmp/unique").expect("valid");
            let mut ids = std::collections::HashSet::new();
            for _ in 0..50 {
                let ws = Workspace::create(name.clone(), path.clone()).expect("created");
                ids.insert(ws.id().clone());
            }
            assert_eq!(
                ids.len(),
                50,
                "each Workspace::create should produce a unique ID"
            );
        }

        #[test]
        fn workspace_id_generated_has_uuid_structure() {
            let name = WorkspaceName::new("uuid-struct").expect("valid");
            let path = WorkspacePath::new("/tmp/uuid").expect("valid");
            let ws = Workspace::create(name, path).expect("created");
            let id_str = ws.id().as_str();
            // ws-{uuid} format: "ws-" + 36-char UUID = 39 chars
            assert!(id_str.starts_with("ws-"));
            let suffix = &id_str[3..];
            assert_eq!(suffix.len(), 36, "UUID should be 36 chars with hyphens");
            assert!(suffix.contains('-'), "UUID should contain hyphens");
        }

        #[test]
        fn workspace_id_accepts_various_formats() {
            // WorkspaceId::new is flexible — accepts any non-empty string
            assert!(WorkspaceId::new("ws-001").is_ok());
            assert!(WorkspaceId::new("custom-id").is_ok());
            assert!(WorkspaceId::new("with spaces").is_ok());
            assert!(WorkspaceId::new("bd-abc123").is_ok());
        }
    }

    // =========================================================================
    // WorkspaceName Extended Type Tests
    // =========================================================================

    mod workspace_name_type_tests {
        use super::*;

        #[test]
        fn workspace_name_clone_preserves_value() {
            let name = WorkspaceName::new("clone-me").expect("valid");
            let cloned = name.clone();
            assert_eq!(name, cloned);
            assert_eq!(name.as_str(), cloned.as_str());
        }

        #[test]
        fn workspace_name_hash_consistency() {
            use std::collections::HashSet;
            let n1 = WorkspaceName::new("hash-ws").expect("valid");
            let n2 = WorkspaceName::new("hash-ws").expect("valid");
            let mut set = HashSet::new();
            set.insert(n1);
            assert!(set.contains(&n2));
        }

        #[test]
        fn workspace_name_serde_roundtrip() {
            let name = WorkspaceName::new("serde-workspace").expect("valid");
            let json = serde_json::to_string(&name).expect("serialize");
            let parsed: WorkspaceName = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(name, parsed);
            assert_eq!(parsed.as_str(), "serde-workspace");
        }

        #[test]
        fn workspace_name_serde_json_output() {
            let name = WorkspaceName::new("json-ws").expect("valid");
            let json = serde_json::to_string(&name).expect("serialize");
            assert_eq!(json, "\"json-ws\"");
        }

        #[test]
        fn workspace_name_boundary_single_char() {
            let name = WorkspaceName::new("a").expect("single char valid");
            assert_eq!(name.as_str(), "a");
        }

        #[test]
        fn workspace_name_with_special_chars_valid() {
            // WorkspaceName allows spaces, dots, etc (unlike SessionName)
            assert!(WorkspaceName::new("my workspace").is_ok());
            assert!(WorkspaceName::new("ws.name").is_ok());
            assert!(WorkspaceName::new("ws/v2").is_ok());
        }

        #[test]
        fn workspace_name_error_type_on_empty() {
            let err = WorkspaceName::new("").unwrap_err();
            assert!(
                matches!(err, SessionError::InvalidWorkspaceName(_)),
                "empty should produce InvalidWorkspaceName"
            );
        }

        #[test]
        fn workspace_name_error_type_on_too_long() {
            let too_long = "x".repeat(WorkspaceName::MAX_LENGTH + 1);
            let err = WorkspaceName::new(too_long).unwrap_err();
            assert!(
                matches!(err, SessionError::InvalidWorkspaceName(_)),
                "too long should produce InvalidWorkspaceName"
            );
        }
    }

    // =========================================================================
    // WorkspacePath Extended Validation Tests
    // =========================================================================

    mod workspace_path_type_tests {
        use super::*;

        #[test]
        fn workspace_path_parent_relative_valid() {
            let path = WorkspacePath::new("../parent/dir").expect("valid");
            assert_eq!(path.as_str(), "../parent/dir");
        }

        #[test]
        fn workspace_path_dotdot_only_valid() {
            // Starts with '.' so accepted
            let path = WorkspacePath::new("..").expect("valid");
            assert_eq!(path.as_str(), "..");
        }

        #[test]
        fn workspace_path_trailing_slash_valid() {
            let path = WorkspacePath::new("/tmp/dir/").expect("valid");
            assert_eq!(path.as_str(), "/tmp/dir/");
        }

        #[test]
        fn workspace_path_root_only_valid() {
            let path = WorkspacePath::new("/").expect("valid");
            assert_eq!(path.as_str(), "/");
        }

        #[test]
        fn workspace_path_deep_nested_valid() {
            let deep = "/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p";
            let path = WorkspacePath::new(deep).expect("valid");
            assert_eq!(path.as_str(), deep);
        }

        #[test]
        fn workspace_path_dot_only_valid() {
            let path = WorkspacePath::new(".").expect("valid");
            assert_eq!(path.as_str(), ".");
        }

        #[test]
        fn workspace_path_dot_slash_valid() {
            let path = WorkspacePath::new("./").expect("valid");
            assert_eq!(path.as_str(), "./");
        }

        #[test]
        fn workspace_path_home_tilde_rejected() {
            // '~' doesn't start with '/' or '.'
            let result = WorkspacePath::new("~/home");
            assert!(result.is_err());
        }

        #[test]
        fn workspace_path_colon_rejected() {
            // Windows-style or URL paths rejected
            let result = WorkspacePath::new("C:/Users");
            assert!(result.is_err());
        }

        #[test]
        fn workspace_path_http_rejected() {
            let result = WorkspacePath::new("http://example.com");
            assert!(result.is_err());
        }

        #[test]
        fn workspace_path_clone_preserves_value() {
            let path = WorkspacePath::new("/tmp/clone-test").expect("valid");
            let cloned = path.clone();
            assert_eq!(path, cloned);
            assert_eq!(path.as_str(), cloned.as_str());
        }

        #[test]
        fn workspace_path_hash_consistency() {
            use std::collections::HashSet;
            let p1 = WorkspacePath::new("/tmp/hash-test").expect("valid");
            let p2 = WorkspacePath::new("/tmp/hash-test").expect("valid");
            let mut set = HashSet::new();
            set.insert(p1);
            assert!(set.contains(&p2));
        }

        #[test]
        fn workspace_path_different_paths_hash_differently() {
            use std::collections::HashSet;
            let p1 = WorkspacePath::new("/tmp/alpha").expect("valid");
            let p2 = WorkspacePath::new("/tmp/beta").expect("valid");
            let mut set = HashSet::new();
            set.insert(p1.clone());
            assert!(!set.contains(&p2));
            set.insert(p2);
            assert_eq!(set.len(), 2);
        }

        #[test]
        fn workspace_path_serde_roundtrip() {
            let path = WorkspacePath::new("/tmp/serde-path").expect("valid");
            let json = serde_json::to_string(&path).expect("serialize");
            let parsed: WorkspacePath = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(path, parsed);
            assert_eq!(parsed.as_str(), "/tmp/serde-path");
        }

        #[test]
        fn workspace_path_serde_json_output() {
            let path = WorkspacePath::new("/tmp/json").expect("valid");
            let json = serde_json::to_string(&path).expect("serialize");
            assert_eq!(json, "\"/tmp/json\"");
        }

        #[test]
        fn workspace_path_error_type_on_empty() {
            let err = WorkspacePath::new("").unwrap_err();
            assert!(
                matches!(err, SessionError::InvalidWorkspacePath(_)),
                "empty should produce InvalidWorkspacePath"
            );
        }

        #[test]
        fn workspace_path_error_type_on_bare_name() {
            let err = WorkspacePath::new("no-prefix").unwrap_err();
            assert!(
                matches!(err, SessionError::InvalidWorkspacePath(_)),
                "bare name should produce InvalidWorkspacePath"
            );
        }

        #[test]
        fn workspace_path_relative_dotdot_deep() {
            let path = WorkspacePath::new("../../../deep/relative").expect("valid");
            assert_eq!(path.as_str(), "../../../deep/relative");
        }
    }

    // =========================================================================
    // Session-Workspace Integration Tests
    // =========================================================================

    mod session_workspace_integration_tests {
        use super::*;
        use crate::domain::entities::session::SessionId;
        use crate::domain::entities::session::{Active, Created, Syncing};
        use crate::domain::entities::session::{BranchState, Session, SessionData};
        use crate::domain::value_objects::{BeadId, SessionName, WorkspaceId as VoWorkspaceId};

        /// Verify Session can reference a Workspace via WorkspaceId from value_objects
        #[test]
        fn session_with_workspace_id_from_parts() {
            let name = SessionName::parse("ws-integration").expect("valid");
            let ws_id = VoWorkspaceId::parse("ws-int-test").expect("valid");
            let bead_id = BeadId::parse("bd-abc123").expect("valid");

            let session = Session::from_parts(SessionData {
                id: SessionId::parse("s-1").expect("valid"),
                name,
                workspace: Some(ws_id.clone()),
                bead: Some(bead_id),
                assigned_agent: None,
                branch: BranchState::OnBranch {
                    name: "feature".into(),
                },
                last_synced: None,
                created_at: chrono::Utc::now(),
            });

            assert!(session.workspace().is_some());
            assert_eq!(session.workspace().unwrap().as_str(), "ws-int-test");
        }

        /// Workspace ID persists through session state transitions
        #[test]
        fn workspace_id_persists_through_session_lifecycle() {
            let name = SessionName::parse("lifecycle-ws").expect("valid");
            let ws_id = VoWorkspaceId::parse("ws-lifecycle").expect("valid");
            let bead_id = BeadId::parse("bd-feed").expect("valid");

            let session = Session::from_parts(SessionData {
                id: SessionId::parse("s-lc").expect("valid"),
                name,
                workspace: Some(ws_id),
                bead: Some(bead_id),
                assigned_agent: None,
                branch: BranchState::OnBranch {
                    name: "main".into(),
                },
                last_synced: None,
                created_at: chrono::Utc::now(),
            });

            assert_eq!(
                session.workspace().map(|w| w.as_str()),
                Some("ws-lifecycle")
            );

            let active: Session<Active> = session.activate().expect("activate");
            assert_eq!(active.workspace().map(|w| w.as_str()), Some("ws-lifecycle"));

            let syncing: Session<Syncing> = active.sync().expect("sync");
            assert_eq!(
                syncing.workspace().map(|w| w.as_str()),
                Some("ws-lifecycle")
            );
        }

        /// Session without workspace (workspace is None)
        #[test]
        fn session_without_workspace() {
            let name = SessionName::parse("no-ws").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            assert!(session.workspace().is_none());
        }

        /// Domain Workspace creates IDs that match the value_objects WorkspaceId format
        #[test]
        fn workspace_create_id_format_matches_value_object_workspace_id() {
            let name = WorkspaceName::new("format-match").expect("valid");
            let path = WorkspacePath::new("/tmp/match").expect("valid");
            let ws = Workspace::create(name, path).expect("created");

            let ws_id_str = ws.id().as_str();
            assert!(ws_id_str.starts_with("ws-"));

            let vo_id = VoWorkspaceId::parse(ws_id_str)
                .expect("domain ID should be parseable by value_objects WorkspaceId");
            assert_eq!(vo_id.as_str(), ws_id_str);
        }

        /// Full session-workspace lifecycle: create workspace, assign to session,
        /// transition through states, verify workspace reference intact
        #[test]
        fn full_session_workspace_lifecycle() {
            // Create a workspace (domain level)
            let ws_name = WorkspaceName::new("lifecycle-ws").expect("valid");
            let ws_path = WorkspacePath::new("/tmp/lifecycle").expect("valid");
            let workspace = Workspace::create(ws_name, ws_path).expect("ws created");

            // Create a session referencing this workspace
            let ws_id = VoWorkspaceId::parse(workspace.id().as_str()).expect("valid");
            let session_name = SessionName::parse("full-lifecycle").expect("valid");
            let bead_id = BeadId::parse("bd-deadbeef").expect("valid");

            let session = Session::from_parts(SessionData {
                id: SessionId::parse("s-full").expect("valid"),
                name: session_name,
                workspace: Some(ws_id),
                bead: Some(bead_id),
                assigned_agent: None,
                branch: BranchState::Detached,
                last_synced: None,
                created_at: chrono::Utc::now(),
            });

            // Full lifecycle: Created → Active → Syncing → Synced → Completed
            let active = session.activate().expect("activate");
            assert!(active.workspace().is_some());

            let syncing = active.sync().expect("sync");
            assert!(syncing.workspace().is_some());

            let synced = syncing.sync_complete().expect("sync_complete");
            assert!(synced.workspace().is_some());

            let completed = synced.complete().expect("complete");
            assert!(completed.workspace().is_some());
            assert_eq!(
                completed.workspace().unwrap().as_str(),
                workspace.id().as_str()
            );
            assert!(completed.state().is_terminal());

            // Meanwhile, workspace lifecycle: Created → Working → Ready → Merged
            let ws_working = workspace.start_working().expect("working");
            let ws_ready = ws_working.mark_ready().expect("ready");
            let ws_merged = ws_ready.merge().expect("merged");
            assert!(ws_merged.is_terminal());
        }

        /// Multiple sessions can reference the same workspace ID
        #[test]
        fn multiple_sessions_same_workspace_id() {
            let ws_id = VoWorkspaceId::parse("ws-shared").expect("valid");

            let s1_name = SessionName::parse("session-a").expect("valid");
            let s1 = Session::from_parts(SessionData {
                id: SessionId::parse("s-a").expect("valid"),
                name: s1_name,
                workspace: Some(ws_id.clone()),
                bead: None,
                assigned_agent: None,
                branch: BranchState::Detached,
                last_synced: None,
                created_at: chrono::Utc::now(),
            });

            let s2_name = SessionName::parse("session-b").expect("valid");
            let s2 = Session::from_parts(SessionData {
                id: SessionId::parse("s-b").expect("valid"),
                name: s2_name,
                workspace: Some(ws_id),
                bead: None,
                assigned_agent: None,
                branch: BranchState::Detached,
                last_synced: None,
                created_at: chrono::Utc::now(),
            });

            assert_eq!(
                s1.workspace().map(|w| w.as_str()),
                s2.workspace().map(|w| w.as_str())
            );
        }
    }

    // =========================================================================
    // WorkspaceId / WorkspaceName / WorkspacePath Proptests
    // =========================================================================

    mod workspace_value_object_proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// WorkspaceId rejects empty, accepts any non-empty string
            #[test]
            fn prop_workspace_id_non_empty_accepted(s in ".+") {
                let result = WorkspaceId::new(&s);
                prop_assert!(result.is_ok());
                let id = result.unwrap();
                prop_assert_eq!(id.as_str(), s);
            }

            /// WorkspaceId always rejects empty
            #[test]
            fn prop_workspace_id_empty_rejected(_ in 0u8..1) {
                prop_assert!(WorkspaceId::new("").is_err());
            }

            /// WorkspaceId clone equals original
            #[test]
            fn prop_workspace_id_clone_equals(s in "[a-zA-Z0-9_-]{1,20}") {
                let id = WorkspaceId::new(&s).unwrap();
                let cloned = id.clone();
                prop_assert_eq!(id, cloned);
            }

            /// WorkspaceId serde roundtrip preserves value
            #[test]
            fn prop_workspace_id_serde_roundtrip(s in "[a-zA-Z0-9/_.-]{1,30}") {
                let id = WorkspaceId::new(&s).unwrap();
                let json = serde_json::to_string(&id).unwrap();
                let parsed: WorkspaceId = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(id, parsed);
            }

            /// WorkspaceName rejects empty and whitespace-only
            #[test]
            fn prop_workspace_name_empty_or_whitespace_rejected(s in "[ \t\n\r]*") {
                let trimmed = s.trim();
                let result = WorkspaceName::new(&s);
                if trimmed.is_empty() {
                    prop_assert!(result.is_err());
                }
            }

            /// WorkspaceName serde roundtrip preserves value for valid names
            #[test]
            fn prop_workspace_name_serde_roundtrip(s in "[a-zA-Z0-9 _/-]{1,100}") {
                let name = WorkspaceName::new(&s);
                if let Ok(name) = name {
                    let json = serde_json::to_string(&name).unwrap();
                    let parsed: WorkspaceName = serde_json::from_str(&json).unwrap();
                    prop_assert_eq!(name, parsed);
                }
            }

            /// WorkspaceName clone equals original for valid names
            #[test]
            fn prop_workspace_name_clone_equals(s in "[a-zA-Z][a-zA-Z0-9 _-]{0,20}") {
                if let Ok(name) = WorkspaceName::new(&s) {
                    let cloned = name.clone();
                    prop_assert_eq!(name, cloned);
                }
            }

            /// WorkspacePath accepts absolute paths
            #[test]
            fn prop_workspace_path_absolute_accepted(s in "/[a-zA-Z0-9_/._-]*") {
                if !s.is_empty() {
                    let result = WorkspacePath::new(&s);
                    prop_assert!(result.is_ok());
                }
            }

            /// WorkspacePath accepts dot-prefixed relative paths
            #[test]
            fn prop_workspace_path_relative_dot_accepted(s in "\\.[a-zA-Z0-9_/._-]*") {
                if !s.is_empty() {
                    let result = WorkspacePath::new(&s);
                    prop_assert!(result.is_ok());
                }
            }

            /// WorkspacePath rejects bare names (no / or . prefix)
            #[test]
            fn prop_workspace_path_bare_name_rejected(s in "[a-zA-Z0-9_-]+") {
                let result = WorkspacePath::new(&s);
                prop_assert!(result.is_err());
            }

            /// WorkspacePath serde roundtrip for valid paths
            #[test]
            fn prop_workspace_path_serde_roundtrip(s in "/[a-zA-Z0-9_/._-]{0,50}") {
                if let Ok(path) = WorkspacePath::new(&s) {
                    let json = serde_json::to_string(&path).unwrap();
                    let parsed: WorkspacePath = serde_json::from_str(&json).unwrap();
                    prop_assert_eq!(path, parsed);
                }
            }
        }
    }
}
