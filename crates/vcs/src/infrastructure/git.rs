//! Git VCS Backend Implementation

use crate::domain::entities::{Branch, Commit, Workspace};
use crate::domain::traits::VcsBackend;
use crate::domain::value_objects::VcsStatus;
use crate::error::{Result, VcsError};
use chrono::Utc;
use std::path::PathBuf;

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

    fn run_git(&self, args: &[&str]) -> Result<std::process::Output> {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(VcsError::Io)
    }
}

impl VcsBackend for GitBackend {
    fn current_branch(&self) -> Result<String> {
        let output = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        if !output.status.success() {
            return Err(VcsError::Conflict(
                "git".into(),
                "Failed to get branch".into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn list_branches(&self) -> Result<Vec<Branch>> {
        let output = self.run_git(&["branch", "-a"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let current = self.current_branch()?;
        let mut branches = Vec::new();

        for line in stdout.lines() {
            let name = line.trim().trim_start_matches("* ").to_string();
            if !name.is_empty() {
                branches.push(Branch::new(name.clone(), name == current, None));
            }
        }
        Ok(branches)
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        let output = self.run_git(&["branch", name])?;
        if !output.status.success() {
            return Err(VcsError::BranchExists(name.to_string()));
        }
        Ok(())
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        let output = self.run_git(&["checkout", name])?;
        if !output.status.success() {
            return Err(VcsError::BranchNotFound(name.to_string()));
        }
        Ok(())
    }

    fn push(&self) -> Result<()> {
        let output = self.run_git(&["push"])?;
        if !output.status.success() {
            return Err(VcsError::PushFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn pull(&self) -> Result<()> {
        let output = self.run_git(&["pull"])?;
        if !output.status.success() {
            return Err(VcsError::PullFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn rebase(&self, onto: &str) -> Result<()> {
        let output = self.run_git(&["rebase", onto])?;
        if !output.status.success() {
            return Err(VcsError::RebaseFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn merge(&self, branch: &str) -> Result<()> {
        let output = self.run_git(&["merge", branch])?;
        if !output.status.success() {
            return Err(VcsError::Conflict(
                branch.to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn log(&self, limit: usize) -> Result<Vec<Commit>> {
        let output = self.run_git(&["log", &format!("-n{}", limit)])?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut commits = Vec::new();
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("commit ") {
                let id = trimmed.trim_start_matches("commit ").to_string();
                commits.push(Commit::new(
                    id,
                    "".to_string(),
                    "unknown".to_string(),
                    Utc::now(),
                    vec![],
                ));
            } else if line.starts_with("    ") && !commits.is_empty() {
                if let Some(last) = commits.last_mut() {
                    last.message = trimmed.to_string();
                }
            }
        }
        Ok(commits)
    }

    fn status(&self) -> Result<VcsStatus> {
        let output = self.run_git(&["status", "--porcelain"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.is_empty() {
            Ok(VcsStatus::Clean)
        } else if stdout.contains("UU") {
            Ok(VcsStatus::Conflicted)
        } else {
            Ok(VcsStatus::Dirty)
        }
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
        Err(VcsError::Unimplemented(
            "Git workspaces use worktrees instead".into(),
        ))
    }

    fn delete_workspace(&self, _name: &str) -> Result<()> {
        Err(VcsError::Unimplemented(
            "Git workspaces use worktrees instead".into(),
        ))
    }

    fn fork_workspace(&self, source: &str, target: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(target);
        let output =
            self.run_git(&["worktree", "add", &worktree_path.to_string_lossy(), source])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") {
                return Err(VcsError::WorkspaceExists(target.to_string()));
            }
            return Err(VcsError::Conflict(
                "worktree add".to_string(),
                stderr.to_string(),
            ));
        }
        Ok(())
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(name);
        if !worktree_path.exists() {
            return Err(VcsError::WorkspaceNotFound(name.to_string()));
        }
        self.switch_branch("main")?;
        let output = self.run_git(&["merge", name])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("conflict") {
                return Err(VcsError::Conflict("merge".to_string(), stderr.to_string()));
            }
            return Err(VcsError::Conflict("merge".to_string(), stderr.to_string()));
        }
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
}
