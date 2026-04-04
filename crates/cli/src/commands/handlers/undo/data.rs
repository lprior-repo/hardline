//! Data types for the undo command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the undo command,
//! which reverts the most recent session merge.

use serde::{Deserialize, Serialize};

// ============================================================================
// Input Types
// ============================================================================

/// Options for the undo command (parsed from CLI).
#[derive(Debug, Clone, Default)]
pub struct UndoOptions {
    /// Preview without executing.
    pub dry_run: bool,

    /// List undo history without reverting.
    pub list: bool,
}

// ============================================================================
// Output Types
// ============================================================================

/// Output from the undo command.
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

    /// Error message (if an error occurred during processing).
    pub error: Option<String>,
}

// ============================================================================
// Shared Types
// ============================================================================

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
    /// Status string (e.g. "completed", "undone").
    pub status: String,
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
    pub status: String,
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

/// Workspace retention window in seconds (24 hours).
pub const WORKSPACE_RETENTION_SECONDS: u64 = 24 * 3600;

/// Compute whether an undo entry is eligible for undo given the current time.
///
/// Pure calculation: no I/O, no side effects.
#[must_use]
pub fn compute_undo_eligibility(
    entry: &UndoEntry,
    now_seconds: u64,
) -> (bool, Option<String>) {
    if entry.pushed_to_remote {
        return (false, Some("Already pushed to remote".to_string()));
    }

    if entry.status == "undone" {
        return (false, Some("Already undone".to_string()));
    }

    if entry.status == "reverted" {
        return (false, Some("Already reverted".to_string()));
    }

    if now_seconds.saturating_sub(entry.timestamp) > WORKSPACE_RETENTION_SECONDS {
        return (
            false,
            Some("Expired after 24 hours".to_string()),
        );
    }

    (true, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- UndoOptions ----

    #[test]
    fn undo_options_default_has_dry_run_false() {
        let opts = UndoOptions::default();
        assert!(!opts.dry_run);
    }

    #[test]
    fn undo_options_default_has_list_false() {
        let opts = UndoOptions::default();
        assert!(!opts.list);
    }

    #[test]
    fn undo_options_with_explicit_fields() {
        let opts = UndoOptions {
            dry_run: true,
            list: true,
        };
        assert!(opts.dry_run);
        assert!(opts.list);
    }

    // ---- UndoOutput ----

    #[test]
    fn undo_output_default_has_empty_fields() {
        let output = UndoOutput::default();
        assert!(output.session_name.is_empty());
        assert!(output.commit_id.is_empty());
        assert!(!output.dry_run);
        assert!(!output.pushed_to_remote);
        assert!(output.error.is_none());
    }

    #[test]
    fn undo_output_serialization_roundtrip_json() {
        let output = UndoOutput {
            session_name: "feature-x".to_string(),
            dry_run: false,
            commit_id: "abc123".to_string(),
            pushed_to_remote: false,
            error: None,
        };

        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: UndoOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.session_name, "feature-x");
        assert_eq!(deserialized.commit_id, "abc123");
    }

    #[test]
    fn undo_output_with_error_field_serializes() {
        let output = UndoOutput {
            session_name: "test".to_string(),
            dry_run: false,
            commit_id: "abc".to_string(),
            pushed_to_remote: false,
            error: Some("undo failed".to_string()),
        };

        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: UndoOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.error.as_deref(), Some("undo failed"));
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
            status: "completed".to_string(),
        };
        assert_eq!(entry.session_name, "feature-x");
        assert_eq!(entry.commit_id, "sha-after");
        assert_eq!(entry.pre_merge_commit_id, "sha-before");
        assert_eq!(entry.timestamp, 1_700_000_000);
        assert!(entry.pushed_to_remote);
        assert_eq!(entry.status, "completed");
    }

    #[test]
    fn undo_entry_serialization_roundtrip() {
        let entry = UndoEntry {
            session_name: "ws-1".to_string(),
            commit_id: "c1".to_string(),
            pre_merge_commit_id: "c0".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: "completed".to_string(),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let deserialized: UndoEntry =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert!(!deserialized.pushed_to_remote);
        assert_eq!(deserialized.session_name, "ws-1");
        assert_eq!(deserialized.status, "completed");
    }

    // ---- UndoHistoryEntry ----

    #[test]
    fn undo_history_entry_can_undo() {
        let entry = UndoHistoryEntry {
            session_name: "test".to_string(),
            commit_id: "abc123".to_string(),
            timestamp: "2025-01-01 00:00:00 UTC".to_string(),
            status: "completed".to_string(),
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
            status: "completed".to_string(),
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
            status: "completed".to_string(),
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
            status: "completed".to_string(),
        };
        let (can_undo, reason) = compute_undo_eligibility(&entry, 2_000);
        assert!(!can_undo);
        assert!(reason.is_some());
        assert!(reason.as_deref().map_or(false, |r| r.contains("pushed")));
    }

    #[test]
    fn compute_eligibility_already_undone() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: "undone".to_string(),
        };
        let (can_undo, reason) = compute_undo_eligibility(&entry, 2_000);
        assert!(!can_undo);
        assert!(reason.is_some());
        assert!(reason.as_deref().map_or(false, |r| r.contains("undone")));
    }

    #[test]
    fn compute_eligibility_already_reverted() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: "reverted".to_string(),
        };
        let (can_undo, reason) = compute_undo_eligibility(&entry, 2_000);
        assert!(!can_undo);
        assert!(reason.is_some());
        assert!(reason.as_deref().map_or(false, |r| r.contains("reverted")));
    }

    #[test]
    fn compute_eligibility_expired() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: "completed".to_string(),
        };
        let now = 1_000 + WORKSPACE_RETENTION_SECONDS + 1;
        let (can_undo, reason) = compute_undo_eligibility(&entry, now);
        assert!(!can_undo);
        assert!(reason.is_some());
        assert!(reason.as_deref().map_or(false, |r| r.contains("Expired")));
    }

    #[test]
    fn compute_eligibility_eligible() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: "completed".to_string(),
        };
        let (can_undo, reason) = compute_undo_eligibility(&entry, 2_000);
        assert!(can_undo);
        assert!(reason.is_none());
    }

    #[test]
    fn compute_eligibility_at_exact_boundary() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 1_000,
            pushed_to_remote: false,
            status: "completed".to_string(),
        };
        // Exactly at the retention boundary should still be eligible.
        let now = 1_000 + WORKSPACE_RETENTION_SECONDS;
        let (can_undo, _reason) = compute_undo_eligibility(&entry, now);
        assert!(can_undo);
    }
}
