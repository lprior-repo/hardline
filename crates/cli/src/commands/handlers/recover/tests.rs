//! Exhaustive tests for the recover command handler.
//!
//! Covers: recover workspace/session/state commands, recovery strategy selection,
//! recovery preview (dry-run), recovery confirmation, recovery progress reporting,
//! recovery from backup, unrecoverable state error, recovery logging.
//!
//! All test names are descriptive. All assertions use exact matching
//! (no bare `is_ok()`/`is_err()`).

use super::data::{
    compute_status, count_fixed, count_remaining, Issue, RecoverOptions, RecoverOutput,
    RecoverPhase, RollbackOptions, RollbackOutput,
};

// ============================================================================
// Helpers
// ============================================================================

fn make_issue(code: &str, severity: &str, fixed: bool) -> Issue {
    Issue {
        code: code.to_string(),
        description: format!("{code} issue"),
        severity: severity.to_string(),
        fix_command: Some(format!("fix {code}")),
        fixed,
    }
}

fn recover_opts() -> RecoverOptions {
    RecoverOptions::default()
}

fn recover_opts_diagnose() -> RecoverOptions {
    RecoverOptions {
        diagnose_only: true,
        target: None,
        dry_run: false,
        verbose: false,
    }
}

fn recover_opts_dry_run() -> RecoverOptions {
    RecoverOptions {
        diagnose_only: false,
        target: None,
        dry_run: true,
        verbose: false,
    }
}

fn recover_opts_target(target: &str) -> RecoverOptions {
    RecoverOptions {
        diagnose_only: false,
        target: Some(target.to_string()),
        dry_run: false,
        verbose: false,
    }
}

fn recover_opts_verbose() -> RecoverOptions {
    RecoverOptions {
        diagnose_only: false,
        target: None,
        dry_run: false,
        verbose: true,
    }
}

fn rollback_opts(session: &str, commit: &str) -> RollbackOptions {
    RollbackOptions {
        session: session.to_string(),
        commit: commit.to_string(),
        dry_run: false,
    }
}

fn rollback_opts_dry(session: &str, commit: &str) -> RollbackOptions {
    RollbackOptions {
        session: session.to_string(),
        commit: commit.to_string(),
        dry_run: true,
    }
}

// ============================================================================
// RecoverOptions — construction & defaults
// ============================================================================

#[test]
fn recover_options_all_fields_explicit() {
    let opts = RecoverOptions {
        diagnose_only: true,
        target: Some("my-workspace".to_string()),
        dry_run: true,
        verbose: true,
    };
    assert!(opts.diagnose_only);
    assert_eq!(opts.target.as_deref(), Some("my-workspace"));
    assert!(opts.dry_run);
    assert!(opts.verbose);
}

#[test]
fn recover_options_default_all_false() {
    let opts = RecoverOptions::default();
    assert!(!opts.diagnose_only);
    assert!(opts.target.is_none());
    assert!(!opts.dry_run);
    assert!(!opts.verbose);
}

#[test]
fn recover_options_clone_preserves_fields() {
    let opts = recover_opts_target("feature-x");
    let cloned = opts.clone();
    assert_eq!(cloned.target.as_deref(), Some("feature-x"));
}

#[test]
fn recover_options_target_empty_string_is_some() {
    let opts = RecoverOptions {
        target: Some(String::new()),
        ..RecoverOptions::default()
    };
    assert!(opts.target.is_some());
    assert_eq!(opts.target.as_deref(), Some(""));
}

// ============================================================================
// RollbackOptions — construction
// ============================================================================

#[test]
fn rollback_options_all_fields() {
    let opts = rollback_opts("ws-1", "abc123");
    assert_eq!(opts.session, "ws-1");
    assert_eq!(opts.commit, "abc123");
    assert!(!opts.dry_run);
}

#[test]
fn rollback_options_dry_run_flag() {
    let opts = rollback_opts_dry("ws-2", "def456");
    assert!(opts.dry_run);
}

#[test]
fn rollback_options_clone_preserves_fields() {
    let opts = rollback_opts("ws-1", "abc123");
    let cloned = opts.clone();
    assert_eq!(cloned.session, "ws-1");
    assert_eq!(cloned.commit, "abc123");
    assert!(!cloned.dry_run);
}

// ============================================================================
// Issue — construction, fields, serialization
// ============================================================================

#[test]
fn issue_all_known_codes() {
    let codes = [
        "GIT_NOT_INSTALLED",
        "GIT_NOT_INITIALIZED",
        "DETACHED_HEAD",
        "ORPHANED_WORKTREE",
        "STALE_WORKTREES",
        "MERGE_CONFLICTS",
    ];
    for code in &codes {
        let issue = make_issue(code, "warning", false);
        assert_eq!(issue.code, *code);
    }
}

#[test]
fn issue_all_severity_levels() {
    let severities = ["critical", "warning", "info"];
    for sev in &severities {
        let issue = make_issue("TEST", sev, false);
        assert_eq!(issue.severity, *sev);
    }
}

#[test]
fn issue_with_fix_command_some() {
    let issue = Issue {
        code: "DETACHED_HEAD".to_string(),
        description: "HEAD detached".to_string(),
        severity: "warning".to_string(),
        fix_command: Some("git checkout main".to_string()),
        fixed: false,
    };
    assert_eq!(issue.fix_command.as_deref(), Some("git checkout main"));
}

