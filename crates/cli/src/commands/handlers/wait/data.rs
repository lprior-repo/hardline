//! Data types for the wait command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Options for the wait command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct WaitOptions {
    /// The condition to wait for.
    pub condition: WaitCondition,
    /// Maximum time to wait before giving up.
    pub timeout: Duration,
    /// Interval between condition checks.
    pub poll_interval: Duration,
}

/// Wait condition types.
#[derive(Debug, Clone)]
pub enum WaitCondition {
    /// Wait for session to exist.
    SessionExists(String),
    /// Wait for session to be unlocked (not in use by another agent).
    SessionUnlocked(String),
    /// Wait for system to be healthy.
    Healthy,
    /// Wait for session to reach a specific status.
    SessionStatus { name: String, status: String },
}

/// Output of a wait command invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitOutput {
    /// Whether the condition was met.
    pub condition_met: bool,
    /// Human-readable description of the condition that was waited for.
    pub condition: String,
    /// How long we waited, in milliseconds.
    pub elapsed_ms: u64,
    /// Whether we timed out before the condition was met.
    pub timed_out: bool,
    /// Current state when the condition was met or timed out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_state: Option<String>,
}

/// Parse a condition expression string into a `WaitCondition`.
///
/// Supported formats:
/// - `"healthy"` → `WaitCondition::Healthy`
/// - `"session-exists:<name>"` → `WaitCondition::SessionExists(name)`
/// - `"session-unlocked:<name>"` → `WaitCondition::SessionUnlocked(name)`
/// - `"session-status:<name>=<status>"` → `WaitCondition::SessionStatus { name, status }`
///
/// Returns `None` for unrecognized or malformed expressions.
///
/// This is a pure function (Tier 1 - no I/O).
pub fn parse_condition(input: &str) -> Option<WaitCondition> {
    if input.is_empty() {
        return None;
    }
    if input == "healthy" {
        return Some(WaitCondition::Healthy);
    }
    if let Some(name) = input.strip_prefix("session-exists:") {
        if name.is_empty() {
            return None;
        }
        return Some(WaitCondition::SessionExists(name.to_string()));
    }
    if let Some(name) = input.strip_prefix("session-unlocked:") {
        if name.is_empty() {
            return None;
        }
        return Some(WaitCondition::SessionUnlocked(name.to_string()));
    }
    if let Some(rest) = input.strip_prefix("session-status:") {
        if let Some((name, status)) = rest.split_once('=') {
            if name.is_empty() || status.is_empty() {
                return None;
            }
            return Some(WaitCondition::SessionStatus {
                name: name.to_string(),
                status: status.to_string(),
            });
        }
    }
    None
}

