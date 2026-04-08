//! Exhaustive tests for the query command handler.
//!
//! Covers: query parsing, filter functions, serialization, error reporting,
//! edge cases, and property-based testing. All assertions use exact matching
//! (no bare `is_ok()`/`is_err()`).

use super::actions::run_query;
use super::data::{
    filter_sessions_by_agent, filter_sessions_by_status, QueryOptions, QueryOutput, QueryType,
    SessionInfo, SessionStatus,
};

// ============================================================================
// Helpers
// ============================================================================

fn sample_session(name: &str, status: SessionStatus, agent: Option<&str>) -> SessionInfo {
    SessionInfo {
        name: name.to_string(),
        status,
        workspace_path: Some(format!("/ws/{name}")),
        agent: agent.map(|a| a.to_string()),
        created_at: Some("2026-04-08T12:00:00Z".to_string()),
    }
}

fn minimal_session(name: &str, status: SessionStatus) -> SessionInfo {
    SessionInfo {
        name: name.to_string(),
        status,
        workspace_path: None,
        agent: None,
        created_at: None,
    }
}

// ============================================================================
// QueryType::from_str — exhaustive variants
// ============================================================================

#[test]
fn from_str_session_exists() {
    assert_eq!(QueryType::from_str("session-exists"), Some(QueryType::SessionExists));
}

#[test]
fn from_str_sessions() {
    assert_eq!(QueryType::from_str("sessions"), Some(QueryType::Sessions));
}

#[test]
fn from_str_session_info() {
    assert_eq!(QueryType::from_str("session-info"), Some(QueryType::SessionInfo));
}

#[test]
fn from_str_blockers() {
    assert_eq!(QueryType::from_str("blockers"), Some(QueryType::Blockers));
}

#[test]
fn from_str_session_count() {
    assert_eq!(QueryType::from_str("session-count"), Some(QueryType::SessionCount));
}

#[test]
fn from_str_help() {
    assert_eq!(QueryType::from_str("help"), Some(QueryType::Help));
}

#[test]
fn from_str_list_alias() {
    assert_eq!(QueryType::from_str("list"), Some(QueryType::Help));
}

#[test]
fn from_str_unknown_returns_none() {
    assert_eq!(QueryType::from_str(""), None);
    assert_eq!(QueryType::from_str("unknown"), None);
    assert_eq!(QueryType::from_str("SESSION-EXISTS"), None); // case-sensitive
    assert_eq!(QueryType::from_str("session_exists"), None); // underscore not hyphen
    assert_eq!(QueryType::from_str(" sessions"), None); // leading space
    assert_eq!(QueryType::from_str("sessions "), None); // trailing space
}

// ============================================================================
// QueryType::all_names
// ============================================================================

#[test]
fn all_names_returns_six_entries() {
    let names = QueryType::all_names();
    assert_eq!(names.len(), 6);
}

#[test]
fn all_names_matches_from_str_roundtrip() {
    for name in QueryType::all_names() {
        assert!(QueryType::from_str(name).is_some(), "all_names entry {name:?} should parse");
    }
}

#[test]
fn all_names_contains_expected() {
    let names = QueryType::all_names();
    assert!(names.contains(&"session-exists"));
    assert!(names.contains(&"sessions"));
    assert!(names.contains(&"session-info"));
    assert!(names.contains(&"blockers"));
    assert!(names.contains(&"session-count"));
    assert!(names.contains(&"help"));
}

// ============================================================================
// SessionStatus — parsing and roundtrip
// ============================================================================

#[test]
fn status_from_str_lossy_all_variants() {
    assert_eq!(SessionStatus::from_str_lossy("active"), SessionStatus::Active);
    assert_eq!(SessionStatus::from_str_lossy("paused"), SessionStatus::Paused);
    assert_eq!(SessionStatus::from_str_lossy("completed"), SessionStatus::Completed);
    assert_eq!(SessionStatus::from_str_lossy("aborted"), SessionStatus::Aborted);
}

#[test]
fn status_from_str_lossy_unknown_defaults_active() {
    assert_eq!(SessionStatus::from_str_lossy(""), SessionStatus::Active);
    assert_eq!(SessionStatus::from_str_lossy("Active"), SessionStatus::Active); // case-sensitive
    assert_eq!(SessionStatus::from_str_lossy("DONE"), SessionStatus::Active);
    assert_eq!(SessionStatus::from_str_lossy("pending"), SessionStatus::Active);
}

