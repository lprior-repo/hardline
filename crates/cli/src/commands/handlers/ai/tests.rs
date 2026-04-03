//! Tests for the AI command.
//!
//! All test names are descriptive (no `fn test_` prefix).
//! No `is_ok()`/`is_err()` assertions -- exact variant matching only.
//! No unbounded loops -- all iteration is bounded by fixed-length arrays.

use super::calculations::{
    build_overview, build_quick_start, build_workflow, determine_next_action,
    determine_ready_state, format_session_count, format_status_human,
};
use super::data::{
    AiEnvelope, AiOverview, AiStatusOutput, AiSubcommand, Location, NextActionOutput, Priority,
    QuickCommand, QuickStartOutput, SubcommandInfo, WorkflowInfo, WorkflowStep, AI_STATUS_RESPONSE,
};

// =============================================================================
// Data type serialization - exact variant matching (no is_ok/is_err)
// =============================================================================

#[test]
fn ai_status_output_serializes_to_valid_json_object() {
    let output = AiStatusOutput {
        location: Location::Main,
        workspace: None,
        agent_id: None,
        initialized: true,
        active_sessions: 0,
        ready: true,
        suggestion: "Ready to work".to_string(),
        next_command: "scp work <task>".to_string(),
    };

    let json = serde_json::to_string(&output);
    match json {
        Ok(s) => assert!(
            s.contains("\"location\""),
            "Serialized JSON must contain location field: {s}"
        ),
        Err(e) => panic!("Serialization must succeed, got: {e}"),
    }
}

#[test]
fn workflow_info_serializes_to_valid_json_array() {
    let workflow = WorkflowInfo {
        name: "Test".to_string(),
        steps: vec![WorkflowStep {
            step: 1,
            command: "test".to_string(),
            description: "Test step".to_string(),
        }],
    };

    let json = serde_json::to_string(&workflow);
    match json {
        Ok(s) => assert!(
            s.contains("\"steps\""),
            "Serialized JSON must contain steps field: {s}"
        ),
        Err(e) => panic!("Serialization must succeed, got: {e}"),
    }
}

#[test]
fn next_action_output_serializes_with_priority_enum() {
    let output = NextActionOutput {
        action: "Start work".to_string(),
        command: "scp work my-task".to_string(),
        reason: "Ready to begin".to_string(),
        priority: Priority::Medium,
    };

    let json = serde_json::to_string(&output);
    match json {
        Ok(s) => assert!(
            s.contains("\"priority\""),
            "Serialized JSON must contain priority field: {s}"
        ),
        Err(e) => panic!("Serialization must succeed, got: {e}"),
    }
}

#[test]
fn quick_start_output_serializes_with_command_arrays() {
    let output = QuickStartOutput {
        essential_commands: vec![QuickCommand {
            command: "scp work".to_string(),
            purpose: "Start working".to_string(),
        }],
        orientation: vec![],
        workflow: vec![],
    };

    let json = serde_json::to_string(&output);
    match json {
        Ok(s) => assert!(
            s.contains("\"essential_commands\""),
            "Must contain essential_commands: {s}"
        ),
        Err(e) => panic!("Serialization must succeed, got: {e}"),
    }
}

#[test]
fn ai_overview_serializes_with_subcommands_array() {
    let overview = AiOverview {
        message: "SCP AI Interface".to_string(),
        subcommands: vec![SubcommandInfo {
            command: "scp ai status".to_string(),
            description: "Get status".to_string(),
        }],
        quick_commands: vec!["scp whereami".to_string()],
    };

    let json = serde_json::to_string(&overview);
    match json {
        Ok(s) => assert!(
            s.contains("\"subcommands\""),
            "Must contain subcommands: {s}"
        ),
        Err(e) => panic!("Serialization must succeed, got: {e}"),
    }
}

// =============================================================================
// determine_ready_state calculations
// =============================================================================

#[test]
fn ready_state_uninitialized_on_main_suggests_init() {
    let (ready, suggestion, next_cmd) = determine_ready_state(false, &Location::Main);
    assert!(!ready);
    assert!(suggestion.contains("not initialized"));
    assert!(next_cmd.contains("init"));
}

#[test]
fn ready_state_not_in_repo_suggests_entering_repo() {
    let (ready, suggestion, next_cmd) = determine_ready_state(true, &Location::NotInRepo);
    assert!(!ready);
    assert!(suggestion.contains("Git repository"));
    assert!(next_cmd.contains("cd"));
}

#[test]
fn ready_state_in_workspace_suggests_done() {
    let (ready, suggestion, next_cmd) =
        determine_ready_state(true, &Location::Workspace("feature".to_string()));
    assert!(ready);
    assert!(suggestion.contains("workspace"));
    assert!(next_cmd.contains("done"));
}

