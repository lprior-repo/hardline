//! Workspace operations - pure helper functions

use std::path::Path;
use std::process::Command;

use scp_core::output::Output;
use scp_core::vcs::{self, VcsBackend, VcsStatus};
use scp_core::Error;

/// Get sorted workspace names
#[must_use]
pub fn sorted_workspace_names(workspaces: &[vcs::Workspace]) -> Vec<String> {
    let mut names: Vec<String> = workspaces.iter().map(|w| w.name.clone()).collect();
    names.sort();
    names
}

/// Find next workspace in alphabetical order
#[must_use]
pub fn find_next_workspace(workspaces: &[vcs::Workspace]) -> Result<String, Error> {
    let sorted_names = sorted_workspace_names(workspaces);
    let current_ws = workspaces.iter().find(|w| w.is_current);

    match current_ws {
        Some(current) => {
            let current_idx = sorted_names
                .iter()
                .position(|n| n == &current.name)
                .ok_or_else(|| Error::internal("current workspace not in list"))?;
            let next_idx = (current_idx + 1) % sorted_names.len();
            Ok(sorted_names[next_idx].clone())
        }
        None => sorted_names
            .first()
            .cloned()
            .ok_or_else(|| Error::workspace_not_found("no workspaces exist")),
    }
}

/// Find previous workspace in alphabetical order
#[must_use]
pub fn find_prev_workspace(workspaces: &[vcs::Workspace]) -> Result<String, Error> {
    let sorted_names = sorted_workspace_names(workspaces);
    let current_ws = workspaces.iter().find(|w| w.is_current);

    match current_ws {
        Some(current) => {
            let current_idx = sorted_names
                .iter()
                .position(|n| n == &current.name)
                .ok_or_else(|| Error::internal("current workspace not in list"))?;
            let prev_idx = if current_idx == 0 {
                sorted_names.len() - 1
            } else {
                current_idx - 1
            };
            Ok(sorted_names[prev_idx].clone())
        }
        None => sorted_names
            .last()
            .cloned()
            .ok_or_else(|| Error::workspace_not_found("no workspaces exist")),
    }
}

/// Helper: Create workspace with optional sync
pub fn spawn_with_sync(backend: &dyn VcsBackend, name: &str, sync: bool) -> Result<(), Error> {
    backend.create_workspace(name)?;
    Output::success(&format!("Created workspace '{}'", name));

    if sync {
        backend.switch_workspace(name)?;
        backend.rebase("main")?;
        Output::success("Synced with main");
    }

    Ok(())
}

/// Helper: Check workspace exists
#[must_use]
pub fn workspace_exists(backend: &dyn VcsBackend, name: &str) -> Result<bool, Error> {
    let workspaces = backend.list_workspaces()?;
    Ok(workspaces.iter().any(|w| w.name == name))
}

/// Helper: Validate clean working copy
#[must_use]
pub fn require_clean_working_copy(backend: &dyn VcsBackend) -> Result<(), Error> {
    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::working_copy_dirty());
    }
    Ok(())
}

/// Helper to get current workspace name from backend
#[must_use]
pub fn get_current_workspace_name(backend: &dyn VcsBackend) -> Result<String, Error> {
    let workspaces = backend.list_workspaces()?;
    workspaces
        .iter()
        .find(|w| w.is_current)
        .map(|w| w.name.clone())
        .ok_or_else(|| Error::workspace_not_found("no current workspace"))
}

/// Helper: Resolve workspace name from Option or get current
#[must_use]
pub fn resolve_workspace_name(
    backend: &dyn VcsBackend,
    name: Option<&str>,
) -> Result<String, Error> {
    match name {
        Some(n) => Ok(n.to_string()),
        None => get_current_workspace_name(backend),
    }
}

/// Helper: Complete workspace workflow (sync + push)
pub fn complete_workspace_workflow(backend: &dyn VcsBackend, name: &str) -> Result<(), Error> {
    backend.rebase("main")?;
    Output::success("Synced with main");

    backend.push()?;
    Output::success("Pushed to remote");

    Output::success(&format!("Workspace '{}' completed", name));
    Ok(())
}

/// Ensure workspace is not main
#[must_use]
pub fn ensure_not_main_workspace(name: &str) -> Result<(), Error> {
    if name == "main" {
        return Err(Error::invalid_state("Cannot complete main workspace"));
    }
    Ok(())
}

/// Execute workspace abort workflow
pub fn execute_workspace_abort(backend: &dyn VcsBackend, name: &str) -> Result<(), Error> {
    backend.abort_workspace(name)?;
    Output::success(&format!("Aborted workspace '{}'", name));
    Ok(())
}

/// Build jj diff command
#[must_use]
pub fn build_jj_diff_command(cwd: &Path, path: Option<&str>) -> Command {
    let mut cmd = Command::new("jj");
    cmd.args(["diff", "--at-op", "working", "--rev", "@"])
        .current_dir(cwd);

    if let Some(p) = path {
        cmd.arg(p);
    }

    cmd
}

/// Split workspace by creating a new branch from current state
pub fn split_workspace(backend: &dyn VcsBackend, path: &str) -> Result<(), Error> {
    let workspace_path = Path::new(path);

    if !workspace_path.exists() {
        return Err(Error::not_found(format!("Path does not exist: {}", path)));
    }

    if !workspace_path.is_dir() {
        return Err(Error::invalid_state(format!(
            "Path is not a directory: {}",
            path
        )));
    }

    let workspaces = backend.list_workspaces()?;
    let path_str = workspace_path.to_string_lossy().to_string();

    for ws in workspaces {
        if ws.name == path_str || ws.branch == path_str {
            return Err(Error::workspace_exists(ws.name));
        }
    }

    Output::info(&format!("Adding workspace at '{}'...", path));

    let output = Command::new("jj")
        .args(["workspace", "add", path])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("workspace add", stderr));
    }

    Output::success(&format!("✓ Added workspace at '{}'", path));

    Ok(())
}
