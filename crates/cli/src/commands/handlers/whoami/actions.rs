//! Action functions for the whoami command handler (Tier 3).
//!
//! I/O operations that determine and display agent identity.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{WhoamiOptions, WhoamiOutput};

/// Execute the whoami command with the given options.
///
/// Reads environment variables to determine agent identity and context.
///
/// # Errors
///
/// Returns an error if serialization fails in JSON mode.
pub fn run_whoami(options: &WhoamiOptions) -> Result<()> {
    let output = build_identity();

    if options.json {
        let json_str = serde_json::to_string_pretty(&output)
            .map_err(|e| Error::io_error(format!("Failed to serialize whoami output: {e}")))?;
        Output::info(&json_str);
    } else {
        Output::info(&output.simple);
    }

    Ok(())
}

/// Build the identity output from environment variables.
///
/// Checks `SCP_AGENT_ID` (with `ISOLATE_AGENT_ID` fallback), `SCP_BEAD_ID`
/// (with `Isolate_BEAD_ID` fallback), and `SCP_WORKSPACE` / `SCP_SESSION`
/// for context.
pub fn build_identity() -> WhoamiOutput {
    let agent_id = std::env::var("SCP_AGENT_ID")
        .ok()
        .or_else(|| std::env::var("ISOLATE_AGENT_ID").ok());

    let bead_id = std::env::var("SCP_BEAD_ID")
        .ok()
        .or_else(|| std::env::var("Isolate_BEAD_ID").ok());

    let env_workspace = std::env::var("SCP_WORKSPACE")
        .ok()
        .or_else(|| std::env::var("Isolate_WORKSPACE").ok());

    let env_session = std::env::var("SCP_SESSION")
        .ok()
        .or_else(|| std::env::var("Isolate_SESSION").ok());

    let current_session = env_session.or_else(|| {
        env_workspace
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .map(String::from)
    });

    match agent_id {
        Some(id) => WhoamiOutput {
            registered: true,
            agent_id: Some(id.clone()),
            current_session,
            current_bead: bead_id,
            simple: id,
        },
        None => WhoamiOutput {
            registered: false,
            agent_id: None,
            current_session,
            current_bead: bead_id,
            simple: "unregistered".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_whoami_default() {
        let options = WhoamiOptions { json: false };
        assert!(run_whoami(&options).is_ok());
    }

    #[test]
    fn run_whoami_json() {
        let options = WhoamiOptions { json: true };
        assert!(run_whoami(&options).is_ok());
    }

    #[test]
    fn build_identity_with_agent_id() {
        std::env::set_var("SCP_AGENT_ID", "test-agent-123");
        std::env::remove_var("ISOLATE_AGENT_ID");
        let output = build_identity();
        std::env::remove_var("SCP_AGENT_ID");

        assert!(output.registered);
        assert_eq!(output.agent_id, Some("test-agent-123".to_string()));
        assert_eq!(output.simple, "test-agent-123");
    }

    #[test]
    fn build_identity_fallback_isolate_agent_id() {
        std::env::remove_var("SCP_AGENT_ID");
        std::env::set_var("ISOLATE_AGENT_ID", "fallback-agent");
        let output = build_identity();
        std::env::remove_var("ISOLATE_AGENT_ID");

        assert!(output.registered);
        assert_eq!(output.agent_id, Some("fallback-agent".to_string()));
    }

    #[test]
    fn build_identity_unregistered() {
        std::env::remove_var("SCP_AGENT_ID");
        std::env::remove_var("ISOLATE_AGENT_ID");
        let output = build_identity();

        assert!(!output.registered);
        assert!(output.agent_id.is_none());
        assert_eq!(output.simple, "unregistered");
    }

    #[test]
    fn build_identity_with_bead() {
        std::env::set_var("SCP_AGENT_ID", "agent-1");
        std::env::set_var("SCP_BEAD_ID", "bead-42");
        let output = build_identity();
        std::env::remove_var("SCP_AGENT_ID");
        std::env::remove_var("SCP_BEAD_ID");

        assert_eq!(output.current_bead, Some("bead-42".to_string()));
    }

    #[test]
    fn build_identity_session_from_env() {
        std::env::set_var("SCP_AGENT_ID", "agent-1");
        std::env::set_var("SCP_SESSION", "my-session");
        let output = build_identity();
        std::env::remove_var("SCP_AGENT_ID");
        std::env::remove_var("SCP_SESSION");

        assert_eq!(output.current_session, Some("my-session".to_string()));
    }

    #[test]
    fn build_identity_session_from_workspace_path() {
        std::env::set_var("SCP_AGENT_ID", "agent-1");
        std::env::remove_var("SCP_SESSION");
        std::env::set_var("SCP_WORKSPACE", "/home/user/worktrees/feature-auth");
        let output = build_identity();
        std::env::remove_var("SCP_AGENT_ID");
        std::env::remove_var("SCP_WORKSPACE");

        assert_eq!(output.current_session, Some("feature-auth".to_string()));
    }

    #[test]
    fn build_identity_no_session_when_nothing_set() {
        std::env::remove_var("SCP_AGENT_ID");
        std::env::remove_var("ISOLATE_AGENT_ID");
        std::env::remove_var("Isolate_AGENT_ID");
        std::env::remove_var("SCP_SESSION");
        std::env::remove_var("ISOLATE_SESSION");
        std::env::remove_var("Isolate_SESSION");
        std::env::remove_var("SCP_WORKSPACE");
        std::env::remove_var("ISOLATE_WORKSPACE");
        std::env::remove_var("Isolate_WORKSPACE");
        std::env::remove_var("SCP_BEAD_ID");
        let output = build_identity();

        assert!(output.current_session.is_none());
    }
}