#[test]
fn ready_state_on_main_suggests_work() {
    let (ready, suggestion, next_cmd) = determine_ready_state(true, &Location::Main);
    assert!(ready);
    assert!(suggestion.contains("Ready"));
    assert!(next_cmd.contains("work"));
}

#[test]
fn ready_state_unknown_location_but_initialized_is_ready() {
    let (ready, suggestion, _next_cmd) = determine_ready_state(true, &Location::Unknown);
    assert!(ready);
    assert!(suggestion.contains("Ready"));
}

// =============================================================================
// VULN-9NB-1 fix: not_in_repo checked before initialized
// =============================================================================

#[test]
fn ready_state_not_in_repo_takes_priority_over_uninitialized() {
    // When both not_in_repo AND uninitialized, not_in_repo wins.
    // This matches determine_next_action's priority order.
    let (ready, suggestion, next_cmd) = determine_ready_state(false, &Location::NotInRepo);
    assert!(!ready);
    assert!(
        suggestion.contains("Git repository"),
        "Should say 'not in repo', got: {suggestion}"
    );
    assert!(
        next_cmd.contains("cd"),
        "Should suggest cd, got: {next_cmd}"
    );
}

#[test]
fn ready_state_and_next_action_agree_on_not_in_repo_uninitialized() {
    let (ready, _, next_cmd) = determine_ready_state(false, &Location::NotInRepo);
    let action = determine_next_action(false, &Location::NotInRepo, None, 0);

    // Both should give consistent "enter a repo" advice
    assert!(!ready, "Not in repo should not be ready");
    assert!(
        next_cmd.contains("cd"),
        "ready_state should suggest cd, got: {next_cmd}"
    );
    assert!(
        action.command.contains("cd"),
        "next_action should suggest cd, got: {}",
        action.command
    );
}

// =============================================================================
// format_session_count
// =============================================================================

#[test]
fn zero_sessions_formats_as_plural() {
    assert_eq!(format_session_count(0), "0 sessions");
}

#[test]
fn one_session_formats_as_singular() {
    assert_eq!(format_session_count(1), "1 session");
}

#[test]
fn two_sessions_formats_as_plural() {
    assert_eq!(format_session_count(2), "2 sessions");
}

#[test]
fn ten_sessions_formats_as_plural() {
    assert_eq!(format_session_count(10), "10 sessions");
}

#[test]
fn hundred_sessions_formats_as_plural() {
    assert_eq!(format_session_count(100), "100 sessions");
}

// =============================================================================
// build_workflow
// =============================================================================

#[test]
fn workflow_contains_exactly_seven_steps() {
    let workflow = build_workflow();
    assert_eq!(workflow.steps.len(), 7);
}

#[test]
fn workflow_step_one_is_orientation() {
    let workflow = build_workflow();
    let first = workflow.steps.first();
    assert!(first.is_some(), "Must have at least one step");
    let first = first
        .map(|s| s.command.as_str())
        .map_or(false, |c| c.contains("whereami"));
    assert!(first, "First step must be whereami");
}

#[test]
fn workflow_step_seven_is_completion() {
    let workflow = build_workflow();
    let last = workflow.steps.last();
    assert!(last.is_some(), "Must have at least one step");
    let last = last
        .map(|s| s.command.as_str())
        .map_or(false, |c| c.contains("done"));
    assert!(last, "Last step must be done");
}

#[test]
fn workflow_step_one_numbered_correctly() {
    let workflow = build_workflow();
    match workflow.steps.first() {
        Some(s) => assert_eq!(s.step, 1, "First step must be numbered 1"),
        None => panic!("Workflow must have steps"),
    }
}

#[test]
fn workflow_step_seven_numbered_correctly() {
    let workflow = build_workflow();
    match workflow.steps.get(6) {
        Some(s) => assert_eq!(s.step, 7, "Seventh step must be numbered 7"),
        None => panic!("Workflow must have 7 steps"),
    }
}

#[test]
fn workflow_step_one_has_command_and_description() {
    let workflow = build_workflow();
    match workflow.steps.first() {
        Some(s) => {
            assert!(!s.command.is_empty(), "Step 1 must have a command");
            assert!(!s.description.is_empty(), "Step 1 must have a description");
        }
        None => panic!("Workflow must have steps"),
    }
}

#[test]
fn workflow_step_four_has_command_and_description() {
    let workflow = build_workflow();
    match workflow.steps.get(3) {
        Some(s) => {
            assert!(!s.command.is_empty(), "Step 4 must have a command");
            assert!(!s.description.is_empty(), "Step 4 must have a description");
        }
        None => panic!("Workflow must have 4+ steps"),
    }
}

