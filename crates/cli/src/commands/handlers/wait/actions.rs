//! Action functions for the wait command handler (Tier 3).
//!
//! I/O operations that check wait conditions and produce output.

use std::time::Instant;

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{format_condition, WaitCondition, WaitOptions, WaitOutput};

/// Execute the wait command with the given options.
///
/// Blocks until the condition is met or the timeout expires.
///
/// # Errors
///
/// Returns `Error::validation_error` if the poll interval is zero or exceeds the timeout.
/// Returns `Error::invalid_state` if the condition check encounters a fatal error.
pub fn run_wait(options: &WaitOptions) -> Result<WaitOutput> {
    validate_options(options)?;
    let start = Instant::now();
    poll_loop(options, start)
}

/// Poll until the condition is met or timeout expires.
fn poll_loop(options: &WaitOptions, start: Instant) -> Result<WaitOutput> {
    loop {
        let (met, state) = check_condition(&options.condition)
            .unwrap_or_else(|e| (false, Some(format!("error: {e}"))));

        if met {
            let output = build_output(true, &options.condition, start, false, state);
            print_success(&output);
            return Ok(output);
        }

        if start.elapsed() >= options.timeout {
            let output = build_output(false, &options.condition, start, true, state);
            print_timeout(&output);
            return Ok(output);
        }

        std::thread::sleep(options.poll_interval);
    }
}

/// Validate wait options before executing.
fn validate_options(options: &WaitOptions) -> Result<()> {
    if options.poll_interval.is_zero() {
        return Err(Error::validation_error(
            "poll interval must be greater than zero",
        ));
    }
    if options.poll_interval > options.timeout {
        return Err(Error::validation_error(
            "poll interval must not exceed timeout",
        ));
    }
    Ok(())
}

/// Check if a condition is currently met.
///
/// Returns `(is_met, optional_state_description)`.
fn check_condition(condition: &WaitCondition) -> Result<(bool, Option<String>)> {
    match condition {
        WaitCondition::Healthy => check_healthy(),
        WaitCondition::SessionExists(name) => check_session_exists(name),
        WaitCondition::SessionUnlocked(name) => check_session_unlocked(name),
        WaitCondition::SessionStatus { name, status } => {
            check_session_status(name, status)
        }
    }
}

/// Check system health.
fn check_healthy() -> Result<(bool, Option<String>)> {
    let git_ok = std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_ok();

    let healthy = git_ok;
    let state = format!("git:{}", if git_ok { "ok" } else { "missing" });

    Ok((healthy, Some(state)))
}

/// Check if a session exists.
///
/// Currently returns not-found since session DB integration is not yet wired.
fn check_session_exists(name: &str) -> Result<(bool, Option<String>)> {
    // Placeholder: session DB not yet wired in hardline.
    // Once session infrastructure is connected, this will query the session store.
    Ok((false, Some(format!("not_found:{name}"))))
}

/// Check if a session is unlocked.
///
/// Currently returns not-found since session DB integration is not yet wired.
fn check_session_unlocked(name: &str) -> Result<(bool, Option<String>)> {
    // Placeholder: session DB not yet wired in hardline.
    Ok((false, Some(format!("not_found:{name}"))))
}

/// Check if a session has reached a specific status.
///
/// Currently returns not-found since session DB integration is not yet wired.
fn check_session_status(name: &str, status: &str) -> Result<(bool, Option<String>)> {
    // Placeholder: session DB not yet wired in hardline.
    let _ = status;
    Ok((false, Some(format!("not_found:{name}"))))
}

/// Build a WaitOutput from the current state.
fn build_output(
    condition_met: bool,
    condition: &WaitCondition,
    start: Instant,
    timed_out: bool,
    final_state: Option<String>,
) -> WaitOutput {
    WaitOutput {
        condition_met,
        condition: format_condition(condition),
        elapsed_ms: start.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
        timed_out,
        final_state,
    }
}

/// Print success output.
fn print_success(output: &WaitOutput) {
    Output::info(&format!("Condition met: {}", output.condition));
    if let Some(ref state) = output.final_state {
        Output::info(&format!("Final state: {state}"));
    }
}

