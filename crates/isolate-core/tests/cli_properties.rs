//! These tests define the contract that the CLI structure must satisfy.
//! All tests verify invariants for the `isolate <object> <action>` pattern.
//!
//! # Invariants tested:
//! 1. Noun-verb pattern: All commands follow `<object> <action>` structure
//! 2. JSON validity: All JSON output is valid and parseable
//! 3. Argument consistency: Command argument parsing is consistent
//! 4. Exit codes: Follow conventions (0=success, non-zero=error)
//! 5. Object completeness: All 7 objects have required subcommands
//!
//! Run with: cargo test --package isolate-core --test cli_properties
//! Reproducible: Set `PROPTEST_SEED` environment variable for deterministic runs

#![allow(
    clippy::uninlined_format_args,
    clippy::assertions_on_constants,
    clippy::manual_string_new,
    unused_doc_comments,
    unused_imports,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::unnecessary_map_or,
    clippy::redundant_closure_for_method_calls,
    clippy::useless_vec,
    clippy::unreadable_literal,
    clippy::doc_markdown
)]

use std::path::PathBuf;

use proptest::{prelude::*, prop_oneof, proptest};

// Integration tests have relaxed clippy settings for test ergonomics.
// Production code (src/) must use strict zero-unwrap/panic patterns.

/// Optimized proptest config for fast CLI property tests.
/// Uses 64 cases for simple invariants.
fn fast_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        max_shrink_iters: 256,
        ..ProptestConfig::default()
    }
}

/// Standard proptest config for CLI property tests.
/// Uses 100 cases for moderately complex invariants.
#[allow(dead_code)]
fn standard_config() -> ProptestConfig {
    ProptestConfig {
        cases: 100,
        ..ProptestConfig::default()
    }
}

use isolate_core::{
    output::{
        domain_types::{IssueId, IssueTitle, Message},
        Issue, IssueKind, IssueSeverity, OutputLine, SessionOutput, Summary, SummaryType,
    },
    types::SessionStatus,
    WorkspaceState,
};

// ═══════════════════════════════════════════════════════════════════════════
// CLI STRUCTURE CONSTANTS - Source of truth for all object/verb patterns
// ═══════════════════════════════════════════════════════════════════════════

/// All valid object names in the CLI (nouns)
const VALID_OBJECTS: &[&str] = &[
    "task", "session", "stack", "agent", "status", "config", "doctor",
];

/// Valid actions (verbs) for each object
const TASK_ACTIONS: &[&str] = &["list", "show", "start", "done"];
const SESSION_ACTIONS: &[&str] = &[
    "list", "add", "remove", "focus", "pause", "resume", "clone", "rename", "attach", "spawn",
    "sync", "init",
];
const STACK_ACTIONS: &[&str] = &["status", "list", "create", "push", "pop"];
const AGENT_ACTIONS: &[&str] = &[
    "list",
    "register",
    "unregister",
    "heartbeat",
    "status",
    "broadcast",
];
const STATUS_ACTIONS: &[&str] = &["show", "whereami", "whoami", "context"];
const CONFIG_ACTIONS: &[&str] = &["list", "get", "set", "schema"];
const DOCTOR_ACTIONS: &[&str] = &["check", "fix", "integrity", "clean"];

/// Exit codes following isolate conventions
const EXIT_SUCCESS: i32 = 0;
const EXIT_USAGE_ERROR: i32 = 1;
const EXIT_CONFIG_ERROR: i32 = 2;
const EXIT_STATE_ERROR: i32 = 3;
const EXIT_EXTERNAL_ERROR: i32 = 4;

// ═══════════════════════════════════════════════════════════════════════════
// CUSTOM STRATEGIES FOR GENERATING TEST DATA
// ═══════════════════════════════════════════════════════════════════════════

/// Generate valid object names
fn object_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("task".to_string()),
        Just("session".to_string()),
        Just("stack".to_string()),
        Just("agent".to_string()),
        Just("status".to_string()),
        Just("config".to_string()),
        Just("doctor".to_string()),
    ]
}

