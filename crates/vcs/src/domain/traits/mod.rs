//! VCS Backend Trait - Domain contract for VCS operations

use crate::domain::entities::{Branch, Commit, Workspace};
use crate::domain::value_objects::VcsStatus;
use crate::error::Result;

pub trait VcsBackend: Send + Sync {
    fn current_branch(&self) -> Result<String>;

    fn list_branches(&self) -> Result<Vec<Branch>>;

    fn create_branch(&self, name: &str) -> Result<()>;

    fn switch_branch(&self, name: &str) -> Result<()>;

    fn push(&self) -> Result<()>;

    fn pull(&self) -> Result<()>;

    fn rebase(&self, onto: &str) -> Result<()>;

    fn merge(&self, branch: &str) -> Result<()>;

    fn log(&self, limit: usize) -> Result<Vec<Commit>>;

    fn status(&self) -> Result<VcsStatus>;

    fn is_initialized(&self) -> Result<bool>;

    fn create_workspace(&self, name: &str) -> Result<()>;

    fn switch_workspace(&self, name: &str) -> Result<()>;

    fn list_workspaces(&self) -> Result<Vec<Workspace>>;

    fn delete_workspace(&self, name: &str) -> Result<()>;

    fn fork_workspace(&self, source: &str, target: &str) -> Result<()>;

    fn merge_workspace(&self, name: &str) -> Result<()>;
}
