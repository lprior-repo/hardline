//! Wait command - blocking primitives for AI agents
//!
//! Provides commands that block until conditions are met:
//! - `isolate wait session-exists <name>` - Wait for session to exist
//! - `isolate wait session-unlocked <name>` - Wait for session to be unlocked
//! - `isolate wait healthy` - Wait for system to be healthy

use std::time::{Duration, Instant};

use isolate_core::{json::SchemaEnvelope, OutputFormat};
use serde::Serialize;

use crate::{commands::get_session_db, IsolateError, Result};

/// Options for wait command
#[derive(Debug, Clone)]
pub struct WaitOptions {
    pub condition: WaitCondition,
    pub timeout: Duration,
    pub poll_interval: Duration,
    pub format: OutputFormat,
}

/// Wait condition types
#[derive(Debug, Clone)]
pub enum WaitCondition {
    /// Wait for session to exist
    SessionExists(String),
    /// Wait for session to be unlocked (not in use by another agent)
    SessionUnlocked(String),
    /// Wait for system to be healthy
    Healthy,
    /// Wait for session to reach a specific status
    SessionStatus { name: String, status: String },
}

/// Wait result output
#[derive(Debug, Clone, Serialize)]
pub struct WaitOutput {
    /// Whether the condition was met
    pub condition_met: bool,
    /// The condition that was waited for
    pub condition: String,
    /// How long we waited (in milliseconds)
    pub elapsed_ms: u64,
    /// Whether we timed out
    pub timed_out: bool,
    /// Current state when condition was met or timed out
    pub final_state: Option<String>,
}

/// Create a `WaitOutput` for a given result
fn make_output(
    success: bool,
    condition: &WaitCondition,
    start: Instant,
    timed_out: bool,
    state: Option<String>,
) -> WaitOutput {
    WaitOutput {
        condition_met: success,
        condition: format_condition(condition),
        elapsed_ms: u64::try_from(start.elapsed().as_millis()).map_or(u64::MAX, |v| v),
        timed_out,
        final_state: state,
    }
}

