//! Action functions for the undo command handler (Tier 3).
//!
//! I/O operations that orchestrate reverting the most recent session merge.
//! Uses `git reset --hard` (no JJ dependency).

use std::path::Path;

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{
    compute_undo_eligibility, Eligibility, UndoEntry, UndoHistoryEntry, UndoHistoryOutput,
    UndoMode, UndoOptions, UndoOutput, UndoStatus, WORKSPACE_RETENTION_SECONDS,
};

/// Path to the undo log relative to the project root.
const UNDO_LOG_PATH: &str = ".scp/undo.log";

// ============================================================================
// Public API
// ============================================================================

/// Execute the undo command with the given options.
///
/// # Errors
///
/// Returns errors for undo log read failures, no undo history,
/// already-pushed-to-remote, expired entries, or Git command failures.
pub fn run_undo(options: &UndoOptions) -> Result<UndoOutput> {
    match options.mode {
        UndoMode::ListHistory => run_list(),
        UndoMode::DryRun => run_execute(true),
        UndoMode::Execute => run_execute(false),
    }
}

// ============================================================================
// Execute Mode
// ============================================================================

/// Execute (or dry-run) the undo operation.
fn run_execute(is_dry_run: bool) -> Result<UndoOutput> {
    let history = read_undo_history()?;
    let entry = find_eligible_entry(&history)?;

    if is_dry_run {
        return format_dry_run_output(&entry);
    }

    execute_reset(&entry)?;
    update_undo_history(&history, &entry)?;

    format_undo_output(&entry)
}

/// Build dry-run output and print a preview.
fn format_dry_run_output(entry: &UndoEntry) -> Result<UndoOutput> {
    Output::info(&format!(
        "Dry-run: would undo session '{}'",
        entry.session_name
    ));
    Output::info(&format!("  Commit to undo: {}", entry.commit_id));
    Output::info(&format!(
        "  Would reset to: {}",
        entry.pre_merge_commit_id
    ));

    Ok(UndoOutput {
        session_name: entry.session_name.clone(),
        dry_run: true,
        commit_id: entry.commit_id.clone(),
        pushed_to_remote: false,
    })
}

/// Print confirmation output after a successful undo.
fn format_undo_output(entry: &UndoEntry) -> Result<UndoOutput> {
    Output::success(&format!(
        "Undone merge from session '{}'",
        entry.session_name
    ));
    Output::info(&format!("  Reset to commit: {}", entry.pre_merge_commit_id));
    Output::info("NEXT: Verify changes and re-commit if needed:");
    Output::info("  git status");
    Output::info(&format!(
        "  git commit -m 'Revert: {}'",
        entry.session_name
    ));

    Ok(UndoOutput {
        session_name: entry.session_name.clone(),
        dry_run: false,
        commit_id: entry.commit_id.clone(),
        pushed_to_remote: false,
    })
}

// ============================================================================
// List Mode
// ============================================================================

/// Display the undo history.
fn run_list() -> Result<UndoOutput> {
    let history = read_undo_history()?;

    if history.is_empty() {
        Output::info("No undo history available.");
        return Ok(UndoOutput::default());
    }

    let now_seconds = current_unix_seconds()?;
    let display_entries = build_history_entries(&history, now_seconds);
    let can_undo_any = display_entries.iter().any(|e| e.can_undo);

    let history_output = UndoHistoryOutput {
        total: display_entries.len(),
        can_undo: can_undo_any,
        entries: display_entries,
    };

    print_history(&history_output);

    Ok(UndoOutput::default())
}

/// Build display entries from raw undo entries and the current time.
fn build_history_entries(
    history: &[UndoEntry],
    now_seconds: u64,
) -> Vec<UndoHistoryEntry> {
    history
        .iter()
        .map(|entry| history_entry_from_undo(entry, now_seconds))
        .collect()
}

/// Convert a single `UndoEntry` to a display `UndoHistoryEntry`.
fn history_entry_from_undo(entry: &UndoEntry, now_seconds: u64) -> UndoHistoryEntry {
    let eligibility = compute_undo_eligibility(entry, now_seconds);

    let (can_undo, reason) = match eligibility {
        Eligibility::Eligible => (true, None),
        Eligibility::Ineligible { reason } => (false, Some(reason)),
    };

    let timestamp_str = format_timestamp(entry.timestamp);

    UndoHistoryEntry {
        session_name: entry.session_name.clone(),
        commit_id: entry.commit_id.clone(),
        timestamp: timestamp_str,
        status: entry.status.clone(),
        pushed_to_remote: entry.pushed_to_remote,
        can_undo,
        reason_cannot_undo: reason,
    }
}

/// Format a unix timestamp as a human-readable UTC string.
fn format_timestamp(timestamp: u64) -> String {
    chrono::DateTime::from_timestamp(i64::try_from(timestamp).map_or(0, |t| t), 0)
        .map_or_else(
            || timestamp.to_string(),
            |dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        )
}

