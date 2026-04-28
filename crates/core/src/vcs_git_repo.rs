//! GitRepo - Simple Git repository interface for TUI applications
//!
//! This provides a clean, read-focused interface to git operations
//! needed by the TUI. It wraps git CLI commands for compatibility
//! with the rest of scp-core.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::error::{Error, Result};

pub struct GitRepo {
    repo_path: PathBuf,
}

impl GitRepo {
    /// Open the repository at the current directory or any parent (cwd discovery)
    pub fn open() -> Result<Self> {
        let repo_path = Self::discover_repo(".")?;
        Ok(Self { repo_path })
    }

    /// Open the repository from a known path (no discovery needed)
    pub fn open_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.join(".git").exists() {
            return Err(Error::vcs_not_initialized());
        }
        Ok(Self {
            repo_path: path.to_path_buf(),
        })
    }

    fn discover_repo(start: &str) -> Result<PathBuf> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(start)
            .output()
            .map_err(|_e| Error::vcs_not_initialized())?;

        if !output.status.success() {
            return Err(Error::vcs_not_initialized());
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(path))
    }

    /// Get the working directory path
    pub fn workdir(&self) -> &Path {
        &self.repo_path
    }

    /// Get the .git directory path
    pub fn git_dir(&self) -> PathBuf {
        self.repo_path.join(".git")
    }

    /// Get the current branch name
    pub fn current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|_e| Error::vcs_not_initialized())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("HEAD is detached") {
                return Err(Error::vcs_not_initialized());
            }
            return Err(Error::vcs_not_initialized());
        }

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() || branch == "HEAD" {
            return Err(Error::vcs_not_initialized());
        }
        Ok(branch)
    }

    /// Check if the repository has uncommitted changes
    pub fn is_dirty(&self) -> Result<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|_e| Error::vcs_not_initialized())?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(!stdout.trim().is_empty())
    }

    /// List all local branches
    pub fn list_branches(&self) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(["branch", "--format=%(refname:short)"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|_e| Error::vcs_not_initialized())?;

        if !output.status.success() {
            return Err(Error::vcs_not_initialized());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let branches: Vec<String> = stdout
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(branches)
    }

    /// Get the tip commit hash of a branch
    pub fn branch_commit(&self, branch_name: &str) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", &format!("refs/heads/{}", branch_name)])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|_e| Error::vcs_not_initialized())?;

        if !output.status.success() {
            return Err(Error::vcs_not_initialized());
        }

        let oid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(oid)
    }

    /// Get the repository path used to open this repo
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Run an arbitrary git command and return the output
    pub fn run_git(&self, args: &[&str]) -> Result<std::process::Output> {
        Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|_e| Error::vcs_not_initialized())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_repo_open_from_temp() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        let result = GitRepo::open_from_path(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_git_repo_open_nonexistent() {
        let result = GitRepo::open_from_path("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_git_repo_creation() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        let repo = GitRepo::open_from_path(dir.path()).expect("should open");
        assert_eq!(repo.repo_path(), dir.path());
    }
}