/// Check if jj command is available
async fn is_jj_available() -> bool {
    tokio::process::Command::new("jj")
        .arg("--version")
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Run the wait command
pub async fn run(options: &WaitOptions) -> Result<i32> {
    let start = Instant::now();

    loop {
        // Check if condition is met
        // We catch errors here and treat them as "not met yet" to allow for transient
        // issues or initialization delays (e.g. database being created)
        let (met, state) = match check_condition(&options.condition).await {
            Ok(res) => res,
            Err(e) => (false, Some(format!("error: {e}"))),
        };

        if met {
            return output_result(
                &make_output(true, &options.condition, start, false, state),
                options.format,
            );
        }

        // Check timeout
        if start.elapsed() >= options.timeout {
            return output_result(
                &make_output(false, &options.condition, start, true, state),
                options.format,
            );
        }

        // Wait before next poll
        tokio::time::sleep(options.poll_interval).await;
    }
}

/// Check if a condition is met
async fn check_condition(condition: &WaitCondition) -> Result<(bool, Option<String>)> {
    match condition {
        WaitCondition::SessionExists(name) => match get_session_db().await {
            Ok(db) => match db.get(name).await {
                Ok(Some(session)) => Ok((true, Some(format!("status:{}", session.status)))),
                Ok(None) => Ok((false, Some("not_found".to_string()))),
                Err(_) => Ok((false, Some("error".to_string()))),
            },
            Err(_) => Ok((false, Some("db_unavailable".to_string()))),
        },

        WaitCondition::SessionUnlocked(name) => {
            match get_session_db().await {
                Ok(db) => {
                    match db.get(name).await {
                        Ok(Some(session)) => {
                            // Check if session is locked (has an active agent)
                            let locked = session
                                .metadata
                                .as_ref()
                                .and_then(|m| m.get("locked_by"))
                                .is_some();
                            Ok((!locked, Some(format!("locked:{locked}"))))
                        }
                        Ok(None) => {
                            // Session doesn't exist - can't be "unlocked"
                            // This is semantically different from being unlocked
                            Ok((false, Some("not_found".to_string())))
                        }
                        Err(_) => Ok((false, Some("error".to_string()))),
                    }
                }
                Err(_) => Ok((false, Some("db_unavailable".to_string()))),
            }
        }

        WaitCondition::Healthy => {
            // Check if system is healthy
            let jj_ok = is_jj_available().await;
            let db_ok = get_session_db().await.is_ok();

            let healthy = jj_ok && db_ok;
            let state = format!(
                "jj:{},db:{}",
                if jj_ok { "ok" } else { "missing" },
                if db_ok { "ok" } else { "error" }
            );

            Ok((healthy, Some(state)))
        }

        WaitCondition::SessionStatus { name, status } => match get_session_db().await {
            Ok(db) => match db.get(name).await {
                Ok(Some(session)) => {
                    let current_status = session.status.to_string();
                    let met = current_status == *status;
                    Ok((met, Some(format!("status:{current_status}"))))
                }
                Ok(None) => Ok((false, Some("not_found".to_string()))),
                Err(_) => Ok((false, Some("error".to_string()))),
            },
            Err(_) => Ok((false, Some("db_unavailable".to_string()))),
        },
    }
}

/// Format condition for display
fn format_condition(condition: &WaitCondition) -> String {
    match condition {
        WaitCondition::SessionExists(name) => format!("session-exists:{name}"),
        WaitCondition::SessionUnlocked(name) => format!("session-unlocked:{name}"),
        WaitCondition::Healthy => "healthy".to_string(),
        WaitCondition::SessionStatus { name, status } => {
            format!("session-status:{name}={status}")
        }
    }
}

/// Output the result
fn output_result(output: &WaitOutput, format: OutputFormat) -> Result<i32> {
    if format.is_json() {
        let envelope = SchemaEnvelope::new("wait-response", "single", output);
        let json_str = serde_json::to_string_pretty(&envelope).map_err(|e| {
            IsolateError::OperationFailed(format!("Failed to serialize wait output: {e}"))
        })?;
        println!("{json_str}");
    } else {
        if output.condition_met {
            println!("Condition met: {}", output.condition);
        } else if output.timed_out {
            println!(
                "Timeout: {} not met after {}ms",
                output.condition, output.elapsed_ms
            );
        } else {
            println!("Failed: {}", output.condition);
        }
        if let Some(ref state) = output.final_state {
            println!("Final state: {state}");
        }
    }

    if output.condition_met {
        Ok(0)
    } else {
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_condition_session_exists() {
        let cond = WaitCondition::SessionExists("test".to_string());
        assert_eq!(format_condition(&cond), "session-exists:test");
    }

    #[test]
    fn test_format_condition_healthy() {
        let cond = WaitCondition::Healthy;
        assert_eq!(format_condition(&cond), "healthy");
    }

    #[test]
    fn test_wait_output_serializes() {
        let output = WaitOutput {
            condition_met: true,
            condition: "healthy".to_string(),
            elapsed_ms: 100,
            timed_out: false,
            final_state: Some("jj:ok,db:ok".to_string()),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
    }

    #[test]
    fn test_wait_output_timeout() {
        let output = WaitOutput {
            condition_met: false,
            condition: "session-exists:test".to_string(),
            elapsed_ms: 30000,
            timed_out: true,
            final_state: Some("not_found".to_string()),
        };

        assert!(!output.condition_met);
        assert!(output.timed_out);
    }

    #[test]
    fn wait_output_result_failure_returns_exit_code_in_json_mode() {
        let output = WaitOutput {
            condition_met: false,
            condition: "healthy".to_string(),
            elapsed_ms: 1000,
            timed_out: true,
            final_state: Some("db:error".to_string()),
        };

        let exit_code = output_result(&output, isolate_core::OutputFormat::Json)
            .expect("json output should serialize");
        assert_eq!(exit_code, 1);
    }

    // ============================================================================
    // Martin Fowler Style Behavior Tests
    // These tests describe the BEHAVIOR of the wait command
    // ============================================================================

    mod condition_formatting_behavior {
        use super::*;

        /// GIVEN: A session-exists condition
        /// WHEN: Formatted for display
        /// THEN: Should include condition type and session name
        #[test]
        fn session_exists_format_includes_name() {
            let condition = WaitCondition::SessionExists("my-feature".to_string());
            let formatted = format_condition(&condition);

            assert!(
                formatted.contains("session-exists"),
                "Should identify condition type"
            );
            assert!(
                formatted.contains("my-feature"),
                "Should include session name"
            );
            assert_eq!(formatted, "session-exists:my-feature");
        }

        /// GIVEN: A session-unlocked condition
        /// WHEN: Formatted for display
        /// THEN: Should include condition type and session name
        #[test]
        fn session_unlocked_format_includes_name() {
            let condition = WaitCondition::SessionUnlocked("locked-session".to_string());
            let formatted = format_condition(&condition);

            assert_eq!(formatted, "session-unlocked:locked-session");
        }

        /// GIVEN: A healthy condition
        /// WHEN: Formatted for display
        /// THEN: Should simply say "healthy"
        #[test]
        fn healthy_format_is_simple() {
            let condition = WaitCondition::Healthy;
            let formatted = format_condition(&condition);

            assert_eq!(formatted, "healthy");
        }

        /// GIVEN: A session-status condition
        /// WHEN: Formatted for display
        /// THEN: Should include session name and target status
        #[test]
        fn session_status_format_includes_name_and_status() {
            let condition = WaitCondition::SessionStatus {
                name: "build-task".to_string(),
                status: "completed".to_string(),
            };
            let formatted = format_condition(&condition);

            assert!(
                formatted.contains("build-task"),
                "Should include session name"
            );
            assert!(
                formatted.contains("completed"),
                "Should include target status"
            );
            assert_eq!(formatted, "session-status:build-task=completed");
        }
    }

    mod wait_output_behavior {
        use super::*;

        /// GIVEN: A condition was met successfully
        /// WHEN: Output is generated
        /// THEN: success should be true and `timed_out` should be false
        #[test]
        fn successful_wait_shows_success() {
            let output = WaitOutput {
                condition_met: true,
                condition: "session-exists:my-session".to_string(),
                elapsed_ms: 50,
                timed_out: false,
                final_state: Some("status:active".to_string()),
            };

            assert!(output.condition_met, "Should indicate success");
            assert!(!output.timed_out, "Should not be timed out");
            assert!(output.elapsed_ms < 1000, "Quick success should be fast");
        }

        /// GIVEN: Timeout occurred before condition was met
        /// WHEN: Output is generated
        /// THEN: success should be false and `timed_out` should be true
        #[test]
        fn timeout_shows_failure_and_timed_out() {
            let output = WaitOutput {
                condition_met: false,
                condition: "session-exists:missing-session".to_string(),
                elapsed_ms: 30000,
                timed_out: true,
                final_state: Some("not_found".to_string()),
            };

            assert!(!output.condition_met, "Should indicate failure");
            assert!(output.timed_out, "Should indicate timeout");
            assert!(
                output.elapsed_ms >= 30000,
                "Should show full timeout duration"
            );
        }

        /// GIVEN: Condition failed for non-timeout reason
        /// WHEN: Output is generated
        /// THEN: success should be false but `timed_out` should also be false
        #[test]
        fn failure_without_timeout_is_distinct() {
            let output = WaitOutput {
                condition_met: false,
                condition: "healthy".to_string(),
                elapsed_ms: 100,
                timed_out: false,
                final_state: Some("jj:missing".to_string()),
            };

            assert!(!output.condition_met, "Should indicate failure");
            assert!(!output.timed_out, "Should not be timeout");
        }

        /// GIVEN: Wait completed
        /// WHEN: Output includes `final_state`
        /// THEN: It should describe why the wait ended
        #[test]
        fn final_state_explains_outcome() {
            // Success case
            let success_output = WaitOutput {
                condition_met: true,
                condition: "session-exists:test".to_string(),
                elapsed_ms: 500,
                timed_out: false,
                final_state: Some("status:active".to_string()),
            };
            assert!(
                success_output
                    .final_state
                    .as_ref()
                    .is_some_and(|s| s.contains("active")),
                "Success final_state should contain 'active'"
            );

            // Failure case
            let failure_output = WaitOutput {
                condition_met: false,
                condition: "healthy".to_string(),
                elapsed_ms: 100,
                timed_out: false,
                final_state: Some("jj:missing".to_string()),
            };
            assert!(
                failure_output
                    .final_state
                    .as_ref()
                    .is_some_and(|s| s.contains("missing")),
                "Failure final_state should contain 'missing'"
            );
        }

        /// GIVEN: a successful wait result
        /// WHEN: output_result is evaluated for JSON mode
        /// THEN: command should return exit code 0
        #[test]
        fn given_successful_output_when_returning_exit_code_then_is_zero() {
            let output = WaitOutput {
                condition_met: true,
                condition: "healthy".to_string(),
                elapsed_ms: 1,
                timed_out: false,
                final_state: Some("jj:ok,db:ok".to_string()),
            };

            let code = output_result(&output, isolate_core::OutputFormat::Json)
                .expect("successful output should serialize");
            assert_eq!(code, 0);
        }

        /// GIVEN: a failed wait result
        /// WHEN: output_result is evaluated for JSON mode
        /// THEN: command should return exit code 1
        #[test]
        fn given_failed_output_when_returning_exit_code_then_is_one() {
            let output = WaitOutput {
                condition_met: false,
                condition: "session-exists:missing".to_string(),
                elapsed_ms: 1000,
                timed_out: true,
                final_state: Some("not_found".to_string()),
            };

            let code = output_result(&output, isolate_core::OutputFormat::Json)
                .expect("failed output should still serialize");
            assert_eq!(code, 1);
        }

        struct ExitCodeCase {
            name: &'static str,
            condition_met: bool,
            timed_out: bool,
            expected_code: i32,
        }

        /// GIVEN: a matrix of wait outcomes
        /// WHEN: output exit-code resolution runs for each row
        /// THEN: exit codes should stay stable across success/failure variants
        #[test]
        fn given_wait_outcome_matrix_when_resolving_exit_code_then_matches_contract() {
            let cases = [
                ExitCodeCase {
                    name: "success without timeout",
                    condition_met: true,
                    timed_out: false,
                    expected_code: 0,
                },
                ExitCodeCase {
                    name: "failure by timeout",
                    condition_met: false,
                    timed_out: true,
                    expected_code: 1,
                },
                ExitCodeCase {
                    name: "failure without timeout",
                    condition_met: false,
                    timed_out: false,
                    expected_code: 1,
                },
            ];

            for case in cases {
                let output = WaitOutput {
                    condition_met: case.condition_met,
                    condition: "healthy".to_string(),
                    elapsed_ms: 12,
                    timed_out: case.timed_out,
                    final_state: Some("jj:ok,db:error".to_string()),
                };

                let code = output_result(&output, isolate_core::OutputFormat::Json)
                    .expect("matrix case should serialize");
                assert_eq!(
                    code, case.expected_code,
                    "case '{}' returned wrong exit code",
                    case.name
                );
            }
        }
    }

    mod wait_condition_behavior {
        use super::*;

        /// GIVEN: Different wait condition types exist
        /// WHEN: They are used
        /// THEN: Each should represent a distinct wait scenario
        #[test]
        fn all_condition_types_are_distinct() {
            let conditions: Vec<WaitCondition> = vec![
                WaitCondition::SessionExists("a".to_string()),
                WaitCondition::SessionUnlocked("b".to_string()),
                WaitCondition::Healthy,
                WaitCondition::SessionStatus {
                    name: "c".to_string(),
                    status: "active".to_string(),
                },
            ];

            // Each condition type should format differently
            let formatted: Vec<String> = conditions.iter().map(format_condition).collect();

            // All should be unique
            let mut unique = formatted.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(
                unique.len(),
                formatted.len(),
                "All conditions should format uniquely"
            );
        }

        /// GIVEN: A session name
        /// WHEN: Used in `SessionExists` condition
        /// THEN: That exact name should be preserved
        #[test]
        fn session_name_is_preserved_in_condition() {
            let session_name = "feature-auth-oauth2-integration";
            let condition = WaitCondition::SessionExists(session_name.to_string());

            assert!(
                matches!(condition, WaitCondition::SessionExists(ref name) if name == session_name),
                "Session name should be preserved exactly in SessionExists variant"
            );
        }

        struct ConditionFormatCase {
            name: &'static str,
            condition: WaitCondition,
            expected_format: &'static str,
        }

        /// GIVEN: a matrix of condition variants
        /// WHEN: each variant is formatted
        /// THEN: formatted text should match the CLI output contract exactly
        #[test]
        fn given_condition_format_matrix_when_formatting_then_outputs_match_exact_contract() {
            let cases = [
                ConditionFormatCase {
                    name: "session exists",
                    condition: WaitCondition::SessionExists("alpha".to_string()),
                    expected_format: "session-exists:alpha",
                },
                ConditionFormatCase {
                    name: "session unlocked",
                    condition: WaitCondition::SessionUnlocked("beta".to_string()),
                    expected_format: "session-unlocked:beta",
                },
                ConditionFormatCase {
                    name: "healthy",
                    condition: WaitCondition::Healthy,
                    expected_format: "healthy",
                },
                ConditionFormatCase {
                    name: "session status",
                    condition: WaitCondition::SessionStatus {
                        name: "gamma".to_string(),
                        status: "active".to_string(),
                    },
                    expected_format: "session-status:gamma=active",
                },
            ];

            for case in cases {
                let actual = format_condition(&case.condition);
                assert_eq!(
                    actual, case.expected_format,
                    "case '{}' has wrong formatted condition",
                    case.name
                );
            }
        }
    }

    mod wait_options_behavior {
        use std::time::Duration;

        use super::*;

        /// GIVEN: Default wait options
        /// WHEN: Created
        /// THEN: Should have sensible defaults for AI agents
        #[test]
        fn default_options_are_sensible() {
            let options = WaitOptions {
                condition: WaitCondition::Healthy,
                timeout: Duration::from_secs(30),
                poll_interval: Duration::from_secs(1),
                format: isolate_core::OutputFormat::Json,
            };

            assert!(
                options.timeout.as_secs() >= 10,
                "Timeout should be at least 10s"
            );
            assert!(
                options.poll_interval.as_millis() >= 100,
                "Poll interval should be at least 100ms"
            );
            assert!(
                options.poll_interval < options.timeout,
                "Poll interval should be less than timeout"
            );
        }

        /// GIVEN: Wait options with JSON format
        /// WHEN: Output is generated
        /// THEN: Format should be preserved for AI consumption
        #[test]
        fn json_format_is_preserved() {
            let options = WaitOptions {
                condition: WaitCondition::Healthy,
                timeout: Duration::from_secs(5),
                poll_interval: Duration::from_secs(1),
                format: isolate_core::OutputFormat::Json,
            };

            assert!(options.format.is_json(), "JSON format should be preserved");
        }
    }

    mod json_output_behavior {
        use super::*;

        #[test]
        fn wait_envelope_has_single_success_field() -> std::result::Result<(), Box<dyn std::error::Error>> {
            let output = WaitOutput {
                condition_met: false,
                condition: "healthy".to_string(),
                elapsed_ms: 1,
                timed_out: true,
                final_state: Some("db:error".to_string()),
            };

            let envelope = SchemaEnvelope::new("wait-response", "single", &output);
            let json = serde_json::to_string(&envelope)?;
            let success_key_count = json.match_indices("\"success\"").count();
            assert_eq!(
                success_key_count, 1,
                "Envelope should contain one success key"
            );
            Ok(())
        }

        /// GIVEN: wait output is wrapped in schema envelope
        /// WHEN: serialized to JSON
        /// THEN: domain payload should expose `condition_met` (not `success`)
        #[test]
        fn given_wait_envelope_when_serialized_then_payload_uses_condition_met_field(
        ) -> std::result::Result<(), Box<dyn std::error::Error>> {
            let output = WaitOutput {
                condition_met: true,
                condition: "healthy".to_string(),
                elapsed_ms: 4,
                timed_out: false,
                final_state: Some("jj:ok,db:ok".to_string()),
            };

            let envelope = SchemaEnvelope::new("wait-response", "single", &output);
            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&envelope)?)?;

            assert_eq!(
                json.get("condition_met")
                    .and_then(serde_json::Value::as_bool),
                Some(true)
            );
            assert!(json
                .get("success")
                .and_then(serde_json::Value::as_bool)
                .is_some());
            Ok(())
        }

        /// GIVEN: `WaitOutput` is serialized to JSON
        /// WHEN: AI agent parses it
        /// THEN: All necessary fields should be present
        #[test]
        fn json_has_all_required_fields() -> std::result::Result<(), Box<dyn std::error::Error>> {
            let output = WaitOutput {
                condition_met: true,
                condition: "healthy".to_string(),
                elapsed_ms: 500,
                timed_out: false,
                final_state: Some("all:ok".to_string()),
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&output)?)?;

            // All fields should be present
            assert!(
                json.get("condition_met").is_some(),
                "Must have condition_met"
            );
            assert!(json.get("condition").is_some(), "Must have condition");
            assert!(json.get("elapsed_ms").is_some(), "Must have elapsed_ms");
            assert!(json.get("timed_out").is_some(), "Must have timed_out");
            assert!(json.get("final_state").is_some(), "Must have final_state");

            // Types should be correct
            assert!(json["condition_met"].is_boolean());
            assert!(json["condition"].is_string());
            assert!(json["elapsed_ms"].is_number());
            assert!(json["timed_out"].is_boolean());
            Ok(())
        }

        /// GIVEN: `WaitOutput` with timeout
        /// WHEN: Serialized and parsed
        /// THEN: Can determine why wait failed
        #[test]
        fn timeout_output_is_diagnosable() -> std::result::Result<(), Box<dyn std::error::Error>> {
            let output = WaitOutput {
                condition_met: false,
                condition: "session-exists:missing".to_string(),
                elapsed_ms: 30000,
                timed_out: true,
                final_state: Some("not_found".to_string()),
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&output)?)?;

            // AI can determine the cause
            assert_eq!(json["condition_met"].as_bool(), Some(false));
            assert_eq!(json["timed_out"].as_bool(), Some(true));
            if let Some(condition) = json["condition"].as_str() {
                assert!(condition.contains("missing"));
            } else {
                return Err(Box::new(IsolateError::OperationFailed("condition field missing or not string".to_string())) as Box<dyn std::error::Error>);
            }
            assert_eq!(json["final_state"].as_str(), Some("not_found"));
            Ok(())
        }
    }
}
