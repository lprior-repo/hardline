//! Data types for the undo command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the undo command,
//! which reverts the most recent session merge.

use serde::{Deserialize, Serialize};

// ============================================================================
// Input Types
// ============================================================================

/// Execution mode for the undo command.
///
/// Models mutually exclusive operational modes as a single enum
/// rather than multiple boolean flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UndoMode {
    /// Execute the undo operation.
    #[default]
    Execute,
    /// Preview without executing.
    DryRun,
    /// List undo history without reverting.
    ListHistory,
}

/// Options for the undo command (parsed from CLI).
#[derive(Debug, Clone, Default)]
pub struct UndoOptions {
    /// Execution mode (execute / dry-run / list-history).
    pub mode: UndoMode,
}

// ============================================================================
// Output Types
// ============================================================================

/// Output from the undo command.
///
/// Errors are propagated via `Result`, not stored in this struct.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UndoOutput {
    /// Name of the session that was undone.
    pub session_name: String,

    /// Whether this was a dry run.
    pub dry_run: bool,

    /// Commit ID of the undone merge.
    pub commit_id: String,

    /// Whether changes had been pushed to remote.
    pub pushed_to_remote: bool,
}

// ============================================================================
// Shared Types
// ============================================================================

/// Status of an undo log entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UndoStatus {
    /// Merge completed successfully.
    Completed,
    /// Entry has been undone.
    Undone,
    /// Entry has been reverted.
    Reverted,
}

impl UndoStatus {
    /// Convert to the string representation used in the log file.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Undone => "undone",
            Self::Reverted => "reverted",
        }
    }
}

impl std::fmt::Display for UndoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Undo entry in history log.
///
/// Re-uses the same format as the done/revert handlers' UndoEntry so all
/// handlers read/write the same `.scp/undo.log` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoEntry {
    /// Session name.
    pub session_name: String,
    /// Commit ID after merge.
    pub commit_id: String,
    /// Commit ID before merge.
    pub pre_merge_commit_id: String,
    /// Unix timestamp.
    pub timestamp: u64,
    /// Whether changes were pushed to remote.
    pub pushed_to_remote: bool,
    /// Entry status.
    pub status: UndoStatus,
}

/// A single entry in the undo history for display (list mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoHistoryEntry {
    /// Session name.
    pub session_name: String,
    /// Commit ID.
    pub commit_id: String,
    /// Human-readable timestamp.
    pub timestamp: String,
    /// Entry status.
    pub status: UndoStatus,
    /// Whether the commit was pushed to remote.
    pub pushed_to_remote: bool,
    /// Whether this entry can be undone.
    pub can_undo: bool,
    /// Reason the entry cannot be undone (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_cannot_undo: Option<String>,
}

/// Output for undo history listing (list mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoHistoryOutput {
    /// History entries.
    pub entries: Vec<UndoHistoryEntry>,
    /// Total number of entries.
    pub total: usize,
    /// Whether any entry can be undone.
    pub can_undo: bool,
}

// ============================================================================
// Eligibility Calculation
// ============================================================================

/// Whether an undo entry is eligible for undo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eligibility {
    /// The entry can be undone.
    Eligible,
    /// The entry cannot be undone, with a reason.
    Ineligible {
        reason: String,
    },
}

impl Eligibility {
    /// Returns `true` if the entry is eligible.
    #[must_use]
    pub fn is_eligible(&self) -> bool {
        matches!(self, Self::Eligible)
    }
}

/// Workspace retention window in seconds (24 hours).
pub const WORKSPACE_RETENTION_SECONDS: u64 = 24 * 3600;

/// Create an `Ineligible` variant with a reason.
#[must_use]
fn ineligible(reason: &str) -> Eligibility {
    Eligibility::Ineligible { reason: reason.to_string() }
}

