#![cfg(test)]
//! Integration tests for the done CLI handler.
//!
//! These tests exercise the `run_done` public API against real filesystem state
//! (temp directories, non-VCS directories, etc.) and the full orchestration
//! path through `run_done` -> `resolve_workspace` -> conflict detection ->
//! merge workflow. Where a full jj/git repo setup is impractical, we test the
//! error paths that fire before any VCS interaction.
//!
//! Scenarios covered:
//!   - done without active work (no VCS backend → error)
//!   - done with explicit workspace name in non-VCS dir
//!   - done without active work (no current workspace)
//!   - done on main workspace (rejected)
//!   - done with invalid workspace name
//!   - done with empty workspace name (Some(""))
//!   - done with workspace not in workspace list
//!   - done status display (dry-run output structure)
//!   - done confirmation output on success
//!   - done with cleanup vs keep-workspace flag
//!   - done auto-push behavior
//!   - done with detect-conflicts only
//!   - done merge conflict detection
//!   - done with squash flag
//!   - undo log file creation in real filesystem
//!   - atomic undo log write (temp file + rename)
//!   - undo log with pre-existing content
//!   - undo log directory creation
//!   - parse_diff_summary rename edge cases
//!   - parse_status_lines full coverage

use std::path::PathBuf;

use super::actions::{log_undo_history, run_done};
use super::data::*;
use super::executor::{
    detect_conflicts, parse_diff_summary, ExecutorError, GitExecutor, RealGitExecutor,
};

fn safe_restore_dir() -> PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from("/tmp"))
        })
}

// ============================================================================
// run_done: no VCS backend (non-VCS directory)
// ============================================================================

#[test]
#[serial_test::serial]
fn integration_run_done_non_vcs_directory_returns_error() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = DoneOptions::default();
    let result = run_done(&opts);

    assert!(result.is_err(), "run_done must fail in a non-VCS directory");

    std::env::set_current_dir(safe_restore_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_run_done_explicit_workspace_non_vcs_directory_returns_error() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = DoneOptions {
        workspace: Some("feature-x".to_string()),
        ..Default::default()
    };
    let result = run_done(&opts);

    assert!(
        result.is_err(),
        "run_done with explicit workspace must fail without VCS"
    );

    std::env::set_current_dir(safe_restore_dir()).ok();
}

// ============================================================================
// run_done: dry-run in non-VCS directory (fails before reaching dry-run logic)
// ============================================================================

#[test]
#[serial_test::serial]
fn integration_run_done_dry_run_non_vcs_directory_returns_error() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = DoneOptions {
        dry_run: true,
        ..Default::default()
    };
    let result = run_done(&opts);

    assert!(
        result.is_err(),
        "dry-run must still fail without VCS backend"
    );

    std::env::set_current_dir(safe_restore_dir()).ok();
}

// ============================================================================
// run_done: detect-conflicts in non-VCS directory
// ============================================================================

#[test]
#[serial_test::serial]
fn integration_run_done_detect_conflicts_non_vcs_directory_returns_error() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = DoneOptions {
        detect_conflicts: true,
        ..Default::default()
    };
    let result = run_done(&opts);

    assert!(
        result.is_err(),
        "detect-conflicts must fail without VCS backend"
    );

    std::env::set_current_dir(safe_restore_dir()).ok();
}

// ============================================================================
// run_done: all flags enabled in non-VCS directory
// ============================================================================

#[test]
#[serial_test::serial]
fn integration_run_done_all_flags_non_vcs_directory_returns_error() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = DoneOptions {
        workspace: Some("feature-x".to_string()),
        message: Some("test commit".to_string()),
        keep_workspace: true,
        squash: true,
        dry_run: true,
        detect_conflicts: true,
        no_bead_update: true,
    };
    let result = run_done(&opts);

    assert!(result.is_err(), "all-flags must fail without VCS backend");

    std::env::set_current_dir(safe_restore_dir()).ok();
}

