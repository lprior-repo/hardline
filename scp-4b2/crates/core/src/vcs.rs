//! Unified VCS abstraction layer for Source Control Plane.
//!
//! Provides trait-based VCS backend supporting both JJ and Git.
//! Zero panic, zero unwrap - all operations return Result.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A VCS commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub parents: Vec<String>,
}

/// A VCS branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub is_current: bool,
    pub tracking: Option<String>,
}

/// A workspace (from Isolate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub branch: String,
    pub is_current: bool,
}

/// VCS backend trait - unified interface for JJ and Git
pub trait VcsBackend: Send + Sync {
    /// Get current branch name
    fn current_branch(&self) -> Result<String>;

    /// List all branches
    fn list_branches(&self) -> Result<Vec<Branch>>;

    /// Create a new branch
    fn create_branch(&self, name: &str) -> Result<()>;

    /// Switch to a branch
    fn switch_branch(&self, name: &str) -> Result<()>;

    /// Push changes to remote
    fn push(&self) -> Result<()>;

    /// Pull changes from remote
    fn pull(&self) -> Result<()>;

    /// Rebase current branch onto another
    fn rebase(&self, onto: &str) -> Result<()>;

    /// Merge a branch into current
    fn merge(&self, branch: &str) -> Result<()>;

    /// Get commit log
    fn log(&self, limit: usize) -> Result<Vec<Commit>>;

    /// Get status of working copy
    fn status(&self) -> Result<VcsStatus>;

    /// Check if VCS is initialized
    fn is_initialized(&self) -> Result<bool>;

    // ========================================================================
    // Workspace operations (from Isolate)
    // ========================================================================

    /// Create a new workspace
    fn create_workspace(&self, name: &str) -> Result<()>;

    /// Switch to a workspace
    fn switch_workspace(&self, name: &str) -> Result<()>;

    /// List workspaces
    fn list_workspaces(&self) -> Result<Vec<Workspace>>;

    /// Delete a workspace
    fn delete_workspace(&self, name: &str) -> Result<()>;

    /// Fork a workspace from another workspace
    fn fork_workspace(&self, source: &str, target: &str) -> Result<()>;

    /// Merge a workspace into main
    fn merge_workspace(&self, name: &str) -> Result<()>;
}

/// Status of working copy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VcsStatus {
    /// Clean - no uncommitted changes
    Clean,
    /// Has uncommitted changes
    Dirty,
    /// Has conflicts
    Conflicted,
    /// Detached HEAD
    Detached,
}

impl std::fmt::Display for VcsStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clean => write!(f, "clean"),
            Self::Dirty => write!(f, "dirty"),
            Self::Conflicted => write!(f, "conflicted"),
            Self::Detached => write!(f, "detached"),
        }
    }
}

/// VCS type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsType {
    /// Jujutsu VCS
    Jujutsu,
    /// Git VCS
    Git,
}

/// Detect which VCS is in use in a directory
pub fn detect_vcs(path: &std::path::Path) -> Option<VcsType> {
    if path.join(".jj").exists() {
        Some(VcsType::Jujutsu)
    } else if path.join(".git").exists() {
        Some(VcsType::Git)
    } else {
        None
    }
}

/// JJ (Jujutsu) backend implementation
pub struct JjBackend {
    repo_path: std::path::PathBuf,
}

impl JjBackend {
    pub fn new(repo_path: std::path::PathBuf) -> Self {
        Self { repo_path }
    }

    fn run_jj(&self, args: &[&str]) -> Result<std::process::Output> {
        std::process::Command::new("jj")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| Error::Io(e))
    }
}

impl VcsBackend for JjBackend {
    fn current_branch(&self) -> Result<String> {
        let output = self.run_jj(&["log", "-r", "@", "-T", " bookmarks()"])?;
        if !output.status.success() {
            return Err(Error::VcsConflict(
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
            if !line.is_empty() && !line.starts_with("!") {
                let name = line.split(':').next().unwrap_or(line).trim();
                let name = name.trim_start_matches('*').trim();
                branches.push(Branch {
                    name: name.to_string(),
                    is_current: line.starts_with('*'),
                    tracking: None,
                });
            }
        }
        Ok(branches)
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["bookmark", "create", name])?;
        if !output.status.success() {
            return Err(Error::BranchExists(name.to_string()));
        }
        Ok(())
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["bookmark", "set", name])?;
        if !output.status.success() {
            return Err(Error::BranchNotFound(name.to_string()));
        }
        Ok(())
    }

