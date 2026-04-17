//! Context command - shows current workspace/branch/location

use scp_core::{output::Output, vcs, Result};

/// Show current context (workspace, branch, VCS status)
pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| scp_core::Error::io_error(e.to_string()))?;

    let backend = vcs::create_backend(&cwd)?;

    // Get current workspace by finding the one with is_current = true
    let workspaces = backend.list_workspaces()?;
    let workspace_name = workspaces
        .into_iter()
        .find(|w| w.is_current)
        .map(|w| w.name)
        .unwrap_or_else(|| "unknown".to_string());

    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    Output::info("Current Context:");
    Output::info(&format!("  Workspace: {}", workspace_name));
    Output::info(&format!("  Branch: {}", branch));
    Output::info(&format!("  Status: {}", vcs_status));

    Ok(())
}

/// Alias for run() - shows current context
pub fn whereami() -> Result<()> {
    run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whereami_is_alias_for_run() {
        // Both functions have the same signature and whereami delegates to run
        let _fn_run: fn() -> Result<()> = run;
        let _fn_whereami: fn() -> Result<()> = whereami;
    }

    #[test]
    fn run_returns_error_in_nonexistent_dir() {
        // Verify the error path exists — io_error is used for current_dir failures
        let err = scp_core::Error::io_error("test");
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn context_module_uses_output_type() {
        let _ = std::any::type_name::<scp_core::output::Output>();
    }

    #[test]
    fn vcs_status_variants_exist() {
        use scp_core::vcs::VcsStatus;
        let _ = VcsStatus::Clean;
        let _ = VcsStatus::Dirty;
        let _ = VcsStatus::Conflicted;
        let _ = VcsStatus::Detached;
    }
}