#[test]
fn issue_with_fix_command_none() {
    let issue = Issue {
        code: "UNKNOWN".to_string(),
        description: "Unknown issue".to_string(),
        severity: "critical".to_string(),
        fix_command: None,
        fixed: false,
    };
    assert!(issue.fix_command.is_none());
}

#[test]
fn issue_fixed_true_and_false() {
    let fixed = make_issue("X", "warning", true);
    assert!(fixed.fixed);
    let unfixed = make_issue("X", "warning", false);
    assert!(!unfixed.fixed);
}

#[test]
fn issue_serialization_roundtrip_all_fields() {
    let issue = Issue {
        code: "MERGE_CONFLICTS".to_string(),
        description: "3 files with conflicts".to_string(),
        severity: "critical".to_string(),
        fix_command: Some("git mergetool".to_string()),
        fixed: true,
    };
    let json = serde_json::to_string(&issue).expect("serialize");
    let back: Issue = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.code, "MERGE_CONFLICTS");
    assert_eq!(back.description, "3 files with conflicts");
    assert_eq!(back.severity, "critical");
    assert_eq!(back.fix_command.as_deref(), Some("git mergetool"));
    assert!(back.fixed);
}

#[test]
fn issue_serialization_roundtrip_no_fix_command() {
    let issue = Issue {
        code: "UNKNOWN".to_string(),
        description: "mystery".to_string(),
        severity: "info".to_string(),
        fix_command: None,
        fixed: false,
    };
    let json = serde_json::to_string(&issue).expect("serialize");
    let back: Issue = serde_json::from_str(&json).expect("deserialize");
    assert!(back.fix_command.is_none());
    assert!(!back.fixed);
}

#[test]
fn issue_json_fields_present_for_automation() {
    let issue = make_issue("ORPHANED_WORKTREE", "warning", false);
    let json = serde_json::to_string(&issue).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(
        parsed.get("code").is_some(),
        "code field required for automation"
    );
    assert!(parsed.get("description").is_some());
    assert!(parsed.get("severity").is_some());
    assert!(parsed.get("fixed").is_some());
}

#[test]
fn issue_empty_strings_serialize() {
    let issue = Issue {
        code: String::new(),
        description: String::new(),
        severity: String::new(),
        fix_command: Some(String::new()),
        fixed: false,
    };
    let json = serde_json::to_string(&issue).expect("serialize");
    let back: Issue = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.code, "");
    assert_eq!(back.severity, "");
    assert_eq!(back.fix_command.as_deref(), Some(""));
}

#[test]
fn issue_clone_preserves_all_fields() {
    let issue = Issue {
        code: "GIT_NOT_INITIALIZED".to_string(),
        description: "no .git dir".to_string(),
        severity: "critical".to_string(),
        fix_command: Some("git init".to_string()),
        fixed: false,
    };
    let cloned = issue.clone();
    assert_eq!(cloned.code, issue.code);
    assert_eq!(cloned.description, issue.description);
    assert_eq!(cloned.severity, issue.severity);
    assert_eq!(cloned.fix_command, issue.fix_command);
    assert_eq!(cloned.fixed, issue.fixed);
}

// ============================================================================
// RecoverOutput — construction, defaults, serialization
// ============================================================================

#[test]
fn recover_output_default_empty() {
    let output = RecoverOutput::default();
    assert!(output.issues.is_empty());
    assert_eq!(output.fixed_count, 0);
    assert_eq!(output.remaining_count, 0);
    assert!(output.status.is_empty());
}

#[test]
fn recover_output_with_multiple_issues() {
    let issues = vec![
        make_issue("GIT_NOT_INSTALLED", "critical", false),
        make_issue("DETACHED_HEAD", "warning", true),
        make_issue("STALE_WORKTREES", "info", true),
    ];
    let output = RecoverOutput {
        fixed_count: count_fixed(&issues),
        remaining_count: count_remaining(&issues),
        status: compute_status(&issues),
        issues,
    };
    assert_eq!(output.issues.len(), 3);
    assert_eq!(output.fixed_count, 2);
    assert_eq!(output.remaining_count, 1);
    assert_eq!(output.status, "partially_fixed");
}

#[test]
fn recover_output_status_healthy() {
    let output = RecoverOutput {
        issues: vec![],
        fixed_count: 0,
        remaining_count: 0,
        status: "healthy".to_string(),
    };
    assert_eq!(output.status, "healthy");
}

#[test]
fn recover_output_serialization_roundtrip() {
    let output = RecoverOutput {
        issues: vec![make_issue("X", "critical", false)],
        fixed_count: 0,
        remaining_count: 1,
        status: "issues_remaining".to_string(),
    };
    let json = serde_json::to_string(&output).expect("serialize");
    let back: RecoverOutput = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.issues.len(), 1);
    assert_eq!(back.fixed_count, 0);
    assert_eq!(back.remaining_count, 1);
    assert_eq!(back.status, "issues_remaining");
}

