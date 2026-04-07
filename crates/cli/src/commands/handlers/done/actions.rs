//! Action functions for the done command handler (Tier 3).
//!
//! I/O operations that orchestrate the workspace completion workflow.
//! All validation is delegated to Tier 2 (calculations).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use scp_core::output::Output;
use scp_core::vcs;
use scp_core::{Error, Result};

use super::data::{
    CommitInfo, ConflictDetectionResult, DoneOptions, DoneOutput, DonePreview, UndoEntry,
};
use super::executor::{
    detect_conflicts, parse_diff_summary, ExecutorError, GitExecutor, RealGitExecutor,
};

// ============================================================================
// Public API
// ============================================================================

/// Execute the done command with the given options.
///
/// This is the main entry point. It validates the workspace state,
/// optionally detects conflicts, and performs the merge workflow.
///
/// # Errors
///
/// Returns errors for workspace validation failures, merge conflicts,
/// or VCS operation failures.
pub fn run_done(options: &DoneOptions) -> Result<DoneOutput> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;
    let executor = RealGitExecutor::new();

    // Phase 1: Validate and resolve workspace
    let workspace_name = resolve_workspace(backend.as_ref(), options.workspace.as_deref())?;

    // Ensure not main workspace
    if workspace_name == "main" {
        return Err(Error::invalid_state("cannot complete the main workspace"));
    }

    // Check workspace exists
    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == workspace_name) {
        return Err(Error::workspace_not_found(workspace_name.clone()));
    }

    // Determine workspace path (current dir if we're in it, or we need to switch)
    let workspace_path_buf = get_workspace_path(&cwd, &workspace_name, backend.as_ref())?;
    let workspace_path = workspace_path_buf
        .to_str()
        .ok_or_else(|| Error::internal("workspace path contains invalid UTF-8"))?;

    Output::info(&format!("Completing workspace '{}'...", workspace_name));

    // Handle detect_conflicts mode
    if options.detect_conflicts {
        return run_conflict_detection_only(&executor, &workspace_name, workspace_path);
    }

    // Handle dry-run
    if options.dry_run {
        return run_dry_run(&workspace_name, workspace_path, &executor, options);
    }

    // Phase 2: Perform the actual done workflow
    execute_done_workflow(
        &workspace_name,
        workspace_path,
        options,
        backend.as_ref(),
        &executor,
    )
}

// ============================================================================
// Conflict Detection
// ============================================================================

/// Run conflict detection only and return results.
fn run_conflict_detection_only(
    executor: &dyn GitExecutor,
    workspace_name: &str,
    workspace_path: &str,
) -> Result<DoneOutput> {
    let ws_executor = WorkspaceGitExecutor::new(executor, workspace_path);
    let result = detect_conflicts(&ws_executor).map_err(Error::from)?;

    // Display results
    println!("{}", result.summary);
    if !result.existing_conflicts.is_empty() {
        println!("\nExisting conflicts:");
        for file in &result.existing_conflicts {
            println!("  - {file}");
        }
    }
    if !result.overlapping_files.is_empty() {
        println!("\nPotential conflicts (files modified in both):");
        for file in &result.overlapping_files {
            println!("  - {file}");
        }
    }
    if !result.workspace_only.is_empty() {
        println!(
            "\nWorkspace-only changes ({} files):",
            result.workspace_only.len()
        );
        for file in result.workspace_only.iter().take(10) {
            println!("  - {file}");
        }
        if result.workspace_only.len() > 10 {
            println!("  ... and {} more", result.workspace_only.len() - 10);
        }
    }
    if result.merge_likely_safe {
        println!("\nMerge is likely safe");
    } else {
        println!("\nReview conflicts before merging");
    }

    if result.has_conflicts() {
        return Err(Error::vcs_conflict(
            workspace_name,
            "Merge conflicts detected",
        ));
    }

    Ok(DoneOutput {
        workspace_name: workspace_name.to_string(),
        dry_run: false,
        error: None,
        ..Default::default()
    })
}

// ============================================================================
// Dry Run
// ============================================================================

/// Run a dry-run preview of the done command.
fn run_dry_run(
    workspace_name: &str,
    workspace_path: &str,
    executor: &dyn GitExecutor,
    options: &DoneOptions,
) -> Result<DoneOutput> {
    let ws_executor = WorkspaceGitExecutor::new(executor, workspace_path);

    let uncommitted_files = get_uncommitted_files(&ws_executor)?;
    let commits_to_merge = get_commits_to_merge(&ws_executor)?;
    let potential_conflicts = get_potential_conflicts(&ws_executor);

    // Run detailed conflict detection if requested
    let conflict_detection = if options.detect_conflicts {
        Some(detect_conflicts(&ws_executor).map_err(Error::from)?)
    } else {
        None
    };

    let preview = DonePreview {
        uncommitted_files,
        commits_to_merge,
        potential_conflicts,
        bead_to_close: None,
        workspace_path: workspace_path.to_string(),
        conflict_detection,
    };

    // Display preview
    Output::info(&format!("Dry-run preview for workspace: {workspace_name}"));
    if !preview.uncommitted_files.is_empty() {
        println!("  Files to commit:");
        for file in &preview.uncommitted_files {
            println!("    - {file}");
        }
    }
    if !preview.commits_to_merge.is_empty() {
        println!("  Commits to merge: {}", preview.commits_to_merge.len());
    }
    if let Some(ref conflict_detection) = preview.conflict_detection {
        println!();
        println!("{}", conflict_detection.summary);
    }

    Ok(DoneOutput {
        workspace_name: workspace_name.to_string(),
        dry_run: true,
        preview: Some(preview),
        error: None,
        ..Default::default()
    })
}

// ============================================================================
// Main Workflow
// ============================================================================

