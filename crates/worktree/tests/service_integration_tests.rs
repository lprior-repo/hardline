//! Integration tests for WorktreeService
//! 
//! Tests the service layer with real repository implementations

use worktree::application::{
    services::WorktreeService,
    commands::{
        CreateWorktreeCommand, InitializeWorktreeCommand, SuspendWorktreeCommand,
        ResumeWorktreeCommand, RemoveWorktreeCommand, ListWorktreesQuery,
    },
    repositories::WorktreeRepository,
};
use worktree::domain::{
    Worktree, WorktreeId, WorktreeName, WorktreeTypeEnum,
    AbsolutePath, BranchName, WorktreeDomainError, WorktreeState,
};

// In-memory repository for testing
#[derive(Default, Clone)]
struct TestRepository {
    worktrees: Vec<Worktree>,
}

#[async_trait::async_trait]
impl WorktreeRepository for TestRepository {
    async fn save(&mut self, worktree: &mut Worktree) -> Result<(), WorktreeDomainError> {
        if let Some(existing) = self.worktrees.iter_mut().find(|w| w.id() == worktree.id()) {
            *existing = worktree.clone();
        } else {
            self.worktrees.push(worktree.clone());
        }
        Ok(())
    }

    async fn find_by_id(&self, id: &WorktreeId) -> Result<Option<Worktree>, WorktreeDomainError> {
        Ok(self.worktrees.iter().find(|w| w.id() == id).cloned())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError> {
        Ok(self.worktrees.iter().find(|w| w.name().as_str() == name).cloned())
    }

    async fn list_all(&self) -> Result<Vec<Worktree>, WorktreeDomainError> {
        Ok(self.worktrees.clone())
    }

    async fn delete(&mut self, id: &WorktreeId) -> Result<(), WorktreeDomainError> {
        self.worktrees.retain(|w| w.id() != id);
        Ok(())
    }

    async fn name_exists(&self, name: &str) -> Result<bool, WorktreeDomainError> {
        Ok(self.worktrees.iter().any(|w| w.name().as_str() == name))
    }
}

// Helper to create commands
fn create_test_command(
    name: &str,
    path: &str,
    parent: &str,
    worktree_type: WorktreeTypeEnum,
    branch: Option<&str>,
) -> CreateWorktreeCommand {
    CreateWorktreeCommand::new(
        WorktreeName::new(name).unwrap(),
        AbsolutePath::new(path).unwrap(),
        AbsolutePath::new(parent).unwrap(),
        worktree_type,
        branch.map(|b| BranchName::new(b).unwrap()),
    )
}

mod worktree_service_integration_tests {
    use super::*;

    // ============================================================
    // CREATE WORKTREE TESTS
    // ============================================================

    #[tokio::test]
    async fn worktree_service_create_worktree_saves_to_repository() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("test-wt", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, Some("main"));
        let result = service.create_worktree(cmd).await;

        assert!(result.is_ok());
        let worktree = result.unwrap();
        assert_eq!(worktree.name().as_str(), "test-wt");
        assert_eq!(worktree.state(), WorktreeState::Creating);
    }

    #[tokio::test]
    async fn worktree_service_create_worktree_with_none_branch() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("test-wt", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        let result = service.create_worktree(cmd).await;

        assert!(result.is_ok());
        let worktree = result.unwrap();
        assert!(worktree.branch().is_none());
    }