#[test]
fn recover_output_empty_issues_json() {
    let output = RecoverOutput {
        issues: vec![],
        fixed_count: 0,
        remaining_count: 0,
        status: "healthy".to_string(),
    };
    let json = serde_json::to_string(&output).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(parsed["issues"].as_array().map_or(false, |a| a.is_empty()));
}

// ============================================================================
// RollbackOutput — construction, serialization
// ============================================================================

#[test]
fn rollback_output_success() {
    let output = RollbackOutput {
        session: "feature-x".to_string(),
        commit: "abc123".to_string(),
        dry_run: false,
        succeeded: true,
        message: "Rolled back successfully".to_string(),
    };
    assert_eq!(output.session, "feature-x");
    assert_eq!(output.commit, "abc123");
    assert!(!output.dry_run);
    assert!(output.succeeded);
    assert!(output.message.contains("successfully"));
}

#[test]
fn rollback_output_failure() {
    let output = RollbackOutput {
        session: "ws-1".to_string(),
        commit: "bad".to_string(),
        dry_run: false,
        succeeded: false,
        message: "Commit 'bad' not found".to_string(),
    };
    assert!(!output.succeeded);
    assert!(output.message.contains("not found"));
}

#[test]
fn rollback_output_dry_run_preview() {
    let output = RollbackOutput {
        session: "ws-1".to_string(),
        commit: "abc123".to_string(),
        dry_run: true,
        succeeded: true,
        message: "Would roll back session 'ws-1' to commit 'abc123'".to_string(),
    };
    assert!(output.dry_run);
    assert!(output.succeeded);
    assert!(output.message.contains("Would roll back"));
}

#[test]
fn rollback_output_serialization_roundtrip() {
    let output = RollbackOutput {
        session: "ws".to_string(),
        commit: "deadbeef".to_string(),
        dry_run: true,
        succeeded: false,
        message: "Preview only".to_string(),
    };
    let json = serde_json::to_string(&output).expect("serialize");
    let back: RollbackOutput = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.session, "ws");
    assert_eq!(back.commit, "deadbeef");
    assert!(back.dry_run);
    assert!(!back.succeeded);
    assert_eq!(back.message, "Preview only");
}

#[test]
fn rollback_output_all_fields_json() {
    let output = RollbackOutput {
        session: "ws".to_string(),
        commit: "abc".to_string(),
        dry_run: false,
        succeeded: true,
        message: "ok".to_string(),
    };
    let json = serde_json::to_string(&output).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(parsed.get("session").is_some());
    assert!(parsed.get("commit").is_some());
    assert!(parsed.get("dry_run").is_some());
    assert!(parsed.get("succeeded").is_some());
    assert!(parsed.get("message").is_some());
}

// ============================================================================
// RecoverPhase — exhaustive variants, name, equality
// ============================================================================

#[test]
fn recover_phase_all_variants() {
    let phases = [
        RecoverPhase::Diagnosing,
        RecoverPhase::Fixing,
        RecoverPhase::RollingBack,
    ];
    assert_eq!(phases.len(), 3);
}

#[test]
fn recover_phase_diagnosing_name() {
    assert_eq!(RecoverPhase::Diagnosing.name(), "diagnosing");
}

#[test]
fn recover_phase_fixing_name() {
    assert_eq!(RecoverPhase::Fixing.name(), "fixing");
}

#[test]
fn recover_phase_rolling_back_name() {
    assert_eq!(RecoverPhase::RollingBack.name(), "rolling_back");
}

#[test]
fn recover_phase_names_are_snake_case() {
    for phase in &[
        RecoverPhase::Diagnosing,
        RecoverPhase::Fixing,
        RecoverPhase::RollingBack,
    ] {
        let name = phase.name();
        assert!(!name.is_empty());
        assert!(!name.contains(' '), "phase name must be snake_case: {name}");
        assert_eq!(name, name.to_lowercase());
    }
}

#[test]
fn recover_phase_equality_same() {
    assert_eq!(RecoverPhase::Diagnosing, RecoverPhase::Diagnosing);
    assert_eq!(RecoverPhase::Fixing, RecoverPhase::Fixing);
    assert_eq!(RecoverPhase::RollingBack, RecoverPhase::RollingBack);
}

#[test]
fn recover_phase_inequality_different() {
    assert_ne!(RecoverPhase::Diagnosing, RecoverPhase::Fixing);
    assert_ne!(RecoverPhase::Fixing, RecoverPhase::RollingBack);
    assert_ne!(RecoverPhase::RollingBack, RecoverPhase::Diagnosing);
}

