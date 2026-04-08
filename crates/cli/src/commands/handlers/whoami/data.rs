//! Data types for the whoami command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

/// Options for the whoami command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct WhoamiOptions {
    /// Output as JSON instead of simple text.
    pub json: bool,
}

/// Output for the whoami command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhoamiOutput {
    /// Whether an agent is registered.
    pub registered: bool,
    /// Agent ID if registered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Current session being worked on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_session: Option<String>,
    /// Current bead being worked on (from env var).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_bead: Option<String>,
    /// Simple one-line representation.
    pub simple: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whoami_output_unregistered() {
        let output = WhoamiOutput {
            registered: false,
            agent_id: None,
            current_session: None,
            current_bead: None,
            simple: "unregistered".to_string(),
        };

        assert!(!output.registered);
        assert_eq!(output.simple, "unregistered");
        assert!(output.agent_id.is_none());
    }

    #[test]
    fn whoami_output_registered() {
        let output = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-12345".to_string()),
            current_session: Some("feature-auth".to_string()),
            current_bead: Some("scp-abc12".to_string()),
            simple: "agent-12345".to_string(),
        };

        assert!(output.registered);
        assert_eq!(output.simple, "agent-12345");
        assert_eq!(output.agent_id, Some("agent-12345".to_string()));
    }

    #[test]
    fn whoami_output_serialization_roundtrip() {
        let output = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-12345".to_string()),
            current_session: None,
            current_bead: None,
            simple: "agent-12345".to_string(),
        };

        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: WhoamiOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.registered, true);
        assert_eq!(deserialized.agent_id, Some("agent-12345".to_string()));
    }

    #[test]
    fn whoami_output_skips_none_fields() {
        let output = WhoamiOutput {
            registered: false,
            agent_id: None,
            current_session: None,
            current_bead: None,
            simple: "unregistered".to_string(),
        };

        let json = serde_json::to_string(&output).expect("serialize");
        assert!(!json.contains("agent_id"));
        assert!(!json.contains("current_session"));
        assert!(!json.contains("current_bead"));
    }

    #[test]
    fn whoami_output_json_all_fields_present() {
        let output = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-1".to_string()),
            current_session: Some("session-1".to_string()),
            current_bead: Some("bead-1".to_string()),
            simple: "agent-1".to_string(),
        };

        let json_str = serde_json::to_string(&output).unwrap_or_default();

        assert!(json_str.contains("registered"));
        assert!(json_str.contains("agent_id"));
        assert!(json_str.contains("current_session"));
        assert!(json_str.contains("current_bead"));
        assert!(json_str.contains("simple"));
    }

    #[test]
    fn whoami_output_deterministic() {
        let make_output = || WhoamiOutput {
            registered: true,
            agent_id: Some("agent-1".to_string()),
            current_session: Some("session-1".to_string()),
            current_bead: None,
            simple: "agent-1".to_string(),
        };

        let json1 = serde_json::to_string(&make_output()).unwrap_or_default();
        let json2 = serde_json::to_string(&make_output()).unwrap_or_default();

        assert_eq!(json1, json2);
    }

    #[test]
    fn whoami_options_default() {
        let options = WhoamiOptions { json: false };
        assert!(!options.json);
    }

    #[test]
    fn whoami_registered_consistency() {
        let registered = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-1".to_string()),
            current_session: None,
            current_bead: None,
            simple: "agent-1".to_string(),
        };
        assert!(registered.agent_id.is_some());

        let unregistered = WhoamiOutput {
            registered: false,
            agent_id: None,
            current_session: None,
            current_bead: None,
            simple: "unregistered".to_string(),
        };
        assert!(unregistered.agent_id.is_none());
    }

    // -----------------------------------------------------------------------
    // Deserialization tests
    // -----------------------------------------------------------------------

    #[test]
    fn whoami_output_deserialization_full() {
        let json = r#"{
            "registered": true,
            "agent_id": "agent-xyz",
            "current_session": "sess-1",
            "current_bead": "bead-1",
            "simple": "agent-xyz"
        }"#;
        let output: WhoamiOutput = serde_json::from_str(json).expect("deserialize");
        assert!(output.registered);
        assert_eq!(output.agent_id, Some("agent-xyz".to_string()));
        assert_eq!(output.current_session, Some("sess-1".to_string()));
        assert_eq!(output.current_bead, Some("bead-1".to_string()));
        assert_eq!(output.simple, "agent-xyz");
    }

    #[test]
    fn whoami_output_deserialization_minimal() {
        let json = r#"{"registered": false, "simple": "unregistered"}"#;
        let output: WhoamiOutput = serde_json::from_str(json).expect("deserialize minimal");
        assert!(!output.registered);
        assert!(output.agent_id.is_none());
        assert!(output.current_session.is_none());
        assert!(output.current_bead.is_none());
        assert_eq!(output.simple, "unregistered");
    }

    #[test]
    fn whoami_output_deserialization_partial_fields() {
        let json = r#"{"registered": true, "agent_id": "a1", "simple": "a1"}"#;
        let output: WhoamiOutput = serde_json::from_str(json).expect("deserialize partial");
        assert!(output.registered);
        assert_eq!(output.agent_id, Some("a1".to_string()));
        assert!(output.current_session.is_none());
        assert!(output.current_bead.is_none());
    }

    #[test]
    fn whoami_output_deserialization_missing_required_field_fails() {
        // "registered" and "simple" are required fields
        let json = r#"{"agent_id": "a1"}"#;
        assert!(serde_json::from_str::<WhoamiOutput>(json).is_err());
    }

    // -----------------------------------------------------------------------
    // Clone / Debug / Equality
    // -----------------------------------------------------------------------

    #[test]
    fn whoami_output_clone_preserves_all_fields() {
        let original = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-clone".to_string()),
            current_session: Some("session-clone".to_string()),
            current_bead: Some("bead-clone".to_string()),
            simple: "agent-clone".to_string(),
        };
        let cloned = original.clone();
        assert_eq!(cloned.registered, original.registered);
        assert_eq!(cloned.agent_id, original.agent_id);
        assert_eq!(cloned.current_session, original.current_session);
        assert_eq!(cloned.current_bead, original.current_bead);
        assert_eq!(cloned.simple, original.simple);
    }

    #[test]
    fn whoami_output_debug_format_contains_fields() {
        let output = WhoamiOutput {
            registered: true,
            agent_id: Some("debug-agent".to_string()),
            current_session: None,
            current_bead: None,
            simple: "debug-agent".to_string(),
        };
        let debug = format!("{:?}", output);
        assert!(debug.contains("registered"));
        assert!(debug.contains("debug-agent"));
    }

    #[test]
    fn whoami_options_debug_format() {
        let opts = WhoamiOptions { json: true };
        let debug = format!("{:?}", opts);
        assert!(debug.contains("json"));
        assert!(debug.contains("true"));
    }

    #[test]
    fn whoami_output_eq_same_values() {
        let a = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-1".to_string()),
            current_session: Some("s1".to_string()),
            current_bead: Some("b1".to_string()),
            simple: "agent-1".to_string(),
        };
        let b = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-1".to_string()),
            current_session: Some("s1".to_string()),
            current_bead: Some("b1".to_string()),
            simple: "agent-1".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn whoami_output_neq_different_simple() {
        let a = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-1".to_string()),
            current_session: None,
            current_bead: None,
            simple: "agent-1".to_string(),
        };
        let b = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-2".to_string()),
            current_session: None,
            current_bead: None,
            simple: "agent-2".to_string(),
        };
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // Edge cases: special characters, empty strings
    // -----------------------------------------------------------------------

    #[test]
    fn whoami_output_special_chars_serialization() {
        let output = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-日本語".to_string()),
            current_session: Some("session-émoji-🤖".to_string()),
            current_bead: Some("bead-<tag>".to_string()),
            simple: "agent-日本語".to_string(),
        };
        let json = serde_json::to_string(&output).expect("serialize special chars");
        let back: WhoamiOutput = serde_json::from_str(&json).expect("deserialize special chars");
        assert_eq!(back.agent_id, Some("agent-日本語".to_string()));
        assert_eq!(back.current_session, Some("session-émoji-🤖".to_string()));
        assert_eq!(back.current_bead, Some("bead-<tag>".to_string()));
    }

    #[test]
    fn whoami_output_empty_strings() {
        let output = WhoamiOutput {
            registered: true,
            agent_id: Some(String::new()),
            current_session: Some(String::new()),
            current_bead: Some(String::new()),
            simple: String::new(),
        };
        let json = serde_json::to_string(&output).expect("serialize empty");
        let back: WhoamiOutput = serde_json::from_str(&json).expect("deserialize empty");
        assert_eq!(back.agent_id, Some(String::new()));
        assert_eq!(back.simple, "");
    }

    #[test]
    fn whoami_output_long_strings() {
        let long = "x".repeat(10_000);
        let output = WhoamiOutput {
            registered: true,
            agent_id: Some(long.clone()),
            current_session: None,
            current_bead: None,
            simple: long.clone(),
        };
        let json = serde_json::to_string(&output).expect("serialize long");
        let back: WhoamiOutput = serde_json::from_str(&json).expect("deserialize long");
        assert_eq!(back.agent_id, Some(long));
    }

    // -----------------------------------------------------------------------
    // JSON structure validation
    // -----------------------------------------------------------------------

    #[test]
    fn whoami_output_json_field_types() {
        let output = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-1".to_string()),
            current_session: Some("sess-1".to_string()),
            current_bead: Some("bead-1".to_string()),
            simple: "agent-1".to_string(),
        };
        let val: serde_json::Value =
            serde_json::to_value(&output).expect("to value");
        assert!(val["registered"].is_boolean());
        assert!(val["agent_id"].is_string());
        assert!(val["current_session"].is_string());
        assert!(val["current_bead"].is_string());
        assert!(val["simple"].is_string());
    }

    #[test]
    fn whoami_output_json_unregistered_no_optional_fields() {
        let output = WhoamiOutput {
            registered: false,
            agent_id: None,
            current_session: None,
            current_bead: None,
            simple: "unregistered".to_string(),
        };
        let val: serde_json::Value =
            serde_json::to_value(&output).expect("to value");
        // skip_serializing_if means None fields are absent
        assert!(val.get("agent_id").is_none());
        assert!(val.get("current_session").is_none());
        assert!(val.get("current_bead").is_none());
        // required fields always present
        assert!(val.get("registered").is_some());
        assert!(val.get("simple").is_some());
    }

    #[test]
    fn whoami_output_json_registered_has_all_fields() {
        let output = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-1".to_string()),
            current_session: Some("sess-1".to_string()),
            current_bead: Some("bead-1".to_string()),
            simple: "agent-1".to_string(),
        };
        let val: serde_json::Value =
            serde_json::to_value(&output).expect("to value");
        assert_eq!(val["registered"], true);
        assert_eq!(val["agent_id"], "agent-1");
        assert_eq!(val["current_session"], "sess-1");
        assert_eq!(val["current_bead"], "bead-1");
        assert_eq!(val["simple"], "agent-1");
    }

    #[test]
    fn whoami_output_pretty_json_is_valid() {
        let output = WhoamiOutput {
            registered: true,
            agent_id: Some("agent-pretty".to_string()),
            current_session: None,
            current_bead: None,
            simple: "agent-pretty".to_string(),
        };
        let pretty = serde_json::to_string_pretty(&output).expect("pretty serialize");
        // Pretty JSON should be valid and parseable
        let back: WhoamiOutput = serde_json::from_str(&pretty).expect("parse pretty");
        assert_eq!(back, output);
    }
}
