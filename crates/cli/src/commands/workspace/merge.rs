//! Workspace fork and merge commands

use std::process::Command;

use scp_core::{output::Output, vcs, Error};

/// Fork workspace
pub fn fork(name: &str, from: Option<&str>) -> Result<(), Error> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "workspace name must not be empty",
        ));
    }
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if workspaces.iter().any(|w| w.name == name) {
        return Err(Error::workspace_exists(name));
    }

    // Help says FROM defaults to current, and the old code defaulted to
    // "main" — the current branch is the honest default. A FROM naming a
    // workspace (not a branch) resolves to that workspace's branch, so
    // forking from a fork works instead of failing on an invalid ref.
    let from_branch = match from {
        Some(f) => workspaces
            .iter()
            .find(|w| w.name == f)
            .filter(|w| !w.branch.is_empty())
            .map(|w| w.branch.clone())
            .unwrap_or_else(|| f.to_string()),
        None => backend.current_branch()?,
    };
    if from_branch.is_empty() {
        return Err(Error::invalid_identifier(
            "cannot fork from a detached HEAD",
        ));
    }

    // `git worktree add <path> <branch>` fails when the branch is already
    // checked out anywhere — the fork-from-main bug. In that case fork
    // onto a new branch named after the new workspace.
    let checked_out = workspaces.iter().any(|w| w.branch == from_branch);
    let mut cmd = Command::new("git");
    if checked_out {
        cmd.args(["worktree", "add", "-b", name, name, &from_branch]);
    } else {
        cmd.args(["worktree", "add", name, &from_branch]);
    }
    let output = cmd.current_dir(&cwd).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("worktree add", stderr));
    }

    if Output::json_output() {
        Output::json(&serde_json::json!({
            "workspace": name,
            "from": from_branch,
            "result": "forked",
        }));
    } else {
        Output::success(&format!(
            "Forked workspace '{name}' from '{from_branch}'"
        ));
    }

    Ok(())
}

/// Merge workspace
pub fn merge(name: &str) -> Result<(), Error> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "workspace name must not be empty",
        ));
    }
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    // The old code called the switch_workspace stub, then rebased, then
    // printed "Merged" — no merge ever happened. merge_workspace really
    // merges the worktree's branch into main; print only after it succeeds.
    backend.merge_workspace(name)?;

    if Output::json_output() {
        Output::json(&serde_json::json!({
            "workspace": name,
            "result": "merged",
        }));
    } else {
        Output::success(&format!("Merged workspace '{name}'"));
    }

    Ok(())
}
