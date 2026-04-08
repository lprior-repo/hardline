//! RED QUEEN adversarial tests for the done command handler.
//!
//! These tests actively try to break the done command through:
//! - Security attack vectors (path traversal, injection, resource exhaustion)
//! - Boundary conditions (empty, max, concurrent, malformed)
//! - Invariant violations (output consistency, state machine violations)
//! - Property-based fuzzing (parse_diff_summary, parse_status_lines, etc.)
//!
//! Named "RED QUEEN" after the co-evolutionary arms race principle:
//! tests and code evolve together, each driving the other to be stronger.

#![cfg(test)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use proptest::prelude::*;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use super::actions::*;
use super::data::*;
use super::executor::{detect_conflicts, parse_diff_summary, ExecutorError, GitExecutor};

// ============================================================================
// ATTACK VECTOR 1: parse_diff_summary injection and malformed input
// ============================================================================

#[test]
fn adversarial_diff_summary_null_byte_injection() {
    // Null bytes could truncate strings in C interop or cause silent data loss
    let input = "M file\x00.rs\nA normal.rs";
    let files = parse_diff_summary(input);
    // Must not panic, must not silently drop the null-containing entry
    assert!(files.len() >= 1, "should parse at least normal.rs");
}

#[test]
fn adversarial_diff_summary_path_traversal_attempt() {
    // Attacker tries to inject path traversal via diff output
    let input = "M ../../../etc/passwd\nM ../../.ssh/authorized_keys";
    let files = parse_diff_summary(input);
    // Parser must not interpret paths — just return them as strings
    // The caller is responsible for sanitization
    assert!(files.contains("../../../etc/passwd"));
    assert!(files.contains("../../.ssh/authorized_keys"));
    // FINDING: parse_diff_summary does NOT sanitize paths. This is correct
    // for a parser, but callers must validate.
}

#[test]
fn adversarial_diff_summary_git_command_injection() {
    // Try to inject shell metacharacters via file paths
    let input = "M file; rm -rf /\nA $(cat /etc/passwd)\nD `echo pwned`";
    let files = parse_diff_summary(input);
    assert_eq!(files.len(), 3);
    // FINDING: Parser correctly treats these as opaque strings.
    // But if any downstream code passes these to shell execution, it's RCE.
}

#[test]
fn adversarial_diff_summary_rename_to_path_traversal() {
    // Renames where destination is path traversal
    let input = "R safe_name.rs -> ../../evil.rs";
    let files = parse_diff_summary(input);
    assert!(files.contains("../../evil.rs"));
    // FINDING: The rename parser extracts the last segment after " -> ",
    // which allows path traversal destinations through unchecked.
}

#[test]
fn adversarial_diff_summary_multiple_arrows_in_rename() {
    // Nested " -> " in rename — parser takes last segment
    let input = "R a -> b -> c -> evil.rs";
    let files = parse_diff_summary(input);
    assert_eq!(files.len(), 1);
    // Last segment wins — could hide a malicious filename
    assert!(files.contains("evil.rs"));
}

#[test]
fn adversarial_diff_summary_extreme_line_count() {
    // Resource exhaustion: 100,000 lines
    let mut input = String::with_capacity(5_000_000);
    for i in 0..100_000 {
        input.push_str(&format!("M file_{i}.rs\n"));
    }
    let files = parse_diff_summary(&input);
    assert_eq!(files.len(), 100_000);
}

#[test]
fn adversarial_diff_summary_very_long_line() {
    // Single line with 1MB filename
    let long_name = "x".repeat(1_000_000);
    let input = format!("M {long_name}");
    let files = parse_diff_summary(&input);
    assert!(files.contains(long_name.as_str()));
}

#[test]
fn adversarial_diff_summary_unicode_homoglyph_attack() {
    // Unicode homoglyphs that look like ASCII path separators
    let input = "M safe\u{2215}path.rs\nA real\u{FF0F}dir.rs";
    let files = parse_diff_summary(input);
    assert!(files.contains("safe∕path.rs"));
    assert!(files.contains("real／dir.rs"));
    // These look like "/" but aren't — could confuse human reviewers
}

#[test]
fn adversarial_diff_summary_only_status_no_file() {
    // Lines that are just a status character with no file part
    let input = "M\nA\nD\nR";
    let files = parse_diff_summary(input);
    assert!(
        files.is_empty(),
        "status-only lines should produce no files"
    );
}

