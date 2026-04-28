//! Conflict detection for the done command handler.
//!
//! Provides the conflict-detection-only mode and the
//! best-effort conflict query used by the merge workflow.

use scp_core::{output::Output, Error, Result};

use super::{
    data::DoneOutput,
    executor::{detect_conflicts, GitExecutor},
    vcs_ops::WorkspaceGitExecutor,
};

/// Run conflict detection only and return results.
pub(crate) fn run_conflict_detection_only(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::done::test_support::*;

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
}
