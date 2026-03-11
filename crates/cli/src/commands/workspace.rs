//! Workspace commands (from Isolate)

use std::process::Command;

use scp_core::{
    vcs::{self, VcsStatus},
    Error, Result,
};

/// Create a new workspace
pub fn spawn(name: &str, sync: bool) -> Result<()> {
    println!("Creating workspace '{}'...", name);

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
    println!("✓ Created workspace '{}'", name);

    if sync {
        backend.switch_workspace(name)?;
        backend.rebase("main")?;
        println!("✓ Synced with main");
    }

    Ok(())
}

/// Switch to a workspace
pub fn switch(name: &str) -> Result<()> {
    println!("Switching to workspace '{}'...", name);

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

    println!("✓ Switched to '{}'", name);
    Ok(())
}

/// List workspaces
pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if workspaces.is_empty() {
        println!("No workspaces found");
    } else {
        println!("Workspaces:");
        for ws in workspaces {
            let current = if ws.is_current { " (current)" } else { "" };
            println!("  - {}{}", ws.name, current);
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

    println!("Current branch: {}", branch);
    println!("Status: {}", vcs_status);

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
            println!("✓ Synced {}", ws.name);
        }
    } else {
        backend.rebase("main")?;
        println!("✓ Synced with main");
    }

    Ok(())
}

/// Complete workspace and merge
pub fn done(name: Option<&str>) -> Result<()> {
    let workspace_name = name.unwrap_or("current");
    println!("Completing workspace '{}'...", workspace_name);

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    // Sync first
    backend.rebase("main")?;
    println!("✓ Synced with main");

    // Push
    backend.push()?;
    println!("✓ Pushed to remote");

    println!("✓ Workspace '{}' completed", workspace_name);
    Ok(())
}

/// Abort workspace
pub fn abort(name: Option<&str>) -> Result<()> {
    let workspace_name = name.unwrap_or("current");
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

/// Show diff of changes
pub fn diff(path: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    // Run jj diff
    let mut cmd = Command::new("jj");
    cmd.arg("diff");
    if let Some(p) = path {
        cmd.arg(p);
    }
    cmd.current_dir(&cwd);

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
