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

/// Pure calculation: extract current workspace name from workspace list
fn extract_current_workspace_name(workspaces: &[vcs::Workspace]) -> String {
    workspaces
        .iter()
        .find(|w| w.is_current)
        .map(|w| w.name.as_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Pure calculation: gather all context data from backend
fn gather_context_data(
    backend: &dyn vcs::VcsBackend,
    workspaces: &[vcs::Workspace],
) -> Result<(String, String, vcs::VcsStatus)> {
    let workspace_name = extract_current_workspace_name(workspaces);
    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;
    Ok((workspace_name, branch, vcs_status))
}

/// Action: output context lines
fn output_context_lines(lines: &[String]) {
    lines.iter().for_each(|line| Output::info(line));
}

/// Show current context (workspace, branch, VCS status)
pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().map_err(scp_core::Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    let (workspace_name, branch, vcs_status) = gather_context_data(&*backend, &workspaces)?;
    let lines = format_context_lines(&workspace_name, &branch, &vcs_status);
    output_context_lines(&lines);

    Ok(())
}

/// Alias for run() - shows current context
pub fn whereami() -> Result<()> {
    run()
}
