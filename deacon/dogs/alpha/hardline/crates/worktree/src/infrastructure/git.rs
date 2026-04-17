use crate::domain::{AbsolutePath, BranchName, WorktreeDomainError};
use git2::Repository as GitRepository;
use std::path::Path;

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
    GitError(#[from] git2::Error),
}

impl From<GitError> for WorktreeDomainError {
    fn from(error: GitError) -> Self {
        WorktreeDomainError::GitError(error.to_string())
    }
}

/// Adapter for Git operations (read-only for now)
pub struct GitWorktreeAdapter {
    repo: GitRepository,
}

impl GitWorktreeAdapter {
    /// Create a new adapter from a repository path
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self, GitError> {
        let path = repo_path.as_ref();

        if !path.exists() {
            return Err(GitError::RepositoryNotFound(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        let repo = GitRepository::open(path)?;
        Ok(Self { repo })
    }

    /// Get the underlying Git repository
    pub fn repository(&self) -> &GitRepository {
        &self.repo
    }

    /// Get the parent repository path
    pub fn get_parent_path(&self) -> Result<AbsolutePath, GitError> {
        let path = self.repo.path();
        let worktree_dir = path.parent().unwrap_or(path);

        AbsolutePath::new(worktree_dir)
            .map_err(|e| GitError::InvalidPath(format!("Invalid repository path: {}", e)))
    }

    /// Get the current branch of the repository
    pub fn get_current_branch(&self) -> Result<Option<BranchName>, GitError> {
        let head = self.repo.head()?;

        if head.is_branch() {
            let branch_name = head.name();
            match branch_name {
                Some(name) => {
                    // Strip refs/heads/ prefix if present
                    let name = name.strip_prefix("refs/heads/").unwrap_or(name);
                    BranchName::new(name)
                        .map(Some)
                        .map_err(|e| GitError::InvalidPath(format!("Invalid branch name: {}", e)))
                }
                None => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// Get all local branches
    pub fn get_local_branches(&self) -> Result<Vec<BranchName>, GitError> {
        let mut branches = Vec::new();
        for (branch, _) in self.repo.branches(None)?.flatten() {
            if let Ok(Some(name)) = branch.name() {
                if let Ok(bn) = BranchName::new(name) {
                    branches.push(bn);
                }
            }
        }
        Ok(branches)
    }

    /// Get all remote branches
    pub fn get_remote_branches(&self) -> Result<Vec<BranchName>, GitError> {
        let mut branches = Vec::new();
        for (branch, _) in self
            .repo
            .branches(Some(git2::BranchType::Remote))?
            .flatten()
        {
            if let Ok(Some(name)) = branch.name() {
                let name = name.strip_prefix("origin/").unwrap_or(name);
                if let Ok(bn) = BranchName::new(name) {
                    branches.push(bn);
                }
            }
        }
        Ok(branches)
    }

    /// List all worktrees (git2 0.20+)
    pub fn list_worktrees(&self) -> Result<Vec<String>, GitError> {
        // Simplified - just return empty for now
        Ok(Vec::new())
    }

    /// Check if a worktree exists
    pub fn worktree_exists(&self, _name: &str) -> Result<bool, GitError> {
        // Simplified - just return false for now
        Ok(false)
    }

    /// Get the path to a worktree
    pub fn get_worktree_path(&self, _name: &str) -> Result<Option<AbsolutePath>, GitError> {
        // Simplified - just return None for now
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, GitWorktreeAdapter) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize a git repository
        GitRepository::init(repo_path).unwrap();

        // Create a test file and commit
        let test_file = repo_path.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let repo = GitRepository::init(repo_path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();

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
        assert_eq!(branch.unwrap().as_str(), "master");
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
