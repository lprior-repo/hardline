use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{AbsolutePath, BranchName, WorktreeId, WorktreeName, WorktreeState, WorktreeTypeEnum};

/// Aggregate root representing a Git worktree
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worktree {
    /// Unique identifier for this worktree
    id: WorktreeId,

    /// Human-readable name for this worktree
    name: WorktreeName,

    /// Absolute path to the worktree location
    path: AbsolutePath,

    /// Current state of the worktree
    state: WorktreeState,

    /// Type of worktree (development, testing, etc.)
    worktree_type: WorktreeTypeEnum,

    /// Branch associated with this worktree
    branch: Option<BranchName>,

    /// Path to the parent repository
    parent_path: AbsolutePath,

    /// Creation timestamp (Unix epoch seconds)
    created_at: i64,

    /// Last modification timestamp (Unix epoch seconds)
    updated_at: i64,

    /// Custom metadata key-value pairs
    metadata: HashMap<String, String>,
}

impl Worktree {
    /// Create a new worktree with initial validation
    pub fn new(
        name: WorktreeName,
        path: AbsolutePath,
        parent_path: AbsolutePath,
        worktree_type: WorktreeTypeEnum,
        branch: Option<BranchName>,
    ) -> Result<Self, super::WorktreeDomainError> {
        let now = chrono::Utc::now().timestamp();

        Ok(Self {
            id: WorktreeId::new_random(),
            name,
            path,
            state: WorktreeState::Creating,
            worktree_type,
            branch,
            parent_path,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        })
    }

    /// Create an uninitialized worktree (for database loading)
    #[allow(clippy::too_many_arguments)]
    pub fn uninitialized(
        id: WorktreeId,
        name: WorktreeName,
        path: AbsolutePath,
        parent_path: AbsolutePath,
        worktree_type: WorktreeTypeEnum,
        branch: Option<BranchName>,
        state: WorktreeState,
        created_at: i64,
        updated_at: i64,
    ) -> Self {
        Self::uninitialized_with_metadata(
            id,
            name,
            path,
            parent_path,
            worktree_type,
            branch,
            state,
            created_at,
            updated_at,
            HashMap::new(),
        )
    }

