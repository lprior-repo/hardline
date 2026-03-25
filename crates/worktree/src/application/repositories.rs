//! Repository trait for worktree persistence

use crate::domain::{Worktree, WorktreeDomainError, WorktreeId};
use async_trait::async_trait;

/// Repository trait for worktree persistence
#[async_trait]
pub trait WorktreeRepository: Send + Sync {
    /// Save a worktree to the repository
    async fn save(&mut self, worktree: &mut Worktree) -> Result<(), WorktreeDomainError>;

    /// Find a worktree by ID
    async fn find_by_id(&self, id: &WorktreeId) -> Result<Option<Worktree>, WorktreeDomainError>;

    /// Find a worktree by name
    async fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError>;

    /// List all worktrees
    async fn list_all(&self) -> Result<Vec<Worktree>, WorktreeDomainError>;

    /// Delete a worktree
    async fn delete(&mut self, id: &WorktreeId) -> Result<(), WorktreeDomainError>;

    /// Check if a worktree with given name exists
    async fn name_exists(&self, name: &str) -> Result<bool, WorktreeDomainError>;
}
