//! Actions for the AI command (Tier 3).
//!
//! I/O boundary: serialization and Output calls.
//! All side-effects are isolated to this module.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::calculations::{
    build_overview, build_quick_start, build_workflow, determine_next_action, determine_ready_state,
};
use super::data::{
    AiEnvelope, AiOptions, AiStatusOutput, AiSubcommand, Location, NextActionOutput,
    AI_NEXT_RESPONSE, AI_OVERVIEW_RESPONSE, AI_QUICKSTART_RESPONSE, AI_STATUS_RESPONSE,
    AI_WORKFLOW_RESPONSE,
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

/// Serialize data into an AiEnvelope and write to Output.
fn serialize_and_output<T: serde::Serialize>(
    schema_name: &str,
    data: &T,
) -> Result<()> {
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
    let (location, workspace) = match cwd {
        Ok(ref path) => detect_location_from_path(path),
        Err(_) => (Location::Unknown, None),
    };

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
    let (location, workspace) = match cwd {
        Ok(ref path) => detect_location_from_path(path),
        Err(_) => (Location::Unknown, None),
    };

    let initialized = check_initialized();
    determine_next_action(initialized, &location, workspace.as_deref(), 0)
}

/// Detect location from the current working directory.
///
/// Checks for JJ or Git markers to determine if we are in a repo.
fn detect_location_from_path(path: &std::path::Path) -> (Location, Option<String>) {
    if path.join(".jj").exists() || path.join(".git").exists() {
        (Location::Main, None)
    } else {
        (Location::NotInRepo, None)
    }
}

/// Check if the current directory appears to be initialized.
fn check_initialized() -> bool {
    std::env::current_dir()
        .map(|path| path.join(".jj").exists() || path.join(".git").exists())
        .map_or(false, |v| v)
}
