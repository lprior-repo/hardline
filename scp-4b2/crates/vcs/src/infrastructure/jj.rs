//! JJ (Jujutsu) VCS Backend Implementation

use crate::domain::entities::{Branch, Commit, Workspace};
use crate::domain::traits::VcsBackend;
use crate::domain::value_objects::VcsStatus;
use crate::error::{Result, VcsError};
use chrono::Utc;
use std::path::PathBuf;

pub struct JjBackend {
    repo_path: PathBuf,
}

impl JjBackend {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    pub fn new_from_path(path: impl Into<PathBuf>) -> Self {
        Self::new(path.into())
    }

    fn run_jj(&self, args: &[&str]) -> Result<std::process::Output> {
        std::process::Command::new("jj")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(VcsError::Io)
    }
}

impl VcsBackend for JjBackend {
    fn current_branch(&self) -> Result<String> {
        let output = self.run_jj(&["log", "-r", "@", "-T", " bookmarks()"])?;
        if !output.status.success() {
            return Err(VcsError::Conflict(
                "jj".into(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn list_branches(&self) -> Result<Vec<Branch>> {
        let output = self.run_jj(&["bookmark", "list"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut branches = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('!') {
                let name = line.split(':').next().unwrap_or(line).trim();
                let name = name.trim_start_matches('*').trim();
                branches.push(Branch::new(name.to_string(), line.starts_with('*'), None));
            }
        }
        Ok(branches)
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["bookmark", "create", name])?;
        if !output.status.success() {
            return Err(VcsError::BranchExists(name.to_string()));
        }
        Ok(())
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["bookmark", "set", name])?;
        if !output.status.success() {
            return Err(VcsError::BranchNotFound(name.to_string()));
        }
        Ok(())
    }

    fn push(&self) -> Result<()> {
        let output = self.run_jj(&["git", "push"])?;
        if !output.status.success() {
            return Err(VcsError::PushFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn pull(&self) -> Result<()> {
        let output = self.run_jj(&["git", "fetch"])?;
        if !output.status.success() {
            return Err(VcsError::PullFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn rebase(&self, onto: &str) -> Result<()> {
        let output = self.run_jj(&["rebase", "-d", onto])?;
        if !output.status.success() {
            return Err(VcsError::RebaseFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn merge(&self, branch: &str) -> Result<()> {
        let output = self.run_jj(&["merge", branch])?;
        if !output.status.success() {
            return Err(VcsError::Conflict(
                branch.to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn log(&self, limit: usize) -> Result<Vec<Commit>> {
        let output = self.run_jj(&["log", "-n", &limit.to_string()])?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut commits = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if !line.is_empty() {
                commits.push(Commit::new(
                    line.to_string(),
                    line.to_string(),
                    "unknown".to_string(),
                    Utc::now(),
                    vec![],
                ));
            }
        }
        Ok(commits)
    }

    fn status(&self) -> Result<VcsStatus> {
        let output = self.run_jj(&["status"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.contains("There are conflicts") {
            return Ok(VcsStatus::Conflicted);
        }

        let has_changes = stdout.lines().any(|l| {
            let trimmed = l.trim();
            trimmed.starts_with("Modified:")
                || trimmed.starts_with("Added:")
                || trimmed.starts_with("Removed:")
        });

        Ok(if has_changes {
            VcsStatus::Dirty
        } else {
            VcsStatus::Clean
        })
    }

    fn is_initialized(&self) -> Result<bool> {
        Ok(self.repo_path.join(".jj").exists())
    }

    fn create_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "add", name])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") || stderr.contains("exists") {
                return Err(VcsError::WorkspaceExists(name.to_string()));
            }
            return Err(VcsError::Conflict(
                "workspace add".to_string(),
                stderr.to_string(),
            ));
        }
        Ok(())
    }

    fn switch_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "root", "--name", name])?;
        if !output.status.success() {
            return Err(VcsError::WorkspaceNotFound(name.to_string()));
        }
        Ok(())
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let output = self.run_jj(&["workspace", "list"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut workspaces = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if !line.is_empty() {
                let name = line.split(':').next().unwrap_or(line).trim();
                let is_current = line.starts_with('*');
                let name = name.trim_start_matches('*').trim();
                workspaces.push(Workspace::new(
                    name.to_string(),
                    name.to_string(),
                    is_current,
                ));
            }
        }
        Ok(workspaces)
    }

    fn delete_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "delete", name])?;
        if !output.status.success() {
            return Err(VcsError::WorkspaceNotFound(name.to_string()));
        }
        Ok(())
    }

    fn fork_workspace(&self, source: &str, target: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "add", target, "-b", source])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") || stderr.contains("exists") {
                return Err(VcsError::WorkspaceExists(target.to_string()));
            }
            return Err(VcsError::Conflict(
                "workspace fork".to_string(),
                stderr.to_string(),
            ));
        }
        Ok(())
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "root", "--name", name])?;
        if !output.status.success() {
            return Err(VcsError::WorkspaceNotFound(name.to_string()));
        }
        self.rebase("main")?;
        self.push()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jj_backend_creation() {
        let backend = JjBackend::new_from_path("/tmp/test");
        assert_eq!(backend.repo_path, std::path::PathBuf::from("/tmp/test"));
    }
}