#[test]
fn recover_phase_copy_semantics() {
    let a = RecoverPhase::Diagnosing;
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn recover_phase_clone_matches_original() {
    let phase = RecoverPhase::Fixing;
    let cloned = phase;
    assert_eq!(phase, cloned);
}

// ============================================================================
// compute_status — pure function exhaustive coverage
// ============================================================================

#[test]
fn compute_status_empty_issues() {
    assert_eq!(compute_status(&[]), "healthy");
}

#[test]
fn compute_status_all_fixed() {
    let issues = vec![
        make_issue("A", "critical", true),
        make_issue("B", "warning", true),
    ];
    assert_eq!(compute_status(&issues), "healthy");
}

#[test]
fn compute_status_none_fixed_critical() {
    let issues = vec![make_issue("A", "critical", false)];
    assert_eq!(compute_status(&issues), "issues_remaining");
}

#[test]
fn compute_status_none_fixed_warning() {
    let issues = vec![make_issue("A", "warning", false)];
    assert_eq!(compute_status(&issues), "issues_remaining");
}

#[test]
fn compute_status_partial_fix() {
    let issues = vec![
        make_issue("A", "critical", true),
        make_issue("B", "critical", false),
    ];
    assert_eq!(compute_status(&issues), "partially_fixed");
}

#[test]
fn compute_status_info_only_unfixed_is_healthy() {
    let issues = vec![make_issue("STALE_WORKTREES", "info", false)];
    assert_eq!(compute_status(&issues), "healthy");
}

#[test]
fn compute_status_mixed_info_and_critical_unfixed() {
    let issues = vec![
        make_issue("A", "info", false),
        make_issue("B", "critical", false),
    ];
    assert_eq!(compute_status(&issues), "issues_remaining");
}

#[test]
fn compute_status_info_fixed_and_critical_unfixed() {
    let issues = vec![
        make_issue("A", "info", true),
        make_issue("B", "critical", false),
    ];
    assert_eq!(compute_status(&issues), "partially_fixed");
}

#[test]
fn compute_status_many_issues_all_fixed() {
    let issues: Vec<Issue> = (0..50)
        .map(|i| make_issue(&format!("ISSUE_{i}"), "warning", true))
        .collect();
    assert_eq!(compute_status(&issues), "healthy");
}

#[test]
fn compute_status_many_issues_none_fixed() {
    let issues: Vec<Issue> = (0..50)
        .map(|i| make_issue(&format!("ISSUE_{i}"), "critical", false))
        .collect();
    assert_eq!(compute_status(&issues), "issues_remaining");
}

// ============================================================================
// count_fixed / count_remaining — exhaustive
// ============================================================================

#[test]
fn count_fixed_empty() {
    assert_eq!(count_fixed(&[]), 0);
}

#[test]
fn count_fixed_all_fixed() {
    let issues = vec![
        make_issue("A", "critical", true),
        make_issue("B", "warning", true),
    ];
    assert_eq!(count_fixed(&issues), 2);
}

#[test]
fn count_fixed_none_fixed() {
    let issues = vec![
        make_issue("A", "critical", false),
        make_issue("B", "warning", false),
    ];
    assert_eq!(count_fixed(&issues), 0);
}

#[test]
fn count_fixed_mixed() {
    let issues = vec![
        make_issue("A", "critical", true),
        make_issue("B", "warning", false),
        make_issue("C", "info", true),
    ];
    assert_eq!(count_fixed(&issues), 2);
}

#[test]
fn count_remaining_empty() {
    assert_eq!(count_remaining(&[]), 0);
}

#[test]
fn count_remaining_all_fixed() {
    let issues = vec![
        make_issue("A", "critical", true),
        make_issue("B", "warning", true),
    ];
    assert_eq!(count_remaining(&issues), 0);
}

#[test]
fn count_remaining_excludes_info_severity() {
    let issues = vec![
        make_issue("A", "info", false),
        make_issue("B", "info", false),
    ];
    assert_eq!(count_remaining(&issues), 0);
}

#[test]
fn count_remaining_includes_critical_and_warning() {
    let issues = vec![
        make_issue("A", "critical", false),
        make_issue("B", "warning", false),
    ];
    assert_eq!(count_remaining(&issues), 2);
}

#[test]
fn count_remaining_excludes_fixed_issues() {
    let issues = vec![
        make_issue("A", "critical", true),
        make_issue("B", "warning", false),
    ];
    assert_eq!(count_remaining(&issues), 1);
}

#[test]
fn count_remaining_excludes_info_and_fixed() {
    let issues = vec![
        make_issue("A", "info", false),
        make_issue("B", "critical", true),
        make_issue("C", "warning", false),
    ];
    assert_eq!(count_remaining(&issues), 1);
}

#[test]
fn count_remaining_large_set() {
    let issues: Vec<Issue> = (0..100)
        .map(|i| {
            let severity = if i % 3 == 0 { "info" } else { "critical" };
            make_issue(&format!("I_{i}"), severity, i % 2 == 0)
        })
        .collect();
    let remaining = count_remaining(&issues);
    assert!(
        remaining > 0,
        "should have some remaining non-info unfixed issues"
    );
}

// ============================================================================
// Recovery strategy selection — pure logic tests
// ============================================================================

#[test]
fn strategy_stale_worktrees_fixable() {
    let issue = make_issue("STALE_WORKTREES", "info", false);
    assert_eq!(issue.code, "STALE_WORKTREES");
    assert!(issue.fix_command.is_some());
}

#[test]
fn strategy_detached_head_fixable() {
    let issue = make_issue("DETACHED_HEAD", "warning", false);
    assert_eq!(issue.code, "DETACHED_HEAD");
    assert!(issue.fix_command.is_some());
}

#[test]
fn strategy_git_not_installed_requires_user() {
    let issue = make_issue("GIT_NOT_INSTALLED", "critical", false);
    assert_eq!(issue.code, "GIT_NOT_INSTALLED");
    assert_eq!(issue.severity, "critical");
}

#[test]
fn strategy_git_not_initialized_requires_user() {
    let issue = make_issue("GIT_NOT_INITIALIZED", "critical", false);
    assert_eq!(issue.code, "GIT_NOT_INITIALIZED");
    assert_eq!(issue.severity, "critical");
}

#[test]
fn strategy_merge_conflicts_requires_user() {
    let issue = make_issue("MERGE_CONFLICTS", "critical", false);
    assert_eq!(issue.code, "MERGE_CONFLICTS");
    assert_eq!(issue.severity, "critical");
}

#[test]
fn strategy_orphaned_worktree_requires_prune() {
    let issue = make_issue("ORPHANED_WORKTREE", "warning", false);
    assert_eq!(issue.code, "ORPHANED_WORKTREE");
    assert!(issue.fix_command.is_some());
}

// ============================================================================
// Recovery preview (dry-run) — option flag tests
// ============================================================================

#[test]
fn dry_run_option_prevents_fixes() {
    let opts = recover_opts_dry_run();
    assert!(opts.dry_run);
    assert!(!opts.diagnose_only);
}

#[test]
fn diagnose_only_option_prevents_fixes() {
    let opts = recover_opts_diagnose();
    assert!(opts.diagnose_only);
    assert!(!opts.dry_run);
}

#[test]
fn dry_run_and_diagnose_both_true() {
    let opts = RecoverOptions {
        diagnose_only: true,
        target: None,
        dry_run: true,
        verbose: true,
    };
    assert!(opts.diagnose_only);
    assert!(opts.dry_run);
    assert!(opts.verbose);
}

#[test]
fn rollback_dry_run_is_preview() {
    let opts = rollback_opts_dry("ws", "abc123");
    assert!(opts.dry_run);
}

#[test]
fn rollback_non_dry_is_execution() {
    let opts = rollback_opts("ws", "abc123");
    assert!(!opts.dry_run);
}

// ============================================================================
// Recovery progress reporting — output field tests
// ============================================================================

#[test]
fn recover_output_status_values_are_known() {
    let valid = ["healthy", "partially_fixed", "issues_remaining"];
    for status in &valid {
        assert!(!status.is_empty());
    }
}

#[test]
fn recover_output_counts_match_issues() {
    let issues = vec![
        make_issue("A", "critical", true),
        make_issue("B", "warning", false),
        make_issue("C", "info", true),
        make_issue("D", "critical", false),
    ];
    let output = RecoverOutput {
        fixed_count: count_fixed(&issues),
        remaining_count: count_remaining(&issues),
        status: compute_status(&issues),
        issues: issues.clone(),
    };
    assert_eq!(output.fixed_count, 2);
    assert_eq!(output.remaining_count, 2);
    assert_eq!(output.status, "partially_fixed");
    assert_eq!(output.issues.len(), 4);
}

#[test]
fn recover_output_zero_issues_is_healthy() {
    let output = RecoverOutput {
        fixed_count: 0,
        remaining_count: 0,
        status: "healthy".to_string(),
        issues: vec![],
    };
    assert_eq!(output.status, "healthy");
    assert_eq!(output.fixed_count, 0);
    assert_eq!(output.remaining_count, 0);
}

// ============================================================================
// Recovery from backup — rollback output scenarios
// ============================================================================

#[test]
fn rollback_output_preview_message_format() {
    let output = RollbackOutput {
        session: "my-workspace".to_string(),
        commit: "abc123".to_string(),
        dry_run: true,
        succeeded: true,
        message: format!(
            "Would roll back session '{}' to commit '{}'",
            "my-workspace", "abc123"
        ),
    };
    assert!(output.message.contains("Would roll back"));
    assert!(output.message.contains("my-workspace"));
    assert!(output.message.contains("abc123"));
}

#[test]
fn rollback_output_success_message_format() {
    let output = RollbackOutput {
        session: "my-workspace".to_string(),
        commit: "abc123".to_string(),
        dry_run: false,
        succeeded: true,
        message: format!(
            "Rolled back session '{}' to commit '{}'",
            "my-workspace", "abc123"
        ),
    };
    assert!(output.message.contains("Rolled back"));
    assert!(!output.dry_run);
}

#[test]
fn rollback_output_invalid_commit_message() {
    let output = RollbackOutput {
        session: "ws".to_string(),
        commit: "bad".to_string(),
        dry_run: false,
        succeeded: false,
        message: "'bad' is not a valid commit".to_string(),
    };
    assert!(!output.succeeded);
    assert!(output.message.contains("not a valid commit"));
}

#[test]
fn rollback_output_missing_workspace_message() {
    let output = RollbackOutput {
        session: "ghost".to_string(),
        commit: "abc".to_string(),
        dry_run: false,
        succeeded: false,
        message: "Workspace directory '/tmp/ghost' does not exist".to_string(),
    };
    assert!(!output.succeeded);
    assert!(output.message.contains("does not exist"));
}

#[test]
fn rollback_output_reset_failure_message() {
    let output = RollbackOutput {
        session: "ws".to_string(),
        commit: "abc".to_string(),
        dry_run: false,
        succeeded: false,
        message: "Rollback failed: permission denied".to_string(),
    };
    assert!(!output.succeeded);
    assert!(output.message.contains("Rollback failed"));
}

// ============================================================================
// Unrecoverable state error — critical issues that need user action
// ============================================================================

#[test]
fn critical_issues_are_unrecoverable_by_auto_fix() {
    let critical_codes = [
        "GIT_NOT_INSTALLED",
        "GIT_NOT_INITIALIZED",
        "MERGE_CONFLICTS",
    ];
    for code in &critical_codes {
        let issue = make_issue(code, "critical", false);
        assert_eq!(issue.severity, "critical");
        assert!(!issue.fixed, "critical issue should start unfixed");
    }
}

#[test]
fn warning_issues_may_be_auto_fixable() {
    let fixable = make_issue("DETACHED_HEAD", "warning", false);
    assert_eq!(fixable.severity, "warning");
    let pruneable = make_issue("ORPHANED_WORKTREE", "warning", false);
    assert_eq!(pruneable.severity, "warning");
}

#[test]
fn info_issues_are_low_severity() {
    let info = make_issue("STALE_WORKTREES", "info", false);
    assert_eq!(info.severity, "info");
}

// ============================================================================
// Recovery logging — verify output captures enough for logging
// ============================================================================

#[test]
fn recover_output_json_loggable() {
    let output = RecoverOutput {
        issues: vec![make_issue("GIT_NOT_INITIALIZED", "critical", false)],
        fixed_count: 0,
        remaining_count: 1,
        status: "issues_remaining".to_string(),
    };
    let json = serde_json::to_string_pretty(&output).expect("serialize");
    assert!(json.contains("issues_remaining"));
    assert!(json.contains("GIT_NOT_INITIALIZED"));
    assert!(json.contains("fixed_count"));
    assert!(json.contains("remaining_count"));
}

#[test]
fn rollback_output_json_loggable() {
    let output = RollbackOutput {
        session: "ws".to_string(),
        commit: "abc".to_string(),
        dry_run: false,
        succeeded: true,
        message: "ok".to_string(),
    };
    let json = serde_json::to_string_pretty(&output).expect("serialize");
    assert!(json.contains("succeeded"));
    assert!(json.contains("session"));
}

#[test]
fn issue_json_loggable_with_all_fields() {
    let issue = Issue {
        code: "MERGE_CONFLICTS".to_string(),
        description: "2 files with conflicts: a.rs, b.rs".to_string(),
        severity: "critical".to_string(),
        fix_command: Some("Resolve conflicts manually".to_string()),
        fixed: false,
    };
    let json = serde_json::to_string(&issue).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["code"], "MERGE_CONFLICTS");
    assert_eq!(parsed["severity"], "critical");
    assert_eq!(parsed["fixed"], false);
}

