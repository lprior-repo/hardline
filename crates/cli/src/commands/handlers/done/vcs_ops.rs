//! VCS operations for the done command handler.
//!
//! Lower-level helpers for workspace resolution, file introspection,
//! commit parsing, undo history, and the WorkspaceGitExecutor wrapper.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{CommitInfo, UndoEntry};
use super::executor::{detect_conflicts, GitExecutor, ExecutorError};

// ============================================================================
// Workspace Resolution
// ============================================================================

/// Resolve workspace name from option or current workspace.
pub(crate) fn resolve_workspace(backend: &dyn scp_core::vcs::VcsBackend, name: Option<&str>) -> Result<String> {
    match name {
        Some(n) => Ok(n.to_string()),
        None => {
            let workspaces = backend.list_workspaces()?;
            workspaces
                .iter()
                .find(|w| w.is_current)
                .map(|w| w.name.clone())
                .ok_or_else(|| Error::workspace_not_found("no current workspace"))
        }
    }
}

/// Get the workspace path for a given workspace name.
pub(crate) fn get_workspace_path(
    cwd: &Path,
    workspace_name: &str,
    backend: &dyn scp_core::vcs::VcsBackend,
) -> Result<PathBuf> {
    let workspaces = backend.list_workspaces()?;
    let is_current = workspaces
        .iter()
        .any(|w| w.name == workspace_name && w.is_current);

    if is_current {
        Ok(cwd.to_path_buf())
    } else {
        let workspace_path = cwd.join(".git").join("worktrees").join(workspace_name);
        if workspace_path.exists() {
            Ok(workspace_path)
        } else {
            Ok(cwd.to_path_buf())
        }
    }
}

// ============================================================================
// File & Commit Introspection
// ============================================================================

/// Get list of uncommitted files via Git status.
pub(crate) fn get_uncommitted_files(executor: &dyn GitExecutor) -> Result<Vec<String>> {
    let output = executor
        .run(&["status", "--no-pager"])
        .map_err(Error::from)?;

    Ok(parse_status_lines(&output))
}

/// Parse Git status output lines into file names.
///
/// Recognised prefixes: `A ` (added), `M ` (modified), `D ` (deleted),
/// `R ` (renamed).  Everything else is ignored.
fn parse_status_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.starts_with("A ")
                || line.starts_with("M ")
                || line.starts_with("D ")
                || line.starts_with("R ")
        })
        .filter_map(|line| line.split_ascii_whitespace().nth(1))
        .map(String::from)
        .collect()
}

/// Get commits that will be merged.
pub(crate) fn get_commits_to_merge(executor: &dyn GitExecutor) -> Result<Vec<CommitInfo>> {
    let output = executor
        .run(&[
            "log",
            "-r",
            "@..@-",
            "--no-graph",
            "-T",
            r#"change_id ++ "\n" ++ commit_id ++ "\n" ++ description ++ "\n" ++ committer.timestamp() ++ "\n""#,
        ])
        .map_err(Error::from)?;

    Ok(parse_commits_output(&output))
}

/// Parse Git log output into `CommitInfo` entries.
///
/// Each commit is 4 consecutive lines: change_id, commit_id, description,
/// timestamp.
fn parse_commits_output(output: &str) -> Vec<CommitInfo> {
    let mut commits = Vec::new();
    let mut lines = output.lines().peekable();

    while lines.peek().is_some() {
        let change_id = lines.next().map_or("", |s| s).trim().to_string();
        let commit_id = lines.next().map_or("", |s| s).trim().to_string();
        let description = lines.next().map_or("", |s| s).trim().to_string();
        let timestamp = lines.next().map_or("", |s| s).trim().to_string();

        if !change_id.is_empty() {
            commits.push(CommitInfo {
                change_id,
                commit_id,
                description,
                timestamp,
            });
        }
    }

    commits
}