/// Execute the full done workflow.
fn execute_done_workflow(
    workspace_name: &str,
    workspace_path: &str,
    options: &DoneOptions,
    backend: &dyn vcs::VcsBackend,
    executor: &dyn GitExecutor,
) -> Result<DoneOutput> {
    let ws_executor = WorkspaceGitExecutor::new(executor, workspace_path);

    // Step 1: Check for conflicts
    let conflicts = get_potential_conflicts(&ws_executor);
    if !conflicts.is_empty() {
        return Err(Error::vcs_conflict(
            workspace_name,
            format!("Merge conflicts detected: {}", conflicts.join(", ")),
        ));
    }

    // Step 2: Rebase onto main (merge workspace changes)
    backend.rebase("main")?;
    Output::success("Merged workspace to main");

    // Step 3: Push to remote
    let pushed_to_remote = if let Err(e) = backend.push() {
        Output::warn(&format!("Push failed (non-fatal): {e}"));
        false
    } else {
        Output::success("Pushed to remote");
        true
    };

    // Step 4: Delete workspace (unless keep_workspace is set)
    let cleaned = if options.keep_workspace {
        Output::info(&format!(
            "Workspace '{}' preserved (--keep-workspace)",
            workspace_name
        ));
        false
    } else {
        backend.delete_workspace(workspace_name)?;
        Output::success(&format!("Workspace '{}' cleaned up", workspace_name));
        true
    };

    // Step 5: Log undo history
    let _ = log_undo_history(workspace_name, &ws_executor, pushed_to_remote);

    // Step 6: Update workspace state to Merged (if applicable)
    let session_updated = update_workspace_state(workspace_name);

    let commits_merged = backend.log(100).map(|_| 0).unwrap_or(0);

    Ok(DoneOutput {
        workspace_name: workspace_name.to_string(),
        bead_id: None,
        files_committed: 0,
        commits_merged,
        merged: true,
        cleaned,
        bead_closed: false,
        session_updated,
        new_status: if session_updated {
            Some("merged".to_string())
        } else {
            None
        },
        pushed_to_remote,
        dry_run: false,
        preview: None,
        error: None,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Resolve workspace name from option or current workspace.
fn resolve_workspace(backend: &dyn vcs::VcsBackend, name: Option<&str>) -> Result<String> {
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
fn get_workspace_path(
    cwd: &Path,
    workspace_name: &str,
    backend: &dyn vcs::VcsBackend,
) -> Result<PathBuf> {
    let workspaces = backend.list_workspaces()?;
    let is_current = workspaces
        .iter()
        .any(|w| w.name == workspace_name && w.is_current);

    if is_current {
        Ok(cwd.to_path_buf())
    } else {
        // For non-current workspaces, the path is typically <repo>/.git/worktrees/<name>
        let workspace_path = cwd.join(".git").join("worktrees").join(workspace_name);
        if workspace_path.exists() {
            Ok(workspace_path)
        } else {
            Ok(cwd.to_path_buf())
        }
    }
}

/// Get list of uncommitted files via Git status.
fn get_uncommitted_files(executor: &dyn GitExecutor) -> Result<Vec<String>> {
    let output = executor
        .run(&["status", "--no-pager"])
        .map_err(Error::from)?;

    let files = output
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
        .collect();

    Ok(files)
}

/// Get commits that will be merged.
fn get_commits_to_merge(executor: &dyn GitExecutor) -> Result<Vec<CommitInfo>> {
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

    Ok(commits)
}

/// Get potential conflicts via conflict detection.
fn get_potential_conflicts(executor: &dyn GitExecutor) -> Vec<String> {
    match detect_conflicts(executor) {
        Ok(result) => {
            let mut conflicts = result.existing_conflicts;
            conflicts.extend(result.overlapping_files);
            conflicts
        }
        Err(e) => {
            // Log warning but don't fail - conflict detection is best-effort
            Output::warn(&format!("Conflict detection failed: {e}"));
            Vec::new()
        }
    }
}

/// Log undo history to .scp/undo.log.
fn log_undo_history(
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

    // Read existing content or start fresh
    let mut content = if undo_log_path.exists() {
        std::fs::read_to_string(&undo_log_path)
            .map_err(|e| Error::io_error(format!("Failed to read undo log: {e}")))?
    } else {
        String::new()
    };
    content.push_str(&json);
    content.push('\n');

    // Ensure parent directory exists
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
/// This is a placeholder for future session state management integration.
fn update_workspace_state(_workspace_name: &str) -> bool {
    // Future: integrate with session state management
    // For now, return false to indicate no update was performed.
    false
}

// ============================================================================
// Workspace Git Executor Wrapper
// ============================================================================

/// Executor that runs Git commands in a specific workspace directory.
struct WorkspaceGitExecutor<'a> {
    inner: &'a dyn GitExecutor,
    workspace_path: String,
}

impl<'a> WorkspaceGitExecutor<'a> {
    fn new(inner: &'a dyn GitExecutor, workspace_path: &str) -> Self {
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
    use crate::commands::handlers::done::data::DonePhase;

    #[test]
    fn test_parse_diff_summary_basic() {
        let output = "M src/lib.rs\nA src/new.rs\nD src/old.rs";
        let files = parse_diff_summary(output);
        assert!(files.contains("src/lib.rs"));
        assert!(files.contains("src/new.rs"));
        assert!(files.contains("src/old.rs"));
    }

    #[test]
    fn test_parse_diff_summary_with_rename() {
        let output = "R src/old_name.rs -> src/new_name.rs";
        let files = parse_diff_summary(output);
        assert!(files.contains("src/new_name.rs"));
    }

    #[test]
    fn test_parse_diff_summary_with_arrow_filename() {
        let output = "M a -> b.txt";
        let files = parse_diff_summary(output);
        // Should NOT treat " -> " as rename marker for non-R status
        assert!(files.contains("a -> b.txt"));
    }

    #[test]
    fn test_parse_diff_summary_empty() {
        let output = "";
        let files = parse_diff_summary(output);
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_diff_summary_with_whitespace() {
        let output = "\n  M src/lib.rs  \n\nA src/new.rs\n  \n";
        let files = parse_diff_summary(output);
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_conflict_detection_result_no_conflicts() {
        let result = ConflictDetectionResult::no_conflicts();
        assert!(!result.has_conflicts());
        assert!(result.merge_likely_safe);
    }

    #[test]
    fn test_conflict_detection_result_with_overlapping() {
        let result = ConflictDetectionResult {
            overlapping_files: vec!["shared.rs".to_string()],
            ..Default::default()
        };
        assert!(result.has_conflicts());
    }

    #[test]
    fn test_conflict_detection_result_with_existing() {
        let result = ConflictDetectionResult {
            has_existing_conflicts: true,
            existing_conflicts: vec!["conflicted.rs".to_string()],
            ..Default::default()
        };
        assert!(result.has_conflicts());
    }

    #[test]
    fn test_done_phase_names() {
        assert_eq!(DonePhase::ValidatingLocation.name(), "validating_location");
        assert_eq!(DonePhase::CommittingChanges.name(), "committing_changes");
        assert_eq!(DonePhase::MergingToMain.name(), "merging_to_main");
    }

    #[test]
    fn test_done_output_serialization() {
        let output = DoneOutput {
            workspace_name: "test-ws".to_string(),
            merged: true,
            cleaned: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("test-ws"));
        assert!(json_str.contains(r#""merged":true"#));
    }

    #[test]
    fn test_done_options_default() {
        let opts = DoneOptions::default();
        assert!(opts.workspace.is_none());
        assert!(opts.message.is_none());
        assert!(!opts.keep_workspace);
        assert!(!opts.squash);
        assert!(!opts.dry_run);
        assert!(!opts.detect_conflicts);
        assert!(!opts.no_bead_update);
    }

    // -----------------------------------------------------------------------
    // DoneOutput defaults
    // -----------------------------------------------------------------------

    #[test]
    fn test_done_output_default_fields() {
        let output = DoneOutput::default();
        assert!(output.workspace_name.is_empty());
        assert!(output.bead_id.is_none());
        assert_eq!(output.files_committed, 0);
        assert_eq!(output.commits_merged, 0);
        assert!(!output.merged);
        assert!(!output.cleaned);
        assert!(!output.bead_closed);
        assert!(!output.session_updated);
        assert!(output.new_status.is_none());
        assert!(!output.pushed_to_remote);
        assert!(!output.dry_run);
        assert!(output.preview.is_none());
        assert!(output.error.is_none());
    }

    #[test]
    fn test_done_output_roundtrip_serialization() {
        let output = DoneOutput {
            workspace_name: "feature-branch".to_string(),
            bead_id: Some("abc123".to_string()),
            files_committed: 5,
            commits_merged: 3,
            merged: true,
            cleaned: true,
            pushed_to_remote: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: DoneOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.workspace_name, "feature-branch");
        assert_eq!(deserialized.bead_id, Some("abc123".to_string()));
        assert_eq!(deserialized.files_committed, 5);
        assert_eq!(deserialized.commits_merged, 3);
        assert!(deserialized.merged);
        assert!(deserialized.cleaned);
        assert!(deserialized.pushed_to_remote);
    }

    // -----------------------------------------------------------------------
    // parse_diff_summary edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_diff_summary_only_whitespace() {
        let output = "   \n\t\n  \n";
        let files = parse_diff_summary(output);
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_diff_summary_garbage_lines() {
        // "not a valid line" still parses as status="not", file="a valid line"
        let output = "M src/lib.rs\nnot a valid line\nM src/other.rs\n??\nsrc/random.rs";
        let files = parse_diff_summary(output);
        assert_eq!(files.len(), 3);
        assert!(files.contains("src/lib.rs"));
        assert!(files.contains("src/other.rs"));
        assert!(files.contains("a valid line"));
    }

    #[test]
    fn test_parse_diff_summary_long_filename() {
        let long_name = "a".repeat(500);
        let output = format!("M {long_name}");
        let files = parse_diff_summary(&output);
        assert!(files.contains(&long_name));
    }

    #[test]
    fn test_parse_diff_summary_multiple_renames() {
        let output = "R old_a.rs -> new_a.rs\nR old_b.rs -> new_b.rs";
        let files = parse_diff_summary(output);
        assert!(files.contains("new_a.rs"));
        assert!(files.contains("new_b.rs"));
        assert!(!files.contains("old_a.rs"));
        assert!(!files.contains("old_b.rs"));
    }

    #[test]
    fn test_parse_diff_summary_file_with_spaces() {
        let output = "M path with spaces/file.txt";
        let files = parse_diff_summary(output);
        assert!(files.contains("path with spaces/file.txt"));
    }

    #[test]
    fn test_parse_diff_summary_duplicate_files() {
        // HashSet deduplicates
        let output = "M src/lib.rs\nM src/lib.rs\nA src/lib.rs";
        let files = parse_diff_summary(output);
        assert_eq!(files.len(), 1);
        assert!(files.contains("src/lib.rs"));
    }

    #[test]
    fn test_parse_diff_summary_rename_with_spaces() {
        let output = "R old name.txt -> new name.txt";
        let files = parse_diff_summary(output);
        assert!(files.contains("new name.txt"));
        assert!(!files.contains("old name.txt"));
    }

    // -----------------------------------------------------------------------
    // parse_uncommitted_files (extracted parsing logic for status output)
    // -----------------------------------------------------------------------

    /// Extract file names from `git status --porcelain` output lines.
    ///
    /// This mirrors the inline parsing in `get_uncommitted_files` so it can be
    /// tested in isolation without a real Git process.
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
        // Lines starting with '?' are untracked and should be skipped
        let output = "? untracked.rs\nM tracked.rs\n? another.untracked";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["tracked.rs"]);
    }

    #[test]
    fn test_parse_status_lines_file_with_spaces() {
        // split_ascii_whitespace splits on ALL whitespace, so files with spaces
        // cannot be correctly parsed -- nth(1) gives "path", not the full path.
        let output = "M path with spaces/file.txt";
        let files = parse_status_lines(output);
        // This limitation is documented: files with spaces in their name
        // are not parsed correctly by the current implementation.
        assert_eq!(files, vec!["path"]);
    }

    // -----------------------------------------------------------------------
    // parse_commits_output (extracted parsing logic for log output)
    // -----------------------------------------------------------------------

    /// Parse Git log output into `CommitInfo` entries.
    ///
    /// Each commit is 4 consecutive lines: change_id, commit_id, description,
    /// timestamp. This mirrors the inline parsing in `get_commits_to_merge`.
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
        // Truncated last entry (only change_id) should be included
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
        // The parser always consumes 4 lines per entry. Leading empty lines
        // shift the alignment: "" and "" become change_id/commit_id (skipped).
        // "abc123" and "def456" become description/timestamp of a skipped entry.
        // "desc" becomes change_id of the final entry -- non-empty, so included.
        let output = "\n\nabc123\ndef456\ndesc\nts\n";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "desc");
        assert_eq!(commits[0].commit_id, "ts");
    }

    #[test]
    fn test_parse_commits_output_multiline_description() {
        // Git templates may produce multi-line descriptions; the parser reads
        // in fixed groups of 4, so a 5-line description shifts alignment and
        // produces 2 entries instead of 1.
        let output = "abc123\ndef456\nfeat: add widget\nwith details\n2024-01-15 10:00:00\n";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 2);
        // First entry: change_id=abc123, commit_id=def456, desc=first line, ts=second line
        assert_eq!(commits[0].change_id, "abc123");
        assert_eq!(commits[0].description, "feat: add widget");
        assert_eq!(commits[0].timestamp, "with details");
        // Second entry: the timestamp string becomes change_id
        assert_eq!(commits[1].change_id, "2024-01-15 10:00:00");
    }

    // -----------------------------------------------------------------------
    // WorkspaceGitExecutor delegation
    // -----------------------------------------------------------------------

    /// A mock Git executor that records calls and returns canned responses.
    struct MockGitExecutor {
        responses: std::sync::Mutex<Vec<std::result::Result<String, ExecutorError>>>,
        calls: std::sync::Mutex<Vec<(String, Option<String>)>>,
    }

    impl MockGitExecutor {
        fn new(responses: Vec<std::result::Result<String, ExecutorError>>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses),
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn run_calls(&self) -> Vec<(String, Option<String>)> {
            self.calls.lock().expect("not poisoned").clone()
        }
    }

    impl GitExecutor for MockGitExecutor {
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

    #[test]
    fn test_workspace_executor_delegates_run() {
        let mock = MockGitExecutor::new(vec![Ok("result".to_string())]);
        let ws = WorkspaceGitExecutor::new(&mock, "/tmp/workspace");

        let result = ws.run(&["status", "--no-pager"]);
        assert_eq!(result.unwrap(), "result");

        let calls = mock.run_calls();
        assert_eq!(calls.len(), 1);
        // run() should delegate to run_in_workspace with the workspace path
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
        // run_in_workspace passes through to inner directly
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
    // DoneOptions completeness
    // -----------------------------------------------------------------------

    #[test]
    fn test_done_options_all_fields_writable() {
        let opts = DoneOptions {
            workspace: Some("my-ws".to_string()),
            message: Some("custom msg".to_string()),
            keep_workspace: true,
            squash: true,
            dry_run: true,
            detect_conflicts: true,
            no_bead_update: true,
        };
        assert_eq!(opts.workspace.as_deref(), Some("my-ws"));
        assert_eq!(opts.message.as_deref(), Some("custom msg"));
        assert!(opts.keep_workspace);
        assert!(opts.squash);
        assert!(opts.dry_run);
        assert!(opts.detect_conflicts);
        assert!(opts.no_bead_update);
    }

    // -----------------------------------------------------------------------
    // ConflictDetectionResult construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_conflict_detection_result_default_is_safe() {
        let result = ConflictDetectionResult::default();
        assert!(!result.has_conflicts());
        assert!(!result.has_existing_conflicts);
        assert!(result.existing_conflicts.is_empty());
        assert!(result.overlapping_files.is_empty());
        assert!(result.workspace_only.is_empty());
        assert!(result.main_only.is_empty());
        assert!(!result.merge_likely_safe); // default is false, not safe
        assert!(result.summary.is_empty());
        assert!(result.merge_base.is_none());
        assert_eq!(result.files_analyzed, 0);
        assert_eq!(result.detection_time_ms, 0);
    }

    #[test]
    fn test_conflict_detection_result_no_conflicts_sets_safe() {
        let result = ConflictDetectionResult::no_conflicts();
        assert!(result.merge_likely_safe);
        assert!(!result.has_conflicts());
        assert_eq!(result.summary, "No conflicts detected - merge is safe");
    }

    #[test]
    fn test_conflict_detection_result_workspace_and_main_only_no_conflict() {
        let result = ConflictDetectionResult {
            workspace_only: vec!["a.rs".to_string()],
            main_only: vec!["b.rs".to_string()],
            merge_likely_safe: true,
            summary: "No conflicts detected - merge is safe".to_string(),
            ..Default::default()
        };
        assert!(!result.has_conflicts());
    }

    // -----------------------------------------------------------------------
    // parse_diff_summary edge cases: binary / garbage filenames
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_diff_summary_binary_like_filename() {
        // Filenames containing null bytes or control characters cannot appear
        // in real diff output, but the parser should not panic on odd inputs.
        let output = "M \x00\x01binary_junk.dat";
        let files = parse_diff_summary(output);
        assert!(files.contains("\x00\x01binary_junk.dat"));
    }

    #[test]
    fn test_parse_diff_summary_filename_with_special_chars() {
        let output = "M src/fix#123!@$.rs";
        let files = parse_diff_summary(output);
        assert!(files.contains("src/fix#123!@$.rs"));
    }

    #[test]
    fn test_parse_diff_summary_unicode_filename() {
        let output = "M src/café.rs";
        let files = parse_diff_summary(output);
        assert!(files.contains("src/café.rs"));
    }

    #[test]
    fn test_parse_diff_summary_single_character_status() {
        // Status letters beyond M/A/D/R are treated as arbitrary single-char
        // status codes -- the parser still extracts the file part.
        let output = "X weird_status.rs\nY another.rs";
        let files = parse_diff_summary(output);
        assert_eq!(files.len(), 2);
        assert!(files.contains("weird_status.rs"));
        assert!(files.contains("another.rs"));
    }

    // -----------------------------------------------------------------------
    // parse_commits_output edge cases: single-line truncated input
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_commits_output_single_line() {
        // A single line input yields one commit with only a change_id.
        let output = "lone_change_id";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "lone_change_id");
        assert!(commits[0].commit_id.is_empty());
        assert!(commits[0].description.is_empty());
        assert!(commits[0].timestamp.is_empty());
    }

    #[test]
    fn test_parse_commits_output_two_lines() {
        // Two lines: change_id + commit_id, rest empty.
        let output = "ch_id\ncm_id";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "ch_id");
        assert_eq!(commits[0].commit_id, "cm_id");
        assert!(commits[0].description.is_empty());
        assert!(commits[0].timestamp.is_empty());
    }

    #[test]
    fn test_parse_commits_output_three_lines() {
        // Three lines: change_id + commit_id + description, timestamp empty.
        let output = "ch_id\ncm_id\ndesc line";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "ch_id");
        assert_eq!(commits[0].commit_id, "cm_id");
        assert_eq!(commits[0].description, "desc line");
        assert!(commits[0].timestamp.is_empty());
    }

    #[test]
    fn test_parse_commits_output_empty_change_id_is_skipped() {
        // Empty leading line: the parser consumes 4 lines per entry, and an
        // empty first line means change_id="" which is skipped.
        let output = "\nabc\ndef\nghi";
        let commits = parse_commits_output(output);
        // The first "entry" has change_id="" -> skipped.
        // The second group consumes lines that don't exist, yielding empty.
        assert!(commits.is_empty());
    }

    // -----------------------------------------------------------------------
    // ConflictDetectionResult with all fields populated simultaneously
    // -----------------------------------------------------------------------

    #[test]
    fn test_conflict_detection_result_all_fields_populated() {
        let result = ConflictDetectionResult {
            has_existing_conflicts: true,
            existing_conflicts: vec!["conflict_a.rs".to_string(), "conflict_b.rs".to_string()],
            overlapping_files: vec!["shared.rs".to_string()],
            workspace_only: vec!["ws_new.rs".to_string(), "ws_only2.rs".to_string()],
            main_only: vec!["main_new.rs".to_string()],
            merge_likely_safe: false,
            summary: "Existing conflicts in 2 files, plus 1 potential overlap".to_string(),
            merge_base: Some("abc123def456".to_string()),
            files_analyzed: 6,
            detection_time_ms: 150,
        };
        // Must report conflicts (both existing and overlapping).
        assert!(result.has_conflicts());
        assert!(result.has_existing_conflicts);
        assert_eq!(result.existing_conflicts.len(), 2);
        assert_eq!(result.overlapping_files.len(), 1);
        assert_eq!(result.workspace_only.len(), 2);
        assert_eq!(result.main_only.len(), 1);
        assert!(!result.merge_likely_safe);
        assert_eq!(result.files_analyzed, 6);
        assert_eq!(result.detection_time_ms, 150);
        assert_eq!(result.merge_base.as_deref(), Some("abc123def456"));

        // Verify serialization round-trip preserves all fields.
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: ConflictDetectionResult =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, result);
    }

    // -----------------------------------------------------------------------
    // parse_status_lines: various Git status prefixes (C for copied, etc.)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_status_lines_copied_prefix_ignored() {
        // "C " (copied) is not in the recognized set (A, M, D, R) and
        // should be filtered out by the parser.
        let output = "C src/copied.rs\nM src/modified.rs";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/modified.rs"]);
    }

    #[test]
    fn test_parse_status_lines_various_unrecognized_prefixes() {
        // Only A, M, D, R are recognized. Other prefixes are ignored.
        let output = "L src/locked.rs\nA src/added.rs\nT src/type_changed.rs\nD src/deleted.rs";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/added.rs", "src/deleted.rs"]);
    }

    #[test]
    fn test_parse_status_lines_lowercase_prefixes_ignored() {
        // Status prefixes are case-sensitive: 'm' != 'M'.
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
        // The filter checks starts_with("M ") (space), not starts_with("M\t")
        // (tab). Tab-separated status output is NOT recognized by the current
        // parser -- this is a known limitation.
        let output = "M\tsrc/tab_separated.rs\nA\tsrc/other.rs";
        let files = parse_status_lines(output);
        assert!(files.is_empty(), "tab-separated lines should not be parsed");
    }

    // ===================================================================
    // Mock VcsBackend
    // ===================================================================

    /// A mock VCS backend that records calls and returns canned results.
    ///
    /// All trait methods have sensible defaults; override individual fields
    /// to simulate specific scenarios (e.g. push failures, dirty status).
    struct MockVcsBackend {
        workspaces: Vec<scp_core::Workspace>,
        rebase_should_fail: bool,
        push_should_fail: bool,
        delete_workspace_should_fail: bool,
        log_entries: Vec<scp_core::Commit>,
    }

    impl MockVcsBackend {
        fn new(workspaces: Vec<scp_core::Workspace>) -> Self {
            Self {
                workspaces,
                rebase_should_fail: false,
                push_should_fail: false,
                delete_workspace_should_fail: false,
                log_entries: Vec::new(),
            }
        }

        fn with_rebase_failure(mut self) -> Self {
            self.rebase_should_fail = true;
            self
        }

        fn with_push_failure(mut self) -> Self {
            self.push_should_fail = true;
            self
        }

        fn with_delete_workspace_failure(mut self) -> Self {
            self.delete_workspace_should_fail = true;
            self
        }

        fn with_log_entries(mut self, entries: Vec<scp_core::Commit>) -> Self {
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

    /// Helper: build a list of mock Git responses that simulate a clean
    /// workspace with no conflicts.
    ///
    /// The detect_conflicts call sequence via WorkspaceGitExecutor.run()
    /// (which delegates to inner.run_in_workspace()) is:
    ///  1. check_existing_conflicts: "log -r @ --no-graph -T ..."
    ///  2. find_merge_base:           "log -r heads(::@ & ::trunk()) ..."
    ///  3. get_workspace_modified:    "diff --from trunk() --to @ --summary"
    ///  4. (no merge base) diff @..trunk: "diff --from @ --to trunk() --summary"
    fn no_conflict_responses() -> Vec<std::result::Result<String, ExecutorError>> {
        vec![
            // 1. check_existing_conflicts: no CONFLICT in output
            Ok(String::new()),
            // 2. find_merge_base: empty output means no merge base
            Ok(String::new()),
            // 3. get_workspace_modified_files: no files
            Ok(String::new()),
            // 4. fallback trunk diff (no merge base): no files
            Ok(String::new()),
        ]
    }

    /// Helper: build mock Git responses that simulate existing conflicts.
    ///
    /// detect_conflicts runs all its steps even after finding conflicts
    /// (it doesn't short-circuit), so we must provide the full sequence.
    fn existing_conflict_responses() -> Vec<std::result::Result<String, ExecutorError>> {
        vec![
            // 1. check_existing_conflicts: CONFLICT present
            Ok("CONFLICT\n".to_string()),
            // 2. resolve --list (called when CONFLICT found)
            Ok("src/conflicted.rs\n".to_string()),
            // 3. find_merge_base: empty (no merge base)
            Ok(String::new()),
            // 4. get_workspace_modified_files: some workspace changes
            Ok("M src/conflicted.rs\n".to_string()),
            // 5. trunk diff fallback (no merge base): some trunk changes
            Ok("M trunk_file.rs\n".to_string()),
        ]
    }

    /// Helper: build mock Git responses that simulate overlapping files
    /// (potential conflicts) but no existing conflicts.
    fn overlapping_conflict_responses() -> Vec<std::result::Result<String, ExecutorError>> {
        vec![
            // 1. check_existing_conflicts: no CONFLICT
            Ok(String::new()),
            // 2. find_merge_base: some merge base
            Ok("abc123merge\n".to_string()),
            // 3. get_workspace_modified_files: workspace modified "shared.rs"
            Ok("M shared.rs\n".to_string()),
            // 4. get_trunk_modified_files (has merge base): trunk also modified "shared.rs"
            Ok("M shared.rs\n".to_string()),
        ]
    }

    /// Helper: build mock Git responses for an empty diff (no changes to commit).
    fn empty_diff_responses() -> Vec<std::result::Result<String, ExecutorError>> {
        vec![
            // 1. check_existing_conflicts: no CONFLICT
            Ok(String::new()),
            // 2. find_merge_base: some merge base
            Ok("abc123\n".to_string()),
            // 3. get_workspace_modified_files: empty
            Ok(String::new()),
            // 4. get_trunk_modified_files (has merge base): some trunk changes
            Ok("M trunk_only.rs\n".to_string()),
        ]
    }

    /// Helper: build mock Git responses for a workspace with files modified
    /// but no conflicts.
    fn workspace_with_changes_responses() -> Vec<std::result::Result<String, ExecutorError>> {
        vec![
            // 1. check_existing_conflicts: no CONFLICT
            Ok(String::new()),
            // 2. find_merge_base: some merge base
            Ok("base123\n".to_string()),
            // 3. get_workspace_modified_files: workspace-only changes
            Ok("M src/new_feature.rs\nA src/new_file.rs\n".to_string()),
            // 4. get_trunk_modified_files (has merge base): different trunk changes
            Ok("M trunk_change.rs\n".to_string()),
        ]
    }

    // ===================================================================
    // execute_done_workflow orchestration tests
    // ===================================================================

    #[test]
    fn test_execute_done_workflow_happy_path() {
        // Happy path: clean workspace, no conflicts, successful merge + push + cleanup
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "feature-x".to_string(),
            branch: "feature-x".to_string(),
            is_current: false,
        }]);

        // detect_conflicts needs 4 responses (no conflicts), then log_undo_history
        // needs 1 response for "log -r @ --no-graph -T commit_id"
        let mut responses = no_conflict_responses();
        responses.push(Ok("commit-sha-abc\n".to_string()));

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions::default();

        let result = execute_done_workflow("feature-x", "/tmp/ws", &options, &backend, &executor);

        let output = result.expect("happy path should succeed");
        assert_eq!(output.workspace_name, "feature-x");
        assert!(output.merged, "should be marked as merged");
        assert!(
            output.cleaned,
            "should be marked as cleaned (default: no keep_workspace)"
        );
        assert!(output.pushed_to_remote, "push should succeed");
        assert!(!output.dry_run);
        assert!(output.preview.is_none());
        assert!(output.error.is_none());
    }

    #[test]
    fn test_execute_done_workflow_keep_workspace() {
        // With --keep_workspace, the workspace should NOT be cleaned up
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "feature-y".to_string(),
            branch: "feature-y".to_string(),
            is_current: false,
        }]);

        let mut responses = no_conflict_responses();
        responses.push(Ok("sha\n".to_string()));

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions {
            keep_workspace: true,
            ..Default::default()
        };

        let result = execute_done_workflow("feature-y", "/tmp/ws", &options, &backend, &executor);
        let output = result.expect("keep-workspace should succeed");
        assert_eq!(output.workspace_name, "feature-y");
        assert!(output.merged);
        assert!(
            !output.cleaned,
            "workspace should NOT be cleaned with --keep-workspace"
        );
        assert!(output.pushed_to_remote);
    }

    #[test]
    fn test_execute_done_workflow_push_failure_is_non_fatal() {
        // Push failure should NOT abort the workflow
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "push-fail-ws".to_string(),
            branch: "push-fail-ws".to_string(),
            is_current: false,
        }])
        .with_push_failure();

        let mut responses = no_conflict_responses();
        responses.push(Ok("sha\n".to_string()));

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions::default();

        let result =
            execute_done_workflow("push-fail-ws", "/tmp/ws", &options, &backend, &executor);
        let output = result.expect("push failure should not abort workflow");
        assert!(output.merged);
        assert!(!output.pushed_to_remote, "push should be marked as failed");
    }

    #[test]
    fn test_execute_done_workflow_rebase_failure() {
        // Rebase failure should abort the workflow with an error
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "rebase-fail-ws".to_string(),
            branch: "rebase-fail-ws".to_string(),
            is_current: false,
        }])
        .with_rebase_failure();

        let executor = MockGitExecutor::new(no_conflict_responses());
        let options = DoneOptions::default();

        let result =
            execute_done_workflow("rebase-fail-ws", "/tmp/ws", &options, &backend, &executor);
        assert!(result.is_err(), "rebase failure should propagate as error");
    }

    #[test]
    fn test_execute_done_workflow_existing_conflicts() {
        // Existing conflicts should cause the workflow to fail
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "conflict-ws".to_string(),
            branch: "conflict-ws".to_string(),
            is_current: false,
        }]);

        let executor = MockGitExecutor::new(existing_conflict_responses());
        let options = DoneOptions::default();

        let result = execute_done_workflow("conflict-ws", "/tmp/ws", &options, &backend, &executor);
        assert!(result.is_err(), "existing conflicts should return an error");
    }

    #[test]
    fn test_execute_done_workflow_overlapping_files_conflict() {
        // Overlapping files (modified in both workspace and trunk) should fail
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "overlap-ws".to_string(),
            branch: "overlap-ws".to_string(),
            is_current: false,
        }]);

        let executor = MockGitExecutor::new(overlapping_conflict_responses());
        let options = DoneOptions::default();

        let result = execute_done_workflow("overlap-ws", "/tmp/ws", &options, &backend, &executor);
        assert!(
            result.is_err(),
            "overlapping files should be detected as conflicts"
        );
    }

    #[test]
    fn test_execute_done_workflow_empty_diff_succeeds() {
        // Empty diff (no workspace changes) should still succeed if no conflicts
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "clean-ws".to_string(),
            branch: "clean-ws".to_string(),
            is_current: false,
        }]);

        let mut responses = empty_diff_responses();
        responses.push(Ok("sha\n".to_string()));

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions::default();

        let result = execute_done_workflow("clean-ws", "/tmp/ws", &options, &backend, &executor);
        let output = result.expect("empty diff should still succeed");
        assert!(output.merged);
        assert!(output.cleaned);
    }

    #[test]
    fn test_execute_done_workflow_workspace_with_changes() {
        // Workspace with changes but no conflicts should succeed
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "changes-ws".to_string(),
            branch: "changes-ws".to_string(),
            is_current: false,
        }]);

        let mut responses = workspace_with_changes_responses();
        responses.push(Ok("sha\n".to_string()));

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions::default();

        let result = execute_done_workflow("changes-ws", "/tmp/ws", &options, &backend, &executor);
        let output = result.expect("workspace with changes should succeed");
        assert!(output.merged);
        assert!(output.pushed_to_remote);
    }

    #[test]
    fn test_execute_done_workflow_delete_workspace_failure() {
        // Failure to delete workspace should propagate as error
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "del-fail-ws".to_string(),
            branch: "del-fail-ws".to_string(),
            is_current: false,
        }])
        .with_delete_workspace_failure();

        let mut responses = no_conflict_responses();
        responses.push(Ok("sha\n".to_string()));

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions::default();

        let result = execute_done_workflow("del-fail-ws", "/tmp/ws", &options, &backend, &executor);
        assert!(
            result.is_err(),
            "delete workspace failure should propagate as error"
        );
    }

    #[test]
    fn test_execute_done_workflow_conflict_detection_executor_error_returns_empty_conflicts() {
        // If conflict detection fails, get_potential_conflicts swallows the error
        // and returns an empty Vec, allowing the workflow to continue.
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "detect-fail-ws".to_string(),
            branch: "detect-fail-ws".to_string(),
            is_current: false,
        }]);

        // First call fails, which means detect_conflicts fails, which
        // get_potential_conflicts catches and returns Vec::new()
        let mut responses = vec![Err(ExecutorError::CommandFailed {
            code: 1,
            stderr: "git log failed".to_string(),
        })];
        responses.push(Ok("sha\n".to_string()));

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions::default();

        let result =
            execute_done_workflow("detect-fail-ws", "/tmp/ws", &options, &backend, &executor);
        let output =
            result.expect("conflict detection failure should not abort workflow (best-effort)");
        assert!(
            output.merged,
            "should still merge even if conflict detection failed"
        );
    }

    // ===================================================================
    // run_conflict_detection_only orchestration tests
    // ===================================================================

    #[test]
    fn test_run_conflict_detection_only_no_conflicts() {
        let executor = MockGitExecutor::new(no_conflict_responses());

        let result = run_conflict_detection_only(&executor, "safe-ws", "/tmp/ws");

        let output = result.expect("no conflicts should succeed");
        assert_eq!(output.workspace_name, "safe-ws");
        assert!(!output.dry_run);
        assert!(output.error.is_none());
    }

    #[test]
    fn test_run_conflict_detection_only_with_existing_conflicts() {
        let executor = MockGitExecutor::new(existing_conflict_responses());

        let result = run_conflict_detection_only(&executor, "conflict-ws", "/tmp/ws");

        assert!(
            result.is_err(),
            "existing conflicts should return error in detect-only mode"
        );
    }

    #[test]
    fn test_run_conflict_detection_only_with_overlapping_files() {
        let executor = MockGitExecutor::new(overlapping_conflict_responses());

        let result = run_conflict_detection_only(&executor, "overlap-ws", "/tmp/ws");

        assert!(
            result.is_err(),
            "overlapping files should return error in detect-only mode"
        );
    }

    // ===================================================================
    // run_dry_run orchestration tests
    // ===================================================================

    #[test]
    fn test_run_dry_run_returns_preview() {
        let executor = MockGitExecutor::new(vec![
            // get_uncommitted_files: "status --no-pager"
            Ok("M src/modified.rs\nA src/added.rs\n".to_string()),
            // get_commits_to_merge: "log -r @..@- ..."
            Ok("change1\ncommit1\nfeat: add widget\n2024-01-15 10:00:00\n".to_string()),
            // get_potential_conflicts -> detect_conflicts: 4 responses
            Ok(String::new()), // check_existing_conflicts
            Ok(String::new()), // find_merge_base
            Ok(String::new()), // workspace modified
            Ok(String::new()), // trunk diff fallback
            // options.detect_conflicts=true -> detect_conflicts called again: 4 more responses
            Ok(String::new()), // check_existing_conflicts
            Ok(String::new()), // find_merge_base
            Ok(String::new()), // workspace modified
            Ok(String::new()), // trunk diff fallback
        ]);

        let options = DoneOptions {
            dry_run: true,
            detect_conflicts: true,
            ..Default::default()
        };

        let result = run_dry_run("preview-ws", "/tmp/ws", &executor, &options);
        let output = result.expect("dry run should succeed");
        assert!(output.dry_run);
        assert_eq!(output.workspace_name, "preview-ws");

        let preview = output.preview.expect("dry run should have preview data");
        assert_eq!(preview.uncommitted_files.len(), 2);
        assert_eq!(preview.commits_to_merge.len(), 1);
        assert!(preview.potential_conflicts.is_empty());
        assert_eq!(preview.workspace_path, "/tmp/ws");
    }

    #[test]
    fn test_run_dry_run_with_conflicts_in_preview() {
        let executor = MockGitExecutor::new(vec![
            // get_uncommitted_files: empty
            Ok(String::new()),
            // get_commits_to_merge: empty
            Ok(String::new()),
            // get_potential_conflicts -> detect_conflicts with overlapping (4 responses)
            Ok(String::new()),                // check_existing_conflicts
            Ok("mergebase123\n".to_string()), // find_merge_base (has base)
            Ok("M shared.rs\nM unique.rs\n".to_string()), // workspace modified
            Ok("M shared.rs\n".to_string()),  // trunk modified (overlap)
            // options.detect_conflicts=true -> detect_conflicts called again: 4 more responses
            Ok(String::new()),                // check_existing_conflicts
            Ok("mergebase123\n".to_string()), // find_merge_base (has base)
            Ok("M shared.rs\nM unique.rs\n".to_string()), // workspace modified
            Ok("M shared.rs\n".to_string()),  // trunk modified (overlap)
        ]);

        let options = DoneOptions {
            dry_run: true,
            detect_conflicts: true,
            ..Default::default()
        };

        let result = run_dry_run("conflict-preview-ws", "/tmp/ws", &executor, &options);
        let output = result.expect("dry run with conflicts should still succeed (preview only)");
        assert!(output.dry_run);

        let preview = output.preview.expect("dry run should have preview");
        // Overlapping files: "shared.rs" appears in both workspace and trunk
        assert_eq!(preview.potential_conflicts.len(), 1);
        assert!(preview
            .potential_conflicts
            .contains(&"shared.rs".to_string()));
    }

    #[test]
    fn test_run_dry_run_without_conflict_detection() {
        let executor = MockGitExecutor::new(vec![
            // get_uncommitted_files
            Ok("M src/lib.rs\n".to_string()),
            // get_commits_to_merge
            Ok(String::new()),
            // get_potential_conflicts -> detect_conflicts: 4 responses
            Ok(String::new()),
            Ok(String::new()),
            Ok(String::new()),
            Ok(String::new()),
            // detect_conflicts is NOT called again since detect_conflicts=false
        ]);

        let options = DoneOptions {
            dry_run: true,
            detect_conflicts: false,
            ..Default::default()
        };

        let result = run_dry_run("simple-ws", "/tmp/ws", &executor, &options);
        let output = result.expect("dry run without conflict detection should succeed");
        let preview = output.preview.expect("dry run should have preview");
        // conflict_detection field should be None when detect_conflicts is false
        assert!(preview.conflict_detection.is_none());
    }

    // ===================================================================
    // get_uncommitted_files via mock executor
    // ===================================================================

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

    // ===================================================================
    // get_commits_to_merge via mock executor
    // ===================================================================

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

    // ===================================================================
    // QA Functional Verification Tests (hq-ftou)
    //
    // Scenarios verified:
    //   1. Workspace completion (happy path, keep-workspace, empty diff)
    //   2. Merge to main (rebase success/failure, push success/non-fatal-failure)
    //   3. Cleanup of worktree (default delete, keep-workspace, delete failure)
    //   4. Conflict handling (existing, overlapping, detection failure)
    //   5. Error cases (resolve_workspace, get_workspace_path, dirty tree)
    // ===================================================================

    // -----------------------------------------------------------------------
    // resolve_workspace: resolve workspace name from option or current
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_workspace_explicit_name_returns_name() {
        let backend = MockVcsBackend::new(vec![]);
        let result = resolve_workspace(&backend, Some("my-feature"));
        assert_eq!(result.expect("explicit name should resolve"), "my-feature");
    }

    #[test]
    fn test_resolve_workspace_none_with_current_workspace() {
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "feature-a".to_string(),
            branch: "feature-a".to_string(),
            is_current: true,
        }]);
        let result = resolve_workspace(&backend, None);
        assert_eq!(
            result.expect("should find current workspace"),
            "feature-a"
        );
    }

    #[test]
    fn test_resolve_workspace_none_without_current_workspace() {
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "feature-a".to_string(),
            branch: "feature-a".to_string(),
            is_current: false,
        }]);
        let result = resolve_workspace(&backend, None);
        assert!(
            result.is_err(),
            "should fail when no current workspace and no explicit name"
        );
    }

    #[test]
    fn test_resolve_workspace_none_with_empty_workspaces() {
        let backend = MockVcsBackend::new(vec![]);
        let result = resolve_workspace(&backend, None);
        assert!(
            result.is_err(),
            "should fail when workspace list is empty"
        );
    }

    #[test]
    fn test_resolve_workspace_none_with_multiple_workspaces_one_current() {
        let backend = MockVcsBackend::new(vec![
            scp_core::Workspace {
                name: "ws-1".to_string(),
                branch: "ws-1".to_string(),
                is_current: false,
            },
            scp_core::Workspace {
                name: "ws-2".to_string(),
                branch: "ws-2".to_string(),
                is_current: true,
            },
            scp_core::Workspace {
                name: "ws-3".to_string(),
                branch: "ws-3".to_string(),
                is_current: false,
            },
        ]);
        let result = resolve_workspace(&backend, None);
        assert_eq!(result.expect("should find current"), "ws-2");
    }

    // -----------------------------------------------------------------------
    // get_workspace_path: resolve filesystem path for a workspace
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_workspace_path_current_workspace_returns_cwd() {
        let cwd = PathBuf::from("/home/user/project");
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "feature-x".to_string(),
            branch: "feature-x".to_string(),
            is_current: true,
        }]);
        let result =
            get_workspace_path(&cwd, "feature-x", &backend).expect("should resolve path");
        assert_eq!(result, cwd, "current workspace should return cwd");
    }

    #[test]
    fn test_get_workspace_path_non_current_workspace_falls_back_to_cwd() {
        let cwd = PathBuf::from("/home/user/project");
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "other-ws".to_string(),
            branch: "other-ws".to_string(),
            is_current: false,
        }]);
        let result =
            get_workspace_path(&cwd, "other-ws", &backend).expect("should resolve path");
        // The worktree path doesn't exist on this machine, so it falls back to cwd
        assert_eq!(result, cwd, "non-current workspace without worktree dir falls back to cwd");
    }

    #[test]
    fn test_get_workspace_path_workspace_not_in_list_still_resolves() {
        let cwd = PathBuf::from("/home/user/project");
        let backend = MockVcsBackend::new(vec![]);
        let result =
            get_workspace_path(&cwd, "ghost-ws", &backend).expect("should resolve path");
        // Workspace not found in list -> is_current=false, worktree path likely
        // doesn't exist -> falls back to cwd
        assert_eq!(result, cwd);
    }

    // -----------------------------------------------------------------------
    // QA Finding: execute_done_workflow does NOT check for dirty tree
    //
    // The workflow proceeds directly to conflict detection and merging
    // without verifying the working tree is clean. This means:
    // - Uncommitted changes could be lost during rebase
    // - The merge could fail with a dirty tree error from the VCS
    //
    // This test documents the current behavior: the workflow succeeds
    // even though it should arguably check for uncommitted changes first.
    // -----------------------------------------------------------------------

    #[test]
    fn test_execute_done_workflow_no_explicit_dirty_tree_check() {
        // QA FINDING: execute_done_workflow does not check for dirty tree
        // before merging. It proceeds directly to conflict detection.
        //
        // To simulate a "dirty tree" scenario, we would need get_potential_conflicts
        // to detect uncommitted files via status -- but it only runs detect_conflicts,
        // which checks for merge conflicts, not uncommitted changes.
        //
        // The workflow will happily try to rebase a dirty tree, relying on the
        // VCS backend to reject the rebase if the tree is dirty. This is a
        // correctness gap: data loss could occur if the rebase succeeds despite
        // dirty state.
        //
        // Verdict: Documented as a known gap. Not a test failure since the
        // behavior is "works as coded" but may be undesirable.

        // Proof: execute_done_workflow with no conflicts succeeds regardless
        // of dirty tree state (it never checks)
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "dirty-ws".to_string(),
            branch: "dirty-ws".to_string(),
            is_current: false,
        }]);

        let mut responses = no_conflict_responses();
        responses.push(Ok("sha\n".to_string()));

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions::default();

        let result =
            execute_done_workflow("dirty-ws", "/tmp/ws", &options, &backend, &executor);

        // The workflow succeeds without checking dirty state
        let output = result.expect("workflow proceeds without dirty tree check");
        assert!(output.merged, "merge proceeds without dirty tree validation");
    }

    // -----------------------------------------------------------------------
    // execute_done_workflow: undo history logging
    // -----------------------------------------------------------------------

    #[test]
    fn test_execute_done_workflow_logs_undo_history() {
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "undo-test-ws".to_string(),
            branch: "undo-test-ws".to_string(),
            is_current: false,
        }]);

        let mut responses = no_conflict_responses();
        responses.push(Ok("pre-merge-sha123\n".to_string()));

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions::default();

        let result = execute_done_workflow(
            "undo-test-ws",
            "/tmp/ws",
            &options,
            &backend,
            &executor,
        );

        let output = result.expect("undo history logging should not fail the workflow");
        assert!(output.merged);
    }

    // -----------------------------------------------------------------------
    // execute_done_workflow: session state update (stub)
    // -----------------------------------------------------------------------

    #[test]
    fn test_execute_done_workflow_session_state_is_not_updated() {
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "session-ws".to_string(),
            branch: "session-ws".to_string(),
            is_current: false,
        }]);

        let mut responses = no_conflict_responses();
        responses.push(Ok("sha\n".to_string()));

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions::default();

        let result = execute_done_workflow(
            "session-ws",
            "/tmp/ws",
            &options,
            &backend,
            &executor,
        );

        let output = result.expect("should succeed");
        assert!(
            !output.session_updated,
            "session state should NOT be updated (stub returns false)"
        );
        assert!(
            output.new_status.is_none(),
            "new_status should be None when session not updated"
        );
    }

    // -----------------------------------------------------------------------
    // execute_done_workflow: conflict detection best-effort on failure
    // -----------------------------------------------------------------------

    #[test]
    fn test_execute_done_workflow_conflict_detect_failure_allows_merge() {
        // When conflict detection itself fails (e.g., git command error),
        // get_potential_conflicts catches the error and returns empty Vec.
        // This means the workflow proceeds with NO conflict protection.
        //
        // QA NOTE: This is a design choice - conflict detection is best-effort.
        // A failure in detection does NOT block the merge, which could lead
        // to undetected conflicts during rebase.
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "detect-err-ws".to_string(),
            branch: "detect-err-ws".to_string(),
            is_current: false,
        }]);

        // First call (detect_conflicts) fails, second call (undo log) succeeds
        let responses = vec![
            Err(ExecutorError::CommandFailed {
                code: 128,
                stderr: "fatal: bad revision 'trunk()'".to_string(),
            }),
            Ok("sha\n".to_string()),
        ];

        let executor = MockGitExecutor::new(responses);
        let options = DoneOptions::default();

        let result = execute_done_workflow(
            "detect-err-ws",
            "/tmp/ws",
            &options,
            &backend,
            &executor,
        );

        let output = result.expect("conflict detection failure should not block merge");
        assert!(
            output.merged,
            "merge should proceed despite failed conflict detection"
        );
    }

    // -----------------------------------------------------------------------
    // update_workspace_state: returns false (stub verification)
    // -----------------------------------------------------------------------

    #[test]
    fn test_update_workspace_state_returns_false() {
        // Stub always returns false - no session integration yet
        assert!(!update_workspace_state("any-ws"));
        assert!(!update_workspace_state("main"));
        assert!(!update_workspace_state(""));
    }

    // -----------------------------------------------------------------------
    // parse_status_lines: additional edge cases for dirty tree detection
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_status_lines_mixed_case_not_recognized() {
        // Only uppercase prefixes (A , M , D , R ) are recognized
        let output = "m src/lower.rs\na src/lower_add.rs\nd src/lower_del.rs\nr src/lower_ren.rs";
        let files = parse_status_lines(output);
        assert!(
            files.is_empty(),
            "lowercase prefixes should not be recognized as status codes"
        );
    }

    #[test]
    fn test_parse_status_lines_double_prefix_status() {
        // Git porcelain format uses two columns: XY where X=index, Y=working tree
        // Our parser only matches single-letter+space prefixes, so "MM" won't match
        let output = "MM src/both-staged-and unstaged.rs\nM src/simple.rs";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/simple.rs"]);
    }

    #[test]
    fn test_parse_status_lines_conflict_markers_not_recognized() {
        // Conflict statuses like "UU", "AA", "DU" are not recognized by
        // the current parser. This is relevant to the dirty tree QA finding.
        let output = "UU src/conflicted.rs\nAA src/both-added.rs\nDU src/deleted-by-us.rs";
        let files = parse_status_lines(output);
        assert!(
            files.is_empty(),
            "conflict status markers should not be parsed by current implementation"
        );
    }

    // -----------------------------------------------------------------------
    // DoneOutput: error field usage
    // -----------------------------------------------------------------------

    #[test]
    fn test_done_output_error_field_not_set_on_success() {
        let output = DoneOutput {
            workspace_name: "success-ws".to_string(),
            merged: true,
            cleaned: true,
            ..Default::default()
        };
        assert!(output.error.is_none(), "no error on successful completion");
    }

    #[test]
    fn test_done_output_error_field_preserves_message() {
        let output = DoneOutput {
            workspace_name: "err-ws".to_string(),
            error: Some("workspace not found".to_string()),
            ..Default::default()
        };
        assert_eq!(output.error.as_deref(), Some("workspace not found"));
        assert!(!output.merged, "should not be merged when error present");
    }

    // -----------------------------------------------------------------------
    // DonePhase: exhaustive coverage of all phases
    // -----------------------------------------------------------------------

    #[test]
    fn test_done_phase_ordering() {
        // Phases should proceed in order: ValidatingLocation -> CommittingChanges -> MergingToMain
        let phases = [
            DonePhase::ValidatingLocation,
            DonePhase::CommittingChanges,
            DonePhase::MergingToMain,
        ];
        let names: Vec<&str> = phases.iter().map(|p| p.name()).collect();
        assert_eq!(
            names,
            ["validating_location", "committing_changes", "merging_to_main"]
        );
    }

    #[test]
    fn test_done_phase_copy_trait() {
        let phase = DonePhase::ValidatingLocation;
        let copied = phase;
        assert_eq!(phase, copied);
    }

    #[test]
    fn test_done_phase_clone_trait() {
        let phase = DonePhase::MergingToMain;
        let cloned = phase.clone();
        assert_eq!(phase, cloned);
    }
}
