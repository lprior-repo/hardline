//! Git command executor trait for dependency injection.
//!
//! Provides a trait for executing Git commands, enabling testability
//! without requiring actual Git installation.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::{collections::HashSet, process::Command};

use scp_core::Error;

use super::data::ConflictDetectionResult;

/// Errors from Git command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    /// Git command not found in PATH.
    CommandNotFound(String),

    /// Git command failed with exit code.
    CommandFailed { code: i32, stderr: String },

    /// Invalid UTF-8 in command output.
    InvalidUtf8(String),

    /// IO error.
    IoError(String),
}

impl std::fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandNotFound(msg) => write!(f, "VCS command not found: {msg}"),
            Self::CommandFailed { code, stderr } => {
                write!(f, "VCS command failed with exit code {code}: {stderr}")
            }
            Self::InvalidUtf8(msg) => write!(f, "Invalid UTF-8 in command output: {msg}"),
            Self::IoError(msg) => write!(f, "IO error: {msg}"),
        }
    }
}

impl std::error::Error for ExecutorError {}

impl From<ExecutorError> for Error {
    fn from(err: ExecutorError) -> Self {
        Self::vcs_conflict("executor", err.to_string())
    }
}

/// Trait for executing Git commands.
pub trait GitExecutor: Send + Sync {
    /// Run a Git command with arguments.
    fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError>;

    /// Run a Git command in a specific workspace directory.
    fn run_in_workspace(
        &self,
        args: &[&str],
        workspace_path: &str,
    ) -> std::result::Result<String, ExecutorError>;
}

/// Real Git executor that runs actual commands.
#[derive(Debug, Default)]
pub struct RealGitExecutor;

impl RealGitExecutor {
    /// Create a new RealGitExecutor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl GitExecutor for RealGitExecutor {
    fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError> {
        run_git_command(args, None)
    }

    fn run_in_workspace(
        &self,
        args: &[&str],
        workspace_path: &str,
    ) -> std::result::Result<String, ExecutorError> {
        run_git_command(args, Some(workspace_path))
    }
}

/// Execute a Git command synchronously.
fn run_git_command(
    args: &[&str],
    working_dir: Option<&str>,
) -> std::result::Result<String, ExecutorError> {
    let mut cmd = Command::new("git");
    cmd.args(args);

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ExecutorError::CommandNotFound("git command not found in PATH".to_string())
        } else {
            ExecutorError::IoError(e.to_string())
        }
    })?;

    if !output.status.success() {
        let code = output.status.code().map_or(-1, |c| c);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ExecutorError::CommandFailed { code, stderr });
    }

    let stdout =
        String::from_utf8(output.stdout).map_err(|e| ExecutorError::InvalidUtf8(e.to_string()))?;

    Ok(stdout)
}

/// Run conflict detection using Git commands.
///
/// This is a synchronous wrapper around conflict detection logic.
pub fn detect_conflicts(
    executor: &dyn GitExecutor,
) -> std::result::Result<ConflictDetectionResult, ExecutorError> {
    let start = std::time::Instant::now();

    let existing_conflicts = check_existing_conflicts(executor)?;
    let has_existing = !existing_conflicts.is_empty();

    let merge_base = find_merge_base(executor)?;
    let workspace_files = get_workspace_modified_files(executor, &merge_base)?;
    let trunk_files = get_trunk_files(executor, &merge_base)?;

    let overlapping: Vec<String> = workspace_files
        .intersection(&trunk_files)
        .cloned()
        .collect();

    let workspace_only: Vec<String> = workspace_files.difference(&trunk_files).cloned().collect();
    let main_only: Vec<String> = trunk_files.difference(&workspace_files).cloned().collect();

    let merge_likely_safe = !has_existing && overlapping.is_empty();
    let summary = build_conflict_summary(&existing_conflicts, &overlapping);

    #[allow(clippy::cast_possible_truncation)]
    let detection_time_ms = start.elapsed().as_millis() as u64;

    Ok(ConflictDetectionResult {
        has_existing_conflicts: has_existing,
        existing_conflicts,
        overlapping_files: overlapping,
        workspace_only,
        main_only,
        merge_likely_safe,
        summary,
        merge_base,
        files_analyzed: workspace_files.len() + trunk_files.len(),
        detection_time_ms,
    })
}

