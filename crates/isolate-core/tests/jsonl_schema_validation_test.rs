#![allow(clippy::uninlined_format_args, clippy::doc_markdown)]
//! Integration test to verify JSONL output types match CUE schemas
//!
//! This test validates that all Rust types serialize correctly to JSON
//! and include required fields per the CUE schema specifications in:
//! - .beads/beads/isolate-20260217-001-jsonl-core-types.cue

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use isolate_core::{domain::SessionName, output::*};
use serde_json::Value;

fn validate_json_structure(json: &Value, required_fields: &[&str]) {
    for field in required_fields {
        assert!(
            json.get(field).is_some(),
            "Missing required field '{}' in JSON: {}",
            field,
            json
        );
    }
}

#[test]
fn test_summary_serialization() {
    let summary = Summary::new(
        SummaryType::Status,
        Message::new("Test summary").expect("valid"),
    )
    .expect("valid")
    .with_details("Test details".to_string());

    let json = serde_json::to_value(&summary).unwrap();

    // Verify type field
    assert!(json.get("type").is_some(), "Summary must have 'type' field");
    validate_json_structure(&json, &["type", "message", "timestamp"]);
    assert!(json.get("details").is_some());
}

#[test]
fn test_session_output_serialization() {
    use isolate_core::{types::SessionStatus, WorkspaceState};

    let session = SessionOutput::new(
        "test-session".to_string(),
        SessionStatus::Active,
        WorkspaceState::Working,
        std::path::PathBuf::from("/tmp/test"),
    )
    .unwrap()
    .with_branch("main".to_string());

    let json = serde_json::to_value(&session).unwrap();

    validate_json_structure(
        &json,
        &[
            "name",
            "status",
            "state",
            "workspace_path",
            "created_at",
            "updated_at",
        ],
    );
    assert!(json.get("branch").is_some());
}

#[test]
fn test_issue_serialization() {
    let issue = Issue::new(
        IssueId::new("TEST-001").expect("valid"),
        IssueTitle::new("Test issue").expect("valid"),
        IssueKind::Validation,
        IssueSeverity::Error,
    )
    .expect("valid")
    .with_session(SessionName::parse("test-session").expect("valid"))
    .with_suggestion("Fix it".to_string());

    let json = serde_json::to_value(&issue).unwrap();

    validate_json_structure(&json, &["id", "title", "kind", "severity"]);
    assert!(json.get("scope").is_some());
    assert!(json.get("suggestion").is_some());
}

#[test]
fn test_plan_serialization() {
    let mut plan = Plan::new(
        PlanTitle::new("Test plan").expect("valid"),
        PlanDescription::new("Test description").expect("valid"),
    )
    .expect("valid");
    plan = plan
        .with_step("Step 1".to_string(), ActionStatus::Pending)
        .expect("valid");
    plan = plan
        .with_step("Step 2".to_string(), ActionStatus::Completed)
        .expect("valid");

    let json = serde_json::to_value(&plan).unwrap();

    validate_json_structure(&json, &["title", "description", "steps", "created_at"]);

    // Verify steps structure
    let steps = json.get("steps").unwrap().as_array().unwrap();
    assert_eq!(steps.len(), 2);
    validate_json_structure(&steps[0], &["order", "description", "status"]);
}

#[test]
fn test_conflict_detail_serialization() {
    let conflict = ConflictDetail::overlapping("test.txt");

    let json = serde_json::to_value(&conflict).unwrap();

    validate_json_structure(
        &json,
        &["file", "conflict_type", "resolutions", "recommended"],
    );

    // Verify resolutions is an array
    let resolutions = json.get("resolutions").unwrap().as_array().unwrap();
    assert!(!resolutions.is_empty());

    // Each resolution should have: strategy, description, risk, automatic
    for resolution in resolutions {
        validate_json_structure(
            resolution,
            &["strategy", "description", "risk", "automatic"],
        );
    }
}

#[test]
fn test_conflict_analysis_serialization() {
    let output_line = OutputLine::conflict_analysis(
        "test-session",
        true,
        vec![
            ConflictDetail::overlapping("file1.txt"),
            ConflictDetail::existing("file2.txt"),
        ],
    );

    let json = serde_json::to_value(&output_line).unwrap();
    let analysis = json
        .get("conflict_analysis")
        .expect("OutputLine::ConflictAnalysis should serialize as wrapped object");

    // Verify outer structure
    validate_json_structure(
        analysis,
        &[
            "type",
            "session",
            "merge_safe",
            "total_conflicts",
            "conflicts",
            "timestamp",
        ],
    );

    // Verify conflicts array
    let conflicts = analysis.get("conflicts").unwrap().as_array().unwrap();
    assert_eq!(conflicts.len(), 2);

    // Verify specific conflict types
    assert_eq!(
        conflicts[0].get("file").unwrap().as_str().unwrap(),
        "file1.txt"
    );
    assert_eq!(
        conflicts[1].get("file").unwrap().as_str().unwrap(),
        "file2.txt"
    );
}

