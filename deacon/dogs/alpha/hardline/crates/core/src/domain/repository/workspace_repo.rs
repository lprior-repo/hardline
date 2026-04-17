//! WorkspaceRepository Trait
//!
//! CRITICAL: This is a TRAIT, not an implementation.
//! The domain defines the interface; infrastructure implements it.

use super::{RepositoryError, RepositoryResult};
use crate::domain::entities::workspace::Workspace;
use crate::domain::identifiers::{WorkspaceId, WorkspaceName};

pub trait WorkspaceRepository: Send + Sync {
    fn save(&self, workspace: &Workspace) -> RepositoryResult<()>;
    fn find_by_id(&self, id: &WorkspaceId) -> RepositoryResult<Option<Workspace>>;
    fn find_by_name(&self, name: &WorkspaceName) -> RepositoryResult<Option<Workspace>>;
    fn list_all(&self) -> RepositoryResult<Vec<Workspace>>;
    fn delete(&self, id: &WorkspaceId) -> RepositoryResult<()>;
}
