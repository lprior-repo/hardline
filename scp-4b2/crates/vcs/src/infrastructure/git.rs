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

// Pure calculation functions

fn parse_branch_line(line: &str, current_branch: &str) -> Option<Branch> {
    let name = line.trim().trim_start_matches("* ");
    if name.is_empty() {
        None
    } else {
        Some(Branch::new(name.to_string(), name == current_branch, None))
    }
}

fn parse_git_log_entry(line: &str) -> Option<Commit> {
    let trimmed = line.trim();
    trimmed.starts_with("commit ").then(|| {
        Commit::new(
            trimmed.trim_start_matches("commit ").to_string(),
            String::new(),
            "unknown".to_string(),
            Utc::now(),
            vec![],
        )
    })
}

fn parse_log_message(line: &str) -> Option<&str> {
    line.starts_with("    ").then(|| line.trim())
}

fn set_commit_message(commit: Commit, message: &str) -> Commit {
    Commit {
        message: message.to_string(),
        ..commit
    }
}

fn attach_messages_to_commits(commits: Vec<Commit>, messages: Vec<&str>) -> Vec<Commit> {
    let commit_count = commits.len();
    commits
        .into_iter()
        .enumerate()
        .map(|(idx, commit)| {
            messages
                .get(idx)
                .map(|msg| set_commit_message(commit, msg))
                .unwrap_or(commit)
        })
        .collect()
}

fn parse_git_log_output(stdout: &str) -> Vec<Commit> {
    let commits: Vec<Commit> = stdout.lines().filter_map(parse_git_log_entry).collect();
    let messages: Vec<&str> = stdout.lines().filter_map(parse_log_message).collect();
    attach_messages_to_commits(commits, messages)
}

fn classify_vcs_status(stdout: &str) -> VcsStatus {
    if stdout.is_empty() {
        VcsStatus::Clean
    } else if stdout.contains("UU") {
        VcsStatus::Conflicted
    } else {
        VcsStatus::Dirty
    }
}

fn handle_worktree_error(stderr: &str, target: &str) -> VcsError {
    if stderr.contains("already exists") {
        VcsError::WorkspaceExists(target.to_string())
    } else {
        VcsError::Conflict("worktree add".to_string(), stderr.to_string())
    }
}

impl VcsBackend for GitBackend {
    fn current_branch(&self) -> Result<String> {
        self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .and_then(|output| {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    Err(VcsError::Conflict(
                        "git".into(),
                        "Failed to get branch".into(),
                    ))
                }
            })
    }

    fn list_branches(&self) -> Result<Vec<Branch>> {
        self.run_git(&["branch", "-a"]).and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let current = self.current_branch()?;
            Ok(stdout
                .lines()
                .filter_map(|line| parse_branch_line(line, &current))
                .collect())
        })
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        self.run_git(&["branch", name]).and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(VcsError::BranchExists(name.to_string()))
            }
        })
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        self.run_git(&["checkout", name]).and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(VcsError::BranchNotFound(name.to_string()))
            }
        })
    }

    fn push(&self) -> Result<()> {
        self.run_git(&["push"]).and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(VcsError::PushFailed(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        })
    }

    fn pull(&self) -> Result<()> {
        self.run_git(&["pull"]).and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(VcsError::PullFailed(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        })
    }

    fn rebase(&self, onto: &str) -> Result<()> {
        self.run_git(&["rebase", onto]).and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(VcsError::RebaseFailed(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        })
    }

    fn merge(&self, branch: &str) -> Result<()> {
        self.run_git(&["merge", branch]).and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(VcsError::Conflict(
                    branch.to_string(),
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        })
    }

    fn log(&self, limit: usize) -> Result<Vec<Commit>> {
        self.run_git(&["log", &format!("-n{}", limit)])
            .map(|output| parse_git_log_output(&String::from_utf8_lossy(&output.stdout)))
    }

    fn status(&self) -> Result<VcsStatus> {
        self.run_git(&["status", "--porcelain"])
            .map(|output| classify_vcs_status(&String::from_utf8_lossy(&output.stdout)))
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
        self.run_git(&["worktree", "add", &worktree_path.to_string_lossy(), source])
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(handle_worktree_error(&stderr, target))
                }
            })
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(name);
        if !worktree_path.exists() {
            return Err(VcsError::WorkspaceNotFound(name.to_string()));
        }
        self.switch_branch("main")?;
        self.run_git(&["merge", name]).and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(VcsError::Conflict("merge".to_string(), stderr.to_string()))
            }
        })?;
        self.push()
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
