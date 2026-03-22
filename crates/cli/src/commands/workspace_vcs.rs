//! Workspace VCS operations

use std::process::Command;

use scp_core::{
    output::Output,
    vcs::{self, VcsStatus},
    Error, Result,
};

use super::workspace_helpers::build_jj_diff_command;

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