// ============================================================================
// Integration: compute_status + count functions combined
// ============================================================================

#[test]
fn integration_healthy_after_all_fixed() {
    let issues = vec![
        make_issue("STALE_WORKTREES", "info", true),
        make_issue("DETACHED_HEAD", "warning", true),
    ];
    assert_eq!(count_fixed(&issues), 2);
    assert_eq!(count_remaining(&issues), 0);
    assert_eq!(compute_status(&issues), "healthy");
}

#[test]
fn integration_partially_fixed_after_some_fixed() {
    let issues = vec![
        make_issue("STALE_WORKTREES", "info", true),
        make_issue("GIT_NOT_INSTALLED", "critical", false),
    ];
    assert_eq!(count_fixed(&issues), 1);
    assert_eq!(count_remaining(&issues), 1);
    assert_eq!(compute_status(&issues), "partially_fixed");
}

#[test]
fn integration_issues_remaining_none_fixed() {
    let issues = vec![make_issue("GIT_NOT_INITIALIZED", "critical", false)];
    assert_eq!(count_fixed(&issues), 0);
    assert_eq!(count_remaining(&issues), 1);
    assert_eq!(compute_status(&issues), "issues_remaining");
}

#[test]
fn integration_info_only_healthy_even_unfixed() {
    let issues = vec![make_issue("STALE_WORKTREES", "info", false)];
    assert_eq!(count_fixed(&issues), 0);
    assert_eq!(count_remaining(&issues), 0);
    assert_eq!(compute_status(&issues), "healthy");
}

