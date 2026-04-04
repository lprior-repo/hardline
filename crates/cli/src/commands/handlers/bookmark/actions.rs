//! Action functions for the bookmark command handler (Tier 3).
//!
//! I/O operations for bookmark management (create, list, delete, track).
//! Delegates to Git commands via the shell and uses `scp_core::Output` for
//! user-facing messages.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{
    validate_bookmark_name, BookmarkCreateOutput, BookmarkDeleteOutput, BookmarkInfo,
    BookmarkListOutput, BookmarkOptions, BookmarkOutput, BookmarkSubcommand,
    BookmarkTrackOutput,
};

/// Execute the bookmark command with the given options.
///
/// # Errors
///
/// Returns errors for:
/// - Invalid bookmark names (`validation_error`)
/// - Bookmark not found (`not_found`)
/// - Bookmark already exists (`invalid_state`)
/// - Git command failures (`io_error`)
pub fn run_bookmark(options: &BookmarkOptions) -> Result<BookmarkOutput> {
    match &options.subcommand {
        BookmarkSubcommand::Create { name, push } => run_create(name, *push),
        BookmarkSubcommand::List { show_all } => run_list(*show_all),
        BookmarkSubcommand::Delete { name } => run_delete(name),
        BookmarkSubcommand::Track { name, remote } => run_track(name, remote.as_deref()),
    }
}

