// //! Builder tests
//!
//! Integration tests for all builders.

use std::path::PathBuf;

use crate::domain::builders::action::ActionBuilder;
use crate::domain::builders::agent_info::{AgentInfoBuilder, AgentState};
use crate::domain::builders::errors::BuilderError;
use crate::domain::builders::issue::{IssueBuilder, IssueKind};
use crate::domain::builders::plan::PlanBuilder;
use crate::domain::builders::session_output::SessionOutputBuilder;
use crate::domain::builders::summary::SummaryBuilder;
use crate::domain::builders::workspace_info::{WorkspaceInfoBuilder, WorkspaceInfoState};
use crate::output_jsonl::{
    domain_types::{IssueId, IssueTitle, Message, PlanDescription, PlanTitle},
    ActionStatus, ActionTarget, ActionVerb, IssueSeverity, OutputSummaryType,
};
use crate::types::SessionStatus as TypesSessionStatus;
use crate::WorkspaceState as TypesWorkspaceState;

#[test]
fn test_session_output_builder_complete() {
    let result = SessionOutputBuilder::new()
        .name("test-session")
        .expect("valid name")
        .status(TypesSessionStatus::Active)
        .state(TypesWorkspaceState::Working)
        .workspace_path("/tmp/workspace")
        .expect("valid path")
        .build();

    assert!(result.is_ok());
    let session = result.expect("valid session");
    assert_eq!(session.name, "test-session");
}

#[test]
fn test_session_output_builder_missing_required() {
    let result = SessionOutputBuilder::new()
        .name("test-session")
        .expect("valid name")
        .status(TypesSessionStatus::Active)
        // Missing state and workspace_path
        .build();

    assert!(result.is_err());
    match result.expect_err("expected error") {
        BuilderError::MissingRequired { field } => {
            assert!(field == "state" || field == "workspace_path");
        }
        _ => panic!("expected MissingRequired error"),
    }
}

#[test]
fn test_session_output_builder_invalid_name() {
    let result = SessionOutputBuilder::new()
        .name("") // Empty name
        .expect_err("should fail");

    match result {
        BuilderError::InvalidValue { field, .. } => {
            assert_eq!(field, "name");
        }
        _ => panic!("expected InvalidValue error"),
    }
}

#[test]
fn test_session_output_builder_relative_path() {
    let result = SessionOutputBuilder::new()
        .name("test-session")
        .expect("valid name")
        .status(TypesSessionStatus::Active)
        .state(TypesWorkspaceState::Working)
        .workspace_path("relative/path") // Not absolute
        .expect_err("should fail");

    match result {
        BuilderError::InvalidValue { field, .. } => {
            assert_eq!(field, "workspace_path");
        }
        _ => panic!("expected InvalidValue error"),
    }
}

#[test]
fn test_issue_builder_complete() {
    let id = IssueId::new("issue-1".to_string()).expect("valid id");
    let title = IssueTitle::new("Test issue").expect("valid title");

    let result = IssueBuilder::new()
        .id(id)
        .title(title)
        .kind(IssueKind::Validation)
        .severity(IssueSeverity::Error)
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_issue_builder_with_suggestion() {
    let id = IssueId::new("issue-2".to_string()).expect("valid id");
    let title = IssueTitle::new("Another issue").expect("valid title");

    let result = IssueBuilder::new()
        .id(id)
        .title(title)
        .kind(IssueKind::Configuration)
        .severity(IssueSeverity::Warning)
        .suggestion("Fix the config".to_string())
        .build();

    assert!(result.is_ok());
    let issue = result.expect("valid issue");
    assert_eq!(issue.suggestion, Some("Fix the config".to_string()));
}

#[test]
fn test_plan_builder_with_steps() {
    let title = PlanTitle::new("Test plan").expect("valid title");
    let description = PlanDescription::new("Plan description").expect("valid description");

    let result = PlanBuilder::new()
        .title(title)
        .description(description)
        .with_step("Step 1", ActionStatus::Pending)
        .expect("valid step")
        .with_step("Step 2", ActionStatus::Completed)
        .expect("valid step")
        .build();

    assert!(result.is_ok());
    let plan = result.expect("valid plan");
    assert_eq!(plan.steps.len(), 2);
}

#[test]
fn test_summary_builder_complete() {
    let message = Message::new("Test message").expect("valid message");

    let result = SummaryBuilder::new()
        .type_field(OutputSummaryType::Info)
        .message(message)
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_action_builder_complete() {
    let verb = ActionVerb::Run;
    let target = ActionTarget::new("test-target").expect("valid target");

    let result = ActionBuilder::new()
        .verb(verb)
        .target(target)
        .status(ActionStatus::Pending)
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_action_builder_with_result() {
    let verb = ActionVerb::Execute;
    let target = ActionTarget::new("another-target").expect("valid target");

    let result = ActionBuilder::new()
        .verb(verb)
        .target(target)
        .status(ActionStatus::Completed)
        .with_completed_result("Success!")
        .build();

    assert!(result.is_ok());
    let action = result.expect("valid action");
    assert!(matches!(
        action.result,
        crate::output_jsonl::ActionResult::Completed { .. }
    ));
}

#[test]
fn test_agent_info_builder_complete() {
    let id = crate::domain::AgentId::parse("test-agent").expect("valid agent id");

    let result = AgentInfoBuilder::new()
        .id(id)
        .state(AgentState::Active)
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_workspace_info_builder_complete() {
    let path = PathBuf::from("/tmp/workspace");

    let result = WorkspaceInfoBuilder::new()
        .path(path.clone())
        .state(WorkspaceInfoState::Active)
        .build();

    assert!(result.is_ok());
    let info = result.expect("valid info");
    assert_eq!(info.path, path);
}