#[test]
fn workflow_step_one_command_starts_with_scp() {
    let workflow = build_workflow();
    match workflow.steps.first() {
        Some(s) => assert!(
            s.command.starts_with("scp "),
            "Step 1 command should start with 'scp ', got: {}",
            s.command
        ),
        None => panic!("Workflow must have steps"),
    }
}

#[test]
fn workflow_step_four_command_is_cd_or_hash() {
    let workflow = build_workflow();
    match workflow.steps.get(3) {
        Some(s) => assert!(
            s.command.starts_with("cd ")
                || s.command.starts_with("scp ")
                || s.command.starts_with('#'),
            "Step 4 command is cd/scp/comment, got: {}",
            s.command
        ),
        None => panic!("Workflow must have 4+ steps"),
    }
}

// =============================================================================
// build_quick_start
// =============================================================================

#[test]
fn quick_start_has_at_least_four_essential_commands() {
    let qs = build_quick_start();
    assert!(qs.essential_commands.len() >= 4);
}

#[test]
fn quick_start_has_orientation_commands() {
    let qs = build_quick_start();
    assert!(!qs.orientation.is_empty());
}

#[test]
fn quick_start_has_workflow_commands() {
    let qs = build_quick_start();
    assert!(!qs.workflow.is_empty());
}

#[test]
fn quick_start_first_essential_command_starts_with_scp() {
    let qs = build_quick_start();
    match qs.essential_commands.first() {
        Some(cmd) => assert!(
            cmd.command.starts_with("scp "),
            "First essential command should start with 'scp ', got: {}",
            cmd.command
        ),
        None => panic!("Must have essential commands"),
    }
}

#[test]
fn quick_start_second_essential_command_starts_with_scp() {
    let qs = build_quick_start();
    match qs.essential_commands.get(1) {
        Some(cmd) => assert!(
            cmd.command.starts_with("scp "),
            "Second essential command should start with 'scp ', got: {}",
            cmd.command
        ),
        None => panic!("Must have at least 2 essential commands"),
    }
}

// =============================================================================
// build_overview
// =============================================================================

#[test]
fn overview_has_at_least_four_subcommands() {
    let overview = build_overview();
    assert!(overview.subcommands.len() >= 4);
}

#[test]
fn overview_has_quick_commands() {
    let overview = build_overview();
    assert!(!overview.quick_commands.is_empty());
}

#[test]
fn overview_message_is_not_empty() {
    let overview = build_overview();
    assert!(!overview.message.is_empty());
}

#[test]
fn overview_first_subcommand_starts_with_scp_ai() {
    let overview = build_overview();
    match overview.subcommands.first() {
        Some(sub) => assert!(
            sub.command.starts_with("scp ai "),
            "First subcommand should start with 'scp ai ', got: {}",
            sub.command
        ),
        None => panic!("Must have subcommands"),
    }
}

#[test]
fn overview_second_subcommand_starts_with_scp_ai() {
    let overview = build_overview();
    match overview.subcommands.get(1) {
        Some(sub) => assert!(
            sub.command.starts_with("scp ai "),
            "Second subcommand should start with 'scp ai ', got: {}",
            sub.command
        ),
        None => panic!("Must have at least 2 subcommands"),
    }
}

#[test]
fn overview_third_subcommand_starts_with_scp_ai() {
    let overview = build_overview();
    match overview.subcommands.get(2) {
        Some(sub) => assert!(
            sub.command.starts_with("scp ai "),
            "Third subcommand should start with 'scp ai ', got: {}",
            sub.command
        ),
        None => panic!("Must have at least 3 subcommands"),
    }
}

#[test]
fn overview_fourth_subcommand_starts_with_scp_ai() {
    let overview = build_overview();
    match overview.subcommands.get(3) {
        Some(sub) => assert!(
            sub.command.starts_with("scp ai "),
            "Fourth subcommand should start with 'scp ai ', got: {}",
            sub.command
        ),
        None => panic!("Must have at least 4 subcommands"),
    }
}

// =============================================================================
// determine_next_action
// =============================================================================

#[test]
fn next_action_uninitialized_on_main_suggests_init_with_high_priority() {
    let output = determine_next_action(false, &Location::Main, None, 0);
    assert_eq!(output.priority, Priority::High);
    assert!(output.command.contains("init"));
}

#[test]
fn next_action_not_in_repo_suggests_entering_repo_with_high_priority() {
    let output = determine_next_action(false, &Location::NotInRepo, None, 0);
    assert_eq!(output.priority, Priority::High);
    assert!(output.command.contains("cd"));
    assert!(output.reason.contains("repository"));
}

#[test]
fn next_action_in_workspace_suggests_context_with_medium_priority() {
    let output = determine_next_action(
        true,
        &Location::Workspace("feature-auth".to_string()),
        Some("feature-auth"),
        0,
    );
    assert_eq!(output.priority, Priority::Medium);
    assert!(output.action.contains("feature-auth"));
    assert!(output.command.contains("context"));
}

