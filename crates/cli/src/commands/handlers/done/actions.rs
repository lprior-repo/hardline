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

    // Ensure not main workspace. An empty resolved name means "the
    // worktree I stand in": that is the main workspace exactly when the
    // current branch is main (a branch can only be checked out once, so a
    // linked worktree can never be on main while main is checked out).
    // Without this, `done` in the parent repo "completes" the main
    // workspace and then fails in the delete step.
    if workspace_name == "main"
        || (workspace_name.is_empty()
            && backend.current_branch().is_ok_and(|b| b == "main"))
    {
        return Err(Error::invalid_state("cannot complete the main workspace"));
    }

    // Check workspace exists. A work-created session lists under its
    // path key (`.scp/workspaces/<name>`) while the user types the
    // short name, so canonicalize a short name to the listed key first
    // (the same match session focus/remove use); the path resolution
    // below then finds the real worktree instead of falling back to
    // the caller's directory. An unmatched name is kept as-is, so the
    // check below still fails cleanly for unknown workspaces.
    let workspaces = backend.list_workspaces()?;
    let workspace_name = match workspaces
        .iter()
        .find(|w| vcs::workspace_name_matches(&w.name, &workspace_name))
    {
        Some(w) => w.name.clone(),
        None => workspace_name,
    };
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
