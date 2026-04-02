//! Data types for the done command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the done command.

use serde::{Deserialize, Serialize};

// ============================================================================
// Input Types
// ============================================================================

/// Options for the done command (parsed from CLI).
#[derive(Debug, Clone, Default)]
#[expect(clippy::struct_excessive_bools)] // CLI flags: independent options
pub struct DoneOptions {
    /// Workspace to complete (None = current workspace)
    pub workspace: Option<String>,

    /// Commit message (auto-generated if not provided)
    pub message: Option<String>,

    /// Keep workspace after merge
    pub keep_workspace: bool,

    /// Squash all commits into one
    pub squash: bool,

    /// Preview without executing
    pub dry_run: bool,

    /// Detect conflicts before merging
    pub detect_conflicts: bool,

    /// Skip bead status update
    pub no_bead_update: bool,
}

// ============================================================================
// Output Types
// ============================================================================

/// Output from the done command.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DoneOutput {
    /// Name of the workspace that was completed
    pub workspace_name: String,

    /// Bead ID that was closed (if linked)
    pub bead_id: Option<String>,

    /// Number of files committed
    pub files_committed: usize,

    /// Number of commits merged
    pub commits_merged: usize,

    /// Whether the workspace was merged
    pub merged: bool,

    /// Whether the workspace was cleaned up
    pub cleaned: bool,

    /// Whether the bead was closed
    pub bead_closed: bool,

    /// Whether the session status was updated
    pub session_updated: bool,

    /// New status of the session after done
    pub new_status: Option<String>,

    /// Whether changes were pushed to remote
    pub pushed_to_remote: bool,

    /// Whether this was a dry run
    pub dry_run: bool,

    /// Preview information (only in dry-run mode)
    pub preview: Option<DonePreview>,

    /// Error message (if an error occurred during processing)
    pub error: Option<String>,
}

/// Preview information for dry-run mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DonePreview {
    /// Uncommitted files that would be committed
    pub uncommitted_files: Vec<String>,

    /// Commits that would be merged
    pub commits_to_merge: Vec<CommitInfo>,

    /// Potential conflicts detected
    pub potential_conflicts: Vec<String>,

    /// Bead that would be closed
    pub bead_to_close: Option<String>,

    /// Workspace path
    pub workspace_path: String,

    /// Detailed conflict detection result (when --detect-conflicts is used)
    pub conflict_detection: Option<ConflictDetectionResult>,
}

/// Information about a commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Change ID (JJ)
    pub change_id: String,
    /// Commit ID
    pub commit_id: String,
    /// Commit description/first line
    pub description: String,
    /// Commit timestamp
    pub timestamp: String,
}

/// Comprehensive result of conflict detection.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictDetectionResult {
    /// Whether there are existing JJ conflicts in the workspace
    pub has_existing_conflicts: bool,

    /// List of files with existing JJ conflicts
    pub existing_conflicts: Vec<String>,

    /// Files modified in both workspace and trunk (potential conflicts)
    pub overlapping_files: Vec<String>,

    /// Files modified only in workspace
    pub workspace_only: Vec<String>,

    /// Files modified only in trunk/main
    pub main_only: Vec<String>,

    /// Whether the merge is likely to succeed without conflicts
    pub merge_likely_safe: bool,

    /// Human-readable summary of the detection result
    pub summary: String,

    /// The merge base commit (common ancestor)
    pub merge_base: Option<String>,

    /// Total number of files analyzed
    pub files_analyzed: usize,

    /// Time taken for detection in milliseconds
    pub detection_time_ms: u64,
}

impl ConflictDetectionResult {
    /// Check if any conflicts (existing or potential) were found.
    #[must_use]
    pub const fn has_conflicts(&self) -> bool {
        self.has_existing_conflicts || !self.overlapping_files.is_empty()
    }

    /// Create a result indicating no conflicts were detected.
    #[cfg(test)]
    #[must_use]
    pub fn no_conflicts() -> Self {
        Self {
            merge_likely_safe: true,
            summary: "No conflicts detected - merge is safe".to_string(),
            ..Default::default()
        }
    }
}

/// Undo entry for history log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoEntry {
    /// Session name
    pub session_name: String,
    /// Commit ID after merge
    pub commit_id: String,
    /// Commit ID before merge
    pub pre_merge_commit_id: String,
    /// Unix timestamp
    pub timestamp: u64,
    /// Whether changes were pushed to remote
    pub pushed_to_remote: bool,
    /// Status string
    pub status: String,
}

/// Phase of the done operation where an error occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonePhase {
    /// Initial validation phase (checking workspace location)
    ValidatingLocation,
    /// Commit phase (committing uncommitted changes)
    CommittingChanges,
    /// Merge and cleanup phase
    MergingToMain,
}

impl DonePhase {
    /// Returns the snake_case name of this phase.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::ValidatingLocation => "validating_location",
            Self::CommittingChanges => "committing_changes",
            Self::MergingToMain => "merging_to_main",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- DoneOutput ----

    #[test]
    fn done_output_default_has_empty_workspace_name() {
        let output = DoneOutput::default();
        assert!(output.workspace_name.is_empty());
    }