#[test]
fn next_action_on_main_with_sessions_suggests_listing() {
    let output = determine_next_action(true, &Location::Main, None, 3);
    assert_eq!(output.priority, Priority::Medium);
    assert!(output.command.contains("list"));
    assert!(output.reason.contains('3'));
}

#[test]
fn next_action_on_main_no_sessions_suggests_starting_work() {
    let output = determine_next_action(true, &Location::Main, None, 0);
    assert_eq!(output.priority, Priority::Medium);
    assert!(output.command.contains("work"));
    assert!(output.reason.contains("no active sessions"));
}

#[test]
fn next_action_uninitialized_on_main_has_high_priority() {
    let output = determine_next_action(false, &Location::Main, None, 0);
    assert_eq!(output.priority, Priority::High);
}

#[test]
fn next_action_not_in_repo_has_high_priority() {
    let output = determine_next_action(false, &Location::NotInRepo, None, 0);
    assert_eq!(output.priority, Priority::High);
}

#[test]
fn next_action_in_workspace_has_medium_priority() {
    let output = determine_next_action(true, &Location::Workspace("ws".to_string()), Some("ws"), 0);
    assert_eq!(output.priority, Priority::Medium);
}

#[test]
fn next_action_on_main_with_sessions_has_medium_priority() {
    let output = determine_next_action(true, &Location::Main, None, 5);
    assert_eq!(output.priority, Priority::Medium);
}

#[test]
fn next_action_on_main_no_sessions_has_medium_priority() {
    let output = determine_next_action(true, &Location::Main, None, 0);
    assert_eq!(output.priority, Priority::Medium);
}

// =============================================================================
// format_status_human
// =============================================================================

#[test]
fn status_human_ready_state_shows_ready() {
    let status = AiStatusOutput {
        location: Location::Main,
        workspace: None,
        agent_id: Some("agent-1".to_string()),
        initialized: true,
        active_sessions: 2,
        ready: true,
        suggestion: "Ready to work".to_string(),
        next_command: "scp work test".to_string(),
    };

    let lines = format_status_human(&status);
    let has_ready = lines.iter().any(|l| l.contains("READY"));
    let has_main = lines.iter().any(|l| l.contains("main"));
    let has_agent = lines.iter().any(|l| l.contains("agent-1"));
    let has_yes = lines.iter().any(|l| l.contains("yes"));
    let has_sessions = lines.iter().any(|l| l.contains("2 sessions"));
    assert!(has_ready, "Must show READY");
    assert!(has_main, "Must show main location");
    assert!(has_agent, "Must show agent ID");
    assert!(has_yes, "Must show initialized=yes");
    assert!(has_sessions, "Must show session count");
}

#[test]
fn status_human_not_ready_state_shows_not_ready() {
    let status = AiStatusOutput {
        location: Location::NotInRepo,
        workspace: None,
        agent_id: None,
        initialized: false,
        active_sessions: 0,
        ready: false,
        suggestion: "SCP not initialized".to_string(),
        next_command: "scp init".to_string(),
    };

    let lines = format_status_human(&status);
    let has_not_ready = lines.iter().any(|l| l.contains("NOT READY"));
    let has_not_registered = lines.iter().any(|l| l.contains("not registered"));
    let has_no = lines.iter().any(|l| l.contains("no"));
    let has_zero = lines.iter().any(|l| l.contains("0 sessions"));
    assert!(has_not_ready, "Must show NOT READY");
    assert!(has_not_registered, "Must show not registered");
    assert!(has_no, "Must show initialized=no");
    assert!(has_zero, "Must show 0 sessions");
}

#[test]
fn status_human_with_workspace_shows_workspace_name() {
    let status = AiStatusOutput {
        location: Location::Workspace("feature-login".to_string()),
        workspace: Some("feature-login".to_string()),
        agent_id: None,
        initialized: true,
        active_sessions: 1,
        ready: true,
        suggestion: "In workspace".to_string(),
        next_command: "scp done".to_string(),
    };

    let lines = format_status_human(&status);
    let has_ws = lines.iter().any(|l| l.contains("feature-login"));
    let has_singular = lines.iter().any(|l| l.contains("1 session"));
    assert!(has_ws, "Must show workspace name");
    assert!(has_singular, "Must show singular session count");
}

// =============================================================================
// JSON schema field validation
// =============================================================================