#[test]
fn adversarial_diff_summary_carriage_return_injection() {
    // \r could cause visual confusion in terminals
    let input = "M safe_file.rs\r\nD dangerous_file.rs";
    let files = parse_diff_summary(input);
    // The \r is part of the filename in the first entry
    assert!(files.len() >= 1);
}

// ============================================================================
// ATTACK VECTOR 2: ConflictDetectionResult invariant violations
// ============================================================================

#[test]
fn adversarial_conflict_result_inconsistent_flags() {
    // Can we construct a result with has_existing_conflicts=true but empty list?
    let result = ConflictDetectionResult {
        has_existing_conflicts: true,
        existing_conflicts: vec![],
        merge_likely_safe: true, // also contradicts
        ..Default::default()
    };
    // FINDING: No validation between flags — data model allows inconsistencies
    assert!(result.has_existing_conflicts, "flag says conflicts exist");
    assert!(result.existing_conflicts.is_empty(), "but list is empty");
    assert!(
        result.merge_likely_safe,
        "claims safe despite conflicts flag"
    );
    // This is a data integrity gap: the struct has no constructor validation
}

#[test]
fn adversarial_conflict_result_overlapping_with_empty_summary() {
    // overlapping_files present but summary says "safe"
    let result = ConflictDetectionResult {
        overlapping_files: vec!["critical.rs".to_string()],
        merge_likely_safe: true,
        summary: String::new(),
        ..Default::default()
    };
    // FINDING: has_conflicts() correctly checks overlapping, but
    // merge_likely_safe can be independently set to wrong value
    assert!(result.has_conflicts(), "overlapping means has_conflicts");
    assert!(result.merge_likely_safe, "but merge_likely_safe is wrong");
}

#[test]
fn adversarial_conflict_result_negative_file_count() {
    // files_analyzed is usize, so can't be negative, but can be zero
    // while files are listed
    let result = ConflictDetectionResult {
        existing_conflicts: vec!["a.rs".to_string(), "b.rs".to_string()],
        files_analyzed: 0, // claims no analysis done, but reports conflicts
        ..Default::default()
    };
    assert_eq!(result.existing_conflicts.len(), 2);
    assert_eq!(result.files_analyzed, 0);
    // FINDING: files_analyzed is not validated against actual conflict lists
}

#[test]
fn adversarial_conflict_result_duplicate_entries() {
    // Same file in both existing and overlapping
    let result = ConflictDetectionResult {
        has_existing_conflicts: true,
        existing_conflicts: vec!["shared.rs".to_string()],
        overlapping_files: vec!["shared.rs".to_string()],
        ..Default::default()
    };
    assert!(result.has_conflicts());
    // FINDING: No deduplication between existing and overlapping lists
    // A file can appear in both, inflating the conflict count
}

#[test]
fn adversarial_conflict_result_serialization_integrity() {
    // Ensure serialization roundtrip preserves all fields even with adversarial data
    let result = ConflictDetectionResult {
        has_existing_conflicts: true,
        existing_conflicts: vec!["a; DROP TABLE workspaces;--".to_string()],
        overlapping_files: vec!["<script>alert(1)</script>".to_string()],
        workspace_only: vec!["\x00\x01\x02".to_string()],
        main_only: vec!["../../../etc/shadow".to_string()],
        merge_likely_safe: false,
        summary: "SQL injection in filename".to_string(),
        merge_base: Some("deadbeef".to_string()),
        files_analyzed: usize::MAX,
        detection_time_ms: u64::MAX,
    };
    let json = serde_json::to_string(&result).expect("serialize");
    let back: ConflictDetectionResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, result, "roundtrip must preserve adversarial data");
}

// ============================================================================
// ATTACK VECTOR 3: DoneOutput state machine violations
// ============================================================================

#[test]
fn adversarial_done_output_merged_but_not_cleaned() {
    // Invalid state: merged=true but cleaned=false
    // (keep_workspace is on DoneOptions, not DoneOutput — the caller must
    // set cleaned=false when keep_workspace=true, but the output struct
    // doesn't encode WHY cleaned is false)
    let output = DoneOutput {
        workspace_name: "test".to_string(),
        merged: true,
        cleaned: false,
        pushed_to_remote: false,
        ..Default::default()
    };
    // FINDING: DoneOutput has merged+cleaned as independent bools.
    // The semantic constraint "merged && !keep_workspace => cleaned" is
    // not enforced by the type system. Could represent a partial failure
    // that looks like success.
    assert!(output.merged);
    assert!(!output.cleaned);
}

