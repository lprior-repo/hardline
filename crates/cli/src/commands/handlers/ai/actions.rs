//! Actions for the AI command (Tier 3).
//!
//! I/O boundary: serialization and Output calls.
//! All side-effects are isolated to this module.

use scp_core::{output::Output, Error, Result};

use super::{
    calculations::{
        build_overview, build_quick_start, build_workflow, determine_next_action,
        determine_ready_state,
    },
    data::{
        AiEnvelope, AiOptions, AiStatusOutput, AiSubcommand, Location, NextActionOutput,
        AI_NEXT_RESPONSE, AI_OVERVIEW_RESPONSE, AI_QUICKSTART_RESPONSE, AI_STATUS_RESPONSE,
        AI_WORKFLOW_RESPONSE,
    },
};

/// Run the ai command dispatcher.
pub fn run(options: &AiOptions) -> Result<()> {
    match options.subcommand {
        AiSubcommand::Status => run_status(),
        AiSubcommand::Workflow => run_workflow(),
        AiSubcommand::QuickStart => run_quick_start(),
        AiSubcommand::Next => run_next(),
        AiSubcommand::Default => run_default(),
    }
}

/// Run AI status - comprehensive state check with guidance.
///
/// Currently returns a default status. Full implementation requires
/// wiring to VCS backend and session database.
pub fn run_status() -> Result<()> {
    let output = build_default_status();
    serialize_and_output(AI_STATUS_RESPONSE, &output)
}

/// Run AI workflow - show the parallel agent workflow.
pub fn run_workflow() -> Result<()> {
    let workflow = build_workflow();
    serialize_and_output(AI_WORKFLOW_RESPONSE, &workflow)
}

/// Run AI quick-start - minimum commands to be productive.
pub fn run_quick_start() -> Result<()> {
    let output = build_quick_start();
    serialize_and_output(AI_QUICKSTART_RESPONSE, &output)
}

/// Run AI next - single next action.
///
/// Currently returns a default next action. Full implementation requires
/// wiring to VCS backend and session database.
pub fn run_next() -> Result<()> {
    let output = build_default_next_action();
    serialize_and_output(AI_NEXT_RESPONSE, &output)
}

/// Run AI default - overview and help.
pub fn run_default() -> Result<()> {
    let output = build_overview();
    serialize_and_output(AI_OVERVIEW_RESPONSE, &output)
}

// =============================================================================
// Private action helpers
// =============================================================================

/// Serialize data into an `AiEnvelope` and write to Output.
fn serialize_and_output<T: serde::Serialize>(schema_name: &str, data: &T) -> Result<()> {
    let envelope = AiEnvelope::new(schema_name, "single", data);
    let json_str = serde_json::to_string_pretty(&envelope)
        .map_err(|e| Error::io_error(format!("Failed to serialize AI response: {e}")))?;
    Output::info(&json_str);
    Ok(())
}

/// Build a default status for the current environment.
///
/// Returns a status reflecting the current state based on environment checks.
fn build_default_status() -> AiStatusOutput {
    let agent_id = std::env::var("SCP_AGENT_ID")
        .ok()
        .or_else(|| std::env::var("ISOLATE_AGENT_ID").ok());

    let cwd = std::env::current_dir();
    let (location, workspace) = cwd.as_ref().map_or((Location::Unknown, None), |path| detect_location_from_path(path));

    let initialized = check_initialized();
    let (ready, suggestion, next_command) = determine_ready_state(initialized, &location);

    AiStatusOutput {
        location,
        workspace,
        agent_id,
        initialized,
        active_sessions: 0,
        ready,
        suggestion,
        next_command,
    }
}

/// Build a default next action for the current environment.
fn build_default_next_action() -> NextActionOutput {
    let cwd = std::env::current_dir();
    let (location, workspace) = cwd.as_ref().map_or((Location::Unknown, None), |path| detect_location_from_path(path));

    let initialized = check_initialized();
    determine_next_action(initialized, &location, workspace.as_deref(), 0)
}