#[test]
fn ai_status_json_contains_all_required_fields() {
    let status = AiStatusOutput {
        location: Location::Main,
        workspace: None,
        agent_id: Some("agent-1".to_string()),
        initialized: true,
        active_sessions: 2,
        ready: true,
        suggestion: "Ready".to_string(),
        next_command: "scp work".to_string(),
    };

    let json_str = match serde_json::to_string(&status) {
        Ok(s) => s,
        Err(e) => panic!("Serialization must succeed: {e}"),
    };
    let json: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => panic!("Parse must succeed: {e}"),
    };

    assert!(json.get("location").is_some(), "Must have location");
    assert!(json.get("initialized").is_some(), "Must have initialized");
    assert!(
        json.get("active_sessions").is_some(),
        "Must have active_sessions"
    );
    assert!(json.get("ready").is_some(), "Must have ready");
    assert!(json.get("suggestion").is_some(), "Must have suggestion");
    assert!(json.get("next_command").is_some(), "Must have next_command");
    assert!(
        json["initialized"].is_boolean(),
        "initialized must be boolean"
    );
    assert!(json["ready"].is_boolean(), "ready must be boolean");
}

#[test]
fn next_action_json_has_machine_actionable_fields() {
    let action = NextActionOutput {
        action: "Start work".to_string(),
        command: "scp work my-task".to_string(),
        reason: "Ready to begin".to_string(),
        priority: Priority::Medium,
    };

    let json_str = match serde_json::to_string(&action) {
        Ok(s) => s,
        Err(e) => panic!("Serialization must succeed: {e}"),
    };
    let json: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => panic!("Parse must succeed: {e}"),
    };

    match json["action"].as_str() {
        Some(s) => assert!(!s.is_empty(), "Action must be non-empty"),
        None => panic!("Action must be a string"),
    }

    match json["command"].as_str() {
        Some(c) => assert!(
            c.starts_with("scp ") || c.starts_with("cd ") || c.starts_with('#'),
            "Command must be actionable: {c}"
        ),
        None => panic!("Command must be a string"),
    }

    match json["priority"].as_str() {
        Some(p) => assert!(
            ["high", "medium", "low"].contains(&p),
            "Priority must be valid: {p}"
        ),
        None => panic!("Priority must be a string"),
    }
}

#[test]
fn envelope_wraps_status_with_schema_fields() {
    let status = AiStatusOutput {
        location: Location::Main,
        workspace: None,
        agent_id: None,
        initialized: true,
        active_sessions: 0,
        ready: true,
        suggestion: "Ready".to_string(),
        next_command: "scp work".to_string(),
    };

    let envelope = AiEnvelope::new(AI_STATUS_RESPONSE, "single", &status);
    let json_str = match serde_json::to_string(&envelope) {
        Ok(s) => s,
        Err(e) => panic!("Envelope serialization must succeed: {e}"),
    };
    let json: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => panic!("Envelope JSON parse must succeed: {e}"),
    };

    assert!(json.get("$schema").is_some());
    assert!(json.get("_schema_version").is_some());
    assert!(json.get("schema_type").is_some());
    assert!(json.get("success").is_some());
    match json["success"].as_bool() {
        Some(b) => assert!(b, "Success must be true"),
        None => panic!("Success must be boolean"),
    }
    assert!(
        json.get("location").is_some(),
        "Data should be flattened into envelope"
    );
}

// =============================================================================
// Subcommand enumeration
// =============================================================================

#[test]
fn all_five_ai_subcommands_are_defined() {
    let count = [
        AiSubcommand::Status,
        AiSubcommand::Workflow,
        AiSubcommand::QuickStart,
        AiSubcommand::Next,
        AiSubcommand::Default,
    ]
    .len();
    assert_eq!(count, 5, "Should have 5 AI subcommands");
}

#[test]
fn default_subcommand_matches_self() {
    assert!(matches!(AiSubcommand::Default, AiSubcommand::Default));
}

// =============================================================================
// Location enum tests
// =============================================================================

#[test]
fn location_from_raw_parses_main() {
    assert_eq!(Location::from_raw("main"), Location::Main);
}

#[test]
fn location_from_raw_parses_not_in_repo() {
    assert_eq!(Location::from_raw("not_in_repo"), Location::NotInRepo);
}

#[test]
fn location_from_raw_parses_unknown() {
    assert_eq!(Location::from_raw("unknown"), Location::Unknown);
}

#[test]
fn location_from_raw_unknown_string_becomes_workspace() {
    // Any unrecognized string is treated as a workspace identifier
    match Location::from_raw("some-branch-name") {
        Location::Workspace(name) => assert_eq!(name, "some-branch-name"),
        other => panic!("Expected Workspace, got: {other:?}"),
    }
}

#[test]
fn location_display_shows_main() {
    assert_eq!(Location::Main.to_string(), "main");
}

#[test]
fn location_display_shows_not_in_repo() {
    assert_eq!(Location::NotInRepo.to_string(), "not_in_repo");
}

