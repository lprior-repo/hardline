//! Action functions for the done command handler (Tier 3).
//!
//! I/O operations that orchestrate the workspace completion workflow.
//! All validation is delegated to Tier 2 (calculations).

use scp_core::{vcs, Error, Result};

use super::{
    conflict::run_conflict_detection_only,
    data::{DoneOptions, DoneOutput},
    executor::RealGitExecutor,
    merge::{execute_done_workflow, run_dry_run},
    vcs_ops::{get_workspace_path, resolve_workspace},
};

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
        return Err(Error::workspace_not_found(workspace_name));
    }

    // Determine workspace path (current dir if we're in it, or we need to switch)
    let workspace_path_buf = get_workspace_path(&cwd, &workspace_name, backend.as_ref())?;
    let workspace_path = workspace_path_buf
        .to_str()
        .ok_or_else(|| Error::internal("workspace path contains invalid UTF-8"))?;

    scp_core::output::Output::info(&format!("Completing workspace '{}'...", workspace_name));

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