    fn push(&self) -> Result<()> {
        let output = self.run_jj(&["git", "push"])?;
        if !output.status.success() {
            return Err(Error::VcsPushFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn pull(&self) -> Result<()> {
        let output = self.run_jj(&["git", "fetch"])?;
        if !output.status.success() {
            return Err(Error::VcsPullFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn rebase(&self, onto: &str) -> Result<()> {
        let output = self.run_jj(&["rebase", "-d", onto])?;
        if !output.status.success() {
            return Err(Error::VcsRebaseFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn merge(&self, branch: &str) -> Result<()> {
        let output = self.run_jj(&["merge", branch])?;
        if !output.status.success() {
            return Err(Error::VcsConflict(
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
                commits.push(Commit {
                    id: line.to_string(),
                    message: line.to_string(),
                    author: "unknown".to_string(),
                    timestamp: Utc::now(),
                    parents: vec![],
                });
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
                return Err(Error::WorkspaceExists(name.to_string()));
            }
            return Err(Error::VcsConflict(
                "workspace add".to_string(),
                stderr.to_string(),
            ));
        }
        Ok(())
    }

    fn switch_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "root", "--name", name])?;
        if !output.status.success() {
            return Err(Error::WorkspaceNotFound(name.to_string()));
        }
        let workspace_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("Workspace '{}' is at: {}", name, workspace_root);
        println!("To switch, run: cd {}", workspace_root);
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
                workspaces.push(Workspace {
                    name: name.to_string(),
                    branch: name.to_string(),
                    is_current,
                });
            }
        }
        Ok(workspaces)
    }

    fn delete_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "delete", name])?;
        if !output.status.success() {
            return Err(Error::WorkspaceNotFound(name.to_string()));
        }
        Ok(())
    }

    fn fork_workspace(&self, source: &str, target: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "add", target, "-b", source])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") || stderr.contains("exists") {
                return Err(Error::WorkspaceExists(target.to_string()));
            }
            return Err(Error::VcsConflict(
                "workspace fork".to_string(),
                stderr.to_string(),
            ));
        }
        Ok(())
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "root", "--name", name])?;
        if !output.status.success() {
            return Err(Error::WorkspaceNotFound(name.to_string()));
        }
        self.rebase("main")?;
        self.push()?;
        Ok(())
    }
}

/// Git backend implementation
pub struct GitBackend {
    repo_path: std::path::PathBuf,
}

impl GitBackend {
    pub fn new(repo_path: std::path::PathBuf) -> Self {
        Self { repo_path }
    }

    fn run_git(&self, args: &[&str]) -> Result<std::process::Output> {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| Error::Io(e))
    }
}

impl VcsBackend for GitBackend {
    fn current_branch(&self) -> Result<String> {
        let output = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        if !output.status.success() {
            return Err(Error::VcsConflict(
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
            return Err(Error::BranchExists(name.to_string()));
        }
        Ok(())
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        let output = self.run_git(&["checkout", name])?;
        if !output.status.success() {
            return Err(Error::BranchNotFound(name.to_string()));
        }
        Ok(())
    }

    fn push(&self) -> Result<()> {
        let output = self.run_git(&["push"])?;
        if !output.status.success() {
            return Err(Error::VcsPushFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn pull(&self) -> Result<()> {
        let output = self.run_git(&["pull"])?;
        if !output.status.success() {
            return Err(Error::VcsPullFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn rebase(&self, onto: &str) -> Result<()> {
        let output = self.run_git(&["rebase", onto])?;
        if !output.status.success() {
            return Err(Error::VcsRebaseFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(())
    }

    fn merge(&self, branch: &str) -> Result<()> {
        let output = self.run_git(&["merge", branch])?;
        if !output.status.success() {
            return Err(Error::VcsConflict(
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
        // Git doesn't have native workspace support
        // Could use git worktree
        Err(Error::Unimplemented(
            "Git workspaces use worktrees instead".into(),
        ))
    }

    fn switch_workspace(&self, _name: &str) -> Result<()> {
        Err(Error::Unimplemented(
            "Git workspaces use worktrees instead".into(),
        ))
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        Err(Error::Unimplemented(
            "Git workspaces use worktrees instead".into(),
        ))
    }

    fn delete_workspace(&self, _name: &str) -> Result<()> {
        Err(Error::Unimplemented(
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
                return Err(Error::WorkspaceExists(target.to_string()));
            }
            return Err(Error::VcsConflict(
                "worktree add".to_string(),
                stderr.to_string(),
            ));
        }
        Ok(())
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(name);
        if !worktree_path.exists() {
            return Err(Error::WorkspaceNotFound(name.to_string()));
        }
        self.switch_branch("main")?;
        let output = self.run_git(&["merge", name])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("conflict") {
                return Err(Error::VcsConflict("merge".to_string(), stderr.to_string()));
            }
            return Err(Error::VcsConflict("merge".to_string(), stderr.to_string()));
        }
        self.push()?;
        Ok(())
    }
}

/// Auto-detect and create appropriate VCS backend
pub fn create_backend(path: &std::path::Path) -> Result<Box<dyn VcsBackend>> {
    match detect_vcs(path) {
        Some(VcsType::Jujutsu) => Ok(Box::new(JjBackend::new(path.to_path_buf()))),
        Some(VcsType::Git) => Ok(Box::new(GitBackend::new(path.to_path_buf()))),
        None => Err(Error::VcsNotInitialized),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_detect_vcs() {
        // Test in a known directory
        let cwd = env::current_dir().unwrap();
        let vcs = detect_vcs(&cwd);
        // May or may not be initialized depending on where we run
        println!("Detected VCS: {:?}", vcs);
    }
}
