//! Workspace helper functions

use std::process::Command;

use scp_core::{
    output::Output,
    vcs::{self, VcsStatus},
    Error, Result,
};

use super::workspace_types::validate_workspace_name;

/// Get sorted workspace names
pub fn sorted_workspace_names(workspaces: &[vcs::Workspace]) -> Vec<String> {
    let mut names: Vec<String> = workspaces.iter().map(|w| w.name.clone()).collect();
    names.sort();
    names
}

/// Find next workspace in alphabetical order
pub fn find_next_workspace(workspaces: &[vcs::Workspace]) -> Result<String> {
    let sorted_names = sorted_workspace_names(workspaces);
    let current_ws = workspaces.iter().find(|w| w.is_current);

    match current_ws {
        Some(current) => {
            let current_idx = sorted_names
                .iter()
                .position(|n| n == &current.name)
                .ok_or_else(|| Error::Internal("current workspace not in list".to_string()))?;
            let next_idx = (current_idx + 1) % sorted_names.len();
            Ok(sorted_names[next_idx].clone())
        }
        None => sorted_names
            .first()
            .cloned()
            .ok_or_else(|| Error::WorkspaceNotFound("no workspaces exist".to_string())),
    }
}

/// Find previous workspace in alphabetical order
pub fn find_prev_workspace(workspaces: &[vcs::Workspace]) -> Result<String> {
    let sorted_names = sorted_workspace_names(workspaces);
    let current_ws = workspaces.iter().find(|w| w.is_current);

    match current_ws {
        Some(current) => {
            let current_idx = sorted_names
                .iter()
                .position(|n| n == &current.name)
                .ok_or_else(|| Error::Internal("current workspace not in list".to_string()))?;
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
            .ok_or_else(|| Error::WorkspaceNotFound("no workspaces exist".to_string())),
    }
}

/// Helper: Create workspace with optional sync
pub fn spawn_with_sync(backend: &dyn vcs::VcsBackend, name: &str, sync: bool) -> Result<()> {
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
pub fn workspace_exists(backend: &dyn vcs::VcsBackend, name: &str) -> Result<bool> {
    let workspaces = backend.list_workspaces()?;
    Ok(workspaces.iter().any(|w| w.name == name))
}

/// Helper: Validate clean working copy
pub fn require_clean_working_copy(backend: &dyn vcs::VcsBackend) -> Result<()> {
    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::WorkingCopyDirty);
    }
    Ok(())
}

/// Helper to get current workspace name from backend
pub fn get_current_workspace_name(backend: &dyn vcs::VcsBackend) -> Result<String> {
    let workspaces = backend.list_workspaces()?;
    workspaces
        .iter()
        .find(|w| w.is_current)
        .map(|w| w.name.clone())
        .ok_or_else(|| Error::WorkspaceNotFound("no current workspace".to_string()))
}

/// Helper: Resolve workspace name from Option or get current
pub fn resolve_workspace_name(backend: &dyn vcs::VcsBackend, name: Option<&str>) -> Result<String> {
    match name {
        Some(n) => Ok(n.to_string()),
        None => get_current_workspace_name(backend),
    }
}

/// Helper: Complete workspace workflow (sync + push)
pub fn complete_workspace_workflow(backend: &dyn vcs::VcsBackend, name: &str) -> Result<()> {
    backend.rebase("main")?;
    Output::success("Synced with main");

    backend.push()?;
    Output::success("Pushed to remote");

    Output::success(&format!("Workspace '{}' completed", name));
    Ok(())
}

/// Helper: Prevent aborting main workspace
pub fn ensure_not_main_workspace(name: &str) -> Result<()> {
    if name == "main" {
        return Err(Error::InvalidOperation(
            "cannot abort the main workspace".to_string(),
        ));
    }
    Ok(())
}

/// Helper: Execute workspace abort (delete)
pub fn execute_workspace_abort(backend: &dyn vcs::VcsBackend, name: &str) -> Result<()> {
    backend.delete_workspace(name)?;
    Output::success(&format!("Workspace '{}' aborted and deleted", name));
    Ok(())
}

/// Build git diff command
pub fn build_git_diff_command(cwd: &std::path::Path, path: Option<&str>) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("diff");
    if let Some(p) = path {
        cmd.arg("--").arg(p);
    }
    cmd.current_dir(cwd);
    cmd
}
