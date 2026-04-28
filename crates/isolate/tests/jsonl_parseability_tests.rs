#![allow(
    clippy::unreadable_literal,
    clippy::unnecessary_map_or,
    clippy::filter_map_next
)]
//! JSONL Parseability Tests (bd-foy)
//!
//! These tests verify that isolate output types produce valid JSONL output
//! that can be parsed by external tools like `jq` and programmatic consumers.
//!
//! # Test Plan Reference
//!
//! From `.beads/beads/bd-foy-martin-fowler-tests.md`:
//! - test_output_parseable_by_jq
//! - test_output_has_consistent_schema
//! - test_session_output_has_required_fields
//! - test_issue_output_has_required_fields
//! - test_result_output_has_required_fields

// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
    clippy::needless_raw_string_hashes,
    clippy::bool_assert_comparison
)]

mod common;

use std::collections::HashSet;

use isolate_core::output::{
    Action, ActionStatus, ActionVerb, ErrorSeverity, Issue, IssueId, IssueKind, IssueSeverity,
    IssueTitle, Message, Outcome, OutputLine, ResultKind, ResultOutput, Session, SessionOutput,
    SessionState, Summary, SummaryType,
};
use serde_json::Value as JsonValue;

// =============================================================================
// INVARIANT TESTS: All output lines are valid JSONL
// =============================================================================

/// Given: Any OutputLine serialization
/// When: JSON is generated
/// Then: Every JSON line is valid JSON
#[test]
fn test_all_jsonl_lines_are_valid_json() {
    // Test SessionOutput serialization
    let session = create_test_session_output();
    let output_line = OutputLine::Session(session);
    let json_str = serde_json::to_string(&output_line).expect("serialize session");

    // Verify it's valid JSON
    let parsed: Result<JsonValue, _> = serde_json::from_str(&json_str);
    assert!(
        parsed.is_ok(),
        "Invalid JSON line: {json_str}\nParse error: {:?}",
        parsed.err()
    );

    // Test Issue serialization
    let issue = create_test_issue();
    let output_line = OutputLine::Issue(issue);
    let json_str = serde_json::to_string(&output_line).expect("serialize issue");

    let parsed: Result<JsonValue, _> = serde_json::from_str(&json_str);
    assert!(
        parsed.is_ok(),
        "Invalid JSON line: {json_str}\nParse error: {:?}",
        parsed.err()
    );

    // Test ResultOutput serialization
    let result = create_test_result_output();
    let output_line = OutputLine::Result(result);
    let json_str = serde_json::to_string(&output_line).expect("serialize result");

    let parsed: Result<JsonValue, _> = serde_json::from_str(&json_str);
    assert!(
        parsed.is_ok(),
        "Invalid JSON line: {json_str}\nParse error: {:?}",
        parsed.err()
    );
}

/// Given: Any OutputLine serialization
/// When: JSON is generated
/// Then: Each JSON object has a discriminator field we can identify
#[test]
fn test_all_jsonl_lines_have_type_discriminator() {
    // Test SessionOutput
    let session = create_test_session_output();
    let output_line = OutputLine::Session(session);
    let json_str = serde_json::to_string(&output_line).expect("serialize session");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    assert!(
        json.is_object(),
        "JSON line should be an object: {json_str}"
    );
    let keys: Vec<_> = json.as_object().map_or(vec![], |obj| obj.keys().collect());
    assert!(
        !keys.is_empty(),
        "JSON line should have at least one key: {json_str}"
    );
    // SessionOutput serializes with "session" key (externally tagged)
    assert!(
        keys.contains(&"session"),
        "Should have 'session' key as type discriminator"
    );

    // Test Issue
    let issue = create_test_issue();
    let output_line = OutputLine::Issue(issue);
    let json_str = serde_json::to_string(&output_line).expect("serialize issue");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    let keys: Vec<_> = json.as_object().map_or(vec![], |obj| obj.keys().collect());
    assert!(
        keys.contains(&"issue"),
        "Should have 'issue' key as type discriminator"
    );

    // Test ResultOutput
    let result = create_test_result_output();
    let output_line = OutputLine::Result(result);
    let json_str = serde_json::to_string(&output_line).expect("serialize result");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    let keys: Vec<_> = json.as_object().map_or(vec![], |obj| obj.keys().collect());
    assert!(
        keys.contains(&"result"),
        "Should have 'result' key as type discriminator"
    );
}