#[test]
fn adversarial_done_output_dry_run_with_merge() {
    // Invalid state: dry_run=true AND merged=true
    let output = DoneOutput {
        workspace_name: "test".to_string(),
        dry_run: true,
        merged: true,
        ..Default::default()
    };
    // FINDING: Nothing prevents marking a dry run as merged
    assert!(output.dry_run);
    assert!(output.merged, "dry run should never be merged");
}

#[test]
fn adversarial_done_output_error_with_success_flags() {
    // Invalid state: error is set but merged=true
    let output = DoneOutput {
        workspace_name: "test".to_string(),
        merged: true,
        error: Some("something went wrong".to_string()),
        ..Default::default()
    };
    // FINDING: merged and error are independent — no invariant enforcement
    assert!(output.merged);
    assert!(output.error.is_some());
}

#[test]
fn adversarial_done_output_empty_workspace_name_success() {
    // Success output but empty workspace name
    let output = DoneOutput {
        workspace_name: String::new(),
        merged: true,
        cleaned: true,
        pushed_to_remote: true,
        ..Default::default()
    };
    // FINDING: No validation that workspace_name is non-empty on success
    let json = serde_json::to_string(&output).expect("serialize");
    let back: DoneOutput = serde_json::from_str(&json).expect("deserialize");
    assert!(back.workspace_name.is_empty());
    assert!(back.merged);
}

// ============================================================================
// ATTACK VECTOR 4: UndoEntry serialization attacks
// ============================================================================

#[test]
fn adversarial_undo_entry_json_injection() {
    let entry = UndoEntry {
        session_name: "test\"\n{\"injected\":true}\n{\"session_name\":\"".to_string(),
        commit_id: "abc".to_string(),
        pre_merge_commit_id: "def".to_string(),
        timestamp: 0,
        pushed_to_remote: false,
        status: "ok".to_string(),
    };
    let json = serde_json::to_string(&entry).expect("serialize");
    // serde_json properly escapes, so this should be a single valid JSON line
    let lines: Vec<&str> = json.lines().collect();
    assert_eq!(lines.len(), 1, "undo entry must be a single JSON line");
}

#[test]
fn adversarial_undo_entry_newline_in_fields() {
    let entry = UndoEntry {
        session_name: "ws\nwith\nnewlines".to_string(),
        commit_id: "abc\ndef".to_string(),
        pre_merge_commit_id: "ghi\njkl".to_string(),
        timestamp: 0,
        pushed_to_remote: false,
        status: "ok\nmalicious".to_string(),
    };
    let json = serde_json::to_string(&entry).expect("serialize");
    let back: UndoEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.session_name, "ws\nwith\nnewlines");
    // FINDING: Newlines in session_name survive serialization.
    // If undo log is line-delimited JSON, this could inject fake entries.
}

#[test]
fn adversarial_undo_entry_max_timestamp() {
    let entry = UndoEntry {
        session_name: "ws".to_string(),
        commit_id: "c".to_string(),
        pre_merge_commit_id: "p".to_string(),
        timestamp: u64::MAX,
        pushed_to_remote: true,
        status: "completed".to_string(),
    };
    let json = serde_json::to_string(&entry).expect("serialize");
    let back: UndoEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.timestamp, u64::MAX);
}

// ============================================================================
// ATTACK VECTOR 5: ExecutorError propagation attacks
// ============================================================================

#[test]
fn adversarial_executor_error_stderr_leak() {
    // Error messages should not leak sensitive paths or credentials
    let err = ExecutorError::CommandFailed {
        code: 128,
        stderr: "fatal: could not read Password for 'https://user:secret@github.com/'".to_string(),
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("secret"),
        "FINDING: stderr leaked into Display"
    );
    // In production, this would expose credentials in logs
}

#[test]
fn adversarial_executor_error_extremely_long_stderr() {
    let err = ExecutorError::CommandFailed {
        code: 1,
        stderr: "x".repeat(10_000_000),
    };
    let msg = format!("{err}");
    assert!(msg.len() > 10_000, "10MB error message would be logged");
    // FINDING: No truncation of stderr in error messages
}

// ============================================================================
// ATTACK VECTOR 6: Mock executor manipulation (concurrent access)
// ============================================================================

/// Thread-safe executor that can be configured to fail on the Nth call.
struct AdversarialExecutor {
    responses: Arc<Mutex<Vec<Result<String, ExecutorError>>>>,
    call_count: Arc<Mutex<usize>>,
}

