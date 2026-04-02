//! Moon gate execution for CI quality gates.
//!
//! This module implements the gate execution logic for running moon CI tasks
//! and parsing their output. It follows the functional core pattern with:
//! - Pure functions for output parsing
//! - No I/O in the parsing layer
//! - Railway-oriented error handling
//!
//! # Gate Execution Order
//! 1. `:quick` - Fast lint check (format + clippy)
//! 2. `:test` - Full test suite (only if quick passes)
//!
//! # State Transitions
//! - On quick pass + test pass -> `ready_to_merge`
//! - On quick fail -> `failed_retryable` (skip test - fail fast)
//! - On test fail -> `failed_retryable`

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt;

use thiserror::Error;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DOMAIN TYPES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Available moon task gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoonGate {
    /// Quick check (format + clippy)
    Quick,
    /// Full test suite
    Test,
}

impl MoonGate {
    /// Returns the moon task name for this gate.
    #[must_use]
    pub const fn as_task(&self) -> &'static str {
        match self {
            Self::Quick => ":quick",
            Self::Test => ":test",
        }
    }

    /// Returns a human-readable description of this gate.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Quick => "Quick check (format + clippy)",
            Self::Test => "Test suite",
        }
    }
}

impl fmt::Display for MoonGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_task())
    }
}

/// Result of running a single moon gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateResult {
    /// Which gate was run
    pub gate: MoonGate,
    /// Whether the gate passed
    pub passed: bool,
    /// Exit code from moon
    pub exit_code: i32,
    /// Raw stdout (for debugging)
    pub stdout: String,
    /// Raw stderr (for debugging)
    pub stderr: String,
    /// Parsed summary message
    pub summary: String,
}

impl GateResult {
    /// Create a new gate result.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        gate: MoonGate,
        passed: bool,
        exit_code: i32,
        stdout: String,
        stderr: String,
        summary: String,
    ) -> Self {
        Self {
            gate,
            passed,
            exit_code,
            stdout,
            stderr,
            summary,
        }
    }

    /// Create a passing result.
    #[must_use]
    pub fn passed(gate: MoonGate, stdout: String, stderr: String) -> Self {
        let summary = parse_summary(&stdout, &stderr);
        Self::new(gate, true, 0, stdout, stderr, summary)
    }

    /// Create a failing result.
    #[must_use]
    pub fn failed(gate: MoonGate, exit_code: i32, stdout: String, stderr: String) -> Self {
        let summary = parse_summary(&stdout, &stderr);
        Self::new(gate, false, exit_code, stdout, stderr, summary)
    }
}

/// Combined result of running all required gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatesOutcome {
    /// Result of the quick gate
    pub quick: GateResult,
    /// Result of the test gate (None if quick failed - fail fast)
    pub test: Option<GateResult>,
    /// Overall outcome
    pub status: GatesStatus,
}

/// Overall status of gate execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatesStatus {
    /// All gates passed
    AllPassed,
    /// Quick gate failed (test skipped)
    QuickFailed,
    /// Test gate failed
    TestFailed,
}

impl GatesStatus {
    /// Returns true if all gates passed.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::AllPassed)
    }

    /// Returns true if any gate failed.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        !self.is_success()
    }
}

