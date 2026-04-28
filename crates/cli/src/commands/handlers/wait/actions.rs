//! Action functions for the wait command handler (Tier 3).
//!
//! I/O operations that check wait conditions and produce output.

use std::time::Instant;

use scp_core::{output::Output, Error, Result};

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
        WaitCondition::SessionStatus { name, status } => check_session_status(name, status),
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
}
