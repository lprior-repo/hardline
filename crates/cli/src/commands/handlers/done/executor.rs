//! JJ command executor trait for dependency injection.
//!
//! Provides a trait for executing JJ commands, enabling testability
//! without requiring actual JJ installation.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::collections::HashSet;
use std::process::Command;

use scp_core::Error;

use super::data::ConflictDetectionResult;

/// Errors from JJ command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    /// JJ command not found in PATH.
    CommandNotFound(String),

    /// JJ command failed with exit code.
    CommandFailed { code: i32, stderr: String },

    /// Invalid UTF-8 in command output.
    InvalidUtf8(String),

    /// IO error.
    IoError(String),
}

impl std::fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandNotFound(msg) => write!(f, "JJ command not found: {msg}"),
            Self::CommandFailed { code, stderr } => {
                write!(f, "JJ command failed with exit code {code}: {stderr}")
            }
            Self::InvalidUtf8(msg) => write!(f, "Invalid UTF-8 in command output: {msg}"),
            Self::IoError(msg) => write!(f, "IO error: {msg}"),
        }
    }
}

impl std::error::Error for ExecutorError {}

impl From<ExecutorError> for Error {
    fn from(err: ExecutorError) -> Self {
        Error::vcs_conflict("executor", err.to_string())
    }
}

/// Trait for executing JJ commands.
pub trait JjExecutor: Send + Sync {
    /// Run a JJ command with arguments.
    fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError>;

    /// Run a JJ command in a specific workspace directory.
    fn run_in_workspace(
        &self,
        args: &[&str],
        workspace_path: &str,
    ) -> std::result::Result<String, ExecutorError>;
}

/// Real JJ executor that runs actual commands.
#[derive(Debug, Default)]
pub struct RealJjExecutor;

impl RealJjExecutor {
    /// Create a new RealJjExecutor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl JjExecutor for RealJjExecutor {
    fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError> {
        run_jj_command(args, None)
    }

    fn run_in_workspace(
        &self,
        args: &[&str],
        workspace_path: &str,
    ) -> std::result::Result<String, ExecutorError> {
        run_jj_command(args, Some(workspace_path))
    }
}

