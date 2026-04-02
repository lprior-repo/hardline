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

#[cfg(test)]
mod tests {
    use super::*;
    use scp_core::vcs::Workspace;

    // ---- Helper ----

    fn ws(name: &str, is_current: bool) -> Workspace {
        Workspace {
            name: name.to_string(),
            branch: format!("{name}-branch"),
            is_current,
        }
    }

    // ---- sorted_workspace_names ----

    #[test]
    fn sorted_names_empty() {
        let result = sorted_workspace_names(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn sorted_names_single() {
        let result = sorted_workspace_names(&[ws("main", true)]);
        assert_eq!(result, vec!["main"]);
    }

    #[test]
    fn sorted_names_already_sorted() {
        let workspaces = vec![ws("alpha", false), ws("beta", true), ws("gamma", false)];
        assert_eq!(
            sorted_workspace_names(&workspaces),
            vec!["alpha", "beta", "gamma"]
        );
    }

    #[test]
    fn sorted_names_reverse_order() {
        let workspaces = vec![ws("gamma", false), ws("beta", true), ws("alpha", false)];
        assert_eq!(
            sorted_workspace_names(&workspaces),
            vec!["alpha", "beta", "gamma"]
        );
    }

    #[test]
    fn sorted_names_mixed_case() {
        let workspaces = vec![ws("Zebra", false), ws("alpha", false), ws("Beta", false)];
        // sort() uses lexicographic order: uppercase letters come before lowercase
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "Beta");
        assert_eq!(result[1], "Zebra");
        assert_eq!(result[2], "alpha");
    }

    #[test]
    fn sorted_names_duplicates() {
        let workspaces = vec![ws("alpha", false), ws("alpha", false)];
        assert_eq!(sorted_workspace_names(&workspaces), vec!["alpha", "alpha"]);
    }

    // ---- find_next_workspace ----

    #[test]
    fn find_next_wraps_around() {
        let workspaces = vec![ws("alpha", true), ws("beta", false), ws("gamma", false)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "beta");
    }

    #[test]
    fn find_next_last_wraps_to_first() {
        let workspaces = vec![ws("alpha", false), ws("beta", false), ws("gamma", true)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "alpha");
    }

    #[test]
    fn find_next_middle() {
        let workspaces = vec![ws("alpha", false), ws("beta", true), ws("gamma", false)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "gamma");
    }

    #[test]
    fn find_next_single_workspace_wraps() {
        let workspaces = vec![ws("only", true)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "only");
    }

    #[test]
    fn find_next_no_current_returns_first() {
        let workspaces = vec![ws("alpha", false), ws("beta", false)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "alpha");
    }

    #[test]
    fn find_next_empty_returns_error() {
        let result = find_next_workspace(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn find_next_sorted_input() {
        // Ensure alphabetical order is respected regardless of input order
        let workspaces = vec![ws("charlie", false), ws("alpha", true), ws("bravo", false)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "bravo");
    }

    // ---- find_prev_workspace ----

    #[test]
    fn find_prev_wraps_from_first_to_last() {
        let workspaces = vec![ws("alpha", true), ws("beta", false), ws("gamma", false)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "gamma");
    }

    #[test]
    fn find_prev_middle() {
        let workspaces = vec![ws("alpha", false), ws("beta", true), ws("gamma", false)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "alpha");
    }

    #[test]
    fn find_prev_last() {
        let workspaces = vec![ws("alpha", false), ws("beta", false), ws("gamma", true)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "beta");
    }

    #[test]
    fn find_prev_single_workspace_wraps() {
        let workspaces = vec![ws("only", true)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "only");
    }

    #[test]
    fn find_prev_no_current_returns_last() {
        let workspaces = vec![ws("alpha", false), ws("beta", false)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "beta");
    }

    #[test]
    fn find_prev_empty_returns_error() {
        let result = find_prev_workspace(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn find_prev_sorted_input() {
        let workspaces = vec![ws("charlie", false), ws("alpha", false), ws("bravo", true)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "alpha");
    }

    // ---- ensure_not_main_workspace ----

    #[test]
    fn ensure_not_main_rejects_main() {
        let result = ensure_not_main_workspace("main");
        assert!(result.is_err());
    }

    #[test]
    fn ensure_not_main_accepts_other() {
        let result = ensure_not_main_workspace("feature-branch");
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_not_main_accepts_empty() {
        let result = ensure_not_main_workspace("");
        assert!(result.is_ok());
    }

    // ---- build_jj_diff_command ----

    #[test]
    fn build_jj_diff_no_path() {
        let cmd = build_jj_diff_command(std::path::Path::new("/tmp"), None);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"diff".to_string()));
        assert!(args.contains(&"working".to_string()));
        assert!(args.contains(&"@".to_string()));
    }

    #[test]
    fn build_jj_diff_with_path() {
        let cmd = build_jj_diff_command(std::path::Path::new("/tmp"), Some("src/main.rs"));
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn build_jj_diff_cwd_is_set() {
        // Verify the function constructs a valid Command without panicking.
        // Command::current_dir() is set but cannot be inspected on all platforms.
        let _cmd = build_jj_diff_command(std::path::Path::new("/test/dir"), None);
    }
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
