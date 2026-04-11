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
    fn new() -> Self {
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

    #[test]
    fn detect_stale_finds_non_main_with_nonexistent_path() {
        let entries = vec![WorktreeEntry {
            name: "feature-x".to_string(),
            path: "/tmp/absolutely-does-not-exist-xyz".to_string(),
            is_main: false,
        }];
        let stale = detect_stale_worktrees(&entries);
        assert_eq!(stale, vec!["feature-x".to_string()]);
    }

    #[test]
    fn detect_stale_skips_non_main_with_existing_path() {
        let entries = vec![WorktreeEntry {
            name: "active-ws".to_string(),
            path: "/tmp".to_string(), // /tmp always exists
            is_main: false,
        }];
        let stale = detect_stale_worktrees(&entries);
        assert!(stale.is_empty());
    }

    #[test]
    fn detect_stale_mixed_entries() {
        let entries = vec![
            WorktreeEntry {
                name: "main".to_string(),
                path: "/nonexistent/main".to_string(),
                is_main: true,
            },
            WorktreeEntry {
                name: "active".to_string(),
                path: "/tmp".to_string(),
                is_main: false,
            },
            WorktreeEntry {
                name: "stale-1".to_string(),
                path: "/nonexistent/stale1".to_string(),
                is_main: false,
            },
        ];
        let stale = detect_stale_worktrees(&entries);
        assert_eq!(stale, vec!["stale-1".to_string()]);
    }

    #[test]
    fn detect_stale_empty_entries() {
        let stale = detect_stale_worktrees(&[]);
        assert!(stale.is_empty());
    }

    #[test]
    fn detect_stale_multiple_stale() {
        let entries = vec![
            WorktreeEntry {
                name: "gone-1".to_string(),
                path: "/nonexistent/gone1".to_string(),
                is_main: false,
            },
            WorktreeEntry {
                name: "gone-2".to_string(),
                path: "/nonexistent/gone2".to_string(),
                is_main: false,
            },
        ];
        let stale = detect_stale_worktrees(&entries);
        assert_eq!(stale.len(), 2);
        assert!(stale.contains(&"gone-1".to_string()));
        assert!(stale.contains(&"gone-2".to_string()));
    }

    // ---- PartialEntry ----

    #[test]
    fn partial_entry_new_has_no_fields() {
        let p = PartialEntry::new();
        assert!(p.path.is_none());
        assert!(p.branch.is_none());
        assert!(!p.is_main);
    }

    #[test]
    fn partial_entry_into_entry_without_path_returns_none() {
        let p = PartialEntry::new();
        assert!(p.into_entry().is_none());
    }

    #[test]
    fn partial_entry_into_entry_with_path_uses_branch_name() {
        let p = PartialEntry {
            path: Some("/some/path".to_string()),
            branch: Some("feature-branch".to_string()),
            is_main: false,
        };
        let entry = p.into_entry().expect("should produce entry");
        assert_eq!(entry.name, "feature-branch");
        assert_eq!(entry.path, "/some/path");
        assert!(!entry.is_main);
    }

    #[test]
    fn partial_entry_into_entry_without_branch_uses_path_as_name() {
        let p = PartialEntry {
            path: Some("/some/path".to_string()),
            branch: None,
            is_main: false,
        };
        let entry = p.into_entry().expect("should produce entry");
        assert_eq!(entry.name, "/some/path");
    }

    // ---- apply_porcelain_line ----

    #[test]
    fn apply_porcelain_line_worktree_starts_new_entry() {
        let entries = vec![];
        let partial = PartialEntry::new();
        let (entries, partial) =
            apply_porcelain_line(entries, partial, "worktree /home/user/repo");
        assert!(entries.is_empty());
        assert_eq!(partial.path, Some("/home/user/repo".to_string()));
    }

    #[test]
    fn apply_porcelain_line_branch_updates_partial() {
        let entries = vec![];
        let partial = PartialEntry {
            path: Some("/repo".to_string()),
            branch: None,
            is_main: false,
        };
        let (entries, new_partial) =
            apply_porcelain_line(entries, partial, "branch refs/heads/feature");
        assert!(entries.is_empty());
        assert_eq!(new_partial.branch, Some("feature".to_string()));
        assert!(!new_partial.is_main);
    }

    #[test]
    fn apply_porcelain_line_branch_main_sets_is_main() {
        let entries = vec![];
        let partial = PartialEntry {
            path: Some("/repo".to_string()),
            branch: None,
            is_main: false,
        };
        let (_, partial) =
            apply_porcelain_line(entries, partial, "branch refs/heads/main");
        assert!(partial.is_main);
    }

    #[test]
    fn apply_porcelain_line_unknown_line_passes_through() {
        let entries = vec![];
        let partial = PartialEntry {
            path: Some("/repo".to_string()),
            branch: None,
            is_main: false,
        };
        let (e, p) = apply_porcelain_line(entries, partial, "HEAD abc123def");
        assert!(e.is_empty());
        assert_eq!(p.path, Some("/repo".to_string()));
    }

    #[test]
    fn apply_porcelain_line_worktree_flushes_previous() {
        let partial = PartialEntry {
            path: Some("/first".to_string()),
            branch: Some("first-branch".to_string()),
            is_main: false,
        };
        let (entries, new_partial) =
            apply_porcelain_line(vec![], partial, "worktree /second");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "first-branch");
        assert_eq!(new_partial.path, Some("/second".to_string()));
    }

    // ---- parse_worktree_porcelain edge cases ----

    #[test]
    fn parse_porcelain_with_blank_lines_between_entries() {
        let input = "\
worktree /repo
HEAD abc
branch refs/heads/main

worktree /repo-ws
HEAD def
branch refs/heads/ws

";
        let entries = parse_worktree_porcelain(input);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn parse_porcelain_single_worktree_no_trailing_newline() {
        let input = "worktree /repo\nHEAD abc\nbranch refs/heads/main";
        let entries = parse_worktree_porcelain(input);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "main");
    }

    #[test]
    fn parse_porcelain_three_worktrees() {
        let input = "\
worktree /main-repo
HEAD aaaa
branch refs/heads/main

worktree /repo-ws1
HEAD bbbb
branch refs/heads/ws1

worktree /repo-ws2
HEAD cccc
branch refs/heads/ws2
";
        let entries = parse_worktree_porcelain(input);
        assert_eq!(entries.len(), 3);
        assert!(entries[0].is_main);
        assert!(!entries[1].is_main);
        assert!(!entries[2].is_main);
    }

    // ---- count_non_empty_lines edge cases ----

    #[test]
    fn count_non_empty_lines_only_whitespace() {
        assert_eq!(count_non_empty_lines("   \n\t\n  \n"), 0);
    }

    #[test]
    fn count_non_empty_lines_trailing_newline() {
        assert_eq!(count_non_empty_lines("line\n"), 1);
    }

    #[test]
    fn count_non_empty_lines_no_trailing_newline() {
        assert_eq!(count_non_empty_lines("line"), 1);
    }

    // ---- flush_partial / finalize_entries ----

    #[test]
    fn flush_partial_empty_returns_empty() {
        let entries = flush_partial(vec![], PartialEntry::new());
        assert!(entries.is_empty());
    }

    #[test]
    fn finalize_entries_empty_returns_empty() {
        let entries = finalize_entries(vec![], PartialEntry::new());
        assert!(entries.is_empty());
    }

    #[test]
    fn finalize_entries_flushes_last_partial() {
        let partial = PartialEntry {
            path: Some("/last".to_string()),
            branch: Some("last-branch".to_string()),
            is_main: false,
        };
        let entries = finalize_entries(vec![], partial);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "last-branch");
    }
}