/// Format a wait condition into a human-readable string.
///
/// This is a pure function (Tier 1 - no I/O).
pub fn format_condition(condition: &WaitCondition) -> String {
    match condition {
        WaitCondition::SessionExists(name) => format!("session-exists:{name}"),
        WaitCondition::SessionUnlocked(name) => format!("session-unlocked:{name}"),
        WaitCondition::Healthy => "healthy".to_string(),
        WaitCondition::SessionStatus { name, status } => {
            format!("session-status:{name}={status}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // format_condition
    // ========================================================================

    #[test]
    fn format_condition_session_exists() {
        let cond = WaitCondition::SessionExists("my-feature".to_string());
        assert_eq!(format_condition(&cond), "session-exists:my-feature");
    }

    #[test]
    fn format_condition_session_unlocked() {
        let cond = WaitCondition::SessionUnlocked("locked-session".to_string());
        assert_eq!(format_condition(&cond), "session-unlocked:locked-session");
    }

    #[test]
    fn format_condition_healthy() {
        let cond = WaitCondition::Healthy;
        assert_eq!(format_condition(&cond), "healthy");
    }

    #[test]
    fn format_condition_session_status() {
        let cond = WaitCondition::SessionStatus {
            name: "build-task".to_string(),
            status: "completed".to_string(),
        };
        assert_eq!(format_condition(&cond), "session-status:build-task=completed");
    }

    // ========================================================================
    // WaitOutput serialization
    // ========================================================================

    #[test]
    fn wait_output_serializes_all_fields() {
        let output = WaitOutput {
            condition_met: true,
            condition: "healthy".to_string(),
            elapsed_ms: 100,
            timed_out: false,
            final_state: Some("git:ok,db:ok".to_string()),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"condition_met\":true"));
        assert!(json.contains("\"elapsed_ms\":100"));
        assert!(json.contains("\"final_state\":\"git:ok,db:ok\""));
    }

    #[test]
    fn wait_output_skips_none_final_state() {
        let output = WaitOutput {
            condition_met: true,
            condition: "healthy".to_string(),
            elapsed_ms: 50,
            timed_out: false,
            final_state: None,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(!json.contains("final_state"));
    }

    #[test]
    fn wait_output_deserialization_roundtrip() {
        let output = WaitOutput {
            condition_met: false,
            condition: "session-exists:test".to_string(),
            elapsed_ms: 30000,
            timed_out: true,
            final_state: Some("not_found".to_string()),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let parsed: WaitOutput = serde_json::from_str(&json).expect("deserialize");
        assert!(!parsed.condition_met);
        assert!(parsed.timed_out);
        assert_eq!(parsed.elapsed_ms, 30000);
        assert_eq!(parsed.final_state, Some("not_found".to_string()));
    }

    #[test]
    fn wait_output_timeout_fields() {
        let output = WaitOutput {
            condition_met: false,
            condition: "session-exists:missing-session".to_string(),
            elapsed_ms: 30000,
            timed_out: true,
            final_state: Some("not_found".to_string()),
        };
        assert!(!output.condition_met);
        assert!(output.timed_out);
    }

    #[test]
    fn wait_output_success_fields() {
        let output = WaitOutput {
            condition_met: true,
            condition: "session-exists:my-session".to_string(),
            elapsed_ms: 50,
            timed_out: false,
            final_state: Some("status:active".to_string()),
        };
        assert!(output.condition_met);
        assert!(!output.timed_out);
    }

    #[test]
    fn failure_without_timeout_is_distinct() {
        let output = WaitOutput {
            condition_met: false,
            condition: "healthy".to_string(),
            elapsed_ms: 100,
            timed_out: false,
            final_state: Some("git:missing".to_string()),
        };
        assert!(!output.condition_met);
        assert!(!output.timed_out);
    }

    // ========================================================================
    // WaitCondition variants
    // ========================================================================

    #[test]
    fn all_condition_types_format_uniquely() {
        let conditions: Vec<WaitCondition> = vec![
            WaitCondition::SessionExists("a".to_string()),
            WaitCondition::SessionUnlocked("b".to_string()),
            WaitCondition::Healthy,
            WaitCondition::SessionStatus {
                name: "c".to_string(),
                status: "active".to_string(),
            },
        ];
        let formatted: Vec<String> = conditions.iter().map(format_condition).collect();
        let mut unique = formatted.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), formatted.len());
    }

    #[test]
    fn session_name_preserved_in_session_exists() {
        let name = "feature-auth-oauth2-integration";
        let condition = WaitCondition::SessionExists(name.to_string());
        assert!(
            matches!(condition, WaitCondition::SessionExists(ref n) if n == name),
            "Session name should be preserved exactly"
        );
    }

    // ========================================================================
    // WaitOptions
    // ========================================================================

    #[test]
    fn wait_options_sensible_defaults() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(30),
            poll_interval: Duration::from_secs(1),
        };
        assert!(options.timeout.as_secs() >= 10, "timeout should be >= 10s");
        assert!(
            options.poll_interval.as_millis() >= 100,
            "poll interval should be >= 100ms"
        );
        assert!(
            options.poll_interval < options.timeout,
            "poll interval should be less than timeout"
        );
    }

    // ========================================================================
    // parse_condition — basic parsing
    // ========================================================================

    #[test]
    fn parse_healthy() {
        let cond = parse_condition("healthy").expect("should parse");
        assert!(matches!(cond, WaitCondition::Healthy));
    }

    #[test]
    fn parse_session_exists() {
        let cond = parse_condition("session-exists:my-session").expect("should parse");
        assert!(matches!(cond, WaitCondition::SessionExists(ref n) if n == "my-session"));
    }

    #[test]
    fn parse_session_unlocked() {
        let cond = parse_condition("session-unlocked:my-session").expect("should parse");
        assert!(matches!(cond, WaitCondition::SessionUnlocked(ref n) if n == "my-session"));
    }

    #[test]
    fn parse_session_status() {
        let cond = parse_condition("session-status:build=completed").expect("should parse");
        match cond {
            WaitCondition::SessionStatus { name, status } => {
                assert_eq!(name, "build");
                assert_eq!(status, "completed");
            }
            other => panic!("expected SessionStatus, got {other:?}"),
        }
    }

    // ========================================================================
    // parse_condition — rejection cases
    // ========================================================================

    #[test]
    fn parse_empty_returns_none() {
        assert!(parse_condition("").is_none());
    }

    #[test]
    fn parse_unknown_prefix_returns_none() {
        assert!(parse_condition("unknown-condition:foo").is_none());
    }

    #[test]
    fn parse_session_exists_empty_name_returns_none() {
        assert!(parse_condition("session-exists:").is_none());
    }

    #[test]
    fn parse_session_unlocked_empty_name_returns_none() {
        assert!(parse_condition("session-unlocked:").is_none());
    }

    #[test]
    fn parse_session_status_no_equals_returns_none() {
        assert!(parse_condition("session-status:build").is_none());
    }

    #[test]
    fn parse_session_status_empty_name_returns_none() {
        assert!(parse_condition("session-status:=active").is_none());
    }

    #[test]
    fn parse_session_status_empty_status_returns_none() {
        assert!(parse_condition("session-status:build=").is_none());
    }

    #[test]
    fn parse_garbage_returns_none() {
        let garbage = ["!!!", "SESSION-EXISTS:foo", "healthy!", "healthy:x"];
        for input in &garbage {
            assert!(parse_condition(input).is_none(), "should reject: {input:?}");
        }
    }

    // ========================================================================
    // parse_condition — roundtrip with format_condition
    // ========================================================================

    #[test]
    fn parse_format_roundtrip_healthy() {
        let original = WaitCondition::Healthy;
        let formatted = format_condition(&original);
        let parsed = parse_condition(&formatted).expect("should roundtrip");
        assert!(matches!(parsed, WaitCondition::Healthy));
    }

    #[test]
    fn parse_format_roundtrip_session_exists() {
        let original = WaitCondition::SessionExists("my-session".to_string());
        let formatted = format_condition(&original);
        let parsed = parse_condition(&formatted).expect("should roundtrip");
        assert!(matches!(parsed, WaitCondition::SessionExists(ref n) if n == "my-session"));
    }

    #[test]
    fn parse_format_roundtrip_session_unlocked() {
        let original = WaitCondition::SessionUnlocked("locked-sess".to_string());
        let formatted = format_condition(&original);
        let parsed = parse_condition(&formatted).expect("should roundtrip");
        assert!(matches!(parsed, WaitCondition::SessionUnlocked(ref n) if n == "locked-sess"));
    }

    #[test]
    fn parse_format_roundtrip_session_status() {
        let original = WaitCondition::SessionStatus {
            name: "task-1".to_string(),
            status: "done".to_string(),
        };
        let formatted = format_condition(&original);
        let parsed = parse_condition(&formatted).expect("should roundtrip");
        match parsed {
            WaitCondition::SessionStatus { name, status } => {
                assert_eq!(name, "task-1");
                assert_eq!(status, "done");
            }
            other => panic!("expected SessionStatus, got {other:?}"),
        }
    }

    // ========================================================================
    // parse_condition — special characters and edge cases
    // ========================================================================

    #[test]
    fn parse_session_name_with_dashes() {
        let cond = parse_condition("session-exists:my-feature-branch").expect("should parse");
        assert!(matches!(cond, WaitCondition::SessionExists(ref n) if n == "my-feature-branch"));
    }

    #[test]
    fn parse_session_name_with_dots() {
        let cond = parse_condition("session-exists:v2.1.0").expect("should parse");
        assert!(matches!(cond, WaitCondition::SessionExists(ref n) if n == "v2.1.0"));
    }

    #[test]
    fn parse_session_name_with_underscores() {
        let cond = parse_condition("session-exists:my_feature").expect("should parse");
        assert!(matches!(cond, WaitCondition::SessionExists(ref n) if n == "my_feature"));
    }

    #[test]
    fn parse_session_status_with_equals_in_status() {
        // status containing '=' — first split wins for name/status
        let cond = parse_condition("session-status:build=result=ok").expect("should parse");
        match cond {
            WaitCondition::SessionStatus { name, status } => {
                assert_eq!(name, "build");
                assert_eq!(status, "result=ok");
            }
            other => panic!("expected SessionStatus, got {other:?}"),
        }
    }

    #[test]
    fn parse_case_sensitive_healthy() {
        assert!(parse_condition("Healthy").is_none());
        assert!(parse_condition("HEALTHY").is_none());
    }

    #[test]
    fn parse_case_sensitive_session_prefixes() {
        assert!(parse_condition("Session-Exists:foo").is_none());
        assert!(parse_condition("SESSION-EXISTS:foo").is_none());
    }

    #[test]
    fn parse_whitespace_in_value() {
        // Leading/trailing whitespace in the name part is preserved
        let cond = parse_condition("session-exists: my-session ").expect("should parse");
        assert!(matches!(cond, WaitCondition::SessionExists(ref n) if n == " my-session "));
    }

    // ========================================================================
    // WaitOutput — timeout expired error contract
    // ========================================================================

    #[test]
    fn timeout_output_has_consistent_fields() {
        // When timed_out is true, condition_met must be false
        let output = WaitOutput {
            condition_met: false,
            condition: "session-exists:gone".to_string(),
            elapsed_ms: 60000,
            timed_out: true,
            final_state: Some("not_found:gone".to_string()),
        };
        assert!(output.timed_out);
        assert!(!output.condition_met);
        assert!(output.final_state.is_some());
        assert!(output.elapsed_ms > 0);
    }

    #[test]
    fn success_output_has_consistent_fields() {
        let output = WaitOutput {
            condition_met: true,
            condition: "healthy".to_string(),
            elapsed_ms: 5,
            timed_out: false,
            final_state: Some("git:ok".to_string()),
        };
        assert!(output.condition_met);
        assert!(!output.timed_out);
        assert!(output.final_state.is_some());
    }

    // ========================================================================
    // WaitOptions — poll interval vs timeout constraints
    // ========================================================================

    #[test]
    fn options_poll_equals_timeout_is_valid() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_secs(10),
            poll_interval: Duration::from_secs(10),
        };
        assert_eq!(options.timeout, options.poll_interval);
    }

    #[test]
    fn options_minimum_durations() {
        let options = WaitOptions {
            condition: WaitCondition::Healthy,
            timeout: Duration::from_nanos(1),
            poll_interval: Duration::from_nanos(1),
        };
        assert!(!options.timeout.is_zero());
        assert!(!options.poll_interval.is_zero());
    }

    // ========================================================================
    // WaitCondition — Debug trait coverage
    // ========================================================================

    #[test]
    fn all_conditions_have_debug_representation() {
        let conditions: Vec<WaitCondition> = vec![
            WaitCondition::SessionExists("a".to_string()),
            WaitCondition::SessionUnlocked("b".to_string()),
            WaitCondition::Healthy,
            WaitCondition::SessionStatus { name: "c".to_string(), status: "d".to_string() },
        ];
        for cond in &conditions {
            let debug = format!("{cond:?}");
            assert!(!debug.is_empty(), "Debug should not be empty for {cond:?}");
        }
    }

    // ========================================================================
    // WaitCondition — Clone coverage
    // ========================================================================

    #[test]
    fn all_conditions_are_clonable() {
        let conditions: Vec<WaitCondition> = vec![
            WaitCondition::SessionExists("clone-me".to_string()),
            WaitCondition::SessionUnlocked("locked".to_string()),
            WaitCondition::Healthy,
            WaitCondition::SessionStatus { name: "n".to_string(), status: "s".to_string() },
        ];
        for cond in &conditions {
            let cloned = cond.clone();
            assert_eq!(format!("{cond:?}"), format!("{cloned:?}"));
        }
    }

    // ========================================================================
    // parse_condition — adversarial inputs
    // ========================================================================

    #[test]
    fn parse_adversarial_injection_strings() {
        let adversarial = [
            "session-exists:; rm -rf /",
            "session-exists:'; DROP TABLE sessions",
            "session-exists:$(whoami)",
            "session-exists:`id`",
            "session-status:n=s; DROP TABLE",
            "session-unlocked:\x00null",
            "session-exists:\nnewline",
        ];
        for input in &adversarial {
            // Should parse (pure function doesn't execute shell commands)
            let result = parse_condition(input);
            assert!(result.is_some(), "should parse adversarial input: {input:?}");
        }
    }

    #[test]
    fn parse_adversarial_very_long_name() {
        let long_name = "x".repeat(65536);
        let input = format!("session-exists:{long_name}");
        let cond = parse_condition(&input).expect("should parse");
        assert!(matches!(cond, WaitCondition::SessionExists(ref n) if n.len() == 65536));
    }

    #[test]
    fn parse_adversarial_unicode() {
        let unicode_names = ["日本語", "🔑-session", "🦀", "session\u{202E}evil"];
        for name in &unicode_names {
            let input = format!("session-exists:{name}");
            let cond = parse_condition(&input).expect("should parse unicode");
            assert!(matches!(cond, WaitCondition::SessionExists(ref n) if n == name));
        }
    }
}