    /// Create an uninitialized worktree with metadata (for database loading)
    #[allow(clippy::too_many_arguments)]
    pub fn uninitialized_with_metadata(
        id: WorktreeId,
        name: WorktreeName,
        path: AbsolutePath,
        parent_path: AbsolutePath,
        worktree_type: WorktreeTypeEnum,
        branch: Option<BranchName>,
        state: WorktreeState,
        created_at: i64,
        updated_at: i64,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            id,
            name,
            path,
            state,
            worktree_type,
            branch,
            parent_path,
            created_at,
            updated_at,
            metadata,
        }
    }

    /// Initialize the worktree (transition to Active state)
    pub fn initialize(&mut self) -> Result<(), super::WorktreeDomainError> {
        if !self.state.can_transition_to(WorktreeState::Active) {
            return Err(super::WorktreeDomainError::InvalidStateTransition(
                self.state,
                WorktreeState::Active,
            ));
        }

        self.state = WorktreeState::Active;
        self.updated_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Suspend the worktree
    pub fn suspend(&mut self) -> Result<(), super::WorktreeDomainError> {
        if !self.state.can_transition_to(WorktreeState::Suspended) {
            return Err(super::WorktreeDomainError::InvalidStateTransition(
                self.state,
                WorktreeState::Suspended,
            ));
        }

        self.state = WorktreeState::Suspended;
        self.updated_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Resume a suspended worktree
    pub fn resume(&mut self) -> Result<(), super::WorktreeDomainError> {
        if !self.state.can_transition_to(WorktreeState::Active) {
            return Err(super::WorktreeDomainError::InvalidStateTransition(
                self.state,
                WorktreeState::Active,
            ));
        }

        self.state = WorktreeState::Active;
        self.updated_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Mark worktree for removal
    pub fn mark_for_removal(&mut self) -> Result<(), super::WorktreeDomainError> {
        if !self.state.can_transition_to(WorktreeState::Removing) {
            return Err(super::WorktreeDomainError::InvalidStateTransition(
                self.state,
                WorktreeState::Removing,
            ));
        }

        self.state = WorktreeState::Removing;
        self.updated_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Complete removal of worktree
    pub fn complete_removal(&mut self) -> Result<(), super::WorktreeDomainError> {
        if !self.state.can_transition_to(WorktreeState::Removed) {
            return Err(super::WorktreeDomainError::InvalidStateTransition(
                self.state,
                WorktreeState::Removed,
            ));
        }

        self.state = WorktreeState::Removed;
        self.updated_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Add metadata to the worktree
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// Remove metadata from the worktree
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        let removed = self.metadata.remove(key);
        if removed.is_some() {
            self.updated_at = chrono::Utc::now().timestamp();
        }
        removed
    }

    /// Get all metadata
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Get all metadata as HashMap
    pub fn all_metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    // Getters

    pub fn id(&self) -> &WorktreeId {
        &self.id
    }

    pub fn name(&self) -> &WorktreeName {
        &self.name
    }

    pub fn name_mut(&mut self) -> &mut WorktreeName {
        &mut self.name
    }

    pub fn path(&self) -> &AbsolutePath {
        &self.path
    }

    pub fn state(&self) -> WorktreeState {
        self.state
    }

    pub fn worktree_type(&self) -> WorktreeTypeEnum {
        self.worktree_type
    }

    pub fn branch(&self) -> Option<&BranchName> {
        self.branch.as_ref()
    }

    pub fn parent_path(&self) -> &AbsolutePath {
        &self.parent_path
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn updated_at(&self) -> i64 {
        self.updated_at
    }

    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    pub fn is_removed(&self) -> bool {
        self.state.is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_worktree() -> Worktree {
        Worktree::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test-worktree").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            Some(BranchName::new("main").unwrap()),
        )
        .unwrap()
    }

    #[test]
    fn worktree_new_returns_worktree_with_creating_state() {
        let worktree = create_test_worktree();
        assert_eq!(worktree.name().as_str(), "test-worktree");
        assert_eq!(worktree.state(), WorktreeState::Creating);
        assert!(worktree.branch().is_some());
    }

    #[test]
    fn worktree_initialize_transitions_from_creating_to_active() {
        let mut worktree = create_test_worktree();
        assert!(worktree.initialize().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Active);
    }

    #[test]
    fn worktree_suspend_transitions_from_active_to_suspended() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        assert!(worktree.suspend().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Suspended);
    }

    #[test]
    fn worktree_resume_transitions_from_suspended_to_active() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        worktree.suspend().unwrap();
        assert!(worktree.resume().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Active);
    }

    #[test]
    fn worktree_removal_flow_transitions_to_removed() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        assert!(worktree.mark_for_removal().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Removing);
        assert!(worktree.complete_removal().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Removed);
    }

    #[test]
    fn worktree_suspend_from_creating_returns_error() {
        let mut worktree = create_test_worktree();
        let result = worktree.suspend();
        assert!(result.is_err());
    }

    #[test]
    fn worktree_metadata_add_and_remove_works() {
        let mut worktree = create_test_worktree();
        worktree.add_metadata("environment", "test");
        assert_eq!(worktree.get_metadata("environment"), Some("test"));

        let removed = worktree.remove_metadata("environment");
        assert_eq!(removed, Some("test".to_string()));
        assert!(worktree.get_metadata("environment").is_none());
    }

    #[test]
    fn worktree_is_active_returns_false_when_creating() {
        let mut worktree = create_test_worktree();
        assert!(!worktree.is_active());
        worktree.initialize().unwrap();
        assert!(worktree.is_active());
    }

    #[test]
    fn worktree_is_removed_returns_true_when_removed() {
        let mut worktree = create_test_worktree();
        assert!(!worktree.is_removed());
        worktree.initialize().unwrap();
        worktree.mark_for_removal().unwrap();
        worktree.complete_removal().unwrap();
        assert!(worktree.is_removed());
    }
}