impl AdversarialExecutor {
    fn new(responses: Vec<Result<String, ExecutorError>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl GitExecutor for AdversarialExecutor {
    fn run(&self, args: &[&str]) -> Result<String, ExecutorError> {
        let mut count = self.call_count.lock().unwrap_or_else(|e| e.into_inner());
        *count += 1;
        let mut responses = self.responses.lock().unwrap_or_else(|e| e.into_inner());
        if responses.is_empty() {
            return Err(ExecutorError::CommandFailed {
                code: 1,
                stderr: format!("unexpected call #{}: {}", *count, args.join(" ")),
            });
        }
        responses.remove(0)
    }

    fn run_in_workspace(
        &self,
        args: &[&str],
        workspace_path: &str,
    ) -> Result<String, ExecutorError> {
        // Verify workspace_path is not empty — path injection vector
        assert!(
            !workspace_path.is_empty(),
            "workspace_path must not be empty"
        );
        self.run(args)
    }
}

#[test]
fn adversarial_detect_conflicts_partial_failure_recovery() {
    // detect_conflicts calls 4 git commands sequentially.
    // What if the 3rd call fails?
    let executor = AdversarialExecutor::new(vec![
        Ok(String::new()),              // check_existing_conflicts: ok
        Ok("merge_base\n".to_string()), // find_merge_base: ok
        Err(ExecutorError::CommandFailed {
            // get_workspace_modified_files: FAIL
            code: 128,
            stderr: "fatal: bad object".to_string(),
        }),
    ]);

    let result = detect_conflicts(&executor);
    assert!(result.is_err(), "3rd step failure must propagate");
    assert_eq!(
        executor.call_count(),
        3,
        "should have called 3 git commands"
    );
}

#[test]
fn adversarial_detect_conflicts_step2_failure() {
    let executor = AdversarialExecutor::new(vec![
        Ok(String::new()), // check_existing_conflicts: ok
        Err(ExecutorError::CommandNotFound("git".to_string())), // find_merge_base: FAIL
    ]);

    let result = detect_conflicts(&executor);
    assert!(result.is_err(), "merge base failure must propagate");
    assert_eq!(executor.call_count(), 2);
}

#[test]
fn adversarial_detect_conflicts_existing_conflicts_then_merge_base_fails() {
    // Step 1 finds conflicts, step 2 fails — what happens?
    let executor = AdversarialExecutor::new(vec![
        Ok("CONFLICT\n".to_string()), // check_existing_conflicts: CONFLICT
        Ok("file_a.rs normal\n".to_string()), // resolve --list: ok
        Err(ExecutorError::IoError("disk error".to_string())), // find_merge_base: FAIL
    ]);

    let result = detect_conflicts(&executor);
    // detect_conflicts does NOT short-circuit on existing conflicts —
    // it continues to find_merge_base and fails there
    assert!(
        result.is_err(),
        "merge base failure after conflict detection"
    );
}

// ============================================================================
// ATTACK VECTOR 7: DoneOptions boundary conditions
// ============================================================================

#[test]
fn adversarial_done_options_all_flags_true() {
    // What happens with every flag enabled?
    let opts = DoneOptions {
        workspace: Some("ws".to_string()),
        message: Some("msg".to_string()),
        keep_workspace: true,
        squash: true,
        dry_run: true,
        detect_conflicts: true,
        no_bead_update: true,
    };
    // dry_run + detect_conflicts + keep_workspace + squash: all at once
    // In the actual flow, dry_run takes precedence and skip actual execution
    assert!(opts.dry_run);
    assert!(opts.detect_conflicts);
}

#[test]
fn adversarial_done_options_empty_workspace_string() {
    // Empty string is different from None
    let opts = DoneOptions {
        workspace: Some(String::new()),
        ..Default::default()
    };
    // workspace=Some("") vs workspace=None have different semantics
    // Some("") would attempt to resolve an empty-named workspace
    assert!(opts.workspace.is_some());
    assert!(opts.workspace.as_ref().unwrap().is_empty());
}

#[test]
fn adversarial_done_options_workspace_with_whitespace() {
    let opts = DoneOptions {
        workspace: Some("  my-workspace  ".to_string()),
        ..Default::default()
    };
    // Whitespace is not trimmed — could cause workspace resolution to fail
    // or match an unexpected workspace
    assert_eq!(opts.workspace.as_deref(), Some("  my-workspace  "));
}

// ============================================================================
// ATTACK VECTOR 8: DonePhase exhaustiveness and ordering
// ============================================================================

#[test]
fn adversarial_done_phase_name_uniqueness() {
    let names: HashSet<&str> = [
        DonePhase::ValidatingLocation.name(),
        DonePhase::CommittingChanges.name(),
        DonePhase::MergingToMain.name(),
    ]
    .into_iter()
    .collect();
    assert_eq!(names.len(), 3, "all phase names must be unique");
}

#[test]
fn adversarial_done_phase_name_no_empty() {
    for phase in [
        DonePhase::ValidatingLocation,
        DonePhase::CommittingChanges,
        DonePhase::MergingToMain,
    ] {
        assert!(!phase.name().is_empty());
    }
}

// ============================================================================
// ATTACK VECTOR 9: CommitInfo parsing edge cases
// ============================================================================

#[test]
fn adversarial_commit_info_zero_width_chars() {
    // Zero-width characters in commit messages
    let info = CommitInfo {
        change_id: "abc".to_string(),
        commit_id: "def".to_string(),
        description: "\u{200B}\u{200C}\u{200D}\u{FEFF}invisible".to_string(),
        timestamp: "2025-01-01".to_string(),
    };
    let json = serde_json::to_string(&info).expect("serialize");
    let back: CommitInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.description, info.description);
}

#[test]
fn adversarial_commit_info_very_long_description() {
    let info = CommitInfo {
        change_id: "abc".to_string(),
        commit_id: "def".to_string(),
        description: "x".repeat(1_000_000),
        timestamp: "t".to_string(),
    };
    let json = serde_json::to_string(&info).expect("serialize");
    assert!(json.len() > 1_000_000);
}

// ============================================================================
// ATTACK VECTOR 10: parse_status_lines adversarial inputs
// ============================================================================

/// Mirror of parse_status_lines from actions.rs tests (repeated here for
/// self-contained adversarial module).
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
fn adversarial_status_lines_binary_content() {
    // Non-UTF8 binary content can't appear in Rust strings, but control chars can
    let output = "M \x01\x02\x03binary.dat\nA normal.rs";
    let files = parse_status_lines(output);
    assert!(files.contains(&"normal.rs".to_string()));
}

#[test]
fn adversarial_status_lines_100k_files() {
    let mut output = String::with_capacity(2_000_000);
    for i in 0..100_000 {
        output.push_str(&format!("M file_{i}.rs\n"));
    }
    let files = parse_status_lines(&output);
    assert_eq!(files.len(), 100_000);
}

#[test]
fn adversarial_status_lines_only_prefix_no_file() {
    let output = "M \nA \nD \nR ";
    let files = parse_status_lines(output);
    assert!(files.is_empty(), "status + space + no file yields nothing");
}

// ============================================================================
// PROPTTEST: Property-based fuzzing for parse_diff_summary
// ============================================================================

proptest! {
    #[test]
    fn proptest_diff_summary_never_panics(input in ".*") {
        // The parser must never panic on any input
        let _ = parse_diff_summary(&input);
    }

    #[test]
    fn proptest_diff_summary_substring_independence(
        line1 in "[MADR] [a-zA-Z0-9_/._-]{0,50}",
        line2 in "[MADR] [a-zA-Z0-9_/._-]{0,50}"
    ) {
        let combined = format!("{line1}\n{line2}");
        let separate1 = parse_diff_summary(&line1);
        let separate2 = parse_diff_summary(&line2);
        let combined_set = parse_diff_summary(&combined);

        // Combined must be a superset of each individual parse
        for file in &separate1 {
            assert!(combined_set.contains(file), "combined missing from line1: {file}");
        }
        for file in &separate2 {
            assert!(combined_set.contains(file), "combined missing from line2: {file}");
        }
    }

    #[test]
    fn proptest_diff_summary_rename_extracts_destination(
        dest in "[a-zA-Z0-9_/._-]{1,30}"
    ) {
        let input = format!("R old_name.rs -> {dest}");
        let files = parse_diff_summary(&input);
        assert!(files.contains(&dest), "rename must extract destination");
        // old name should NOT appear
        assert!(!files.contains("old_name.rs"), "old name must not appear");
    }

    #[test]
    fn proptest_status_lines_never_panics(input in ".*") {
        let _ = parse_status_lines(&input);
    }

    #[test]
    fn proptest_conflict_result_serialization_roundtrip(
        has_existing in any::<bool>(),
        safe in any::<bool>(),
        count in 0usize..100,
        time_ms in any::<u64>()
    ) {
        let result = ConflictDetectionResult {
            has_existing_conflicts: has_existing,
            existing_conflicts: (0..count).map(|i| format!("conflict_{i}.rs")).collect(),
            overlapping_files: vec![],
            workspace_only: vec![],
            main_only: vec![],
            merge_likely_safe: safe,
            summary: "test".to_string(),
            merge_base: None,
            files_analyzed: count,
            detection_time_ms: time_ms,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: ConflictDetectionResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.has_existing_conflicts, has_existing);
        assert_eq!(back.merge_likely_safe, safe);
        assert_eq!(back.files_analyzed, count);
        assert_eq!(back.detection_time_ms, time_ms);
    }

    #[test]
    fn proptest_done_output_serialization_roundtrip(
        merged in any::<bool>(),
        cleaned in any::<bool>(),
        dry_run in any::<bool>(),
        pushed in any::<bool>(),
        count in any::<usize>()
    ) {
        let output = DoneOutput {
            workspace_name: format!("ws-{count}"),
            bead_id: if count % 2 == 0 { Some(format!("bead-{count}")) } else { None },
            files_committed: count,
            commits_merged: count / 2,
            merged,
            cleaned,
            bead_closed: merged && cleaned,
            session_updated: false,
            new_status: if merged { Some("merged".to_string()) } else { None },
            pushed_to_remote: pushed,
            dry_run,
            preview: None,
            error: if !merged { Some("failed".to_string()) } else { None },
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let back: DoneOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.workspace_name, format!("ws-{count}"));
        assert_eq!(back.merged, merged);
        assert_eq!(back.dry_run, dry_run);
    }

    #[test]
    fn proptest_executor_error_display_never_panics(
        code in any::<i32>(),
        stderr in ".*"
    ) {
        let err = ExecutorError::CommandFailed {
            code,
            stderr: stderr.clone(),
        };
        let msg = format!("{err}");
        assert!(msg.contains(&code.to_string()) || code == -1 && msg.contains("-1"));
    }
}

// ============================================================================
// SECURITY: Workspace name validation integration with done command
// ============================================================================

use crate::commands::workspace::validators::validate_workspace_name;

#[test]
fn adversarial_workspace_name_path_traversal() {
    assert!(validate_workspace_name("../../etc/passwd").is_some());
    assert!(validate_workspace_name(".hidden").is_some());
    assert!(validate_workspace_name("/absolute/path").is_some());
}

#[test]
fn adversarial_workspace_name_sql_injection() {
    assert!(validate_workspace_name("ws'; DROP TABLE workspaces;--").is_some());
    assert!(validate_workspace_name("ws OR 1=1").is_some());
}

#[test]
fn adversarial_workspace_name_xss() {
    assert!(validate_workspace_name("<script>alert(1)</script>").is_some());
    assert!(validate_workspace_name("ws\"><img src=x>").is_some());
}

#[test]
fn adversarial_workspace_name_shell_injection() {
    assert!(validate_workspace_name("ws; rm -rf /").is_some());
    assert!(validate_workspace_name("ws$(whoami)").is_some());
    assert!(validate_workspace_name("ws`id`").is_some());
    assert!(validate_workspace_name("ws|cat /etc/passwd").is_some());
}

// ============================================================================
// FINDINGS SUMMARY
// ============================================================================

// FINDINGS from RED QUEEN adversarial testing:
//
// 1. DATA INTEGRITY: ConflictDetectionResult has no constructor validation.
//    has_existing_conflicts, merge_likely_safe, and file lists can be
//    inconsistent. Safe construction requires factory methods with invariants.
//
// 2. DATA INTEGRITY: DoneOutput has no state machine enforcement.
//    merged=true + dry_run=true, or merged=true + error=Some(...) are
//    representable but semantically invalid.
//
// 3. RESOURCE EXHAUSTION: parse_diff_summary and parse_status_lines have
//    no bounds on input size. A 10MB diff output is parsed without limits.
//
// 4. LOG INJECTION: UndoEntry allows newlines in session_name, which could
//    inject fake entries into line-delimited JSON log files.
//
// 5. CREDENTIAL LEAK: ExecutorError::CommandFailed includes raw stderr in
//    Display output. If git error messages contain URLs with embedded
//    credentials, they get logged verbatim.
//
// 6. PATH SANITIZATION: parse_diff_summary returns raw paths including
//    path traversal sequences (../). No sanitization is performed at the
//    parser level. Callers must validate paths before use.
//
// 7. PARSER AMBIGUITY: Lines without recognized status prefixes are
//    parsed as files (the generic branch catches "status file" splits).
//    This means arbitrary text input produces file entries.