    #[test]
    fn done_output_default_all_optional_fields_are_none() {
        let output = DoneOutput::default();
        assert!(output.bead_id.is_none());
        assert!(output.new_status.is_none());
        assert!(output.preview.is_none());
        assert!(output.error.is_none());
    }

    #[test]
    fn done_output_default_all_numeric_fields_are_zero() {
        let output = DoneOutput::default();
        assert_eq!(output.files_committed, 0);
        assert_eq!(output.commits_merged, 0);
    }

    #[test]
    fn done_output_default_all_bool_fields_are_false() {
        let output = DoneOutput::default();
        assert!(!output.merged);
        assert!(!output.cleaned);
        assert!(!output.bead_closed);
        assert!(!output.session_updated);
        assert!(!output.pushed_to_remote);
        assert!(!output.dry_run);
    }

    #[test]
    fn done_output_serialization_roundtrip_json() {
        let mut output = DoneOutput::default();
        output.workspace_name = "feature-xyz".to_string();
        output.bead_id = Some("bead-42".to_string());
        output.files_committed = 5;
        output.merged = true;

        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: DoneOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.workspace_name, "feature-xyz");
        assert_eq!(deserialized.bead_id.as_deref(), Some("bead-42"));
        assert_eq!(deserialized.files_committed, 5);
        assert!(deserialized.merged);
    }

    #[test]
    fn done_output_with_error_field_serializes() {
        let mut output = DoneOutput::default();
        output.error = Some("workspace not found".to_string());

        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: DoneOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.error.as_deref(), Some("workspace not found"));
    }

    // ---- DoneOptions ----

    #[test]
    fn done_options_default_all_fields_are_default() {
        let opts = DoneOptions::default();
        assert!(opts.workspace.is_none());
        assert!(opts.message.is_none());
        assert!(!opts.keep_workspace);
        assert!(!opts.squash);
        assert!(!opts.dry_run);
        assert!(!opts.detect_conflicts);
        assert!(!opts.no_bead_update);
    }

    #[test]
    fn done_options_with_explicit_fields() {
        let opts = DoneOptions {
            workspace: Some("my-workspace".to_string()),
            message: Some("my commit msg".to_string()),
            keep_workspace: true,
            squash: true,
            dry_run: true,
            detect_conflicts: true,
            no_bead_update: true,
        };
        assert_eq!(opts.workspace.as_deref(), Some("my-workspace"));
        assert_eq!(opts.message.as_deref(), Some("my commit msg"));
        assert!(opts.keep_workspace);
        assert!(opts.squash);
        assert!(opts.dry_run);
        assert!(opts.detect_conflicts);
        assert!(opts.no_bead_update);
    }

    // ---- DonePreview ----

    #[test]
    fn done_preview_construction_with_all_fields() {
        let preview = DonePreview {
            uncommitted_files: vec!["file1.rs".to_string(), "file2.rs".to_string()],
            commits_to_merge: vec![CommitInfo {
                change_id: "abc123".to_string(),
                commit_id: "def456".to_string(),
                description: "feat: add something".to_string(),
                timestamp: "2025-01-01 00:00:00".to_string(),
            }],
            potential_conflicts: vec![],
            bead_to_close: Some("bead-7".to_string()),
            workspace_path: "/tmp/ws".to_string(),
            conflict_detection: None,
        };
        assert_eq!(preview.uncommitted_files.len(), 2);
        assert_eq!(preview.commits_to_merge.len(), 1);
        assert!(preview.potential_conflicts.is_empty());
        assert_eq!(preview.bead_to_close.as_deref(), Some("bead-7"));
        assert!(preview.conflict_detection.is_none());
    }

    #[test]
    fn done_preview_empty_construction() {
        let preview = DonePreview {
            uncommitted_files: vec![],
            commits_to_merge: vec![],
            potential_conflicts: vec![],
            bead_to_close: None,
            workspace_path: String::new(),
            conflict_detection: None,
        };
        assert!(preview.uncommitted_files.is_empty());
        assert!(preview.commits_to_merge.is_empty());
        assert!(preview.bead_to_close.is_none());
    }

    #[test]
    fn done_preview_serialization_roundtrip() {
        let preview = DonePreview {
            uncommitted_files: vec!["a.rs".to_string()],
            commits_to_merge: vec![],
            potential_conflicts: vec!["conflict.rs".to_string()],
            bead_to_close: None,
            workspace_path: "/ws".to_string(),
            conflict_detection: Some(ConflictDetectionResult::no_conflicts()),
        };
        let json = serde_json::to_string(&preview).expect("serialize");
        let deserialized: DonePreview =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.uncommitted_files.len(), 1);
        assert_eq!(deserialized.potential_conflicts.len(), 1);
        assert!(deserialized.conflict_detection.is_some());
    }

    // ---- CommitInfo ----

    #[test]
    fn commit_info_construction_and_field_access() {
        let info = CommitInfo {
            change_id: "change-abc".to_string(),
            commit_id: "commit-xyz".to_string(),
            description: "initial commit".to_string(),
            timestamp: "2025-06-15 12:00:00".to_string(),
        };
        assert_eq!(info.change_id, "change-abc");
        assert_eq!(info.commit_id, "commit-xyz");
        assert_eq!(info.description, "initial commit");
        assert_eq!(info.timestamp, "2025-06-15 12:00:00");
    }

    #[test]
    fn commit_info_serialization_roundtrip() {
        let info = CommitInfo {
            change_id: "ch".to_string(),
            commit_id: "cm".to_string(),
            description: "d".to_string(),
            timestamp: "t".to_string(),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let deserialized: CommitInfo =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.change_id, "ch");
    }

    // ---- ConflictDetectionResult ----

    #[test]
    fn conflict_detection_result_default_has_no_conflicts() {
        let result = ConflictDetectionResult::default();
        assert!(!result.has_conflicts());
        assert!(!result.has_existing_conflicts);
        assert!(result.overlapping_files.is_empty());
    }

    #[test]
    fn conflict_detection_result_no_conflicts_helper() {
        let result = ConflictDetectionResult::no_conflicts();
        assert!(!result.has_conflicts());
        assert!(result.merge_likely_safe);
        assert!(!result.has_existing_conflicts);
        assert!(result.overlapping_files.is_empty());
    }

    #[test]
    fn conflict_detection_has_conflicts_when_existing() {
        let result = ConflictDetectionResult {
            has_existing_conflicts: true,
            existing_conflicts: vec!["file.rs".to_string()],
            ..Default::default()
        };
        assert!(result.has_conflicts());
    }

    #[test]
    fn conflict_detection_has_conflicts_when_overlapping() {
        let result = ConflictDetectionResult {
            overlapping_files: vec!["lib.rs".to_string(), "main.rs".to_string()],
            ..Default::default()
        };
        assert!(result.has_conflicts());
    }

    #[test]
    fn conflict_detection_no_conflicts_when_workspace_only_modified() {
        let result = ConflictDetectionResult {
            workspace_only: vec!["new_file.rs".to_string()],
            main_only: vec!["other_file.rs".to_string()],
            ..Default::default()
        };
        assert!(!result.has_conflicts());
    }

    #[test]
    fn conflict_detection_has_conflicts_both_existing_and_overlapping() {
        let result = ConflictDetectionResult {
            has_existing_conflicts: true,
            existing_conflicts: vec!["a.rs".to_string()],
            overlapping_files: vec!["b.rs".to_string()],
            ..Default::default()
        };
        assert!(result.has_conflicts());
    }

    #[test]
    fn conflict_detection_serialization_roundtrip() {
        let result = ConflictDetectionResult {
            has_existing_conflicts: false,
            existing_conflicts: vec![],
            overlapping_files: vec!["shared.rs".to_string()],
            workspace_only: vec!["ws_only.rs".to_string()],
            main_only: vec![],
            merge_likely_safe: false,
            summary: "1 potential conflict".to_string(),
            merge_base: Some("abc123".to_string()),
            files_analyzed: 3,
            detection_time_ms: 42,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: ConflictDetectionResult =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.overlapping_files.len(), 1);
        assert_eq!(deserialized.files_analyzed, 3);
        assert_eq!(deserialized.detection_time_ms, 42);
        assert_eq!(deserialized.merge_base.as_deref(), Some("abc123"));
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
            status: "success".to_string(),
        };
        assert_eq!(entry.session_name, "feature-x");
        assert_eq!(entry.commit_id, "sha-after");
        assert_eq!(entry.pre_merge_commit_id, "sha-before");
        assert_eq!(entry.timestamp, 1_700_000_000);
        assert!(entry.pushed_to_remote);
        assert_eq!(entry.status, "success");
    }

    #[test]
    fn undo_entry_serialization_roundtrip() {
        let entry = UndoEntry {
            session_name: "ws-1".to_string(),
            commit_id: "c1".to_string(),
            pre_merge_commit_id: "c0".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: "ok".to_string(),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let deserialized: UndoEntry =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert!(!deserialized.pushed_to_remote);
        assert_eq!(deserialized.session_name, "ws-1");
    }

    // ---- DonePhase ----

    #[test]
    fn done_phase_validating_location_name() {
        assert_eq!(DonePhase::ValidatingLocation.name(), "validating_location");
    }

    #[test]
    fn done_phase_committing_changes_name() {
        assert_eq!(DonePhase::CommittingChanges.name(), "committing_changes");
    }

    #[test]
    fn done_phase_merging_to_main_name() {
        assert_eq!(DonePhase::MergingToMain.name(), "merging_to_main");
    }

    #[test]
    fn done_phase_all_variants_exhaustive_match() {
        let phases = [
            DonePhase::ValidatingLocation,
            DonePhase::CommittingChanges,
            DonePhase::MergingToMain,
        ];
        for phase in &phases {
            let name = phase.name();
            assert!(name.contains('_'));
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn done_phase_equality() {
        assert_eq!(DonePhase::ValidatingLocation, DonePhase::ValidatingLocation);
        assert_ne!(DonePhase::ValidatingLocation, DonePhase::CommittingChanges);
    }
}
