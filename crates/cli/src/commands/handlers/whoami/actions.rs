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
/// (with `ISOLATE_BEAD_ID` fallback), and `SCP_WORKSPACE` / `SCP_SESSION`
/// for context.
pub fn build_identity() -> WhoamiOutput {
    let agent_id = std::env::var("SCP_AGENT_ID")
        .ok()
        .or_else(|| std::env::var("ISOLATE_AGENT_ID").ok());

    let bead_id = std::env::var("SCP_BEAD_ID")
        .ok()
        .or_else(|| std::env::var("ISOLATE_BEAD_ID").ok());

    let env_workspace = std::env::var("SCP_WORKSPACE")
        .ok()
        .or_else(|| std::env::var("ISOLATE_WORKSPACE").ok());

    let env_session = std::env::var("SCP_SESSION")
        .ok()
        .or_else(|| std::env::var("ISOLATE_SESSION").ok());

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

    // -----------------------------------------------------------------------
    // Subprocess-isolated env-var tests
    //
    // Each test that touches env vars spawns itself as a subprocess with
    // isolated environment.  The subprocess re-enters this test function,
    // detects its marker env var, runs the assertion, and exits.
    //
    // This avoids race conditions from parallel tests mutating global state.
    // -----------------------------------------------------------------------

    /// Run a closure in an isolated subprocess with the given env vars set.
    ///
    /// The subprocess re-executes the current test binary with a filter that
    /// matches only the calling test. The `marker` env var is checked on entry
    /// — when present, `body` runs and the process exits.  When absent
    /// (parent process), the binary is re-launched with the marker, the
    /// specified env overrides, and a narrow test filter.
    fn isolated(
        marker: &str,
        test_name: &str,
        env_set: &[(&str, &str)],
        env_remove: &[&str],
        body: impl FnOnce(),
    ) {
        if std::env::var(marker).is_ok() {
            body();
            std::process::exit(0);
        }

        let exe = std::env::current_exe().expect("current exe");
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("--exact");
        cmd.arg(test_name);
        cmd.arg("--test-threads=1");
        cmd.env(marker, "1");
        for (k, v) in env_set {
            cmd.env(k, v);
        }
        for k in env_remove {
            cmd.env_remove(k);
        }
        let status = cmd.status().expect("subprocess failed");
        assert!(status.success(), "isolated test subprocess failed");
    }

    // -- Tests that don't touch env vars (safe to run in-process) --

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

    // -- Subprocess-isolated env-var tests --

    #[test]
    fn build_identity_with_agent_id() {
        isolated(
            "__TEST_WHOMAI_AGENT_ID",
            "commands::handlers::whoami::actions::tests::build_identity_with_agent_id",
            &[("SCP_AGENT_ID", "test-agent-123")],
            &["ISOLATE_AGENT_ID"],
            || {
                let output = build_identity();
                assert!(output.registered);
                assert_eq!(output.agent_id, Some("test-agent-123".to_string()));
                assert_eq!(output.simple, "test-agent-123");
            },
        );
    }

    #[test]
    fn build_identity_fallback_isolate_agent_id() {
        isolated(
            "__TEST_WHOMAI_FALLBACK",
            "commands::handlers::whoami::actions::tests::build_identity_fallback_isolate_agent_id",
            &[("ISOLATE_AGENT_ID", "fallback-agent")],
            &["SCP_AGENT_ID"],
            || {
                let output = build_identity();
                assert!(output.registered);
                assert_eq!(output.agent_id, Some("fallback-agent".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_unregistered() {
        isolated(
            "__TEST_WHOMAI_UNREG",
            "commands::handlers::whoami::actions::tests::build_identity_unregistered",
            &[],
            &["SCP_AGENT_ID", "ISOLATE_AGENT_ID"],
            || {
                let output = build_identity();
                assert!(!output.registered);
                assert!(output.agent_id.is_none());
                assert_eq!(output.simple, "unregistered");
            },
        );
    }

    #[test]
    fn build_identity_with_bead() {
        isolated(
            "__TEST_WHOMAI_BEAD",
            "commands::handlers::whoami::actions::tests::build_identity_with_bead",
            &[("SCP_AGENT_ID", "agent-1"), ("SCP_BEAD_ID", "bead-42")],
            &[],
            || {
                let output = build_identity();
                assert_eq!(output.current_bead, Some("bead-42".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_session_from_env() {
        isolated(
            "__TEST_WHOMAI_SESSION_ENV",
            "commands::handlers::whoami::actions::tests::build_identity_session_from_env",
            &[("SCP_AGENT_ID", "agent-1"), ("SCP_SESSION", "my-session")],
            &["SCP_WORKSPACE"],
            || {
                let output = build_identity();
                assert_eq!(output.current_session, Some("my-session".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_session_from_workspace_path() {
        isolated(
            "__TEST_WHOMAI_SESSION_WS",
            "commands::handlers::whoami::actions::tests::build_identity_session_from_workspace_path",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("SCP_WORKSPACE", "/home/user/worktrees/feature-auth"),
            ],
            &["SCP_SESSION"],
            || {
                let output = build_identity();
                assert_eq!(
                    output.current_session,
                    Some("feature-auth".to_string())
                );
            },
        );
    }

    #[test]
    fn build_identity_no_session_when_nothing_set() {
        isolated(
            "__TEST_WHOMAI_NO_SESSION",
            "commands::handlers::whoami::actions::tests::build_identity_no_session_when_nothing_set",
            &[],
            &[
                "SCP_AGENT_ID",
                "ISOLATE_AGENT_ID",
                "SCP_SESSION",
                "ISOLATE_SESSION",
                "SCP_WORKSPACE",
                "ISOLATE_WORKSPACE",
                "SCP_BEAD_ID",
            ],
            || {
                let output = build_identity();
                assert!(output.current_session.is_none());
            },
        );
    }
}
