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
    fn isolated(marker: &str, test_name: &str, env_set: &[(&str, &str)], env_remove: &[&str], body: impl FnOnce()) {
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

    // -----------------------------------------------------------------------
    // Priority tests: SCP_ env vars take precedence over ISOLATE_ fallbacks
    // -----------------------------------------------------------------------

    #[test]
    fn build_identity_scp_agent_priority_over_isolate() {
        isolated(
            "__TEST_WHOMAI_AGENT_PRIORITY",
            "commands::handlers::whoami::actions::tests::build_identity_scp_agent_priority_over_isolate",
            &[("SCP_AGENT_ID", "scp-agent"), ("ISOLATE_AGENT_ID", "isolate-agent")],
            &[],
            || {
                let output = build_identity();
                assert_eq!(output.agent_id, Some("scp-agent".to_string()));
                assert_eq!(output.simple, "scp-agent");
            },
        );
    }

    #[test]
    fn build_identity_bead_fallback_isolate() {
        isolated(
            "__TEST_WHOMAI_BEAD_FALLBACK",
            "commands::handlers::whoami::actions::tests::build_identity_bead_fallback_isolate",
            &[("SCP_AGENT_ID", "agent-1"), ("ISOLATE_BEAD_ID", "isolate-bead-99")],
            &["SCP_BEAD_ID"],
            || {
                let output = build_identity();
                assert_eq!(output.current_bead, Some("isolate-bead-99".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_bead_priority_scp_over_isolate() {
        isolated(
            "__TEST_WHOMAI_BEAD_PRIORITY",
            "commands::handlers::whoami::actions::tests::build_identity_bead_priority_scp_over_isolate",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("SCP_BEAD_ID", "scp-bead"),
                ("ISOLATE_BEAD_ID", "isolate-bead"),
            ],
            &[],
            || {
                let output = build_identity();
                assert_eq!(output.current_bead, Some("scp-bead".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_workspace_fallback_isolate_for_session() {
        isolated(
            "__TEST_WHOMAI_WS_FALLBACK",
            "commands::handlers::whoami::actions::tests::build_identity_workspace_fallback_isolate_for_session",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("ISOLATE_WORKSPACE", "/home/user/worktrees/my-feature"),
            ],
            &["SCP_WORKSPACE", "SCP_SESSION", "ISOLATE_SESSION"],
            || {
                let output = build_identity();
                assert_eq!(output.current_session, Some("my-feature".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_workspace_priority_scp_over_isolate() {
        isolated(
            "__TEST_WHOMAI_WS_PRIORITY",
            "commands::handlers::whoami::actions::tests::build_identity_workspace_priority_scp_over_isolate",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("SCP_WORKSPACE", "/home/user/worktrees/scp-feature"),
                ("ISOLATE_WORKSPACE", "/home/user/worktrees/isolate-feature"),
            ],
            &["SCP_SESSION", "ISOLATE_SESSION"],
            || {
                let output = build_identity();
                assert_eq!(output.current_session, Some("scp-feature".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_session_fallback_isolate_session() {
        isolated(
            "__TEST_WHOMAI_SESSION_FALLBACK",
            "commands::handlers::whoami::actions::tests::build_identity_session_fallback_isolate_session",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("ISOLATE_SESSION", "isolate-session-42"),
            ],
            &["SCP_SESSION", "SCP_WORKSPACE", "ISOLATE_WORKSPACE"],
            || {
                let output = build_identity();
                assert_eq!(output.current_session, Some("isolate-session-42".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_session_priority_scp_over_isolate() {
        isolated(
            "__TEST_WHOMAI_SESSION_PRIORITY",
            "commands::handlers::whoami::actions::tests::build_identity_session_priority_scp_over_isolate",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("SCP_SESSION", "scp-session"),
                ("ISOLATE_SESSION", "isolate-session"),
            ],
            &["SCP_WORKSPACE", "ISOLATE_WORKSPACE"],
            || {
                let output = build_identity();
                assert_eq!(output.current_session, Some("scp-session".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_session_over_workspace_basename() {
        // SCP_SESSION should take priority over workspace path derivation
        isolated(
            "__TEST_WHOMAI_SESSION_OVER_WS",
            "commands::handlers::whoami::actions::tests::build_identity_session_over_workspace_basename",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("SCP_SESSION", "explicit-session"),
                ("SCP_WORKSPACE", "/home/user/worktrees/workspace-session"),
            ],
            &[],
            || {
                let output = build_identity();
                assert_eq!(output.current_session, Some("explicit-session".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_isolate_session_over_workspace_basename() {
        // ISOLATE_SESSION should take priority over workspace path derivation
        isolated(
            "__TEST_WHOMAI_ISO_SESSION_OVER_WS",
            "commands::handlers::whoami::actions::tests::build_identity_isolate_session_over_workspace_basename",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("ISOLATE_SESSION", "isolate-explicit"),
                ("ISOLATE_WORKSPACE", "/home/user/worktrees/ws-session"),
            ],
            &["SCP_SESSION", "SCP_WORKSPACE"],
            || {
                let output = build_identity();
                assert_eq!(output.current_session, Some("isolate-explicit".to_string()));
            },
        );
    }

    // -----------------------------------------------------------------------
    // Edge cases: path handling
    // -----------------------------------------------------------------------

    #[test]
    fn build_identity_workspace_trailing_slash() {
        isolated(
            "__TEST_WHOMAI_WS_TRAILING",
            "commands::handlers::whoami::actions::tests::build_identity_workspace_trailing_slash",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("SCP_WORKSPACE", "/home/user/worktrees/feature-x/"),
            ],
            &["SCP_SESSION", "ISOLATE_SESSION"],
            || {
                let output = build_identity();
                // Trailing slash: file_name of "feature-x/" should still be "feature-x"
                // (Rust's Path::file_name strips trailing slashes)
                assert!(
                    output.current_session.is_some(),
                    "session should be derived from workspace path even with trailing slash"
                );
            },
        );
    }

    #[test]
    fn build_identity_workspace_root_path() {
        isolated(
            "__TEST_WHOMAI_WS_ROOT",
            "commands::handlers::whoami::actions::tests::build_identity_workspace_root_path",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("SCP_WORKSPACE", "/"),
            ],
            &["SCP_SESSION", "ISOLATE_SESSION"],
            || {
                let output = build_identity();
                // Root path "/" has no file_name, so session should be None
                assert!(output.current_session.is_none());
            },
        );
    }

    #[test]
    fn build_identity_workspace_single_component() {
        isolated(
            "__TEST_WHOMAI_WS_SINGLE",
            "commands::handlers::whoami::actions::tests::build_identity_workspace_single_component",
            &[
                ("SCP_AGENT_ID", "agent-1"),
                ("SCP_WORKSPACE", "my-session-dir"),
            ],
            &["SCP_SESSION", "ISOLATE_SESSION"],
            || {
                let output = build_identity();
                assert_eq!(output.current_session, Some("my-session-dir".to_string()));
            },
        );
    }

    // -----------------------------------------------------------------------
    // Edge cases: empty env vars, unicode
    // -----------------------------------------------------------------------

    #[test]
    fn build_identity_empty_scp_agent_id_is_registered() {
        // std::env::var("X") returns Ok("") for empty env var, so it's Some("")
        isolated(
            "__TEST_WHOMAI_EMPTY_AGENT",
            "commands::handlers::whoami::actions::tests::build_identity_empty_scp_agent_id_is_registered",
            &[("SCP_AGENT_ID", "")],
            &["ISOLATE_AGENT_ID"],
            || {
                let output = build_identity();
                // Empty string is still Some -> registered
                assert!(output.registered);
                assert_eq!(output.agent_id, Some(String::new()));
                assert_eq!(output.simple, "");
            },
        );
    }

    #[test]
    fn build_identity_unicode_agent_id() {
        isolated(
            "__TEST_WHOMAI_UNICODE",
            "commands::handlers::whoami::actions::tests::build_identity_unicode_agent_id",
            &[("SCP_AGENT_ID", "ポールキャット-ジャスパー")],
            &["ISOLATE_AGENT_ID"],
            || {
                let output = build_identity();
                assert!(output.registered);
                assert_eq!(output.simple, "ポールキャット-ジャスパー");
                assert_eq!(output.agent_id, Some("ポールキャット-ジャスパー".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_special_chars_agent_id() {
        isolated(
            "__TEST_WHOMAI_SPECIAL",
            "commands::handlers::whoami::actions::tests::build_identity_special_chars_agent_id",
            &[("SCP_AGENT_ID", "agent/foo:bar\\baz")],
            &["ISOLATE_AGENT_ID"],
            || {
                let output = build_identity();
                assert_eq!(output.simple, "agent/foo:bar\\baz");
            },
        );
    }

    // -----------------------------------------------------------------------
    // Combined field tests
    // -----------------------------------------------------------------------

    #[test]
    fn build_identity_bead_without_agent_unregistered() {
        isolated(
            "__TEST_WHOMAI_BEAD_NO_AGENT",
            "commands::handlers::whoami::actions::tests::build_identity_bead_without_agent_unregistered",
            &[("SCP_BEAD_ID", "orphan-bead-42")],
            &["SCP_AGENT_ID", "ISOLATE_AGENT_ID"],
            || {
                let output = build_identity();
                assert!(!output.registered);
                assert!(output.agent_id.is_none());
                assert_eq!(output.simple, "unregistered");
                assert_eq!(output.current_bead, Some("orphan-bead-42".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_all_env_vars_set() {
        isolated(
            "__TEST_WHOMAI_ALL_ENV",
            "commands::handlers::whoami::actions::tests::build_identity_all_env_vars_set",
            &[
                ("SCP_AGENT_ID", "full-agent"),
                ("SCP_BEAD_ID", "full-bead"),
                ("SCP_SESSION", "full-session"),
                ("SCP_WORKSPACE", "/home/user/workspaces/full-ws"),
            ],
            &[],
            || {
                let output = build_identity();
                assert!(output.registered);
                assert_eq!(output.agent_id, Some("full-agent".to_string()));
                assert_eq!(output.simple, "full-agent");
                assert_eq!(output.current_bead, Some("full-bead".to_string()));
                // SCP_SESSION takes priority over workspace basename
                assert_eq!(output.current_session, Some("full-session".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_all_isolate_env_vars() {
        isolated(
            "__TEST_WHOMAI_ALL_ISOLATE",
            "commands::handlers::whoami::actions::tests::build_identity_all_isolate_env_vars",
            &[
                ("ISOLATE_AGENT_ID", "iso-agent"),
                ("ISOLATE_BEAD_ID", "iso-bead"),
                ("ISOLATE_SESSION", "iso-session"),
                ("ISOLATE_WORKSPACE", "/home/user/workspaces/iso-ws"),
            ],
            &["SCP_AGENT_ID", "SCP_BEAD_ID", "SCP_SESSION", "SCP_WORKSPACE"],
            || {
                let output = build_identity();
                assert!(output.registered);
                assert_eq!(output.agent_id, Some("iso-agent".to_string()));
                assert_eq!(output.simple, "iso-agent");
                assert_eq!(output.current_bead, Some("iso-bead".to_string()));
                assert_eq!(output.current_session, Some("iso-session".to_string()));
            },
        );
    }

    #[test]
    fn build_identity_mixed_scp_and_isolate() {
        // SCP_AGENT_ID present but bead/session from ISOLATE_
        isolated(
            "__TEST_WHOMAI_MIXED",
            "commands::handlers::whoami::actions::tests::build_identity_mixed_scp_and_isolate",
            &[
                ("SCP_AGENT_ID", "scp-agent"),
                ("ISOLATE_BEAD_ID", "iso-bead"),
                ("ISOLATE_SESSION", "iso-session"),
            ],
            &["SCP_BEAD_ID", "SCP_SESSION", "SCP_WORKSPACE", "ISOLATE_WORKSPACE", "ISOLATE_AGENT_ID"],
            || {
                let output = build_identity();
                assert_eq!(output.agent_id, Some("scp-agent".to_string()));
                assert_eq!(output.current_bead, Some("iso-bead".to_string()));
                assert_eq!(output.current_session, Some("iso-session".to_string()));
            },
        );
    }

    // -----------------------------------------------------------------------
    // run_whoami execution tests (subprocess isolated)
    // -----------------------------------------------------------------------

    #[test]
    fn run_whoami_text_mode_registered() {
        isolated(
            "__TEST_WHOMAI_RUN_TEXT",
            "commands::handlers::whoami::actions::tests::run_whoami_text_mode_registered",
            &[("SCP_AGENT_ID", "text-agent")],
            &["ISOLATE_AGENT_ID"],
            || {
                let options = WhoamiOptions { json: false };
                assert!(run_whoami(&options).is_ok());
            },
        );
    }

    #[test]
    fn run_whoami_json_mode_registered() {
        isolated(
            "__TEST_WHOMAI_RUN_JSON",
            "commands::handlers::whoami::actions::tests::run_whoami_json_mode_registered",
            &[("SCP_AGENT_ID", "json-agent")],
            &["ISOLATE_AGENT_ID"],
            || {
                let options = WhoamiOptions { json: true };
                assert!(run_whoami(&options).is_ok());
            },
        );
    }

    #[test]
    fn run_whoami_json_unregistered() {
        isolated(
            "__TEST_WHOMAI_RUN_JSON_UNREG",
            "commands::handlers::whoami::actions::tests::run_whoami_json_unregistered",
            &[],
            &["SCP_AGENT_ID", "ISOLATE_AGENT_ID"],
            || {
                let options = WhoamiOptions { json: true };
                assert!(run_whoami(&options).is_ok());
            },
        );
    }

    // -----------------------------------------------------------------------
    // build_identity pure output structure tests (subprocess isolated)
    // -----------------------------------------------------------------------

    #[test]
    fn build_identity_output_has_consistent_fields_registered() {
        isolated(
            "__TEST_WHOMAI_STRUCT_REG",
            "commands::handlers::whoami::actions::tests::build_identity_output_has_consistent_fields_registered",
            &[("SCP_AGENT_ID", "struct-agent"), ("SCP_BEAD_ID", "struct-bead"), ("SCP_SESSION", "struct-session")],
            &["ISOLATE_AGENT_ID"],
            || {
                let output = build_identity();
                assert!(output.registered);
                assert!(output.agent_id.is_some());
                assert!(output.current_session.is_some());
                assert!(output.current_bead.is_some());
                assert!(!output.simple.is_empty());
                // simple should match agent_id
                assert_eq!(output.simple, output.agent_id.unwrap());
            },
        );
    }

    #[test]
    fn build_identity_output_has_consistent_fields_unregistered() {
        isolated(
            "__TEST_WHOMAI_STRUCT_UNREG",
            "commands::handlers::whoami::actions::tests::build_identity_output_has_consistent_fields_unregistered",
            &[],
            &["SCP_AGENT_ID", "ISOLATE_AGENT_ID", "SCP_SESSION", "ISOLATE_SESSION", "SCP_WORKSPACE", "ISOLATE_WORKSPACE", "SCP_BEAD_ID", "ISOLATE_BEAD_ID"],
            || {
                let output = build_identity();
                assert!(!output.registered);
                assert!(output.agent_id.is_none());
                assert!(output.current_session.is_none());
                assert!(output.current_bead.is_none());
                assert_eq!(output.simple, "unregistered");
            },
        );
    }
}
