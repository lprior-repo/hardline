use crate::application::{
    commands::{
        CreateWorktreeCommand, InitializeWorktreeCommand, SuspendWorktreeCommand,
        ResumeWorktreeCommand, RemoveWorktreeCommand, ListWorktreesQuery,
    },
    repositories::WorktreeRepository,
};
use crate::domain::{Worktree, WorktreeId, WorktreeDomainError};
use std::collections::HashMap;

/// Service for managing worktrees
/// 
/// This is the application layer service that orchestrates worktree operations
/// using the repository pattern.
pub struct WorktreeService<R: WorktreeRepository> {
    repository: R,
    /// In-memory cache of worktrees (for demonstration, in production would be more sophisticated)
    cache: HashMap<WorktreeId, Worktree>,
}

impl<R: WorktreeRepository> WorktreeService<R> {
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            cache: HashMap::new(),
        }
    }

    /// Create a new worktree
    pub async fn create_worktree(
        &mut self,
        cmd: CreateWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        // Validate no duplicate names
        if self.repository.name_exists(cmd.name.as_str()).await? {
            return Err(WorktreeDomainError::NameAlreadyExists(cmd.name.as_str().to_string()));
        }

        // Create the worktree
        let mut worktree = Worktree::new(
            cmd.name,
            cmd.path,
            cmd.parent_path,
            cmd.worktree_type,
            cmd.branch,
        )?;

        // Persist
        self.repository.save(&mut worktree).await?;

        // Cache
        let id = worktree.id().clone();
        self.cache.insert(id.clone(), worktree.clone());

        Ok(worktree)
    }

    /// Initialize an existing worktree
    pub async fn initialize_worktree(
        &mut self,
        cmd: InitializeWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        let mut worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        worktree.initialize()?;

        self.repository.save(&mut worktree).await?;

        let id = worktree.id().clone();
        self.cache.insert(id.clone(), worktree.clone());

        Ok(worktree)
    }

    /// Suspend a worktree
    pub async fn suspend_worktree(
        &mut self,
        cmd: SuspendWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        let mut worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        worktree.suspend()?;

        self.repository.save(&mut worktree).await?;

        let id = worktree.id().clone();
        self.cache.insert(id.clone(), worktree.clone());

        Ok(worktree)
    }

    /// Resume a suspended worktree
    pub async fn resume_worktree(
        &mut self,
        cmd: ResumeWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        let mut worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        worktree.resume()?;

        self.repository.save(&mut worktree).await?;

        let id = worktree.id().clone();
        self.cache.insert(id.clone(), worktree.clone());

        Ok(worktree)
    }

    /// Remove a worktree
    pub async fn remove_worktree(
        &mut self,
        cmd: RemoveWorktreeCommand,
    ) -> Result<(), WorktreeDomainError> {
        let mut worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        worktree.mark_for_removal()?;
        self.repository.save(&mut worktree).await?;

        worktree.complete_removal()?;
        self.repository.save(&mut worktree).await?;

        self.repository.delete(&cmd.worktree_id).await?;

        self.cache.remove(&cmd.worktree_id);

        Ok(())
    }

    /// Find a worktree by ID (cache only)
    pub fn find_by_id(&self, id: &WorktreeId) -> Result<Option<Worktree>, WorktreeDomainError> {
        // Try cache first
        Ok(self.cache.get(id).cloned())
    }

    /// Find a worktree by name (cache only)
    pub fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError> {
        // In production, this would query the repository
        Ok(self.cache.values().find(|w| w.name().as_str() == name).cloned())
    }

    /// List all worktrees with optional filters (cache only)
    pub fn list_worktrees(&self, query: ListWorktreesQuery) -> Result<Vec<Worktree>, WorktreeDomainError> {
        let worktrees: Vec<Worktree> = self.cache.values().cloned().collect();

        let result: Vec<Worktree> = worktrees
            .into_iter()
            .filter(|w| query.include_removed || !w.is_removed())
            .filter(|w| {
                query
                    .state_filter
                    .is_none_or(|state| w.state() == state)
            })
            .filter(|w| {
                query
                    .worktree_type_filter
                    .is_none_or(|wt| w.worktree_type() == wt)
            })
            .filter(|w| {
                query
                    .name_prefix
                    .as_ref()
                    .is_none_or(|prefix| w.name().as_str().starts_with(prefix))
            })
            .collect();

        Ok(result)
    }

    /// Add metadata to a worktree
    pub async fn add_metadata(
        &mut self,
        worktree_id: &WorktreeId,
        key: &str,
        value: &str,
    ) -> Result<(), WorktreeDomainError> {
        let mut worktree = self
            .repository
            .find_by_id(worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(worktree_id.clone()))?;

        worktree.add_metadata(key, value);
        self.repository.save(&mut worktree).await?;

        let id = worktree.id().clone();
        self.cache.insert(id.clone(), worktree.clone());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AbsolutePath, BranchName, WorktreeTypeEnum, WorktreeName};

    #[derive(Default)]
    struct InMemoryRepository {
        worktrees: Vec<Worktree>,
    }

    #[async_trait::async_trait]
    impl WorktreeRepository for InMemoryRepository {
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

    #[tokio::test]
    async fn worktree_service_create_saves_worktree_to_repository() {
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = CreateWorktreeCommand::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            Some(BranchName::new("main").unwrap()),
        );

        let result = service.create_worktree(cmd).await;
        assert!(result.is_ok());
        let worktree = result.unwrap();
        assert_eq!(worktree.name().as_str(), "test-worktree");
    }

    #[tokio::test]
    async fn worktree_service_create_duplicate_name_returns_error() {
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd1 = CreateWorktreeCommand::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test1").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );

        let cmd2 = CreateWorktreeCommand::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test2").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );

        assert!(service.create_worktree(cmd1).await.is_ok());
        assert!(service.create_worktree(cmd2).await.is_err());
    }

    #[tokio::test]
    async fn worktree_service_initialize_transitions_worktree_to_active() {
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = CreateWorktreeCommand::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        let init_cmd = InitializeWorktreeCommand::new(id);
        let result = service.initialize_worktree(init_cmd).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_active());
    }

    #[tokio::test]
    async fn worktree_service_suspend_and_resume_worktree() {
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = CreateWorktreeCommand::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        service
            .initialize_worktree(InitializeWorktreeCommand::new(id.clone()))
            .await
            .unwrap();

        let suspend_cmd = SuspendWorktreeCommand::new(id.clone());
        let result = service.suspend_worktree(suspend_cmd).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_active());

        let resume_cmd = ResumeWorktreeCommand::new(id);
        let result = service.resume_worktree(resume_cmd).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_active());
    }

    #[tokio::test]
    async fn worktree_service_remove_deletes_worktree_from_repository() {
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = CreateWorktreeCommand::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        service
            .initialize_worktree(InitializeWorktreeCommand::new(id.clone()))
            .await
            .unwrap();
        service
            .remove_worktree(RemoveWorktreeCommand::new(id.clone()))
            .await
            .unwrap();

        let result = service.find_by_id(&id);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

#[tokio::test]
    async fn worktree_service_list_returns_all_worktrees_with_filters() {
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd1 = CreateWorktreeCommand::new(
            WorktreeName::new("worktree-1").unwrap(),
            AbsolutePath::new("/tmp/test1").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );
        let cmd2 = CreateWorktreeCommand::new(
            WorktreeName::new("worktree-2").unwrap(),
            AbsolutePath::new("/tmp/test2").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Testing,
            None,
        );

        service.create_worktree(cmd1).await.unwrap();
        service.create_worktree(cmd2).await.unwrap();

        let query = ListWorktreesQuery::new();
        let results = service.list_worktrees(query).unwrap();
        assert_eq!(results.len(), 2);

        let query = ListWorktreesQuery::new().with_worktree_type(WorktreeTypeEnum::Testing);
        let results = service.list_worktrees(query).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name().as_str(), "worktree-2");
    }
}