    #[tokio::test]
    async fn worktree_service_create_worktree_duplicate_name_returns_name_already_exists_error() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd1 = create_test_command("duplicate-wt", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let cmd2 = create_test_command("duplicate-wt", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);

        assert!(service.create_worktree(cmd1).await.is_ok());
        let result = service.create_worktree(cmd2).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            WorktreeDomainError::NameAlreadyExists("duplicate-wt".to_string())
        );
    }

    #[tokio::test]
    async fn worktree_service_create_worktree_caches_worktree() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("cache-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Review, None);
        let result = service.create_worktree(cmd).await;

        assert!(result.is_ok());
        let worktree = result.unwrap();
        let id = worktree.id().clone();

        // Should be in cache
        let cached = service.find_by_id(&id);
        assert!(cached.is_ok());
        assert!(cached.unwrap().is_some());
    }

    // ============================================================
    // INITIALIZE WORKTREE TESTS
    // ============================================================

    #[tokio::test]
    async fn worktree_service_initialize_worktree_transitions_to_active() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        // Create a worktree
        let cmd = create_test_command("init-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        // Initialize it
        let result = service.initialize_worktree(InitializeWorktreeCommand::new(id.clone())).await;

        assert!(result.is_ok());
        let initialized = result.unwrap();
        assert_eq!(initialized.state(), WorktreeState::Active);
    }

    #[tokio::test]
    async fn worktree_service_initialize_worktree_not_found_returns_not_found_error() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let nonexistent_id = WorktreeId::new_random();
        let result = service.initialize_worktree(InitializeWorktreeCommand::new(nonexistent_id)).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorktreeDomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn worktree_service_initialize_worktree_updates_cache() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("init-cache-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        service.initialize_worktree(InitializeWorktreeCommand::new(id.clone())).await.unwrap();

        // Cache should be updated
        let cached = service.find_by_id(&id).unwrap().unwrap();
        assert_eq!(cached.state(), WorktreeState::Active);
    }

    // ============================================================
    // SUSPEND WORKTREE TESTS
    // ============================================================

    #[tokio::test]
    async fn worktree_service_suspend_worktree_transitions_to_suspended() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("suspend-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        service.initialize_worktree(InitializeWorktreeCommand::new(id.clone())).await.unwrap();

        let result = service.suspend_worktree(SuspendWorktreeCommand::new(id.clone())).await;

        assert!(result.is_ok());
        let suspended = result.unwrap();
        assert_eq!(suspended.state(), WorktreeState::Suspended);
    }

    #[tokio::test]
    async fn worktree_service_suspend_worktree_not_found_returns_not_found_error() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let nonexistent_id = WorktreeId::new_random();
        let result = service.suspend_worktree(SuspendWorktreeCommand::new(nonexistent_id)).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorktreeDomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn worktree_service_suspend_worktree_updates_repository() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("suspend-repo-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Review, None);
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        service.initialize_worktree(InitializeWorktreeCommand::new(id.clone())).await.unwrap();
        service.suspend_worktree(SuspendWorktreeCommand::new(id.clone())).await.unwrap();

        // Service's list_worktrees should see updated state
        let query = ListWorktreesQuery::new();
        let results = service.list_worktrees(query).unwrap();
        let suspended = results.iter().find(|w| w.id() == &id).unwrap();
        assert_eq!(suspended.state(), WorktreeState::Suspended);
    }

    // ============================================================
    // RESUME WORKTREE TESTS
    // ============================================================

    #[tokio::test]
    async fn worktree_service_resume_worktree_transitions_to_active() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("resume-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Debugging, None);
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        service.initialize_worktree(InitializeWorktreeCommand::new(id.clone())).await.unwrap();
        service.suspend_worktree(SuspendWorktreeCommand::new(id.clone())).await.unwrap();

        let result = service.resume_worktree(ResumeWorktreeCommand::new(id.clone())).await;

        assert!(result.is_ok());
        let resumed = result.unwrap();
        assert_eq!(resumed.state(), WorktreeState::Active);
    }

    #[tokio::test]
    async fn worktree_service_resume_worktree_not_found_returns_not_found_error() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let nonexistent_id = WorktreeId::new_random();
        let result = service.resume_worktree(ResumeWorktreeCommand::new(nonexistent_id)).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorktreeDomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn worktree_service_resume_worktree_from_non_suspended_fails() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("resume-fail-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Research, None);
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        // Initialize first, then try to resume without suspending
        service.initialize_worktree(InitializeWorktreeCommand::new(id.clone())).await.unwrap();
        
        // Try to resume without suspending first - should fail
        let result = service.resume_worktree(ResumeWorktreeCommand::new(id)).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorktreeDomainError::InvalidStateTransition(_, _)));
    }

    // ============================================================
    // REMOVE WORKTREE TESTS
    // ============================================================

    #[tokio::test]
    async fn worktree_service_remove_worktree_deletes_from_repository() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("remove-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        service.initialize_worktree(InitializeWorktreeCommand::new(id.clone())).await.unwrap();
        service.remove_worktree(RemoveWorktreeCommand::new(id.clone())).await.unwrap();

        // Should be deleted from service's list
        let query = ListWorktreesQuery::new();
        let results = service.list_worktrees(query).unwrap();
        let found = results.iter().any(|w| w.id() == &id);
        assert!(!found);
    }

    #[tokio::test]
    async fn worktree_service_remove_worktree_not_found_returns_not_found_error() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let nonexistent_id = WorktreeId::new_random();
        let result = service.remove_worktree(RemoveWorktreeCommand::new(nonexistent_id)).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorktreeDomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn worktree_service_remove_worktree_clears_cache() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("remove-cache-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        service.initialize_worktree(InitializeWorktreeCommand::new(id.clone())).await.unwrap();
        service.remove_worktree(RemoveWorktreeCommand::new(id.clone())).await.unwrap();

        // Should be removed from cache
        let cached = service.find_by_id(&id);
        assert!(cached.is_ok());
        assert!(cached.unwrap().is_none());
    }

    // ============================================================
    // LIST WORKTREES TESTS
    // ============================================================

    #[tokio::test]
    async fn worktree_service_list_worktrees_returns_all_worktrees() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        service.create_worktree(create_test_command("wt-1", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None)).await.unwrap();
        service.create_worktree(create_test_command("wt-2", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None)).await.unwrap();
        service.create_worktree(create_test_command("wt-3", "/tmp/wt3", "/home/user/proj", WorktreeTypeEnum::Review, None)).await.unwrap();

        let query = ListWorktreesQuery::new();
        let results = service.list_worktrees(query).unwrap();

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn worktree_service_list_worktrees_filtered_by_state_returns_matching_worktrees() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd1 = create_test_command("active-1", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let worktree1 = service.create_worktree(cmd1).await.unwrap();
        service.initialize_worktree(InitializeWorktreeCommand::new(worktree1.id().clone())).await.unwrap();

        let cmd2 = create_test_command("active-2", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        let worktree2 = service.create_worktree(cmd2).await.unwrap();
        service.initialize_worktree(InitializeWorktreeCommand::new(worktree2.id().clone())).await.unwrap();

        let cmd3 = create_test_command("creating-3", "/tmp/wt3", "/home/user/proj", WorktreeTypeEnum::Review, None);
        service.create_worktree(cmd3).await.unwrap();

        let query = ListWorktreesQuery::new().with_state(WorktreeState::Active);
        let results = service.list_worktrees(query).unwrap();

        assert_eq!(results.len(), 2);
        for wt in results.iter() {
            assert_eq!(wt.state(), WorktreeState::Active);
        }
    }

    #[tokio::test]
    async fn worktree_service_list_worktrees_filtered_by_type_returns_matching_worktrees() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        service.create_worktree(create_test_command("dev-1", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None)).await.unwrap();
        service.create_worktree(create_test_command("test-1", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None)).await.unwrap();
        service.create_worktree(create_test_command("dev-2", "/tmp/wt3", "/home/user/proj", WorktreeTypeEnum::Development, None)).await.unwrap();

        let query = ListWorktreesQuery::new().with_worktree_type(WorktreeTypeEnum::Testing);
        let results = service.list_worktrees(query).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].worktree_type(), WorktreeTypeEnum::Testing);
    }

    #[tokio::test]
    async fn worktree_service_list_worktrees_filtered_by_name_prefix_returns_matching_worktrees() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        service.create_worktree(create_test_command("feature-a", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None)).await.unwrap();
        service.create_worktree(create_test_command("feature-b", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None)).await.unwrap();
        service.create_worktree(create_test_command("bugfix-c", "/tmp/wt3", "/home/user/proj", WorktreeTypeEnum::Review, None)).await.unwrap();

        let query = ListWorktreesQuery::new().with_name_prefix("feature-");
        let results = service.list_worktrees(query).unwrap();

        assert_eq!(results.len(), 2);
        for wt in results.iter() {
            assert!(wt.name().as_str().starts_with("feature-"));
        }
    }

    #[tokio::test]
    async fn worktree_service_list_worktrees_with_include_removed_returns_removed_worktrees() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd1 = create_test_command("active-1", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let worktree1 = service.create_worktree(cmd1).await.unwrap();
        let id1 = worktree1.id().clone();
        service.initialize_worktree(InitializeWorktreeCommand::new(id1.clone())).await.unwrap();
        service.remove_worktree(RemoveWorktreeCommand::new(id1.clone())).await.unwrap();

        let cmd2 = create_test_command("active-2", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        let worktree2 = service.create_worktree(cmd2).await.unwrap();
        let _id2 = worktree2.id().clone();

        // Without include_removed
        let query = ListWorktreesQuery::new();
        let results = service.list_worktrees(query).unwrap();
        assert_eq!(results.len(), 1);

        // With include_removed
        let query = ListWorktreesQuery::new().with_include_removed(true);
        let results = service.list_worktrees(query).unwrap();
        assert_eq!(results.len(), 1);
    }

    // ============================================================
    // ADD METADATA TESTS
    // ============================================================

    #[tokio::test]
    async fn worktree_service_add_metadata_updates_worktree() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("metadata-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        service.add_metadata(&id, "environment", "test").await.unwrap();

        // Check service has metadata via list_worktrees
        let query = ListWorktreesQuery::new();
        let results = service.list_worktrees(query).unwrap();
        let updated = results.iter().find(|w| w.id() == &id).unwrap();
        assert_eq!(updated.get_metadata("environment"), Some("test"));
    }

    #[tokio::test]
    async fn worktree_service_add_metadata_not_found_returns_not_found_error() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let nonexistent_id = WorktreeId::new_random();
        let result = service.add_metadata(&nonexistent_id, "key", "value").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorktreeDomainError::NotFound(_)));
    }

    // ============================================================
    // ERROR HANDLING TESTS
    // ============================================================

    #[tokio::test]
    async fn worktree_service_all_operations_use_domain_errors() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        // Test NameAlreadyExists error
        let cmd1 = create_test_command("error-test", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let cmd2 = create_test_command("error-test", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);

        service.create_worktree(cmd1).await.unwrap();
        let result = service.create_worktree(cmd2).await;

        assert!(matches!(result.unwrap_err(), WorktreeDomainError::NameAlreadyExists(_)));

        // Test NotFound error
        let nonexistent_id = WorktreeId::new_random();
        let result = service.initialize_worktree(InitializeWorktreeCommand::new(nonexistent_id)).await;
        assert!(matches!(result.unwrap_err(), WorktreeDomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn worktree_service_suspension_state_transition_errors() {
        let repo = TestRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = create_test_command("state-error-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        // Try to suspend a creating worktree - should return InvalidStateTransition
        let result = service.suspend_worktree(SuspendWorktreeCommand::new(id.clone())).await;
        assert!(matches!(result.unwrap_err(), WorktreeDomainError::InvalidStateTransition(_, _)));

        // Initialize first
        service.initialize_worktree(InitializeWorktreeCommand::new(id.clone())).await.unwrap();

        // Now suspend should work
        service.suspend_worktree(SuspendWorktreeCommand::new(id.clone())).await.unwrap();

        // Try to suspend again - should return InvalidStateTransition (already suspended)
        let result = service.suspend_worktree(SuspendWorktreeCommand::new(id)).await;
        assert!(matches!(result.unwrap_err(), WorktreeDomainError::InvalidStateTransition(_, _)));
    }
}
