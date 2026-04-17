//! Action functions for the bookmark command handler (Tier 3).
//!
//! I/O operations for bookmark management (create, list, delete, track).
//! Delegates to Git commands via the shell and uses `scp_core::Output` for
//! user-facing messages.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{
    parse_branch_list, validate_bookmark_name, BookmarkCreateOutput, BookmarkDeleteOutput,
    BookmarkInfo, BookmarkListOutput, BookmarkOptions, BookmarkOutput, BookmarkSubcommand,
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
    validate_name(name)?;

    create_bookmark(name)?;

    let revision = get_current_revision()?;

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
    let bookmarks = list_bookmarks(show_all)?;
    format_bookmark_list(&bookmarks);

    let count = bookmarks.len();
    Ok(BookmarkOutput::List(BookmarkListOutput {
        bookmarks,
        count,
    }))
}

/// Delete a bookmark.
fn run_delete(name: &str) -> Result<BookmarkOutput> {
    validate_name(name)?;

    let existing = list_branches(false)?;
    if !existing.iter().any(|b| b.name == name) {
        return Err(Error::not_found(format!(
            "Bookmark '{name}' not found"
        )));
    }

    delete_bookmark(name)?;

    Output::success(&format!("Deleted bookmark '{}'", name));

    Ok(BookmarkOutput::Delete(BookmarkDeleteOutput {
        name: name.to_string(),
    }))
}

/// Track a remote bookmark (set upstream).
fn run_track(name: &str, remote: Option<&str>) -> Result<BookmarkOutput> {
    validate_name(name)?;

    let remote_name = remote.map_or("origin", |r| r);

    set_upstream(name, remote_name)?;

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
// Private Helpers - I/O Operations
// ============================================================================

/// Validate a bookmark name, returning an error if invalid.
fn validate_name(name: &str) -> Result<()> {
    if validate_bookmark_name(name) {
        Ok(())
    } else {
        Err(Error::validation_error(format!(
            "Invalid bookmark name '{name}': must be alphanumeric, underscore, or hyphen"
        )))
    }
}

/// Run `git branch <name>` to create a branch.
///
/// # Errors
///
/// Returns `invalid_state` if the branch already exists, or `io_error` on failure.
fn create_bookmark(name: &str) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["branch", name])
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

    Ok(())
}

/// Run `git branch -d <name>` to delete a branch.
///
/// # Errors
///
/// Returns `io_error` on git failure.
fn delete_bookmark(name: &str) -> Result<()> {
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

    Ok(())
}

/// Run `git branch --set-upstream-to <remote>/<name> <name>`.
///
/// # Errors
///
/// Returns `not_found` if the remote branch does not exist, or `io_error` on failure.
fn set_upstream(name: &str, remote_name: &str) -> Result<()> {
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
        return handle_upstream_error(&output.stderr, remote_name, name);
    }

    Ok(())
}

/// Handle the stderr from a failed `git branch --set-upstream-to`.
///
/// Intentionally uses string matching on stderr to classify errors.
/// This is fragile but unavoidable without a Git library.
fn handle_upstream_error(
    stderr: &[u8],
    remote_name: &str,
    name: &str,
) -> Result<()> {
    let stderr = String::from_utf8_lossy(stderr);
    if stderr.contains("does not exist") || stderr.contains("No upstream") {
        return Err(Error::not_found(format!(
            "Remote bookmark '{remote_name}/{name}' not found"
        )));
    }
    Err(Error::io_error(format!(
        "git branch track failed: {stderr}"
    )))
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

/// List branches by running `git branch`.
///
/// Intentionally uses string matching on stderr to detect "not a git repository".
/// This is fragile but unavoidable without a Git library.
fn list_branches(show_all: bool) -> Result<Vec<BookmarkInfo>> {
    let args = build_list_args(show_all);

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

/// List bookmarks (alias for `list_branches` that is semantically named).
fn list_bookmarks(show_all: bool) -> Result<Vec<BookmarkInfo>> {
    list_branches(show_all)
}

// ============================================================================
// Pure Helpers
// ============================================================================

/// Build git args for listing branches.
fn build_list_args(show_all: bool) -> Vec<&'static str> {
    if show_all {
        vec!["branch", "--all"]
    } else {
        vec!["branch"]
    }
}

/// Format and print the bookmark list to the user.
fn format_bookmark_list(bookmarks: &[BookmarkInfo]) {
    if bookmarks.is_empty() {
        Output::info("No bookmarks found.");
    } else {
        Output::info(&format!("Bookmarks ({}):", bookmarks.len()));
        bookmarks.iter().for_each(|bookmark| {
            let remote_marker = if bookmark.remote { " (remote)" } else { "" };
            Output::info(&format!(
                "  {} -> {}{}",
                bookmark.name, bookmark.revision, remote_marker
            ));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::bookmark::data::BookmarkSubcommand;

    // -- validate_name --

    #[test]
    fn validate_rejects_empty_name() {
        assert!(!validate_bookmark_name(""));
    }

    #[test]
    fn validate_accepts_standard_name() {
        assert!(validate_bookmark_name("feature-auth"));
    }

    // -- build_list_args --

    #[test]
    fn build_list_args_default() {
        assert_eq!(build_list_args(false), vec!["branch"]);
    }

    #[test]
    fn build_list_args_show_all() {
        assert_eq!(build_list_args(true), vec!["branch", "--all"]);
    }

    // -- format_bookmark_list (does not panic) --

    #[test]
    fn format_bookmark_list_empty() {
        format_bookmark_list(&[]);
    }

    #[test]
    fn format_bookmark_list_with_entries() {
        let bookmarks = vec![
            BookmarkInfo {
                name: "main".to_string(),
                revision: "abc123".to_string(),
                remote: false,
            },
            BookmarkInfo {
                name: "origin/main".to_string(),
                revision: String::new(),
                remote: true,
            },
        ];
        format_bookmark_list(&bookmarks);
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