/// Get potential conflicts via conflict detection.
///
/// Best-effort: logs a warning on failure but returns an empty list.
pub(crate) fn get_potential_conflicts(executor: &dyn GitExecutor) -> Vec<String> {
    match detect_conflicts(executor) {
        Ok(result) => {
            let mut conflicts = result.existing_conflicts;
            conflicts.extend(result.overlapping_files);
            conflicts
        }
        Err(e) => {
            Output::warn(&format!("Conflict detection failed: {e}"));
            Vec::new()
        }
    }
}

// ============================================================================
// Undo History
// ============================================================================

/// Log undo history to `.scp/undo.log`.
pub(crate) fn log_undo_history(
    workspace_name: &str,
    executor: &dyn GitExecutor,
    pushed_to_remote: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let undo_log_path = cwd.join(".scp/undo.log");

    let pre_merge_commit_id = executor
        .run(&["log", "-r", "@", "--no-graph", "-T", "commit_id"])
        .unwrap_or_default();

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let undo_entry = UndoEntry {
        session_name: workspace_name.to_string(),
        commit_id: String::new(),
        pre_merge_commit_id: pre_merge_commit_id.trim().to_string(),
        timestamp,
        pushed_to_remote,
        status: "completed".to_string(),
    };

    let json = serde_json::to_string(&undo_entry).map_err(|e| Error::io_error(e.to_string()))?;

    let mut content = if undo_log_path.exists() {
        std::fs::read_to_string(&undo_log_path)
            .map_err(|e| Error::io_error(format!("Failed to read undo log: {e}")))?
    } else {
        String::new()
    };
    content.push_str(&json);
    content.push('\n');

    if let Some(parent) = undo_log_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::io_error(format!("Failed to create undo log directory: {e}")))?;
    }

    std::fs::write(&undo_log_path, &content)
        .map_err(|e| Error::io_error(format!("Failed to write undo log: {e}")))?;

    Ok(())
}

/// Update workspace state to Merged.
///
/// Placeholder for future session state management integration.
#[allow(clippy::unused_self)]
pub(crate) fn update_workspace_state(_workspace_name: &str) -> bool {
    false
}

// ============================================================================
// Workspace Git Executor Wrapper
// ============================================================================

/// Executor that runs Git commands in a specific workspace directory.
pub(crate) struct WorkspaceGitExecutor<'a> {
    inner: &'a dyn GitExecutor,
    workspace_path: String,
}

impl<'a> WorkspaceGitExecutor<'a> {
    pub(crate) fn new(inner: &'a dyn GitExecutor, workspace_path: &str) -> Self {
        Self {
            inner,
            workspace_path: workspace_path.to_string(),
        }
    }
}