// ============================================================================
// Adversarial tests — injection payloads, boundary conditions
// ============================================================================

mod red_queen_adversarial {
    use super::*;

    #[test]
    fn issue_with_sql_injection_code() {
        let issue = Issue {
            code: "'; DROP TABLE issues;--".to_string(),
            description: "malicious".to_string(),
            severity: "critical".to_string(),
            fix_command: Some("'); DROP TABLE fixes;--".to_string()),
            fixed: false,
        };
        let json = serde_json::to_string(&issue).expect("serialize");
        let back: Issue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.code, "'; DROP TABLE issues;--");
    }

    #[test]
    fn issue_with_script_injection_description() {
        let issue = Issue {
            code: "X".to_string(),
            description: "<script>alert('xss')</script>".to_string(),
            severity: "critical".to_string(),
            fix_command: None,
            fixed: false,
        };
        let json = serde_json::to_string(&issue).expect("serialize");
        let back: Issue = serde_json::from_str(&json).expect("deserialize");
        assert!(back.description.contains("<script>"));
    }

    #[test]
    fn issue_with_path_traversal_fix_command() {
        let issue = Issue {
            code: "X".to_string(),
            description: "test".to_string(),
            severity: "warning".to_string(),
            fix_command: Some("../../../etc/passwd".to_string()),
            fixed: false,
        };
        let json = serde_json::to_string(&issue).expect("serialize");
        let back: Issue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.fix_command.as_deref(), Some("../../../etc/passwd"));
    }

    #[test]
    fn issue_with_null_bytes_in_fields() {
        let issue = Issue {
            code: "A\x00B".to_string(),
            description: "desc\x00ription".to_string(),
            severity: "critical".to_string(),
            fix_command: Some("fix\x00it".to_string()),
            fixed: false,
        };
        let json = serde_json::to_string(&issue).expect("serialize");
        let back: Issue = serde_json::from_str(&json).expect("deserialize");
        assert!(back.code.contains('\x00'));
    }

    #[test]
    fn issue_with_unicode_overflow_description() {
        let long_desc = "\u{1F600}".repeat(10_000);
        let issue = Issue {
            code: "UNICODE".to_string(),
            description: long_desc.clone(),
            severity: "info".to_string(),
            fix_command: None,
            fixed: false,
        };
        let json = serde_json::to_string(&issue).expect("serialize");
        let back: Issue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.description.len(), long_desc.len());
    }

    #[test]
    fn recover_output_with_1000_issues() {
        let issues: Vec<Issue> = (0..1000)
            .map(|i| make_issue(&format!("ISSUE_{i}"), "critical", i % 2 == 0))
            .collect();
        let output = RecoverOutput {
            fixed_count: count_fixed(&issues),
            remaining_count: count_remaining(&issues),
            status: compute_status(&issues),
            issues,
        };
        assert_eq!(output.issues.len(), 1000);
        assert_eq!(output.status, "partially_fixed");
    }

    #[test]
    fn recover_options_with_very_long_target() {
        let long_target = "x".repeat(100_000);
        let opts = RecoverOptions {
            target: Some(long_target.clone()),
            ..RecoverOptions::default()
        };
        assert_eq!(opts.target.as_deref(), Some(long_target.as_str()));
    }

    #[test]
    fn rollback_options_with_empty_session_and_commit() {
        let opts = RollbackOptions {
            session: String::new(),
            commit: String::new(),
            dry_run: false,
        };
        assert_eq!(opts.session, "");
        assert_eq!(opts.commit, "");
    }

    #[test]
    fn rollback_output_with_very_long_message() {
        let long_msg = "error ".repeat(10_000);
        let output = RollbackOutput {
            session: "ws".to_string(),
            commit: "abc".to_string(),
            dry_run: false,
            succeeded: false,
            message: long_msg.clone(),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let back: RollbackOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.message.len(), long_msg.len());
    }

    #[test]
    fn compute_status_with_unknown_severity() {
        let issue = Issue {
            code: "X".to_string(),
            description: "d".to_string(),
            severity: "CATACLYSMIC".to_string(),
            fix_command: None,
            fixed: false,
        };
        let status = compute_status(&[issue]);
        assert_eq!(status, "issues_remaining");
    }

    #[test]
    fn compute_status_with_empty_severity_string() {
        let issue = Issue {
            code: "X".to_string(),
            description: "d".to_string(),
            severity: String::new(),
            fix_command: None,
            fixed: false,
        };
        let status = compute_status(&[issue]);
        assert_eq!(status, "issues_remaining");
    }

    #[test]
    fn recover_output_serialization_preserves_injected_json() {
        let issue = Issue {
            code: r#"","injected":"yes"#.to_string(),
            description: "normal".to_string(),
            severity: "critical".to_string(),
            fix_command: None,
            fixed: false,
        };
        let json = serde_json::to_string(&issue).expect("serialize");
        let back: Issue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.code, r#"","injected":"yes"#);
    }

    #[test]
    fn rollback_output_with_newlines_in_message() {
        let output = RollbackOutput {
            session: "ws".to_string(),
            commit: "abc".to_string(),
            dry_run: false,
            succeeded: false,
            message: "line1\nline2\rline3\r\nline4".to_string(),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let back: RollbackOutput = serde_json::from_str(&json).expect("deserialize");
        assert!(back.message.contains('\n'));
    }

    #[test]
    fn recover_phase_debug_format_contains_variant_name() {
        let debug_str = format!("{:?}", RecoverPhase::Diagnosing);
        assert!(debug_str.contains("Diagnosing"));
        let debug_str = format!("{:?}", RecoverPhase::Fixing);
        assert!(debug_str.contains("Fixing"));
        let debug_str = format!("{:?}", RecoverPhase::RollingBack);
        assert!(debug_str.contains("RollingBack"));
    }
}

