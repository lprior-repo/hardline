//! Context command - shows current workspace/branch/location

use scp_core::{output::Output, vcs, Result};

/// Pure calculation: format the context lines for output
fn format_context_lines(
    workspace_name: &str,
    branch: &str,
    vcs_status: &vcs::VcsStatus,
) -> Vec<String> {
    vec![
        "Current Context:".to_string(),
        format!("  Workspace: {}", workspace_name),
        format!("  Branch: {}", branch),
        format!("  Status: {}", vcs_status),
    ]
}

/// Extract current workspace name from workspace list
fn extract_current_workspace_name(workspaces: Vec<vcs::Workspace>) -> String {
    workspaces
        .into_iter()
        .find(|w| w.is_current)
        .map(|w| w.name)
        .unwrap_or_else(|| "unknown".to_string())
}

/// Show current context (workspace, branch, VCS status)
pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().map_err(scp_core::Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    let workspace_name = extract_current_workspace_name(workspaces);

    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    let lines = format_context_lines(&workspace_name, &branch, &vcs_status);
    for line in &lines {
        Output::info(line);
    }

    Ok(())
}

/// Alias for run() - shows current context
pub fn whereami() -> Result<()> {
    run()
}