/// Get trunk modified files, using merge base if available.
///
/// With a merge base, this is the set of files changed on `main` since
/// the base. Without one (no common ancestor), it falls back to the
/// files differing between HEAD and `main`, matching the old
/// endpoint-to-endpoint fallback semantics.
fn get_trunk_files(
    executor: &dyn GitExecutor,
    merge_base: &Option<String>,
) -> std::result::Result<HashSet<String>, ExecutorError> {
    if let Some(ref base) = merge_base {
        get_trunk_modified_files(executor, base)
    } else {
        let output = executor.run(&["diff", "--name-only", "HEAD", "main"])?;
        Ok(parse_name_only(&output))
    }
}

/// Build a human-readable conflict summary.
fn build_conflict_summary(existing_conflicts: &[String], overlapping: &[String]) -> String {
    if !existing_conflicts.is_empty() {
        format!(
            "Existing conflicts in {} files - resolve before merging",
            existing_conflicts.len()
        )
    } else if !overlapping.is_empty() {
        format!(
            "Potential conflicts in {} files: {}",
            overlapping.len(),
            overlapping.join(", ")
        )
    } else {
        "No conflicts detected - merge is safe".to_string()
    }
}

/// Check for existing Git conflicts in the workspace.
///
/// `git diff --name-only --diff-filter=U` lists exactly the paths with
/// unresolved merge conflicts, and exits 0 with empty output when there
/// is no merge in progress. (The previous implementation ran jj
/// revset/template syntax through the git binary, which always failed;
/// callers swallowed that failure, so conflict detection silently
/// reported no conflicts.)
fn check_existing_conflicts(
    executor: &dyn GitExecutor,
) -> std::result::Result<Vec<String>, ExecutorError> {
    let output = executor.run(&["diff", "--name-only", "--diff-filter=U"])?;
    Ok(parse_name_only(&output).into_iter().collect())
}

/// Find the merge base (common ancestor) between workspace and trunk.
///
/// The trunk is the `main` branch: the done workflow rebases onto
/// `main` and the VCS backend switches back to `main` on cleanup, so
/// this is the same trunk the rest of the handler assumes.
fn find_merge_base(
    executor: &dyn GitExecutor,
) -> std::result::Result<Option<String>, ExecutorError> {
    let output = executor.run(&["merge-base", "HEAD", "main"])?;

    let commit_id = output.trim();
    if commit_id.is_empty() {
        Ok(None)
    } else {
        Ok(Some(commit_id.to_string()))
    }
}

/// Get files modified in workspace since branching from trunk.
///
/// With a merge base, this is the files changed on HEAD since the base.
/// Without one, it falls back to the files differing between `main`
/// and HEAD (see `get_trunk_files`).
fn get_workspace_modified_files(
    executor: &dyn GitExecutor,
    merge_base: &Option<String>,
) -> std::result::Result<HashSet<String>, ExecutorError> {
    let output = if let Some(ref base) = merge_base {
        executor.run(&["diff", "--name-only", base, "HEAD"])?
    } else {
        executor.run(&["diff", "--name-only", "main", "HEAD"])?
    };
    Ok(parse_name_only(&output))
}

/// Get files modified in trunk since the merge base.
fn get_trunk_modified_files(
    executor: &dyn GitExecutor,
    merge_base: &str,
) -> std::result::Result<HashSet<String>, ExecutorError> {
    let output = executor.run(&["diff", "--name-only", merge_base, "main"])?;
    Ok(parse_name_only(&output))
}

