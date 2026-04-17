//! Repository trait for worktree persistence

use crate::domain::{Worktree, WorktreeDomainError, WorktreeId};
use async_trait::async_trait;

#[async_trait]
pub trait WorktreeRepository: Send + Sync {
    async fn save<S: Send>(&mut self, worktree: Worktree<S>) -> Result<(), WorktreeDomainError>;

    async fn find_by_id(&self, id: &WorktreeId) -> Result<Option<Worktree>, WorktreeDomainError>;

    async fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError>;

    async fn list_all(&self) -> Result<Vec<Worktree>, WorktreeDomainError>;

    async fn delete(&mut self, id: &WorktreeId) -> Result<(), WorktreeDomainError>;

    async fn name_exists(&self, name: &str) -> Result<bool, WorktreeDomainError>;
}