/// Create a new bookmark at the current revision.
fn run_create(name: &str, push: bool) -> Result<BookmarkOutput> {
    if !validate_bookmark_name(name) {
        return Err(Error::validation_error(format!(
            "Invalid bookmark name '{name}': must be alphanumeric, underscore, or hyphen"
        )));
    }

    let git_args = build_create_args(name);

    let output = std::process::Command::new("git")
        .args(&git_args)
        .output()
        .map_err(|e| Error::io_error(format!("Failed to execute git branch: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already exists") {
            return Err(Error::invalid_state(format!(
                "Bookmark '{name}' already exists"
            )));
        }
        return Err(Error::io_error(format!(
            "git branch create failed: {stderr}"
        )));
    }

    // Get current revision
    let revision = get_current_revision()?;

    // Push to remote if requested
    if push {
        push_bookmark(name)?;
    }

    Output::success(&format!(
        "Created bookmark '{}' at revision {}",
        name, revision
    ));
    if push {
        Output::info("Pushed to remote");
    }

    Ok(BookmarkOutput::Create(BookmarkCreateOutput {
        name: name.to_string(),
        revision,
        pushed: push,
    }))
}

/// List bookmarks.
fn run_list(show_all: bool) -> Result<BookmarkOutput> {
    let mut git_args = vec!["branch".to_string()];
    if show_all {
        git_args.push("--all".to_string());
    }

    let output = std::process::Command::new("git")
        .args(&git_args)
        .output()
        .map_err(|e| Error::io_error(format!("Failed to execute git branch: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If not in a git repo, return empty list
        if stderr.contains("not a git repository") {
            let list_output = BookmarkListOutput {
                bookmarks: vec![],
                count: 0,
            };
            Output::info("No bookmarks found.");
            return Ok(BookmarkOutput::List(list_output));
        }
        return Err(Error::io_error(format!(
            "git branch list failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bookmarks = parse_branch_list(&stdout);

    if bookmarks.is_empty() {
        Output::info("No bookmarks found.");
    } else {
        Output::info(&format!("Bookmarks ({}):", bookmarks.len()));
        for bookmark in &bookmarks {
            let remote_marker = if bookmark.remote { " (remote)" } else { "" };
            Output::info(&format!(
                "  {} -> {}{}",
                bookmark.name, bookmark.revision, remote_marker
            ));
        }
    }

    let count = bookmarks.len();
    Ok(BookmarkOutput::List(BookmarkListOutput {
        bookmarks,
        count,
    }))
}

/// Delete a bookmark.
fn run_delete(name: &str) -> Result<BookmarkOutput> {
    if !validate_bookmark_name(name) {
        return Err(Error::validation_error(format!(
            "Invalid bookmark name '{name}': must be alphanumeric, underscore, or hyphen"
        )));
    }

    // Check the bookmark exists before attempting to delete
    let existing = list_branches(false)?;
    if !existing.iter().any(|b| b.name == name) {
        return Err(Error::not_found(format!(
            "Bookmark '{name}' not found"
        )));
    }

    let output = std::process::Command::new("git")
        .args(["branch", "-d", name])
        .output()
        .map_err(|e| Error::io_error(format!("Failed to execute git branch -d: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::io_error(format!(
            "git branch delete failed: {stderr}"
        )));
    }

    Output::success(&format!("Deleted bookmark '{}'", name));

    Ok(BookmarkOutput::Delete(BookmarkDeleteOutput {
        name: name.to_string(),
    }))
}

/// Track a remote bookmark (set upstream).
fn run_track(name: &str, remote: Option<&str>) -> Result<BookmarkOutput> {
    if !validate_bookmark_name(name) {
        return Err(Error::validation_error(format!(
            "Invalid bookmark name '{name}': must be alphanumeric, underscore, or hyphen"
        )));
    }

    let remote_name = remote.unwrap_or("origin");

    let output = std::process::Command::new("git")
        .args([
            "branch",
            "--set-upstream-to",
            &format!("{remote_name}/{name}"),
            name,
        ])
        .output()
        .map_err(|e| Error::io_error(format!("Failed to execute git branch --set-upstream-to: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("does not exist") || stderr.contains("No upstream") {
            return Err(Error::not_found(format!(
                "Remote bookmark '{remote_name}/{name}' not found"
            )));
        }
        return Err(Error::io_error(format!(
            "git branch track failed: {stderr}"
        )));
    }

    Output::success(&format!(
        "Bookmark '{}' now tracking '{}/{}'",
        name, remote_name, name
    ));

    Ok(BookmarkOutput::Track(BookmarkTrackOutput {
        name: name.to_string(),
        remote: remote_name.to_string(),
    }))
}

// ============================================================================
// Private Helpers
// ============================================================================

/// Build git args for creating a branch.
fn build_create_args(name: &str) -> Vec<String> {
    vec!["branch".to_string(), name.to_string()]
}

/// Get the current revision (commit hash).
fn get_current_revision() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map_err(|e| Error::io_error(format!("Failed to get current revision: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::io_error(format!(
            "git rev-parse failed: {stderr}"
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Push a bookmark to the default remote.
fn push_bookmark(name: &str) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["push", "-u", "origin", name])
        .output()
        .map_err(|e| Error::io_error(format!("Failed to push bookmark: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::io_error(format!("git push failed: {stderr}")));
    }

    Ok(())
}

/// List branches by running `git branch`.
fn list_branches(show_all: bool) -> Result<Vec<BookmarkInfo>> {
    let mut args = vec!["branch"];
    if show_all {
        args.push("--all");
    }

    let output = std::process::Command::new("git")
        .args(&args)
        .output()
        .map_err(|e| Error::io_error(format!("Failed to list branches: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Ok(vec![]);
        }
        return Err(Error::io_error(format!(
            "git branch list failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_branch_list(&stdout))
}

/// Parse `git branch` output into `BookmarkInfo` structs.
///
/// Handles format:
/// - `  branch_name` (non-active)
/// - `* branch_name` (active)
/// - `  remotes/origin/branch_name` (with --all)
fn parse_branch_list(output: &str) -> Vec<BookmarkInfo> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }

            // Remove the active marker '* '
            let name = trimmed.strip_prefix("* ").unwrap_or(trimmed);

            // Skip remote tracking branches for display but mark as remote
            let is_remote = name.starts_with("remotes/");
            let display_name = if is_remote {
                name.strip_prefix("remotes/")?
            } else {
                name
            };

            Some(BookmarkInfo {
                name: display_name.to_string(),
                revision: String::new(), // Revision not available from branch listing
                remote: is_remote,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::bookmark::data::BookmarkSubcommand;

    // -- parse_branch_list (pure, no I/O) --

    #[test]
    fn parse_branch_list_empty() {
        let result = parse_branch_list("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_branch_list_single_branch() {
        let output = "  main\n";
        let result = parse_branch_list(output);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "main");
        assert!(!result[0].remote);
    }

    #[test]
    fn parse_branch_list_active_branch() {
        let output = "* feature-auth\n";
        let result = parse_branch_list(output);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "feature-auth");
        assert!(!result[0].remote);
    }

    #[test]
    fn parse_branch_list_multiple_branches() {
        let output = "  bugfix-123\n* main\n  feature\n";
        let result = parse_branch_list(output);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "bugfix-123");
        assert_eq!(result[1].name, "main");
        assert_eq!(result[2].name, "feature");
    }

    #[test]
    fn parse_branch_list_with_remotes() {
        let output = "  main\n  remotes/origin/main\n";
        let result = parse_branch_list(output);
        assert_eq!(result.len(), 2);
        assert!(!result[0].remote);
        assert!(result[1].remote);
        assert_eq!(result[1].name, "origin/main");
    }

    // -- build_create_args --

    #[test]
    fn build_create_args_contains_name() {
        let args = build_create_args("feature-auth");
        assert_eq!(args, vec!["branch", "feature-auth"]);
    }

    // -- validate_bookmark_name integration --

    #[test]
    fn validate_rejects_empty_name() {
        assert!(!validate_bookmark_name(""));
    }

    #[test]
    fn validate_accepts_standard_name() {
        assert!(validate_bookmark_name("feature-auth"));
    }

    // -- run_bookmark validation gating --

    #[test]
    fn run_bookmark_create_rejects_invalid_name() {
        let opts = BookmarkOptions {
            subcommand: BookmarkSubcommand::Create {
                name: "".to_string(),
                push: false,
            },
        };
        let result = run_bookmark(&opts);
        assert!(result.is_err());
        let err_msg = result.err().map_or(String::new(), |e| e.to_string());
        assert!(
            err_msg.contains("Invalid bookmark name"),
            "Expected validation error, got: {err_msg}"
        );
    }

    #[test]
    fn run_bookmark_delete_rejects_invalid_name() {
        let opts = BookmarkOptions {
            subcommand: BookmarkSubcommand::Delete {
                name: "bad name!".to_string(),
            },
        };
        let result = run_bookmark(&opts);
        assert!(result.is_err());
        let err_msg = result.err().map_or(String::new(), |e| e.to_string());
        assert!(
            err_msg.contains("Invalid bookmark name"),
            "Expected validation error, got: {err_msg}"
        );
    }

    #[test]
    fn run_bookmark_track_rejects_invalid_name() {
        let opts = BookmarkOptions {
            subcommand: BookmarkSubcommand::Track {
                name: "".to_string(),
                remote: None,
            },
        };
        let result = run_bookmark(&opts);
        assert!(result.is_err());
        let err_msg = result.err().map_or(String::new(), |e| e.to_string());
        assert!(
            err_msg.contains("Invalid bookmark name"),
            "Expected validation error, got: {err_msg}"
        );
    }

    // -- BookmarkOutput variants from non-I/O paths --

    #[test]
    fn bookmark_output_create_equality() {
        let output = BookmarkCreateOutput {
            name: "test".to_string(),
            revision: "abc123".to_string(),
            pushed: false,
        };
        assert_eq!(output.name, "test");
        assert!(!output.pushed);
    }

    #[test]
    fn bookmark_output_delete_equality() {
        let output = BookmarkDeleteOutput {
            name: "old".to_string(),
        };
        assert_eq!(output.name, "old");
    }

    #[test]
    fn bookmark_output_track_equality() {
        let output = BookmarkTrackOutput {
            name: "main".to_string(),
            remote: "origin".to_string(),
        };
        assert_eq!(output.remote, "origin");
    }

    #[test]
    fn bookmark_output_list_equality() {
        let output = BookmarkListOutput {
            bookmarks: vec![],
            count: 0,
        };
        assert_eq!(output.count, 0);
    }

    // -- Error factory methods produce correct messages --

    #[test]
    fn validation_error_message() {
        let err = Error::validation_error("bad name");
        let msg = err.to_string();
        assert!(msg.contains("bad name"), "Got: {msg}");
    }

    #[test]
    fn not_found_error_message() {
        let err = Error::not_found("bookmark 'x'");
        let msg = err.to_string();
        assert!(msg.contains("bookmark 'x'"), "Got: {msg}");
    }

    #[test]
    fn invalid_state_error_message() {
        let err = Error::invalid_state("already exists");
        let msg = err.to_string();
        assert!(msg.contains("already exists"), "Got: {msg}");
    }

    #[test]
    fn io_error_message() {
        let err = Error::io_error("disk full");
        let msg = err.to_string();
        assert!(msg.contains("disk full"), "Got: {msg}");
    }
}