impl GitExecutor for WorkspaceGitExecutor<'_> {
    fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError> {
        self.inner.run_in_workspace(args, &self.workspace_path)
    }

    fn run_in_workspace(
        &self,
        args: &[&str],
        workspace_path: &str,
    ) -> std::result::Result<String, ExecutorError> {
        self.inner.run_in_workspace(args, workspace_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::done::test_support::*;

    // -----------------------------------------------------------------------
    // parse_status_lines
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_status_lines_basic() {
        let output = "M src/lib.rs\nA src/new.rs\nD src/old.rs\nR src/renamed.rs";
        let files = parse_status_lines(output);
        assert_eq!(
            files,
            vec!["src/lib.rs", "src/new.rs", "src/old.rs", "src/renamed.rs",]
        );
    }

    #[test]
    fn test_parse_status_lines_empty() {
        let files = parse_status_lines("");
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_status_lines_with_noise() {
        let output = "M src/lib.rs\nThe working copy is clean\nA src/new.rs\n";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/lib.rs", "src/new.rs"]);
    }

    #[test]
    fn test_parse_status_lines_untracked_ignored() {
        let output = "? untracked.rs\nM tracked.rs\n? another.untracked";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["tracked.rs"]);
    }

    #[test]
    fn test_parse_status_lines_file_with_spaces() {
        let output = "M path with spaces/file.txt";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["path"]);
    }

    #[test]
    fn test_parse_status_lines_copied_prefix_ignored() {
        let output = "C src/copied.rs\nM src/modified.rs";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/modified.rs"]);
    }

    #[test]
    fn test_parse_status_lines_various_unrecognized_prefixes() {
        let output = "L src/locked.rs\nA src/added.rs\nT src/type_changed.rs\nD src/deleted.rs";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/added.rs", "src/deleted.rs"]);
    }

    #[test]
    fn test_parse_status_lines_lowercase_prefixes_ignored() {
        let output = "m src/lowercase.rs\nM src/uppercase.rs";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/uppercase.rs"]);
    }

    #[test]
    fn test_parse_status_lines_only_unrecognized_prefixes() {
        let output = "C copy.rs\nL lock.rs\nT type.rs\n? untracked.rs";
        let files = parse_status_lines(output);
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_status_lines_tab_separated() {
        let output = "M\tsrc/tab_separated.rs\nA\tsrc/other.rs";
        let files = parse_status_lines(output);
        assert!(files.is_empty(), "tab-separated lines should not be parsed");
    }

    // -----------------------------------------------------------------------
    // parse_commits_output
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_commits_output_basic() {
        let output = "abc123\ndef456\nfeat: add widget\n2024-01-15 10:00:00\n";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "abc123");
        assert_eq!(commits[0].commit_id, "def456");
        assert_eq!(commits[0].description, "feat: add widget");
        assert_eq!(commits[0].timestamp, "2024-01-15 10:00:00");
    }

    #[test]
    fn test_parse_commits_output_multiple() {
        let output = "abc123\ndef456\nfeat: first\n2024-01-15 10:00:00\n\
                      xyz789\nghi012\nfix: second\n2024-01-15 11:00:00\n";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].change_id, "abc123");
        assert_eq!(commits[1].change_id, "xyz789");
    }

    #[test]
    fn test_parse_commits_output_empty() {
        let commits = parse_commits_output("");
        assert!(commits.is_empty());
    }

    #[test]
    fn test_parse_commits_output_incomplete_trailing() {
        let output = "abc123\ndef456\nfeat: add widget\n2024-01-15 10:00:00\n\
                      partial_id";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[1].change_id, "partial_id");
        assert!(commits[1].commit_id.is_empty());
        assert!(commits[1].description.is_empty());
        assert!(commits[1].timestamp.is_empty());
    }

    #[test]
    fn test_parse_commits_output_skips_empty_leading() {
        let output = "\n\nabc123\ndef456\ndesc\nts\n";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "desc");
        assert_eq!(commits[0].commit_id, "ts");
    }

    #[test]
    fn test_parse_commits_output_multiline_description() {
        let output = "abc123\ndef456\nfeat: add widget\nwith details\n2024-01-15 10:00:00\n";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].change_id, "abc123");
        assert_eq!(commits[0].description, "feat: add widget");
        assert_eq!(commits[0].timestamp, "with details");
        assert_eq!(commits[1].change_id, "2024-01-15 10:00:00");
    }

    #[test]
    fn test_parse_commits_output_single_line() {
        let output = "lone_change_id";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "lone_change_id");
        assert!(commits[0].commit_id.is_empty());
    }

    #[test]
    fn test_parse_commits_output_two_lines() {
        let output = "ch_id\ncm_id";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "ch_id");
        assert_eq!(commits[0].commit_id, "cm_id");
    }

    #[test]
    fn test_parse_commits_output_three_lines() {
        let output = "ch_id\ncm_id\ndesc line";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].description, "desc line");
    }

    #[test]
    fn test_parse_commits_output_empty_change_id_is_skipped() {
        let output = "\nabc\ndef\nghi";
        let commits = parse_commits_output(output);
        assert!(commits.is_empty());
    }

    // -----------------------------------------------------------------------
    // WorkspaceGitExecutor delegation
    // -----------------------------------------------------------------------

    #[test]
    fn test_workspace_executor_delegates_run() {
        let mock = MockGitExecutor::new(vec![Ok("result".to_string())]);
        let ws = WorkspaceGitExecutor::new(&mock, "/tmp/workspace");

        let result = ws.run(&["status", "--no-pager"]);
        assert_eq!(result.unwrap(), "result");

        let calls = mock.run_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "status --no-pager");
        assert_eq!(calls[0].1.as_deref(), Some("/tmp/workspace"));
    }

    #[test]
    fn test_workspace_executor_delegates_run_in_workspace() {
        let mock = MockGitExecutor::new(vec![Ok("in-workspace-result".to_string())]);
        let ws = WorkspaceGitExecutor::new(&mock, "/tmp/ws-a");

        let result = ws.run_in_workspace(&["diff", "--summary"], "/tmp/ws-b");
        assert_eq!(result.unwrap(), "in-workspace-result");

        let calls = mock.run_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "diff --summary");
        assert_eq!(calls[0].1.as_deref(), Some("/tmp/ws-b"));
    }

    #[test]
    fn test_workspace_executor_propagates_errors() {
        let mock = MockGitExecutor::new(vec![Err(ExecutorError::CommandNotFound(
            "git not found".to_string(),
        ))]);
        let ws = WorkspaceGitExecutor::new(&mock, "/tmp/workspace");

        let result = ws.run(&["log"]);
        assert_eq!(
            result.unwrap_err(),
            ExecutorError::CommandNotFound("git not found".to_string()),
        );
    }

    // -----------------------------------------------------------------------
    // get_uncommitted_files via mock executor
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_uncommitted_files_clean_tree() {
        let executor = MockGitExecutor::new(vec![Ok("The working copy is clean\n".to_string())]);
        let files = get_uncommitted_files(&executor).expect("should succeed");
        assert!(files.is_empty());
    }

    #[test]
    fn test_get_uncommitted_files_with_changes() {
        let executor = MockGitExecutor::new(vec![Ok(
            "M src/lib.rs\nA src/new.rs\nD src/old.rs\nR src/renamed.rs\n".to_string(),
        )]);
        let files = get_uncommitted_files(&executor).expect("should succeed");
        assert_eq!(files.len(), 4);
        assert!(files.contains(&"src/lib.rs".to_string()));
        assert!(files.contains(&"src/new.rs".to_string()));
    }

    #[test]
    fn test_get_uncommitted_files_executor_error_propagates() {
        let executor = MockGitExecutor::new(vec![Err(ExecutorError::CommandFailed {
            code: 1,
            stderr: "git status failed".to_string(),
        })]);
        let result = get_uncommitted_files(&executor);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // get_commits_to_merge via mock executor
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_commits_to_merge_multiple() {
        let executor =
            MockGitExecutor::new(vec![Ok("ch1\ncm1\nfeat: first\n2024-01-15 10:00:00\n\
             ch2\ncm2\nfix: second\n2024-01-15 11:00:00\n"
                .to_string())]);
        let commits = get_commits_to_merge(&executor).expect("should succeed");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].change_id, "ch1");
        assert_eq!(commits[1].description, "fix: second");
    }

    #[test]
    fn test_get_commits_to_merge_empty() {
        let executor = MockGitExecutor::new(vec![Ok(String::new())]);
        let commits = get_commits_to_merge(&executor).expect("should succeed");
        assert!(commits.is_empty());
    }

    #[test]
    fn test_get_commits_to_merge_executor_error_propagates() {
        let executor = MockGitExecutor::new(vec![Err(ExecutorError::CommandFailed {
            code: 1,
            stderr: "git log failed".to_string(),
        })]);
        let result = get_commits_to_merge(&executor);
        assert!(result.is_err());
    }
}
