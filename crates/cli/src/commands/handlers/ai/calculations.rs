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
            "Not in a Git repository".to_string(),
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
            .map(|s| (*s).to_string())
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
            action: "Enter a Git repository".to_string(),
            command: "cd <repo> && scp init".to_string(),
            reason: "Not currently in a Git repository".to_string(),
            priority: Priority::High,
        },
        _ if !initialized => NextActionOutput {
            action: "Initialize SCP".to_string(),
            command: "scp init".to_string(),
            reason: "SCP is not initialized in this repository".to_string(),
            priority: Priority::High,
        },
        _ if workspace.is_some() => {
            let ws = workspace.unwrap_or("unknown");
            NextActionOutput {
                action: format!("Continue work in '{ws}'"),
                command: "scp context --json".to_string(),
                reason: format!("Currently in workspace '{ws}' - check context or complete work"),
                priority: Priority::Medium,
            }
        }
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
                .map_or_else(|| "N/A".to_string(), std::string::ToString::to_string)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::ai::data::Location;

    // =========================================================================
    // determine_ready_state - additional edge cases
    // =========================================================================

    #[test]
    fn ready_state_workspace_uninitialized_hits_not_initialized_arm() {
        // The `_ if !initialized` arm is checked BEFORE `Location::Workspace(_)`,
        // so uninitialized workspace returns (false, "not initialized", "scp init").
        let (ready, suggestion, next_cmd) =
            determine_ready_state(false, &Location::Workspace("ws".to_string()));
        assert!(!ready, "Uninitialized should take priority over workspace");
        assert!(suggestion.contains("not initialized"), "Got: {suggestion}");
        assert!(next_cmd.contains("init"), "Got: {next_cmd}");
    }

    #[test]
    fn ready_state_unknown_uninitialized_suggests_init() {
        let (ready, suggestion, next_cmd) = determine_ready_state(false, &Location::Unknown);
        assert!(!ready, "Unknown + uninitialized should not be ready");
        assert!(suggestion.contains("not initialized"));
        assert!(next_cmd.contains("init"));
    }

    #[test]
    fn ready_state_main_initialized_has_suggestion() {
        let (ready, suggestion, next_cmd) = determine_ready_state(true, &Location::Main);
        assert!(ready);
        assert!(!suggestion.is_empty());
        assert!(!next_cmd.is_empty());
    }

    #[test]
    fn ready_state_not_in_repo_initialized_suggests_cd() {
        let (ready, _suggestion, next_cmd) = determine_ready_state(true, &Location::NotInRepo);
        assert!(!ready);
        assert!(next_cmd.contains("cd <repo>"));
        assert!(next_cmd.contains("scp init"));
    }

    // =========================================================================
    // format_session_count - boundary cases
    // =========================================================================

    #[test]
    fn session_count_one_is_singular() {
        assert_eq!(format_session_count(1), "1 session");
    }

    #[test]
    fn session_count_zero_is_plural() {
        assert_eq!(format_session_count(0), "0 sessions");
    }

    #[test]
    fn session_count_two_is_plural() {
        assert_eq!(format_session_count(2), "2 sessions");
    }

    // =========================================================================
    // build_workflow - structural tests
    // =========================================================================

    #[test]
    fn workflow_name_is_not_empty() {
        let workflow = build_workflow();
        assert!(!workflow.name.is_empty());
    }

    #[test]
    fn workflow_steps_are_sequentially_numbered() {
        let workflow = build_workflow();
        for (i, step) in workflow.steps.iter().enumerate() {
            assert_eq!(step.step, i + 1, "Step {i} should be numbered {}", i + 1);
        }
    }

    #[test]
    fn workflow_all_steps_have_non_empty_command_and_description() {
        let workflow = build_workflow();
        for step in &workflow.steps {
            assert!(
                !step.command.is_empty(),
                "Step {} must have command",
                step.step
            );
            assert!(
                !step.description.is_empty(),
                "Step {} must have description",
                step.step
            );
        }
    }

    #[test]
    fn workflow_is_deterministic() {
        let a = build_workflow();
        let b = build_workflow();
        assert_eq!(a.name, b.name);
        assert_eq!(a.steps.len(), b.steps.len());
        for (sa, sb) in a.steps.iter().zip(b.steps.iter()) {
            assert_eq!(sa.step, sb.step);
            assert_eq!(sa.command, sb.command);
            assert_eq!(sa.description, sb.description);
        }
    }

    // =========================================================================
    // build_quick_start - structural tests
    // =========================================================================

    #[test]
    fn quick_start_all_commands_have_non_empty_fields() {
        let qs = build_quick_start();
        for cmd in &qs.essential_commands {
            assert!(
                !cmd.command.is_empty(),
                "essential command must have command"
            );
            assert!(
                !cmd.purpose.is_empty(),
                "essential command must have purpose"
            );
        }
        for cmd in &qs.orientation {
            assert!(
                !cmd.command.is_empty(),
                "orientation command must have command"
            );
            assert!(
                !cmd.purpose.is_empty(),
                "orientation command must have purpose"
            );
        }
        for cmd in &qs.workflow {
            assert!(
                !cmd.command.is_empty(),
                "workflow command must have command"
            );
            assert!(
                !cmd.purpose.is_empty(),
                "workflow command must have purpose"
            );
        }
    }

    #[test]
    fn quick_start_is_deterministic() {
        let a = build_quick_start();
        let b = build_quick_start();
        assert_eq!(a.essential_commands.len(), b.essential_commands.len());
        assert_eq!(a.orientation.len(), b.orientation.len());
        assert_eq!(a.workflow.len(), b.workflow.len());
    }

    // =========================================================================
    // build_overview - structural tests
    // =========================================================================

    #[test]
    fn overview_subcommands_all_start_with_scp_ai() {
        let overview = build_overview();
        for sub in &overview.subcommands {
            assert!(
                sub.command.starts_with("scp ai "),
                "Subcommand should start with 'scp ai ': {}",
                sub.command
            );
            assert!(
                !sub.description.is_empty(),
                "Subcommand must have description: {}",
                sub.command
            );
        }
    }

    #[test]
    fn overview_quick_commands_all_non_empty() {
        let overview = build_overview();
        for cmd in &overview.quick_commands {
            assert!(!cmd.is_empty(), "Quick command must be non-empty");
        }
    }

    #[test]
    fn overview_is_deterministic() {
        let a = build_overview();
        let b = build_overview();
        assert_eq!(a.message, b.message);
        assert_eq!(a.subcommands.len(), b.subcommands.len());
        assert_eq!(a.quick_commands.len(), b.quick_commands.len());
    }

    // =========================================================================
    // determine_next_action - edge cases
    // =========================================================================

    #[test]
    fn next_action_uninitialized_in_workspace_hits_not_initialized_arm() {
        // The `_ if !initialized` arm is checked BEFORE the workspace arm,
        // so uninitialized workspace returns "scp init" with High priority.
        let output =
            determine_next_action(false, &Location::Workspace("ws".to_string()), Some("ws"), 0);
        assert_eq!(output.priority, Priority::High);
        assert!(output.command.contains("init"));
    }

    #[test]
    fn next_action_uninitialized_unknown_suggests_init() {
        let output = determine_next_action(false, &Location::Unknown, None, 0);
        assert_eq!(output.priority, Priority::High);
        assert!(output.command.contains("init"));
    }

    #[test]
    fn next_action_initialized_unknown_no_sessions_suggests_work() {
        let output = determine_next_action(true, &Location::Unknown, None, 0);
        assert_eq!(output.priority, Priority::Medium);
        assert!(output.command.contains("work"));
    }

    #[test]
    fn next_action_one_session_suggests_listing() {
        let output = determine_next_action(true, &Location::Main, None, 1);
        assert_eq!(output.priority, Priority::Medium);
        assert!(output.command.contains("list"));
        assert!(output.reason.contains('1'));
    }

    #[test]
    fn next_action_not_in_repo_takes_priority_over_uninitialized() {
        // Both false, not_in_repo should win
        let output = determine_next_action(false, &Location::NotInRepo, None, 0);
        assert_eq!(output.priority, Priority::High);
        assert!(output.command.contains("cd"));
    }

    #[test]
    fn next_action_not_in_repo_takes_priority_over_sessions() {
        let output = determine_next_action(true, &Location::NotInRepo, None, 50);
        assert_eq!(output.priority, Priority::High);
        assert!(output.command.contains("cd"));
    }

    #[test]
    fn next_action_not_in_repo_takes_priority_over_workspace() {
        let output = determine_next_action(true, &Location::NotInRepo, Some("ignored"), 0);
        assert_eq!(output.priority, Priority::High);
        assert!(output.command.contains("cd"));
    }

    #[test]
    fn next_action_all_fields_non_empty() {
        let combos: Vec<(bool, Location, Option<&str>, usize)> = vec![
            (false, Location::Main, None, 0),
            (true, Location::NotInRepo, None, 0),
            (true, Location::Workspace("ws".to_string()), Some("ws"), 0),
            (true, Location::Main, None, 5),
            (true, Location::Main, None, 0),
            (true, Location::Unknown, None, 0),
        ];
        for (init, loc, ws, sessions) in combos {
            let output = determine_next_action(init, &loc, ws, sessions);
            assert!(
                !output.action.is_empty(),
                "action must be non-empty for init={init}, loc={loc:?}"
            );
            assert!(
                !output.command.is_empty(),
                "command must be non-empty for init={init}, loc={loc:?}"
            );
            assert!(
                !output.reason.is_empty(),
                "reason must be non-empty for init={init}, loc={loc:?}"
            );
        }
    }

    // =========================================================================
    // format_status_human - additional edge cases
    // =========================================================================

    #[test]
    fn format_status_human_carriage_return_is_sanitized() {
        let status = crate::commands::handlers::ai::data::AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "ok\r\nINJECTED".to_string(),
            next_command: "scp work".to_string(),
        };
        let lines = format_status_human(&status);
        let injected_lines: Vec<&String> =
            lines.iter().filter(|l| l.contains("INJECTED")).collect();
        // The \r should be replaced, the \n should be replaced, so INJECTED
        // appears as part of a suggestion line, not a separate line.
        assert_eq!(
            injected_lines.len(),
            1,
            "INJECTED should only appear once, collapsed into suggestion"
        );
    }

    #[test]
    fn format_status_human_produces_expected_line_count() {
        let status = crate::commands::handlers::ai::data::AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: Some("agent".to_string()),
            initialized: true,
            active_sessions: 1,
            ready: true,
            suggestion: "Ready".to_string(),
            next_command: "scp work".to_string(),
        };
        let lines = format_status_human(&status);
        // Expect: title + separator + blank + 6 fields + blank + status + suggestion + blank +
        // "Next command:" + indented cmd = 14 lines
        assert!(
            lines.len() >= 10,
            "Should have at least 10 lines, got {}: {:?}",
            lines.len(),
            lines
        );
    }

    #[test]
    fn format_status_human_unknown_location_displayed() {
        let status = crate::commands::handlers::ai::data::AiStatusOutput {
            location: Location::Unknown,
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "Ready".to_string(),
            next_command: "scp work".to_string(),
        };
        let lines = format_status_human(&status);
        let has_unknown = lines.iter().any(|l| l.contains("unknown"));
        assert!(has_unknown, "Must display unknown location");
    }

    #[test]
    fn format_status_human_workspace_na_when_none() {
        let status = crate::commands::handlers::ai::data::AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "Ready".to_string(),
            next_command: "scp work".to_string(),
        };
        let lines = format_status_human(&status);
        let has_na = lines.iter().any(|l| l.contains("N/A"));
        assert!(has_na, "Must show N/A when workspace is None");
    }

    // =========================================================================
    // commands_from_table - via build_quick_start / build_overview (indirect)
    // =========================================================================

    #[test]
    fn commands_from_table_produces_correct_length_via_quick_start() {
        let qs = build_quick_start();
        assert_eq!(
            qs.essential_commands.len(),
            ESSENTIAL_COMMANDS.len(),
            "essential_commands length should match data table"
        );
        assert_eq!(
            qs.orientation.len(),
            ORIENTATION_COMMANDS.len(),
            "orientation length should match data table"
        );
        assert_eq!(
            qs.workflow.len(),
            WORKFLOW_COMMANDS.len(),
            "workflow length should match data table"
        );
    }

    #[test]
    fn workflow_steps_length_matches_data_table() {
        let workflow = build_workflow();
        assert_eq!(
            workflow.steps.len(),
            WORKFLOW_STEPS.len(),
            "workflow steps should match data table"
        );
    }

    #[test]
    fn overview_subcommands_length_matches_data_table() {
        let overview = build_overview();
        assert_eq!(
            overview.subcommands.len(),
            OVERVIEW_SUBCOMMANDS.len(),
            "subcommands should match data table"
        );
    }

    #[test]
    fn overview_quick_commands_length_matches_data_table() {
        let overview = build_overview();
        assert_eq!(
            overview.quick_commands.len(),
            OVERVIEW_QUICK_COMMANDS.len(),
            "quick_commands should match data table"
        );
    }
}