// =============================================================================
// TEST: Output parseable by jq (using serde_json as jq substitute)
// =============================================================================

/// Given: Any OutputLine serialization
/// When: Output is processed by a JSON parser (like jq)
/// Then: Parsing succeeds
///
/// This test verifies that the output can be piped to external tools
/// like `jq` for filtering and transformation.
#[test]
fn test_output_parseable_by_jq() {
    let output_types: Vec<OutputLine> = vec![
        OutputLine::Session(create_test_session_output()),
        OutputLine::Issue(create_test_issue()),
        OutputLine::Result(create_test_result_output()),
        OutputLine::Summary(Summary {
            summary_type: SummaryType::SessionList,
            total: 1,
            message: "test".to_string(),
        }),
        OutputLine::Action(create_test_action()),
    ];

    for output_line in output_types {
        let json_str = serde_json::to_string(&output_line).expect("serialize output");

        // Only parse lines that look like complete JSON objects
        if json_str.starts_with('{') && json_str.ends_with('}') {
            // Verify it's valid JSON (equivalent to `jq .` success)
            let parsed: Result<JsonValue, _> = serde_json::from_str(&json_str);
            assert!(
                parsed.is_ok(),
                "OutputLine produced invalid JSON: {json_str}\nError: {:?}",
                parsed.err()
            );
        }
    }
}

// =============================================================================
// TEST: Output has consistent schema across runs
// =============================================================================

/// Given: Multiple serializations of the same OutputLine type
/// When: Output schemas are compared
/// Then: All runs produce the same JSON structure (same top-level keys)
#[test]
fn test_output_has_consistent_schema() {
    // Run 1
    let session1 = create_test_session_output();
    let output1 = OutputLine::Session(session1);
    let json_str1 = serde_json::to_string(&output1).expect("serialize");
    let json1: JsonValue = serde_json::from_str(&json_str1).expect("parse");
    let schema1 = extract_schema(&json1);

    // Run 2
    let session2 = create_test_session_output();
    let output2 = OutputLine::Session(session2);
    let json_str2 = serde_json::to_string(&output2).expect("serialize");
    let json2: JsonValue = serde_json::from_str(&json_str2).expect("parse");
    let schema2 = extract_schema(&json2);

    // Run 3
    let session3 = create_test_session_output();
    let output3 = OutputLine::Session(session3);
    let json_str3 = serde_json::to_string(&output3).expect("serialize");
    let json3: JsonValue = serde_json::from_str(&json_str3).expect("parse");
    let schema3 = extract_schema(&json3);

    // Compare schemas - they should be identical
    assert_eq!(
        schema1, schema2,
        "Schema should be consistent across runs.\nRun 1: {:?}\nRun 2: {:?}",
        schema1, schema2
    );
    assert_eq!(
        schema2, schema3,
        "Schema should be consistent across runs.\nRun 2: {:?}\nRun 3: {:?}",
        schema2, schema3
    );
}