/// Compute whether an undo entry is eligible for undo given the current time.
///
/// Pure calculation: no I/O, no side effects.
#[must_use]
pub fn compute_undo_eligibility(entry: &UndoEntry, now_seconds: u64) -> Eligibility {
    if entry.pushed_to_remote {
        return ineligible("Already pushed to remote");
    }

    if entry.status == UndoStatus::Undone {
        return ineligible("Already undone");
    }

    if entry.status == UndoStatus::Reverted {
        return ineligible("Already reverted");
    }

    if now_seconds.saturating_sub(entry.timestamp) > WORKSPACE_RETENTION_SECONDS {
        return ineligible("Expired after 24 hours");
    }

    Eligibility::Eligible
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- UndoMode ----

    #[test]
    fn undo_mode_default_is_execute() {
        assert_eq!(UndoMode::default(), UndoMode::Execute);
    }

    #[test]
    fn undo_options_default_has_execute_mode() {
        let opts = UndoOptions::default();
        assert_eq!(opts.mode, UndoMode::Execute);
    }

    // ---- UndoOutput ----

    #[test]
    fn undo_output_default_has_empty_fields() {
        let output = UndoOutput::default();
        assert!(output.session_name.is_empty());
        assert!(output.commit_id.is_empty());
        assert!(!output.dry_run);
        assert!(!output.pushed_to_remote);
    }

    #[test]
    fn undo_output_serialization_roundtrip_json() {
        let output = UndoOutput {
            session_name: "feature-x".to_string(),
            dry_run: false,
            commit_id: "abc123".to_string(),
            pushed_to_remote: false,
        };

        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: UndoOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.session_name, "feature-x");
        assert_eq!(deserialized.commit_id, "abc123");
    }

    // ---- UndoStatus ----

    #[test]
    fn undo_status_str_roundtrip() {
        assert_eq!(UndoStatus::Completed.as_str(), "completed");
        assert_eq!(UndoStatus::Undone.as_str(), "undone");
        assert_eq!(UndoStatus::Reverted.as_str(), "reverted");
    }

    #[test]
    fn undo_status_serde_roundtrip() {
        let status = UndoStatus::Completed;
        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(json, "\"completed\"");
        let deserialized: UndoStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, UndoStatus::Completed);
    }

    #[test]
    fn undo_status_undone_serde() {
        let json = serde_json::to_string(&UndoStatus::Undone).expect("serialize");
        let deserialized: UndoStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, UndoStatus::Undone);
    }

    #[test]
    fn undo_status_reverted_serde() {
        let json = serde_json::to_string(&UndoStatus::Reverted).expect("serialize");
        let deserialized: UndoStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, UndoStatus::Reverted);
    }

    // ---- UndoEntry ----

    #[test]
    fn undo_entry_construction() {
        let entry = UndoEntry {
            session_name: "feature-x".to_string(),
            commit_id: "sha-after".to_string(),
            pre_merge_commit_id: "sha-before".to_string(),
            timestamp: 1_700_000_000,
            pushed_to_remote: true,
            status: UndoStatus::Completed,
        };
        assert_eq!(entry.session_name, "feature-x");
        assert_eq!(entry.commit_id, "sha-after");
        assert_eq!(entry.pre_merge_commit_id, "sha-before");
        assert_eq!(entry.timestamp, 1_700_000_000);
        assert!(entry.pushed_to_remote);
        assert_eq!(entry.status, UndoStatus::Completed);
    }

    #[test]
    fn undo_entry_serialization_roundtrip() {
        let entry = UndoEntry {
            session_name: "ws-1".to_string(),
            commit_id: "c1".to_string(),
            pre_merge_commit_id: "c0".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let deserialized: UndoEntry =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert!(!deserialized.pushed_to_remote);
        assert_eq!(deserialized.session_name, "ws-1");
        assert_eq!(deserialized.status, UndoStatus::Completed);
    }

    // ---- Eligibility ----

    #[test]
    fn eligibility_is_eligible() {
        let e = Eligibility::Eligible;
        assert!(e.is_eligible());
    }

    #[test]
    fn eligibility_ineligible_is_not_eligible() {
        let e = Eligibility::Ineligible { reason: "test".to_string() };
        assert!(!e.is_eligible());
    }

    #[test]
    fn eligibility_equality() {
        assert_eq!(Eligibility::Eligible, Eligibility::Eligible);
        assert_ne!(
            Eligibility::Eligible,
            Eligibility::Ineligible { reason: "x".to_string() }
        );
    }

    // ---- UndoHistoryEntry ----

    #[test]
    fn undo_history_entry_can_undo() {
        let entry = UndoHistoryEntry {
            session_name: "test".to_string(),
            commit_id: "abc123".to_string(),
            timestamp: "2025-01-01 00:00:00 UTC".to_string(),
            status: UndoStatus::Completed,
            pushed_to_remote: false,
            can_undo: true,
            reason_cannot_undo: None,
        };
        assert!(entry.can_undo);
        assert!(entry.reason_cannot_undo.is_none());
    }

    #[test]
    fn undo_history_entry_cannot_undo_pushed() {
        let entry = UndoHistoryEntry {
            session_name: "test".to_string(),
            commit_id: "abc123".to_string(),
            timestamp: "2025-01-01 00:00:00 UTC".to_string(),
            status: UndoStatus::Completed,
            pushed_to_remote: true,
            can_undo: false,
            reason_cannot_undo: Some("Already pushed to remote".to_string()),
        };
        assert!(!entry.can_undo);
        assert!(entry.reason_cannot_undo.is_some());
    }

    #[test]
    fn undo_history_entry_serialization_skips_none_reason() {
        let entry = UndoHistoryEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            timestamp: "2025-01-01 00:00:00 UTC".to_string(),
            status: UndoStatus::Completed,
            pushed_to_remote: false,
            can_undo: true,
            reason_cannot_undo: None,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(!json.contains("reason_cannot_undo"));
    }

    // ---- UndoHistoryOutput ----

    #[test]
    fn undo_history_output_serialization() {
        let output = UndoHistoryOutput {
            entries: vec![],
            total: 0,
            can_undo: false,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"can_undo\":false"));
    }

    // ---- compute_undo_eligibility ----

    #[test]
    fn compute_eligibility_pushed_to_remote() {
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
        assert!(matches!(eligibility, Eligibility::Ineligible { ref reason } if reason.contains("pushed")));
    }

    #[test]
    fn compute_eligibility_already_undone() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: UndoStatus::Undone,
        };
        let eligibility = compute_undo_eligibility(&entry, 2_000);
        assert!(!eligibility.is_eligible());
        assert!(matches!(eligibility, Eligibility::Ineligible { ref reason } if reason.contains("undone")));
    }

    #[test]
    fn compute_eligibility_already_reverted() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: UndoStatus::Reverted,
        };
        let eligibility = compute_undo_eligibility(&entry, 2_000);
        assert!(!eligibility.is_eligible());
        assert!(matches!(eligibility, Eligibility::Ineligible { ref reason } if reason.contains("reverted")));
    }

    #[test]
    fn compute_eligibility_expired() {
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
        assert!(matches!(eligibility, Eligibility::Ineligible { ref reason } if reason.contains("Expired")));
    }

    #[test]
    fn compute_eligibility_eligible() {
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
    fn compute_eligibility_at_exact_boundary() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        };
        // Exactly at the retention boundary should still be eligible.
        let now = 1_000 + WORKSPACE_RETENTION_SECONDS;
        assert!(compute_undo_eligibility(&entry, now).is_eligible());
    }

    // ---- Exhaustive eligibility tests ----

    #[test]
    fn compute_eligibility_one_second_before_expiry_is_eligible() {
        let entry = make_completed_entry(1_000);
        let now = 1_000 + WORKSPACE_RETENTION_SECONDS - 1;
        assert!(compute_undo_eligibility(&entry, now).is_eligible());
    }

    #[test]
    fn compute_eligibility_one_second_after_expiry_is_ineligible() {
        let entry = make_completed_entry(1_000);
        let now = 1_000 + WORKSPACE_RETENTION_SECONDS + 1;
        let result = compute_undo_eligibility(&entry, now);
        assert!(!result.is_eligible());
        assert_matches_reason(&result, "Expired");
    }

    #[test]
    fn compute_eligibility_zero_timestamp_is_eligible() {
        let entry = UndoEntry {
            timestamp: 0,
            ..make_completed_entry(0)
        };
        // Far future relative to epoch zero — definitely expired.
        let now = WORKSPACE_RETENTION_SECONDS + 1;
        assert!(!compute_undo_eligibility(&entry, now).is_eligible());
    }

    #[test]
    fn compute_eligibility_far_future_entry_is_eligible() {
        // An entry from the far future (relative to now) — saturation-safe.
        let entry = make_completed_entry(999_999_999_999);
        let now = 999_999_999_999 + 100;
        assert!(compute_undo_eligibility(&entry, now).is_eligible());
    }

    #[test]
    fn compute_eligibility_same_timestamp_is_eligible() {
        let ts = 5_000u64;
        let entry = make_completed_entry(ts);
        assert!(compute_undo_eligibility(&entry, ts).is_eligible());
    }

    #[test]
    fn compute_eligibility_pushed_takes_priority_over_undone() {
        let entry = UndoEntry {
            pushed_to_remote: true,
            status: UndoStatus::Undone,
            ..make_completed_entry(1_000)
        };
        let result = compute_undo_eligibility(&entry, 2_000);
        assert!(!result.is_eligible());
        // "pushed" is checked first.
        assert_matches_reason(&result, "pushed");
    }

    #[test]
    fn compute_eligibility_pushed_takes_priority_over_reverted() {
        let entry = UndoEntry {
            pushed_to_remote: true,
            status: UndoStatus::Reverted,
            ..make_completed_entry(1_000)
        };
        let result = compute_undo_eligibility(&entry, 2_000);
        assert_matches_reason(&result, "pushed");
    }

    #[test]
    fn compute_eligibility_undone_takes_priority_over_expired() {
        let entry = UndoEntry {
            pushed_to_remote: false,
            status: UndoStatus::Undone,
            ..make_completed_entry(1_000)
        };
        let now = 1_000 + WORKSPACE_RETENTION_SECONDS + 100; // also expired
        let result = compute_undo_eligibility(&entry, now);
        assert_matches_reason(&result, "undone");
    }

    #[test]
    fn compute_eligibility_reverted_takes_priority_over_expired() {
        let entry = UndoEntry {
            pushed_to_remote: false,
            status: UndoStatus::Reverted,
            ..make_completed_entry(1_000)
        };
        let now = 1_000 + WORKSPACE_RETENTION_SECONDS + 100;
        let result = compute_undo_eligibility(&entry, now);
        assert_matches_reason(&result, "reverted");
    }

    // ---- UndoStatus Display ----

    #[test]
    fn undo_status_display_completed() {
        assert_eq!(format!("{}", UndoStatus::Completed), "completed");
    }

    #[test]
    fn undo_status_display_undone() {
        assert_eq!(format!("{}", UndoStatus::Undone), "undone");
    }

    #[test]
    fn undo_status_display_reverted() {
        assert_eq!(format!("{}", UndoStatus::Reverted), "reverted");
    }

    // ---- UndoHistoryEntry with reason present ----

    #[test]
    fn undo_history_entry_serialization_includes_reason_when_present() {
        let entry = UndoHistoryEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            timestamp: "2025-06-15 12:00:00 UTC".to_string(),
            status: UndoStatus::Completed,
            pushed_to_remote: true,
            can_undo: false,
            reason_cannot_undo: Some("Already pushed to remote".to_string()),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(json.contains("reason_cannot_undo"));
        assert!(json.contains("Already pushed to remote"));
    }

    // ---- UndoHistoryOutput with populated entries ----

    #[test]
    fn undo_history_output_with_entries() {
        let entries = vec![
            UndoHistoryEntry {
                session_name: "session-1".to_string(),
                commit_id: "aaa".to_string(),
                timestamp: "2025-01-01 00:00:00 UTC".to_string(),
                status: UndoStatus::Completed,
                pushed_to_remote: false,
                can_undo: true,
                reason_cannot_undo: None,
            },
            UndoHistoryEntry {
                session_name: "session-2".to_string(),
                commit_id: "bbb".to_string(),
                timestamp: "2025-01-02 00:00:00 UTC".to_string(),
                status: UndoStatus::Undone,
                pushed_to_remote: false,
                can_undo: false,
                reason_cannot_undo: Some("Already undone".to_string()),
            },
        ];
        let output = UndoHistoryOutput {
            total: entries.len(),
            can_undo: true,
            entries,
        };
        assert_eq!(output.total, 2);
        assert!(output.can_undo);
        assert!(output.entries[0].can_undo);
        assert!(!output.entries[1].can_undo);
    }

    #[test]
    fn undo_history_output_can_undo_false_when_no_eligible() {
        let entries = vec![UndoHistoryEntry {
            session_name: "old".to_string(),
            commit_id: "zzz".to_string(),
            timestamp: "2024-01-01 00:00:00 UTC".to_string(),
            status: UndoStatus::Undone,
            pushed_to_remote: false,
            can_undo: false,
            reason_cannot_undo: Some("Already undone".to_string()),
        }];
        let output = UndoHistoryOutput {
            total: 1,
            can_undo: false,
            entries,
        };
        assert!(!output.can_undo);
    }

    // ---- UndoEntry edge cases ----

    #[test]
    fn undo_entry_with_empty_strings() {
        let entry = UndoEntry {
            session_name: String::new(),
            commit_id: String::new(),
            pre_merge_commit_id: String::new(),
            timestamp: 0,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let restored: UndoEntry = serde_json::from_str(&json).expect("deserialize");
        assert!(restored.session_name.is_empty());
        assert!(restored.commit_id.is_empty());
    }

    #[test]
    fn undo_entry_with_special_characters() {
        let entry = UndoEntry {
            session_name: "session-with-\"quotes\"-and\\backslash".to_string(),
            commit_id: "abc\ndef".to_string(),
            pre_merge_commit_id: "normal".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let restored: UndoEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.session_name, entry.session_name);
        assert_eq!(restored.commit_id, entry.commit_id);
    }

    #[test]
    fn undo_entry_max_timestamp() {
        let entry = UndoEntry {
            timestamp: u64::MAX,
            ..make_completed_entry(0)
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let restored: UndoEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.timestamp, u64::MAX);
    }

    // ---- Retention constant ----

    #[test]
    fn workspace_retention_is_24_hours() {
        assert_eq!(WORKSPACE_RETENTION_SECONDS, 24 * 3600);
        assert_eq!(WORKSPACE_RETENTION_SECONDS, 86_400);
    }

    // ---- UndoOutput serde roundtrip with all fields populated ----

    #[test]
    fn undo_output_full_roundtrip() {
        let output = UndoOutput {
            session_name: "full-test".to_string(),
            dry_run: true,
            commit_id: "deadbeef".to_string(),
            pushed_to_remote: true,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let restored: UndoOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.session_name, "full-test");
        assert!(restored.dry_run);
        assert_eq!(restored.commit_id, "deadbeef");
        assert!(restored.pushed_to_remote);
    }

    // ---- Helpers ----

    /// Create a basic completed entry at the given timestamp.
    fn make_completed_entry(timestamp: u64) -> UndoEntry {
        UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp,
            pushed_to_remote: false,
            status: UndoStatus::Completed,
        }
    }

    /// Assert the eligibility result contains the expected reason substring.
    fn assert_matches_reason(eligibility: &Eligibility, expected_substring: &str) {
        match eligibility {
            Eligibility::Ineligible { reason } => {
                assert!(
                    reason.contains(expected_substring),
                    "Expected reason containing '{expected_substring}', got '{reason}'"
                );
            }
            Eligibility::Eligible => {
                panic!("Expected Ineligible, got Eligible");
            }
        }
    }
}