#[test]
fn test_output_line_enum_discriminator() {
    // All OutputLine variants should serialize as a single-key wrapper
    use isolate_core::{types::SessionStatus, WorkspaceState};

    let variants = vec![
        OutputLine::Summary(
            Summary::new(SummaryType::Info, Message::new("Test").expect("valid")).expect("valid"),
        ),
        OutputLine::Session(
            SessionOutput::new(
                "test".to_string(),
                SessionStatus::Active,
                WorkspaceState::Working,
                std::path::PathBuf::from("/tmp/test"),
            )
            .unwrap(),
        ),
        OutputLine::Issue(
            Issue::new(
                IssueId::new("TEST").expect("valid"),
                IssueTitle::new("Test").expect("valid"),
                IssueKind::Validation,
                IssueSeverity::Warning,
            )
            .unwrap(),
        ),
        OutputLine::Plan(
            Plan::new(
                PlanTitle::new("Test").expect("valid"),
                PlanDescription::new("Desc").expect("valid"),
            )
            .unwrap(),
        ),
        OutputLine::Action(Action::new(
            ActionVerb::new("test").expect("valid action verb"),
            ActionTarget::new("target").expect("valid action target"),
            ActionStatus::Pending,
        )),
        OutputLine::Warning(
            Warning::new(
                WarningCode::new("TEST").expect("valid warning code"),
                Message::new("Test warning").expect("valid"),
            )
            .expect("valid"),
        ),
        OutputLine::Result(
            ResultOutput::success(
                ResultKind::Command,
                Message::new("Test result").expect("valid"),
            )
            .expect("valid"),
        ),
    ];

    let allowed_wrappers = [
        "summary", "session", "issue", "plan", "action", "warning", "result",
    ];

    for variant in variants {
        let json = serde_json::to_value(&variant).unwrap();
        let obj = json
            .as_object()
            .expect("OutputLine should serialize as a JSON object");

        assert_eq!(
            obj.len(),
            1,
            "OutputLine should have exactly one wrapper key: {}",
            json
        );

        let wrapper = obj
            .keys()
            .next()
            .expect("OutputLine object should have one key");

        assert!(
            allowed_wrappers.contains(&wrapper.as_str()),
            "OutputLine used unexpected wrapper key '{}': {}",
            wrapper,
            json,
        );
    }
}

#[test]
fn test_enum_value_serialization() {
    let issue = Issue::new(
        IssueId::new("TEST").expect("valid"),
        IssueTitle::new("Test").expect("valid"),
        IssueKind::Validation,
        IssueSeverity::Error,
    )
    .unwrap();

    let json = serde_json::to_value(&issue).unwrap();

    // kind should be lowercase (snake_case from serde(rename_all = "snake_case"))
    assert_eq!(json.get("kind").unwrap().as_str().unwrap(), "validation");

    // severity should be lowercase (snake_case from serde(rename_all = "snake_case"))
    assert_eq!(json.get("severity").unwrap().as_str().unwrap(), "error");
}

#[test]
fn test_timestamp_format() {
    let summary = Summary::new(SummaryType::Status, Message::new("Test").expect("valid")).unwrap();
    let json = serde_json::to_value(&summary).unwrap();

    // timestamp should be a number (milliseconds since epoch)
    let timestamp = json.get("timestamp").unwrap();
    assert!(timestamp.is_number());

    // Should be relatively recent (after 2020)
    let ts = timestamp.as_i64().unwrap();
    assert!(ts > 1_577_836_800_000); // Jan 1, 2020 in milliseconds
}

#[test]
fn test_recovery_type_serialization() {
    use isolate_core::output::{ErrorSeverity, RecoveryCapability};

    let recovery = Recovery::new(
        IssueId::new("ISSUE-001").expect("valid"),
        Assessment {
            severity: ErrorSeverity::High,
            capability: RecoveryCapability::Recoverable {
                recommended_action: "Fix the issue".to_string(),
            },
        },
    )
    .with_action("Step 1".to_string(), Some("cmd1".to_string()), true)
    .unwrap();

    let json = serde_json::to_value(&recovery).unwrap();

    validate_json_structure(&json, &["issue_id", "assessment", "actions"]);

    // Verify assessment structure
    let assessment = json.get("assessment").unwrap();
    validate_json_structure(assessment, &["severity", "capability"]);

    // Verify capability has the right structure
    let capability = assessment.get("capability").unwrap();
    // In our manual structure check, we need to match how serde serializes the RecoveryCapability
    // enum
    assert!(capability.get("recoverable").is_some());
}
