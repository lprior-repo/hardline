//! Git CLI Backend - Core Implementation
//!
//! Executes git CLI commands and parses output into domain types.

use crate::domain::value_objects::VcsStatus;
use crate::error::{Result, VcsError};
use chrono::{DateTime, TimeZone, Utc};
use std::process::Command;

pub struct GitCliBackend {
    repo_path: std::path::PathBuf,
}

impl GitCliBackend {
    pub fn new(repo_path: std::path::PathBuf) -> Self {
        Self { repo_path }
    }

    pub fn new_from_path(path: impl Into<std::path::PathBuf>) -> Self {
        Self::new(path.into())
    }

    pub fn repo_path(&self) -> &std::path::PathBuf {
        &self.repo_path
    }

    pub(crate) fn is_git_repo(&self) -> bool {
        self.repo_path.join(".git").exists()
    }

    pub fn diff(&self) -> Result<String> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        let output = self.run_git_command(&["diff"])?;
        Ok(output)
    }

    pub fn diff_staged(&self) -> Result<String> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        let output = self.run_git_command(&["diff", "--cached"])?;
        Ok(output)
    }

    pub fn add(&self, paths: &[&str]) -> Result<()> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        let mut args = vec!["add"];
        args.extend(paths.iter());
        let args_refs: Vec<&str> = args.iter().map(|s| s as &str).collect();
        self.run_git_command(&args_refs)?;
        Ok(())
    }

    pub fn commit(&self, message: &str) -> Result<String> {
        if !self.is_git_repo() {
            return Err(VcsError::NotInitialized);
        }
        self.run_git_command_with_stdin(&["commit", "-F", "-"], message)?;
        self.run_git_command(&["rev-parse", "HEAD"])
    }

    pub fn status(&self) -> Result<VcsStatus> {
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

    pub fn is_initialized(&self) -> Result<bool> {
        Ok(self.is_git_repo())
    }

    pub(crate) fn run_git_command(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .env("LC_ALL", "C")
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    VcsError::GitNotInstalled
                } else {
                    VcsError::Io(e)
                }
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            if stderr.contains("Not a git repository")
                || stderr.contains("fatal: not a git repository")
            {
                Err(VcsError::NotInitialized)
            } else if stderr.contains("does not exist") || stderr.contains("not found") {
                Err(VcsError::BranchNotFound(stderr))
            } else if stderr.contains("already exists") {
                Err(VcsError::BranchExists(stderr))
            } else {
                Err(VcsError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("git exited with {}: {}", exit_code, stderr),
                )))
            }
        }
    }

    pub(crate) fn run_git_command_with_stdin(&self, args: &[&str], input: &str) -> Result<String> {
        use std::io::Write;
        use std::process::Stdio;

        let mut child = Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .env("LC_ALL", "C")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    VcsError::GitNotInstalled
                } else {
                    VcsError::Io(e)
                }
            })?;

        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(input.as_bytes()).map_err(VcsError::Io)?;
        }

        let output = child.wait_with_output().map_err(VcsError::Io)?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(VcsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("git exited with {:?}: {}", output.status.code(), stderr),
            )))
        }
    }

    pub(crate) fn parse_timestamp(timestamp: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(timestamp, 0)
            .single()
            .unwrap_or_else(Utc::now)
    }
}