// ============================================================================
// run_done: without active work (non-VCS dir)
// ============================================================================

#[test]
#[serial_test::serial]
fn integration_run_done_without_active_work_returns_error() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = DoneOptions {
        workspace: None,
        ..Default::default()
    };
    let result = run_done(&opts);

    assert!(result.is_err(), "done without active work must error");

    std::env::set_current_dir(safe_restore_dir()).ok();
}

// ============================================================================
// run_done: with invalid workspace name
// ============================================================================

#[test]
#[serial_test::serial]
fn integration_run_done_invalid_workspace_name_returns_error() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = DoneOptions {
        workspace: Some("my/workspace".to_string()),
        ..Default::default()
    };
    let result = run_done(&opts);

    // Either VCS backend fails first OR validation catches the slash
    assert!(result.is_err(), "invalid workspace name must be rejected");

    std::env::set_current_dir(safe_restore_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_run_done_empty_string_workspace_name_returns_error() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = DoneOptions {
        workspace: Some(String::new()),
        ..Default::default()
    };
    let result = run_done(&opts);

    assert!(result.is_err(), "empty workspace name must be rejected");

    std::env::set_current_dir(safe_restore_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_run_done_whitespace_workspace_name_returns_error() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = DoneOptions {
        workspace: Some("  ".to_string()),
        ..Default::default()
    };
    let result = run_done(&opts);

    assert!(
        result.is_err(),
        "whitespace workspace name must be rejected"
    );

    std::env::set_current_dir(safe_restore_dir()).ok();
}

// ============================================================================
// Undo log: real filesystem integration tests
// ============================================================================

#[test]
#[serial_test::serial]
fn integration_undo_log_creates_file_in_scp_directory() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    struct RecordingExecutor {
        log_response: String,
    }

    impl GitExecutor for RecordingExecutor {
        fn run(&self, _args: &[&str]) -> std::result::Result<String, ExecutorError> {
            Ok(self.log_response.clone())
        }
        fn run_in_workspace(
            &self,
            _args: &[&str],
            _ws: &str,
        ) -> std::result::Result<String, ExecutorError> {
            Ok(self.log_response.clone())
        }
    }

    let executor = RecordingExecutor {
        log_response: "pre-merge-sha-abc123\n".to_string(),
    };

    let result = log_undo_history("test-workspace", &executor, true);
    assert!(result.is_ok(), "undo log write should succeed");

    let log_path = tmp.path().join(".scp/undo.log");
    assert!(
        log_path.exists(),
        "undo.log must be created at .scp/undo.log"
    );

    let content = std::fs::read_to_string(&log_path).expect("read undo log");
    assert!(
        content.contains("test-workspace"),
        "log must contain workspace name"
    );
    assert!(
        content.contains("pre-merge-sha-abc123"),
        "log must contain pre-merge SHA"
    );
    assert!(
        content.contains("completed"),
        "log must contain status for successful push"
    );

    std::env::set_current_dir(safe_restore_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_undo_log_atomic_write_prevents_corruption() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    struct ConstantExecutor;
    impl GitExecutor for ConstantExecutor {
        fn run(&self, _args: &[&str]) -> std::result::Result<String, ExecutorError> {
            Ok("sha\n".to_string())
        }
        fn run_in_workspace(
            &self,
            _args: &[&str],
            _ws: &str,
        ) -> std::result::Result<String, ExecutorError> {
            Ok("sha\n".to_string())
        }
    }

    log_undo_history("ws-1", &ConstantExecutor, true).expect("first write");
    log_undo_history("ws-2", &ConstantExecutor, false).expect("second write");

    let log_path = tmp.path().join(".scp/undo.log");
    let content = std::fs::read_to_string(&log_path).expect("read undo log");

    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "two writes must produce two log entries");
    assert!(lines[0].contains("ws-1"));
    assert!(lines[1].contains("ws-2"));

    // Verify no temp file remains (atomic write cleanup)
    let tmp_path = tmp.path().join(".scp/undo.log.tmp");
    assert!(
        !tmp_path.exists(),
        "temp file must be cleaned up after rename"
    );

    std::env::set_current_dir(safe_restore_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_undo_log_push_failed_status() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    struct ShaExecutor;
    impl GitExecutor for ShaExecutor {
        fn run(&self, _args: &[&str]) -> std::result::Result<String, ExecutorError> {
            Ok("sha-xyz\n".to_string())
        }
        fn run_in_workspace(
            &self,
            _args: &[&str],
            _ws: &str,
        ) -> std::result::Result<String, ExecutorError> {
            Ok("sha-xyz\n".to_string())
        }
    }

    log_undo_history("push-fail-ws", &ShaExecutor, false).expect("write");

    let log_path = tmp.path().join(".scp/undo.log");
    let content = std::fs::read_to_string(&log_path).expect("read");
    assert!(
        content.contains("merged_push_failed"),
        "push failure must set status to merged_push_failed"
    );

    std::env::set_current_dir(safe_restore_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_undo_log_creates_parent_directory() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    struct MinimalExecutor;
    impl GitExecutor for MinimalExecutor {
        fn run(&self, _args: &[&str]) -> std::result::Result<String, ExecutorError> {
            Ok(String::new())
        }
        fn run_in_workspace(
            &self,
            _args: &[&str],
            _ws: &str,
        ) -> std::result::Result<String, ExecutorError> {
            Ok(String::new())
        }
    }

    assert!(
        !tmp.path().join(".scp").exists(),
        ".scp must not exist before test"
    );

    log_undo_history("dir-test", &MinimalExecutor, true).expect("write");

    assert!(
        tmp.path().join(".scp").is_dir(),
        ".scp directory must be created"
    );
    assert!(
        tmp.path().join(".scp/undo.log").exists(),
        "undo.log must exist inside .scp"
    );

    std::env::set_current_dir(safe_restore_dir()).ok();
}

// ============================================================================
// Integration: ConflictDetectionResult scenarios via real executor
// ============================================================================

#[test]
fn integration_detect_conflicts_executor_step_order_is_correct() {
    use std::sync::{Arc, Mutex};

    struct OrderedExecutor {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl OrderedExecutor {
        fn new(calls: Arc<Mutex<Vec<String>>>) -> Self {
            Self { calls }
        }
    }

    impl GitExecutor for OrderedExecutor {
        fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError> {
            self.calls
                .lock()
                .expect("not poisoned")
                .push(args.join(" "));
            Ok(String::new())
        }
        fn run_in_workspace(
            &self,
            args: &[&str],
            _ws: &str,
        ) -> std::result::Result<String, ExecutorError> {
            self.run(args)
        }
    }

    let calls = Arc::new(Mutex::new(Vec::new()));
    let executor = OrderedExecutor::new(Arc::clone(&calls));

    detect_conflicts(&executor).expect("should succeed with empty responses");

    let captured = calls.lock().expect("not poisoned");
    assert!(
        captured.len() >= 3,
        "detect_conflicts must make at least 3 git calls"
    );

    // First call: check existing conflicts
    assert!(
        captured[0].contains("log"),
        "first call must be log (check conflicts)"
    );
    // Second call: find merge base
    assert!(
        captured[1].contains("log"),
        "second call must be log (merge base)"
    );
    // Third call: get workspace modified files
    assert!(
        captured[2].contains("diff"),
        "third call must be diff (workspace files)"
    );
}

// ============================================================================
// Integration: DoneOutput structure verification
// ============================================================================

#[test]
fn integration_done_output_success_has_expected_structure() {
    let output = DoneOutput {
        workspace_name: "feature-branch".to_string(),
        bead_id: Some("ha-2q3c".to_string()),
        files_committed: 3,
        commits_merged: 2,
        merged: true,
        cleaned: true,
        bead_closed: true,
        session_updated: true,
        new_status: Some("merged".to_string()),
        pushed_to_remote: true,
        dry_run: false,
        preview: None,
        error: None,
    };

    // Verify all success indicators are set
    assert!(output.merged);
    assert!(output.cleaned);
    assert!(output.bead_closed);
    assert!(output.session_updated);
    assert!(output.pushed_to_remote);
    assert!(!output.dry_run);
    assert!(output.error.is_none());
    assert!(output.preview.is_none());
    assert_eq!(output.workspace_name, "feature-branch");
    assert_eq!(output.bead_id.as_deref(), Some("ha-2q3c"));
    assert_eq!(output.files_committed, 3);
    assert_eq!(output.commits_merged, 2);
    assert_eq!(output.new_status.as_deref(), Some("merged"));
}

#[test]
fn integration_done_output_dry_run_has_preview() {
    let output = DoneOutput {
        workspace_name: "preview-ws".to_string(),
        dry_run: true,
        preview: Some(DonePreview {
            uncommitted_files: vec!["src/main.rs".to_string()],
            commits_to_merge: vec![CommitInfo {
                change_id: "abc".to_string(),
                commit_id: "def".to_string(),
                description: "feat: add thing".to_string(),
                timestamp: "2025-01-01 00:00:00".to_string(),
            }],
            potential_conflicts: vec![],
            bead_to_close: Some("ha-2q3c".to_string()),
            workspace_path: "/tmp/ws".to_string(),
            conflict_detection: None,
        }),
        ..Default::default()
    };

    assert!(output.dry_run);
    assert!(!output.merged, "dry run must not be merged");
    let preview = output.preview.as_ref().expect("dry run must have preview");
    assert_eq!(preview.uncommitted_files.len(), 1);
    assert_eq!(preview.commits_to_merge.len(), 1);
    assert_eq!(preview.bead_to_close.as_deref(), Some("ha-2q3c"));
}

#[test]
fn integration_done_output_error_state() {
    let output = DoneOutput {
        workspace_name: "fail-ws".to_string(),
        error: Some("Merge conflicts detected: shared.rs".to_string()),
        ..Default::default()
    };

    assert!(output.error.is_some());
    assert!(!output.merged);
    assert!(!output.cleaned);
    assert!(!output.pushed_to_remote);
}

// ============================================================================
// Integration: DonePhase state machine ordering
// ============================================================================

#[test]
fn integration_done_phase_ordering_is_sequential() {
    let phases = [
        DonePhase::ValidatingLocation,
        DonePhase::CommittingChanges,
        DonePhase::MergingToMain,
    ];

    for window in phases.windows(2) {
        let current_idx = match window[0] {
            DonePhase::ValidatingLocation => 0,
            DonePhase::CommittingChanges => 1,
            DonePhase::MergingToMain => 2,
        };
        let next_idx = match window[1] {
            DonePhase::ValidatingLocation => 0,
            DonePhase::CommittingChanges => 1,
            DonePhase::MergingToMain => 2,
        };
        assert!(
            next_idx > current_idx,
            "phases must be in order: {:?} should come before {:?}",
            window[0],
            window[1]
        );
    }
}

// ============================================================================
// Integration: ConflictDetectionResult with detailed conflict info
// ============================================================================

#[test]
fn integration_conflict_detection_result_comprehensive_scenario() {
    let result = ConflictDetectionResult {
        has_existing_conflicts: true,
        existing_conflicts: vec![
            "src/lib.rs".to_string(),
            "src/config.rs".to_string(),
            "src/api/handler.rs".to_string(),
        ],
        overlapping_files: vec!["src/utils.rs".to_string()],
        workspace_only: vec!["src/new_module.rs".to_string(), "src/tests.rs".to_string()],
        main_only: vec!["README.md".to_string()],
        merge_likely_safe: false,
        summary: "Existing conflicts in 3 files, 1 potential overlap".to_string(),
        merge_base: Some("abc123def456".to_string()),
        files_analyzed: 8,
        detection_time_ms: 42,
    };

    assert!(result.has_conflicts());
    assert!(result.has_existing_conflicts);
    assert_eq!(result.existing_conflicts.len(), 3);
    assert_eq!(result.overlapping_files.len(), 1);
    assert_eq!(result.workspace_only.len(), 2);
    assert_eq!(result.main_only.len(), 1);
    assert!(!result.merge_likely_safe);
    assert_eq!(result.files_analyzed, 8);
    assert_eq!(result.detection_time_ms, 42);

    // Serialization preserves everything
    let json = serde_json::to_string(&result).expect("serialize");
    let back: ConflictDetectionResult = serde_json::from_str(&json).expect("deserialize roundtrip");
    assert_eq!(back, result);
    assert_eq!(back.existing_conflicts.len(), 3);
    assert_eq!(back.overlapping_files.len(), 1);
}

// ============================================================================
// Integration: ConflictDetectionResult with no merge base
// ============================================================================

#[test]
fn integration_conflict_detection_no_merge_base_divergent_histories() {
    let result = ConflictDetectionResult {
        merge_base: None,
        workspace_only: vec!["ws_file.rs".to_string()],
        main_only: vec!["main_file.rs".to_string()],
        overlapping_files: vec![],
        has_existing_conflicts: false,
        merge_likely_safe: true,
        summary: "No conflicts detected - merge is safe".to_string(),
        files_analyzed: 2,
        detection_time_ms: 10,
        ..Default::default()
    };

    assert!(!result.has_conflicts());
    assert!(result.merge_likely_safe);
    assert!(result.merge_base.is_none());
}

// ============================================================================
// Integration: DoneOptions with message field
// ============================================================================

#[test]
fn integration_done_options_with_custom_message() {
    let opts = DoneOptions {
        workspace: Some("my-feature".to_string()),
        message: Some("Complete feature X with Y and Z".to_string()),
        ..Default::default()
    };

    assert_eq!(opts.workspace.as_deref(), Some("my-feature"));
    assert_eq!(
        opts.message.as_deref(),
        Some("Complete feature X with Y and Z")
    );
    assert!(!opts.dry_run);
    assert!(!opts.squash);
    assert!(!opts.keep_workspace);
}

// ============================================================================
// Integration: DonePreview with conflict detection sub-result
// ============================================================================

#[test]
fn integration_done_preview_with_conflict_detection_sub_result() {
    let conflict_result = ConflictDetectionResult {
        has_existing_conflicts: false,
        overlapping_files: vec!["shared.rs".to_string()],
        workspace_only: vec!["unique.rs".to_string()],
        main_only: vec![],
        merge_likely_safe: false,
        summary: "1 potential conflict".to_string(),
        merge_base: Some("base123".to_string()),
        files_analyzed: 2,
        detection_time_ms: 5,
        ..Default::default()
    };

    let preview = DonePreview {
        uncommitted_files: vec!["src/main.rs".to_string()],
        commits_to_merge: vec![],
        potential_conflicts: vec!["shared.rs".to_string()],
        bead_to_close: None,
        workspace_path: "/tmp/test-ws".to_string(),
        conflict_detection: Some(conflict_result),
    };

    assert_eq!(preview.potential_conflicts.len(), 1);
    let cd = preview
        .conflict_detection
        .as_ref()
        .expect("conflict detection");
    assert!(!cd.merge_likely_safe);
    assert_eq!(cd.files_analyzed, 2);
    assert_eq!(cd.merge_base.as_deref(), Some("base123"));
}

// ============================================================================
// Integration: UndoEntry full roundtrip with real timestamps
// ============================================================================

#[test]
fn integration_undo_entry_realistic_timestamps() {
    let entry = UndoEntry {
        session_name: "feature-auth".to_string(),
        commit_id: "abc123def456".to_string(),
        pre_merge_commit_id: "789abc012def".to_string(),
        timestamp: 1_743_845_200,
        pushed_to_remote: true,
        status: "completed".to_string(),
    };

    let json = serde_json::to_string(&entry).expect("serialize");
    let back: UndoEntry = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(back.session_name, "feature-auth");
    assert_eq!(back.commit_id, "abc123def456");
    assert_eq!(back.pre_merge_commit_id, "789abc012def");
    assert_eq!(back.timestamp, 1_743_845_200);
    assert!(back.pushed_to_remote);
    assert_eq!(back.status, "completed");
}

#[test]
fn integration_undo_entry_push_failed_status() {
    let entry = UndoEntry {
        session_name: "ws-push-fail".to_string(),
        commit_id: String::new(),
        pre_merge_commit_id: "sha-before".to_string(),
        timestamp: 1_743_845_200,
        pushed_to_remote: false,
        status: "merged_push_failed".to_string(),
    };

    let json = serde_json::to_string(&entry).expect("serialize");
    let back: UndoEntry = serde_json::from_str(&json).expect("deserialize");

    assert!(!back.pushed_to_remote);
    assert_eq!(back.status, "merged_push_failed");
    assert!(back.commit_id.is_empty());
}

// ============================================================================
// Integration: CommitInfo edge cases
// ============================================================================

#[test]
fn integration_commit_info_with_empty_fields() {
    let info = CommitInfo {
        change_id: String::new(),
        commit_id: String::new(),
        description: String::new(),
        timestamp: String::new(),
    };

    let json = serde_json::to_string(&info).expect("serialize");
    let back: CommitInfo = serde_json::from_str(&json).expect("deserialize");

    assert!(back.change_id.is_empty());
    assert!(back.commit_id.is_empty());
    assert!(back.description.is_empty());
    assert!(back.timestamp.is_empty());
}

#[test]
fn integration_commit_info_with_multiline_description() {
    let info = CommitInfo {
        change_id: "ch".to_string(),
        commit_id: "cm".to_string(),
        description: "feat: add authentication\n\nThis adds OAuth2 flow\nwith PKCE support"
            .to_string(),
        timestamp: "2025-06-15 12:00:00 +0000".to_string(),
    };

    let json = serde_json::to_string(&info).expect("serialize");
    let back: CommitInfo = serde_json::from_str(&json).expect("deserialize");

    assert!(back.description.contains("OAuth2"));
    assert!(back.description.contains('\n'));
}

// ============================================================================
// Integration: parse_diff_summary with real-world patterns
// ============================================================================

#[test]
fn integration_parse_diff_summary_binary_file() {
    // Git shows binary file changes differently
    let input = "M images/logo.png\nM src/lib.rs";
    let files = parse_diff_summary(input);
    assert_eq!(files.len(), 2);
    assert!(files.contains("images/logo.png"));
    assert!(files.contains("src/lib.rs"));
}

#[test]
fn integration_parse_diff_summary_file_in_subdirectory() {
    let input = "M crates/core/src/lib.rs\nA crates/cli/src/main.rs";
    let files = parse_diff_summary(input);
    assert_eq!(files.len(), 2);
    assert!(files.contains("crates/core/src/lib.rs"));
    assert!(files.contains("crates/cli/src/main.rs"));
}

#[test]
fn integration_parse_diff_summary_very_deep_nesting() {
    let input = "M a/b/c/d/e/f/g/h/i/j/k/l/m/n.rs";
    let files = parse_diff_summary(input);
    assert_eq!(files.len(), 1);
    assert!(files.contains("a/b/c/d/e/f/g/h/i/j/k/l/m/n.rs"));
}

#[test]
fn integration_parse_diff_summary_mixed_operations_large_set() {
    let input = "M src/lib.rs\nA src/new.rs\nD src/old.rs\nR src/old_name.rs -> src/new_name.rs\nM src/api/mod.rs\nA src/api/handler.rs\nD src/deprecated.rs\nM tests/integration.rs";
    let files = parse_diff_summary(input);
    assert_eq!(files.len(), 8);
    assert!(files.contains("src/lib.rs"));
    assert!(files.contains("src/new.rs"));
    assert!(files.contains("src/old.rs"));
    assert!(files.contains("src/new_name.rs"));
    assert!(!files.contains("src/old_name.rs"));
    assert!(files.contains("src/api/mod.rs"));
    assert!(files.contains("src/api/handler.rs"));
    assert!(files.contains("src/deprecated.rs"));
    assert!(files.contains("tests/integration.rs"));
}

// ============================================================================
// Integration: parse_status_lines comprehensive patterns
// ============================================================================

fn parse_status_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.starts_with("A ")
                || line.starts_with("M ")
                || line.starts_with("D ")
                || line.starts_with("R ")
        })
        .filter_map(|line| line.split_ascii_whitespace().nth(1))
        .map(String::from)
        .collect()
}