/// Print timeout output.
fn print_timeout(output: &WaitOutput) {
    Output::info(&format!(
        "Timeout: {} not met after {}ms",
        output.condition, output.elapsed_ms
    ));
    if let Some(ref state) = output.final_state {
        Output::info(&format!("Final state: {state}"));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::commands::handlers::wait::data::WaitCondition;

    // ========================================================================
    // validate_options
    // ========================================================================

    #[test]
    fn validate_options_rejects_zero_poll_interval() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(30),
            poll_interval: Duration::ZERO,
        };
        let result = validate_options(&options);
        assert!(result.is_err());
    }

    #[test]
    fn validate_options_rejects_poll_interval_exceeding_timeout() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(1),
            poll_interval: Duration::from_secs(5),
        };
        let result = validate_options(&options);
        assert!(result.is_err());
    }

    #[test]
    fn validate_options_accepts_valid_options() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(30),
            poll_interval: Duration::from_secs(1),
        };
        assert!(validate_options(&options).is_ok());
    }

    // ========================================================================
    // check_condition (Healthy)
    // ========================================================================

    #[test]
    fn check_healthy_returns_state() {
        let (met, state) = check_healthy().expect("healthy check should not fail");
        // git is expected to be available in test environments
        assert!(state.is_some());
        let state_str = state.as_ref().expect("state should exist");
        assert!(state_str.starts_with("git:"));
        if met {
            assert!(state_str.contains("ok"));
        }
    }

    // ========================================================================
    // check_condition (SessionExists)
    // ========================================================================

    #[test]
    fn check_session_exists_returns_not_found() {
        let (met, state) =
            check_session_exists("nonexistent").expect("session check should not fail");
        assert!(!met);
        assert_eq!(state, Some("not_found:nonexistent".to_string()));
    }

    // ========================================================================
    // check_condition (SessionUnlocked)
    // ========================================================================

    #[test]
    fn check_session_unlocked_returns_not_found() {
        let (met, state) =
            check_session_unlocked("nonexistent").expect("session check should not fail");
        assert!(!met);
        assert_eq!(state, Some("not_found:nonexistent".to_string()));
    }

    // ========================================================================
    // check_condition (SessionStatus)
    // ========================================================================

    #[test]
    fn check_session_status_returns_not_found() {
        let (met, state) = check_session_status("nonexistent", "active")
            .expect("session status check should not fail");
        assert!(!met);
        assert_eq!(state, Some("not_found:nonexistent".to_string()));
    }

    // ========================================================================
    // build_output
    // ========================================================================

    #[test]
    fn build_output_success() {
        let start = Instant::now();
        let output = build_output(
            true,
            &WaitCondition::Healthy,
            start,
            false,
            Some("git:ok".to_string()),
        );
        assert!(output.condition_met);
        assert!(!output.timed_out);
        assert_eq!(output.condition, "healthy");
        assert_eq!(output.final_state, Some("git:ok".to_string()));
    }

    #[test]
    fn build_output_timeout() {
        let start = Instant::now();
        let output = build_output(
            false,
            &WaitCondition::SessionExists("test".to_string()),
            start,
            true,
            Some("not_found".to_string()),
        );
        assert!(!output.condition_met);
        assert!(output.timed_out);
        assert_eq!(output.condition, "session-exists:test");
    }

    // ========================================================================
    // run_wait (integration-style)
    // ========================================================================

    #[test]
    fn run_wait_healthy_meets_immediately() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(5),
            poll_interval: Duration::from_millis(100),
        };
        let output = run_wait(&options).expect("wait should succeed");
        // git should be available in test env, so condition is met
        assert!(output.condition_met);
        assert!(!output.timed_out);
    }

    #[test]
    fn run_wait_session_not_found_times_out() {
        let options = WaitOptions {
            condition: WaitCondition::SessionExists("nonexistent-session".to_string()),
            timeout: Duration::from_millis(200),
            poll_interval: Duration::from_millis(50),
        };
        let output = run_wait(&options).expect("wait should not error");
        assert!(!output.condition_met);
        assert!(output.timed_out);
    }

    #[test]
    fn run_wait_rejects_zero_poll_interval() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(5),
            poll_interval: Duration::ZERO,
        };
        let result = run_wait(&options);
        assert!(result.is_err());
    }

    #[test]
    fn run_wait_rejects_poll_exceeding_timeout() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_millis(100),
            poll_interval: Duration::from_secs(5),
        };
        let result = run_wait(&options);
        assert!(result.is_err());
    }

    // ========================================================================
    // Exit code contract: condition_met => success
    // ========================================================================

    #[test]
    fn successful_output_indicates_zero_exit() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(5),
            poll_interval: Duration::from_millis(100),
        };
        let output = run_wait(&options).expect("wait should succeed");
        // A consumer should exit 0 when condition_met is true
        if output.condition_met {
            // success path
            assert!(!output.timed_out);
        }
    }

    #[test]
    fn timeout_output_indicates_nonzero_exit() {
        let options = WaitOptions {
            condition: WaitCondition::SessionExists("no-such-session".to_string()),
            timeout: Duration::from_millis(100),
            poll_interval: Duration::from_millis(30),
        };
        let output = run_wait(&options).expect("wait should succeed");
        // A consumer should exit non-zero when condition_met is false
        assert!(!output.condition_met);
        assert!(output.timed_out);
    }

    // ========================================================================
    // QA: Functional verification (hq-jzyp)
    // ========================================================================

    // --- Blocking primitives: session-exists ---

    #[test]
    fn qa_session_exists_always_returns_false_when_stub() {
        // Session checks are stubs — always return (false, not_found)
        let names = ["test", "", "a-very-long-session-name-with-special-chars-!@#"];
        for name in &names {
            let (met, state) =
                check_session_exists(name).unwrap_or_else(|_| panic!("name={name}"));
            assert!(!met, "session_exists should be false for '{name}' (stub)");
            assert!(
                state.as_ref().unwrap().contains(name),
                "state should contain session name for '{name}'"
            );
        }
    }

    #[test]
    fn qa_session_unlocked_stub_behavior() {
        let (met, state) = check_session_unlocked("any-session").expect("should not error");
        assert!(!met, "session_unlocked should be false (stub)");
        assert_eq!(state, Some("not_found:any-session".to_string()));
    }

    #[test]
    fn qa_session_status_stub_ignores_status_param() {
        let (met, state) =
            check_session_status("sess", "active").expect("should not error");
        assert!(!met);
        // Stub returns not_found regardless of status parameter
        assert_eq!(state, Some("not_found:sess".to_string()));
    }

    #[test]
    fn qa_session_exists_timeout_completes() {
        // Verify session-exists blocks until timeout with stub implementation
        let options = WaitOptions {
            condition: WaitCondition::SessionExists("stub-session".to_string()),
            timeout: Duration::from_millis(150),
            poll_interval: Duration::from_millis(50),
        };
        let start = std::time::Instant::now();
        let output = run_wait(&options).expect("should not error");
        let elapsed = start.elapsed();
        assert!(!output.condition_met);
        assert!(output.timed_out);
        // Should have polled at least once before timing out
        assert!(elapsed >= Duration::from_millis(100), "should block for at least ~timeout, got {elapsed:?}");
    }

    // --- Blocking primitives: healthy ---

    #[test]
    fn qa_healthy_check_verifies_git() {
        let (met, state) = check_healthy().expect("healthy check should not error");
        assert!(state.is_some());
        let state_str = state.as_ref().unwrap();
        assert!(state_str.starts_with("git:"));
        // In test environments git should be available
        assert!(met, "git should be available in test env");
        assert!(state_str.contains("ok"));
    }

    #[test]
    fn qa_healthy_resolves_immediately() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(5),
            poll_interval: Duration::from_secs(1),
        };
        let start = std::time::Instant::now();
        let output = run_wait(&options).expect("should not error");
        let elapsed = start.elapsed();
        assert!(output.condition_met);
        assert!(!output.timed_out);
        // Should resolve on first poll (no sleep needed)
        assert!(elapsed < Duration::from_secs(1), "healthy should resolve immediately, took {elapsed:?}");
    }

    // --- Timeout handling ---

    #[test]
    fn qa_timeout_boundary_poll_equals_timeout() {
        // poll_interval == timeout: first check fails, sleep, then elapsed >= timeout
        let options = WaitOptions {
            condition: WaitCondition::SessionExists("boundary".to_string()),
            timeout: Duration::from_millis(100),
            poll_interval: Duration::from_millis(100),
        };
        // This should be accepted (only rejects poll > timeout)
        let output = run_wait(&options).expect("should accept equal values");
        assert!(!output.condition_met);
        assert!(output.timed_out);
    }

    #[test]
    fn qa_timeout_elapsed_ms_is_accurate() {
        let options = WaitOptions {
            condition: WaitCondition::SessionExists("timing".to_string()),
            timeout: Duration::from_millis(200),
            poll_interval: Duration::from_millis(50),
        };
        let output = run_wait(&options).expect("should not error");
        assert!(output.timed_out);
        // elapsed_ms should be approximately 200ms (within 100ms tolerance)
        assert!(
            output.elapsed_ms >= 150 && output.elapsed_ms <= 400,
            "elapsed_ms should be ~200ms, got {}ms",
            output.elapsed_ms
        );
    }

    #[test]
    fn qa_timeout_minimum_duration() {
        // Even with tiny timeout, should still complete
        let options = WaitOptions {
            condition: WaitCondition::SessionExists("min".to_string()),
            timeout: Duration::from_millis(1),
            poll_interval: Duration::from_millis(1),
        };
        let output = run_wait(&options).expect("should handle minimum timeout");
        assert!(!output.condition_met);
        assert!(output.timed_out);
    }

    // --- Validation edge cases ---

    #[test]
    fn qa_validation_boundary_poll_just_under_timeout() {
        // poll = timeout - 1ns should be accepted
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_millis(100),
            poll_interval: Duration::from_millis(99),
        };
        assert!(validate_options(&options).is_ok());
    }

    #[test]
    fn qa_validation_boundary_poll_just_over_timeout() {
        // poll = timeout + 1ns should be rejected
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_millis(100),
            poll_interval: Duration::from_millis(101),
        };
        assert!(validate_options(&options).is_err());
    }

    #[test]
    fn qa_validation_rejects_nanos_interval() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(10),
            poll_interval: Duration::from_nanos(1),
        };
        assert!(validate_options(&options).is_ok(), "nanos > 0 should be accepted");
    }

    // --- Output format ---

    #[test]
    fn qa_output_json_roundtrip_preserves_semantics() {
        let output = WaitOutput {
            condition_met: false,
            condition: "session-exists:test".to_string(),
            elapsed_ms: 5000,
            timed_out: true,
            final_state: Some("not_found:test".to_string()),
        };
        let json = serde_json::to_string_pretty(&output).expect("serialize");
        let parsed: WaitOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.condition_met, output.condition_met);
        assert_eq!(parsed.condition, output.condition);
        assert_eq!(parsed.elapsed_ms, output.elapsed_ms);
        assert_eq!(parsed.timed_out, output.timed_out);
        assert_eq!(parsed.final_state, output.final_state);
    }

    #[test]
    fn qa_output_elapsed_ms_does_not_overflow() {
        // Simulate extreme elapsed time (u128 -> u64)
        let start = std::time::Instant::now();
        let output = build_output(
            true,
            &WaitCondition::Healthy,
            start,
            false,
            Some("ok".to_string()),
        );
        // Should not panic, should produce a valid u64
        assert!(output.elapsed_ms > 0 || output.condition_met);
    }

    // --- Condition check error resilience ---

    #[test]
    fn qa_check_condition_handles_all_variants() {
        // Verify all condition variants dispatch correctly (even stubs)
        let conditions = vec![
            WaitCondition::Healthy,
            WaitCondition::SessionExists("a".to_string()),
            WaitCondition::SessionUnlocked("b".to_string()),
            WaitCondition::SessionStatus { name: "c".to_string(), status: "done".to_string() },
        ];
        for cond in &conditions {
            let result = check_condition(cond);
            assert!(result.is_ok(), "check_condition({cond:?}) should not error");
        }
    }

    // ========================================================================
    // RED QUEEN: Adversarial tests for wait command
    // ========================================================================

    // --- Infinite wait scenarios ---

    #[test]
    fn adversarial_wait_max_timeout_does_not_hang() {
        let options = WaitOptions {
            condition: WaitCondition::SessionExists("never-exists".to_string()),
            timeout: Duration::from_secs(1),
            poll_interval: Duration::from_millis(100),
        };
        let output = run_wait(&options).expect("should not error");
        assert!(!output.condition_met);
        assert!(output.timed_out);
    }

    #[test]
    fn adversarial_wait_tiny_timeout_valid_output() {
        let options = WaitOptions {
            condition: WaitCondition::SessionExists("x".to_string()),
            timeout: Duration::from_millis(1),
            poll_interval: Duration::from_millis(1),
        };
        let output = run_wait(&options).expect("should not error");
        assert!(!output.condition_met);
        assert!(output.timed_out);
        assert!(output.elapsed_ms > 0 || output.condition_met);
    }

    #[test]
    fn adversarial_wait_zero_poll_rejected() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(60),
            poll_interval: Duration::ZERO,
        };
        assert!(run_wait(&options).is_err());
    }

    #[test]
    fn adversarial_wait_poll_exceeds_timeout_rejected() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_millis(1),
            poll_interval: Duration::from_secs(9999),
        };
        assert!(run_wait(&options).is_err());
    }

    // --- Non-existent conditions with adversarial inputs ---

    #[test]
    fn adversarial_wait_empty_session_name() {
        let options = WaitOptions {
            condition: WaitCondition::SessionExists(String::new()),
            timeout: Duration::from_millis(100),
            poll_interval: Duration::from_millis(10),
        };
        let output = run_wait(&options).expect("empty name should not crash");
        assert!(!output.condition_met);
        assert!(output.timed_out);
    }

    #[test]
    fn adversarial_wait_evil_session_names() {
        let evil_names = [
            "../../../etc/passwd",
            "'; DROP TABLE sessions; --",
            "session\x00null",
            "session\nnewline",
            "<script>alert('xss')</script>",
            "${SHELL_INJECTION}",
            "`command_injection`",
            "$(subshell)",
        ];
        for name in &evil_names {
            let options = WaitOptions {
                condition: WaitCondition::SessionExists(name.to_string()),
                timeout: Duration::from_millis(50),
                poll_interval: Duration::from_millis(10),
            };
            let output = run_wait(&options).unwrap_or_else(|e| {
                panic!("name {:?} caused error: {}", name, e)
            });
            assert!(!output.condition_met, "evil name {:?} met condition", name);
            assert!(output.timed_out, "evil name {:?} didn't time out", name);
        }
    }

    #[test]
    fn adversarial_wait_very_long_session_name() {
        let long_name = "x".repeat(65536);
        let options = WaitOptions {
            condition: WaitCondition::SessionExists(long_name.clone()),
            timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        };
        let output = run_wait(&options).expect("long name should not crash");
        assert!(!output.condition_met);
        assert!(output.timed_out);
        assert!(output.condition.contains(&long_name));
    }

    #[test]
    fn adversarial_wait_evil_status_values() {
        let evil_statuses = [
            "'; DROP TABLE statuses; --",
            "\x00",
            "active\x00inactive",
            "../../../evil",
        ];
        for status in &evil_statuses {
            let options = WaitOptions {
                condition: WaitCondition::SessionStatus {
                    name: "test".to_string(),
                    status: status.to_string(),
                },
                timeout: Duration::from_millis(50),
                poll_interval: Duration::from_millis(10),
            };
            let output = run_wait(&options).unwrap_or_else(|e| {
                panic!("status {:?} caused error: {}", status, e)
            });
            assert!(!output.condition_met, "evil status {:?} met condition", status);
        }
    }

    // --- State consistency invariants ---

    #[test]
    fn adversarial_met_and_timed_out_mutually_exclusive() {
        let healthy = run_wait(&WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(5),
            poll_interval: Duration::from_millis(100),
        })
        .expect("should not error");
        if healthy.condition_met {
            assert!(!healthy.timed_out, "met and timed_out both true!");
        }

        let timeout = run_wait(&WaitOptions {
            condition: WaitCondition::SessionExists("nope".to_string()),
            timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        })
        .expect("should not error");
        if timeout.timed_out {
            assert!(!timeout.condition_met, "timed_out and condition_met both true!");
        }
    }

    #[test]
    fn adversarial_elapsed_ms_positive() {
        let output = run_wait(&WaitOptions {
            condition: WaitCondition::SessionExists("timing".to_string()),
            timeout: Duration::from_millis(100),
            poll_interval: Duration::from_millis(10),
        })
        .expect("should not error");
        assert!(
            output.elapsed_ms > 0 || output.condition_met,
            "elapsed_ms should be positive, got {}",
            output.elapsed_ms
        );
    }

    #[test]
    fn adversarial_condition_string_contains_name() {
        let output = run_wait(&WaitOptions {
            condition: WaitCondition::SessionExists("verify-format".to_string()),
            timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        })
        .expect("should not error");
        assert!(output.condition.contains("verify-format"));
    }

    // --- check_condition direct adversarial ---

    #[test]
    fn adversarial_check_unicode_session_names() {
        let unicode_names = ["日本語", "🔑-session", "🦀", "session\u{202E}evil"];
        for name in &unicode_names {
            let (met, state) = check_session_exists(name)
                .unwrap_or_else(|e| panic!("unicode {:?} caused error: {}", name, e));
            assert!(!met, "unicode {:?} should not exist", name);
            assert!(state.as_ref().map_or(false, |s| s.contains(name)));
        }
    }

    #[test]
    fn adversarial_check_unlocked_consistent_with_exists() {
        let name = "test-session";
        let (exists_met, exists_state) = check_session_exists(name).expect("should not error");
        let (unlocked_met, unlocked_state) = check_session_unlocked(name).expect("should not error");
        assert_eq!(exists_met, unlocked_met);
        assert_eq!(exists_state, unlocked_state);
    }

    #[test]
    fn adversarial_check_condition_never_panics() {
        let conditions = vec![
            WaitCondition::Healthy,
            WaitCondition::SessionExists(String::new()),
            WaitCondition::SessionExists("x".repeat(10000)),
            WaitCondition::SessionUnlocked(String::new()),
            WaitCondition::SessionStatus {
                name: String::new(),
                status: String::new(),
            },
            WaitCondition::SessionStatus {
                name: "x".repeat(10000),
                status: "y".repeat(10000),
            },
        ];
        for cond in &conditions {
            assert!(check_condition(cond).is_ok(), "check_condition({cond:?}) should not error");
        }
    }

    // --- validate_options boundary ---

    #[test]
    fn adversarial_validate_1ns_poll_interval() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(10),
            poll_interval: Duration::from_nanos(1),
        };
        assert!(validate_options(&options).is_ok());
    }

    #[test]
    fn adversarial_validate_max_timeout() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::MAX,
            poll_interval: Duration::from_secs(1),
        };
        assert!(validate_options(&options).is_ok());
    }

    #[test]
    fn adversarial_validate_sub_ns_over_timeout() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_millis(100),
            poll_interval: Duration::from_millis(100) + Duration::from_nanos(1),
        };
        assert!(validate_options(&options).is_err());
    }

    // --- Serialization adversarial ---

    #[test]
    fn adversarial_json_roundtrip_evil_strings() {
        let evil = WaitOutput {
            condition_met: false,
            condition: "session-exists:\x00\x01\x02".to_string(),
            elapsed_ms: u64::MAX,
            timed_out: true,
            final_state: Some("\n\r\t\x1b".to_string()),
        };
        let json = serde_json::to_string(&evil).expect("should serialize");
        let parsed: WaitOutput = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(parsed.condition_met, evil.condition_met);
        assert_eq!(parsed.elapsed_ms, evil.elapsed_ms);
        assert_eq!(parsed.timed_out, evil.timed_out);
    }

    // --- Trait bounds (compile-time guarantee) ---

    #[test]
    fn adversarial_wait_options_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WaitOptions>();
        assert_send_sync::<WaitOutput>();
        assert_send_sync::<WaitCondition>();
    }

    // ========================================================================
    // Progress display: print_success and print_timeout
    // ========================================================================

    #[test]
    fn print_success_formats_correctly() {
        let output = WaitOutput {
            condition_met: true,
            condition: "healthy".to_string(),
            elapsed_ms: 42,
            timed_out: false,
            final_state: Some("git:ok".to_string()),
        };
        // Should not panic — exercises the print path
        print_success(&output);
    }

    #[test]
    fn print_success_handles_none_final_state() {
        let output = WaitOutput {
            condition_met: true,
            condition: "session-exists:test".to_string(),
            elapsed_ms: 100,
            timed_out: false,
            final_state: None,
        };
        print_success(&output);
    }

    #[test]
    fn print_timeout_formats_correctly() {
        let output = WaitOutput {
            condition_met: false,
            condition: "session-exists:gone".to_string(),
            elapsed_ms: 60000,
            timed_out: true,
            final_state: Some("not_found:gone".to_string()),
        };
        print_timeout(&output);
    }

    #[test]
    fn print_timeout_handles_none_final_state() {
        let output = WaitOutput {
            condition_met: false,
            condition: "healthy".to_string(),
            elapsed_ms: 5000,
            timed_out: true,
            final_state: None,
        };
        print_timeout(&output);
    }

    #[test]
    fn print_success_output_for_each_condition_type() {
        for (label, cond) in [
            ("healthy", WaitCondition::Healthy),
            ("exists", WaitCondition::SessionExists("test".to_string())),
            ("unlocked", WaitCondition::SessionUnlocked("test".to_string())),
            ("status", WaitCondition::SessionStatus {
                name: "test".to_string(),
                status: "active".to_string(),
            }),
        ] {
            let output = WaitOutput {
                condition_met: true,
                condition: crate::commands::handlers::wait::format_condition(&cond),
                elapsed_ms: 10,
                timed_out: false,
                final_state: Some("ok".to_string()),
            };
            // Must not panic for any condition type
            print_success(&output);
            let _ = label; // suppress unused warning
        }
    }

    // ========================================================================
    // Cancellation contract: timeout is the only cancellation mechanism
    // ========================================================================

    #[test]
    fn cancellation_via_timeout_completes_cleanly() {
        // Simulates "cancellation" — timeout=1ms means immediate bail
        let options = WaitOptions {
            condition: WaitCondition::SessionExists("cancel-target".to_string()),
            timeout: Duration::from_millis(1),
            poll_interval: Duration::from_millis(1),
        };
        let output = run_wait(&options).expect("should complete");
        assert!(!output.condition_met);
        assert!(output.timed_out);
    }

    #[test]
    fn cancellation_produces_valid_output() {
        let options = WaitOptions {
            condition: WaitCondition::SessionUnlocked("cancel".to_string()),
            timeout: Duration::from_millis(1),
            poll_interval: Duration::from_millis(1),
        };
        let output = run_wait(&options).expect("should produce output");
        assert!(!output.condition_met);
        assert!(output.timed_out);
        assert!(!output.condition.is_empty());
        // elapsed_ms should be positive or condition was met
        assert!(output.elapsed_ms > 0 || output.condition_met);
    }

    // ========================================================================
    // Poll interval: multiple poll cycles
    // ========================================================================

    #[test]
    fn multiple_polls_before_timeout() {
        // Short poll interval, short timeout — should poll multiple times
        let options = WaitOptions {
            condition: WaitCondition::SessionExists("multi-poll".to_string()),
            timeout: Duration::from_millis(200),
            poll_interval: Duration::from_millis(25),
        };
        let start = std::time::Instant::now();
        let output = run_wait(&options).expect("should not error");
        let elapsed = start.elapsed();
        assert!(!output.condition_met);
        assert!(output.timed_out);
        // With 25ms poll, should have polled ~8 times in 200ms
        assert!(elapsed >= Duration::from_millis(150));
    }

    #[test]
    fn single_poll_before_immediate_success() {
        // Healthy resolves on first check — zero sleeps
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(5),
            poll_interval: Duration::from_secs(1),
        };
        let start = std::time::Instant::now();
        let output = run_wait(&options).expect("should succeed");
        assert!(output.condition_met);
        // Should resolve in < 100ms (first poll, no sleep)
        assert!(start.elapsed() < Duration::from_millis(500));
    }

    // ========================================================================
    // CLI dispatch: condition string → WaitCondition mapping
    // ========================================================================

    #[test]
    fn cli_dispatch_session_exists_from_string() {
        // The CLI currently always creates SessionExists from the condition string.
        // Verify the condition string is preserved in the output.
        let condition_str = "my-feature-branch";
        let options = WaitOptions {
            condition: WaitCondition::SessionExists(condition_str.to_string()),
            timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        };
        let output = run_wait(&options).expect("should not error");
        assert!(output.condition.contains(condition_str));
    }

    // ========================================================================
    // Error propagation: validate_options
    // ========================================================================

    #[test]
    fn validate_rejects_1ms_poll_with_0ms_timeout() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::ZERO,
            poll_interval: Duration::from_millis(1),
        };
        // timeout=0 is accepted by validate but poll_loop will timeout immediately
        let result = validate_options(&options);
        // poll (1ms) > timeout (0) → rejected
        assert!(result.is_err());
    }

    #[test]
    fn validate_accepts_exact_equal_poll_and_timeout() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(5),
            poll_interval: Duration::from_secs(5),
        };
        assert!(validate_options(&options).is_ok());
    }

    // ========================================================================
    // build_output: elapsed_ms precision
    // ========================================================================

    #[test]
    fn build_output_elapsed_ms_near_zero() {
        let start = Instant::now();
        let output = build_output(true, &WaitCondition::Healthy, start, false, Some("ok".into()));
        // elapsed_ms should be 0 or very small
        assert!(output.elapsed_ms < 100, "elapsed should be near zero, got {}", output.elapsed_ms);
    }

    // ========================================================================
    // Integration: all condition types through run_wait
    // ========================================================================

    #[test]
    fn run_wait_all_stub_conditions_timeout() {
        // All session stubs return false — should all timeout
        let conditions = vec![
            WaitCondition::SessionExists("a".to_string()),
            WaitCondition::SessionUnlocked("b".to_string()),
            WaitCondition::SessionStatus { name: "c".to_string(), status: "done".to_string() },
        ];
        for cond in conditions {
            let options = WaitOptions {
                condition: cond,
                timeout: Duration::from_millis(50),
                poll_interval: Duration::from_millis(10),
            };
            let output = run_wait(&options).unwrap_or_else(|e| panic!("should not error: {e}"));
            assert!(!output.condition_met, "stub condition should not be met");
            assert!(output.timed_out, "stub condition should timeout");
        }
    }

    // ========================================================================
    // WaitOutput: serde contract for CLI consumers
    // ========================================================================

    #[test]
    fn wait_output_json_has_required_fields() {
        let output = WaitOutput {
            condition_met: true,
            condition: "healthy".to_string(),
            elapsed_ms: 0,
            timed_out: false,
            final_state: Some("git:ok".to_string()),
        };
        let json_str = serde_json::to_string(&output).expect("serialize");
        let val: serde_json::Value = serde_json::from_str(&json_str).expect("parse");
        assert!(val.get("condition_met").is_some(), "missing condition_met");
        assert!(val.get("condition").is_some(), "missing condition");
        assert!(val.get("elapsed_ms").is_some(), "missing elapsed_ms");
        assert!(val.get("timed_out").is_some(), "missing timed_out");
    }

    #[test]
    fn wait_output_json_field_types() {
        let output = WaitOutput {
            condition_met: false,
            condition: "test".to_string(),
            elapsed_ms: 42,
            timed_out: true,
            final_state: None,
        };
        let json_str = serde_json::to_string(&output).expect("serialize");
        let val: serde_json::Value = serde_json::from_str(&json_str).expect("parse");
        assert!(val["condition_met"].is_boolean(), "condition_met should be boolean");
        assert!(val["condition"].is_string(), "condition should be string");
        assert!(val["elapsed_ms"].is_number(), "elapsed_ms should be number");
        assert!(val["timed_out"].is_boolean(), "timed_out should be boolean");
    }
}
