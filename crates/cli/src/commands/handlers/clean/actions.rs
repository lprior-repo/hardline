//! Action functions for the clean command handler (Tier 3).
//!
//! I/O operations that detect and remove stale workspace sessions.
//! A workspace is considered stale when its git worktree directory
//! no longer exists on disk but the worktree reference remains.

use std::path::Path;

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{CleanOptions, CleanOutput};

/// Execute the clean command with the given options.
///
/// Workflow:
/// 1. List git worktrees from the current repository.
/// 2. Filter to find stale worktrees (directory no longer exists).
/// 3. In dry-run mode, list stale worktrees and exit.
/// 4. Otherwise, prune stale worktree references via `git worktree prune`.
/// 5. Report results.
///
/// # Errors
///
/// Returns errors if the current directory is not a git repository,
/// or if git commands fail during worktree enumeration or pruning.
pub fn run_clean(options: &CleanOptions) -> Result<CleanOutput> {
    let cwd = std::env::current_dir()
        .map_err(|e| Error::io_error(format!("Failed to determine current directory: {e}")))?;

    // Verify we are in a git repository
    let backend = scp_core::vcs::create_backend(&cwd)?;

    // 1. List worktrees via git worktree list
    let worktree_entries = list_worktrees(backend.as_ref(), &cwd)?;

    // 2. Find stale worktrees (directory no longer exists)
    let stale_sessions: Vec<String> = worktree_entries
        .into_iter()
        .filter(|entry| {
            // Skip the main worktree (always at repo root, always valid)
            if entry.is_main {
                return false;
            }
            !Path::new(&entry.path).exists()
        })
        .map(|entry| entry.name)
        .collect();

    // 3. Handle no stale sessions
    if stale_sessions.is_empty() {
        Output::success("No stale sessions found");
        Output::info("  All sessions have valid workspaces");
        return Ok(CleanOutput::no_stale());
    }

    // 4. Dry-run: list and exit
    if options.dry_run {
        Output::info(&format!(
            "Found {} stale session(s) (dry-run, no changes made):",
            stale_sessions.len()
        ));
        for name in &stale_sessions {
            Output::info(&format!("  - {name}"));
        }
        Output::info("");
        Output::info("Run 'hardline clean' to remove these sessions");
        return Ok(CleanOutput::dry_run(stale_sessions));
    }

    // 5. Report stale sessions in verbose mode
    if options.verbose {
        Output::info(&format!("Found {} stale session(s):", stale_sessions.len()));
        for name in &stale_sessions {
            Output::info(&format!("  - {name}"));
        }
    }

    // 6. Prune stale worktrees
    let removed_count = prune_stale_worktrees(backend.as_ref())?;

    // 7. Report results
    Output::success(&format!("Removed {removed_count} stale session(s)"));
    if options.verbose {
        for name in &stale_sessions {
            Output::info(&format!("  - {name}"));
        }
    }

    Ok(CleanOutput::cleaned(removed_count, stale_sessions))
}

/// A parsed worktree entry from `git worktree list`.
struct WorktreeEntry {
    /// Display name derived from the worktree path.
    name: String,
    /// Absolute path to the worktree directory.
    path: String,
    /// Whether this is the main/primary worktree.
    is_main: bool,
}

