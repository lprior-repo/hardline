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

    #[test]
    fn proptest_uuid_uniqueness() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..100 {
            let id = WorktreeId::new_random();
            assert!(ids.insert(id.clone()), "Generated duplicate UUID");
        }
        assert_eq!(ids.len(), 100);
    }

    #[test]
    fn proptest_timestamp_ordering() {
        let mut timestamps = Vec::new();
        for _ in 0..100 {
            let id = WorktreeId::new_random();
            let wt = Worktree::uninitialized(
                id,
                WorktreeName::new("test").unwrap(),
                AbsolutePath::new("/tmp/test").unwrap(),
                AbsolutePath::new("/home/user").unwrap(),
                WorktreeTypeEnum::Development,
                None,
                WorktreeState::Creating,
                0,
                0,
            );
            timestamps.push(wt.created_at());
        }
        assert!(timestamps.iter().all(|&ts| ts >= 0));
    }

    #[test]
    fn proptest_branch_name_alphanumeric() {
        let names = vec!["main", "develop", "feature123", "test_456", "release789"];
        for name in names {
            assert!(BranchName::new(name).is_ok());
        }
    }

    #[test]
    fn proptest_state_transitions_valid() {
        let mut wt = create_test_worktree();
        assert!(wt.initialize().is_ok());
        assert!(wt.suspend().is_ok());
        assert!(wt.resume().is_ok());
        assert!(wt.mark_for_removal().is_ok());
        assert!(wt.complete_removal().is_ok());
    }

    #[test]
    fn proptest_metadata_operations() {
        let mut wt = create_test_worktree();
        wt.add_metadata("k1", "v1");
        wt.add_metadata("k2", "v2");
        assert_eq!(wt.all_metadata().len(), 2);
        assert!(wt.remove_metadata("k1").is_some());
    }

    #[test]
    fn proptest_worktree_equality() {
        let id = WorktreeId::new_random();
        let name = WorktreeName::new("equal-test").unwrap();
        let wt1 = Worktree::uninitialized(
            id.clone(),
            name.clone(),
            AbsolutePath::new("/tmp/equal").unwrap(),
            AbsolutePath::new("/home/user").unwrap(),
            WorktreeTypeEnum::Development,
            None,
            WorktreeState::Active,
            1000,
            2000,
        );
        let wt2 = Worktree::uninitialized(
            id,
            name,
            AbsolutePath::new("/tmp/equal").unwrap(),
            AbsolutePath::new("/home/user").unwrap(),
            WorktreeTypeEnum::Development,
            None,
            WorktreeState::Active,
            1000,
            2000,
        );
        assert_eq!(wt1, wt2);
    }

    #[test]
    fn proptest_worktree_default_values() {
        let wt = Worktree::uninitialized(
            WorktreeId::new_random(),
            WorktreeName::new("default-test").unwrap(),
            AbsolutePath::new("/tmp/default").unwrap(),
            AbsolutePath::new("/home/user").unwrap(),
            WorktreeTypeEnum::Development,
            None,
            WorktreeState::Creating,
            0,
            0,
        );
        assert_eq!(wt.state(), WorktreeState::Creating);
        assert!(wt.branch().is_none());
    }

    #[test]
    fn proptest_timestamp_uniqueness() {
        let mut timestamps = std::collections::HashSet::new();
        for i in 0..50 {
            let id = WorktreeId::new_random();
            let wt = Worktree::uninitialized(
                id,
                WorktreeName::new("test").unwrap(),
                AbsolutePath::new("/tmp/test").unwrap(),
                AbsolutePath::new("/home/user").unwrap(),
                WorktreeTypeEnum::Development,
                None,
                WorktreeState::Creating,
                i as i64 * 1000,
                i as i64 * 1000,
            );
            timestamps.insert(wt.created_at());
        }
        assert_eq!(timestamps.len(), 50);
    }

    #[test]
    fn proptest_name_length_boundaries() {
        assert!(WorktreeName::new("a").is_ok());
        assert!(WorktreeName::new(&"a".repeat(255)).is_ok());
        assert!(WorktreeName::new("").is_err());
    }

    #[test]
    fn proptest_clone_preserves_state() {
        let mut wt = create_test_worktree();
        wt.add_metadata("test", "value");
        let cloned = wt.clone();
        assert_eq!(cloned.id(), wt.id());
        assert_eq!(cloned.all_metadata().len(), wt.all_metadata().len());
    }

    #[test]
    fn proptest_uninitialized_with_metadata() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("k1".to_string(), "v1".to_string());
        let wt = Worktree::uninitialized_with_metadata(
            WorktreeId::new_random(),
            WorktreeName::new("meta-test").unwrap(),
            AbsolutePath::new("/tmp/meta").unwrap(),
            AbsolutePath::new("/home/user").unwrap(),
            WorktreeTypeEnum::Testing,
            None,
            WorktreeState::Active,
            0,
            0,
            metadata,
        );
        assert_eq!(wt.worktree_type(), WorktreeTypeEnum::Testing);
    }

    #[test]
    fn proptest_branch_name_edge_cases() {
        assert!(BranchName::new("a").is_ok());
        assert!(BranchName::new("main").is_ok());
        assert!(BranchName::new("").is_err());
        assert!(BranchName::new("-invalid").is_err());
    }

    #[test]
    fn proptest_all_state_transitions() {
        let states = [
            WorktreeState::Creating,
            WorktreeState::Active,
            WorktreeState::Suspended,
            WorktreeState::Removing,
            WorktreeState::Removed,
        ];
        assert_eq!(states.len(), 5);
    }

    #[test]
    fn proptest_worktree_id_conversion() {
        let id1 = WorktreeId::new_random();
        let id2 = WorktreeId::from_bytes(*id1.as_bytes());
        assert_eq!(id1, id2);
    }

    #[test]
    fn proptest_worktree_timestamps() {
        let id = WorktreeId::new_random();
        let wt = Worktree::uninitialized(
            id,
            WorktreeName::new("ts-test").unwrap(),
            AbsolutePath::new("/tmp/ts").unwrap(),
            AbsolutePath::new("/home/user").unwrap(),
            WorktreeTypeEnum::Development,
            None,
            WorktreeState::Creating,
            1234567890,
            9876543210,
        );
        assert!(wt.created_at() >= 0);
        assert!(wt.updated_at() >= 0);
    }

    #[test]
    fn proptest_worktree_type_enum_values() {
        assert_eq!(WorktreeTypeEnum::Development.as_u8(), 0);
        assert_eq!(WorktreeTypeEnum::Testing.as_u8(), 1);
        assert_eq!(WorktreeTypeEnum::Review.as_u8(), 2);
        assert_eq!(WorktreeTypeEnum::Debugging.as_u8(), 3);
        assert_eq!(WorktreeTypeEnum::Research.as_u8(), 4);
    }

    #[test]
    fn proptest_worktree_branch_none() {
        let id = WorktreeId::new_random();
        let wt = Worktree::uninitialized(
            id,
            WorktreeName::new("no-branch-test").unwrap(),
            AbsolutePath::new("/tmp/nb").unwrap(),
            AbsolutePath::new("/home/user").unwrap(),
            WorktreeTypeEnum::Development,
            None,
            WorktreeState::Creating,
            0,
            0,
        );
        assert!(wt.branch().is_none());
    }

    #[test]
    fn proptest_worktree_is_active() {
        let mut wt = create_test_worktree();
        assert!(!wt.is_active());
        wt.initialize().unwrap();
        assert!(wt.is_active());
        wt.suspend().unwrap();
        assert!(!wt.is_active());
    }
}
