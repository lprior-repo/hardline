//! Action functions for the clean command handler (Tier 3).
//!
//! I/O operations that detect and remove stale workspace sessions.
//! A workspace is considered stale when its git worktree directory
//! no longer exists on disk but the worktree reference remains.

use std::path::Path;

use scp_core::{output::Output, Error, Result};

use super::data::{CleanOptions, CleanOutput};

/// Execute the clean command with the given options.
///
/// # Errors
///
/// Returns errors if the current directory is not a git repository,
/// or if git commands fail during worktree enumeration or pruning.
pub fn run_clean(options: &CleanOptions) -> Result<CleanOutput> {
    let cwd = std::env::current_dir()
        .map_err(|e| Error::io_error(format!("Failed to determine current directory: {e}")))?;

    let _backend = scp_core::vcs::create_backend(&cwd)?;
    let worktree_entries = list_worktrees(&cwd)?;
    let stale_sessions = detect_stale_worktrees(&worktree_entries);

    if stale_sessions.is_empty() {
        Output::success("No stale sessions found");
        Output::info("  All sessions have valid workspaces");
        return Ok(CleanOutput::no_stale());
    }

    if options.dry_run {
        report_dry_run(&stale_sessions);
        return Ok(CleanOutput::dry_run(stale_sessions));
    }

    report_verbose_stale(options.verbose, &stale_sessions);
    let removed_count = execute_prune()?;
    report_results(removed_count, options.verbose, &stale_sessions);
    Ok(CleanOutput::cleaned(removed_count, stale_sessions))
}

/// Filter worktree entries to find those whose directories no longer exist.
fn detect_stale_worktrees(entries: &[WorktreeEntry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| !entry.is_main && !Path::new(&entry.path).exists())
        .map(|entry| entry.name.clone())
        .collect()
}

/// Report dry-run results without making changes.
fn report_dry_run(stale_sessions: &[String]) {
    Output::info(&format!(
        "Found {} stale session(s) (dry-run, no changes made):",
        stale_sessions.len()
    ));
    for name in stale_sessions {
        Output::info(&format!("  - {name}"));
    }
    Output::info("");
    Output::info("Run 'hardline clean' to remove these sessions");
}

/// Report stale sessions when verbose mode is enabled.
fn report_verbose_stale(verbose: bool, stale_sessions: &[String]) {
    if verbose {
        Output::info(&format!("Found {} stale session(s):", stale_sessions.len()));
        for name in stale_sessions {
            Output::info(&format!("  - {name}"));
        }
    }
}

/// Execute `git worktree prune` and return the number pruned.
///
/// # Errors
///
/// Returns an error if the git command fails.
fn execute_prune() -> Result<usize> {
    let output = std::process::Command::new("git")
        .args(["worktree", "prune", "--verbose"])
        .output()
        .map_err(|e| Error::io_error(format!("Failed to run git worktree prune: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::io_error(format!(
            "git worktree prune failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(count_non_empty_lines(&stdout))
}

/// Count non-empty lines in a string.
fn count_non_empty_lines(output: &str) -> usize {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

/// Report final cleanup results.
fn report_results(removed_count: usize, verbose: bool, stale_sessions: &[String]) {
    Output::success(&format!("Removed {removed_count} stale session(s)"));
    if verbose {
        for name in stale_sessions {
            Output::info(&format!("  - {name}"));
        }
    }
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
/// # Errors
///
/// Returns an error if the git command fails.
fn list_worktrees(cwd: &Path) -> Result<Vec<WorktreeEntry>> {
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

/// Accumulator for parsing a single worktree entry from porcelain output.
struct PartialEntry {
    path: Option<String>,
    branch: Option<String>,
    is_main: bool,
}

impl PartialEntry {
    const fn new() -> Self {
        Self {
            path: None,
            branch: None,
            is_main: false,
        }
    }

    /// Convert into a `WorktreeEntry` if a path was recorded.
    fn into_entry(self) -> Option<WorktreeEntry> {
        self.path.map(|path| {
            let name = self.branch.as_deref().unwrap_or(path.as_str()).to_string();
            WorktreeEntry {
                name,
                path,
                is_main: self.is_main,
            }
        })
    }
}

/// Parse `git worktree list --porcelain` output into structured entries.
fn parse_worktree_porcelain(output: &str) -> Vec<WorktreeEntry> {
    let (entries, partial) = output.lines().fold(
        (Vec::new(), PartialEntry::new()),
        |(entries, partial), line| apply_porcelain_line(entries, partial, line),
    );
    finalize_entries(entries, partial)
}

/// Process a single line of porcelain output, returning updated accumulator.
fn apply_porcelain_line(
    entries: Vec<WorktreeEntry>,
    partial: PartialEntry,
    line: &str,
) -> (Vec<WorktreeEntry>, PartialEntry) {
    if let Some(path) = line.strip_prefix("worktree ") {
        let updated = flush_partial(entries, partial);
        let new_partial = PartialEntry {
            path: Some(path.to_string()),
            branch: None,
            is_main: false,
        };
        (updated, new_partial)
    } else if let Some(branch) = line.strip_prefix("branch ") {
        let display = branch
            .strip_prefix("refs/heads/")
            .unwrap_or(branch)
            .to_string();
        let is_main = branch == "refs/heads/main" || branch == "refs/heads/master";
        let new_partial = PartialEntry {
            path: partial.path,
            branch: Some(display),
            is_main,
        };
        (entries, new_partial)
    } else {
        (entries, partial)
    }
}

/// Append a completed partial entry to the entries list, if present.
fn flush_partial(entries: Vec<WorktreeEntry>, partial: PartialEntry) -> Vec<WorktreeEntry> {
    match partial.into_entry() {
        Some(entry) => entries.into_iter().chain(std::iter::once(entry)).collect(),
        None => entries,
    }
}

/// Flush the final partial entry into the entries list.
fn finalize_entries(entries: Vec<WorktreeEntry>, partial: PartialEntry) -> Vec<WorktreeEntry> {
    match partial.into_entry() {
        Some(entry) => entries.into_iter().chain(std::iter::once(entry)).collect(),
        None => entries,
    }
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

    // ---- count_non_empty_lines ----

    #[test]
    fn count_non_empty_lines_counts_real_lines() {
        assert_eq!(count_non_empty_lines("a\nb\nc\n"), 3);
    }

    #[test]
    fn count_non_empty_lines_skips_blanks() {
        assert_eq!(count_non_empty_lines("a\n\n  \nb\n"), 2);
    }

    #[test]
    fn count_non_empty_lines_empty_string() {
        assert_eq!(count_non_empty_lines(""), 0);
    }

    // ---- detect_stale_worktrees ----

    #[test]
    fn detect_stale_skips_main_worktree() {
        let entries = vec![WorktreeEntry {
            name: "main".to_string(),
            path: "/nonexistent/path".to_string(),
            is_main: true,
        }];
        // Main worktree is always skipped regardless of path existence
        let stale = detect_stale_worktrees(&entries);
        assert!(stale.is_empty());
    }
}
