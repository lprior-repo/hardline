//! Shared test infrastructure for the done command handler.
//!
//! Provides mock executors, mock VCS backends, and canned response
//! builders used across the conflict, merge, and vcs_ops test modules.

#![cfg(test)]

use super::executor::ExecutorError;

/// A mock Git executor that records calls and returns canned responses
/// in FIFO order.
pub struct MockGitExecutor {
    responses: std::sync::Mutex<Vec<std::result::Result<String, ExecutorError>>>,
    calls: std::sync::Mutex<Vec<(String, Option<String>)>>,
}

impl MockGitExecutor {
    pub fn new(responses: Vec<std::result::Result<String, ExecutorError>>) -> Self {
        Self {
            responses: std::sync::Mutex::new(responses),
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn run_calls(&self) -> Vec<(String, Option<String>)> {
        self.calls.lock().expect("not poisoned").clone()
    }
}

impl super::executor::GitExecutor for MockGitExecutor {
    fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError> {
        self.calls
            .lock()
            .expect("not poisoned")
            .push((args.join(" "), None));
        let mut resp = self.responses.lock().expect("not poisoned");
        resp.remove(0)
    }

    fn run_in_workspace(
        &self,
        args: &[&str],
        workspace_path: &str,
    ) -> std::result::Result<String, ExecutorError> {
        self.calls
            .lock()
            .expect("not poisoned")
            .push((args.join(" "), Some(workspace_path.to_string())));
        let mut resp = self.responses.lock().expect("not poisoned");
        resp.remove(0)
    }
}

// ============================================================================
// Mock VcsBackend
// ============================================================================

/// A mock VCS backend that records calls and returns canned results.
pub struct MockVcsBackend {
    pub workspaces: Vec<scp_core::Workspace>,
    pub rebase_should_fail: bool,
    pub push_should_fail: bool,
    pub delete_workspace_should_fail: bool,
    pub log_entries: Vec<scp_core::Commit>,
}

impl MockVcsBackend {
    pub fn new(workspaces: Vec<scp_core::Workspace>) -> Self {
        Self {
            workspaces,
            rebase_should_fail: false,
            push_should_fail: false,
            delete_workspace_should_fail: false,
            log_entries: Vec::new(),
        }
    }

    pub fn with_rebase_failure(mut self) -> Self {
        self.rebase_should_fail = true;
        self
    }

    pub fn with_push_failure(mut self) -> Self {
        self.push_should_fail = true;
        self
    }

    pub fn with_delete_workspace_failure(mut self) -> Self {
        self.delete_workspace_should_fail = true;
        self
    }

    pub fn with_log_entries(mut self, entries: Vec<scp_core::Commit>) -> Self {
        self.log_entries = entries;
        self
    }
}

impl scp_core::VcsBackend for MockVcsBackend {
    fn current_branch(&self) -> scp_core::Result<String> {
        Ok("main".to_string())
    }

    fn list_branches(&self) -> scp_core::Result<Vec<scp_core::Branch>> {
        Ok(vec![])
    }

    fn create_branch(&self, _name: &str) -> scp_core::Result<()> {
        Ok(())
    }

    fn switch_branch(&self, _name: &str) -> scp_core::Result<()> {
        Ok(())
    }

    fn push(&self) -> scp_core::Result<()> {
        if self.push_should_fail {
            Err(scp_core::Error::io_error("push rejected by remote"))
        } else {
            Ok(())
        }
    }

    fn pull(&self) -> scp_core::Result<()> {
        Ok(())
    }

    fn rebase(&self, _onto: &str) -> scp_core::Result<()> {
        if self.rebase_should_fail {
            Err(scp_core::Error::io_error("rebase failed"))
        } else {
            Ok(())
        }
    }

    fn merge(&self, _branch: &str) -> scp_core::Result<()> {
        Ok(())
    }

    fn log(&self, _limit: usize) -> scp_core::Result<Vec<scp_core::Commit>> {
        Ok(self.log_entries.clone())
    }

    fn status(&self) -> scp_core::Result<scp_core::VcsStatus> {
        Ok(scp_core::VcsStatus::Clean)
    }

    fn is_initialized(&self) -> scp_core::Result<bool> {
        Ok(true)
    }

    fn repo_exists(&self, _path: &str) -> bool {
        true
    }

    fn checkout(&self, _target: &str) -> scp_core::Result<()> {
        Ok(())
    }

    fn commit(&self, _message: &str) -> scp_core::Result<scp_core::vcs::CommitId> {
        Ok(scp_core::vcs::CommitId::new("fake-commit-id").expect("valid commit id"))
    }

    fn diff(
        &self,
        _from: &scp_core::vcs::CommitId,
        _to: &scp_core::vcs::CommitId,
    ) -> scp_core::Result<String> {
        Ok(String::new())
    }

    fn repo_status(&self) -> scp_core::Result<scp_core::vcs::RepoStatus> {
        Ok(scp_core::vcs::RepoStatus::default())
    }

    fn create_workspace(&self, _name: &str) -> scp_core::Result<()> {
        Ok(())
    }

    fn switch_workspace(&self, _name: &str) -> scp_core::Result<()> {
        Ok(())
    }

    fn list_workspaces(&self) -> scp_core::Result<Vec<scp_core::Workspace>> {
        Ok(self.workspaces.clone())
    }

    fn delete_workspace(&self, _name: &str) -> scp_core::Result<()> {
        if self.delete_workspace_should_fail {
            Err(scp_core::Error::io_error("delete workspace failed"))
        } else {
            Ok(())
        }
    }

    fn fork_workspace(&self, _source: &str, _target: &str) -> scp_core::Result<()> {
        Ok(())
    }

    fn merge_workspace(&self, _name: &str) -> scp_core::Result<()> {
        Ok(())
    }

    fn abort_workspace(&self, _name: &str) -> scp_core::Result<()> {
        Ok(())
    }
}

// ============================================================================
// Canned response builders
// ============================================================================

/// Clean workspace with no conflicts.
pub fn no_conflict_responses() -> Vec<std::result::Result<String, ExecutorError>> {
    vec![
        Ok(String::new()),
        Ok(String::new()),
        Ok(String::new()),
        Ok(String::new()),
    ]
}

/// Workspace with existing conflicts.
pub fn existing_conflict_responses() -> Vec<std::result::Result<String, ExecutorError>> {
    vec![
        Ok("CONFLICT\n".to_string()),
        Ok("src/conflicted.rs\n".to_string()),
        Ok(String::new()),
        Ok("M src/conflicted.rs\n".to_string()),
        Ok("M trunk_file.rs\n".to_string()),
    ]
}

/// Workspace with overlapping files but no existing conflicts.
pub fn overlapping_conflict_responses() -> Vec<std::result::Result<String, ExecutorError>> {
    vec![
        Ok(String::new()),
        Ok("abc123merge\n".to_string()),
        Ok("M shared.rs\n".to_string()),
        Ok("M shared.rs\n".to_string()),
    ]
}

/// Empty diff (no workspace changes).
pub fn empty_diff_responses() -> Vec<std::result::Result<String, ExecutorError>> {
    vec![
        Ok(String::new()),
        Ok("abc123\n".to_string()),
        Ok(String::new()),
        Ok("M trunk_only.rs\n".to_string()),
    ]
}

/// Workspace with changes but no conflicts.
pub fn workspace_with_changes_responses() -> Vec<std::result::Result<String, ExecutorError>> {
    vec![
        Ok(String::new()),
        Ok("base123\n".to_string()),
        Ok("M src/new_feature.rs\nA src/new_file.rs\n".to_string()),
        Ok("M trunk_change.rs\n".to_string()),
    ]
}
