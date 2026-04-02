//! JJ (Jujutsu) backend implementation

use crate::error::Result;
use crate::error_io::IoErrorKind;
use crate::error_vcs::VcsErrorKind;
use crate::error_workspace::WorkspaceErrorKind;
use chrono::Utc;
use std::process::Command;

use super::types::{Branch, Commit, CommitId, RepoStatus, VcsStatus, Workspace};
use super::VcsBackend;

pub struct JjBackend {
    repo_path: std::path::PathBuf,
}

impl JjBackend {
    pub fn new(repo_path: std::path::PathBuf) -> Self {
        Self { repo_path }
    }

    fn run_jj(&self, args: &[&str]) -> Result<std::process::Output> {
        Command::new("jj")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| IoErrorKind::IoError(e.to_string()).into())
    }
}

impl VcsBackend for JjBackend {
    fn current_branch(&self) -> Result<String> {
        let output = self.run_jj(&["log", "-r", "@", "-T", " bookmarks()"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::Conflict(
                "jj".into(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn list_branches(&self) -> Result<Vec<Branch>> {
        let output = self.run_jj(&["bookmark", "list"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_jj_bookmark_list(&stdout))
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["bookmark", "create", name])?;
        if !output.status.success() {
            return Err(VcsErrorKind::BranchExists(name.to_string()).into());
        }
        Ok(())
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["bookmark", "set", name])?;
        if !output.status.success() {
            return Err(VcsErrorKind::BranchNotFound(name.to_string()).into());
        }
        Ok(())
    }

    fn push(&self) -> Result<()> {
        let output = self.run_jj(&["git", "push"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::PushFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn pull(&self) -> Result<()> {
        let output = self.run_jj(&["git", "fetch"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::PullFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn rebase(&self, onto: &str) -> Result<()> {
        let output = self.run_jj(&["rebase", "-d", onto])?;
        if !output.status.success() {
            return Err(VcsErrorKind::RebaseFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn merge(&self, branch: &str) -> Result<()> {
        let output = self.run_jj(&["merge", branch])?;
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
        let output = self.run_jj(&["log", "-n", &limit.to_string()])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_jj_log(&stdout))
    }

    fn status(&self) -> Result<VcsStatus> {
        let output = self.run_jj(&["status"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_jj_status(&stdout))
    }

    fn is_initialized(&self) -> Result<bool> {
        Ok(self.repo_path.join(".jj").exists())
    }

    fn repo_exists(&self, path: &str) -> bool {
        std::path::Path::new(path).join(".jj").exists()
    }

    fn checkout(&self, target: &str) -> Result<()> {
        let output = self.run_jj(&["new", target])?;
        if !output.status.success() {
            return Err(VcsErrorKind::CheckoutFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn commit(&self, message: &str) -> Result<CommitId> {
        let output = self.run_jj(&["commit", "-m", message])?;
        if !output.status.success() {
            return Err(VcsErrorKind::CommitFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        let log_output = self.run_jj(&["log", "-r", "@", "-T", "commit_id"])?;
        if !log_output.status.success() {
            return Err(VcsErrorKind::CommitFailed(
                "Failed to retrieve commit ID after commit".to_string(),
            )
            .into());
        }
        let id = String::from_utf8_lossy(&log_output.stdout).trim().to_string();
        CommitId::new(id)
            .ok_or_else(|| VcsErrorKind::CommitFailed("Commit returned empty ID".to_string()))
            .map_err(Into::into)
    }

    fn diff(&self, from: &CommitId, to: &CommitId) -> Result<String> {
        let output = self.run_jj(&["diff", "-r", &format!("{}..{}", from.as_str(), to.as_str())])?;
        if !output.status.success() {
            return Err(VcsErrorKind::DiffFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn repo_status(&self) -> Result<RepoStatus> {
        let vcs_status = self.status()?;
        let clean = vcs_status == VcsStatus::Clean;
        let has_conflicts = vcs_status == VcsStatus::Conflicted;

        // Get current branch
        let branch = self.current_branch().ok().and_then(|b| {
            let trimmed = b.trim().to_string();
            if trimmed.is_empty() { None } else { super::types::BranchName::new(trimmed) }
        });

        // Get current commit ID
        let commit_id = {
            let output = self.run_jj(&["log", "-r", "@", "-T", "commit_id"])?;
            if output.status.success() {
                let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                CommitId::from_unchecked(id)
            } else {
                return Err(VcsErrorKind::Conflict(
                    "repo_status".into(),
                    "Failed to get current commit ID".into(),
                )
                .into());
            }
        };

        // Get list of changed files
        let uncommitted_files = {
            let output = self.run_jj(&["status"])?;
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                parse_jj_status_files(&stdout)
            } else {
                vec![]
            }
        };

        Ok(RepoStatus {
            clean,
            branch,
            commit_id: Some(commit_id),
            has_conflicts,
            uncommitted_files,
        })
    }

    fn create_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "add", name])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") || stderr.contains("exists") {
                return Err(WorkspaceErrorKind::Exists(name.to_string()).into());
            }
            return Err(
                VcsErrorKind::Conflict("workspace add".to_string(), stderr.to_string()).into(),
            );
        }
        Ok(())
    }

    fn switch_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "root", "--name", name])?;
        if !output.status.success() {
            return Err(WorkspaceErrorKind::NotFound(name.to_string()).into());
        }
        let workspace_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("Workspace '{}' is at: {}", name, workspace_root);
        println!("To switch, run: cd {}", workspace_root);
        Ok(())
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let output = self.run_jj(&["workspace", "list"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_jj_workspace_list(&stdout))
    }

    fn delete_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "delete", name])?;
        if !output.status.success() {
            return Err(WorkspaceErrorKind::NotFound(name.to_string()).into());
        }
        Ok(())
    }

    fn fork_workspace(&self, source: &str, target: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "add", target, "-b", source])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") || stderr.contains("exists") {
                return Err(WorkspaceErrorKind::Exists(target.to_string()).into());
            }
            return Err(
                VcsErrorKind::Conflict("workspace fork".to_string(), stderr.to_string()).into(),
            );
        }
        Ok(())
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        let output = self.run_jj(&["workspace", "root", "--name", name])?;
        if !output.status.success() {
            return Err(WorkspaceErrorKind::NotFound(name.to_string()).into());
        }
        self.rebase("main")?;
        self.push()?;
        Ok(())
    }

    fn abort_workspace(&self, _name: &str) -> Result<()> {
        // Abort workspace by restoring working copy to last commit
        // This uses jj restore to discard uncommitted changes
        let output = self.run_jj(&["restore", "-r", "@"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::Conflict(
                "jj restore".into(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }
}

// ============================================================================
// Parsing helpers (extracted for unit testing without subprocess calls)
// ============================================================================

/// Parse `jj bookmark list` output into a list of branches.
fn parse_jj_bookmark_list(stdout: &str) -> Vec<Branch> {
    let mut branches = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('!') {
            let name = line.split(':').next().unwrap_or(line).trim();
            let name = name.trim_start_matches('*').trim();
            branches.push(Branch {
                name: name.to_string(),
                is_current: line.starts_with('*'),
                tracking: None,
            });
        }
    }
    branches
}

/// Parse `jj status` output into a VcsStatus.
fn parse_jj_status(stdout: &str) -> VcsStatus {
    if stdout.contains("There are conflicts") {
        return VcsStatus::Conflicted;
    }

    let has_changes = stdout.lines().any(|l| {
        let trimmed = l.trim();
        trimmed.starts_with("Modified:")
            || trimmed.starts_with("Added:")
            || trimmed.starts_with("Removed:")
    });

    if has_changes {
        VcsStatus::Dirty
    } else {
        VcsStatus::Clean
    }
}

/// Parse `jj workspace list` output into a list of workspaces.
fn parse_jj_workspace_list(stdout: &str) -> Vec<Workspace> {
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
    workspaces
}

/// Extract filenames from `jj status` output.
fn parse_jj_status_files(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty()
                && (trimmed.starts_with("Modified:")
                    || trimmed.starts_with("Added:")
                    || trimmed.starts_with("Removed:"))
        })
        .map(|l| l.trim().to_string())
        .collect()
}

/// Parse `jj log` output (single-line template) into Commit entries.
fn parse_jj_log(stdout: &str) -> Vec<Commit> {
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
    commits
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- JjBackend construction --

    #[test]
    fn jj_backend_new_stores_path() {
        let path = std::path::PathBuf::from("/tmp/test-repo");
        let backend = JjBackend::new(path.clone());
        assert_eq!(backend.repo_path, path);
    }

    #[test]
    fn jj_backend_is_initialized_true_when_jj_exists() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".jj")).expect("create .jj");
        let backend = JjBackend::new(dir.path().to_path_buf());
        assert_eq!(backend.is_initialized().expect("ok"), true);
    }

    #[test]
    fn jj_backend_is_initialized_false_when_no_jj() {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = JjBackend::new(dir.path().to_path_buf());
        assert_eq!(backend.is_initialized().expect("ok"), false);
    }

    #[test]
    fn jj_backend_repo_exists_true_when_jj_in_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".jj")).expect("create .jj");
        let backend = JjBackend::new("/tmp".into());
        assert!(backend.repo_exists(dir.path().to_str().expect("path")));
    }

    #[test]
    fn jj_backend_repo_exists_false_when_no_jj() {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = JjBackend::new("/tmp".into());
        assert!(!backend.repo_exists(dir.path().to_str().expect("path")));
    }

    // -- parse_jj_bookmark_list --

    #[test]
    fn parse_jj_bookmark_list_single_branch() {
        let output = "main: lzmmnrxq e5e10d2e (empty) Initial commit\n";
        let branches = parse_jj_bookmark_list(output);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
        assert!(!branches[0].is_current);
    }

    #[test]
    fn parse_jj_bookmark_list_current_branch_starred() {
        let output = "*main: lzmmnrxq e5e10d2e (empty) Initial commit\n";
        let branches = parse_jj_bookmark_list(output);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
        assert!(branches[0].is_current);
    }

    #[test]
    fn parse_jj_bookmark_list_multiple_branches() {
        let output = "\
main: lzmmnrxq e5e10d2e (empty) Initial commit
*feature/test: yostqyxq abc1234d Some feature
develop: kpqrsuvw def5678a Dev branch
";
        let branches = parse_jj_bookmark_list(output);
        assert_eq!(branches.len(), 3);
        assert_eq!(branches[0].name, "main");
        assert!(!branches[0].is_current);
        assert_eq!(branches[1].name, "feature/test");
        assert!(branches[1].is_current);
        assert_eq!(branches[2].name, "develop");
        assert!(!branches[2].is_current);
    }

    #[test]
    fn parse_jj_bookmark_list_empty_output() {
        let branches = parse_jj_bookmark_list("");
        assert!(branches.is_empty());
    }

    #[test]
    fn parse_jj_bookmark_list_skips_immutable_marker() {
        let output = "\
!main: lzmmnrxq e5e10d2e (empty) Initial commit
feature: yostqyxq abc1234d Some feature
";
        let branches = parse_jj_bookmark_list(output);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "feature");
    }