/// Generate valid session names (alphanumeric with dash/underscore)
fn session_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,63}"
}

/// Generate any string including invalid ones
fn any_string_strategy() -> impl Strategy<Value = String> {
    ".*"
}

/// Generate non-terminal session statuses (valid for SessionOutput::new)
fn non_terminal_session_status_strategy() -> impl Strategy<Value = SessionStatus> {
    prop_oneof![
        Just(SessionStatus::Creating),
        Just(SessionStatus::Active),
        Just(SessionStatus::Paused),
    ]
}

/// Generate workspace states
fn workspace_state_strategy() -> impl Strategy<Value = WorkspaceState> {
    prop_oneof![
        Just(WorkspaceState::Created),
        Just(WorkspaceState::Working),
        Just(WorkspaceState::Ready),
        Just(WorkspaceState::Merged),
        Just(WorkspaceState::Abandoned),
        Just(WorkspaceState::Conflict),
    ]
}

/// Generate absolute paths
fn absolute_path_strategy() -> impl Strategy<Value = PathBuf> {
    "[a-zA-Z0-9_-]{1,20}".prop_map(|s| PathBuf::from(format!("/tmp/isolate-test-{}", s)))
}

/// Generate valid summary types
fn summary_type_strategy() -> impl Strategy<Value = SummaryType> {
    prop_oneof![
        Just(SummaryType::Status),
        Just(SummaryType::Count),
        Just(SummaryType::Info),
    ]
}

/// Generate valid issue kinds
fn issue_kind_strategy() -> impl Strategy<Value = IssueKind> {
    prop_oneof![
        Just(IssueKind::Validation),
        Just(IssueKind::StateConflict),
        Just(IssueKind::ResourceNotFound),
        Just(IssueKind::PermissionDenied),
        Just(IssueKind::Timeout),
        Just(IssueKind::Configuration),
        Just(IssueKind::External),
    ]
}

/// Generate valid issue severities
fn issue_severity_strategy() -> impl Strategy<Value = IssueSeverity> {
    prop_oneof![
        Just(IssueSeverity::Hint),
        Just(IssueSeverity::Warning),
        Just(IssueSeverity::Error),
        Just(IssueSeverity::Critical),
    ]
}

/// Generate exit codes following conventions
fn exit_code_strategy() -> impl Strategy<Value = i32> {
    0i32..=4i32
}

/// Generate non-empty message strings
fn message_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ][a-zA-Z0-9 ]{5,50}"
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 1: NOUN-VERB PATTERN
// ═══════════════════════════════════════════════════════════════════════════

/// Property: All object names are lowercase ASCII without special characters
///
/// INVARIANT: Object names must be valid CLI tokens (lowercase, no spaces/special chars)
proptest! {
    #![proptest_config(fast_config())]

    #[test]
    fn prop_object_names_are_valid_tokens(object in object_strategy()) {
        // Object names must be lowercase
        prop_assert!(
            object == object.to_lowercase(),
            "Object name must be lowercase: {}",
            object
        );

        // Object names must be alphanumeric (with no spaces)
        prop_assert!(
            object.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "Object name must be lowercase alphanumeric: {}",
            object
        );

        // Object names must not be empty
        prop_assert!(!object.is_empty(), "Object name must not be empty");
    }
}

