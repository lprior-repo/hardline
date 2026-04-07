//! Workspace completion commands

use scp_core::output::Output;
use scp_core::vcs;
use scp_core::Error;

use super::operations::*;
use super::validators::validate_workspace_name;

/// Complete workspace and merge
pub fn done(name: Option<&str>) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    // P4: Check for dirty working copy BEFORE any operations
    require_clean_working_copy(backend.as_ref())?;

    // Resolve workspace name: if None, get current workspace
    let workspace_name = resolve_workspace_name(backend.as_ref(), name)?;

    // P1: Validate workspace name format (prevents path traversal)
    if let Some(err) = validate_workspace_name(&workspace_name) {
        return Err(err);
    }

    // P3: Check workspace exists
    if !workspace_exists(backend.as_ref(), &workspace_name)? {
        return Err(Error::workspace_not_found(workspace_name.clone()));
    }

    Output::info(&format!("Completing workspace '{}'...", workspace_name));

    // P2: Ensure not main workspace
    ensure_not_main_workspace(&workspace_name)?;

    complete_workspace_workflow(backend.as_ref(), &workspace_name)?;

    Ok(())
}

/// Abort workspace
pub fn abort(name: Option<&str>) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    // P4: Check for dirty working copy BEFORE any operations
    require_clean_working_copy(backend.as_ref())?;

    // Resolve workspace name: if None, get current workspace
    let workspace_name = resolve_workspace_name(backend.as_ref(), name)?;

    // P1: Validate workspace name format (prevents path traversal)
    if let Some(err) = validate_workspace_name(&workspace_name) {
        return Err(err);
    }

    // P3: Check workspace exists
    if !workspace_exists(backend.as_ref(), &workspace_name)? {
        return Err(Error::workspace_not_found(workspace_name.clone()));
    }

    // P2: Ensure not main workspace
    ensure_not_main_workspace(&workspace_name)?;

    Output::info(&format!("Aborting workspace '{}'...", workspace_name));

    execute_workspace_abort(backend.as_ref(), &workspace_name)?;

    Ok(())
}
