//! Workspace commands (from Isolate)

use std::process::Command;

use scp_core::{
    output::Output,
    vcs::{self, VcsStatus},
    Error, Result,
};

/// Create a new workspace
pub fn spawn(name: &str, sync: bool) -> Result<()> {
    // P1: Validate workspace name is not empty
    if name.is_empty() {
        return Err(Error::InvalidIdentifier(
            "workspace name cannot be empty".to_string(),
        ));
    }

    // P1: Validate workspace name format (must start with letter)
    if !name
        .chars()
        .next()
        .map(|c| c.is_alphabetic())
        .unwrap_or(false)
    {
        return Err(Error::InvalidIdentifier(format!(
            "workspace name must start with a letter, got '{}'",
            name
        )));
    }

    Output::info(&format!("Creating workspace '{}'...", name));

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    // Detect VCS
    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace already exists
    let workspaces = backend.list_workspaces()?;
    if workspaces.iter().any(|w| w.name == name) {
        return Err(Error::WorkspaceExists(name.to_string()));
    }

    // Create workspace
    backend.create_workspace(name)?;
    Output::success(&format!("Created workspace '{}'", name));

    if sync {
        backend.switch_workspace(name)?;
        backend.rebase("main")?;
        Output::success("Synced with main");
    }

    Ok(())
}

/// Switch to a workspace
pub fn switch(name: &str) -> Result<()> {
    Output::info(&format!("Switching to workspace '{}'...", name));

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace exists
    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::WorkspaceNotFound(name.to_string()));
    }

    // Check for uncommitted changes
    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::WorkingCopyDirty);
    }

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

/// Sync workspace with main
pub fn sync(_name: Option<&str>, all: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    if all {
        // Sync all workspaces
        let workspaces = backend.list_workspaces()?;
        for ws in workspaces {
            if !ws.is_current {
                backend.switch_workspace(&ws.name)?;
            }
            backend.rebase("main")?;
            Output::success(&format!("Synced {}", ws.name));
        }
    } else {
        backend.rebase("main")?;
        Output::success("Synced with main");
    }

    Ok(())
}

/// Complete workspace and merge
pub fn done(name: Option<&str>) -> Result<()> {
    let workspace_name = name.unwrap_or("current");

    // Don't use "current" for existence check - it will fail
    if name.is_some() {
        let workspace_name = name.unwrap();

        let cwd = std::env::current_dir().map_err(Error::Io)?;
        let backend = vcs::create_backend(&cwd)?;

        // P3: Check workspace exists
        let workspaces = backend.list_workspaces()?;
        if !workspaces.iter().any(|w| w.name == workspace_name) {
            return Err(Error::WorkspaceNotFound(workspace_name.to_string()));
        }
    }

    Output::info(&format!("Completing workspace '{}'...", workspace_name));

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    // Sync first
    backend.rebase("main")?;
    Output::success("Synced with main");

    // Push
    backend.push()?;
    Output::success("Pushed to remote");

    Output::success(&format!("Workspace '{}' completed", workspace_name));
    Ok(())
}

/// Abort workspace
pub fn abort(name: Option<&str>) -> Result<()> {
    let workspace_name = name.unwrap_or("current");

    // P5: Prevent aborting main workspace
    if workspace_name == "main" {
        return Err(Error::InvalidOperation(
            "cannot abort the main workspace".to_string(),
        ));
    }

    println!("Aborting workspace '{}'...", workspace_name);

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace exists
    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == workspace_name) {
        return Err(Error::WorkspaceNotFound(workspace_name.to_string()));
    }

    // For JJ, we delete the workspace
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

/// Show uncommitted changes
pub fn uncommitted() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let status = backend.status()?;

    match status {
        VcsStatus::Clean => println!("Working copy is clean"),
        VcsStatus::Dirty => {
            println!("Uncommitted changes:");
            let output = Command::new("jj")
                .arg("status")
                .current_dir(&cwd)
                .output()
                .map_err(Error::Io)?;
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        VcsStatus::Conflicted => {
            println!("Conflicted files:");
            let output = Command::new("jj")
                .arg("log")
                .arg("-r")
                .arg("@")
                .arg("-T")
                .arg("conflicts()")
                .current_dir(&cwd)
                .output()
                .map_err(Error::Io)?;
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        VcsStatus::Detached => println!("Detached HEAD"),
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

    // Run jj describe to set commit message
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

    // Run jj bookmark delete
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

/// Switch to next workspace (alphabetically)
pub fn next() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if workspaces.is_empty() {
        return Err(Error::WorkspaceNotFound("no workspaces exist".to_string()));
    }

    let sorted_names: Vec<&str> = {
        let mut names: Vec<&str> = workspaces.iter().map(|w| w.name.as_str()).collect();
        names.sort();
        names
    };

    let current_ws = workspaces.iter().find(|w| w.is_current);

    let target_name = match current_ws {
        Some(current) => {
            let current_idx = sorted_names
                .iter()
                .position(|&n| n == current.name)
                .ok_or_else(|| Error::Internal("current workspace not in list".to_string()))?;
            let next_idx = (current_idx + 1) % sorted_names.len();
            sorted_names[next_idx]
        }
        None => sorted_names[0],
    };

    println!("Switching to workspace '{}'...", target_name);

    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::WorkingCopyDirty);
    }

    backend.switch_workspace(target_name)?;

    println!("✓ Switched to '{}'", target_name);
    Ok(())
}

/// Switch to previous workspace (alphabetically)
pub fn prev() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if workspaces.is_empty() {
        return Err(Error::WorkspaceNotFound("no workspaces exist".to_string()));
    }

    let sorted_names: Vec<&str> = {
        let mut names: Vec<&str> = workspaces.iter().map(|w| w.name.as_str()).collect();
        names.sort();
        names
    };

    let current_ws = workspaces.iter().find(|w| w.is_current);

    let target_name = match current_ws {
        Some(current) => {
            let current_idx = sorted_names
                .iter()
                .position(|&n| n == current.name)
                .ok_or_else(|| Error::Internal("current workspace not in list".to_string()))?;
            let prev_idx = if current_idx == 0 {
                sorted_names.len() - 1
            } else {
                current_idx - 1
            };
            sorted_names[prev_idx]
        }
        None => sorted_names[sorted_names.len() - 1],
    };

    println!("Switching to workspace '{}'...", target_name);

    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::WorkingCopyDirty);
    }

    backend.switch_workspace(target_name)?;

    println!("✓ Switched to '{}'", target_name);
    Ok(())
}

/// Add an existing path as a workspace
pub fn add(path: &str) -> Result<()> {
    let workspace_path = std::path::Path::new(path);

    if !workspace_path.exists() {
        return Err(Error::NotFound(format!("Path does not exist: {}", path)));
    }

    if !workspace_path.is_dir() {
        return Err(Error::InvalidState(format!(
            "Path is not a directory: {}",
            path
        )));
    }

    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    let path_str = workspace_path.to_string_lossy().to_string();

    for ws in workspaces {
        if ws.name == path_str || ws.branch == path_str {
            return Err(Error::WorkspaceExists(ws.name));
        }
    }

    println!("Adding workspace at '{}'...", path);

    let output = Command::new("jj")
        .args(["workspace", "add", path])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "workspace add".to_string(),
            stderr.to_string(),
        ));
    }

    println!("✓ Added workspace at '{}'", path);

    Ok(())
}
