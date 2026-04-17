//! SessionRepository Trait
//!
//! CRITICAL: This is a TRAIT, not an implementation.
//! The domain defines the interface; infrastructure implements it.

use super::{RepositoryError, RepositoryResult};
use crate::domain::entities::session::Session;
use crate::domain::identifiers::{SessionId, WorkspaceId};

pub trait SessionRepository: Send + Sync {
    fn save(&self, session: &Session) -> RepositoryResult<()>;
    fn find_by_id(&self, id: &SessionId) -> RepositoryResult<Option<Session>>;
    fn find_by_workspace(&self, workspace_id: &WorkspaceId) -> RepositoryResult<Vec<Session>>;
    fn delete(&self, id: &SessionId) -> RepositoryResult<()>;
}
