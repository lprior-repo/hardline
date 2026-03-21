//! Git CLI Backend - VCS Backend Trait Implementation

use crate::domain::entities::{Branch, Commit, Workspace};
use crate::domain::traits::VcsBackend;
use crate::domain::value_objects::VcsStatus;
use crate::error::{Result, VcsError};
use crate::infrastructure::git_cli::core::GitCliBackend;

impl VcsBackend for GitCliBackend {
    fn current_branch(&self) -> Result<String> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        let output = self.run_git_command(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        if output.is_empty() || output == "HEAD" {
            Ok(String::new())
        } else {
            Ok(output)
        }
    }

    fn list_branches(&self) -> Result<Vec<Branch>> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        let output = self.run_git_command(&["branch"])?;
        let branches: Vec<Branch> = output
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }
                let is_current = trimmed.starts_with('*');
                let name = trimmed.trim_start_matches("* ").trim().to_string();
                if name.is_empty() {
                    None
                } else {
                    Some(Branch::new(name, is_current, None))
                }
            })
            .collect();
        Ok(branches)
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        self.run_git_command(&["branch", name])?;
        Ok(())
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        self.run_git_command(&["checkout", name])?;
        Ok(())
    }

    fn push(&self) -> Result<()> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        self.run_git_command(&["push"])?;
        Ok(())
    }

    fn pull(&self) -> Result<()> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        self.run_git_command(&["pull"])?;
        Ok(())
    }

    fn rebase(&self, _onto: &str) -> Result<()> {
        Err(VcsError::Unimplemented("rebase not yet implemented".into()))
    }

    fn merge(&self, _branch: &str) -> Result<()> {
        Err(VcsError::Unimplemented("merge not yet implemented".into()))
    }

    fn log(&self, limit: usize) -> Result<Vec<Commit>> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        let format = "%H%x00%ct%x00%an%x00%ae%x00%P%x00%B%n---COMMIT---%n";
        let output = self.run_git_command(&[
            "log",
            &format!("-{}", limit),
            &format!("--format={}", format),
        ])?;

        if output.is_empty() {
            return Ok(Vec::new());
        }

        let commits: Vec<Commit> = output
            .split("---COMMIT---")
            .filter_map(|s| {
                let parts: Vec<&str> = s.trim().split('\x00').collect();
                if parts.len() >= 6 {
                    let id = parts[0].to_string();
                    let timestamp: i64 = parts[1].parse().ok()?;
                    let author = parts[2].to_string();
                    let email = parts[3].to_string();
                    let _parent_count: usize = parts[4].parse().ok().unwrap_or(1);
                    let message = parts[5].to_string();
                    let author_full = if email.is_empty() {
                        author.clone()
                    } else {
                        format!("{} <{}>", author, email)
                    };
                    Some(Commit::new(
                        id,
                        message,
                        author_full,
                        GitCliBackend::parse_timestamp(timestamp),
                        Vec::new(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        Ok(commits)
    }

    fn status(&self) -> Result<VcsStatus> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        let output = self.run_git_command(&["status", "--porcelain"])?;
        if output.is_empty() {
            Ok(VcsStatus::Clean)
        } else {
            Ok(VcsStatus::Dirty)
        }
    }

    fn is_initialized(&self) -> Result<bool> {
        Ok(self.is_git_repo())
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
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        let output = self.run_git_command(&["worktree", "list", "--porcelain"])?;
        let mut workspaces = Vec::new();
        let mut current_name = String::new();
        let mut current_branch = String::new();
        let mut is_main = false;

        for line in output.lines() {
            if line.starts_with("worktree ") {
                if !current_name.is_empty() {
                    workspaces.push(Workspace::new(
                        current_name.clone(),
                        current_branch.clone(),
                        is_main,
                    ));
                }
                current_name = line.trim_start_matches("worktree ").to_string();
                current_branch = String::new();
                is_main = false;
            } else if line.starts_with("branch ") {
                current_branch = line.trim_start_matches("branch refs/heads/").to_string();
                is_main = current_branch == "main" || current_branch == "master";
            }
        }

        if !current_name.is_empty() {
            workspaces.push(Workspace::new(current_name, current_branch, is_main));
        }
        Ok(workspaces)
    }

    fn delete_workspace(&self, _name: &str) -> Result<()> {
        Err(VcsError::Unimplemented(
            "Git workspaces use worktrees instead".into(),
        ))
    }

    fn fork_workspace(&self, _source: &str, _target: &str) -> Result<()> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        self.run_git_command(&["worktree", "add"])?;
        Ok(())
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        self.switch_branch("main")?;
        self.merge(name)?;
        self.push()?;
        Ok(())
    }
}