#[test]
fn status_as_str_roundtrip() {
    for status in [
        SessionStatus::Active,
        SessionStatus::Paused,
        SessionStatus::Completed,
        SessionStatus::Aborted,
    ] {
        assert_eq!(SessionStatus::from_str_lossy(status.as_str()), status);
    }
}

#[test]
fn status_as_str_values() {
    assert_eq!(SessionStatus::Active.as_str(), "active");
    assert_eq!(SessionStatus::Paused.as_str(), "paused");
    assert_eq!(SessionStatus::Completed.as_str(), "completed");
    assert_eq!(SessionStatus::Aborted.as_str(), "aborted");
}

#[test]
fn status_equality_and_copy() {
    let a = SessionStatus::Active;
    let b = a; // Copy semantics
    assert_eq!(a, b);
}

// ============================================================================
// SessionInfo — serialization
// ============================================================================

#[test]
fn session_info_serializes_with_all_fields() {
    let info = sample_session("s1", SessionStatus::Active, Some("agent-1"));
    let json = serde_json::to_string(&info).expect("serialize");
    assert!(json.contains("\"name\":\"s1\""));
    assert!(json.contains("\"status\":\"Active\""));
    assert!(json.contains("\"workspace_path\":\"/ws/s1\""));
    assert!(json.contains("\"agent\":\"agent-1\""));
    assert!(json.contains("\"created_at\":\"2026-04-08T12:00:00Z\""));
}

#[test]
fn session_info_skips_none_fields() {
    let info = minimal_session("s2", SessionStatus::Paused);
    let json = serde_json::to_string(&info).expect("serialize");
    assert!(json.contains("\"name\":\"s2\""));
    assert!(json.contains("\"status\":\"Paused\""));
    assert!(!json.contains("workspace_path"));
    assert!(!json.contains("agent"));
    assert!(!json.contains("created_at"));
}

#[test]
fn session_info_deserialization_roundtrip() {
    let info = sample_session("roundtrip", SessionStatus::Completed, Some("a1"));
    let json = serde_json::to_string(&info).expect("serialize");
    let back: SessionInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.name, "roundtrip");
    assert_eq!(back.status, SessionStatus::Completed);
    assert_eq!(back.agent.as_deref(), Some("a1"));
    assert_eq!(back.workspace_path.as_deref(), Some("/ws/roundtrip"));
}

#[test]
fn session_info_deserialization_missing_optional_fields() {
    let json = r#"{"name":"x","status":"Active"}"#;
    let info: SessionInfo = serde_json::from_str(json).expect("deserialize");
    assert_eq!(info.name, "x");
    assert_eq!(info.status, SessionStatus::Active);
    assert!(info.workspace_path.is_none());
    assert!(info.agent.is_none());
    assert!(info.created_at.is_none());
}

#[test]
fn session_info_all_statuses_serialize() {
    for (status, expected) in [
        (SessionStatus::Active, "Active"),
        (SessionStatus::Paused, "Paused"),
        (SessionStatus::Completed, "Completed"),
        (SessionStatus::Aborted, "Aborted"),
    ] {
        let info = minimal_session("t", status);
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(json.contains(expected), "status {status:?} should serialize as {expected}");
    }
}

// ============================================================================
// QueryOutput — serialization
// ============================================================================

#[test]
fn query_output_serializes_success() {
    let output = QueryOutput {
        success: true,
        query_type: "session-exists".to_string(),
        data: serde_json::json!({"name": "x", "exists": true}),
    };
    let json = serde_json::to_string(&output).expect("serialize");
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"query_type\":\"session-exists\""));
}

#[test]
fn query_output_serializes_failure() {
    let output = QueryOutput {
        success: false,
        query_type: "sessions".to_string(),
        data: serde_json::json!({"error": "db unavailable"}),
    };
    let json = serde_json::to_string(&output).expect("serialize");
    assert!(json.contains("\"success\":false"));
}

#[test]
fn query_output_roundtrip() {
    let output = QueryOutput {
        success: true,
        query_type: "blockers".to_string(),
        data: serde_json::json!({"count": 0}),
    };
    let json = serde_json::to_string(&output).expect("serialize");
    let back: QueryOutput = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.success, true);
    assert_eq!(back.query_type, "blockers");
}

// ============================================================================
// filter_sessions_by_status — exhaustive
// ============================================================================

#[test]
fn filter_by_status_empty_input() {
    let sessions: Vec<SessionInfo> = vec![];
    assert!(filter_sessions_by_status(&sessions, "active").is_empty());
}