/// Execute a JJ command synchronously.
fn run_jj_command(
    args: &[&str],
    working_dir: Option<&str>,
) -> std::result::Result<String, ExecutorError> {
    let mut cmd = Command::new("jj");
    cmd.args(args);

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ExecutorError::CommandNotFound("jj command not found in PATH".to_string())
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

/// Run conflict detection using JJ commands.
///
/// This is a synchronous wrapper around conflict detection logic.
pub fn detect_conflicts(
    executor: &dyn JjExecutor,
) -> std::result::Result<ConflictDetectionResult, ExecutorError> {
    let start = std::time::Instant::now();

    // Step 1: Check for existing conflicts
    let existing_conflicts = check_existing_conflicts(executor)?;
    let has_existing = !existing_conflicts.is_empty();

    // Step 2: Find merge base
    let merge_base = find_merge_base(executor)?;

    // Step 3: Get workspace modified files
    let workspace_files = get_workspace_modified_files(executor)?;

    // Step 4: Get trunk modified files
    let trunk_files = if let Some(ref base) = merge_base {
        get_trunk_modified_files(executor, base)?
    } else {
        // If no merge base found, get diff between @ and trunk()
        let output = executor.run(&["diff", "--from", "@", "--to", "trunk()", "--summary"])?;
        parse_diff_summary(&output)
    };

    // Step 5: Compute overlapping files
    let overlapping: Vec<String> = workspace_files
        .intersection(&trunk_files)
        .cloned()
        .collect();

    let workspace_only: Vec<String> = workspace_files.difference(&trunk_files).cloned().collect();
    let main_only: Vec<String> = trunk_files.difference(&workspace_files).cloned().collect();

    // Step 6: Determine if merge is safe
    let merge_likely_safe = !has_existing && overlapping.is_empty();

    // Step 7: Generate summary
    let summary = if has_existing {
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
    };

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

/// Check for existing JJ conflicts in the workspace.
fn check_existing_conflicts(
    executor: &dyn JjExecutor,
) -> std::result::Result<Vec<String>, ExecutorError> {
    let output = executor.run(&[
        "log",
        "-r",
        "@",
        "--no-graph",
        "-T",
        r#"if(conflict, "CONFLICT\n", "")"#,
    ])?;

    if output.contains("CONFLICT") {
        let resolve_output = executor.run(&["resolve", "--list"])?;

        let conflicts: Vec<String> = resolve_output
            .lines()
            .filter(|line: &&str| !line.trim().is_empty())
            .map(|line: &str| {
                line.split_whitespace()
                    .next()
                    .map_or_else(|| line.trim().to_string(), str::to_string)
            })
            .collect();

        Ok(conflicts)
    } else {
        Ok(Vec::new())
    }
}

/// Find the merge base (common ancestor) between workspace and trunk.
fn find_merge_base(
    executor: &dyn JjExecutor,
) -> std::result::Result<Option<String>, ExecutorError> {
    let output = executor.run(&[
        "log",
        "-r",
        "heads(::@ & ::trunk())",
        "--no-graph",
        "-T",
        "commit_id ++ \"\\n\"",
        "--limit",
        "1",
    ])?;

    let commit_id = output.trim();
    if commit_id.is_empty() {
        Ok(None)
    } else {
        Ok(Some(commit_id.to_string()))
    }
}

/// Get files modified in workspace since branching from trunk.
fn get_workspace_modified_files(
    executor: &dyn JjExecutor,
) -> std::result::Result<HashSet<String>, ExecutorError> {
    let output = executor.run(&["diff", "--from", "trunk()", "--to", "@", "--summary"])?;
    Ok(parse_diff_summary(&output))
}

/// Get files modified in trunk since the merge base.
fn get_trunk_modified_files(
    executor: &dyn JjExecutor,
    merge_base: &str,
) -> std::result::Result<HashSet<String>, ExecutorError> {
    let output = executor.run(&["diff", "--from", merge_base, "--to", "trunk()", "--summary"])?;
    Ok(parse_diff_summary(&output))
}

/// Parse JJ diff --summary output to extract file paths.
///
/// Format: "M path/to/file" or "A path" or "D path" or "R old -> new"
pub fn parse_diff_summary(output: &str) -> HashSet<String> {
    output
        .lines()
        .filter_map(|line: &str| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }

            let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
            let status_opt = parts.first().copied();
            let file_part_opt = parts.get(1).copied();

            match (status_opt, file_part_opt) {
                (Some(status), Some(file_part)) if status == "R" && file_part.contains(" -> ") => {
                    file_part
                        .split(" -> ")
                        .last()
                        .map(std::string::ToString::to_string)
                }
                (Some(_), Some(file_part)) => Some(file_part.to_string()),
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ExecutorError Display ----

    #[test]
    fn executor_error_command_not_found_display() {
        let err = ExecutorError::CommandNotFound("jj not found".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("not found"));
        assert!(msg.contains("jj not found"));
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

    // ---- parse_diff_summary ----

    #[test]
    fn parse_diff_summary_empty_input() {
        let result = parse_diff_summary("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_diff_summary_whitespace_only() {
        let result = parse_diff_summary("   \n  \n");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_diff_summary_modified_files() {
        let input = "M src/main.rs\nM src/lib.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 2);
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("src/lib.rs"));
    }

    #[test]
    fn parse_diff_summary_added_files() {
        let input = "A new_file.rs\nA another.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 2);
        assert!(result.contains("new_file.rs"));
        assert!(result.contains("another.rs"));
    }

    #[test]
    fn parse_diff_summary_deleted_files() {
        let input = "D old_file.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains("old_file.rs"));
    }

    #[test]
    fn parse_diff_summary_renamed_files_extracts_new_name() {
        let input = "R old_name.rs -> new_name.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains("new_name.rs"));
        assert!(!result.contains("old_name.rs"));
    }

    #[test]
    fn parse_diff_summary_mixed_statuses() {
        let input = "M src/main.rs\nA src/new.rs\nD src/old.rs\nR old.rs -> renamed.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 4);
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("src/new.rs"));
        assert!(result.contains("src/old.rs"));
        assert!(result.contains("renamed.rs"));
    }

    #[test]
    fn parse_diff_summary_ignores_empty_lines() {
        let input = "M file.rs\n\n\nA other.rs\n";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn parse_diff_summary_handles_leading_trailing_whitespace() {
        let input = "  M file.rs  \n  A other.rs  ";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn parse_diff_summary_no_file_part_is_skipped() {
        // Line with just status letter, no file path
        let input = "M\nA file.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains("file.rs"));
    }

    // ---- Mock JjExecutor ----

    struct MockJjExecutor {
        responses: std::collections::HashMap<String, String>,
    }

    impl MockJjExecutor {
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

    impl JjExecutor for MockJjExecutor {
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
        let mock = MockJjExecutor::new().with_response("status", "ok");
        let result = mock.run(&["status"]).expect("should succeed");
        assert_eq!(result, "ok");
    }

    #[test]
    fn mock_executor_returns_error_for_unconfigured_command() {
        let mock = MockJjExecutor::new();
        let result = mock.run(&["unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn mock_executor_run_in_workspace_delegates_to_run() {
        let mock = MockJjExecutor::new().with_response("log", "log output");
        let result = mock
            .run_in_workspace(&["log"], "/some/workspace")
            .expect("should succeed");
        assert_eq!(result, "log output");
    }

    // ---- parse_diff_summary additional edge cases ----

    #[test]
    fn parse_diff_summary_path_with_spaces() {
        let input = "M path/with spaces/file.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains("path/with spaces/file.rs"));
    }

    #[test]
    fn parse_diff_summary_deeply_nested_paths() {
        let input = "M a/b/c/d/e/f/g/h.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains("a/b/c/d/e/f/g/h.rs"));
    }

    #[test]
    fn parse_diff_summary_multiple_renames() {
        let input = "R old_a.rs -> new_a.rs\nR old_b.rs -> new_b.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 2);
        assert!(result.contains("new_a.rs"));
        assert!(result.contains("new_b.rs"));
        assert!(!result.contains("old_a.rs"));
        assert!(!result.contains("old_b.rs"));
    }

    #[test]
    fn parse_diff_summary_line_without_status_prefix() {
        // Line that doesn't start with a recognized status letter
        // "garbage line" -> splitn gives status="garbage", file="line"
        // This is captured by the generic branch.
        let input = "garbage line\nM valid.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 2);
        assert!(result.contains("valid.rs"));
        assert!(result.contains("line"));
    }

    #[test]
    fn parse_diff_summary_copies_treated_as_files() {
        // 'C' (copy) is not a recognized special status, so it falls through
        // to the generic `(Some(_), Some(file_part))` branch.
        // splitn(2, ' ') on "C original.rs copy.rs" gives ["C", "original.rs copy.rs"]
        let input = "C original.rs copy.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains("original.rs copy.rs"));
    }

    #[test]
    fn parse_diff_summary_single_newline() {
        let result = parse_diff_summary("\n");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_diff_summary_tabs_in_path() {
        let input = "M path\twith\ttabs.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 1);
        // The tab is part of the file portion after the first space split
    }

    #[test]
    fn parse_diff_summary_deduplicates_same_path() {
        // If the same file appears twice (e.g., modified then modified again),
        // the HashSet deduplicates
        let input = "M file.rs\nM file.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn parse_diff_summary_rename_with_arrow_in_filename() {
        // Edge case: filename that itself contains " -> "
        let input = "R old -> name -> new.rs";
        let result = parse_diff_summary(input);
        assert_eq!(result.len(), 1);
        // Should extract the last part after " -> "
        assert!(result.contains("new.rs"));
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
        let mock = MockJjExecutor::new()
            .with_response(
                "log -r @ --no-graph -T if(conflict, \"CONFLICT\\n\", \"\")",
                "",
            )
            .with_response(
                "log -r heads(::@ & ::trunk()) --no-graph -T commit_id ++ \"\\n\" --limit 1",
                "abc123\n",
            )
            .with_response(
                "diff --from trunk() --to @ --summary",
                "M workspace_file.rs",
            )
            .with_response(
                "diff --from abc123 --to trunk() --summary",
                "M trunk_file.rs",
            );

        let result = detect_conflicts(&mock).expect("should succeed");
        assert!(result.merge_likely_safe);
        assert!(!result.has_existing_conflicts);
        assert!(result.overlapping_files.is_empty());
        assert_eq!(result.files_analyzed, 2);
        assert!(result.detection_time_ms > 0 || result.detection_time_ms == 0);
    }

    #[test]
    fn detect_conflicts_existing_conflicts_with_mock() {
        let mock = MockJjExecutor::new()
            .with_response(
                "log -r @ --no-graph -T if(conflict, \"CONFLICT\\n\", \"\")",
                "CONFLICT\n",
            )
            .with_response("resolve --list", "file_a.rs  normal\nfile_b.rs  text")
            .with_response(
                "log -r heads(::@ & ::trunk()) --no-graph -T commit_id ++ \"\\n\" --limit 1",
                "def456\n",
            )
            .with_response("diff --from trunk() --to @ --summary", "")
            .with_response("diff --from def456 --to trunk() --summary", "");

        let result = detect_conflicts(&mock).expect("should succeed");
        assert!(!result.merge_likely_safe);
        assert!(result.has_existing_conflicts);
        assert_eq!(result.existing_conflicts.len(), 2);
    }

    #[test]
    fn detect_conflicts_overlapping_files_with_mock() {
        let mock = MockJjExecutor::new()
            .with_response(
                "log -r @ --no-graph -T if(conflict, \"CONFLICT\\n\", \"\")",
                "",
            )
            .with_response(
                "log -r heads(::@ & ::trunk()) --no-graph -T commit_id ++ \"\\n\" --limit 1",
                "\n", // empty = no merge base
            )
            .with_response(
                "diff --from trunk() --to @ --summary",
                "M shared.rs\nM workspace_only.rs",
            )
            .with_response(
                "diff --from @ --to trunk() --summary",
                "M shared.rs\nM trunk_only.rs",
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