    #[test]
    fn parse_jj_bookmark_list_skips_whitespace_only_lines() {
        let output = "   \n\n  main: abc123 (empty) init\n  \n";
        let branches = parse_jj_bookmark_list(output);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
    }

    #[test]
    fn parse_jj_bookmark_list_trims_star_from_name() {
        let output = "*main: abc123\n";
        let branches = parse_jj_bookmark_list(output);
        assert_eq!(branches[0].name, "main");
    }

    #[test]
    fn parse_jj_bookmark_list_no_colon_falls_back_to_full_line() {
        let output = "simple-branch-name\n";
        let branches = parse_jj_bookmark_list(output);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "simple-branch-name");
    }

    // -- parse_jj_status --

    #[test]
    fn parse_jj_status_clean() {
        let output = "The working copy is clean.\n";
        assert_eq!(parse_jj_status(output), VcsStatus::Clean);
    }

    #[test]
    fn parse_jj_status_conflicted() {
        let output = "There are conflicts in 2 files.\n";
        assert_eq!(parse_jj_status(output), VcsStatus::Conflicted);
    }

    #[test]
    fn parse_jj_status_modified() {
        // The parser checks for lines starting with "Modified:" (colon immediately after)
        let output = "Modified: src/main.rs\n";
        assert_eq!(parse_jj_status(output), VcsStatus::Dirty);
    }

    #[test]
    fn parse_jj_status_added() {
        let output = "Added: new_file.txt\n";
        assert_eq!(parse_jj_status(output), VcsStatus::Dirty);
    }

    #[test]
    fn parse_jj_status_removed() {
        let output = "Removed: old_file.txt\n";
        assert_eq!(parse_jj_status(output), VcsStatus::Dirty);
    }

    #[test]
    fn parse_jj_status_empty() {
        assert_eq!(parse_jj_status(""), VcsStatus::Clean);
    }

    #[test]
    fn parse_jj_status_conflicts_take_precedence() {
        let output = "There are conflicts in 1 file.\nModified regular file:\n  src/main.rs\n";
        assert_eq!(parse_jj_status(output), VcsStatus::Conflicted);
    }

    // -- parse_jj_workspace_list --

    #[test]
    fn parse_jj_workspace_list_single() {
        let output = "default: /path/to/repo\n";
        let workspaces = parse_jj_workspace_list(output);
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].name, "default");
        assert!(!workspaces[0].is_current);
    }

    #[test]
    fn parse_jj_workspace_list_current_starred() {
        let output = "*default: /path/to/repo\n";
        let workspaces = parse_jj_workspace_list(output);
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].name, "default");
        assert!(workspaces[0].is_current);
    }

    #[test]
    fn parse_jj_workspace_list_multiple() {
        let output = "\
default: /repo
*feature: /repo-feature
hotfix: /repo-hotfix
";
        let workspaces = parse_jj_workspace_list(output);
        assert_eq!(workspaces.len(), 3);
        assert!(!workspaces[0].is_current);
        assert!(workspaces[1].is_current);
        assert!(!workspaces[2].is_current);
    }

    #[test]
    fn parse_jj_workspace_list_empty() {
        let workspaces = parse_jj_workspace_list("");
        assert!(workspaces.is_empty());
    }

    // -- parse_jj_status_files --

    #[test]
    fn parse_jj_status_files_empty() {
        assert!(parse_jj_status_files("").is_empty());
    }

    #[test]
    fn parse_jj_status_files_mixed() {
        let output = "Modified: src/main.rs\nAdded: new.txt\nRemoved: old.rs\n";
        let files = parse_jj_status_files(output);
        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.contains("main.rs")));
        assert!(files.iter().any(|f| f.contains("new.txt")));
        assert!(files.iter().any(|f| f.contains("old.rs")));
    }

    #[test]
    fn parse_jj_status_files_ignores_non_file_lines() {
        let output = "The working copy has changes.\nModified: src/lib.rs\n";
        let files = parse_jj_status_files(output);
        assert_eq!(files.len(), 1);
    }

    // -- parse_jj_log --

    #[test]
    fn parse_jj_log_multiple_entries() {
        let output = "\
abc123 feature commit
def456 initial commit
";
        let commits = parse_jj_log(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].id, "abc123 feature commit");
        assert_eq!(commits[1].id, "def456 initial commit");
    }

    #[test]
    fn parse_jj_log_empty() {
        let commits = parse_jj_log("");
        assert!(commits.is_empty());
    }

    #[test]
    fn parse_jj_log_skips_empty_lines() {
        let output = "abc123 commit1\n\n\ndef456 commit2\n";
        let commits = parse_jj_log(output);
        assert_eq!(commits.len(), 2);
    }

    // -- VcsStatus equality --

    #[test]
    fn vcs_status_equality() {
        assert_eq!(VcsStatus::Clean, VcsStatus::Clean);
        assert_eq!(VcsStatus::Dirty, VcsStatus::Dirty);
        assert_eq!(VcsStatus::Conflicted, VcsStatus::Conflicted);
        assert_eq!(VcsStatus::Detached, VcsStatus::Detached);
        assert_ne!(VcsStatus::Clean, VcsStatus::Dirty);
        assert_ne!(VcsStatus::Dirty, VcsStatus::Conflicted);
    }

    #[test]
    fn vcs_status_display() {
        assert_eq!(format!("{}", VcsStatus::Clean), "clean");
        assert_eq!(format!("{}", VcsStatus::Dirty), "dirty");
        assert_eq!(format!("{}", VcsStatus::Conflicted), "conflicted");
        assert_eq!(format!("{}", VcsStatus::Detached), "detached");
    }
}