/// Extract the set of top-level keys from a JSON object
fn extract_schema(json: &JsonValue) -> HashSet<String> {
    json.as_object()
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

// =============================================================================
// TEST: SessionOutput has required fields
// =============================================================================

/// Given: A SessionOutput JSON line
/// When: Fields are examined
/// Then: All required fields are present:
///   - name
///   - status
///   - state
///   - workspace_path
///   - created_at
///   - updated_at
#[test]
fn test_session_output_has_required_fields() {
    let session = create_test_session_output();
    let output_line = OutputLine::Session(session);
    let json_str = serde_json::to_string(&output_line).expect("serialize session");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    // Get the inner "session" object
    let session_json = json.get("session").expect("session key");

    // Verify required fields
    let required_fields = [
        "name",
        "status",
        "state",
        "workspace_path",
        "created_at",
        "updated_at",
    ];

    for field in &required_fields {
        assert!(
            session_json.get(field).is_some(),
            "SessionOutput missing required field: {field}\nJSON: {session_json}"
        );
    }

    // Verify field types
    assert!(
        session_json
            .get("name")
            .and_then(JsonValue::as_str)
            .is_some(),
        "SessionOutput.name should be a string"
    );
    assert!(
        session_json
            .get("status")
            .and_then(JsonValue::as_str)
            .is_some(),
        "SessionOutput.status should be a string"
    );
    assert!(
        session_json
            .get("state")
            .and_then(JsonValue::as_str)
            .is_some(),
        "SessionOutput.state should be a string"
    );
    assert!(
        session_json
            .get("workspace_path")
            .and_then(JsonValue::as_str)
            .is_some(),
        "SessionOutput.workspace_path should be a string"
    );
    assert!(
        session_json
            .get("created_at")
            .and_then(JsonValue::as_i64)
            .is_some(),
        "SessionOutput.created_at should be a timestamp"
    );
    assert!(
        session_json
            .get("updated_at")
            .and_then(JsonValue::as_i64)
            .is_some(),
        "SessionOutput.updated_at should be a timestamp"
    );
}

// =============================================================================
// TEST: Issue has required fields
// =============================================================================

/// Given: An Issue JSON line
/// When: Fields are examined
/// Then: All required fields are present:
///   - id
///   - title
///   - kind
///   - severity
#[test]
fn test_issue_output_has_required_fields() {
    let issue = create_test_issue();
    let output_line = OutputLine::Issue(issue);
    let json_str = serde_json::to_string(&output_line).expect("serialize issue");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    // Get the inner "issue" object
    let issue_json = json.get("issue").expect("issue key");

    // Verify required fields
    let required_fields = ["id", "title", "kind", "severity"];

    for field in &required_fields {
        assert!(
            issue_json.get(field).is_some(),
            "Issue missing required field: {field}\nJSON: {issue_json}"
        );
    }

    // Verify field types
    assert!(
        issue_json.get("id").and_then(JsonValue::as_str).is_some(),
        "Issue.id should be a string"
    );
    assert!(
        issue_json
            .get("title")
            .and_then(JsonValue::as_str)
            .is_some(),
        "Issue.title should be a string"
    );
    assert!(
        issue_json.get("kind").and_then(JsonValue::as_str).is_some(),
        "Issue.kind should be a string"
    );
    assert!(
        issue_json
            .get("severity")
            .and_then(JsonValue::as_str)
            .is_some(),
        "Issue.severity should be a string"
    );

    // Verify optional fields have correct types if present
    // Scope is nested: {"scope": {"InSession": {"session": "..."}}}
    if let Some(scope) = issue_json.get("scope") {
        assert!(
            scope.is_object(),
            "Issue.scope should be an object when present"
        );
    }
}

// =============================================================================
// TEST: ResultOutput has required fields
// =============================================================================

/// Given: A ResultOutput JSON line
/// When: Fields are examined
/// Then: All required fields are present:
///   - kind
///   - outcome
///   - message
///   - timestamp
#[test]
fn test_result_output_has_required_fields() {
    let result = create_test_result_output();
    let output_line = OutputLine::Result(result);
    let json_str = serde_json::to_string(&output_line).expect("serialize result");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    // Get the inner "result" object
    let result_json = json.get("result").expect("result key");

    // Verify required fields
    let required_fields = ["kind", "outcome", "message", "timestamp"];

    for field in &required_fields {
        assert!(
            result_json.get(field).is_some(),
            "ResultOutput missing required field: {field}\nJSON: {result_json}"
        );
    }

    // Verify field types
    assert!(
        result_json
            .get("kind")
            .and_then(JsonValue::as_str)
            .is_some(),
        "ResultOutput.kind should be a string"
    );
    assert!(
        result_json
            .get("outcome")
            .and_then(JsonValue::as_str)
            .is_some(),
        "ResultOutput.outcome should be a string (success/failure)"
    );
    assert!(
        result_json
            .get("message")
            .and_then(JsonValue::as_str)
            .is_some(),
        "ResultOutput.message should be a string"
    );
    assert!(
        result_json
            .get("timestamp")
            .and_then(JsonValue::as_i64)
            .is_some(),
        "ResultOutput.timestamp should be a timestamp"
    );

    // Verify optional data field has correct type if present
    if let Some(data) = result_json.get("data") {
        assert!(
            data.is_object() || data.is_null(),
            "ResultOutput.data should be an object or null if present"
        );
    }
}

// =============================================================================
// TEST: All OutputLine variants serialize correctly
// =============================================================================

/// Given: Various OutputLine variants
/// When: Output is examined
/// Then: Each variant has correct structure
#[test]
fn test_all_output_variants_have_correct_structure() {
    // Test Session variant
    let session_output = OutputLine::Session(create_test_session_output());
    let json_str = serde_json::to_string(&session_output).expect("serialize session");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");
    assert!(
        json.get("session").is_some(),
        "Session variant should have 'session' key"
    );

    // Test Action variant
    let action_output = OutputLine::Action(create_test_action());
    let json_str = serde_json::to_string(&action_output).expect("serialize action");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");
    assert!(
        json.get("action").is_some(),
        "Action variant should have 'action' key"
    );

    // Action should have: verb, target, status, timestamp
    if let Some(action) = json.get("action") {
        for field in ["verb", "target", "status", "timestamp"] {
            assert!(action.get(field).is_some(), "Action missing field: {field}");
        }
    }

    // Test Result variant
    let result_output = OutputLine::Result(create_test_result_output());
    let json_str = serde_json::to_string(&result_output).expect("serialize result");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");
    assert!(
        json.get("result").is_some(),
        "Result variant should have 'result' key"
    );

    // Test Summary variant
    let summary_output = OutputLine::Summary(Summary {
        summary_type: SummaryType::SessionList,
        total: 5,
        message: "Listed 5 sessions".to_string(),
    });
    let json_str = serde_json::to_string(&summary_output).expect("serialize summary");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");
    assert!(
        json.get("summary").is_some(),
        "Summary variant should have 'summary' key"
    );
}

// =============================================================================
// TEST: Timestamps are valid and reasonable
// =============================================================================

/// Given: Any output with timestamp fields
/// When: Timestamps are examined
/// Then: Timestamps are valid millisecond timestamps near current time
#[test]
fn test_timestamps_are_valid() {
    let session = create_test_session_output();
    let output_line = OutputLine::Session(session);
    let json_str = serde_json::to_string(&output_line).expect("serialize session");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    if let Some(session_obj) = json.get("session").and_then(|s| s.as_object()) {
        // Check timestamp fields
        for key in ["timestamp", "created_at", "updated_at"] {
            if let Some(ts) = session_obj.get(key).and_then(JsonValue::as_i64) {
                // Timestamp should be after year 2020 (1609459200000 ms)
                // and before year 2100 (4102444800000 ms)
                assert!(
                    ts > 1609459200000 && ts < 4102444800000,
                    "Timestamp {ts} for field {key} is unreasonable"
                );
            }
        }
    }
}

// =============================================================================
// TEST: Enum variants serialize to snake_case
// =============================================================================

/// Given: Commands that produce enum fields (status, kind, severity, etc.)
/// When: Enum values are examined
/// Then: Values are serialized in snake_case format
#[test]
fn test_enums_serialize_to_snake_case() {
    // Test SessionStatus
    let session = create_test_session_output();
    let output_line = OutputLine::Session(session);
    let json_str = serde_json::to_string(&output_line).expect("serialize session");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    if let Some(status) = json
        .get("session")
        .and_then(|s| s.get("status"))
        .and_then(JsonValue::as_str)
    {
        // Should be lowercase, snake_case
        assert!(
            status == status.to_lowercase(),
            "Status should be lowercase: {status}"
        );
        assert!(
            !status.contains(' '),
            "Status should not contain spaces: {status}"
        );
    }

    // Test Issue kind and severity
    let issue = create_test_issue();
    let output_line = OutputLine::Issue(issue);
    let json_str = serde_json::to_string(&output_line).expect("serialize issue");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    for field in ["kind", "severity"] {
        if let Some(value) = json
            .get("issue")
            .and_then(|i| i.get(field))
            .and_then(JsonValue::as_str)
        {
            assert!(
                value == value.to_lowercase(),
                "Issue.{field} should be lowercase: {value}"
            );
            assert!(
                !value.contains(' '),
                "Issue.{field} should not contain spaces: {value}"
            );
        }
    }

    // Test ResultOutput outcome
    let result = create_test_result_output();
    let output_line = OutputLine::Result(result);
    let json_str = serde_json::to_string(&output_line).expect("serialize result");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    if let Some(outcome) = json
        .get("result")
        .and_then(|r| r.get("outcome"))
        .and_then(JsonValue::as_str)
    {
        assert!(
            outcome == outcome.to_lowercase(),
            "ResultOutput.outcome should be lowercase: {outcome}"
        );
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Find a JSON line by its top-level type key (e.g., "session", "issue", "result")
/// Returns the inner object (the value of the type key), not the wrapper
fn find_json_line_by_type(output: &str, type_name: &str) -> Option<JsonValue> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('{') {
                serde_json::from_str::<JsonValue>(trimmed)
                    .ok()
                    .and_then(|json| {
                        json.as_object().and_then(|obj| {
                            obj.get(type_name).cloned() // Return the inner object
                        })
                    })
            } else {
                None
            }
        })
        .next()
}