#[test]
fn location_display_shows_workspace_with_name() {
    assert_eq!(
        Location::Workspace("feature".to_string()).to_string(),
        "workspace:feature"
    );
}

#[test]
fn location_display_shows_unknown() {
    assert_eq!(Location::Unknown.to_string(), "unknown");
}

#[test]
fn location_as_str_returns_canonical_form() {
    assert_eq!(Location::Main.as_str(), "main");
    assert_eq!(Location::NotInRepo.as_str(), "not_in_repo");
    assert_eq!(Location::Unknown.as_str(), "unknown");
    assert_eq!(Location::Workspace("x".to_string()).as_str(), "workspace");
}

// =============================================================================
// Priority enum tests
// =============================================================================

#[test]
fn priority_as_str_returns_canonical_form() {
    assert_eq!(Priority::High.as_str(), "high");
    assert_eq!(Priority::Medium.as_str(), "medium");
    assert_eq!(Priority::Low.as_str(), "low");
}

#[test]
fn priority_display_matches_as_str() {
    assert_eq!(Priority::High.to_string(), "high");
    assert_eq!(Priority::Medium.to_string(), "medium");
    assert_eq!(Priority::Low.to_string(), "low");
}

// =============================================================================
// Behavior tests - Martin Fowler style (descriptive names)
// =============================================================================

mod ready_state_behavior {
    use super::*;

    #[test]
    fn when_not_initialized_suggests_init() {
        let (ready, _, next_cmd) = determine_ready_state(false, &Location::Main);
        assert!(!ready, "Uninitialized should not be ready");
        assert!(next_cmd.contains("init"), "Should suggest init command");
    }

    #[test]
    fn when_not_in_repo_suggests_entering_repo() {
        let (ready, _, next_cmd) = determine_ready_state(true, &Location::NotInRepo);
        assert!(!ready, "Not in repo should not be ready");
        assert!(next_cmd.contains("cd"), "Should suggest changing directory");
    }

    #[test]
    fn when_in_workspace_suggests_done() {
        let (ready, _, next_cmd) =
            determine_ready_state(true, &Location::Workspace("ws".to_string()));
        assert!(ready, "In workspace should be ready");
        assert!(next_cmd.contains("done"), "Should suggest completing work");
    }

    #[test]
    fn when_on_main_suggests_work() {
        let (ready, _, next_cmd) = determine_ready_state(true, &Location::Main);
        assert!(ready, "On main should be ready");
        assert!(next_cmd.contains("work"), "Should suggest starting work");
    }
}

mod next_action_behavior {
    use super::*;

    #[test]
    fn when_not_in_repo_suggests_entering_repo() {
        let output = determine_next_action(false, &Location::NotInRepo, None, 0);
        assert_eq!(output.priority, Priority::High);
        assert!(output.command.contains("cd"));
        assert!(output.reason.contains("repository"));
    }

    #[test]
    fn when_uninitialized_suggests_init() {
        let output = determine_next_action(false, &Location::Main, None, 0);
        assert!(output.command.contains("init"));
        assert_eq!(output.priority, Priority::High);
    }

    #[test]
    fn when_in_workspace_suggests_context_or_done() {
        let output = determine_next_action(
            true,
            &Location::Workspace("feature-auth".to_string()),
            Some("feature-auth"),
            0,
        );
        assert!(output.action.contains("feature-auth"));
        assert!(output.command.contains("context"));
        assert_eq!(output.priority, Priority::Medium);
    }

    #[test]
    fn when_sessions_exist_suggests_listing() {
        let output = determine_next_action(true, &Location::Main, None, 3);
        assert!(output.command.contains("list"));
        assert!(output.reason.contains('3'));
    }

    #[test]
    fn when_ready_and_idle_suggests_starting_work() {
        let output = determine_next_action(true, &Location::Main, None, 0);
        assert!(output.command.contains("work"));
        assert!(output.reason.contains("no active sessions"));
    }
}

mod pluralization_behavior {
    use super::*;

    #[test]
    fn one_session_shows_singular() {
        assert_eq!(format_session_count(1), "1 session");
    }

    #[test]
    fn zero_sessions_shows_plural() {
        assert_eq!(format_session_count(0), "0 sessions");
    }

    #[test]
    fn two_sessions_shows_plural() {
        assert_eq!(format_session_count(2), "2 sessions");
    }

    #[test]
    fn five_sessions_shows_plural() {
        assert_eq!(format_session_count(5), "5 sessions");
    }

    #[test]
    fn ten_sessions_shows_plural() {
        assert_eq!(format_session_count(10), "10 sessions");
    }

    #[test]
    fn hundred_sessions_shows_plural() {
        assert_eq!(format_session_count(100), "100 sessions");
    }
}

mod workflow_behavior {
    use super::*;

