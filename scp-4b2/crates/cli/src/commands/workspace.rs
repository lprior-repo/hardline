//! Workspace commands (from Isolate)

use std::process::Command;

use itertools::sorted;
use scp_core::{
    output::Output,
    vcs::{self, VcsStatus},
    Error, Result,
};

/// Validate workspace name format
fn validate_workspace_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidIdentifier(
            "workspace name cannot be empty".to_string(),
        ));
    }

    let starts_with_letter = name
        .chars()
        .next()
        .map(|c| c.is_alphabetic())
        .unwrap_or(false);

    if !starts_with_letter {
        return Err(Error::InvalidIdentifier(format!(
            "workspace name must start with a letter, got '{}'",
            name
        )));
    }

    Ok(())
}

/// Check if workspace exists in list
fn workspace_exists(workspaces: &[vcs::Workspace], name: &str) -> bool {
    workspaces.iter().any(|w| w.name == name)
}

/// Create a new workspace
pub fn spawn(name: &str, sync: bool) -> Result<()> {
    validate_workspace_name(name)?;

    Output::info(&format!("Creating workspace '{}'...", name));

    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if workspace_exists(&workspaces, name) {
        return Err(Error::WorkspaceExists(name.to_string()));
    }

    backend.create_workspace(name)?;
    Output::success(&format!("Created workspace '{}'", name));

    if sync {
        backend.switch_workspace(name)?;
        backend.rebase("main")?;
        Output::success("Synced with main");
    }

    Ok(())
}

/// Validate workspace switch prerequisites
fn validate_workspace_switch(backend: &dyn vcs::VcsBackend, name: &str) -> Result<()> {
    let workspaces = backend.list_workspaces()?;
    if !workspace_exists(&workspaces, name) {
        return Err(Error::WorkspaceNotFound(name.to_string()));
    }

    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::WorkingCopyDirty);
    }

    Ok(())
}

/// Switch to a workspace
pub fn switch(name: &str) -> Result<()> {
    validate_workspace_name(name)?;

    Output::info(&format!("Switching to workspace '{}'...", name));

    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    validate_workspace_switch(backend.as_ref(), name)?;

    backend.switch_workspace(name)?;

    Output::success(&format!("Switched to '{}'", name));
    Ok(())
}

/// List workspaces
pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if workspaces.is_empty() {
        Output::info("No workspaces found");
    } else {
        Output::info("Workspaces:");
        for ws in workspaces {
            let current = if ws.is_current { " (current)" } else { "" };
            Output::info(&format!("  - {}{}", ws.name, current));
        }
    }

    Ok(())
}

/// Show workspace status
pub fn status() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    Output::info(&format!("Current branch: {}", branch));
    Output::info(&format!("Status: {}", vcs_status));

    Ok(())
}

/// Sync all workspaces with main
fn sync_all_workspaces(backend: &dyn vcs::VcsBackend) -> Result<()> {
    let workspaces = backend.list_workspaces()?;
    for ws in workspaces {
        if !ws.is_current {
            backend.switch_workspace(&ws.name)?;
        }
        backend.rebase("main")?;
        Output::success(&format!("Synced {}", ws.name));
    }
    Ok(())
}

/// Sync workspace with main
pub fn sync(_name: Option<&str>, all: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    if all {
        sync_all_workspaces(backend.as_ref())?;
    } else {
        backend.rebase("main")?;
        Output::success("Synced with main");
    }

    Ok(())
}

/// Validate workspace exists if name provided
fn validate_workspace_for_done(name: Option<&str>) -> Result<()> {
    if let Some(ws_name) = name {
        let cwd = std::env::current_dir().map_err(Error::Io)?;
        let backend = vcs::create_backend(&cwd)?;
        let workspaces = backend.list_workspaces()?;
        if !workspace_exists(&workspaces, ws_name) {
            return Err(Error::WorkspaceNotFound(ws_name.to_string()));
        }
    }
    Ok(())
}

/// Complete workspace and merge
pub fn done(name: Option<&str>) -> Result<()> {
    let workspace_name = name.unwrap_or("current");
    validate_workspace_for_done(name)?;

    Output::info(&format!("Completing workspace '{}'...", workspace_name));

    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    backend.rebase("main")?;
    Output::success("Synced with main");

    backend.push()?;
    Output::success("Pushed to remote");

    Output::success(&format!("Workspace '{}' completed", workspace_name));
    Ok(())
}

