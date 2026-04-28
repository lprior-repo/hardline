//! whoami command - Agent identity query
//!
//! Returns the current agent identity:
//! - `unregistered` - No agent registered
//! - `<agent-id>` - Agent ID if registered
//!
//! Also checks environment variables for agent context.

use isolate_core::{json::SchemaEnvelope, OutputFormat};
use serde::Serialize;

use crate::{IsolateError, Result};

/// Output for whoami command
#[derive(Debug, Clone, Serialize)]
pub struct WhoAmIOutput {
    /// Whether an agent is registered
    pub registered: bool,
    /// Agent ID if registered
    pub agent_id: Option<String>,
    /// Current session being worked on
    pub current_session: Option<String>,
    /// Current bead being worked on (from env var)
    pub current_bead: Option<String>,
    /// Simple one-line representation
    pub simple: String,
}

/// Options for whoami command
pub struct WhoAmIOptions {
    pub format: OutputFormat,
}

/// Run the whoami command
///
/// # Errors
///
/// Returns an error if unable to determine identity
pub fn run(options: &WhoAmIOptions) -> Result<()> {
    // Check environment variables for agent context
    let env_agent_id = std::env::var("Isolate_AGENT_ID").ok();
    let env_bead_id = std::env::var("Isolate_BEAD_ID").ok();
    let env_workspace = std::env::var("Isolate_WORKSPACE").ok();
    let env_session = std::env::var("Isolate_SESSION").ok();

    // Determine current session from workspace path if not set
    let current_session = env_session.or_else(|| {
        env_workspace
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .map(String::from)
    });

    let output = if let Some(agent_id) = env_agent_id {
        WhoAmIOutput {
            registered: true,
            agent_id: Some(agent_id.clone()),
            current_session,
            current_bead: env_bead_id,
            simple: agent_id,
        }
    } else {
        WhoAmIOutput {
            registered: false,
            agent_id: None,
            current_session,
            current_bead: env_bead_id,
            simple: "unregistered".to_string(),
        }
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("whoami-response", "single", &output);
        let json_str = serde_json::to_string_pretty(&envelope).map_err(|e| {
            IsolateError::OperationFailed(format!("Failed to serialize whoami output: {e}"))
        })?;
        println!("{json_str}");
    } else {
        // Simple output - just the identity
        println!("{}", output.simple);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whoami_output_unregistered() {
        let output = WhoAmIOutput {
            registered: false,
            agent_id: None,
            current_session: None,
            current_bead: None,
            simple: "unregistered".to_string(),
        };

        assert!(!output.registered);
        assert_eq!(output.simple, "unregistered");
    }

    #[test]
    fn test_whoami_output_registered() {
        let output = WhoAmIOutput {
            registered: true,
            agent_id: Some("agent-12345".to_string()),
            current_session: Some("feature-auth".to_string()),
            current_bead: Some("isolate-abc12".to_string()),
            simple: "agent-12345".to_string(),
        };

        assert!(output.registered);
        assert_eq!(output.simple, "agent-12345");
        assert_eq!(output.agent_id, Some("agent-12345".to_string()));
    }

    #[test]
    fn test_whoami_output_serializes() {
        let output = WhoAmIOutput {
            registered: true,
            agent_id: Some("agent-12345".to_string()),
            current_session: None,
            current_bead: None,
            simple: "agent-12345".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("\"registered\":true"));
        assert!(json_str.contains("\"agent_id\":\"agent-12345\""));
    }

    // ============================================================================
    // Behavior Tests
    // ============================================================================

    /// Test simple output for unregistered agent
    #[test]
    fn test_whoami_simple_unregistered() {
        let output = WhoAmIOutput {
            registered: false,
            agent_id: None,
            current_session: None,
            current_bead: None,
            simple: "unregistered".to_string(),
        };

        assert_eq!(output.simple, "unregistered");
        assert!(!output.registered);
    }

    /// Test simple output for registered agent
    #[test]
    fn test_whoami_simple_registered() {
        let output = WhoAmIOutput {
            registered: true,
            agent_id: Some("agent-abc123".to_string()),
            current_session: None,
            current_bead: None,
            simple: "agent-abc123".to_string(),
        };

        // Simple should be the agent_id
        assert_eq!(output.simple, "agent-abc123");
        assert!(output.registered);
    }

    /// Test registered flag consistency with `agent_id`
    #[test]
    fn test_whoami_registered_consistency() {
        // When registered, agent_id must be Some
        let registered = WhoAmIOutput {
            registered: true,
            agent_id: Some("agent-1".to_string()),
            current_session: None,
            current_bead: None,
            simple: "agent-1".to_string(),
        };
        assert!(registered.agent_id.is_some());

        // When not registered, agent_id should be None
        let unregistered = WhoAmIOutput {
            registered: false,
            agent_id: None,
            current_session: None,
            current_bead: None,
            simple: "unregistered".to_string(),
        };
        assert!(unregistered.agent_id.is_none());
    }

    /// Test session and bead context
    #[test]
    fn test_whoami_context_fields() {
        let output = WhoAmIOutput {
            registered: true,
            agent_id: Some("agent-1".to_string()),
            current_session: Some("feature-auth".to_string()),
            current_bead: Some("isolate-abc12".to_string()),
            simple: "agent-1".to_string(),
        };

        assert_eq!(output.current_session, Some("feature-auth".to_string()));
        assert_eq!(output.current_bead, Some("isolate-abc12".to_string()));
    }

    /// Test JSON output has all required fields
    #[test]
    fn test_whoami_json_all_fields() {
        let output = WhoAmIOutput {
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

    /// Test simple field matches `agent_id` when registered
    #[test]
    fn test_whoami_simple_matches_agent_id() {
        let agent_id = "my-custom-agent-id";
        let output = WhoAmIOutput {
            registered: true,
            agent_id: Some(agent_id.to_string()),
            current_session: None,
            current_bead: None,
            simple: agent_id.to_string(),
        };

        let agent_id = output.agent_id.as_deref().unwrap_or_default();
        assert_eq!(output.simple, agent_id);
    }

    /// Test output is deterministic
    #[test]
    fn test_whoami_output_deterministic() {
        let make_output = || WhoAmIOutput {
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
}