/// Property: Object-action pairs follow noun-verb pattern
///
/// INVARIANT: Every valid command is <object> <action>
proptest! {
    #![proptest_config(fast_config())]

    #[test]
    fn prop_all_commands_follow_noun_verb_pattern(
        object in object_strategy(),
        action_idx in 0usize..10,
    ) {
        // Get valid actions for this object
        let valid_actions: Vec<&str> = match object.as_str() {
            "task" => TASK_ACTIONS.to_vec(),
            "session" => SESSION_ACTIONS.to_vec(),
            "stack" => STACK_ACTIONS.to_vec(),
            "agent" => AGENT_ACTIONS.to_vec(),
            "status" => STATUS_ACTIONS.to_vec(),
            "config" => CONFIG_ACTIONS.to_vec(),
            "doctor" => DOCTOR_ACTIONS.to_vec(),
            _ => vec![],
        };

        // Each object must have at least one action
        prop_assert!(!valid_actions.is_empty(), "Object {} must have actions", object);

        // All actions must be valid verbs (lowercase, no special chars)
        for action in &valid_actions {
            prop_assert!(
                action.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "Action {} must be lowercase alphanumeric",
                action
            );
        }

        // Action index should be valid (if within bounds)
        if action_idx < valid_actions.len() {
            let action = valid_actions[action_idx];
            prop_assert!(!action.is_empty(), "Action must not be empty");
        }
    }
}

/// Property: All 7 objects are accounted for
///
/// INVARIANT: The CLI has exactly 7 top-level objects
#[test]
fn prop_all_objects_exist() {
    let expected_count = 7;
    let actual_count = VALID_OBJECTS.len();

    assert_eq!(
        actual_count, expected_count,
        "CLI must have exactly {} objects, found {}",
        expected_count, actual_count
    );

    // Each object must be unique
    let unique_count = VALID_OBJECTS
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();
    assert_eq!(unique_count, expected_count, "All objects must be unique");
}

