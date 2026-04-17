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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
