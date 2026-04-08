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

    // ========================================================================
    // Exhaustive tests: parse_log_lines
    // ========================================================================

    #[test]
    fn parse_log_lines_valid_single_entry() {
        let entry = UndoEntry {
            session_name: "ws-1".to_string(),
            commit_id: "c1".to_string(),
            pre_merge_commit_id: "c0".to_string(),
            timestamp: 1000,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        };
        let line = serde_json::to_string(&entry).expect("serialize");
        let parsed = parse_log_lines(&line).expect("parse");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].session_name, "ws-1");
        assert_eq!(parsed[0].commit_id, "c1");
    }

    #[test]
    fn parse_log_lines_valid_multiple_entries() {
        let e1 = make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Completed);
        let e2 = make_entry("ws-2", "c2", "c1", 2000, false, UndoStatus::Completed);
        let e3 = make_entry("ws-3", "c3", "c2", 3000, true, UndoStatus::Undone);
        let content = format!(
            "{}\n{}\n{}\n",
            serde_json::to_string(&e1).unwrap(),
            serde_json::to_string(&e2).unwrap(),
            serde_json::to_string(&e3).unwrap(),
        );
        let parsed = parse_log_lines(&content).expect("parse");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].session_name, "ws-1");
        assert_eq!(parsed[1].session_name, "ws-2");
        assert_eq!(parsed[2].session_name, "ws-3");
        assert!(parsed[2].pushed_to_remote);
        assert_eq!(parsed[2].status, UndoStatus::Undone);
    }

    #[test]
    fn parse_log_lines_empty_content() {
        let parsed = parse_log_lines("").expect("parse empty");
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_log_lines_only_whitespace() {
        let parsed = parse_log_lines("   \n  \n\n  ").expect("parse whitespace");
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_log_lines_blank_lines_between_entries() {
        let e1 = make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Completed);
        let e2 = make_entry("ws-2", "c2", "c1", 2000, false, UndoStatus::Completed);
        let content = format!(
            "{}\n\n  \n{}\n",
            serde_json::to_string(&e1).unwrap(),
            serde_json::to_string(&e2).unwrap(),
        );
        let parsed = parse_log_lines(&content).expect("parse");
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn parse_log_lines_malformed_json_fails() {
        let content = "not valid json\n";
        let result = parse_log_lines(content);
        assert!(result.is_err());
    }

    #[test]
    fn parse_log_lines_partial_malform_fails() {
        let e1 = make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Completed);
        let content = format!(
            "{}\n{{bad json}}\n",
            serde_json::to_string(&e1).unwrap(),
        );
        let result = parse_log_lines(&content);
        assert!(result.is_err());
    }

    #[test]
    fn parse_log_lines_trailing_newline() {
        let e = make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Completed);
        let content = serde_json::to_string(&e).unwrap() + "\n";
        let parsed = parse_log_lines(&content).expect("parse");
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn parse_log_lines_no_trailing_newline() {
        let e = make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Completed);
        let content = serde_json::to_string(&e).unwrap();
        let parsed = parse_log_lines(&content).expect("parse");
        assert_eq!(parsed.len(), 1);
    }

    // ========================================================================
    // Exhaustive tests: format_timestamp
    // ========================================================================

    #[test]
    fn format_timestamp_zero_falls_back_to_raw() {
        let result = format_timestamp(0);
        // chrono returns 1970-01-01 00:00:00 UTC for timestamp 0
        assert!(result.contains("1970") || result == "0");
    }

    #[test]
    fn format_timestamp_known_value() {
        // 2024-01-15 12:30:00 UTC
        let ts: u64 = 1_705_322_600;
        let result = format_timestamp(ts);
        assert!(result.contains("2024"));
        assert!(result.contains("UTC"));
    }

    #[test]
    fn format_timestamp_large_value() {
        // 2100-01-01 00:00:00 UTC
        let ts: u64 = 4_102_444_800;
        let result = format_timestamp(ts);
        assert!(result.contains("UTC"));
    }

    // ========================================================================
    // Exhaustive tests: serialize_updated_history
    // ========================================================================

    #[test]
    fn serialize_history_no_matching_entry_preserves_all() {
        let history = vec![
            make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Completed),
            make_entry("ws-2", "c2", "c1", 2000, false, UndoStatus::Completed),
        ];
        let target = make_entry("ws-99", "zzz", "yyy", 9999, false, UndoStatus::Completed);
        let result = serialize_updated_history(&history, &target).expect("serialize");
        // No entry should be marked undone — all remain "completed"
        assert_eq!(result.matches("\"status\":\"completed\"").count(), 2);
        assert!(!result.contains("\"status\":\"undone\""));
    }

    #[test]
    fn serialize_history_single_entry_marks_undone() {
        let history = vec![
            make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Completed),
        ];
        let target = &history[0];
        let result = serialize_updated_history(&history, target).expect("serialize");
        assert!(result.contains("\"status\":\"undone\""));
        assert!(!result.contains("\"status\":\"completed\""));
    }

    #[test]
    fn serialize_history_preserves_non_matching_status() {
        let history = vec![
            make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Completed),
            make_entry("ws-2", "c2", "c1", 2000, false, UndoStatus::Completed),
            make_entry("ws-3", "c3", "c2", 3000, true, UndoStatus::Completed),
        ];
        let target = make_entry("ws-2", "c2", "c1", 2000, false, UndoStatus::Completed);
        let result = serialize_updated_history(&history, &target).expect("serialize");
        assert!(result.contains("\"status\":\"undone\""));
        // Two completed entries remain (ws-1 and ws-3)
        assert_eq!(result.matches("\"status\":\"completed\"").count(), 2);
    }

    #[test]
    fn serialize_history_output_has_trailing_newline() {
        let history = vec![
            make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Completed),
        ];
        let target = &history[0];
        let result = serialize_updated_history(&history, target).expect("serialize");
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn serialize_history_empty_input() {
        let history: Vec<UndoEntry> = vec![];
        let target = make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Completed);
        let result = serialize_updated_history(&history, &target).expect("serialize");
        assert_eq!(result, "\n");
    }

    // ========================================================================
    // Exhaustive tests: build_history_entries
    // ========================================================================

    #[test]
    fn build_history_empty() {
        let entries = build_history_entries(&[], 2_000);
        assert!(entries.is_empty());
    }

    #[test]
    fn build_history_all_eligible() {
        let history = vec![
            make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Completed),
            make_entry("ws-2", "c2", "c1", 1_100, false, UndoStatus::Completed),
        ];
        let entries = build_history_entries(&history, 2_000);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].can_undo);
        assert!(entries[1].can_undo);
    }

    #[test]
    fn build_history_all_ineligible() {
        let history = vec![
            make_entry("ws-1", "c1", "c0", 1_000, true, UndoStatus::Completed),
            make_entry("ws-2", "c2", "c1", 1_100, false, UndoStatus::Undone),
        ];
        let entries = build_history_entries(&history, 2_000);
        assert_eq!(entries.len(), 2);
        assert!(!entries[0].can_undo);
        assert!(!entries[1].can_undo);
        assert!(entries[0].reason_cannot_undo.is_some());
        assert!(entries[1].reason_cannot_undo.is_some());
    }

    #[test]
    fn build_history_mixed_eligibility() {
        let history = vec![
            make_entry("ws-1", "c1", "c0", 1_000, true, UndoStatus::Completed),
            make_entry("ws-2", "c2", "c1", 1_100, false, UndoStatus::Completed),
            make_entry("ws-3", "c3", "c2", 1_200, false, UndoStatus::Undone),
        ];
        let entries = build_history_entries(&history, 2_000);
        assert!(!entries[0].can_undo); // pushed
        assert!(entries[1].can_undo);  // eligible
        assert!(!entries[2].can_undo); // undone
    }

    #[test]
    fn build_history_expired_entry() {
        let history = vec![
            make_entry("ws-old", "c1", "c0", 1_000, false, UndoStatus::Completed),
        ];
        let now = 1_000 + WORKSPACE_RETENTION_SECONDS + 1;
        let entries = build_history_entries(&history, now);
        assert!(!entries[0].can_undo);
        assert!(entries[0].reason_cannot_undo.is_some());
    }

    // ========================================================================
    // Exhaustive tests: history_entry_from_undo
    // ========================================================================

    #[test]
    fn history_entry_preserves_all_fields() {
        let entry = UndoEntry {
            session_name: "my-session".to_string(),
            commit_id: "abc123".to_string(),
            pre_merge_commit_id: "def456".to_string(),
            timestamp: 1_700_000_000,
            pushed_to_remote: true,
            status: UndoStatus::Completed,
        };
        let display = history_entry_from_undo(&entry, 1_700_000_100);

        assert_eq!(display.session_name, "my-session");
        assert_eq!(display.commit_id, "abc123");
        assert_eq!(display.status, UndoStatus::Completed);
        assert!(display.pushed_to_remote);
        assert!(!display.can_undo);
        assert!(display.reason_cannot_undo.is_some());
        assert!(display.timestamp.contains("UTC"));
    }

    #[test]
    fn history_entry_eligible_has_no_reason() {
        let entry = make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Completed);
        let display = history_entry_from_undo(&entry, 2_000);
        assert!(display.can_undo);
        assert!(display.reason_cannot_undo.is_none());
    }

    #[test]
    fn history_entry_undone_has_reason() {
        let entry = make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Undone);
        let display = history_entry_from_undo(&entry, 2_000);
        assert!(!display.can_undo);
        assert!(display.reason_cannot_undo.unwrap().contains("undone"));
    }

    // ========================================================================
    // Exhaustive tests: find_eligible_entry (via compute_undo_eligibility)
    // ========================================================================

    #[test]
    fn find_eligible_from_multi_entry_first_wins() {
        // History in most-recent-first order (as returned by read_undo_history).
        let history = vec![
            make_entry("ws-3", "c3", "c2", 3_000, false, UndoStatus::Completed),
            make_entry("ws-2", "c2", "c1", 2_000, false, UndoStatus::Completed),
            make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Completed),
        ];
        // All are eligible; first (most recent) should be found.
        let now = 4_000u64;
        let found = history
            .iter()
            .find(|e| compute_undo_eligibility(e, now).is_eligible())
            .cloned();
        assert!(found.is_some());
        assert_eq!(found.unwrap().session_name, "ws-3");
    }

    #[test]
    fn find_eligible_skips_ineligible_first() {
        let history = vec![
            make_entry("ws-3", "c3", "c2", 3_000, true, UndoStatus::Completed), // pushed
            make_entry("ws-2", "c2", "c1", 2_000, false, UndoStatus::Completed), // eligible
            make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Undone),    // undone
        ];
        let now = 4_000u64;
        let found = history
            .iter()
            .find(|e| compute_undo_eligibility(e, now).is_eligible())
            .cloned();
        assert!(found.is_some());
        assert_eq!(found.unwrap().session_name, "ws-2");
    }

    #[test]
    fn find_eligible_all_ineligible_returns_none() {
        let history = vec![
            make_entry("ws-3", "c3", "c2", 3_000, true, UndoStatus::Completed),
            make_entry("ws-2", "c2", "c1", 2_000, false, UndoStatus::Undone),
            make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Reverted),
        ];
        let now = 4_000u64;
        let found = history
            .iter()
            .find(|e| compute_undo_eligibility(e, now).is_eligible());
        assert!(found.is_none());
    }

    #[test]
    fn find_eligible_empty_history_returns_none() {
        let history: Vec<UndoEntry> = vec![];
        let found = history
            .iter()
            .find(|e| compute_undo_eligibility(e, 2_000).is_eligible());
        assert!(found.is_none());
    }

    #[test]
    fn find_eligible_only_expired_returns_none() {
        let history = vec![
            make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Completed),
        ];
        let now = 1_000 + WORKSPACE_RETENTION_SECONDS + 1;
        let found = history
            .iter()
            .find(|e| compute_undo_eligibility(e, now).is_eligible());
        assert!(found.is_none());
    }

    // ========================================================================
    // Exhaustive tests: Stack depth & multiple undo scenarios
    // ========================================================================

    #[test]
    fn large_history_finds_most_recent_eligible() {
        // Simulate a deep undo stack (50 entries).
        let mut history: Vec<UndoEntry> = (0..50)
            .map(|i| {
                make_entry(
                    &format!("ws-{i}"),
                    &format!("c{i}"),
                    &format!("c{}", (i as i64).saturating_sub(1)),
                    1000 + i as u64 * 100,
                    false,
                    UndoStatus::Completed,
                )
            })
            .collect();

        // Reverse for most-recent-first order (simulates read_undo_history).
        history.reverse();
        // history[0] is now ws-49 (newest), history[49] is ws-0 (oldest).

        // Mark the 10 most recent (first 10 in reversed order) as pushed.
        for entry in history.iter_mut().take(10) {
            entry.pushed_to_remote = true;
        }

        let now = 10_000u64;
        let found = history
            .iter()
            .find(|e| compute_undo_eligibility(e, now).is_eligible())
            .cloned();
        assert!(found.is_some());
        // First eligible should be ws-39 (first non-pushed in reversed order).
        assert_eq!(found.unwrap().session_name, "ws-39");
    }

    #[test]
    fn undo_one_then_next_is_eligible() {
        // Scenario: Two completed entries. After marking one as undone,
        // the other should still be eligible.
        let mut history = vec![
            make_entry("ws-2", "c2", "c1", 2_000, false, UndoStatus::Completed),
            make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Completed),
        ];

        // "Undo" the first one by marking it undone.
        history[0] = mark_entry_undone(&history[0]);
        assert_eq!(history[0].status, UndoStatus::Undone);

        // The second entry should now be eligible.
        let now = 3_000u64;
        let found = history
            .iter()
            .find(|e| compute_undo_eligibility(e, now).is_eligible())
            .cloned();
        assert!(found.is_some());
        assert_eq!(found.unwrap().session_name, "ws-1");
    }

    #[test]
    fn undo_sequence_all_entries() {
        // Three entries — simulate undoing them one by one.
        let mut history = vec![
            make_entry("ws-3", "c3", "c2", 3_000, false, UndoStatus::Completed),
            make_entry("ws-2", "c2", "c1", 2_000, false, UndoStatus::Completed),
            make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Completed),
        ];

        // Undo ws-3.
        history[0] = mark_entry_undone(&history[0]);
        assert_eq!(history[0].status, UndoStatus::Undone);
        let now = 4_000u64;
        let found = history.iter().find(|e| compute_undo_eligibility(e, now).is_eligible()).unwrap();
        assert_eq!(found.session_name, "ws-2");

        // Undo ws-2.
        let idx = history.iter().position(|e| e.session_name == "ws-2").unwrap();
        history[idx] = mark_entry_undone(&history[idx]);

        let found = history.iter().find(|e| compute_undo_eligibility(e, now).is_eligible()).unwrap();
        assert_eq!(found.session_name, "ws-1");

        // Undo ws-1.
        let idx = history.iter().position(|e| e.session_name == "ws-1").unwrap();
        history[idx] = mark_entry_undone(&history[idx]);

        // All undone — no eligible left.
        let found = history.iter().find(|e| compute_undo_eligibility(e, now).is_eligible());
        assert!(found.is_none());
    }

    // ========================================================================
    // Exhaustive tests: mark_entry_undone field preservation
    // ========================================================================

    #[test]
    fn mark_undone_preserves_all_fields_except_status() {
        let entry = UndoEntry {
            session_name: "complex-session".to_string(),
            commit_id: "sha-after-merge".to_string(),
            pre_merge_commit_id: "sha-before-merge".to_string(),
            timestamp: 1_700_000_000,
            pushed_to_remote: true,
            status: UndoStatus::Completed,
        };
        let updated = mark_entry_undone(&entry);
        assert_eq!(updated.session_name, "complex-session");
        assert_eq!(updated.commit_id, "sha-after-merge");
        assert_eq!(updated.pre_merge_commit_id, "sha-before-merge");
        assert_eq!(updated.timestamp, 1_700_000_000);
        assert!(updated.pushed_to_remote);
        assert_eq!(updated.status, UndoStatus::Undone);
    }

    #[test]
    fn mark_undone_from_already_undone_is_idempotent() {
        let entry = make_entry("ws-1", "c1", "c0", 1000, false, UndoStatus::Undone);
        let updated = mark_entry_undone(&entry);
        assert_eq!(updated.status, UndoStatus::Undone);
        assert_eq!(updated.session_name, "ws-1");
    }

    // ========================================================================
    // Exhaustive tests: UndoMode dispatch via run_undo options
    // ========================================================================

    #[test]
    fn undo_options_list_history_mode() {
        let opts = UndoOptions {
            mode: UndoMode::ListHistory,
        };
        assert_eq!(opts.mode, UndoMode::ListHistory);
    }

    #[test]
    fn undo_options_dry_run_mode() {
        let opts = UndoOptions {
            mode: UndoMode::DryRun,
        };
        assert_eq!(opts.mode, UndoMode::DryRun);
    }

    #[test]
    fn undo_options_execute_mode() {
        let opts = UndoOptions {
            mode: UndoMode::Execute,
        };
        assert_eq!(opts.mode, UndoMode::Execute);
    }

    // ========================================================================
    // Exhaustive tests: UndoOutput construction variations
    // ========================================================================

    #[test]
    fn undo_output_with_pushed_to_remote() {
        let output = UndoOutput {
            session_name: "pushed-session".to_string(),
            dry_run: false,
            commit_id: "abc".to_string(),
            pushed_to_remote: true,
        };
        assert!(output.pushed_to_remote);
    }

    #[test]
    fn undo_output_default_for_list_mode() {
        let output = UndoOutput::default();
        assert!(output.session_name.is_empty());
        assert!(!output.dry_run);
        assert!(output.commit_id.is_empty());
        assert!(!output.pushed_to_remote);
    }

    // ========================================================================
    // Exhaustive tests: Redo / Revert status scenarios
    // ========================================================================

    #[test]
    fn reverted_entry_not_eligible_for_undo() {
        let entry = make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Reverted);
        let result = compute_undo_eligibility(&entry, 2_000);
        assert!(!result.is_eligible());
    }

    #[test]
    fn multiple_status_changes_tracked_in_history() {
        let original = make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Completed);

        // Undo it.
        let undone = mark_entry_undone(&original);
        assert_eq!(undone.status, UndoStatus::Undone);

        // A "reverted" entry represents a different operation (from revert handler).
        let reverted = UndoEntry {
            status: UndoStatus::Reverted,
            ..original.clone()
        };
        assert_eq!(reverted.status, UndoStatus::Reverted);

        // Neither can be undone again.
        assert!(!compute_undo_eligibility(&undone, 2_000).is_eligible());
        assert!(!compute_undo_eligibility(&reverted, 2_000).is_eligible());
    }

    // ========================================================================
    // Exhaustive tests: Side effects — serialize round-trip through history
    // ========================================================================

    #[test]
    fn serialize_then_parse_preserves_data() {
        let history = vec![
            make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Completed),
            make_entry("ws-2", "c2", "c1", 2_000, false, UndoStatus::Completed),
        ];
        let target = make_entry("ws-1", "c1", "c0", 1_000, false, UndoStatus::Completed);
        let serialized = serialize_updated_history(&history, &target).expect("serialize");

        // Parse it back.
        let restored = parse_log_lines(&serialized).expect("parse");
        assert_eq!(restored.len(), 2);

        // Find ws-1 — should be undone.
        let ws1 = restored.iter().find(|e| e.session_name == "ws-1").unwrap();
        assert_eq!(ws1.status, UndoStatus::Undone);

        // ws-2 should remain completed.
        let ws2 = restored.iter().find(|e| e.session_name == "ws-2").unwrap();
        assert_eq!(ws2.status, UndoStatus::Completed);
    }

    #[test]
    fn serialize_preserves_chronological_order() {
        let history = vec![
            make_entry("ws-newest", "c3", "c2", 3_000, false, UndoStatus::Completed),
            make_entry("ws-middle", "c2", "c1", 2_000, false, UndoStatus::Completed),
            make_entry("ws-oldest", "c1", "c0", 1_000, false, UndoStatus::Completed),
        ];
        let target = make_entry("ws-newest", "c3", "c2", 3_000, false, UndoStatus::Completed);
        let serialized = serialize_updated_history(&history, &target).expect("serialize");
        let restored = parse_log_lines(&serialized).expect("parse");

        // After serialize (which reverses to chronological) + parse, the order
        // should be chronological: oldest first.
        assert_eq!(restored[0].session_name, "ws-oldest");
        assert_eq!(restored[1].session_name, "ws-middle");
        assert_eq!(restored[2].session_name, "ws-newest");
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    fn make_entry(
        session: &str,
        commit: &str,
        pre_merge: &str,
        ts: u64,
        pushed: bool,
        status: UndoStatus,
    ) -> UndoEntry {
        UndoEntry {
            session_name: session.to_string(),
            commit_id: commit.to_string(),
            pre_merge_commit_id: pre_merge.to_string(),
            timestamp: ts,
            pushed_to_remote: pushed,
            status,
        }
    }
}