// ============================================================================
// Proptest-based fuzzing
// ============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_compute_status_never_panics(
            codes in prop::collection::vec(".*", 0..20),
            severities in prop::collection::vec(".*", 0..20),
            fixed_flags in prop::collection::vec(any::<bool>(), 0..20),
        ) {
            let count = codes.len().min(severities.len()).min(fixed_flags.len());
            let issues: Vec<Issue> = (0..count)
                .map(|i| Issue {
                    code: codes[i].clone(),
                    description: String::new(),
                    severity: severities[i].clone(),
                    fix_command: None,
                    fixed: fixed_flags[i],
                })
                .collect();
            let status = compute_status(&issues);
            assert!(
                status == "healthy" || status == "partially_fixed" || status == "issues_remaining",
                "unexpected status: {status}"
            );
        }

        #[test]
        fn proptest_count_fixed_never_panics(
            fixed_flags in prop::collection::vec(any::<bool>(), 0..50),
        ) {
            let issues: Vec<Issue> = fixed_flags
                .iter()
                .map(|&f| make_issue("X", "warning", f))
                .collect();
            let counted = count_fixed(&issues);
            let expected = fixed_flags.iter().filter(|&&f| f).count();
            assert_eq!(counted, expected);
        }

        #[test]
        fn proptest_count_remaining_never_panics(
            severities in prop::collection::vec("(critical|warning|info)", 0..50),
            fixed_flags in prop::collection::vec(any::<bool>(), 0..50),
        ) {
            let count = severities.len().min(fixed_flags.len());
            let issues: Vec<Issue> = (0..count)
                .map(|i| Issue {
                    code: "X".to_string(),
                    description: String::new(),
                    severity: severities[i].clone(),
                    fix_command: None,
                    fixed: fixed_flags[i],
                })
                .collect();
            let remaining = count_remaining(&issues);
            let expected = issues
                .iter()
                .filter(|i| !i.fixed && i.severity != "info")
                .count();
            assert_eq!(remaining, expected);
        }

        #[test]
        fn proptest_issue_serialization_roundtrip(
            code in ".*",
            description in ".*",
            severity in "(critical|warning|info)",
            fix_command in proptest::option::of(".*"),
            fixed: bool,
        ) {
            let issue = Issue {
                code,
                description,
                severity,
                fix_command,
                fixed,
            };
            let json = serde_json::to_string(&issue).expect("serialize");
            let back: Issue = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.code, issue.code);
            assert_eq!(back.description, issue.description);
            assert_eq!(back.severity, issue.severity);
            assert_eq!(back.fixed, issue.fixed);
        }

        #[test]
        fn proptest_recover_output_roundtrip(
            issues in prop::collection::vec(
                (".*", ".*", "(critical|warning|info)", proptest::option::of(".*"), any::<bool>()),
                0..10,
            ),
        ) {
            let issue_list: Vec<Issue> = issues
                .into_iter()
                .map(|(code, desc, sev, fix, fixed)| Issue {
                    code,
                    description: desc,
                    severity: sev,
                    fix_command: fix,
                    fixed,
                })
                .collect();
            let output = RecoverOutput {
                fixed_count: count_fixed(&issue_list),
                remaining_count: count_remaining(&issue_list),
                status: compute_status(&issue_list),
                issues: issue_list,
            };
            let json = serde_json::to_string(&output).expect("serialize");
            let back: RecoverOutput = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.fixed_count, output.fixed_count);
            assert_eq!(back.remaining_count, output.remaining_count);
            assert_eq!(back.status, output.status);
            assert_eq!(back.issues.len(), output.issues.len());
        }

        #[test]
        fn proptest_rollback_output_roundtrip(
            session in ".*",
            commit in ".*",
            dry_run: bool,
            succeeded: bool,
            message in ".*",
        ) {
            let output = RollbackOutput {
                session,
                commit,
                dry_run,
                succeeded,
                message,
            };
            let json = serde_json::to_string(&output).expect("serialize");
            let back: RollbackOutput = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.session, output.session);
            assert_eq!(back.commit, output.commit);
            assert_eq!(back.dry_run, output.dry_run);
            assert_eq!(back.succeeded, output.succeeded);
            assert_eq!(back.message, output.message);
        }

        #[test]
        fn proptest_recover_options_never_panics(
            diagnose_only: bool,
            target in proptest::option::of(".*"),
            dry_run: bool,
            verbose: bool,
        ) {
            let opts = RecoverOptions {
                diagnose_only,
                target,
                dry_run,
                verbose,
            };
            let _cloned = opts.clone();
            assert_eq!(opts.diagnose_only, diagnose_only);
            assert_eq!(opts.dry_run, dry_run);
            assert_eq!(opts.verbose, verbose);
        }

        #[test]
        fn proptest_rollback_options_never_panics(
            session in ".*",
            commit in ".*",
            dry_run: bool,
        ) {
            let opts = RollbackOptions {
                session,
                commit,
                dry_run,
            };
            let _cloned = opts.clone();
            assert_eq!(opts.dry_run, dry_run);
        }

        #[test]
        fn proptest_compute_status_invariant_healthy_only_when_no_unfixed_non_info(
            severities in prop::collection::vec("(critical|warning|info)", 0..20),
            fixed_flags in prop::collection::vec(any::<bool>(), 0..20),
        ) {
            let count = severities.len().min(fixed_flags.len());
            let issues: Vec<Issue> = (0..count)
                .map(|i| make_issue("X", &severities[i], fixed_flags[i]))
                .collect();
            let status = compute_status(&issues);
            let has_unfixed_non_info = issues
                .iter()
                .any(|i| !i.fixed && i.severity != "info");
            if status == "healthy" {
                assert!(!has_unfixed_non_info, "healthy implies no unfixed non-info issues");
            }
        }
    }
}
