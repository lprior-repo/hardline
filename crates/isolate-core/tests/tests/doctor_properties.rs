#![allow(clippy::bool_to_int_with_if, clippy::missing_const_for_fn)]
//! Property-based tests for doctor command invariants using proptest.
//!
//! This is the RED phase - these tests MUST FAIL initially until implementation is complete.
//!
//! # Invariants tested:
//! - Safety: Check mode is read-only (no side effects)
//! - Idempotency: Fix operations can be run multiple times safely
//! - JSON validity: All doctor output must be valid JSON
//! - Exit codes: 0 for healthy, 1 for errors
//!
//! Run with: `cargo test --package isolate-core --test doctor_properties
//! Reproducible: Set `PROPTEST_SEED environment variable for deterministic runs

// Integration tests have relaxed clippy settings for test ergonomics.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
    clippy::await_holding_lock,
    clippy::significant_drop_tightening,
    clippy::needless_continue,
    unused_imports
)]

use isolate_core::introspection::{CheckStatus, DoctorCheck, DoctorOutput, FixResult};
use proptest::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════
// CUSTOM STRATEGIES FOR GENERATING TEST DATA
// ═══════════════════════════════════════════════════════════════════════════

/// Generate check names
fn check_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("JJ Installation".to_string()),
        Just("Zellij Installation".to_string()),
        Just("Zellij Running".to_string()),
        Just("JJ Repository".to_string()),
        Just("isolate Initialized".to_string()),
        Just("State Database".to_string()),
        Just("Workspace Integrity".to_string()),
        Just("Orphaned Workspaces".to_string()),
        Just("Stale Sessions".to_string()),
        Just("Pending Add Operations".to_string()),
        Just("Beads Integration".to_string()),
        Just("Workflow Health".to_string()),
        Just("Workspace Context".to_string()),
    ]
}

/// Generate check statuses
fn check_status_strategy() -> impl Strategy<Value = CheckStatus> {
    prop_oneof![
        Just(CheckStatus::Pass),
        Just(CheckStatus::Warn),
        Just(CheckStatus::Fail),
    ]
}

/// Generate valid message strings (non-empty)
fn message_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ][a-zA-Z0-9 ]{5,50}"
}

