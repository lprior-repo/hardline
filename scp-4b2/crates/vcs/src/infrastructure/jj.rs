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

// ============================================================================
// Pure Calculation Functions (Data → Calc)
// ============================================================================

fn extract_branch_name(trimmed: &str) -> &str {
    let name = match trimmed.split(':').next() {
        Some(n) => n,
        None => trimmed,
    };
    name.trim_start_matches('*').trim()
}

fn is_current_branch_line(trimmed: &str) -> bool {
    trimmed.starts_with('*')
}

fn is_valid_branch_line(trimmed: &str) -> bool {
    !trimmed.is_empty() && !trimmed.starts_with('!')
}

fn parse_branch_line(line: &str) -> Option<Branch> {
    let trimmed = line.trim();
    if !is_valid_branch_line(trimmed) {
        return None;
    }
    let name = extract_branch_name(trimmed);
    let is_current = is_current_branch_line(trimmed);
    Some(Branch::new(name.to_string(), is_current, None))
}

fn extract_workspace_name(trimmed: &str) -> &str {
    match trimmed.split(':').next() {
        Some(n) => n,
        None => trimmed,
    }
    .trim_start_matches('*')
    .trim()
}

fn parse_workspace_line(line: &str) -> Option<Workspace> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let name = extract_workspace_name(trimmed);
    let is_current = trimmed.starts_with('*');
    Some(Workspace::new(
        name.to_string(),
        name.to_string(),
        is_current,
    ))
}

fn parse_commit_line(line: &str) -> Option<Commit> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(Commit::new(
        trimmed.to_string(),
        trimmed.to_string(),
        "unknown".to_string(),
        Utc::now(),
        vec![],
    ))
}

fn detect_has_changes(stdout: &str) -> bool {
    stdout.lines().any(|l| {
        let trimmed = l.trim();
        trimmed.starts_with("Modified:")
            || trimmed.starts_with("Added:")
            || trimmed.starts_with("Removed:")
    })
}

fn contains_conflict_marker(stdout: &str) -> bool {
    stdout.contains("There are conflicts")
}

fn map_status_from_output(stdout: &str) -> VcsStatus {
    if contains_conflict_marker(stdout) {
        VcsStatus::Conflicted
    } else if detect_has_changes(stdout) {
        VcsStatus::Dirty
    } else {
        VcsStatus::Clean
    }
}

fn workspace_error_from_stderr(name: &str, stderr: &str) -> VcsError {
    if stderr.contains("already exists") || stderr.contains("exists") {
        VcsError::WorkspaceExists(name.to_string())
    } else {
        VcsError::Conflict("workspace add".to_string(), stderr.to_string())
    }
}

fn fork_workspace_error_from_stderr(name: &str, stderr: &str) -> VcsError {
    if stderr.contains("already exists") || stderr.contains("exists") {
        VcsError::WorkspaceExists(name.to_string())
    } else {
        VcsError::Conflict("workspace fork".to_string(), stderr.to_string())
    }
}

// ============================================================================
// VcsBackend Implementation
// ============================================================================

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

        let branches = stdout.lines().filter_map(parse_branch_line).collect();
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

        let commits = stdout.lines().filter_map(parse_commit_line).collect();
        Ok(commits)
    }

    fn status(&self) -> Result<VcsStatus> {
        let output = self.run_jj(&["status"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(map_status_from_output(&stdout))
    }

    fn is_initialized(&self) -> Result<bool> {
        Ok(self.repo_path.join(".jj").exists())
    }

    fn create_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "add", name])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(workspace_error_from_stderr(name, &stderr));
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

        let workspaces = stdout.lines().filter_map(parse_workspace_line).collect();
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
            return Err(fork_workspace_error_from_stderr(target, &stderr));
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

    #[test]
    fn test_parse_branch_line_current() {
        let branch = parse_branch_line("* main: abc123");
        assert!(branch.is_some());
        let branch = branch.unwrap();
        assert!(branch.is_current);
        assert_eq!(branch.name, "main");
    }

    #[test]
    fn test_parse_branch_line_not_current() {
        let branch = parse_branch_line("feature: def456");
        assert!(branch.is_some());
        let branch = branch.unwrap();
        assert!(!branch.is_current);
        assert_eq!(branch.name, "feature");
    }

    #[test]
    fn test_parse_branch_line_empty() {
        assert!(parse_branch_line("").is_none());
        assert!(parse_branch_line("   ").is_none());
        assert!(parse_branch_line("! ignored").is_none());
    }

    #[test]
    fn test_parse_commit_line() {
        let commit = parse_commit_line("abc123 test commit");
        assert!(commit.is_some());
        let commit = commit.unwrap();
        assert_eq!(commit.message, "abc123 test commit");
    }

    #[test]
    fn test_parse_commit_line_empty() {
        assert!(parse_commit_line("").is_none());
        assert!(parse_commit_line("   ").is_none());
    }

    #[test]
    fn test_detect_has_changes() {
        assert!(detect_has_changes("Modified: foo.rs\nAdded: bar.rs"));
        assert!(detect_has_changes("Removed: baz.rs"));
        assert!(!detect_has_changes("Only read-only changes"));
    }

    #[test]
    fn test_contains_conflict_marker() {
        assert!(contains_conflict_marker("There are conflicts"));
        assert!(!contains_conflict_marker("All good"));
    }

    #[test]
    fn test_map_status_from_output() {
        assert_eq!(
            map_status_from_output("There are conflicts"),
            VcsStatus::Conflicted
        );
        assert_eq!(map_status_from_output("Modified: foo.rs"), VcsStatus::Dirty);
        assert_eq!(map_status_from_output("Only read-only"), VcsStatus::Clean);
    }

    #[test]
    fn test_parse_workspace_line() {
        let ws = parse_workspace_line("* default: /path/to/workspace");
        assert!(ws.is_some());
        let ws = ws.unwrap();
        assert!(ws.is_current);
        assert_eq!(ws.name, "default");
    }

    #[test]
    fn test_parse_workspace_line_not_current() {
        let ws = parse_workspace_line("secondary: /path/to/secondary");
        assert!(ws.is_some());
        let ws = ws.unwrap();
        assert!(!ws.is_current);
        assert_eq!(ws.name, "secondary");
    }
}