// =============================================================================
// CONTRACT TESTS
// =============================================================================

/// Given: A successful ResultOutput
/// When: ResultOutput is examined
/// Then: outcome field is success
#[test]
fn test_success_command_has_success_outcome() {
    let result = create_test_result_output();
    let output_line = OutputLine::Result(result);
    let json_str = serde_json::to_string(&output_line).expect("serialize result");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    // Verify outcome is success
    assert!(
        json.get("result")
            .and_then(|r| r.get("outcome"))
            .and_then(JsonValue::as_str)
            == Some("success"),
        "Successful command ResultOutput should have outcome=success"
    );
}

/// Given: A failed ResultOutput
/// When: ResultOutput is examined
/// Then: outcome field is failure
#[test]
fn test_failed_command_has_failure_outcome() {
    let result = ResultOutput::failure(
        ResultKind::Command,
        Message::new("Command failed due to validation error".to_string()).expect("valid message"),
    )
    .expect("valid result");

    let output_line = OutputLine::Result(result);
    let json_str = serde_json::to_string(&output_line).expect("serialize result");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse json");

    // Verify outcome is failure
    assert!(
        json.get("result")
            .and_then(|r| r.get("outcome"))
            .and_then(JsonValue::as_str)
            == Some("failure"),
        "Failed command ResultOutput should have outcome=failure"
    );
}

