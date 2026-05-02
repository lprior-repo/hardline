//! Action functions for the prune command handler (Tier 3).
//!
//! I/O operations that discover and remove stale workspace directories.
//! A workspace is "stale" if its directory no longer exists on disk,
//! or its branch has been merged into trunk.

use std::io::{self, BufRead, Write};

use scp_core::{output::Output, vcs, Error, Result};

use super::data::{PrunableItem, PruneMode, PruneOptions, PruneOutput};

/// Execute the prune command with the given options.
///
/// Dispatches based on `PruneMode`: dry-run shows what would be pruned,
/// confirm skips prompts, interactive asks the user.
///
/// # Errors
///
/// Returns errors for VCS backend failures or workspace removal failures.
pub fn run_prune(options: &PruneOptions) -> Result<PruneOutput> {
    let stale = discover_stale_workspaces()?;

    match options.mode {
        PruneMode::DryRun => Ok(PruneOutput::dry_run(
            stale.into_iter().map(|i| i.name).collect(),
        )),
        PruneMode::Confirm => prune_confirmed(stale),
        PruneMode::Interactive => prune_interactive(stale),
    }
}

/// Discover stale workspaces by comparing VCS workspaces against
/// the filesystem and checking branch merge status.
///
/// A workspace is stale if:
/// - Its directory no longer exists on disk, OR
/// - Its branch has been merged into trunk (main/master)
fn discover_stale_workspaces() -> Result<Vec<PrunableItem>> {
    let cwd = std::env::current_dir()
        .map_err(|e| Error::io_error(format!("Failed to determine current directory: {e}")))?;
    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend
        .list_workspaces()
        .map_err(|e| Error::internal(format!("Failed to list workspaces: {e}")))?;

    let current_branch = backend
        .current_branch()
        .map_err(|e| Error::internal(format!("Failed to get current branch: {e}")))?;

    let trunk_branches = ["main", "master"];
    let is_on_trunk = trunk_branches.contains(&current_branch.as_str());

    let mut stale = Vec::new();

    for ws in &workspaces {
        // Skip the current workspace
        if ws.is_current {
            continue;
        }

        let ws_path = std::path::Path::new(&ws.name);
        let full_path = cwd.join(ws_path);

        // Stale if directory missing
        if !full_path.exists() {
            stale.push(PrunableItem {
                name: ws.name.clone(),
                workspace_path: full_path.to_string_lossy().to_string(),
            });
            continue;
        }

        // Stale if branch merged into trunk and we are on trunk
        if is_on_trunk && branch_is_merged(&ws.branch, &current_branch, backend.as_ref())? {
            stale.push(PrunableItem {
                name: ws.name.clone(),
                workspace_path: full_path.to_string_lossy().to_string(),
            });
        }
    }

    Ok(stale)
}