impl fmt::Display for GatesStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllPassed => write!(f, "all gates passed"),
            Self::QuickFailed => write!(f, "quick gate failed"),
            Self::TestFailed => write!(f, "test gate failed"),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ERROR TYPES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Errors that can occur during gate execution.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GateError {
    /// Failed to execute the moon command
    #[error("failed to execute moon {gate}: {reason}")]
    ExecutionFailed { gate: MoonGate, reason: String },

    /// Moon binary not found
    #[error("moon binary not found in PATH")]
    MoonNotFound,

    /// Working directory does not exist
    #[error("working directory does not exist: {0}")]
    WorkingDirectoryNotFound(String),

    /// Output parsing failed
    #[error("failed to parse moon output: {0}")]
    ParseError(String),
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PURE PARSING FUNCTIONS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Parse moon output and determine if the gate passed.
///
/// Moon exits with:
/// - 0: All tasks succeeded
/// - 1: One or more tasks failed
///
/// # Arguments
/// * `exit_code` - The exit code from the moon process
/// * `stdout` - Standard output from moon
/// * `stderr` - Standard error from moon
///
/// # Returns
/// `true` if the gate passed, `false` otherwise
#[must_use]
pub const fn classify_exit_code(exit_code: i32) -> bool {
    exit_code == 0
}

/// Parse a summary message from moon output.
///
/// Extracts relevant information from stdout/stderr to create a
/// human-readable summary.
///
/// # Arguments
/// * `stdout` - Standard output from moon
/// * `stderr` - Standard error from moon
///
/// # Returns
/// A summary string suitable for display
#[must_use]
pub fn parse_summary(stdout: &str, stderr: &str) -> String {
    // Moon typically outputs task results to stdout
    // Look for common patterns in the output

    let stdout_lines: Vec<&str> = stdout.lines().collect();
    let stderr_lines: Vec<&str> = stderr.lines().collect();

    // Check for explicit pass/fail indicators
    let has_passed = stdout_lines.iter().any(|line| {
        line.contains("passed")
            || line.contains("succeeded")
            || line.contains("completed")
            || line.contains("PASS")
    });

    let has_failed = stdout_lines.iter().chain(stderr_lines.iter()).any(|line| {
        line.contains("failed")
            || line.contains("error")
            || line.contains("FAIL")
            || line.contains("Error:")
    });

    match (has_passed, has_failed) {
        (true, false) => "Gate passed".to_string(),
        (false, true) => extract_failure_summary(&stdout_lines, &stderr_lines),
        (true, true) => "Gate completed with errors".to_string(),
        (false, false) => {
            // No clear indicators - return first non-empty line or default
            match stdout_lines.first() {
                Some(line) if !line.trim().is_empty() => (*line).to_string(),
                _ => "Gate completed".to_string(),
            }
        }
    }
}

/// Extract a failure summary from output lines.
fn extract_failure_summary(stdout_lines: &[&str], stderr_lines: &[&str]) -> String {
    // Look for error patterns
    let error_line = stdout_lines.iter().chain(stderr_lines.iter()).find(|line| {
        line.to_lowercase().contains("error")
            || line.to_lowercase().contains("failed")
            || line.contains("FAIL")
    });

    error_line.map_or_else(
        || "Gate failed".to_string(),
        |line| {
            // Truncate long lines
            if line.len() > 100 {
                format!("{}...", &line[..97])
            } else {
                (*line).to_string()
            }
        },
    )
}

/// Determine the overall gates status from individual results.
///
/// This implements the fail-fast logic:
/// - If quick failed, test is None and status is `QuickFailed`
/// - If quick passed but test failed, status is `TestFailed`
/// - If both passed, status is `AllPassed`
///
/// # Arguments
/// * `quick_result` - Result of the quick gate
/// * `test_result` - Result of the test gate (None if quick failed)
///
/// # Returns
/// The combined gates outcome
#[must_use]
pub const fn combine_results(
    quick_result: GateResult,
    test_result: Option<GateResult>,
) -> GatesOutcome {
    let status = match (&quick_result, &test_result) {
        (quick, None) if !quick.passed => GatesStatus::QuickFailed,
        (_quick, None) => {
            // Quick passed but test was not run (shouldn't happen in normal flow)
            GatesStatus::AllPassed
        }
        (quick, Some(_test)) if !quick.passed => {
            // Quick failed but test was run (shouldn't happen with fail-fast)
            GatesStatus::QuickFailed
        }
        (_quick, Some(test)) if !test.passed => GatesStatus::TestFailed,
        _ => GatesStatus::AllPassed,
    };

    GatesOutcome {
        quick: quick_result,
        test: test_result,
        status,
    }
}

/// Create an error message for a failed gate.
#[must_use]
pub fn format_failure_message(outcome: &GatesOutcome) -> String {
    match outcome.status {
        GatesStatus::QuickFailed => {
            format!(
                "Quick gate failed (exit code {}): {}",
                outcome.quick.exit_code, outcome.quick.summary
            )
        }
        GatesStatus::TestFailed => {
            let test_result = outcome.test.as_ref();
            test_result.map_or_else(
                || "Test gate failed".to_string(),
                |test| {
                    format!(
                        "Test gate failed (exit code {}): {}",
                        test.exit_code, test.summary
                    )
                },
            )
        }
        GatesStatus::AllPassed => "All gates passed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // MOON GATE TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_moon_gate_as_task() {
        assert_eq!(MoonGate::Quick.as_task(), ":quick");
        assert_eq!(MoonGate::Test.as_task(), ":test");
    }

    #[test]
    fn test_moon_gate_description() {
        assert!(MoonGate::Quick.description().contains("Quick"));
        assert!(MoonGate::Test.description().contains("Test"));
    }

    #[test]
    fn test_classify_exit_code() {
        assert!(classify_exit_code(0));
        assert!(!classify_exit_code(1));
        assert!(!classify_exit_code(2));
        assert!(!classify_exit_code(-1));
    }

    #[test]
    fn test_parse_summary_passed() {
        let stdout = "Running :quick...\nAll tasks passed!\nDone in 2.3s";
        let stderr = "";
        let summary = parse_summary(stdout, stderr);
        assert_eq!(summary, "Gate passed");
    }

    #[test]
    fn test_parse_summary_failed() {
        let stdout = "Running :quick...\nerror: formatting check failed\nDone";
        let stderr = "";
        let summary = parse_summary(stdout, stderr);
        assert!(summary.contains("error"));
    }

    #[test]
    fn test_parse_summary_empty() {
        let summary = parse_summary("", "");
        assert_eq!(summary, "Gate completed");
    }

    #[test]
    fn test_gate_result_passed() {
        let result = GateResult::passed(MoonGate::Quick, "passed".to_string(), String::new());
        assert!(result.passed);
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.gate, MoonGate::Quick);
    }

    #[test]
    fn test_gate_result_failed() {
        let result = GateResult::failed(MoonGate::Quick, 1, "error".to_string(), String::new());
        assert!(!result.passed);
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn test_combine_results_all_passed() {
        let quick = GateResult::passed(MoonGate::Quick, String::new(), String::new());
        let test = GateResult::passed(MoonGate::Test, String::new(), String::new());
        let outcome = combine_results(quick, Some(test));

        assert_eq!(outcome.status, GatesStatus::AllPassed);
        assert!(outcome.status.is_success());
        assert!(!outcome.status.is_failure());
    }

    #[test]
    fn test_combine_results_quick_failed() {
        let quick = GateResult::failed(MoonGate::Quick, 1, String::new(), String::new());
        let outcome = combine_results(quick, None);

        assert_eq!(outcome.status, GatesStatus::QuickFailed);
        assert!(outcome.status.is_failure());
        assert!(outcome.test.is_none());
    }

    #[test]
    fn test_combine_results_test_failed() {
        let quick = GateResult::passed(MoonGate::Quick, String::new(), String::new());
        let test = GateResult::failed(MoonGate::Test, 1, String::new(), String::new());
        let outcome = combine_results(quick, Some(test));

        assert_eq!(outcome.status, GatesStatus::TestFailed);
        assert!(outcome.status.is_failure());
        assert!(outcome.test.is_some());
    }

    #[test]
    fn test_format_failure_message_quick() {
        let quick = GateResult::failed(
            MoonGate::Quick,
            1,
            "format error".to_string(),
            String::new(),
        );
        let outcome = combine_results(quick, None);

        let msg = format_failure_message(&outcome);
        assert!(msg.contains("Quick gate failed"));
        assert!(msg.contains("exit code 1"));
    }

    #[test]
    fn test_format_failure_message_test() {
        let quick = GateResult::passed(MoonGate::Quick, String::new(), String::new());
        let test = GateResult::failed(MoonGate::Test, 1, "test failed".to_string(), String::new());
        let outcome = combine_results(quick, Some(test));

        let msg = format_failure_message(&outcome);
        assert!(msg.contains("Test gate failed"));
    }

    #[test]
    fn test_format_failure_message_all_passed() {
        let quick = GateResult::passed(MoonGate::Quick, String::new(), String::new());
        let test = GateResult::passed(MoonGate::Test, String::new(), String::new());
        let outcome = combine_results(quick, Some(test));

        let msg = format_failure_message(&outcome);
        assert_eq!(msg, "All gates passed");
    }

    #[test]
    fn test_extract_failure_summary() {
        let stdout = vec!["Running tests...", "error: test case failed"];
        let stderr: Vec<&str> = vec![];

        let summary = extract_failure_summary(&stdout, &stderr);
        assert!(summary.contains("error"));
    }

    #[test]
    fn test_extract_failure_summary_truncation() {
        let long_error = "x".repeat(150);
        let stdout = vec![long_error.as_str()];
        let stderr: Vec<&str> = vec![];

        let summary = extract_failure_summary(&stdout, &stderr);
        assert!(summary.len() <= 103); // 100 chars + "..."
    }

    #[test]
    fn test_gates_status_display() {
        assert_eq!(format!("{}", GatesStatus::AllPassed), "all gates passed");
        assert_eq!(format!("{}", GatesStatus::QuickFailed), "quick gate failed");
        assert_eq!(format!("{}", GatesStatus::TestFailed), "test gate failed");
    }

    #[test]
    fn test_gate_error_display() {
        let err = GateError::ExecutionFailed {
            gate: MoonGate::Quick,
            reason: "timeout".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains(":quick"));
        assert!(msg.contains("timeout"));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // MOON GATE DISPLAY & TRAIT TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_moon_gate_display() {
        assert_eq!(format!("{}", MoonGate::Quick), ":quick");
        assert_eq!(format!("{}", MoonGate::Test), ":test");
    }

    #[test]
    fn test_moon_gate_debug() {
        let debug_quick = format!("{:?}", MoonGate::Quick);
        let debug_test = format!("{:?}", MoonGate::Test);
        assert!(debug_quick.contains("Quick"));
        assert!(debug_test.contains("Test"));
    }

    #[test]
    fn test_moon_gate_clone_and_copy() {
        let gate = MoonGate::Quick;
        let copied = gate;
        let cloned = gate.clone();
        assert_eq!(gate, copied);
        assert_eq!(gate, cloned);
    }

    #[test]
    fn test_moon_gate_equality() {
        assert_eq!(MoonGate::Quick, MoonGate::Quick);
        assert_eq!(MoonGate::Test, MoonGate::Test);
        assert_ne!(MoonGate::Quick, MoonGate::Test);
    }

    #[test]
    fn test_gate_error_moon_not_found_display() {
        let err = GateError::MoonNotFound;
        let msg = format!("{err}");
        assert!(msg.contains("moon binary not found"));
    }

    #[test]
    fn test_gate_error_working_directory_not_found_display() {
        let err = GateError::WorkingDirectoryNotFound("/nonexistent".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("/nonexistent"));
        assert!(msg.contains("working directory does not exist"));
    }

    #[test]
    fn test_gate_error_parse_error_display() {
        let err = GateError::ParseError("unexpected output format".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("unexpected output format"));
        assert!(msg.contains("failed to parse"));
    }

    #[test]
    fn test_gate_error_equality() {
        let err1 = GateError::MoonNotFound;
        let err2 = GateError::MoonNotFound;
        let err3 = GateError::ExecutionFailed {
            gate: MoonGate::Quick,
            reason: "x".to_string(),
        };
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_gate_error_execution_failed_with_test_gate() {
        let err = GateError::ExecutionFailed {
            gate: MoonGate::Test,
            reason: "oom".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains(":test"));
        assert!(msg.contains("oom"));
    }

    #[test]
    fn test_gate_error_debug() {
        let err = GateError::MoonNotFound;
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("MoonNotFound"));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // GATE RESULT & GATES OUTCOME TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_gate_result_new() {
        let result = GateResult::new(
            MoonGate::Test,
            true,
            0,
            "stdout output".to_string(),
            "stderr output".to_string(),
            "All tests passed".to_string(),
        );
        assert_eq!(result.gate, MoonGate::Test);
        assert!(result.passed);
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "stdout output");
        assert_eq!(result.stderr, "stderr output");
        assert_eq!(result.summary, "All tests passed");
    }

    #[test]
    fn test_gate_result_clone() {
        let result = GateResult::passed(MoonGate::Quick, "out".to_string(), "err".to_string());
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn test_gate_result_equality() {
        let r1 = GateResult::new(
            MoonGate::Quick,
            true,
            0,
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        );
        let r2 = GateResult::new(
            MoonGate::Quick,
            true,
            0,
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        );
        let r3 = GateResult::new(
            MoonGate::Test,
            true,
            0,
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        );
        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
    }

    #[test]
    fn test_gates_outcome_construction_all_passed() {
        let quick = GateResult::passed(MoonGate::Quick, String::new(), String::new());
        let test = GateResult::passed(MoonGate::Test, String::new(), String::new());
        let outcome = GatesOutcome {
            quick: quick.clone(),
            test: Some(test.clone()),
            status: GatesStatus::AllPassed,
        };
        assert_eq!(outcome.status, GatesStatus::AllPassed);
        assert_eq!(outcome.quick, quick);
        assert_eq!(outcome.test, Some(test));
    }

    #[test]
    fn test_gates_outcome_construction_quick_failed() {
        let quick = GateResult::failed(MoonGate::Quick, 1, String::new(), String::new());
        let outcome = GatesOutcome {
            quick: quick.clone(),
            test: None,
            status: GatesStatus::QuickFailed,
        };
        assert_eq!(outcome.status, GatesStatus::QuickFailed);
        assert!(outcome.test.is_none());
    }

    #[test]
    fn test_gates_outcome_clone() {
        let quick = GateResult::passed(MoonGate::Quick, String::new(), String::new());
        let test = GateResult::passed(MoonGate::Test, String::new(), String::new());
        let outcome = combine_results(quick, Some(test));
        let cloned = outcome.clone();
        assert_eq!(outcome, cloned);
    }

    #[test]
    fn test_gates_status_all_variants_success_failure() {
        assert!(GatesStatus::AllPassed.is_success());
        assert!(!GatesStatus::AllPassed.is_failure());
        assert!(!GatesStatus::QuickFailed.is_success());
        assert!(GatesStatus::QuickFailed.is_failure());
        assert!(!GatesStatus::TestFailed.is_success());
        assert!(GatesStatus::TestFailed.is_failure());
    }

    #[test]
    fn test_gates_status_debug() {
        assert!(format!("{:?}", GatesStatus::AllPassed).contains("AllPassed"));
        assert!(format!("{:?}", GatesStatus::QuickFailed).contains("QuickFailed"));
        assert!(format!("{:?}", GatesStatus::TestFailed).contains("TestFailed"));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PARSE SUMMARY ADDITIONAL COVERAGE
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_parse_summary_succeeded_keyword() {
        let stdout = "Build succeeded in 1.2s";
        let stderr = "";
        let summary = parse_summary(stdout, stderr);
        assert_eq!(summary, "Gate passed");
    }

    #[test]
    fn test_parse_summary_completed_keyword() {
        let stdout = "Tasks completed successfully";
        let stderr = "";
        let summary = parse_summary(stdout, stderr);
        assert_eq!(summary, "Gate passed");
    }

    #[test]
    fn test_parse_summary_pass_uppercase() {
        // "PASS" without "FAIL" → Gate passed
        let stdout = "2 tests PASS";
        let stderr = "";
        let summary = parse_summary(stdout, stderr);
        assert_eq!(summary, "Gate passed");
    }

    #[test]
    fn test_parse_summary_fail_uppercase() {
        let stdout = "FAIL: test_example";
        let stderr = "";
        let summary = parse_summary(stdout, stderr);
        assert!(summary.contains("FAIL"));
    }

    #[test]
    fn test_parse_summary_mixed_pass_and_fail() {
        let stdout = "1 test passed\nFAIL: another test";
        let stderr = "";
        let summary = parse_summary(stdout, stderr);
        assert_eq!(summary, "Gate completed with errors");
    }

    #[test]
    fn test_parse_summary_error_in_stderr() {
        let stdout = "";
        let stderr = "Error: out of memory";
        let summary = parse_summary(stdout, stderr);
        assert!(summary.contains("Error:"));
    }

    #[test]
    fn test_parse_summary_failure_in_stderr() {
        let stdout = "some output";
        let stderr = "task failed with exit code 1";
        let summary = parse_summary(stdout, stderr);
        assert!(summary.contains("failed"));
    }

    #[test]
    fn test_parse_summary_fallback_to_first_stdout_line() {
        let stdout = "First line of output\nSecond line";
        let stderr = "";
        let summary = parse_summary(stdout, stderr);
        assert_eq!(summary, "First line of output");
    }

    #[test]
    fn test_parse_summary_only_whitespace_stdout() {
        let stdout = "   \n  \t  ";
        let stderr = "";
        let summary = parse_summary(stdout, stderr);
        assert_eq!(summary, "Gate completed");
    }

    #[test]
    fn test_extract_failure_summary_no_errors_found() {
        let stdout: Vec<&str> = vec!["some line", "another line"];
        let stderr: Vec<&str> = vec!["unrelated output"];
        let summary = extract_failure_summary(&stdout, &stderr);
        assert_eq!(summary, "Gate failed");
    }

    #[test]
    fn test_extract_failure_summary_just_at_100_chars() {
        // 97 chars of 'x' + "FAIL" = exactly 100 chars — should NOT be truncated
        let line = format!("{}FAIL", "x".repeat(96));
        let stdout = vec![line.as_str()];
        let stderr: Vec<&str> = vec![];
        let summary = extract_failure_summary(&stdout, &stderr);
        assert_eq!(summary.len(), 100);
        assert!(!summary.ends_with("..."));
    }

    #[test]
    fn test_extract_failure_summary_from_stderr() {
        let stdout: Vec<&str> = vec!["running tests"];
        let stderr: Vec<&str> = vec!["Error: assertion failed"];
        let summary = extract_failure_summary(&stdout, &stderr);
        assert!(summary.contains("assertion failed"));
    }
}
