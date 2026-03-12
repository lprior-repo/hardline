//! Workspace commands (from Isolate)

use std::process::Command;

use scp_core::{
    output::Output,
    vcs::{self, VcsStatus},
    Error, Result,
};

/// Sync option for spawn command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOption {
    /// Do not sync with main
    NoSync,
    /// Sync with main after spawning
    WithSync,
}

impl SyncOption {
    /// Convert bool to SyncOption
    pub fn from_bool(sync: bool) -> Self {
        if sync {
            SyncOption::WithSync
        } else {
            SyncOption::NoSync
        }
    }

    /// Returns true if sync is enabled
    pub fn is_sync(&self) -> bool {
        matches!(self, SyncOption::WithSync)
    }
}

/// Validate workspace name (P1)
/// Returns Some(Error) if invalid, None if valid
/// Enforces regex: ^[a-zA-Z][a-zA-Z0-9_-]*$
fn validate_workspace_name(name: &str) -> Option<Error> {
    if name.is_empty() {
        return Some(Error::InvalidIdentifier(
            "workspace name cannot be empty".to_string(),
        ));
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    // Must start with a letter
    if !first.is_alphabetic() {
        return Some(Error::InvalidIdentifier(format!(
            "workspace name must start with a letter, got '{}'",
            name
        )));
    }

    // Remaining chars must be alphanumeric, dash, or underscore
    if !chars.all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Some(Error::InvalidIdentifier(format!(
            "workspace name must be alphanumeric, dash, or underscore only, got '{}'",
            name
        )));
    }

    None
}

/// Get sorted workspace names
fn sorted_workspace_names(workspaces: &[vcs::Workspace]) -> Vec<String> {
    let mut names: Vec<String> = workspaces.iter().map(|w| w.name.clone()).collect();
    names.sort();
    names
}

/// Find next workspace in alphabetical order
fn find_next_workspace(workspaces: &[vcs::Workspace]) -> Result<String> {
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
fn find_prev_workspace(workspaces: &[vcs::Workspace]) -> Result<String> {
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
fn spawn_with_sync(backend: &dyn vcs::VcsBackend, name: &str, sync: bool) -> Result<()> {
    backend.create_workspace(name)?;
    Output::success(&format!("Created workspace '{}'", name));

    if sync {
        backend.switch_workspace(name)?;
        backend.rebase("main")?;
        Output::success("Synced with main");
    }

    Ok(())
}

/// Create a new workspace
pub fn spawn(name: &str, sync: bool) -> Result<()> {
    // P1: Validate workspace name BEFORE any I/O
    if let Some(err) = validate_workspace_name(name) {
        return Err(err);
    }

    Output::info(&format!("Creating workspace '{}'...", name));

    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace already exists
    let workspaces = backend.list_workspaces()?;
    if workspaces.iter().any(|w| w.name == name) {
        return Err(Error::WorkspaceExists(name.to_string()));
    }

    spawn_with_sync(backend.as_ref(), name, sync)
}

/// Helper: Check workspace exists
fn workspace_exists(backend: &dyn vcs::VcsBackend, name: &str) -> Result<bool> {
    let workspaces = backend.list_workspaces()?;
    Ok(workspaces.iter().any(|w| w.name == name))
}

/// Helper: Validate clean working copy
fn require_clean_working_copy(backend: &dyn vcs::VcsBackend) -> Result<()> {
    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::WorkingCopyDirty);
    }
    Ok(())
}