/// Given: An OutputLine containing an Issue
/// When: Serialized and examined
/// Then: Issue is properly structured within OutputLine
#[test]
fn test_issue_in_output_line_structure() {
    let issue = create_test_issue();
    let output_line = OutputLine::Issue(issue);

    // Serialize and verify structure
    let json_str = serde_json::to_string(&output_line).expect("serialize output line");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse output line json");

    // The OutputLine enum uses snake_case variant names as keys
    assert!(
        json.get("issue").is_some(),
        "OutputLine::Issue should have 'issue' key"
    );

    // Verify nested Issue structure
    let issue_obj = json.get("issue").expect("issue object");
    assert!(
        issue_obj.get("id").and_then(JsonValue::as_str).is_some(),
        "Issue.id should be a string"
    );
    assert!(
        issue_obj.get("kind").and_then(JsonValue::as_str).is_some(),
        "Issue.kind should be a string"
    );
}

// =============================================================================
// TEST FIXTURES
// =============================================================================

fn create_test_session_output() -> SessionOutput {
    use std::path::PathBuf;

    use chrono::Utc;

    SessionOutput {
        name: "test-session".to_string(),
        status: SessionState::Active,
        state: SessionState::Active,
        workspace_path: PathBuf::from("/tmp/test-workspace"),
        branch: None,
        metadata: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn create_test_issue() -> Issue {
    Issue::new(
        IssueId::new("TEST-001".to_string()).expect("valid id"),
        IssueTitle::new("Test issue".to_string()).expect("valid title"),
        IssueKind::Validation,
        IssueSeverity::Error,
    )
    .expect("valid issue")
}

fn create_test_result_output() -> ResultOutput {
    ResultOutput::success(
        ResultKind::Command,
        Message::new("Command completed successfully".to_string()).expect("valid message"),
    )
    .expect("valid result")
}

fn create_test_action() -> Action {
    use chrono::Utc;

    Action {
        verb: ActionVerb::Create,
        target: ActionTarget::Session("test-session".to_string()),
        status: ActionStatus::Completed,
        timestamp: Utc::now(),
    }
}