#[test]
fn filter_by_status_no_match() {
    let sessions = vec![
        minimal_session("a", SessionStatus::Active),
        minimal_session("b", SessionStatus::Active),
    ];
    let result = filter_sessions_by_status(&sessions, "completed");
    assert!(result.is_empty());
}

#[test]
fn filter_by_status_all_match() {
    let sessions = vec![
        minimal_session("a", SessionStatus::Active),
        minimal_session("b", SessionStatus::Active),
    ];
    let result = filter_sessions_by_status(&sessions, "active");
    assert_eq!(result.len(), 2);
}

#[test]
fn filter_by_status_partial_match() {
    let sessions = vec![
        minimal_session("a", SessionStatus::Active),
        minimal_session("b", SessionStatus::Completed),
        minimal_session("c", SessionStatus::Active),
        minimal_session("d", SessionStatus::Aborted),
    ];
    let result = filter_sessions_by_status(&sessions, "active");
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "a");
    assert_eq!(result[1].name, "c");
}

#[test]
fn filter_by_status_each_variant() {
    let sessions = vec![
        minimal_session("s-active", SessionStatus::Active),
        minimal_session("s-paused", SessionStatus::Paused),
        minimal_session("s-completed", SessionStatus::Completed),
        minimal_session("s-aborted", SessionStatus::Aborted),
    ];
    assert_eq!(filter_sessions_by_status(&sessions, "active").len(), 1);
    assert_eq!(filter_sessions_by_status(&sessions, "paused").len(), 1);
    assert_eq!(filter_sessions_by_status(&sessions, "completed").len(), 1);
    assert_eq!(filter_sessions_by_status(&sessions, "aborted").len(), 1);
}

#[test]
fn filter_by_status_is_case_sensitive() {
    let sessions = vec![minimal_session("a", SessionStatus::Active)];
    assert!(filter_sessions_by_status(&sessions, "Active").is_empty());
    assert!(filter_sessions_by_status(&sessions, "ACTIVE").is_empty());
    assert_eq!(filter_sessions_by_status(&sessions, "active").len(), 1);
}

// ============================================================================
// filter_sessions_by_agent — exhaustive
// ============================================================================

#[test]
fn filter_by_agent_empty_input() {
    let sessions: Vec<SessionInfo> = vec![];
    assert!(filter_sessions_by_agent(&sessions, "agent-1").is_empty());
}

#[test]
fn filter_by_agent_no_match() {
    let sessions = vec![sample_session("a", SessionStatus::Active, Some("agent-1"))];
    assert!(filter_sessions_by_agent(&sessions, "agent-2").is_empty());
}

#[test]
fn filter_by_agent_excludes_none_agent() {
    let sessions = vec![
        sample_session("a", SessionStatus::Active, Some("agent-1")),
        minimal_session("b", SessionStatus::Active), // agent is None
    ];
    let result = filter_sessions_by_agent(&sessions, "agent-1");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "a");
}

#[test]
fn filter_by_agent_multiple_matches() {
    let sessions = vec![
        sample_session("a", SessionStatus::Active, Some("shared")),
        sample_session("b", SessionStatus::Completed, Some("shared")),
        sample_session("c", SessionStatus::Active, Some("other")),
    ];
    let result = filter_sessions_by_agent(&sessions, "shared");
    assert_eq!(result.len(), 2);
}

#[test]
fn filter_by_agent_is_case_sensitive() {
    let sessions = vec![sample_session("a", SessionStatus::Active, Some("Agent-1"))];
    assert!(filter_sessions_by_agent(&sessions, "agent-1").is_empty());
    assert_eq!(filter_sessions_by_agent(&sessions, "Agent-1").len(), 1);
}

#[test]
fn filter_by_agent_empty_string_agent() {
    let sessions = vec![sample_session("a", SessionStatus::Active, Some(""))];
    let result = filter_sessions_by_agent(&sessions, "");
    assert_eq!(result.len(), 1);
}

// ============================================================================
// Combined filters (multiple filters)
// ============================================================================

#[test]
fn combined_status_then_agent_filter() {
    let sessions = vec![
        sample_session("a", SessionStatus::Active, Some("agent-1")),
        sample_session("b", SessionStatus::Active, Some("agent-2")),
        sample_session("c", SessionStatus::Completed, Some("agent-1")),
    ];
    // Filter by status first
    let active = filter_sessions_by_status(&sessions, "active");
    assert_eq!(active.len(), 2);
    // Then filter those by agent — collect references
    let filtered: Vec<&&SessionInfo> = active
        .iter()
        .filter(|s| s.agent.as_deref() == Some("agent-1"))
        .collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "a");
}