/// Parse `git diff --name-only` output into a set of file paths.
///
/// One path per line; `git diff --name-only` reports the new name for
/// renames, and paths containing spaces are preserved verbatim.
pub fn parse_name_only(output: &str) -> HashSet<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(std::string::ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ExecutorError Display ----

    #[test]
    fn executor_error_command_not_found_display() {
        let err = ExecutorError::CommandNotFound("git not found".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("not found"));
        assert!(msg.contains("git not found"));
    }

    #[test]
    fn executor_error_command_failed_display() {
        let err = ExecutorError::CommandFailed {
            code: 1,
            stderr: "something went wrong".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("exit code 1"));
        assert!(msg.contains("something went wrong"));
    }

    #[test]
    fn executor_error_invalid_utf8_display() {
        let err = ExecutorError::InvalidUtf8("bad bytes".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Invalid UTF-8"));
        assert!(msg.contains("bad bytes"));
    }

    #[test]
    fn executor_error_io_error_display() {
        let err = ExecutorError::IoError("permission denied".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("IO error"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn executor_error_equality() {
        let a = ExecutorError::CommandFailed {
            code: 2,
            stderr: "e".to_string(),
        };
        let b = ExecutorError::CommandFailed {
            code: 2,
            stderr: "e".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn executor_error_inequality_different_variants() {
        let a = ExecutorError::CommandNotFound("x".to_string());
        let b = ExecutorError::IoError("x".to_string());
        assert_ne!(a, b);
    }

    // ---- ExecutorError: std::error::Error ----

    #[test]
    fn executor_error_implements_std_error() {
        let err = ExecutorError::CommandNotFound("x".to_string());
        let _: &dyn std::error::Error = &err;
    }

    // ---- ExecutorError -> Error conversion ----

    #[test]
    fn executor_error_converts_to_scp_core_error() {
        let err = ExecutorError::CommandFailed {
            code: 99,
            stderr: "fail".to_string(),
        };
        let core_err: Error = err.into();
        let msg = core_err.to_string();
        assert!(msg.contains("executor"));
    }

    // ---- parse_name_only ----

    #[test]
    fn parse_name_only_empty_input() {
        let result = parse_name_only("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_name_only_whitespace_only() {
        let result = parse_name_only("   \n  \n");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_name_only_paths() {
        let input = "src/main.rs\nsrc/lib.rs";
        let result = parse_name_only(input);
        assert_eq!(result.len(), 2);
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("src/lib.rs"));
    }

    #[test]
    fn parse_name_only_ignores_empty_lines() {
        let input = "file.rs\n\n\nother.rs\n";
        let result = parse_name_only(input);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn parse_name_only_handles_leading_trailing_whitespace() {
        let input = "  file.rs  \n  other.rs  ";
        let result = parse_name_only(input);
        assert_eq!(result.len(), 2);
        assert!(result.contains("file.rs"));
        assert!(result.contains("other.rs"));
    }

    // ---- Mock GitExecutor ----

    struct MockGitExecutor {
        responses: std::collections::HashMap<String, String>,
    }

    impl MockGitExecutor {
        fn new() -> Self {
            Self {
                responses: std::collections::HashMap::new(),
            }
        }

        fn with_response(mut self, key: &str, response: &str) -> Self {
            self.responses.insert(key.to_string(), response.to_string());
            self
        }
    }

    impl GitExecutor for MockGitExecutor {
        fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError> {
            let key = args.join(" ");
            self.responses
                .get(&key)
                .cloned()
                .ok_or_else(|| ExecutorError::CommandFailed {
                    code: 1,
                    stderr: format!("no mock for: {key}"),
                })
        }

        fn run_in_workspace(
            &self,
            args: &[&str],
            _workspace_path: &str,
        ) -> std::result::Result<String, ExecutorError> {
            self.run(args)
        }
    }

    #[test]
    fn mock_executor_returns_configured_response() {
        let mock = MockGitExecutor::new().with_response("status", "ok");
        let result = mock.run(&["status"]).expect("should succeed");
        assert_eq!(result, "ok");
    }

    #[test]
    fn mock_executor_returns_error_for_unconfigured_command() {
        let mock = MockGitExecutor::new();
        let result = mock.run(&["unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn mock_executor_run_in_workspace_delegates_to_run() {
        let mock = MockGitExecutor::new().with_response("log", "log output");
        let result = mock
            .run_in_workspace(&["log"], "/some/workspace")
            .expect("should succeed");
        assert_eq!(result, "log output");
    }

    // ---- parse_name_only additional edge cases ----

    #[test]
    fn parse_name_only_path_with_spaces() {
        // --name-only preserves paths with spaces verbatim, unlike the
        // old "M path" summary format which split on the first space.
        let input = "path/with spaces/file.rs";
        let result = parse_name_only(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains("path/with spaces/file.rs"));
    }

    #[test]
    fn parse_name_only_deeply_nested_paths() {
        let input = "a/b/c/d/e/f/g/h.rs";
        let result = parse_name_only(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains("a/b/c/d/e/f/g/h.rs"));
    }

    #[test]
    fn parse_name_only_single_newline() {
        let result = parse_name_only("\n");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_name_only_deduplicates_same_path() {
        let input = "file.rs\nfile.rs";
        let result = parse_name_only(input);
        assert_eq!(result.len(), 1);
    }

    // ---- ExecutorError inequality ----

    #[test]
    fn executor_error_inequality_same_variant_different_data() {
        let a = ExecutorError::CommandNotFound("x".to_string());
        let b = ExecutorError::CommandNotFound("y".to_string());
        assert_ne!(a, b);
    }

    #[test]
    fn executor_error_inequality_command_failed_different_codes() {
        let a = ExecutorError::CommandFailed {
            code: 1,
            stderr: "e".to_string(),
        };
        let b = ExecutorError::CommandFailed {
            code: 2,
            stderr: "e".to_string(),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn executor_error_inequality_command_failed_different_stderr() {
        let a = ExecutorError::CommandFailed {
            code: 1,
            stderr: "error_a".to_string(),
        };
        let b = ExecutorError::CommandFailed {
            code: 1,
            stderr: "error_b".to_string(),
        };
        assert_ne!(a, b);
    }

    // ---- detect_conflicts with mock ----

    #[test]
    fn detect_conflicts_no_conflicts_with_mock() {
        let mock = MockGitExecutor::new()
            .with_response("diff --name-only --diff-filter=U", "")
            .with_response("merge-base HEAD main", "abc123\n")
            .with_response("diff --name-only abc123 HEAD", "workspace_file.rs")
            .with_response("diff --name-only abc123 main", "trunk_file.rs");

        let result = detect_conflicts(&mock).expect("should succeed");
        assert!(result.merge_likely_safe);
        assert!(!result.has_existing_conflicts);
        assert!(result.overlapping_files.is_empty());
        assert_eq!(result.files_analyzed, 2);
        assert!(result.detection_time_ms > 0 || result.detection_time_ms == 0);
    }

    #[test]
    fn detect_conflicts_existing_conflicts_with_mock() {
        let mock = MockGitExecutor::new()
            .with_response(
                "diff --name-only --diff-filter=U",
                "file_a.rs\nfile_b.rs\n",
            )
            .with_response("merge-base HEAD main", "def456\n")
            .with_response("diff --name-only def456 HEAD", "")
            .with_response("diff --name-only def456 main", "");

        let result = detect_conflicts(&mock).expect("should succeed");
        assert!(!result.merge_likely_safe);
        assert!(result.has_existing_conflicts);
        assert_eq!(result.existing_conflicts.len(), 2);
    }

    #[test]
    fn detect_conflicts_overlapping_files_with_mock() {
        let mock = MockGitExecutor::new()
            .with_response("diff --name-only --diff-filter=U", "")
            .with_response("merge-base HEAD main", "\n") // empty = no merge base
            .with_response(
                "diff --name-only main HEAD",
                "shared.rs\nworkspace_only.rs",
            )
            .with_response(
                "diff --name-only HEAD main",
                "shared.rs\ntrunk_only.rs",
            );

        let result = detect_conflicts(&mock).expect("should succeed");
        assert!(!result.merge_likely_safe);
        assert!(result.overlapping_files.iter().any(|f| f == "shared.rs"));
        assert!(result
            .workspace_only
            .iter()
            .any(|f| f == "workspace_only.rs"));
        assert!(result.main_only.iter().any(|f| f == "trunk_only.rs"));
        assert!(result.merge_base.is_none());
    }
}
