//! Git VCS Backend Implementation

use std::path::PathBuf;

use crate::{
    domain::{
        entities::{Branch, Commit, Workspace},
        traits::VcsBackend,
        value_objects::VcsStatus,
    },
    error::{Result, VcsError},
    gix,
};

pub struct GitBackend {
    repo_path: PathBuf,
}

impl GitBackend {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    pub fn new_from_path(path: impl Into<PathBuf>) -> Self {
        Self::new(path.into())
    }

    fn repo(&self) -> Result<::gix::Repository> {
        gix::repository::open(&self.repo_path).map_err(VcsError::from)
    }
}

impl VcsBackend for GitBackend {
    fn current_branch(&self) -> Result<String> {
        let repo = self.repo()?;
        gix::branch::current(&repo).map_err(VcsError::from)
    }

    fn list_branches(&self) -> Result<Vec<Branch>> {
        let repo = self.repo()?;
        gix::branch::list(&repo, false).map_err(VcsError::from)
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        let repo = self.repo()?;
        gix::branch::create(&repo, name, false).map_err(VcsError::from)
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        let repo = self.repo()?;
        gix::branch::switch(&repo, name, false).map_err(VcsError::from)
    }

    fn push(&self) -> Result<()> {
        let repo = self.repo()?;
        gix::remote::push(&repo, "origin", None, false, false, false).map_err(VcsError::from)
    }

    fn pull(&self) -> Result<()> {
        let repo = self.repo()?;
        gix::remote::pull(&repo, None, false).map_err(VcsError::from)?;
        Ok(())
    }

    fn rebase(&self, _onto: &str) -> Result<()> {
        Err(VcsError::Unimplemented(
            "rebase not yet implemented with gix".into(),
        ))
    }

    fn merge(&self, _branch: &str) -> Result<()> {
        Err(VcsError::Unimplemented(
            "merge not yet implemented with gix".into(),
        ))
    }

    fn log(&self, limit: usize) -> Result<Vec<Commit>> {
        let repo = self.repo()?;
        gix::commit::log(&repo, limit).map_err(VcsError::from)
    }

    fn status(&self) -> Result<VcsStatus> {
        let repo = self.repo()?;
        gix::status::status(&repo).map_err(VcsError::from)
    }

    fn is_initialized(&self) -> Result<bool> {
        Ok(self.repo_path.join(".git").exists())
    }

    fn create_workspace(&self, _name: &str) -> Result<()> {
        Err(VcsError::Unimplemented(
            "Git workspaces use worktrees instead".into(),
        ))
    }

    fn switch_workspace(&self, _name: &str) -> Result<()> {
        Err(VcsError::Unimplemented(
            "Git workspaces use worktrees instead".into(),
        ))
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let repo = self.repo()?;
        let worktrees = gix::worktree::list(&repo).map_err(VcsError::from)?;
        Ok(worktrees
            .into_iter()
            .map(|w| {
                Workspace::new(
                    w.path.to_string_lossy().to_string(),
                    w.branch.unwrap_or_default(),
                    w.is_main,
                )
            })
            .collect())
    }

    fn delete_workspace(&self, _name: &str) -> Result<()> {
        Err(VcsError::Unimplemented(
            "Git workspaces use worktrees instead".into(),
        ))
    }

    fn fork_workspace(&self, source: &str, target: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(target);
        let repo = self.repo()?;
        gix::worktree::add(&repo, &worktree_path, Some(source)).map_err(VcsError::from)
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(name);
        if !worktree_path.exists() {
            return Err(VcsError::WorkspaceNotFound(name.to_string()));
        }
        self.switch_branch("main")?;
        self.merge(name)?;
        self.push()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_backend_creation() {
        let backend = GitBackend::new_from_path("/tmp/test");
        assert_eq!(backend.repo_path, std::path::PathBuf::from("/tmp/test"));
    }

    #[test]
    fn git_backend_new() {
        let path = std::path::PathBuf::from("/tmp/git-test");
        let backend = GitBackend::new(path.clone());
        assert_eq!(backend.repo_path, path);
    }

    #[test]
    fn git_backend_is_initialized_true() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        let backend = GitBackend::new(dir.path().to_path_buf());
        assert!(backend.is_initialized().expect("ok"));
    }

    #[test]
    fn git_backend_is_initialized_false() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let backend = GitBackend::new(dir.path().to_path_buf());
        assert!(!backend.is_initialized().expect("ok"));
    }

    #[test]
    fn git_backend_is_initialized_no_path() {
        let backend = GitBackend::new(std::path::PathBuf::from("/nonexistent"));
        assert!(!backend.is_initialized().expect("ok"));
    }
}
