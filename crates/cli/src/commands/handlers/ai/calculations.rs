//! Calculations for the AI command (Tier 2).
//!
//! Pure functions with no I/O. All business logic lives here.
//! Every function is deterministic and side-effect-free.

use super::data::{
    AiOverview, AiStatusOutput, Location, NextActionOutput, Priority, QuickCommand,
    QuickStartOutput, SubcommandInfo, WorkflowInfo, WorkflowStep,
};

// =============================================================================
// Const data tables (DEFECT-9NB-4: replaces long builder functions)
// =============================================================================

/// Workflow step data table.
///
/// Each entry is `(step_number, command, description)`.
const WORKFLOW_STEPS: &[(&str, &str)] = &[
    ("scp whereami", "Orient: Check current location"),
    ("scp agent register", "Register: Identify yourself"),
    (
        "scp work <task-name> --idempotent",
        "Isolate: Create workspace",
    ),
    (
        "cd $(scp context --field location.path)",
        "Enter: Navigate to workspace",
    ),
    ("# implement changes", "Implement: Do the work"),
    ("scp agent heartbeat", "Heartbeat: Signal liveness"),
    ("scp done", "Complete: Merge and cleanup"),
];

/// Essential commands data table.
const ESSENTIAL_COMMANDS: &[(&str, &str)] = &[
    ("scp whereami", "Returns 'main' or 'workspace:<name>'"),
    ("scp work <name>", "Create workspace and start working"),
    ("scp done", "Complete work and merge"),
    ("scp abort", "Abandon work without merging"),
];

/// Orientation commands data table.
const ORIENTATION_COMMANDS: &[(&str, &str)] = &[
    ("scp whereami", "Location check"),
    ("scp whoami", "Agent identity"),
    ("scp ai status", "Full status with guidance"),
];

/// Workflow commands data table.
const WORKFLOW_COMMANDS: &[(&str, &str)] = &[
    ("scp work task-name --idempotent", "Safe to retry"),
    ("scp done", "Merge when done"),
];

/// Overview subcommands data table.
const OVERVIEW_SUBCOMMANDS: &[(&str, &str)] = &[
    ("scp ai status", "Current state with guided next action"),
    ("scp ai next", "Single next action with copy-paste command"),
    ("scp ai workflow", "7-step parallel agent workflow"),
    ("scp ai quick-start", "Minimum commands to be productive"),
];

/// Overview quick commands data table.
const OVERVIEW_QUICK_COMMANDS: &[&str] = &[
    "scp whereami          # Location",
    "scp work <name>       # Start work",
    "scp done              # Finish work",
];

// =============================================================================
// Pure functions
// =============================================================================

/// Determine readiness state and next command based on current context.
///
/// Pure function that maps `(initialized, location)` to a readiness tuple.
///
/// # Priority order (VULN-9NB-1 fix)
///
/// `not_in_repo` is checked BEFORE `initialized` because advising `scp init`
/// when not even in a repository is misleading. The more actionable advice is
/// to enter a repo first, matching the priority order in `determine_next_action`.
#[must_use]
pub fn determine_ready_state(initialized: bool, location: &Location) -> (bool, String, String) {
    match location {
        Location::NotInRepo => (
            false,
            "Not in a JJ repository".to_string(),
            "cd <repo> && scp init".to_string(),
        ),
        _ if !initialized => (
            false,
            "SCP not initialized".to_string(),
            "scp init".to_string(),
        ),
        Location::Workspace(_) => (
            true,
            "In workspace - continue working or complete".to_string(),
            "scp done".to_string(),
        ),
        _ => (
            true,
            "Ready to start work".to_string(),
            "scp work <task-name>".to_string(),
        ),
    }
}

/// Format session count with proper pluralization.
///
/// Returns "X session" for count == 1, "X sessions" otherwise.
#[must_use]
pub fn format_session_count(count: usize) -> String {
    if count == 1 {
        "1 session".to_string()
    } else {
        format!("{count} sessions")
    }
}

/// Build the parallel agent workflow from the const data table.
///
/// Pure function returning the canonical 7-step workflow.
#[must_use]
pub fn build_workflow() -> WorkflowInfo {
    WorkflowInfo {
        name: "Parallel Agent Workflow".to_string(),
        steps: WORKFLOW_STEPS
            .iter()
            .enumerate()
            .map(|(i, &(cmd, desc))| WorkflowStep {
                step: i + 1,
                command: cmd.to_string(),
                description: desc.to_string(),
            })
            .collect(),
    }
}