/// Abort workspace
pub fn abort(name: Option<&str>) -> Result<()> {
    let workspace_name = name.unwrap_or("current");

    if workspace_name == "main" {
        return Err(Error::InvalidOperation(
            "cannot abort the main workspace".to_string(),
        ));
    }

    println!("Aborting workspace '{}'...", workspace_name);

    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspace_exists(&workspaces, workspace_name) {
        return Err(Error::WorkspaceNotFound(workspace_name.to_string()));
    }

    backend.delete_workspace(workspace_name)?;

    println!("✓ Workspace '{}' aborted and deleted", workspace_name);
    Ok(())
}

/// Show workspace log
pub fn log(limit: Option<usize>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let commits = backend.log(limit.unwrap_or(10))?;

    if commits.is_empty() {
        println!("No commits found");
    } else {
        for commit in commits {
            println!("{}", commit.id);
            println!("  {}", commit.message);
            println!();
        }
    }

    Ok(())
}

/// Build JJ diff command
fn build_jj_diff_command(cwd: &std::path::Path, path: Option<&str>) -> Command {
    let mut cmd = Command::new("jj");
    cmd.arg("diff");
    if let Some(p) = path {
        cmd.arg(p);
    }
    cmd.current_dir(cwd);
    cmd
}

/// Show diff of changes
pub fn diff(path: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let mut cmd = build_jj_diff_command(&cwd, path);
    let output = cmd.output().map_err(Error::Io)?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        return Err(Error::VcsConflict(
            "diff".to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}

/// Handle clean working copy status
fn handle_clean_status() {
    println!("Working copy is clean");
}

/// Handle dirty working copy status
fn handle_dirty_status(cwd: &std::path::Path) -> Result<()> {
    println!("Uncommitted changes:");
    let output = Command::new("jj")
        .arg("status")
        .current_dir(cwd)
        .output()
        .map_err(Error::Io)?;
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

/// Handle conflicted working copy status
fn handle_conflicted_status(cwd: &std::path::Path) -> Result<()> {
    println!("Conflicted files:");
    let output = Command::new("jj")
        .arg("log")
        .arg("-r")
        .arg("@")
        .arg("-T")
        .arg("conflicts()")
        .current_dir(cwd)
        .output()
        .map_err(Error::Io)?;
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

/// Handle detached HEAD status
fn handle_detached_status() {
    println!("Detached HEAD");
}

/// Show uncommitted changes
pub fn uncommitted() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;
    let status = backend.status()?;

    match status {
        VcsStatus::Clean => handle_clean_status(),
        VcsStatus::Dirty => handle_dirty_status(&cwd)?,
        VcsStatus::Conflicted => handle_conflicted_status(&cwd)?,
        VcsStatus::Detached => handle_detached_status(),
    }

    Ok(())
}

/// Commit uncommitted changes
pub fn commit(message: &str) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let status = backend.status()?;

    if status == VcsStatus::Clean {
        println!("No changes to commit");
        return Ok(());
    }

    let output = Command::new("jj")
        .args(["describe", "-m", message])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::VcsConflict(
            "commit".to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    println!("✓ Committed: {}", message);
    Ok(())
}

/// List branches
pub fn branches() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let branches = backend.list_branches()?;

    if branches.is_empty() {
        println!("No branches found");
    } else {
        println!("Branches:");
        for branch in branches {
            let current = if branch.is_current { " (current)" } else { "" };
            println!("  - {}{}", branch.name, current);
        }
    }

    Ok(())
}

/// Create a new branch
pub fn branch_create(name: &str) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    backend.create_branch(name)?;

    println!("✓ Created branch '{}'", name);
    Ok(())
}

/// Delete a branch
pub fn branch_delete(name: &str) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["bookmark", "delete", name])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::BranchNotFound(name.to_string()));
    }

    println!("✓ Deleted branch '{}'", name);
    Ok(())
}

/// Show current branch info
pub fn branch_current() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let branch = backend.current_branch()?;

    println!("Current branch: {}", branch);
    Ok(())
}

/// Fork a workspace from another workspace
pub fn fork(name: &str, from: Option<&str>) -> Result<()> {
    let source = from.unwrap_or("main");
    println!("Forking workspace '{}' from '{}'...", name, source);

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    backend.fork_workspace(source, name)?;
    println!("✓ Forked workspace '{}' from '{}'", name, source);

    Ok(())
}

/// Merge a workspace into main
pub fn merge(name: &str) -> Result<()> {
    println!("Merging workspace '{}' into main...", name);

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    backend.merge_workspace(name)?;
    println!("✓ Merged workspace '{}' into main", name);

    Ok(())
}