/// Switch to a workspace
pub fn switch(name: &str) -> Result<()> {
    // P1: Validate workspace name is not empty
    if name.is_empty() {
        return Err(Error::InvalidIdentifier(
            "workspace name cannot be empty".to_string(),
        ));
    }

    Output::info(&format!("Switching to workspace '{}'...", name));

    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace exists and working copy is clean
    if !workspace_exists(backend.as_ref(), name)? {
        return Err(Error::WorkspaceNotFound(name.to_string()));
    }
    require_clean_working_copy(backend.as_ref())?;

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

/// Helper to get current workspace name from backend
fn get_current_workspace_name(backend: &dyn vcs::VcsBackend) -> Result<String> {
    let workspaces = backend.list_workspaces()?;
    workspaces
        .iter()
        .find(|w| w.is_current)
        .map(|w| w.name.clone())
        .ok_or_else(|| Error::WorkspaceNotFound("no current workspace".to_string()))
}

/// Helper: Resolve workspace name from Option or get current
fn resolve_workspace_name(backend: &dyn vcs::VcsBackend, name: Option<&str>) -> Result<String> {
    match name {
        Some(n) => Ok(n.to_string()),
        None => get_current_workspace_name(backend),
    }
}

/// Helper: Complete workspace workflow (sync + push)
fn complete_workspace_workflow(backend: &dyn vcs::VcsBackend, name: &str) -> Result<()> {
    backend.rebase("main")?;
    Output::success("Synced with main");

    backend.push()?;
    Output::success("Pushed to remote");

    Output::success(&format!("Workspace '{}' completed", name));
    Ok(())
}

/// Complete workspace and merge
pub fn done(name: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    // P4: Check for dirty working copy BEFORE any operations
    require_clean_working_copy(backend.as_ref())?;

    // Resolve workspace name: if None, get current workspace
    let workspace_name = resolve_workspace_name(backend.as_ref(), name)?;

    // P3: Check workspace exists
    if !workspace_exists(backend.as_ref(), &workspace_name)? {
        return Err(Error::WorkspaceNotFound(workspace_name.clone()));
    }

    Output::info(&format!("Completing workspace '{}'...", workspace_name));

    complete_workspace_workflow(backend.as_ref(), &workspace_name)
}

/// Helper: Prevent aborting main workspace
fn ensure_not_main_workspace(name: &str) -> Result<()> {
    if name == "main" {
        return Err(Error::InvalidOperation(
            "cannot abort the main workspace".to_string(),
        ));
    }
    Ok(())
}

/// Helper: Execute workspace abort (delete)
fn execute_workspace_abort(backend: &dyn vcs::VcsBackend, name: &str) -> Result<()> {
    backend.delete_workspace(name)?;
    Output::success(&format!("Workspace '{}' aborted and deleted", name));
    Ok(())
}

/// Abort workspace
pub fn abort(name: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    require_clean_working_copy(backend.as_ref())?;

    let workspace_name = resolve_workspace_name(backend.as_ref(), name)?;
    ensure_not_main_workspace(&workspace_name)?;

    if !workspace_exists(backend.as_ref(), &workspace_name)? {
        return Err(Error::WorkspaceNotFound(workspace_name.clone()));
    }

    Output::info(&format!("Aborting workspace '{}'...", workspace_name));
    execute_workspace_abort(backend.as_ref(), &workspace_name)
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

    // Use helper function to find next workspace
    let target_name = find_next_workspace(&workspaces)?;

    // P4: Check for dirty working copy
    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::WorkingCopyDirty);
    }

    Output::info(&format!("Switching to workspace '{}'...", target_name));
    backend.switch_workspace(&target_name)?;
    Output::success(&format!("Switched to '{}'", target_name));
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

    // Use helper function to find previous workspace
    let target_name = find_prev_workspace(&workspaces)?;

    // P4: Check for dirty working copy
    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::WorkingCopyDirty);
    }

    Output::info(&format!("Switching to workspace '{}'...", target_name));
    backend.switch_workspace(&target_name)?;
    Output::success(&format!("Switched to '{}'", target_name));
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

#[cfg(test)]
mod tests {
    use super::*;

    // Test validate_workspace_name directly (pure function)
    #[test]
    fn test_validate_empty_name_returns_error() {
        let result = validate_workspace_name("");
        assert!(result.is_some());
    }

    #[test]
    fn test_validate_starts_with_digit_returns_error() {
        let result = validate_workspace_name("123invalid");
        assert!(result.is_some());
    }

    #[test]
    fn test_validate_starts_with_special_char_returns_error() {
        let result = validate_workspace_name("@invalid");
        assert!(result.is_some());
    }

    #[test]
    fn test_validate_valid_simple_name_returns_none() {
        let result = validate_workspace_name("abc");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_valid_with_dash_returns_none() {
        let result = validate_workspace_name("abc-def");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_valid_with_underscore_returns_none() {
        let result = validate_workspace_name("abc_def");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_valid_with_numbers_returns_none() {
        let result = validate_workspace_name("abc123");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_valid_mixed_returns_none() {
        let result = validate_workspace_name("abc-def_123");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_invalid_with_special_char_returns_error() {
        let result = validate_workspace_name("abc@def");
        assert!(result.is_some());
    }

    #[test]
    fn test_validate_invalid_with_exclamation_returns_error() {
        let result = validate_workspace_name("valid-name!");
        assert!(result.is_some());
    }

    #[test]
    fn test_validate_invalid_with_at_sign_returns_error() {
        let result = validate_workspace_name("abc@#$%");
        assert!(result.is_some());
    }

    #[test]
    fn test_validate_invalid_with_space_returns_error() {
        let result = validate_workspace_name("abc def");
        assert!(result.is_some());
    }

    // Test SyncOption
    #[test]
    fn test_sync_option_from_bool_true() {
        assert_eq!(SyncOption::from_bool(true), SyncOption::WithSync);
    }

    #[test]
    fn test_sync_option_from_bool_false() {
        assert_eq!(SyncOption::from_bool(false), SyncOption::NoSync);
    }

    #[test]
    fn test_sync_option_is_sync_with_sync() {
        assert!(SyncOption::WithSync.is_sync());
    }

    #[test]
    fn test_sync_option_is_sync_without_sync() {
        assert!(!SyncOption::NoSync.is_sync());
    }
}