/// Detect location from the current working directory.
///
/// Checks for Git markers to determine if we are in a repo.
fn detect_location_from_path(path: &std::path::Path) -> (Location, Option<String>) {
    if path.join(".git").exists() {
        (Location::Main, None)
    } else {
        (Location::NotInRepo, None)
    }
}

/// Check if the current directory appears to be initialized.
fn check_initialized() -> bool {
    std::env::current_dir()
        .map(|path| path.join(".git").exists())
        .is_ok_and(|v| v)
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;
    use crate::commands::handlers::ai::data::AiSubcommand;

    // =========================================================================
    // Error handling patterns
    // =========================================================================

    #[test]
    fn serialize_and_output_succeeds_with_valid_struct_data() {
        // serialize_and_output wraps data in AiEnvelope which uses #[serde(flatten)],
        // so only struct/map data is supported (not primitives).
        #[derive(serde::Serialize)]
        struct TestPayload {
            value: String,
        }
        let result = serialize_and_output(
            "test-schema",
            &TestPayload {
                value: "hello".to_string(),
            },
        );
        match result {
            Ok(()) => {}
            Err(e) => panic!("serialize_and_output with valid struct data should succeed: {e}"),
        }
    }

    #[test]
    fn serialize_and_output_succeeds_with_struct_data() {
        let data = super::super::data::NextActionOutput {
            action: "test".to_string(),
            command: "scp work".to_string(),
            reason: "testing".to_string(),
            priority: super::super::data::Priority::Medium,
        };
        let result = serialize_and_output("test-next", &data);
        match result {
            Ok(()) => {}
            Err(e) => panic!("serialize_and_output with NextActionOutput should succeed: {e}"),
        }
    }

    #[test]
    fn serialize_and_output_succeeds_with_empty_struct() {
        #[derive(serde::Serialize)]
        struct Empty {}
        let result = serialize_and_output("test-empty", &Empty {});
        match result {
            Ok(()) => {}
            Err(e) => panic!("serialize_and_output with empty struct should succeed: {e}"),
        }
    }

    #[test]
    fn serialize_and_output_succeeds_with_nested_data() {
        let data = super::super::data::WorkflowInfo {
            name: "test".to_string(),
            steps: vec![],
        };
        let result = serialize_and_output("test-workflow", &data);
        match result {
            Ok(()) => {}
            Err(e) => panic!("serialize_and_output with WorkflowInfo should succeed: {e}"),
        }
    }

    #[test]
    fn serialize_and_output_produces_valid_json() {
        let data = super::super::data::AiOverview {
            message: "test".to_string(),
            subcommands: vec![],
            quick_commands: vec![],
        };
        // Capture output by verifying the function returns Ok
        let result = serialize_and_output("test-overview", &data);
        assert!(result.is_ok(), "Should produce valid JSON output");

        // Also verify the serialization step directly
        let envelope = super::super::data::AiEnvelope::new("test-overview", "single", &data);
        match serde_json::to_string(&envelope) {
            Ok(json_str) => {
                assert!(
                    json_str.contains("\"$schema\""),
                    "Envelope must have $schema"
                );
                assert!(
                    json_str.contains("\"success\""),
                    "Envelope must have success"
                );
                // Verify it's parseable
                match serde_json::from_str::<serde_json::Value>(&json_str) {
                    Ok(_) => {}
                    Err(e) => panic!("Envelope JSON should be parseable: {e}"),
                }
            }
            Err(e) => panic!("Should serialize to valid JSON: {e}"),
        }
    }

    #[test]
    fn run_status_returns_ok() {
        let result = run_status();
        match result {
            Ok(()) => {}
            Err(e) => panic!("run_status should succeed: {e}"),
        }
    }

    #[test]
    fn run_workflow_returns_ok() {
        let result = run_workflow();
        match result {
            Ok(()) => {}
            Err(e) => panic!("run_workflow should succeed: {e}"),
        }
    }

    #[test]
    fn run_quick_start_returns_ok() {
        let result = run_quick_start();
        match result {
            Ok(()) => {}
            Err(e) => panic!("run_quick_start should succeed: {e}"),
        }
    }

    #[test]
    fn run_next_returns_ok() {
        let result = run_next();
        match result {
            Ok(()) => {}
            Err(e) => panic!("run_next should succeed: {e}"),
        }
    }

    #[test]
    fn run_default_returns_ok() {
        let result = run_default();
        match result {
            Ok(()) => {}
            Err(e) => panic!("run_default should succeed: {e}"),
        }
    }

    #[test]
    fn run_dispatches_status_subcommand() {
        let opts = super::super::data::AiOptions {
            subcommand: AiSubcommand::Status,
        };
        let result = run(&opts);
        match result {
            Ok(()) => {}
            Err(e) => panic!("run with Status should succeed: {e}"),
        }
    }

    #[test]
    fn run_dispatches_workflow_subcommand() {
        let opts = super::super::data::AiOptions {
            subcommand: AiSubcommand::Workflow,
        };
        let result = run(&opts);
        match result {
            Ok(()) => {}
            Err(e) => panic!("run with Workflow should succeed: {e}"),
        }
    }

    #[test]
    fn run_dispatches_quick_start_subcommand() {
        let opts = super::super::data::AiOptions {
            subcommand: AiSubcommand::QuickStart,
        };
        let result = run(&opts);
        match result {
            Ok(()) => {}
            Err(e) => panic!("run with QuickStart should succeed: {e}"),
        }
    }

    #[test]
    fn run_dispatches_next_subcommand() {
        let opts = super::super::data::AiOptions {
            subcommand: AiSubcommand::Next,
        };
        let result = run(&opts);
        match result {
            Ok(()) => {}
            Err(e) => panic!("run with Next should succeed: {e}"),
        }
    }

    #[test]
    fn run_dispatches_default_subcommand() {
        let opts = super::super::data::AiOptions {
            subcommand: AiSubcommand::Default,
        };
        let result = run(&opts);
        match result {
            Ok(()) => {}
            Err(e) => panic!("run with Default should succeed: {e}"),
        }
    }

    // =========================================================================
    // detect_location_from_path - test with temp directories
    // =========================================================================

    #[test]
    fn detect_location_returns_main_for_git_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        let (loc, ws) = detect_location_from_path(dir.path());
        assert_eq!(loc, Location::Main);
        assert!(ws.is_none());
    }

    #[test]
    fn detect_location_returns_not_in_repo_for_plain_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (loc, ws) = detect_location_from_path(dir.path());
        assert_eq!(loc, Location::NotInRepo);
        assert!(ws.is_none());
    }

    // =========================================================================
    // check_initialized
    // =========================================================================

    #[test]
    #[serial]
    fn check_initialized_returns_true_in_git_repo() {
        let original_dir = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        if std::fs::create_dir(dir.path().join(".git")).is_err() {
            return;
        }
        if std::env::set_current_dir(dir.path()).is_err() {
            return;
        }
        let result = check_initialized();
        // Restore cwd before TempDir drops so the cwd isn't invalidated
        std::env::set_current_dir(&original_dir).ok();
        assert!(result, "Should be initialized in a git repo");
    }

    #[test]
    #[serial]
    fn check_initialized_returns_false_in_plain_dir() {
        let original_dir = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            return;
        }
        let result = check_initialized();
        std::env::set_current_dir(&original_dir).ok();
        assert!(!result, "Should not be initialized in a plain dir");
    }

    // =========================================================================
    // build_default_status - returns valid structure
    // =========================================================================

    #[test]
    fn build_default_status_returns_structured_output() {
        let output = build_default_status();
        // Verify all fields are populated
        assert!(!format!("{:?}", output.location).is_empty());
        // initialized should reflect current dir state
        // ready, suggestion, next_command should be consistent
        assert!(!output.suggestion.is_empty());
        assert!(!output.next_command.is_empty());
    }

    #[test]
    fn build_default_next_action_returns_structured_output() {
        let output = build_default_next_action();
        assert!(!output.action.is_empty());
        assert!(!output.command.is_empty());
        assert!(!output.reason.is_empty());
    }
}
