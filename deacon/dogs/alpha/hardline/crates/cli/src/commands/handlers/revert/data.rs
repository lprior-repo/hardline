//! Data types for the revert command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the revert command.

use serde::{Deserialize, Serialize};

// ============================================================================
// Input Types
// ============================================================================

/// Options for the revert command (parsed from CLI).
#[derive(Debug, Clone, Default)]
pub struct RevertOptions {
    /// Session name to revert.
    pub session_name: String,

    /// Preview without executing.
    pub dry_run: bool,
}

// ============================================================================
// Output Types
// ============================================================================

/// Output from the revert command.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RevertOutput {
    /// Name of the session that was reverted.
    pub session_name: String,

    /// Whether this was a dry run.
    pub dry_run: bool,

    /// Commit ID of the reverted merge.
    pub commit_id: String,

    /// Pre-merge commit ID (target for reset).
    pub pre_merge_commit_id: String,

    /// Whether changes were pushed to remote.
    pub pushed_to_remote: bool,

    /// Error message (if an error occurred during processing).
    pub error: Option<String>,
}

// ============================================================================
// Shared Types
// ============================================================================

/// Undo entry in history log.
///
/// Re-uses the same format as the done handler's UndoEntry so both
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
    /// Status string (e.g. "completed", "reverted").
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- RevertOptions ----

    #[test]
    fn revert_options_default_has_empty_session_name() {
        let opts = RevertOptions::default();
        assert!(opts.session_name.is_empty());
    }

    #[test]
    fn revert_options_default_dry_run_is_false() {
        let opts = RevertOptions::default();
        assert!(!opts.dry_run);
    }

    #[test]
    fn revert_options_with_explicit_fields() {
        let opts = RevertOptions {
            session_name: "feature-x".to_string(),
            dry_run: true,
        };
        assert_eq!(opts.session_name, "feature-x");
        assert!(opts.dry_run);
    }

    // ---- RevertOutput ----

    #[test]
    fn revert_output_default_has_empty_fields() {
        let output = RevertOutput::default();
        assert!(output.session_name.is_empty());
        assert!(output.commit_id.is_empty());
        assert!(output.pre_merge_commit_id.is_empty());
        assert!(!output.dry_run);
        assert!(!output.pushed_to_remote);
        assert!(output.error.is_none());
    }

    #[test]
    fn revert_output_serialization_roundtrip_json() {
        let output = RevertOutput {
            session_name: "feature-x".to_string(),
            dry_run: false,
            commit_id: "abc123".to_string(),
            pre_merge_commit_id: "def456".to_string(),
            pushed_to_remote: false,
            error: None,
        };

        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: RevertOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.session_name, "feature-x");
        assert_eq!(deserialized.commit_id, "abc123");
        assert_eq!(deserialized.pre_merge_commit_id, "def456");
    }

    #[test]
    fn revert_output_with_error_field_serializes() {
        let output = RevertOutput {
            session_name: "test".to_string(),
            dry_run: false,
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            pushed_to_remote: false,
            error: Some("revert failed".to_string()),
        };

        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: RevertOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.error.as_deref(), Some("revert failed"));
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
        let deserialized: UndoEntry = serde_json::from_str(&json).expect("deserialize roundtrip");
        assert!(!deserialized.pushed_to_remote);
        assert_eq!(deserialized.session_name, "ws-1");
        assert_eq!(deserialized.status, "completed");
    }
}