/// Print history output in human-readable format.
fn print_history(output: &UndoHistoryOutput) {
    Output::info(&format!("Undo History ({} entries):", output.total));
    Output::info("");

    for (i, entry) in output.entries.iter().enumerate() {
        let indicator = if entry.can_undo { "[ok]" } else { "[x]" };
        let index = i + 1;

        print_single_entry(index, indicator, entry);
    }

    if output.can_undo {
        Output::info("Run 'scp undo' to revert the most recent undoable entry.");
    } else {
        Output::info("No entries can be undone.");
    }
}

/// Print a single history entry.
fn print_single_entry(index: usize, indicator: &str, entry: &UndoHistoryEntry) {
    Output::info(&format!(
        "{index}. {indicator} {} ({})",
        entry.session_name, entry.status
    ));
    Output::info(&format!("      Commit: {}", entry.commit_id));
    Output::info(&format!("      Time:   {}", entry.timestamp));

    if let Some(reason) = &entry.reason_cannot_undo {
        Output::info(&format!("      Cannot undo: {reason}"));
    }
    Output::info("");
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Parse non-empty lines into `UndoEntry` values.
fn parse_log_lines(content: &str) -> Result<Vec<UndoEntry>> {
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(idx, line)| {
            serde_json::from_str::<UndoEntry>(line).map_err(|e| {
                Error::io_error(format!(
                    "Failed to parse undo log entry at line {}: {e}",
                    idx + 1
                ))
            })
        })
        .collect::<Result<Vec<_>>>()
}

/// Read undo history from `.scp/undo.log` in most-recent-first order.
///
/// The log file stores entries in chronological order (oldest first).
/// Uses `.rev()` on the iterator to avoid `let mut`.
fn read_undo_history() -> Result<Vec<UndoEntry>> {
    let undo_log_path = Path::new(UNDO_LOG_PATH);

    if !undo_log_path.exists() {
        return Err(Error::not_found("No undo history found. Cannot undo."));
    }

    let content = std::fs::read_to_string(undo_log_path)
        .map_err(|e| Error::io_error(format!("Failed to read undo log: {e}")))?;

    let entries = parse_log_lines(&content)?;
    Ok(entries.into_iter().rev().collect())
}

/// Find the first eligible entry in the history for undo.
///
/// The history is assumed to be in most-recent-first order.
fn find_eligible_entry(history: &[UndoEntry]) -> Result<UndoEntry> {
    let now_seconds = current_unix_seconds()?;

    history
        .iter()
        .find(|entry| compute_undo_eligibility(entry, now_seconds).is_eligible())
        .cloned()
        .ok_or_else(|| Error::not_found("No eligible undo entries found."))
}

/// Execute `git reset --hard` to revert to the pre-merge commit.
fn execute_reset(entry: &UndoEntry) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["reset", "--hard", &entry.pre_merge_commit_id])
        .output()
        .map_err(|e| Error::io_error(format!("Failed to run git reset: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::invalid_state(format!(
            "Failed to revert merge: {stderr}"
        )));
    }

    Ok(())
}

/// Mark an entry as undone using functional construction.
///
/// Preserves the chronological order of entries in the log file
/// (writes back in the same order as received, with the matching
/// entry's status updated to `UndoStatus::Undone`).
fn mark_entry_undone(entry: &UndoEntry) -> UndoEntry {
    UndoEntry {
        session_name: entry.session_name.clone(),
        commit_id: entry.commit_id.clone(),
        pre_merge_commit_id: entry.pre_merge_commit_id.clone(),
        timestamp: entry.timestamp,
        pushed_to_remote: entry.pushed_to_remote,
        status: UndoStatus::Undone,
    }
}

/// Serialize history entries back to the log file format.
///
/// Reverses most-recent-first order back to chronological, marks
/// the matching entry as undone.
fn serialize_updated_history(
    history: &[UndoEntry],
    target: &UndoEntry,
) -> Result<String> {
    let lines: std::result::Result<Vec<_>, _> = history
        .iter()
        .rev()
        .map(|e| {
            let updated = if e.session_name == target.session_name
                && e.commit_id == target.commit_id
            {
                mark_entry_undone(e)
            } else {
                e.clone()
            };
            serde_json::to_string(&updated)
        })
        .collect();

    lines
        .map(|l| l.join("\n") + "\n")
        .map_err(|e| Error::io_error(format!("Failed to serialize undo entry: {e}")))
}

/// Update undo history after a successful undo.
///
/// Reverses the most-recent-first order back to chronological before writing.
fn update_undo_history(history: &[UndoEntry], entry: &UndoEntry) -> Result<()> {
    let undo_log_path = Path::new(UNDO_LOG_PATH);
    let new_content = serialize_updated_history(history, entry)?;

    std::fs::write(undo_log_path, &new_content)
        .map_err(|e| Error::io_error(format!("Failed to write undo log: {e}")))?;

    Ok(())
}