/// Build the quick-start command reference from const data tables.
///
/// Pure function returning essential commands for AI agents.
#[must_use]
pub fn build_quick_start() -> QuickStartOutput {
    QuickStartOutput {
        essential_commands: commands_from_table(ESSENTIAL_COMMANDS),
        orientation: commands_from_table(ORIENTATION_COMMANDS),
        workflow: commands_from_table(WORKFLOW_COMMANDS),
    }
}

/// Build the AI overview from const data tables.
///
/// Pure function returning the default overview with subcommands.
#[must_use]
pub fn build_overview() -> AiOverview {
    AiOverview {
        message: "SCP AI Agent Interface - Start here for AI-driven workflows".to_string(),
        subcommands: OVERVIEW_SUBCOMMANDS
            .iter()
            .map(|&(cmd, desc)| SubcommandInfo {
                command: cmd.to_string(),
                description: desc.to_string(),
            })
            .collect(),
        quick_commands: OVERVIEW_QUICK_COMMANDS
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

/// Determine the single best next action based on current state.
///
/// Pure function mapping `(initialized, location, workspace, active_sessions)` to
/// a `NextActionOutput`.
#[must_use]
pub fn determine_next_action(
    initialized: bool,
    location: &Location,
    workspace: Option<&str>,
    active_sessions: usize,
) -> NextActionOutput {
    match location {
        Location::NotInRepo => NextActionOutput {
            action: "Enter a JJ repository".to_string(),
            command: "cd <repo> && scp init".to_string(),
            reason: "Not currently in a JJ repository".to_string(),
            priority: Priority::High,
        },
        _ if !initialized => NextActionOutput {
            action: "Initialize SCP".to_string(),
            command: "scp init".to_string(),
            reason: "SCP is not initialized in this repository".to_string(),
            priority: Priority::High,
        },
        _ if let Some(ws) = workspace => NextActionOutput {
            action: format!("Continue work in '{ws}'"),
            command: "scp context --json".to_string(),
            reason: format!("Currently in workspace '{ws}' - check context or complete work"),
            priority: Priority::Medium,
        },
        _ if active_sessions > 0 => NextActionOutput {
            action: "Check existing sessions".to_string(),
            command: "scp list --json".to_string(),
            reason: format!("{active_sessions} active session(s) exist - review or continue work"),
            priority: Priority::Medium,
        },
        _ => NextActionOutput {
            action: "Start new work session".to_string(),
            command: "scp work <task-name>".to_string(),
            reason: "Ready to start work - no active sessions".to_string(),
            priority: Priority::Medium,
        },
    }
}

/// Sanitize a string by replacing newlines with spaces.
///
/// Prevents newline injection when formatting user-controlled strings
/// into line-oriented output (VULN-9NB-2).
#[must_use]
fn sanitize_newlines(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}

/// Format status as human-readable lines.
///
/// Pure function returning an array of output lines.
/// Newlines in `suggestion` and `next_command` are sanitized to prevent
/// line-injection attacks (VULN-9NB-2).
#[must_use]
pub fn format_status_human(output: &AiStatusOutput) -> Vec<String> {
    let safe_suggestion = sanitize_newlines(&output.suggestion);
    let safe_next_command = sanitize_newlines(&output.next_command);

    let agent_line = match output.agent_id {
        Some(ref agent) => format!("Agent ID:      {agent}"),
        None => "Agent ID:      (not registered)".to_string(),
    };

    vec![
        "AI Agent Status".to_string(),
        "===============".to_string(),
        String::new(),
        format!("Location:      {}", output.location),
        format!(
            "Workspace:     {}",
            output
                .workspace
                .as_deref()
                .map_or_else(|| "N/A".to_string(), |ws| ws.to_string())
        ),
        agent_line,
        format!(
            "Initialized:   {}",
            if output.initialized { "yes" } else { "no" }
        ),
        format!(
            "Active work:   {}",
            format_session_count(output.active_sessions)
        ),
        String::new(),
        format!(
            "Status: {}",
            if output.ready { "READY" } else { "NOT READY" }
        ),
        format!("Suggestion: {safe_suggestion}"),
        String::new(),
        "Next command:".to_string(),
        format!("  {safe_next_command}"),
    ]
}

// =============================================================================
// Private helpers
// =============================================================================

/// Convert a `&[(&str, &str)]` data table into `Vec<QuickCommand>`.
fn commands_from_table(table: &[(&str, &str)]) -> Vec<QuickCommand> {
    table
        .iter()
        .map(|&(cmd, purpose)| QuickCommand {
            command: cmd.to_string(),
            purpose: purpose.to_string(),
        })
        .collect()
}