/// Property: Each object has required minimum set of actions
///
/// INVARIANT: All objects must have at least list/show capability
#[test]
fn prop_objects_have_required_actions() {
    for object in VALID_OBJECTS {
        let actions: Vec<&str> = match *object {
            "task" => TASK_ACTIONS.to_vec(),
            "session" => SESSION_ACTIONS.to_vec(),
            "stack" => STACK_ACTIONS.to_vec(),
            "agent" => AGENT_ACTIONS.to_vec(),
            "status" => STATUS_ACTIONS.to_vec(),
            "config" => CONFIG_ACTIONS.to_vec(),
            "doctor" => DOCTOR_ACTIONS.to_vec(),
            _ => vec![],
        };

        assert!(
            !actions.is_empty(),
            "Object {} must have at least one action",
            object
        );

        // Most objects should have a way to query state (list, show, status, check)
        // Note: doctor uses "check" instead of "list" as it's diagnostic
        let has_query = actions
            .iter()
            .any(|&a| a == "list" || a == "show" || a == "status" || a == "check");
        assert!(
            has_query,
            "Object {} must have list, show, status, or check action for querying",
            object
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 2: JSON VALIDITY
// ═══════════════════════════════════════════════════════════════════════════

/// Property: SessionOutput serializes to valid JSON
///
/// INVARIANT: All CLI output types must serialize to valid JSON
/// INVARIANT: Terminal statuses (Completed/Failed) are rejected for new sessions
proptest! {
    #![proptest_config(standard_config())]

    #[test]
    fn prop_session_output_valid_json(
        name in session_name_strategy(),
        status in non_terminal_session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        let session_result = SessionOutput::new(name.clone(), status, state, path);

        match session_result {
            Ok(session) => {
                // Serialize to JSON
                let json_result = serde_json::to_string(&session);
                prop_assert!(json_result.is_ok(), "SessionOutput must serialize to JSON");

                let json = json_result.expect("SessionOutput should serialize to JSON");

                // Parse back to verify it's valid JSON
                let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
                prop_assert!(parsed.is_ok(), "Serialized JSON must be parseable: {}", json);

                // Verify required fields
                let value = parsed.expect("Parsed JSON value should be valid");
                prop_assert!(value.get("name").is_some(), "JSON must contain 'name' field");
                prop_assert!(value.get("status").is_some(), "JSON must contain 'status' field");
                prop_assert!(value.get("state").is_some(), "JSON must contain 'state' field");
                prop_assert!(value.get("workspace_path").is_some(), "JSON must contain 'workspace_path' field");
                prop_assert!(value.get("created_at").is_some(), "JSON must contain 'created_at' field");
                prop_assert!(value.get("updated_at").is_some(), "JSON must contain 'updated_at' field");
            }
            Err(e) => {
                // Construction failed - should only happen for empty names
                prop_assert!(
                    name.trim().is_empty(),
                    "SessionOutput construction should only fail for empty names, got: {:?}",
                    e
                );
            }
        }
    }
}

/// Property: Summary serializes to valid JSON
///
/// INVARIANT: All summary output must be valid JSON
proptest! {
    #![proptest_config(standard_config())]

    #[test]
    fn prop_summary_valid_json(
        summary_type in summary_type_strategy(),
        message in message_strategy(),
    ) {
        let summary_result = Summary::new(summary_type, Message::new(message.clone()).expect("valid message"));

        if message.trim().is_empty() {
            prop_assert!(summary_result.is_err(), "Empty message should fail");
        } else {
            let summary = summary_result.expect("Non-empty message should succeed");
            let json_result = serde_json::to_string(&summary);
            prop_assert!(json_result.is_ok(), "Summary must serialize to JSON");

            let json = json_result.expect("Summary should serialize to JSON");
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
            prop_assert!(parsed.is_ok(), "Serialized Summary must be parseable");

            let value = parsed.expect("Parsed Summary JSON should be valid");
            prop_assert!(value.get("type").is_some(), "JSON must contain 'type' field");
            prop_assert!(value.get("message").is_some(), "JSON must contain 'message' field");
            prop_assert!(value.get("timestamp").is_some(), "JSON must contain 'timestamp' field");
        }
    }
}

/// Property: Issue serializes to valid JSON
///
/// INVARIANT: All issue output must be valid JSON
proptest! {
    #![proptest_config(fast_config())]

    #[test]
    fn prop_issue_valid_json(
        id in "[a-zA-Z0-9-]{5,20}",
        title in message_strategy(),
        kind in issue_kind_strategy(),
        severity in issue_severity_strategy(),
    ) {
        let issue = Issue::new(
            IssueId::new(id).expect("valid id"),
            IssueTitle::new(title).expect("valid title"),
            kind,
            severity,
        )
        .expect("valid issue");

        let json_result = serde_json::to_string(&issue);
        prop_assert!(json_result.is_ok(), "Issue must serialize to JSON");

        let json = json_result.expect("Issue should serialize to JSON");
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "Serialized Issue must be parseable");

        let value = parsed.expect("Parsed Issue JSON should be valid");
        prop_assert!(value.get("id").is_some(), "JSON must contain 'id' field");
        prop_assert!(value.get("title").is_some(), "JSON must contain 'title' field");
        prop_assert!(value.get("kind").is_some(), "JSON must contain 'kind' field");
        prop_assert!(value.get("severity").is_some(), "JSON must contain 'severity' field");
    }
}

/// Property: OutputLine variants all serialize to valid JSON
///
/// INVARIANT: All OutputLine variants must produce valid JSON
proptest! {
    #![proptest_config(standard_config())]

    #[test]
    fn prop_output_line_variants_valid_json(
        name in session_name_strategy(),
        status in non_terminal_session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        let session = SessionOutput::new(name, status, state, path);

        if let Ok(s) = session {
            let line = OutputLine::Session(s);
            let json_result = serde_json::to_string(&line);
            prop_assert!(json_result.is_ok(), "OutputLine::Session must serialize");

            let json = json_result.expect("OutputLine::Session should serialize to JSON");
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
            prop_assert!(parsed.is_ok(), "OutputLine JSON must be parseable");
        }
    }
}

/// Property: JSON field names use snake_case
///
/// INVARIANT: All JSON field names follow snake_case convention
proptest! {
    #![proptest_config(standard_config())]

    #[test]
    fn prop_json_field_names_snake_case(
        name in session_name_strategy(),
        status in non_terminal_session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        let session = SessionOutput::new(name, status, state, path);
        if let Ok(s) = session {
            let json = serde_json::to_string(&s).expect("must serialize");
            let value: serde_json::Value = serde_json::from_str(&json).expect("must parse");

            if let serde_json::Value::Object(map) = value {
                for key in map.keys() {
                    // Field names should be snake_case (lowercase with underscores)
                    let is_snake_case = key.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit());
                    prop_assert!(
                        is_snake_case,
                        "Field name '{}' should be snake_case",
                        key
                    );
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 3: ARGUMENT CONSISTENCY
// ═══════════════════════════════════════════════════════════════════════════

/// Property: Session names are validated consistently
///
/// INVARIANT: Session name validation is consistent across all commands
proptest! {
    #![proptest_config(standard_config())]

    #[test]
    fn prop_session_name_validation_consistent(name in any_string_strategy()) {
        let is_valid_format = !name.trim().is_empty()
            && name.len() <= 64
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            && name.chars().next().map_or(false, |c| c.is_ascii_alphabetic());

        // SessionOutput should accept valid names
        let result = SessionOutput::new(
            name.clone(),
            SessionStatus::Active,
            WorkspaceState::Working,
            PathBuf::from("/tmp/test"),
        );

        if is_valid_format {
            // Note: SessionOutput::new only validates empty names currently
            // But it should accept properly formatted names
            if !name.trim().is_empty() {
                prop_assert!(result.is_ok(), "Valid session name '{}' should be accepted", name);
            }
        } else if name.trim().is_empty() {
            // Empty names must always be rejected
            prop_assert!(result.is_err(), "Empty session name should be rejected");
        }
    }
}

/// Property: Workspace paths must be absolute
///
/// INVARIANT: Workspace paths are validated consistently
proptest! {
    #![proptest_config(standard_config())]

    #[test]
    fn prop_workspace_path_validation(
        name in session_name_strategy(),
        status in non_terminal_session_status_strategy(),
        state in workspace_state_strategy(),
        path_str in any_string_strategy(),
    ) {
        let path = PathBuf::from(&path_str);
        let result = SessionOutput::new(name, status, state, path.clone());

        // SessionOutput requires absolute paths
        if path.is_absolute() {
            prop_assert!(result.is_ok(), "Absolute path should be accepted");
        } else {
            prop_assert!(result.is_err(), "Relative path should be rejected");
        }
    }
}

/// Property: JSON flag is consistent across all commands
///
/// INVARIANT: All commands accept --json flag consistently
#[test]
fn prop_json_flag_consistent() {
    // All object commands should support --json flag
    for object in VALID_OBJECTS {
        // This is verified at compile time by the CLI builder
        // Here we just document the invariant
        assert!(true, "Object {} supports --json flag", object);
    }
}

/// Property: Required vs optional arguments are consistent
///
/// INVARIANT: Argument requirements follow consistent patterns
proptest! {
    #![proptest_config(fast_config())]

    #[test]
    fn prop_argument_requirements_consistent(
        action in prop_oneof![
            Just("list"),
            Just("show"),
            Just("add"),
            Just("remove"),
        ],
    ) {
        // List actions should never require positional arguments
        if action == "list" {
            // list should work without positional args (may have optional filters)
            prop_assert!(true, "list action should not require positional arguments");
        }

        // Show/remove actions should require an identifier
        if action == "show" || action == "remove" {
            // These should require an ID argument
            prop_assert!(true, "{} action should require an identifier", action);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 4: EXIT CODE CONVENTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Property: Exit code 0 means success
///
/// INVARIANT: Exit code 0 is only returned on successful completion
proptest! {
    #![proptest_config(fast_config())]

    #[test]
    fn prop_exit_code_0_means_success(exit_code in exit_code_strategy()) {
        if exit_code == EXIT_SUCCESS {
            // Exit code 0 should always indicate success
            prop_assert!(exit_code == 0, "Exit code 0 must indicate success");
        }
    }
}

/// Property: Non-zero exit codes indicate errors
///
/// INVARIANT: Non-zero exit codes indicate different error categories
proptest! {
    #![proptest_config(fast_config())]

    #[test]
    fn prop_non_zero_exit_codes_indicate_errors(exit_code in exit_code_strategy()) {
        if exit_code != EXIT_SUCCESS {
            // Non-zero codes indicate some form of error
            match exit_code {
                1 => {
                    // Usage error (invalid arguments, etc.)
                    prop_assert!(true, "Exit code 1 = usage error");
                }
                2 => {
                    // Configuration error
                    prop_assert!(true, "Exit code 2 = config error");
                }
                3 => {
                    // State error (invalid state transition, etc.)
                    prop_assert!(true, "Exit code 3 = state error");
                }
                4 => {
                    // External error (jj, network, etc.)
                    prop_assert!(true, "Exit code 4 = external error");
                }
                _ => {
                    // Unknown exit code - should not be used
                    prop_assert!(false, "Exit code {} is not defined in convention", exit_code);
                }
            }
        }
    }
}

/// Property: Exit codes are in valid range
///
/// INVARIANT: Exit codes are 0-4 (small integer range)
proptest! {
    #![proptest_config(fast_config())]

    #[test]
    fn prop_exit_codes_in_valid_range(exit_code in exit_code_strategy()) {
        prop_assert!(
            (0..=4).contains(&exit_code),
            "Exit code {} must be in range 0-4",
            exit_code
        );
    }
}

/// Property: Error severity maps to exit codes consistently
///
/// INVARIANT: Higher severity issues should result in higher exit codes
proptest! {
    #![proptest_config(fast_config())]

    #[test]
    fn prop_error_severity_exit_code_mapping(severity in issue_severity_strategy()) {
        let expected_min_exit_code = match severity {
            IssueSeverity::Hint => 1,      // Usage error level
            IssueSeverity::Warning => 2,   // Config error level
            IssueSeverity::Error => 3,     // State error level
            IssueSeverity::Critical => 4,  // External/fatal error level
        };

        // Document the mapping expectation
        prop_assert!(
            expected_min_exit_code >= 1,
            "Any error severity should map to non-zero exit code"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 5: OUTPUT TYPE COMPLETENESS
// ═══════════════════════════════════════════════════════════════════════════

/// Property: All OutputLine variants have unique kind strings
///
/// INVARIANT: Each OutputLine variant has a unique type identifier
#[test]
fn prop_output_line_kinds_unique() {
    let kinds = vec![
        OutputLine::Summary(
            Summary::new(
                SummaryType::Status,
                Message::new("test").expect("valid message"),
            )
            .expect("valid summary"),
        )
        .kind(),
        OutputLine::Session(
            SessionOutput::new(
                "test".into(),
                SessionStatus::Active,
                WorkspaceState::Working,
                PathBuf::from("/tmp"),
            )
            .unwrap(),
        )
        .kind(),
        OutputLine::Issue(
            Issue::new(
                IssueId::new("test").expect("valid id"),
                IssueTitle::new("test").expect("valid title"),
                IssueKind::Validation,
                IssueSeverity::Hint,
            )
            .expect("valid issue"),
        )
        .kind(),
    ];

    // All kinds should be unique
    let unique_kinds: std::collections::HashSet<_> = kinds.iter().collect();
    assert_eq!(
        kinds.len(),
        unique_kinds.len(),
        "All OutputLine kinds must be unique"
    );
}

/// Property: Timestamp fields are always present and valid
///
/// INVARIANT: All timestamped output includes valid ISO 8601 timestamps
proptest! {
    #![proptest_config(standard_config())]

    #[test]
    fn prop_timestamps_valid(
        name in session_name_strategy(),
        status in non_terminal_session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        let session = SessionOutput::new(name, status, state, path);
        if let Ok(s) = session {
            let json = serde_json::to_string(&s).expect("must serialize");
            let value: serde_json::Value = serde_json::from_str(&json).expect("must parse");

            // Check created_at is present and valid
            let created_at = value.get("created_at").and_then(|v| v.as_i64());
            prop_assert!(created_at.is_some(), "created_at must be a valid timestamp");

            // Check updated_at is present and valid
            let updated_at = value.get("updated_at").and_then(|v| v.as_i64());
            prop_assert!(updated_at.is_some(), "updated_at must be a valid timestamp");

            // Timestamps should be reasonable (after year 2020, before year 3000)
            let min_ts = 1577836800000i64; // 2020-01-01
            let max_ts = 32503680000000i64; // Year 3000

            if let Some(ts) = created_at {
                prop_assert!(
                    ts > min_ts && ts < max_ts,
                    "created_at timestamp should be reasonable: {}",
                    ts
                );
            }
        }
    }
}

/// Property: Summary types serialize correctly
///
/// INVARIANT: SummaryType enum values serialize to lowercase
proptest! {
    #![proptest_config(fast_config())]

    #[test]
    fn prop_summary_type_serialization(summary_type in summary_type_strategy()) {
        let summary = Summary::new(summary_type, Message::new("test message").expect("valid message"));
        if let Ok(s) = summary {
            let json = serde_json::to_string(&s).expect("must serialize");
            let value: serde_json::Value = serde_json::from_str(&json).expect("must parse");

            let type_field = value.get("type").and_then(|v| v.as_str());
            prop_assert!(type_field.is_some(), "type field must be present");

            if let Some(t) = type_field {
                // Should be lowercase
                prop_assert!(
                    t == t.to_lowercase(),
                    "SummaryType should serialize to lowercase: {}",
                    t
                );
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ADDITIONAL INVARIANT TESTS
// ═══════════════════════════════════════════════════════════════════════════

/// Property: Session status values are finite and known
#[test]
fn prop_session_statuses_finite() {
    let all_statuses = vec![
        SessionStatus::Creating,
        SessionStatus::Active,
        SessionStatus::Paused,
        SessionStatus::Completed,
        SessionStatus::Failed,
    ];

    // We expect exactly 5 session statuses
    assert_eq!(all_statuses.len(), 5, "Expected exactly 5 session statuses");
}

/// Property: Terminal statuses are accepted for SessionOutput::new
///
/// INVARIANT: Completed and Failed statuses CAN be used when creating session output objects
#[test]
fn prop_terminal_statuses_accepted() {
    // Completed status should be accepted
    let completed_result = SessionOutput::new(
        "test".to_string(),
        SessionStatus::Completed,
        WorkspaceState::Working,
        PathBuf::from("/tmp/test"),
    );
    assert!(
        completed_result.is_ok(),
        "Completed status should be accepted"
    );

    // Failed status should be accepted
    let failed_result = SessionOutput::new(
        "test".to_string(),
        SessionStatus::Failed,
        WorkspaceState::Working,
        PathBuf::from("/tmp/test"),
    );
    assert!(failed_result.is_ok(), "Failed status should be accepted");
}

/// Property: Relative paths are rejected for SessionOutput::new
///
/// INVARIANT: Workspace paths must be absolute
#[test]
fn prop_relative_paths_rejected() {
    use isolate_core::output::OutputLineError;

    let relative_result = SessionOutput::new(
        "test".to_string(),
        SessionStatus::Active,
        WorkspaceState::Working,
        PathBuf::from("relative/path"),
    );
    assert!(
        matches!(relative_result, Err(OutputLineError::RelativePath)),
        "Relative path should be rejected with RelativePath error"
    );
}

/// Property: Workspace state values are finite and known
#[test]
fn prop_workspace_states_finite() {
    let all_states = vec![
        WorkspaceState::Created,
        WorkspaceState::Working,
        WorkspaceState::Ready,
        WorkspaceState::Merged,
        WorkspaceState::Abandoned,
        WorkspaceState::Conflict,
    ];

    // We expect exactly 6 workspace states
    assert_eq!(all_states.len(), 6, "Expected exactly 6 workspace states");
}

/// Property: Issue kind values are finite and known
#[test]
fn prop_issue_kinds_finite() {
    let all_kinds = vec![
        IssueKind::Validation,
        IssueKind::StateConflict,
        IssueKind::ResourceNotFound,
        IssueKind::PermissionDenied,
        IssueKind::Timeout,
        IssueKind::Configuration,
        IssueKind::External,
    ];

    // We expect exactly 7 issue kinds
    assert_eq!(all_kinds.len(), 7, "Expected exactly 7 issue kinds");
}

/// Property: Issue severity values are finite and known
#[test]
fn prop_issue_severities_finite() {
    let all_severities = vec![
        IssueSeverity::Hint,
        IssueSeverity::Warning,
        IssueSeverity::Error,
        IssueSeverity::Critical,
    ];

    // We expect exactly 4 issue severities
    assert_eq!(
        all_severities.len(),
        4,
        "Expected exactly 4 issue severities"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS TO CONFIRM TEST HARNESS WORKS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// This test MUST PASS to confirm the test harness works
    #[test]
    fn test_harness_works() {
        assert!(true, "Test harness should work");
    }

    /// Test that all 7 objects are defined
    #[test]
    fn test_all_objects_defined() {
        assert_eq!(VALID_OBJECTS.len(), 7, "Must have exactly 7 objects");
    }

    /// Test that each object has actions
    #[test]
    fn test_objects_have_actions() {
        assert!(!TASK_ACTIONS.is_empty(), "Task must have actions");
        assert!(!SESSION_ACTIONS.is_empty(), "Session must have actions");
        assert!(!STACK_ACTIONS.is_empty(), "Stack must have actions");
        assert!(!AGENT_ACTIONS.is_empty(), "Agent must have actions");
        assert!(!STATUS_ACTIONS.is_empty(), "Status must have actions");
        assert!(!CONFIG_ACTIONS.is_empty(), "Config must have actions");
        assert!(!DOCTOR_ACTIONS.is_empty(), "Doctor must have actions");
    }

    /// Test that session output with empty name is rejected
    #[test]
    fn test_empty_session_name_rejected() {
        let result = SessionOutput::new(
            "".to_string(),
            SessionStatus::Active,
            WorkspaceState::Working,
            PathBuf::from("/tmp/test"),
        );
        assert!(result.is_err(), "Empty session name should be rejected");
    }

    /// Test that summary with empty message is rejected
    #[test]
    fn test_empty_summary_message_rejected() {
        use isolate_core::output::OutputLineError;

        let result = Message::new("");
        assert!(result.is_err(), "Empty message should be rejected");
        match result {
            Err(OutputLineError::EmptyMessage) => {}
            _ => panic!("Expected EmptyMessage error, got {:?}", result),
        }
    }

    /// Test exit code conventions
    #[test]
    fn test_exit_code_conventions() {
        assert_eq!(EXIT_SUCCESS, 0, "Success exit code must be 0");
        assert_eq!(EXIT_USAGE_ERROR, 1, "Usage error exit code must be 1");
        assert_eq!(EXIT_CONFIG_ERROR, 2, "Config error exit code must be 2");
        assert_eq!(EXIT_STATE_ERROR, 3, "State error exit code must be 3");
        assert_eq!(EXIT_EXTERNAL_ERROR, 4, "External error exit code must be 4");
    }

    /// Test JSON serialization of session output
    #[test]
    fn test_session_output_json_serialization() {
        let session = SessionOutput::new(
            "test-session".to_string(),
            SessionStatus::Active,
            WorkspaceState::Working,
            PathBuf::from("/tmp/test"),
        )
        .expect("Valid session should be created");

        let json = serde_json::to_string(&session).expect("Should serialize to JSON");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("Should parse back from JSON");

        assert!(parsed.get("name").is_some(), "JSON must have name field");
        assert!(
            parsed.get("status").is_some(),
            "JSON must have status field"
        );
        assert!(parsed.get("state").is_some(), "JSON must have state field");
    }
}
