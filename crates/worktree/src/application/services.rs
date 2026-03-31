#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::application::{
    commands::{
        CreateWorktreeCommand, InitializeWorktreeCommand, ListWorktreesQuery,
        RemoveWorktreeCommand, ResumeWorktreeCommand, SuspendWorktreeCommand,
    },
    repositories::WorktreeRepository,
};
use crate::domain::{Worktree, WorktreeState};
use crate::domain::{WorktreeDomainError, WorktreeId};

pub struct WorktreeService<R: WorktreeRepository> {
    repository: R,
}

impl<R: WorktreeRepository> WorktreeService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create_worktree(
        &mut self,
        cmd: CreateWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        if self.repository.name_exists(cmd.name.as_str()).await? {
            return Err(WorktreeDomainError::NameAlreadyExists(
                cmd.name.as_str().to_string(),
            ));
        }

        let worktree = Worktree::new(
            cmd.name,
            cmd.path,
            cmd.parent_path,
            cmd.worktree_type,
            cmd.branch,
        );

        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;
        Ok(worktree_clone)
    }

    pub async fn initialize_worktree(
        &mut self,
        cmd: InitializeWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        let worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        let worktree = worktree.activate();

        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;
        Ok(worktree_clone.into())
    }

    pub async fn suspend_worktree(
        &mut self,
        cmd: SuspendWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        let worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        // Transition through active state first, then suspend
        let worktree = worktree.activate();
        let worktree = worktree.suspend();

        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;
        Ok(worktree_clone.into())
    }

    pub async fn resume_worktree(
        &mut self,
        cmd: ResumeWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        let worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        // Transition to active first, then suspend then resume
        let worktree = worktree.activate();
        let worktree = worktree.suspend();
        let worktree = worktree.resume();

        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;
        Ok(worktree_clone.into())
    }

    pub async fn remove_worktree(
        &mut self,
        cmd: RemoveWorktreeCommand,
    ) -> Result<(), WorktreeDomainError> {
        let worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        // Transition to active first, then mark for removal
        let worktree = worktree.activate();
        let worktree = worktree.mark_for_removal();
        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;

        let worktree = worktree_clone;
        let worktree = worktree.complete_removal();
        self.repository.save(worktree).await?;

        self.repository.delete(&cmd.worktree_id).await
    }

    pub async fn find_by_id(
        &self,
        id: &WorktreeId,
    ) -> Result<Option<Worktree>, WorktreeDomainError> {
        self.repository.find_by_id(id).await
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError> {
        self.repository.find_by_name(name).await
    }

    pub async fn list_worktrees(
        &self,
        query: ListWorktreesQuery,
    ) -> Result<Vec<Worktree>, WorktreeDomainError> {
        let worktrees = self.repository.list_all().await?;

        let result: Vec<Worktree> = worktrees
            .into_iter()
            .filter(|w| query.include_removed || !w.is_removed())
            .filter(|w| query.state_filter.is_none_or(|state| w.state() == state))
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

    pub async fn add_metadata(
        &mut self,
        worktree_id: &WorktreeId,
        key: &str,
        value: &str,
    ) -> Result<Worktree, WorktreeDomainError> {
        let mut worktree = self
            .repository
            .find_by_id(worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(worktree_id.clone()))?;

        worktree.add_metadata(key, value);
        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;
        Ok(worktree_clone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repositories::WorktreeRepository;
    use crate::domain::{AbsolutePath, BranchName, WorktreeName, WorktreeTypeEnum};

    #[derive(Default)]
    struct InMemoryRepository {
        worktrees: Vec<Worktree>,
    }

    #[async_trait::async_trait]
    impl WorktreeRepository for InMemoryRepository {
        async fn save<S: Send>(
            &mut self,
            worktree: Worktree<S>,
        ) -> Result<(), WorktreeDomainError> {
            if let Some(existing) = self.worktrees.iter_mut().find(|w| w.id() == worktree.id()) {
                *existing = worktree.into_state();
            } else {
                self.worktrees.push(worktree.into_state());
            }
            Ok(())
        }

        async fn find_by_id(
            &self,
            id: &WorktreeId,
        ) -> Result<Option<Worktree>, WorktreeDomainError> {
            Ok(self.worktrees.iter().find(|w| w.id() == id).cloned())
        }

        async fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError> {
            Ok(self
                .worktrees
                .iter()
                .find(|w| w.name().as_str() == name)
                .cloned())
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
        assert_eq!(result.unwrap().state(), WorktreeState::Active);
    }
}