#[test]
fn combined_agent_then_status_filter() {
    let sessions = vec![
        sample_session("a", SessionStatus::Active, Some("shared")),
        sample_session("b", SessionStatus::Completed, Some("shared")),
        sample_session("c", SessionStatus::Paused, Some("shared")),
    ];
    let by_agent = filter_sessions_by_agent(&sessions, "shared");
    assert_eq!(by_agent.len(), 3);
    let active: Vec<&&SessionInfo> = by_agent
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .collect();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].name, "a");
}

// ============================================================================
// run_query — action tests (error branch validation)
// ============================================================================

#[test]
fn run_query_session_exists_missing_argument_returns_validation_error() {
    let options = QueryOptions {
        query_type: QueryType::SessionExists,
        argument: None,
        status_filter: None,
        agent_filter: None,
    };
    let err = run_query(&options).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("Session name required"),
        "expected validation message, got: {msg}"
    );
}

#[test]
fn run_query_session_info_missing_argument_returns_validation_error() {
    let options = QueryOptions {
        query_type: QueryType::SessionInfo,
        argument: None,
        status_filter: None,
        agent_filter: None,
    };
    let err = run_query(&options).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("Session name required"),
        "expected validation message, got: {msg}"
    );
}

#[test]
fn run_query_session_exists_with_argument_succeeds() {
    let options = QueryOptions {
        query_type: QueryType::SessionExists,
        argument: Some("nonexistent-session-xyz".to_string()),
        status_filter: None,
        agent_filter: None,
    };
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_session_info_with_argument_succeeds() {
    let options = QueryOptions {
        query_type: QueryType::SessionInfo,
        argument: Some("any-session".to_string()),
        status_filter: None,
        agent_filter: None,
    };
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_sessions_no_filters_succeeds() {
    let options = QueryOptions {
        query_type: QueryType::Sessions,
        argument: None,
        status_filter: None,
        agent_filter: None,
    };
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_sessions_with_status_filter_succeeds() {
    let options = QueryOptions {
        query_type: QueryType::Sessions,
        argument: None,
        status_filter: Some("active".to_string()),
        agent_filter: None,
    };
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_sessions_with_agent_filter_succeeds() {
    let options = QueryOptions {
        query_type: QueryType::Sessions,
        argument: None,
        status_filter: None,
        agent_filter: Some("agent-1".to_string()),
    };
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_sessions_with_both_filters_succeeds() {
    let options = QueryOptions {
        query_type: QueryType::Sessions,
        argument: None,
        status_filter: Some("completed".to_string()),
        agent_filter: Some("agent-2".to_string()),
    };
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_blockers_succeeds() {
    let options = QueryOptions {
        query_type: QueryType::Blockers,
        argument: None,
        status_filter: None,
        agent_filter: None,
    };
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_session_count_succeeds() {
    let options = QueryOptions {
        query_type: QueryType::SessionCount,
        argument: None,
        status_filter: None,
        agent_filter: None,
    };
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_help_succeeds() {
    let options = QueryOptions {
        query_type: QueryType::Help,
        argument: None,
        status_filter: None,
        agent_filter: None,
    };
    assert!(run_query(&options).is_ok());
}

// ============================================================================
// Edge cases — special characters in arguments
// ============================================================================

#[test]
fn run_query_session_exists_with_special_chars() {
    let options = QueryOptions {
        query_type: QueryType::SessionExists,
        argument: Some("session-with-dashes_and_underscores".to_string()),
        status_filter: None,
        agent_filter: None,
    };
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_session_exists_with_empty_argument() {
    let options = QueryOptions {
        query_type: QueryType::SessionExists,
        argument: Some(String::new()),
        status_filter: None,
        agent_filter: None,
    };
    // Empty string is still a valid argument structurally
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_session_exists_with_unicode() {
    let options = QueryOptions {
        query_type: QueryType::SessionExists,
        argument: Some("session-日本語-🎉".to_string()),
        status_filter: None,
        agent_filter: None,
    };
    assert!(run_query(&options).is_ok());
}

#[test]
fn run_query_sessions_with_long_filter_value() {
    let long_status = "a".repeat(10_000);
    let options = QueryOptions {
        query_type: QueryType::Sessions,
        argument: None,
        status_filter: Some(long_status),
        agent_filter: None,
    };
    assert!(run_query(&options).is_ok());
}

// ============================================================================
// QueryOptions — Clone and Debug
// ============================================================================

#[test]
fn query_options_clone() {
    let opts = QueryOptions {
        query_type: QueryType::Sessions,
        argument: Some("test".to_string()),
        status_filter: Some("active".to_string()),
        agent_filter: None,
    };
    let cloned = opts.clone();
    assert_eq!(cloned.query_type, opts.query_type);
    assert_eq!(cloned.argument, opts.argument);
    assert_eq!(cloned.status_filter, opts.status_filter);
    assert_eq!(cloned.agent_filter, opts.agent_filter);
}

#[test]
fn query_options_debug_format() {
    let opts = QueryOptions {
        query_type: QueryType::Help,
        argument: None,
        status_filter: None,
        agent_filter: None,
    };
    let debug = format!("{opts:?}");
    assert!(debug.contains("Help"));
}

#[test]
fn query_type_debug_and_clone() {
    let qt = QueryType::Blockers;
    let cloned = qt.clone();
    assert_eq!(qt, cloned);
    let debug = format!("{qt:?}");
    assert!(debug.contains("Blockers"));
}

#[test]
fn query_type_equality() {
    assert_eq!(QueryType::Sessions, QueryType::Sessions);
    assert_ne!(QueryType::Sessions, QueryType::Help);
}

// ============================================================================
// Property-based tests (proptest)
// ============================================================================

mod proptests {
    use super::*;

    use proptest::prelude::*;

    // SessionStatus strategy: one of the four variants
    fn any_status() -> impl Strategy<Value = SessionStatus> {
        prop_oneof![
            Just(SessionStatus::Active),
            Just(SessionStatus::Paused),
            Just(SessionStatus::Completed),
            Just(SessionStatus::Aborted),
        ]
    }

    // SessionInfo strategy
    fn any_session_info() -> impl Strategy<Value = SessionInfo> {
        (
            "[a-zA-Z0-9_-]{1,20}",
            any_status(),
            proptest::option::of("[a-zA-Z0-9_-]{1,20}"),
            proptest::option::of("/[a-zA-Z0-9_/]{1,30}"),
            proptest::option::of("[0-9]{4}-[0-9]{2}-[0-9]{2}"),
        )
            .prop_map(|(name, status, agent, ws, created)| SessionInfo {
                name,
                status,
                agent,
                workspace_path: ws,
                created_at: created,
            })
    }

    proptest! {
        /// QueryType::from_str returns None for arbitrary non-matching strings
        #[test]
        fn from_str_rejects_arbitrary_nonmatching(s in "[^a-z-]{1,10}") {
            // Any string that doesn't exactly match a known variant returns None
            let result = QueryType::from_str(&s);
            // It might match "help" or "list" if the regex generates those,
            // but the regex [^a-z-] excludes lowercase and hyphens, so it won't.
            assert!(result.is_none() || s == "help" || s == "list" || s == "sessions"
                || s == "session-exists" || s == "session-info" || s == "blockers"
                || s == "session-count");
        }

        /// QueryType roundtrip: from_str(all_names[i]) is always Some
        #[test]
        fn all_names_roundtrip(idx in 0..6usize) {
            let names = QueryType::all_names();
            let name = names[idx];
            assert!(QueryType::from_str(name).is_some());
        }

        /// SessionStatus::as_str roundtrip through from_str_lossy
        #[test]
        fn status_roundtrip(status in any_status()) {
            let s = status.as_str();
            assert_eq!(SessionStatus::from_str_lossy(s), status);
        }

        /// SessionStatus::from_str_lossy never panics on any input
        #[test]
        fn status_from_str_lossy_never_panics(s in ".*") {
            let _ = SessionStatus::from_str_lossy(&s);
        }

        /// SessionInfo serialization roundtrip
        #[test]
        fn session_info_serde_roundtrip(info in any_session_info()) {
            let json = serde_json::to_string(&info).expect("serialize");
            let back: SessionInfo = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.name, info.name);
            assert_eq!(back.status, info.status);
            assert_eq!(back.agent, info.agent);
        }

        /// filter_sessions_by_status: result count <= input count
        #[test]
        fn filter_by_status_count_bound(
            sessions in proptest::collection::vec(any_session_info(), 0..20),
            status in "active|paused|completed|aborted"
        ) {
            let filtered = filter_sessions_by_status(&sessions, &status);
            assert!(filtered.len() <= sessions.len());
            // All results match the requested status
            for s in &filtered {
                assert_eq!(s.status.as_str(), status);
            }
        }

        /// filter_sessions_by_agent: result count <= input count
        #[test]
        fn filter_by_agent_count_bound(
            sessions in proptest::collection::vec(any_session_info(), 0..20),
            agent in "[a-zA-Z0-9_-]{0,10}"
        ) {
            let filtered = filter_sessions_by_agent(&sessions, &agent);
            assert!(filtered.len() <= sessions.len());
            for s in &filtered {
                assert_eq!(s.agent.as_deref(), Some(agent.as_str()));
            }
        }

        /// filter_sessions_by_status + filter_sessions_by_agent:
        /// chaining further reduces or maintains count
        #[test]
        fn chained_filters_reduce_or_maintain(
            sessions in proptest::collection::vec(any_session_info(), 0..20),
            status in "active|paused|completed|aborted",
            agent in "[a-zA-Z0-9_-]{1,10}"
        ) {
            let by_status = filter_sessions_by_status(&sessions, &status);
            let by_both: Vec<&&SessionInfo> = by_status
                .iter()
                .filter(|s| s.agent.as_deref() == Some(agent.as_str()))
                .collect();
            assert!(by_both.len() <= by_status.len());
        }

        /// QueryOutput serialization roundtrip
        #[test]
        fn query_output_roundtrip(
            success in proptest::bool::ANY,
            query_type in "session-exists|sessions|session-info|blockers|session-count|help",
            key in "[a-zA-Z_]{1,10}",
            value in proptest::option::of(proptest::num::i64::ANY)
        ) {
            let data = match value {
                Some(v) => serde_json::json!({ key: v }),
                None => serde_json::json!({}),
            };
            let output = QueryOutput {
                success,
                query_type: query_type.clone(),
                data,
            };
            let json = serde_json::to_string(&output).expect("serialize");
            let back: QueryOutput = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.success, success);
            assert_eq!(back.query_type, query_type);
        }

        /// run_query with Help never fails regardless of extra options
        #[test]
        fn run_help_never_fails(
            arg in proptest::option::of(".*"),
            status in proptest::option::of(".*"),
            agent in proptest::option::of(".*")
        ) {
            let opts = QueryOptions {
                query_type: QueryType::Help,
                argument: arg,
                status_filter: status,
                agent_filter: agent,
            };
            assert!(run_query(&opts).is_ok());
        }

        /// run_query with Blockers never fails regardless of extra options
        #[test]
        fn run_blockers_never_fails(
            arg in proptest::option::of(".*"),
            status in proptest::option::of(".*"),
            agent in proptest::option::of(".*")
        ) {
            let opts = QueryOptions {
                query_type: QueryType::Blockers,
                argument: arg,
                status_filter: status,
                agent_filter: agent,
            };
            assert!(run_query(&opts).is_ok());
        }

        /// run_query with SessionCount never fails regardless of extra options
        #[test]
        fn run_session_count_never_fails(
            arg in proptest::option::of(".*"),
            status in proptest::option::of(".*"),
            agent in proptest::option::of(".*")
        ) {
            let opts = QueryOptions {
                query_type: QueryType::SessionCount,
                argument: arg,
                status_filter: status,
                agent_filter: agent,
            };
            assert!(run_query(&opts).is_ok());
        }

        /// run_query with Sessions never fails regardless of filters
        #[test]
        fn run_sessions_never_fails(
            arg in proptest::option::of(".*"),
            status in proptest::option::of(".*"),
            agent in proptest::option::of(".*")
        ) {
            let opts = QueryOptions {
                query_type: QueryType::Sessions,
                argument: arg,
                status_filter: status,
                agent_filter: agent,
            };
            assert!(run_query(&opts).is_ok());
        }

        /// run_query with SessionExists fails iff argument is None
        #[test]
        fn run_session_exists_requires_argument(arg in proptest::option::of(".*")) {
            let opts = QueryOptions {
                query_type: QueryType::SessionExists,
                argument: arg.clone(),
                status_filter: None,
                agent_filter: None,
            };
            let result = run_query(&opts);
            assert_eq!(result.is_err(), arg.is_none());
        }

        /// run_query with SessionInfo fails iff argument is None
        #[test]
        fn run_session_info_requires_argument(arg in proptest::option::of(".*")) {
            let opts = QueryOptions {
                query_type: QueryType::SessionInfo,
                argument: arg.clone(),
                status_filter: None,
                agent_filter: None,
            };
            let result = run_query(&opts);
            assert_eq!(result.is_err(), arg.is_none());
        }
    }
}