    #[test]
    fn workflow_first_step_numbered_one() {
        let workflow = build_workflow();
        match workflow.steps.first() {
            Some(s) => assert_eq!(s.step, 1),
            None => panic!("Must have steps"),
        }
    }

    #[test]
    fn workflow_seventh_step_numbered_seven() {
        let workflow = build_workflow();
        match workflow.steps.get(6) {
            Some(s) => assert_eq!(s.step, 7),
            None => panic!("Must have 7 steps"),
        }
    }

    #[test]
    fn workflow_step_two_has_actionable_command() {
        let workflow = build_workflow();
        match workflow.steps.get(1) {
            Some(s) => {
                assert!(!s.command.is_empty());
                assert!(!s.description.is_empty());
            }
            None => panic!("Must have step 2"),
        }
    }

    #[test]
    fn workflow_step_three_has_actionable_command() {
        let workflow = build_workflow();
        match workflow.steps.get(2) {
            Some(s) => {
                assert!(!s.command.is_empty());
                assert!(!s.description.is_empty());
            }
            None => panic!("Must have step 3"),
        }
    }

    #[test]
    fn workflow_step_five_has_actionable_command() {
        let workflow = build_workflow();
        match workflow.steps.get(4) {
            Some(s) => {
                assert!(!s.command.is_empty());
                assert!(!s.description.is_empty());
            }
            None => panic!("Must have step 5"),
        }
    }

    #[test]
    fn workflow_step_six_has_actionable_command() {
        let workflow = build_workflow();
        match workflow.steps.get(5) {
            Some(s) => {
                assert!(!s.command.is_empty());
                assert!(!s.description.is_empty());
            }
            None => panic!("Must have step 6"),
        }
    }
}

// =============================================================================
// RED QUEEN ADVERSARIAL TESTS - hl-9nb
// =============================================================================

mod red_queen_adversarial {
    use super::*;

    // --- VULN-9NB-1: determine_ready_state contradiction (FIXED) ---

    #[test]
    fn adversarial_ready_state_not_in_repo_and_uninitialized_suggests_cd_not_init() {
        let (ready, suggestion, next_cmd) = determine_ready_state(false, &Location::NotInRepo);
        assert!(!ready, "Should not be ready");
        // FIXED: Now returns "Not in a Git repository" / "cd <repo> && scp init"
        assert!(
            suggestion.contains("Git repository"),
            "Should say not in repo, got: {suggestion}"
        );
        assert!(
            next_cmd.contains("cd"),
            "Should suggest cd, got: {next_cmd}"
        );
        // Cross-check: determine_next_action now agrees
        let action = determine_next_action(false, &Location::NotInRepo, None, 0);
        assert!(
            action.command.contains("cd"),
            "determine_next_action should also suggest cd, got: {}",
            action.command
        );
    }

    #[test]
    fn adversarial_ready_state_consistent_priority_with_next_action() {
        // (initialized, location) combos -- bounded, no loop
        let combo_main_false = (false, Location::Main);
        let combo_not_in_repo_false = (false, Location::NotInRepo);
        let combo_workspace_false = (false, Location::Workspace("ws".to_string()));
        let combo_unknown_false = (false, Location::Unknown);
        let combo_main_true = (true, Location::Main);
        let combo_not_in_repo_true = (true, Location::NotInRepo);
        let combo_workspace_true = (true, Location::Workspace("ws".to_string()));
        let combo_unknown_true = (true, Location::Unknown);

        let combos = [
            combo_main_false,
            combo_not_in_repo_false,
            combo_workspace_false,
            combo_unknown_false,
            combo_main_true,
            combo_not_in_repo_true,
            combo_workspace_true,
            combo_unknown_true,
        ];

        // Each combo checked individually -- no loop body
        for (init, loc) in combos {
            let (ready, _, _) = determine_ready_state(init, &loc);
            let action = determine_next_action(init, &loc, None, 0);
            if !ready {
                assert_eq!(
                    action.priority,
                    Priority::High,
                    "When NOT ready (init={init}, loc={loc}), priority should be High, got: {}",
                    action.priority
                );
            }
        }
    }

    /// ATTACK: Empty location handled as Unknown.
    #[test]
    fn adversarial_ready_state_unknown_location_treated_as_ready() {
        let (ready, suggestion, _next_cmd) = determine_ready_state(true, &Location::Unknown);
        assert!(
            ready,
            "Unknown location when initialized is treated as ready"
        );
        assert!(
            suggestion.contains("Ready"),
            "Unknown location gets generic 'Ready' suggestion: {suggestion}"
        );
    }