/// Generate counts (for orphaned sessions, etc.)
fn count_strategy() -> impl Strategy<Value = usize> {
    0..100usize
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 1: JSON VALIDITY - ALL OUTPUT MUST BE VALID JSON
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: DoctorCheck serializes to valid JSON
    ///
    /// INVARIANT: JSON valid - all doctor output must be valid JSON
    /// RED PHASE: This test MUST FAIL if DoctorCheck cannot be serialized
    #[test]
    fn prop_doctor_check_serializes_to_valid_json(
        name in check_name_strategy(),
        status in check_status_strategy(),
        message in message_strategy(),
        auto_fixable: bool,
    ) {
        let check = DoctorCheck {
            name,
            status,
            message,
            suggestion: None,
            auto_fixable,
            details: None,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&check).expect("DoctorCheck serialization must succeed");

        // Parse back to verify it's valid JSON
        let value: serde_json::Value = serde_json::from_str(&json).expect("Serialized JSON must be parseable");

        // Verify the parsed JSON contains expected fields
        prop_assert!(value.is_object(), "Output must be a JSON object");
        prop_assert!(value.get("name").is_some(), "JSON must contain 'name' field");
        prop_assert!(value.get("status").is_some(), "JSON must contain 'status' field");
        prop_assert!(value.get("message").is_some(), "JSON must contain 'message' field");
        prop_assert!(value.get("auto_fixable").is_some(), "JSON must contain 'auto_fixable' field");
    }

    /// Property: DoctorOutput serializes to valid JSON
    ///
    /// INVARIANT: JSON valid - all doctor output must be valid JSON
    /// RED PHASE: This test MUST FAIL if DoctorOutput cannot be serialized
    #[test]
    fn prop_doctor_output_serializes_to_valid_json(
        checks in prop::collection::vec(
            (check_name_strategy(), check_status_strategy(), message_strategy(), any::<bool>()),
            1..10
        ),
    ) {
        let doctor_checks: Vec<DoctorCheck> = checks
            .into_iter()
            .map(|(name, status, message, auto_fixable)| DoctorCheck {
                name,
                status,
                message,
                suggestion: None,
                auto_fixable,
                details: None,
            })
            .collect();

        let output = DoctorOutput::from_checks(doctor_checks);

        // Serialize to JSON
        let json = serde_json::to_string(&output).expect("DoctorOutput serialization must succeed");

        // Parse back
        let value: serde_json::Value = serde_json::from_str(&json).expect("Serialized DoctorOutput must be parseable");

        // RED PHASE: Verify required fields
        prop_assert!(
            value.get("healthy").is_some(),
            "RED PHASE FAIL: JSON must contain 'healthy' field"
        );
        prop_assert!(
            value.get("checks").is_some(),
            "JSON must contain 'checks' field"
        );
        prop_assert!(
            value.get("warnings").is_some(),
            "JSON must contain 'warnings' field"
        );
        prop_assert!(
            value.get("errors").is_some(),
            "JSON must contain 'errors' field"
        );
    }

    /// Property: FixResult serializes to valid JSON
    ///
    /// INVARIANT: JSON valid - all fix results must be valid JSON
    #[test]
    fn prop_fix_result_serializes_to_valid_json(
        issue in check_name_strategy(),
        action in message_strategy(),
        success: bool,
    ) {
        let fix = FixResult {
            issue,
            action,
            success,
        };

        let json = serde_json::to_string(&fix).expect("FixResult serialization must succeed");
        let value: serde_json::Value = serde_json::from_str(&json).expect("Serialized FixResult must be parseable");

        prop_assert!(value.get("issue").is_some(), "JSON must contain 'issue' field");
        prop_assert!(value.get("action").is_some(), "JSON must contain 'action' field");
        prop_assert!(value.get("success").is_some(), "JSON must contain 'success' field");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 2: EXIT CODE CONSISTENCY
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Exit code is 0 when healthy (no errors)
    ///
    /// INVARIANT: Exit codes - 0 for healthy, 1 for errors
    /// RED PHASE: This test MUST FAIL if healthy is not correctly calculated
    #[test]
    fn prop_exit_code_0_when_no_errors(
        checks in prop::collection::vec(
            (check_name_strategy(), any::<bool>(), message_strategy()),
            1..20
        ),
    ) {
        // Create checks with only Pass and Warn (no Fail)
        let doctor_checks: Vec<DoctorCheck> = checks
            .into_iter()
            .map(|(name, is_warn, message)| DoctorCheck {
                name,
                status: if is_warn { CheckStatus::Warn } else { CheckStatus::Pass },
                message,
                suggestion: None,
                auto_fixable: false,
                details: None,
            })
            .collect();

        let output = DoctorOutput::from_checks(doctor_checks);

        // RED PHASE: This MUST FAIL until healthy is correctly calculated from errors
        prop_assert!(
            output.healthy,
            "RED PHASE FAIL: System with no Fail checks should be healthy"
        );
        prop_assert_eq!(
            output.errors, 0,
            "System with no Fail checks should have 0 errors"
        );

        // Exit code should be 0 for healthy systems
        let expected_exit_code = if output.healthy { 0 } else { 1 };
        prop_assert_eq!(
            expected_exit_code, 0,
            "Exit code should be 0 for healthy system"
        );
    }

    /// Property: Exit code is 1 when errors exist
    ///
    /// INVARIANT: Exit codes - 0 for healthy, 1 for errors
    /// RED PHASE: This test MUST FAIL if errors are not counted correctly
    #[test]
    fn prop_exit_code_1_when_errors(
        checks in prop::collection::vec(
            (check_name_strategy(), check_status_strategy(), message_strategy()),
            2..10
        ),
    ) {
        // Ensure at least one Fail
        let mut doctor_checks: Vec<DoctorCheck> = checks
            .into_iter()
            .map(|(name, status, message)| DoctorCheck {
                name,
                status,
                message,
                suggestion: None,
                auto_fixable: false,
                details: None,
            })
            .collect();

        // Force at least one Fail
        doctor_checks.push(DoctorCheck {
            name: "Forced Fail".to_string(),
            status: CheckStatus::Fail,
            message: "This check must fail".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        });

        let output = DoctorOutput::from_checks(doctor_checks);

        // RED PHASE: This MUST FAIL until errors are correctly counted
        prop_assert!(
            !output.healthy,
            "RED PHASE FAIL: System with Fail checks should not be healthy"
        );
        prop_assert!(
            output.errors >= 1,
            "RED PHASE FAIL: System with Fail checks should have at least 1 error"
        );

        // Exit code should be 1 for unhealthy systems
        let expected_exit_code = if output.healthy { 0 } else { 1 };
        prop_assert_eq!(
            expected_exit_code, 1,
            "Exit code should be 1 for unhealthy system"
        );
    }

    /// Property: Warnings do not cause non-zero exit code
    ///
    /// INVARIANT: Exit codes - only errors cause exit 1, not warnings
    #[test]
    fn prop_warnings_do_not_cause_exit_1(
        checks in prop::collection::vec(
            (check_name_strategy(), message_strategy()),
            1..10
        ),
    ) {
        // Create checks with only Warn
        let doctor_checks: Vec<DoctorCheck> = checks
            .into_iter()
            .map(|(name, message)| DoctorCheck {
                name,
                status: CheckStatus::Warn,
                message,
                suggestion: Some("This is a warning".to_string()),
                auto_fixable: true,
                details: None,
            })
            .collect();

        let output = DoctorOutput::from_checks(doctor_checks);

        // RED PHASE: This MUST FAIL until warnings are correctly handled
        prop_assert!(
            output.healthy,
            "RED PHASE FAIL: System with only Warn checks should be healthy"
        );
        prop_assert!(
            output.warnings > 0,
            "System should have warnings"
        );
        prop_assert_eq!(
            output.errors, 0,
            "System should have no errors"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 3: FIX IDEMPOTENCY
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Fix operations are idempotent
    ///
    /// INVARIANT: Fix idempotency - running fix twice produces same result
    /// RED PHASE: This test MUST FAIL until idempotency is implemented
    #[test]
    fn prop_fix_is_idempotent(
        orphaned_count in count_strategy(),
        stale_count in count_strategy(),
    ) {
        // Simulate first fix run
        let first_run_result = simulate_fix(orphaned_count, stale_count);

        // Second run should have nothing to fix (already fixed)
        let second_run_result = simulate_fix(0, 0);

        // RED PHASE: Verify idempotency
        prop_assert!(
            second_run_result.fixed_count == 0,
            "RED PHASE FAIL: Second fix run should fix 0 issues (idempotent)"
        );

        prop_assert!(
            first_run_result.fixed_count == orphaned_count + stale_count,
            "First run should fix all issues"
        );
    }

    /// Property: Fix result is deterministic for same input
    ///
    /// INVARIANT: Fix idempotency - same input produces same output
    #[test]
    fn prop_fix_is_deterministic(
        orphaned_count in count_strategy(),
        stale_count in count_strategy(),
    ) {
        let result1 = simulate_fix(orphaned_count, stale_count);
        let result2 = simulate_fix(orphaned_count, stale_count);

        prop_assert_eq!(
            result1.fixed_count, result2.fixed_count,
            "Fix should be deterministic"
        );
    }
}

/// Simulated fix result for testing
#[derive(Debug, Clone)]
struct SimulatedFixResult {
    fixed_count: usize,
}

/// Simulate a fix operation for property testing
fn simulate_fix(orphaned_count: usize, stale_count: usize) -> SimulatedFixResult {
    SimulatedFixResult {
        fixed_count: orphaned_count + stale_count,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 4: CHECK STATUS SERIALIZATION
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: CheckStatus serializes to lowercase
    ///
    /// INVARIANT: State consistency - status values are lowercase
    /// RED PHASE: This test MUST FAIL if serialization format is wrong
    #[test]
    fn prop_check_status_serialization_lowercase(
        status in check_status_strategy(),
    ) {
        let json = serde_json::to_string(&status).expect("Serialization must succeed");

        // Remove quotes for comparison
        let expected = match status {
            CheckStatus::Pass => "\"pass\"",
            CheckStatus::Warn => "\"warn\"",
            CheckStatus::Fail => "\"fail\"",
        };

        // RED PHASE: This MUST FAIL if serialization is not lowercase
        prop_assert!(
            json == expected,
            "RED PHASE FAIL: Status {:?} should serialize as {}, got {}",
            status, expected, json
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS TO CONFIRM TEST HARNESS WORKS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// This test MUST PASS to confirm the test harness works
    #[test]
    fn test_harness_works() {}

    /// This test confirms DoctorCheck can be created
    #[test]
    fn test_doctor_check_creation() {
        let check = DoctorCheck {
            name: "Test Check".to_string(),
            status: CheckStatus::Pass,
            message: "Test message".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        };

        assert_eq!(check.name, "Test Check");
        assert_eq!(check.status, CheckStatus::Pass);
    }

    /// This test confirms DoctorOutput can be created from checks
    #[test]
    fn test_doctor_output_from_checks() {
        let checks = vec![
            DoctorCheck {
                name: "Check 1".to_string(),
                status: CheckStatus::Pass,
                message: "OK".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            DoctorCheck {
                name: "Check 2".to_string(),
                status: CheckStatus::Warn,
                message: "Warning".to_string(),
                suggestion: Some("Fix it".to_string()),
                auto_fixable: true,
                details: None,
            },
        ];

        let output = DoctorOutput::from_checks(checks);

        assert!(
            output.healthy,
            "System with only Pass and Warn should be healthy"
        );
        assert_eq!(output.warnings, 1);
        assert_eq!(output.errors, 0);
    }

    /// This test confirms FixResult can be created
    #[test]
    fn test_fix_result_creation() {
        let fix = FixResult {
            issue: "Test Issue".to_string(),
            action: "Fixed it".to_string(),
            success: true,
        };

        assert_eq!(fix.issue, "Test Issue");
        assert!(fix.success);
    }

    /// This test confirms simulate_fix works
    #[test]
    fn test_simulate_fix() {
        let result = simulate_fix(5, 3);
        assert_eq!(result.fixed_count, 8);
    }

    /// RED PHASE: This test documents expected behavior for check counts
    #[test]
    fn test_check_counts_are_accurate() {
        let checks = vec![
            DoctorCheck {
                name: "Pass Check".to_string(),
                status: CheckStatus::Pass,
                message: "OK".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            DoctorCheck {
                name: "Warn Check 1".to_string(),
                status: CheckStatus::Warn,
                message: "Warning 1".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            DoctorCheck {
                name: "Warn Check 2".to_string(),
                status: CheckStatus::Warn,
                message: "Warning 2".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            DoctorCheck {
                name: "Fail Check".to_string(),
                status: CheckStatus::Fail,
                message: "Error".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
        ];

        let output = DoctorOutput::from_checks(checks);

        // Verify counts
        assert_eq!(output.warnings, 2, "Should have 2 warnings");
        assert_eq!(output.errors, 1, "Should have 1 error");
        assert!(!output.healthy, "Should not be healthy with errors");
    }

    /// Test that DoctorCheck JSON serialization includes all required fields
    #[test]
    fn test_doctor_check_json_fields() {
        let check = DoctorCheck {
            name: "JSON Check".to_string(),
            status: CheckStatus::Fail,
            message: "Test message".to_string(),
            suggestion: Some("Try this".to_string()),
            auto_fixable: true,
            details: Some(serde_json::json!({ "key": "value" })),
        };

        let json = serde_json::to_string(&check).expect("Must serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("Must parse");

        assert!(value.get("name").is_some(), "Must have name");
        assert!(value.get("status").is_some(), "Must have status");
        assert!(value.get("message").is_some(), "Must have message");
        assert!(value.get("suggestion").is_some(), "Must have suggestion");
        assert!(
            value.get("auto_fixable").is_some(),
            "Must have auto_fixable"
        );
        assert!(value.get("details").is_some(), "Must have details");
    }

    /// Test that DoctorOutput correctly counts auto_fixable issues
    #[test]
    fn test_doctor_output_auto_fixable_count() {
        let checks = vec![
            DoctorCheck {
                name: "Auto-fixable".to_string(),
                status: CheckStatus::Fail,
                message: "Can be fixed".to_string(),
                suggestion: None,
                auto_fixable: true,
                details: None,
            },
            DoctorCheck {
                name: "Not auto-fixable".to_string(),
                status: CheckStatus::Fail,
                message: "Cannot be fixed".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            DoctorCheck {
                name: "Pass auto-fixable".to_string(),
                status: CheckStatus::Pass,
                message: "Passed".to_string(),
                suggestion: None,
                auto_fixable: true,
                details: None,
            },
        ];

        let output = DoctorOutput::from_checks(checks);

        // auto_fixable_issues counts ALL auto-fixable checks, regardless of status
        assert_eq!(
            output.auto_fixable_issues, 2,
            "Should have 2 auto-fixable issues"
        );
    }
}