/// List all git worktrees in the current repository.
///
/// Parses the output of `git worktree list --porcelain` to extract
/// worktree paths and identifiers.
///
/// # Errors
///
/// Returns an error if the git command fails.
fn list_worktrees(
    _backend: &dyn scp_core::vcs::VcsBackend,
    cwd: &Path,
) -> Result<Vec<WorktreeEntry>> {
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(cwd)
        .output()
        .map_err(|e| Error::io_error(format!("Failed to run git worktree list: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::io_error(format!(
            "git worktree list failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_worktree_porcelain(&stdout))
}

/// Parse `git worktree list --porcelain` output into structured entries.
///
/// The porcelain format is:
/// ```text
/// worktree /path/to/worktree
/// HEAD abc123...
/// branch refs/heads/branch-name
///
/// worktree /path/to/another
/// ...
/// ```
fn parse_worktree_porcelain(output: &str) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut is_main = false;

    for line in output.lines() {
        if line.starts_with("worktree ") {
            // Save previous entry if any
            if let Some(path) = current_path.take() {
                let name = current_branch
                    .as_deref()
                    .unwrap_or_else(|| path.as_str())
                    .to_string();
                entries.push(WorktreeEntry {
                    name,
                    path,
                    is_main,
                });
            }
            current_path = Some(line.strip_prefix("worktree ").unwrap_or("").to_string());
            current_branch = None;
            is_main = false;
        } else if line.starts_with("branch ") {
            let branch = line.strip_prefix("branch ").unwrap_or("");
            // Main worktree typically has refs/heads/main or refs/heads/master
            current_branch = branch
                .strip_prefix("refs/heads/")
                .unwrap_or(branch)
                .to_string()
                .into();
            is_main = branch == "refs/heads/main" || branch == "refs/heads/master";
        }
    }

    // Don't forget the last entry
    if let Some(path) = current_path {
        let name = current_branch
            .as_deref()
            .unwrap_or_else(|| path.as_str())
            .to_string();
        entries.push(WorktreeEntry {
            name,
            path,
            is_main,
        });
    }

    entries
}

/// Prune stale git worktree references.
///
/// Runs `git worktree prune` which removes worktree administrative
/// files for worktrees whose directories no longer exist.
///
/// # Errors
///
/// Returns an error if the git command fails.
fn prune_stale_worktrees(_backend: &dyn scp_core::vcs::VcsBackend) -> Result<usize> {
    let cwd = std::env::current_dir()
        .map_err(|e| Error::io_error(format!("Failed to determine current directory: {e}")))?;

    let output = std::process::Command::new("git")
        .args(["worktree", "prune", "--verbose"])
        .current_dir(&cwd)
        .output()
        .map_err(|e| Error::io_error(format!("Failed to run git worktree prune: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::io_error(format!(
            "git worktree prune failed: {stderr}"
        )));
    }

    // Count pruned worktrees from verbose output (each line is a removed entry)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_worktree_porcelain ----

    #[test]
    fn parse_empty_output_returns_no_entries() {
        let entries = parse_worktree_porcelain("");
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_single_main_worktree() {
        let input = "worktree /home/user/repo\nHEAD abc123\nbranch refs/heads/main\n";
        let entries = parse_worktree_porcelain(input);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "main");
        assert_eq!(entries[0].path, "/home/user/repo");
        assert!(entries[0].is_main);
    }

    #[test]
    fn parse_multiple_worktrees() {
        let input = "\
worktree /home/user/repo
HEAD abc123
branch refs/heads/main

worktree /home/user/repo-feature-auth
HEAD def456
branch refs/heads/feature-auth
";
        let entries = parse_worktree_porcelain(input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "main");
        assert!(entries[0].is_main);
        assert_eq!(entries[1].name, "feature-auth");
        assert!(!entries[1].is_main);
    }

    #[test]
    fn parse_worktree_without_branch_uses_path_as_name() {
        let input = "worktree /home/user/repo-extra\nHEAD abc123\n";
        let entries = parse_worktree_porcelain(input);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "/home/user/repo-extra");
        assert!(!entries[0].is_main);
    }

    #[test]
    fn parse_detached_head_worktree() {
        let input = "\
worktree /home/user/repo
HEAD abc123
branch refs/heads/main

worktree /home/user/repo-detached
HEAD def456
";
        let entries = parse_worktree_porcelain(input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "main");
        assert_eq!(entries[1].name, "/home/user/repo-detached");
    }

    #[test]
    fn parse_master_branch_is_main() {
        let input = "worktree /home/user/repo\nHEAD abc123\nbranch refs/heads/master\n";
        let entries = parse_worktree_porcelain(input);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_main);
    }

    // ---- CleanOutput helpers ----

    #[test]
    fn clean_output_no_stale_has_zero_counts() {
        let output = CleanOutput::no_stale();
        assert_eq!(output.stale_count, 0);
        assert_eq!(output.removed_count, 0);
    }

    #[test]
    fn clean_output_dry_run_preserves_names() {
        let names = vec!["stale-1".to_string(), "stale-2".to_string()];
        let output = CleanOutput::dry_run(names.clone());
        assert_eq!(output.stale_count, 2);
        assert_eq!(output.removed_count, 0);
        assert_eq!(output.stale_sessions, names);
    }

    #[test]
    fn clean_output_cleaned_reports_counts() {
        let output =
            CleanOutput::cleaned(3, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(output.removed_count, 3);
        assert_eq!(output.stale_count, 3);
    }
}
