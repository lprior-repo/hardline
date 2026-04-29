//! Merge workflow for the done command handler.
//!
//! Contains the dry-run preview and the full done workflow that
//! performs rebase, push, cleanup, and undo logging.

use scp_core::{output::Output, vcs, Error, Result};

use super::{
    conflict::run_conflict_detection_only,
    data::{DoneOptions, DoneOutput, DonePreview},
    executor::{detect_conflicts, GitExecutor},
    vcs_ops::{
        get_commits_to_merge, get_potential_conflicts, get_uncommitted_files, log_undo_history,
        update_workspace_state, WorkspaceGitExecutor,
    },
};

// ============================================================================
// Dry Run
// ============================================================================

/// Run a dry-run preview of the done command.
pub fn run_dry_run(
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
pub fn execute_done_workflow(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::done::{executor::ExecutorError, test_support::*};

    // ===================================================================
    // execute_done_workflow orchestration tests
    // ===================================================================

    #[test]
    fn test_execute_done_workflow_happy_path() {
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "feature-x".to_string(),
            branch: "feature-x".to_string(),
            is_current: false,
        }]);

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
        let backend = MockVcsBackend::new(vec![scp_core::Workspace {
            name: "detect-fail-ws".to_string(),
            branch: "detect-fail-ws".to_string(),
            is_current: false,
        }]);

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
    // run_dry_run orchestration tests
    // ===================================================================

    #[test]
    fn test_run_dry_run_returns_preview() {
        let executor = MockGitExecutor::new(vec![
            Ok("M src/modified.rs\nA src/added.rs\n".to_string()),
            Ok("change1\ncommit1\nfeat: add widget\n2024-01-15 10:00:00\n".to_string()),
            Ok(String::new()),
            Ok(String::new()),
            Ok(String::new()),
            Ok(String::new()),
            Ok(String::new()),
            Ok(String::new()),
            Ok(String::new()),
            Ok(String::new()),
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
            Ok(String::new()),
            Ok(String::new()),
            Ok(String::new()),
            Ok("mergebase123\n".to_string()),
            Ok("M shared.rs\nM unique.rs\n".to_string()),
            Ok("M shared.rs\n".to_string()),
            Ok(String::new()),
            Ok("mergebase123\n".to_string()),
            Ok("M shared.rs\nM unique.rs\n".to_string()),
            Ok("M shared.rs\n".to_string()),
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
        assert_eq!(preview.potential_conflicts.len(), 1);
        assert!(preview
            .potential_conflicts
            .contains(&"shared.rs".to_string()));
    }

    #[test]
    fn test_run_dry_run_without_conflict_detection() {
        let executor = MockGitExecutor::new(vec![
            // get_uncommitted_files: "status --no-pager"
            Ok("M src/lib.rs\n".to_string()),
            // get_commits_to_merge: "log -r @..@- ..."
            Ok(String::new()),
            // get_potential_conflicts -> detect_conflicts: 4 responses
            Ok(String::new()), // check_existing_conflicts
            Ok(String::new()), // find_merge_base
            Ok(String::new()), // workspace modified
            Ok(String::new()), // trunk diff fallback
        ]);

        let options = DoneOptions {
            dry_run: true,
            detect_conflicts: false,
            ..Default::default()
        };

        let result = run_dry_run("simple-ws", "/tmp/ws", &executor, &options);
        let output = result.expect("dry run without conflict detection should succeed");
        let preview = output.preview.expect("dry run should have preview");
        assert!(preview.conflict_detection.is_none());
    }
}
