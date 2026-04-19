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
        assert_eq!(
            format_condition(&cond),
            "session-status:build-task=completed"
        );
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
}