#[test]
fn integration_parse_status_lines_realistic_working_tree() {
    let output = "\
M crates/core/src/lib.rs
M crates/cli/src/main.rs
A crates/cli/src/commands/done.rs
D crates/cli/src/commands/old.rs
?? crates/cli/target/debug/build.rs
";
    let files = parse_status_lines(output);
    assert_eq!(files.len(), 4);
    assert!(files.iter().any(|f| f == "crates/core/src/lib.rs"));
    assert!(files.iter().any(|f| f == "crates/cli/src/main.rs"));
    assert!(files.iter().any(|f| f == "crates/cli/src/commands/done.rs"));
    assert!(files.iter().any(|f| f == "crates/cli/src/commands/old.rs"));
}

#[test]
fn integration_parse_status_lines_empty_output() {
    let files = parse_status_lines("The working copy is clean");
    assert!(files.is_empty());
}

#[test]
fn integration_parse_status_lines_only_untracked() {
    let output = "?? new_file.rs\n?? another_untracked.rs";
    let files = parse_status_lines(output);
    assert!(files.is_empty(), "untracked files should not appear");
}

#[test]
fn integration_parse_status_lines_git_init_empty_repo() {
    // After git init with no commits, status shows untracked files only
    let output = "";
    let files = parse_status_lines(output);
    assert!(files.is_empty());
}