/// Check whether a branch has been merged into the target branch.
///
/// Uses `git branch --merged <target>` via the VCS backend's
/// `list_branches` combined with a subprocess call.
fn branch_is_merged(branch: &str, target: &str, _backend: &dyn vcs::VcsBackend) -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["branch", "--merged", target])
        .output()
        .map_err(|e| Error::io_error(format!("Failed to run git branch --merged: {e}")))?;

    if !output.status.success() {
        return Err(Error::internal(
            "git branch --merged returned non-zero status".to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim().trim_start_matches('*').trim();
        if trimmed == branch {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Remove stale workspaces without prompting (confirm mode).
fn prune_confirmed(items: Vec<PrunableItem>) -> Result<PruneOutput> {
    let names: Vec<String> = items.iter().map(|i| i.name.clone()).collect();
    let mut removed = 0;

    for item in &items {
        match remove_workspace(item) {
            Ok(()) => removed += 1,
            Err(e) => Output::warn(&format!("Failed to remove '{}': {e}", item.name)),
        }
    }

    Ok(PruneOutput::completed(names, removed))
}

/// Remove stale workspaces interactively, prompting for each.
fn prune_interactive(items: Vec<PrunableItem>) -> Result<PruneOutput> {
    if items.is_empty() {
        Output::info("No stale workspaces found");
        return Ok(PruneOutput::empty());
    }

    Output::info(&format!("Found {} stale workspace(s):", items.len()));
    for (i, item) in items.iter().enumerate() {
        println!("  [{}] {} ({})", i + 1, item.name, item.workspace_path);
    }
    println!();

    let names: Vec<String> = items.iter().map(|i| i.name.clone()).collect();
    let mut removed = 0;

    for item in &items {
        if prompt_yes_no(&format!("Remove workspace '{}'?", item.name))? {
            match remove_workspace(item) {
                Ok(()) => {
                    removed += 1;
                    Output::success(&format!("Removed '{}'", item.name));
                }
                Err(e) => Output::warn(&format!("Failed to remove '{}': {e}", item.name)),
            }
        }
    }

    Ok(PruneOutput::completed(names, removed))
}

/// Prompt the user with a yes/no question, returning the answer.
fn prompt_yes_no(question: &str) -> Result<bool> {
    print!("{question} [y/N] ");
    io::stdout()
        .flush()
        .map_err(|e| Error::io_error(format!("Failed to flush stdout: {e}")))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| Error::io_error(format!("Failed to read input: {e}")))?;

    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

/// Remove a workspace directory and its contents.
fn remove_workspace(item: &PrunableItem) -> Result<()> {
    let path = std::path::Path::new(&item.workspace_path);
    if !path.exists() {
        // Already gone, consider it removed
        return Ok(());
    }

    std::fs::remove_dir_all(path).map_err(|e| {
        Error::io_error(format!(
            "Failed to remove workspace directory '{}': {e}",
            item.workspace_path
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: initialise a real Git repo in `dir` so gix can open it.
    fn git_init(dir: &std::path::Path) {
        let output = std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .expect("git init");
        assert!(
            output.status.success(),
            "git init failed: {:?}",
            output.stderr
        );

        let commit = std::process::Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .expect("git commit");
        assert!(
            commit.status.success(),
            "git commit failed: {:?}",
            commit.stderr
        );
    }

    #[test]
    fn run_prune_dry_run_no_stale_in_real_repo() {
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        git_init(dir.path());
        let original = match std::env::current_dir() {
            Ok(d) => d,
            Err(_) => return,
        };
        let _ = std::env::set_current_dir(dir.path());

        let opts = PruneOptions {
            mode: PruneMode::DryRun,
        };
        let result = run_prune(&opts);
        assert!(result.is_ok());
        let output = result.expect("ok");
        assert_eq!(output.invalid_count, 0);

        let _ = std::env::set_current_dir(&original);
    }

    #[test]
    fn run_prune_confirm_no_stale_in_real_repo() {
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        git_init(dir.path());
        let original = match std::env::current_dir() {
            Ok(d) => d,
            Err(_) => return,
        };
        let _ = std::env::set_current_dir(dir.path());

        let opts = PruneOptions {
            mode: PruneMode::Confirm,
        };
        let result = run_prune(&opts);
        assert!(result.is_ok());

        let _ = std::env::set_current_dir(&original);
    }

    #[test]
    fn run_prune_dry_run_with_no_vcs() {
        let Ok(dir) = tempfile::tempdir() else { return };
        let Ok(original) = std::env::current_dir() else {
            return;
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            return;
        }

        let opts = PruneOptions {
            mode: PruneMode::DryRun,
        };
        let result = run_prune(&opts);
        assert!(result.is_err());

        let _ = std::env::set_current_dir(&original);
    }

    #[test]
    fn prune_output_empty() {
        let output = PruneOutput::empty();
        assert_eq!(output.invalid_count, 0);
        assert_eq!(output.removed_count, 0);
        assert!(output.invalid_sessions.is_empty());
    }

    #[test]
    fn prune_output_dry_run_construction() {
        let sessions = vec!["ws-a".to_string(), "ws-b".to_string()];
        let output = PruneOutput::dry_run(sessions.clone());
        assert_eq!(output.invalid_count, 2);
        assert_eq!(output.removed_count, 0);
    }

    #[test]
    fn prune_output_completed_construction() {
        let sessions = vec!["ws-a".to_string()];
        let output = PruneOutput::completed(sessions.clone(), 1);
        assert_eq!(output.invalid_count, 1);
        assert_eq!(output.removed_count, 1);
    }

    #[test]
    fn remove_workspace_nonexistent_is_ok() {
        let item = PrunableItem {
            name: "ghost".to_string(),
            workspace_path: "/nonexistent/ghost".to_string(),
        };
        assert!(remove_workspace(&item).is_ok());
    }

    #[test]
    fn remove_workspace_actually_removes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ws_dir = dir.path().join("to-remove");
        std::fs::create_dir_all(ws_dir.join("subdir")).expect("create");
        assert!(ws_dir.exists());

        let item = PrunableItem {
            name: "to-remove".to_string(),
            workspace_path: ws_dir.to_string_lossy().to_string(),
        };
        assert!(remove_workspace(&item).is_ok());
        assert!(!ws_dir.exists());
    }
}
