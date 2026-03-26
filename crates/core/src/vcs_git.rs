//! Git backend implementation

use crate::error::Result;
use crate::error_internal::InternalErrorKind;
use crate::error_io::IoErrorKind;
use crate::error_vcs::VcsErrorKind;
use crate::error_workspace::WorkspaceErrorKind;
use chrono::Utc;
use std::process::Command;

use super::types::{Branch, Commit, VcsStatus, Workspace};
use super::VcsBackend;

pub struct GitBackend {
    repo_path: std::path::PathBuf,
}

impl GitBackend {
    pub fn new(repo_path: std::path::PathBuf) -> Self {
        Self { repo_path }
    }

    fn run_git(&self, args: &[&str]) -> Result<std::process::Output> {
        Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| IoErrorKind::IoError(e.to_string()).into())
    }
}

impl VcsBackend for GitBackend {
    fn current_branch(&self) -> Result<String> {
        let output = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::Conflict("git".into(), "Failed to get branch".into()).into());
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
                branches.push(Branch {
                    name: name.clone(),
                    is_current: name == current,
                    tracking: None,
                });
            }
        }
        Ok(branches)
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        let output = self.run_git(&["branch", name])?;
        if !output.status.success() {
            return Err(VcsErrorKind::BranchExists(name.to_string()).into());
        }
        Ok(())
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        let output = self.run_git(&["checkout", name])?;
        if !output.status.success() {
            return Err(VcsErrorKind::BranchNotFound(name.to_string()).into());
        }
        Ok(())
    }

    fn push(&self) -> Result<()> {
        let output = self.run_git(&["push"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::PushFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn pull(&self) -> Result<()> {
        let output = self.run_git(&["pull"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::PullFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn rebase(&self, onto: &str) -> Result<()> {
        let output = self.run_git(&["rebase", onto])?;
        if !output.status.success() {
            return Err(VcsErrorKind::RebaseFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn merge(&self, branch: &str) -> Result<()> {
        let output = self.run_git(&["merge", branch])?;
        if !output.status.success() {
            return Err(VcsErrorKind::Conflict(
                branch.to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn log(&self, limit: usize) -> Result<Vec<Commit>> {
        let output = self.run_git(&["log", &format!("-n{}", limit)])?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut commits = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("commit ") {
                let id = line.trim_start_matches("commit ").to_string();
                commits.push(Commit {
                    id,
                    message: "".to_string(),
                    author: "unknown".to_string(),
                    timestamp: Utc::now(),
                    parents: vec![],
                });
            } else if line.starts_with("    ") && !commits.is_empty() {
                if let Some(last) = commits.last_mut() {
                    last.message = line.trim().to_string();
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
        Err(InternalErrorKind::Unimplemented("Git workspaces use worktrees instead".into()).into())
    }

    fn switch_workspace(&self, _name: &str) -> Result<()> {
        Err(InternalErrorKind::Unimplemented("Git workspaces use worktrees instead".into()).into())
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        Err(InternalErrorKind::Unimplemented("Git workspaces use worktrees instead".into()).into())
    }

    fn delete_workspace(&self, _name: &str) -> Result<()> {
        Err(InternalErrorKind::Unimplemented("Git workspaces use worktrees instead".into()).into())
    }

    fn fork_workspace(&self, source: &str, target: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(target);
        let output =
            self.run_git(&["worktree", "add", &worktree_path.to_string_lossy(), source])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") {
                return Err(WorkspaceErrorKind::Exists(target.to_string()).into());
            }
            return Err(
                VcsErrorKind::Conflict("worktree add".to_string(), stderr.to_string()).into(),
            );
        }
        Ok(())
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(name);
        if !worktree_path.exists() {
            return Err(WorkspaceErrorKind::NotFound(name.to_string()).into());
        }
        self.switch_branch("main")?;
        let output = self.run_git(&["merge", name])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("conflict") {
                return Err(VcsErrorKind::Conflict("merge".to_string(), stderr.to_string()).into());
            }
            return Err(VcsErrorKind::Conflict("merge".to_string(), stderr.to_string()).into());
        }
        self.push()?;
        Ok(())
    }

    fn abort_workspace(&self, _name: &str) -> Result<()> {
        // Abort workspace by restoring working copy to last commit
        // This uses git checkout -- . to discard uncommitted changes
        let output = self.run_git(&["checkout", "--", "."])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(
                VcsErrorKind::Conflict("git checkout".to_string(), stderr.to_string()).into(),
            );
        }
        Ok(())
    }
}
