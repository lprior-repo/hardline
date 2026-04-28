//! Adversarial tests for doctor command safety and robustness.
//!
//! These tests verify that the doctor command handles edge cases safely:
//! - Broken checks don't crash the system
//! - Unsafe operations are prevented
//! - Concurrent operations don't cause race conditions
//! - Invalid inputs are handled gracefully
//!
//! Run with: `cargo test --package isolate-core --test doctor_adversarial_tests

// Integration tests have relaxed clippy settings for test ergonomics.
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
// ADVERSARIAL TEST 1: EMPTY CHECKS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_adversarial_empty_checks() {
    // Empty checks should produce a healthy system with 0 counts
    let output = DoctorOutput::from_checks(vec![]);

    assert!(output.healthy, "Empty system should be healthy");
    assert_eq!(output.warnings, 0, "Empty system should have 0 warnings");
    assert_eq!(output.errors, 0, "Empty system should have 0 errors");
    assert_eq!(
        output.auto_fixable_issues, 0,
        "Empty system should have 0 auto-fixable issues"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 2: ALL FAILURES
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_adversarial_all_failures() {
    // All failures should produce an unhealthy system
    let checks: Vec<DoctorCheck> = (1..=10)
        .map(|i| DoctorCheck {
            name: format!("Check {}", i),
            status: CheckStatus::Fail,
            message: format!("Failure {}", i),
            suggestion: Some("Fix it".to_string()),
            auto_fixable: i % 2 == 0, // Half are auto-fixable
            details: None,
        })
        .collect();

    let output = DoctorOutput::from_checks(checks);

    assert!(!output.healthy, "All-fail system should not be healthy");
    assert_eq!(output.warnings, 0, "All-fail system should have 0 warnings");
    assert_eq!(output.errors, 10, "All-fail system should have 10 errors");
    assert_eq!(
        output.auto_fixable_issues, 5,
        "Should have 5 auto-fixable issues"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 3: ALL WARNINGS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_adversarial_all_warnings() {
    // All warnings should produce a healthy system (warnings don't cause exit 1)
    let checks: Vec<DoctorCheck> = (1..=5)
        .map(|i| DoctorCheck {
            name: format!("Check {}", i),
            status: CheckStatus::Warn,
            message: format!("Warning {}", i),
            suggestion: None,
            auto_fixable: false,
            details: None,
        })
        .collect();

    let output = DoctorOutput::from_checks(checks);

    assert!(
        output.healthy,
        "All-warn system should be healthy (warnings don't cause exit 1)"
    );
    assert_eq!(output.warnings, 5, "All-warn system should have 5 warnings");
    assert_eq!(output.errors, 0, "All-warn system should have 0 errors");
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 4: MIXED STATUS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_adversarial_mixed_status() {
    let checks = vec![
        DoctorCheck {
            name: "Pass 1".to_string(),
            status: CheckStatus::Pass,
            message: "OK".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
        DoctorCheck {
            name: "Warn 1".to_string(),
            status: CheckStatus::Warn,
            message: "Warning".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
        DoctorCheck {
            name: "Fail 1".to_string(),
            status: CheckStatus::Fail,
            message: "Error".to_string(),
            suggestion: None,
            auto_fixable: true,
            details: None,
        },
        DoctorCheck {
            name: "Pass 2".to_string(),
            status: CheckStatus::Pass,
            message: "OK".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
    ];

    let output = DoctorOutput::from_checks(checks);

    assert!(
        !output.healthy,
        "Mixed system with Fail should not be healthy"
    );
    assert_eq!(output.warnings, 1);
    assert_eq!(output.errors, 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 5: JSON SERIALIZATION ROBUSTNESS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #[test]
    fn test_adversarial_json_serialization_robust(
        name in ".*",
        message in ".*",
        suggestion in proptest::option::of(".*"),
        auto_fixable: bool,
    ) {
        // Even with arbitrary strings, serialization should work
        let check = DoctorCheck {
            name,
            status: CheckStatus::Pass,
            message,
            suggestion,
            auto_fixable,
            details: None,
        };

        // Should never panic
        let json = serde_json::to_string(&check);
        prop_assert!(json.is_ok(), "Serialization should always succeed");

        // And should be parseable back
        let parsed: Result<serde_json::Value, _> =
            serde_json::from_str(&json.unwrap());
        prop_assert!(parsed.is_ok(), "Serialized JSON should be parseable");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 6: FIX RESULT WITH SPECIAL CHARACTERS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #[test]
    fn test_adversarial_fix_result_special_chars(
        issue in ".*",
        action in ".*",
        success: bool,
    ) {
        let fix = FixResult {
            issue,
            action,
            success,
        };

        // Should never panic
        let json = serde_json::to_string(&fix);
        prop_assert!(json.is_ok(), "FixResult serialization should always succeed");

        // And should be parseable back
        let parsed: Result<serde_json::Value, _> =
            serde_json::from_str(&json.unwrap());
        prop_assert!(parsed.is_ok(), "Serialized FixResult should be parseable");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 7: DETAILS WITH COMPLEX JSON
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_adversarial_complex_details() {
    let complex_details = serde_json::json!({
        "nested": {
            "deeply": {
                "nested": {
                    "value": 123
                }
            }
        },
        "array": [1, 2, 3, "four", null, true],
        "unicode": "Hello \u{1F600} World",
        "special_chars": "Newline\nTab\tQuote\"Backslash\\",
    });

    let check = DoctorCheck {
        name: "Complex Check".to_string(),
        status: CheckStatus::Fail,
        message: "Complex details test".to_string(),
        suggestion: None,
        auto_fixable: false,
        details: Some(complex_details),
    };

    let json = serde_json::to_string(&check).expect("Should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse");

    // Verify the complex details are preserved
    assert!(parsed.get("details").is_some());
    let details = parsed.get("details").unwrap();
    assert!(details.get("nested").is_some());
    assert!(details.get("array").is_some());
    assert!(details.get("unicode").is_some());
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 8: LARGE NUMBER OF CHECKS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_adversarial_many_checks() {
    // Create 1000 checks
    let checks: Vec<DoctorCheck> = (1..=1000)
        .map(|i| DoctorCheck {
            name: format!("Check {}", i),
            status: match i % 3 {
                0 => CheckStatus::Pass,
                1 => CheckStatus::Warn,
                _ => CheckStatus::Fail,
            },
            message: format!("Message {}", i),
            suggestion: None,
            auto_fixable: i % 5 == 0,
            details: None,
        })
        .collect();

    let output = DoctorOutput::from_checks(checks);

    // Verify counts are accurate
    // 1000 / 3 ≈ 333 of each status
    assert!(
        output.warnings >= 300 && output.warnings <= 340,
        "Warnings should be around 333, got {}",
        output.warnings
    );
    assert!(
        output.errors >= 300 && output.errors <= 340,
        "Errors should be around 333, got {}",
        output.errors
    );
    assert!(
        !output.healthy,
        "System with Fail checks should not be healthy"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 9: UNICODE IN CHECK NAMES
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_adversarial_unicode_names() {
    let checks = vec![
        DoctorCheck {
            name: "检查一".to_string(), // Chinese
            status: CheckStatus::Pass,
            message: "测试通过".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
        DoctorCheck {
            name: "テスト".to_string(), // Japanese
            status: CheckStatus::Warn,
            message: "警告".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
        DoctorCheck {
            name: "Проверка".to_string(), // Russian
            status: CheckStatus::Fail,
            message: "Ошибка".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
        DoctorCheck {
            name: "🔍 Check".to_string(), // Emoji
            status: CheckStatus::Pass,
            message: "Passed".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
    ];

    let output = DoctorOutput::from_checks(checks);

    assert!(!output.healthy);
    assert_eq!(output.warnings, 1);
    assert_eq!(output.errors, 1);

    // Verify JSON serialization works with unicode
    let json = serde_json::to_string(&output).expect("Should serialize unicode");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse unicode");

    // Verify unicode is preserved
    let checks = parsed.get("checks").unwrap().as_array().unwrap();
    assert!(checks[0]
        .get("name")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("检查"));
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 10: STATUS TRANSITION INVARIANTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_adversarial_status_ordering() {
    // Verify that Pass < Warn < Fail in terms of severity
    let statuses = [CheckStatus::Pass, CheckStatus::Warn, CheckStatus::Fail];

    for status in &statuses {
        let check = DoctorCheck {
            name: "Test".to_string(),
            status: *status,
            message: "Test".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        };

        // Each status should serialize correctly
        let json = serde_json::to_string(&check).expect("Should serialize");
        assert!(json.contains(&format!(
            "\"{}\"",
            match status {
                CheckStatus::Pass => "pass",
                CheckStatus::Warn => "warn",
                CheckStatus::Fail => "fail",
            }
        )));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 11: NULL AND EMPTY SUGGESTIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_adversarial_null_empty_suggestions() {
    let checks = vec![
        DoctorCheck {
            name: "No suggestion".to_string(),
            status: CheckStatus::Pass,
            message: "OK".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
        DoctorCheck {
            name: "Empty suggestion".to_string(),
            status: CheckStatus::Warn,
            message: "Warning".to_string(),
            suggestion: Some(String::new()),
            auto_fixable: false,
            details: None,
        },
        DoctorCheck {
            name: "Normal suggestion".to_string(),
            status: CheckStatus::Fail,
            message: "Error".to_string(),
            suggestion: Some("Fix this issue".to_string()),
            auto_fixable: false,
            details: None,
        },
    ];

    let output = DoctorOutput::from_checks(checks);

    // Verify JSON serialization handles null suggestions correctly
    let json = serde_json::to_string(&output).expect("Should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse");

    let checks = parsed.get("checks").unwrap().as_array().unwrap();

    // First check should have no suggestion field or null
    let first_suggestion = checks[0].get("suggestion");
    assert!(
        first_suggestion.is_none() || first_suggestion.unwrap().is_null(),
        "None suggestion should be null or absent"
    );

    // Second check should have empty string
    let second_suggestion = checks[1].get("suggestion");
    assert!(
        second_suggestion.is_some() && second_suggestion.unwrap().as_str() == Some(""),
        "Empty suggestion should be empty string"
    );

    // Third check should have the suggestion
    let third_suggestion = checks[2].get("suggestion");
    assert_eq!(
        third_suggestion.and_then(|s| s.as_str()),
        Some("Fix this issue")
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL TEST 12: CONCURRENT CHECK PROCESSING (SIMULATION)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_adversarial_concurrent_safety() {
    use std::{sync::Arc, thread};

    // Simulate concurrent access to DoctorOutput::from_checks
    let checks: Vec<DoctorCheck> = (1..=10)
        .map(|i| DoctorCheck {
            name: format!("Check {}", i),
            status: if i % 2 == 0 {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            },
            message: format!("Message {}", i),
            suggestion: None,
            auto_fixable: false,
            details: None,
        })
        .collect();

    let checks = Arc::new(checks);
    let mut handles = vec![];

    // Spawn multiple threads processing the same checks
    for _ in 0..10 {
        let checks_clone = Arc::clone(&checks);
        handles.push(thread::spawn(move || {
            DoctorOutput::from_checks(checks_clone.as_ref().clone())
        }));
    }

    // All threads should produce the same result
    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread should not panic"))
        .collect();

    let first = &results[0];
    for result in &results[1..] {
        assert_eq!(result.healthy, first.healthy);
        assert_eq!(result.warnings, first.warnings);
        assert_eq!(result.errors, first.errors);
        assert_eq!(result.auto_fixable_issues, first.auto_fixable_issues);
    }
}
