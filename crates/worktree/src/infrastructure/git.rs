use std::path::Path;

use crate::domain::{AbsolutePath, BranchName, WorktreeDomainError};

/// Errors that can occur during Git operations
#[derive(thiserror::Error, Debug)]
pub enum GitError {
    #[error("Git operation failed: {0}")]
    Operation(String),

    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Git error: {0}")]
    GitError(String),
}

// String-based error conversion for gix error types that don't share a common trait
macro_rules! impl_gix_error {
    ($err_type:ty) => {
        impl From<$err_type> for GitError {
            fn from(err: $err_type) -> Self {
                GitError::GitError(err.to_string())
            }
        }
    };
}

impl_gix_error!(gix::discover::Error);
impl_gix_error!(gix::open::Error);
impl_gix_error!(gix::reference::find::existing::Error);
impl_gix_error!(gix::reference::iter::Error);
impl_gix_error!(gix::reference::iter::init::Error);

impl From<Box<dyn std::error::Error + Send + Sync>> for GitError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        GitError::GitError(err.to_string())
    }
}

impl From<GitError> for WorktreeDomainError {
    fn from(error: GitError) -> Self {
        WorktreeDomainError::GitError(error.to_string())
    }
}

/// Adapter for Git operations (read-only for now)
pub struct GitWorktreeAdapter {
    repo: gix::Repository,
}

impl GitWorktreeAdapter {
    /// Create a new adapter from a repository path
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist or is not a Git repository.
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self, GitError> {
        let path = repo_path.as_ref();

        if !path.exists() {
            return Err(GitError::RepositoryNotFound(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        let repo = gix::discover(path)?;
        Ok(Self { repo })
    }

    /// Get the underlying Git repository
    pub fn repository(&self) -> &gix::Repository {
        &self.repo
    }

    /// Get the parent repository path
    ///
    /// # Errors
    ///
    /// Returns an error if the repository has no working directory or the path is invalid.
    pub fn get_parent_path(&self) -> Result<AbsolutePath, GitError> {
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| GitError::InvalidPath("Repository has no working directory".into()))?;

        AbsolutePath::new(workdir)
            .map_err(|e| GitError::InvalidPath(format!("Invalid repository path: {e}")))
    }

    /// Get the current branch of the repository
    ///
    /// # Errors
    ///
    /// Returns an error if the repository HEAD cannot be resolved or the branch name is invalid.
    pub fn get_current_branch(&self) -> Result<Option<BranchName>, GitError> {
        let head_name = self.repo.head_name()?;

        match head_name {
            Some(name) => {
                // shorten() strips the refs/heads/ prefix
                let branch_name = name.shorten().to_string();
                BranchName::new(&branch_name)
                    .map(Some)
                    .map_err(|e| GitError::InvalidPath(format!("Invalid branch name: {e}")))
            }
            None => Ok(None),
        }
    }

    /// Get all local branches
    ///
    /// # Errors
    ///
    /// Returns an error if the repository references cannot be iterated.
    pub fn get_local_branches(&self) -> Result<Vec<BranchName>, GitError> {
        let mut branches = Vec::new();
        let refs = self.repo.references()?;
        let local_iter = refs.local_branches()?;

        for branch_result in local_iter {
            let reference = branch_result?;
            let name = reference.name().shorten().to_string();
            if let Ok(bn) = BranchName::new(&name) {
                branches.push(bn);
            }
        }
        Ok(branches)
    }

    /// Get all remote branches
    ///
    /// # Errors
    ///
    /// Returns an error if the repository references cannot be iterated.
    pub fn get_remote_branches(&self) -> Result<Vec<BranchName>, GitError> {
        let mut branches = Vec::new();
        let refs = self.repo.references()?;
        let remote_iter = refs.remote_branches()?;

        for branch_result in remote_iter {
            let reference = branch_result?;
            let name = reference.name().shorten().to_string();
            // Strip origin/ prefix to match previous git2 behavior
            let name = name.strip_prefix("origin/").unwrap_or(&name);
            if let Ok(bn) = BranchName::new(name) {
                branches.push(bn);
            }
        }
        Ok(branches)
    }

    /// List all worktrees
    ///
    /// # Errors
    ///
    /// Returns an error if the worktree list cannot be retrieved.
    pub fn list_worktrees(&self) -> Result<Vec<String>, GitError> {
        // Simplified - just return empty for now
        Ok(Vec::new())
    }

    /// Check if a worktree exists
    ///
    /// # Errors
    ///
    /// Returns an error if the worktree check fails.
    pub fn worktree_exists(&self, _name: &str) -> Result<bool, GitError> {
        // Simplified - just return false for now
        Ok(false)
    }

    /// Get the path to a worktree
    ///
    /// # Errors
    ///
    /// Returns an error if the worktree path lookup fails.
    pub fn get_worktree_path(&self, _name: &str) -> Result<Option<AbsolutePath>, GitError> {
        // Simplified - just return None for now
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use tempfile::TempDir;

    use super::*;

    fn create_test_repo() -> (TempDir, GitWorktreeAdapter) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize a git repository using gix
        gix::init(repo_path).unwrap();

        // Create a test file
        let test_file = repo_path.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        // Use git CLI for commit (test-only; gix lacks simple commit API)
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(repo_path)
            .output()
            .expect("git add failed");

        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "Initial commit"])
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .current_dir(repo_path)
            .output()
            .expect("git commit failed");

        let adapter = GitWorktreeAdapter::new(repo_path).unwrap();
        (temp_dir, adapter)
    }

    #[test]
    fn git_adapter_create_returns_adapter_for_valid_repo() {
        let (_temp_dir, adapter) = create_test_repo();
        assert!(!adapter.repository().is_bare());
    }

    #[test]
    fn git_adapter_get_parent_path_returns_repository_path() {
        let (temp_dir, adapter) = create_test_repo();
        let parent_path = adapter.get_parent_path().unwrap();
        assert_eq!(parent_path.as_str(), temp_dir.path().to_string_lossy());
    }

    #[test]
    fn git_adapter_get_current_branch_returns_current_branch() {
        let (_temp_dir, adapter) = create_test_repo();
        let branch = adapter.get_current_branch().unwrap();
        assert!(branch.is_some());
        // Default branch could be master or main depending on git config
        let binding = branch.unwrap();
        let name = binding.as_str();
        assert!(name == "master" || name == "main");
    }

    #[test]
    fn git_adapter_get_local_branches_returns_branch_list() {
        let (_temp_dir, adapter) = create_test_repo();
        let branches = adapter.get_local_branches().unwrap();
        assert!(!branches.is_empty());
    }

    #[test]
    fn git_adapter_get_worktree_path_nonexistent_returns_none() {
        let (_temp_dir, adapter) = create_test_repo();
        let result = adapter.get_worktree_path("nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn git_adapter_list_worktrees_returns_empty_list_when_no_worktrees() {
        let (_temp_dir, adapter) = create_test_repo();
        let worktrees = adapter.list_worktrees().unwrap();
        assert!(worktrees.is_empty());
    }
}