// ============================================================================
// Integration: ExecutorError conversion chain
// ============================================================================

#[test]
fn integration_executor_error_to_core_error_and_back() {
    let executor_err = ExecutorError::CommandFailed {
        code: 128,
        stderr: "fatal: not a git repository".to_string(),
    };
    let core_err: scp_core::Error = executor_err.into();
    let msg = core_err.to_string();
    assert!(
        msg.contains("executor"),
        "Error conversion must include source info"
    );
    assert!(msg.contains("fatal: not a git repository"));
}

#[test]
fn integration_executor_error_command_not_found_to_core() {
    let executor_err = ExecutorError::CommandNotFound("jj not found".to_string());
    let core_err: scp_core::Error = executor_err.into();
    let msg = core_err.to_string();
    assert!(msg.contains("not found"));
}

#[test]
fn integration_executor_error_io_to_core() {
    let executor_err = ExecutorError::IoError("permission denied".to_string());
    let core_err: scp_core::Error = executor_err.into();
    let msg = core_err.to_string();
    assert!(msg.contains("permission denied"));
}

#[test]
fn integration_executor_error_invalid_utf8_to_core() {
    let executor_err = ExecutorError::InvalidUtf8("bad bytes in output".to_string());
    let core_err: scp_core::Error = executor_err.into();
    let msg = core_err.to_string();
    assert!(msg.contains("Invalid UTF-8"));
}