/// Get the current time as seconds since the Unix epoch.
fn current_unix_seconds() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|e| Error::io_error(format!("System time error: {e}")))
        .map(|d| d.as_secs())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- find_first_eligible_entry tests ----

    #[test]
    fn find_first_eligible_entry_finds_eligible() {
        let now = 2_000u64;
        let history = vec![UndoEntry {
            session_name: "feature-x".to_string(),
            commit_id: "abc123".to_string(),
            pre_merge_commit_id: "def456".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        }];

        let eligibility = compute_undo_eligibility(&history[0], now);
        assert!(eligibility.is_eligible());
    }

    #[test]
    fn find_first_eligible_entry_empty_history() {
        let history: Vec<UndoEntry> = vec![];
        assert!(history.is_empty());
    }

    // ---- validate_undo_possible (via compute_undo_eligibility) ----

    #[test]
    fn validate_pushed_to_remote_fails() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: true,
            status: UndoStatus::Completed,
        };

        let eligibility = compute_undo_eligibility(&entry, 2_000);
        assert!(!eligibility.is_eligible());
    }

    #[test]
    fn validate_not_pushed_succeeds() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        };

        let eligibility = compute_undo_eligibility(&entry, 2_000);
        assert!(eligibility.is_eligible());
    }

    #[test]
    fn validate_expired_fails() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        };

        let now = 1_000 + WORKSPACE_RETENTION_SECONDS + 1;
        let eligibility = compute_undo_eligibility(&entry, now);
        assert!(!eligibility.is_eligible());
        assert!(
            matches!(eligibility, Eligibility::Ineligible { ref reason } if reason.contains("Expired"))
        );
    }

    // ---- build_history_entries tests ----

    #[test]
    fn build_history_entries_converts_entries() {
        let history = vec![UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_700_000_000,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        }];

        let entries = build_history_entries(&history, 1_700_000_100);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_name, "test");
        assert!(entries[0].can_undo);
    }

    #[test]
    fn build_history_entries_marks_pushed_as_cannot_undo() {
        let history = vec![UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: true,
            status: UndoStatus::Completed,
        }];

        let entries = build_history_entries(&history, 2_000);
        assert!(!entries[0].can_undo);
        assert!(entries[0].reason_cannot_undo.is_some());
    }

    // ---- update_undo_history serialization logic ----

    #[test]
    fn update_undo_history_marks_entry_as_undone() {
        let history = vec![
            UndoEntry {
                session_name: "feature-x".to_string(),
                commit_id: "abc".to_string(),
                pre_merge_commit_id: "def".to_string(),
                timestamp: 100,
                pushed_to_remote: false,
                status: UndoStatus::Completed,
            },
            UndoEntry {
                session_name: "feature-y".to_string(),
                commit_id: "ghi".to_string(),
                pre_merge_commit_id: "jkl".to_string(),
                timestamp: 200,
                pushed_to_remote: false,
                status: UndoStatus::Completed,
            },
        ];

        let target = &history[0];
        let new_content = serialize_updated_history(&history, target).expect("serialize");

        assert!(new_content.contains("\"status\":\"undone\""));
        assert!(new_content.contains("feature-x"));
        assert!(new_content.contains("feature-y"));
        assert!(new_content.contains("\"status\":\"completed\""));
    }

    // ---- current_unix_seconds ----

    #[test]
    fn current_unix_seconds_returns_reasonable_value() {
        let seconds = current_unix_seconds().expect("should succeed");
        // Should be after 2020-01-01 (1577836800).
        assert!(seconds > 1_577_836_800);
    }

    // ---- UndoOutput construction ----

    #[test]
    fn undo_output_dry_run_construction() {
        let output = UndoOutput {
            session_name: "test-session".to_string(),
            dry_run: true,
            commit_id: "abc123".to_string(),
            pushed_to_remote: false,
        };
        assert_eq!(output.session_name, "test-session");
        assert!(output.dry_run);
        assert_eq!(output.commit_id, "abc123");
    }

    #[test]
    fn undo_output_normal_construction() {
        let output = UndoOutput {
            session_name: "feature-auth".to_string(),
            dry_run: false,
            commit_id: "def456".to_string(),
            pushed_to_remote: false,
        };
        assert!(!output.dry_run);
        assert_eq!(output.session_name, "feature-auth");
    }

    // ---- mark_entry_undone ----

    #[test]
    fn mark_entry_undone_functional() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        };

        let updated = mark_entry_undone(&entry);
        assert_eq!(updated.status, UndoStatus::Undone);
        assert_eq!(updated.session_name, "test");
        assert_eq!(updated.commit_id, "abc");
    }
}