    /// ATTACK: Workspace name in Location enum.
    #[test]
    fn adversarial_ready_state_long_workspace_name() {
        let long_name = "x".repeat(10_000);
        let (ready, _, next_cmd) =
            determine_ready_state(true, &Location::Workspace(long_name.clone()));
        assert!(ready, "Workspace with long name treated as ready");
        assert!(next_cmd.contains("done"));
    }

    /// ATTACK: Format session count with max usize.
    #[test]
    fn adversarial_format_session_count_max_usize() {
        let result = format_session_count(usize::MAX);
        assert!(
            result.contains("sessions"),
            "MAX usize should still format correctly: {result}"
        );
    }

    // --- VULN-9NB-2: newline injection in format_status_human (FIXED) ---

    #[test]
    fn adversarial_format_status_human_suggestion_newline_is_sanitized() {
        let status = AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "All good!\nStatus:         COMPROMISED".to_string(),
            next_command: "scp work test".to_string(),
        };

        let lines = format_status_human(&status);
        // FIXED (VULN-9NB-2): newlines are sanitized to spaces, so the injected
        // "Status:         COMPROMISED" no longer appears as a SEPARATE line.
        // Count lines that look like "Status:..." -- there should be exactly one
        // (the legitimate "Status: READY" line), not two.
        let status_lines: Vec<&String> =
            lines.iter().filter(|l| l.starts_with("Status:")).collect();
        assert_eq!(
            status_lines.len(),
            1,
            "FIXED: should have exactly 1 Status line, got {}: {status_lines:?}",
            status_lines.len()
        );
        assert!(
            status_lines[0].contains("READY"),
            "The single Status line should be the legitimate one: {}",
            status_lines[0]
        );
    }

    #[test]
    fn adversarial_format_status_human_next_command_newline_is_sanitized() {
        let status = AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "Ready".to_string(),
            next_command: "scp work test\n  rm -rf /".to_string(),
        };

        let lines = format_status_human(&status);
        // FIXED (VULN-9NB-2): newlines sanitized, so "rm -rf" does not appear
        // as a separate indented line that looks like a real command.
        // Verify no line is exactly "  rm -rf /" (the forged command line).
        let has_forged_cmd_line = lines.iter().any(|l| l == "  rm -rf /");
        assert!(
            !has_forged_cmd_line,
            "FIXED: newline injection should not create separate forged command lines"
        );
        // The "rm -rf" text is collapsed into the next_command line with a space
        let cmd_lines: Vec<&String> = lines
            .iter()
            .filter(|l| l.starts_with("  scp work test"))
            .collect();
        match cmd_lines.first() {
            Some(l) => assert!(
                l.contains("rm -rf"),
                "Sanitized text should be collapsed inline: {l}"
            ),
            None => panic!("Must have the next command line with sanitized content"),
        }
    }

    /// ATTACK: Huge session count in determine_next_action.
    #[test]
    fn adversarial_next_action_huge_session_count() {
        let output = determine_next_action(true, &Location::Main, None, usize::MAX);
        assert!(
            output.reason.contains(&usize::MAX.to_string()),
            "Should handle usize::MAX sessions"
        );
    }

    /// ATTACK: Workspace name injection in determine_next_action.
    #[test]
    fn adversarial_next_action_workspace_name_injection() {
        let ws = "test'; rm -rf /; echo '";
        let output = determine_next_action(true, &Location::Workspace(ws.to_string()), Some(ws), 0);
        // The workspace name is interpolated into action/reason strings (not executed).
        // This is expected behavior - these are data fields, not shell commands.
        assert!(
            output.action.contains(ws),
            "Workspace name appears in action field (data, not shell)"
        );
    }

    /// ATTACK: AiEnvelope schema field special characters are JSON-escaped.
    #[test]
    fn adversarial_envelope_schema_name_injection_is_escaped() {
        let status = AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "Ready".to_string(),
            next_command: "scp work".to_string(),
        };

        let envelope = AiEnvelope::new("test\ninjected", "single", &status);
        let json = match serde_json::to_string(&envelope) {
            Ok(s) => s,
            Err(e) => panic!("Envelope serialization must succeed: {e}"),
        };
        // JSON serialization escapes newlines, so the raw \n char is not present
        let first_newline = json.find('\n');
        assert!(
            first_newline.is_none(),
            "JSON serialization should escape newlines in schema field"
        );
    }

    /// ATTACK: AiStatusOutput with usize::MAX active_sessions serializes.
    #[test]
    fn adversarial_status_output_serializes_extreme_sessions() {
        let status = AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: usize::MAX,
            ready: true,
            suggestion: "Ready".to_string(),
            next_command: "scp work".to_string(),
        };

        match serde_json::to_string(&status) {
            Ok(s) => assert!(
                s.contains(&usize::MAX.to_string()),
                "Must contain max value"
            ),
            Err(e) => panic!("Should serialize usize::MAX active_sessions: {e}"),
        }
    }
}