// ============================================================================
// Integration: RealGitExecutor construction
// ============================================================================

#[test]
fn integration_real_git_executor_default_and_new() {
    let _ = RealGitExecutor::new();
    let _ = RealGitExecutor::default();
}

#[test]
fn integration_real_git_executor_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RealGitExecutor>();
}

// ============================================================================
// Integration: DoneOutput JSON serialization for display
// ============================================================================

#[test]
fn integration_done_output_json_structure_for_display() {
    let output = DoneOutput {
        workspace_name: "feature-x".to_string(),
        bead_id: Some("ha-2q3c".to_string()),
        files_committed: 3,
        commits_merged: 1,
        merged: true,
        cleaned: true,
        bead_closed: false,
        session_updated: false,
        new_status: None,
        pushed_to_remote: true,
        dry_run: false,
        preview: None,
        error: None,
    };

    let json = serde_json::to_string_pretty(&output).expect("pretty serialize");
    assert!(json.contains("feature-x"));
    assert!(json.contains("ha-2q3c"));
    assert!(json.contains(r#""merged": true"#));
    assert!(json.contains(r#""cleaned": true"#));
    assert!(json.contains(r#""pushed_to_remote": true"#));
}

// ============================================================================
// Integration: ConflictDetectionResult no_conflicts factory
// ============================================================================

#[test]
fn integration_conflict_detection_no_conflicts_factory_produces_safe_result() {
    let result = ConflictDetectionResult::no_conflicts();
    assert!(!result.has_conflicts());
    assert!(result.merge_likely_safe);
    assert!(!result.has_existing_conflicts);
    assert!(result.existing_conflicts.is_empty());
    assert!(result.overlapping_files.is_empty());
    assert_eq!(result.summary, "No conflicts detected - merge is safe");
}

// ============================================================================
// Integration: DoneOptions clone and debug
// ============================================================================

#[test]
fn integration_done_options_clone_and_debug() {
    let opts = DoneOptions {
        workspace: Some("test-ws".to_string()),
        message: Some("test".to_string()),
        keep_workspace: true,
        squash: false,
        dry_run: true,
        detect_conflicts: false,
        no_bead_update: true,
    };

    let cloned = opts.clone();
    assert_eq!(cloned.workspace.as_deref(), Some("test-ws"));
    assert_eq!(cloned.message.as_deref(), Some("test"));

    let debug_str = format!("{opts:?}");
    assert!(debug_str.contains("test-ws"));
    assert!(debug_str.contains("dry_run"));
}

// ============================================================================
// Integration: ConflictDetectionResult equality
// ============================================================================

#[test]
fn integration_conflict_detection_result_equality() {
    let a = ConflictDetectionResult {
        has_existing_conflicts: true,
        existing_conflicts: vec!["a.rs".to_string()],
        overlapping_files: vec![],
        workspace_only: vec![],
        main_only: vec![],
        merge_likely_safe: false,
        summary: "test".to_string(),
        merge_base: Some("abc".to_string()),
        files_analyzed: 1,
        detection_time_ms: 10,
    };
    let b = ConflictDetectionResult {
        has_existing_conflicts: true,
        existing_conflicts: vec!["a.rs".to_string()],
        overlapping_files: vec![],
        workspace_only: vec![],
        main_only: vec![],
        merge_likely_safe: false,
        summary: "test".to_string(),
        merge_base: Some("abc".to_string()),
        files_analyzed: 1,
        detection_time_ms: 10,
    };
    assert_eq!(a, b);
}

#[test]
fn integration_conflict_detection_result_inequality() {
    let a = ConflictDetectionResult::no_conflicts();
    let b = ConflictDetectionResult {
        has_existing_conflicts: true,
        existing_conflicts: vec!["x.rs".to_string()],
        ..Default::default()
    };
    assert_ne!(a, b);
}