/// Direction for workspace navigation
enum Direction {
    Next,
    Prev,
}

/// Get sorted workspace names using iterator pipeline
fn get_sorted_workspace_names(workspaces: &[vcs::Workspace]) -> Vec<String> {
    sorted(workspaces.iter().map(|w| w.name.clone())).collect()
}

/// Find adjacent workspace name
fn find_adjacent_workspace(
    workspaces: &[vcs::Workspace],
    sorted_names: &[String],
    direction: Direction,
) -> Result<String> {
    let current_ws = workspaces.iter().find(|w| w.is_current);

    match current_ws {
        Some(current) => {
            let current_idx = sorted_names
                .iter()
                .position(|n| n == &current.name)
                .ok_or_else(|| Error::Internal("current workspace not in list".to_string()))?;
            let next_idx = match direction {
                Direction::Next => (current_idx + 1) % sorted_names.len(),
                Direction::Prev => {
                    if current_idx == 0 {
                        sorted_names.len() - 1
                    } else {
                        current_idx - 1
                    }
                }
            };
            Ok(sorted_names[next_idx].clone())
        }
        None => {
            let idx = match direction {
                Direction::Next => 0,
                Direction::Prev => sorted_names.len() - 1,
            };
            Ok(sorted_names[idx].clone())
        }
    }
}

/// Validate workspace navigation is possible
fn validate_workspace_navigation(backend: &dyn vcs::VcsBackend) -> Result<()> {
    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::WorkingCopyDirty);
    }
    Ok(())
}

/// Switch to next workspace (alphabetically)
#[allow(dead_code)]
pub fn next() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if workspaces.is_empty() {
        return Err(Error::WorkspaceNotFound("no workspaces exist".to_string()));
    }

    let sorted_names = get_sorted_workspace_names(&workspaces);
    let target_name = find_adjacent_workspace(&workspaces, &sorted_names, Direction::Next)?;

    println!("Switching to workspace '{}'...", target_name);
    validate_workspace_navigation(backend.as_ref())?;
    backend.switch_workspace(&target_name)?;
    println!("✓ Switched to '{}'", target_name);
    Ok(())
}

/// Switch to previous workspace (alphabetically)
#[allow(dead_code)]
pub fn prev() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if workspaces.is_empty() {
        return Err(Error::WorkspaceNotFound("no workspaces exist".to_string()));
    }

    let sorted_names = get_sorted_workspace_names(&workspaces);
    let target_name = find_adjacent_workspace(&workspaces, &sorted_names, Direction::Prev)?;

    println!("Switching to workspace '{}'...", target_name);
    validate_workspace_navigation(backend.as_ref())?;
    backend.switch_workspace(&target_name)?;
    println!("✓ Switched to '{}'", target_name);
    Ok(())
}

/// Validate path exists and is a directory
fn validate_workspace_path(path: &str) -> Result<std::path::PathBuf> {
    let workspace_path = std::path::Path::new(path).to_path_buf();

    if !workspace_path.exists() {
        return Err(Error::NotFound(format!("Path does not exist: {}", path)));
    }

    if !workspace_path.is_dir() {
        return Err(Error::InvalidState(format!(
            "Path is not a directory: {}",
            path
        )));
    }

    Ok(workspace_path)
}

/// Check if workspace path already exists
fn workspace_path_exists(workspaces: &[vcs::Workspace], path_str: &str) -> Option<String> {
    workspaces
        .iter()
        .find(|ws| ws.name == path_str || ws.branch == path_str)
        .map(|ws| ws.name.clone())
}

/// Execute jj workspace add command
fn exec_workspace_add(cwd: &std::path::Path, path: &str) -> Result<()> {
    let output = Command::new("jj")
        .args(["workspace", "add", path])
        .current_dir(cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "workspace add".to_string(),
            stderr.to_string(),
        ));
    }

    Ok(())
}

/// Add an existing path as a workspace
pub fn add(path: &str) -> Result<()> {
    let workspace_path = validate_workspace_path(path)?;

    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    let path_str = workspace_path.to_string_lossy().to_string();

    if let Some(existing) = workspace_path_exists(&workspaces, &path_str) {
        return Err(Error::WorkspaceExists(existing));
    }

    println!("Adding workspace at '{}'...", path);
    exec_workspace_add(&cwd, path)?;
    println!("✓ Added workspace at '{}'", path);

    Ok(())
}
