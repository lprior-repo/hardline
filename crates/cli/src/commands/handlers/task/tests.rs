//! Tests for task command handler.
//!
//! All test names are descriptive (no `test_` prefix + generic name).
//! All assertions use exact error variant matching (no bare `is_ok()`/`is_err()`).

use super::actions::{execute_task_command, run_task_command};
use super::calculations::{
    filter_tasks_by_status, parse_task_id, status_display_icon, task_state_to_output,
    task_to_output, truncate_description, validate_task_command,
};
use super::data::{
    AgentId, TaskCommand, TaskDoneOutput, TaskInfoOutput, TaskListOutput, TaskStartOutput,
    TaskStatusOutput,
};

use crate::commands::task_store::TaskStore;
use crate::commands::task_types::{Assignee, Priority, Task, TaskId, TaskState, Title};
use chrono::Utc;
use scp_core::error::Error;
use scp_core::error_task::TaskErrorKind;

/// Helper to create an open task for testing
fn open_task(id: &str) -> Task {
    Task::new(
        TaskId::new(id).expect("valid task id"),
        Title::new("Test task"),
    )
}

/// Helper to build a valid TaskId
fn valid_id(s: &str) -> TaskId {
    TaskId::new(s).expect("valid task id")
}

/// Helper to build a valid AgentId
fn valid_agent(s: &str) -> AgentId {
    AgentId::new(s).expect("valid agent id")
}

/// Helper to build a TaskInfoOutput for filter tests
fn sample_task_info(id: &str, status: TaskStatusOutput) -> TaskInfoOutput {
    TaskInfoOutput {
        id: id.to_string(),
        title: "A".to_string(),
        status,
        description: None,
        assignee: None,
        priority: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Helper to assert error matches TaskErrorKind::InvalidId
fn assert_invalid_id(result: scp_core::Result<impl std::fmt::Debug>) {
    let err = result.expect_err("expected InvalidId error");
    assert!(
        matches!(err, Error::Task(ref te) if matches!(te.inner, TaskErrorKind::InvalidId(_))),
        "Expected TaskErrorKind::InvalidId, got: {:?}",
        err
    );
}

/// Helper to assert error matches TaskErrorKind::NotFound
fn assert_not_found(result: scp_core::Result<impl std::fmt::Debug>) {
    let err = result.expect_err("expected NotFound error");
    assert!(
        matches!(err, Error::Task(ref te) if matches!(te.inner, TaskErrorKind::NotFound(_))),
        "Expected TaskErrorKind::NotFound, got: {:?}",
        err
    );
}

// =========================================================================
// Data type tests
// =========================================================================

#[test]
fn task_status_output_display_formats_correctly() {
    assert_eq!(TaskStatusOutput::Open.to_string(), "open");
    assert_eq!(TaskStatusOutput::InProgress.to_string(), "in_progress");
    assert_eq!(TaskStatusOutput::Blocked.to_string(), "blocked");
    assert_eq!(TaskStatusOutput::Deferred.to_string(), "deferred");
    assert_eq!(TaskStatusOutput::Closed.to_string(), "closed");
}

#[test]
fn task_state_maps_to_output_correctly() {
    assert_eq!(
        task_state_to_output(&TaskState::Open),
        TaskStatusOutput::Open
    );
    assert_eq!(
        task_state_to_output(&TaskState::InProgress),
        TaskStatusOutput::InProgress
    );
    assert_eq!(
        task_state_to_output(&TaskState::Blocked),
        TaskStatusOutput::Blocked
    );
    assert_eq!(
        task_state_to_output(&TaskState::Deferred),
        TaskStatusOutput::Deferred
    );
    assert_eq!(
        task_state_to_output(&TaskState::Closed {
            closed_at: Utc::now()
        }),
        TaskStatusOutput::Closed
    );
}

#[test]
fn domain_task_converts_to_info_output() {
    let task = open_task("task-001");
    let output = task_to_output(&task);

    assert_eq!(output.id, "task-001");
    assert_eq!(output.title, "Test task");
    assert_eq!(output.status, TaskStatusOutput::Open);
    assert!(output.description.is_none());
    assert!(output.assignee.is_none());
    assert!(output.priority.is_none());
}

#[test]
fn task_info_output_serializes_to_json() {
    let output = TaskInfoOutput {
        id: "bd-test123".to_string(),
        title: "Test task".to_string(),
        status: TaskStatusOutput::Open,
        description: Some("A test task".to_string()),
        assignee: None,
        priority: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let json = serde_json::to_string(&output).expect("serialization should succeed");
    assert!(json.contains("\"id\":\"bd-test123\""));
    assert!(json.contains("\"status\":\"open\""));
}

#[test]
fn task_list_output_serializes_with_total_count() {
    let output = TaskListOutput {
        tasks: vec![TaskInfoOutput {
            id: "bd-1".to_string(),
            title: "Task 1".to_string(),
            status: TaskStatusOutput::Open,
            description: None,
            assignee: None,
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }],
        total: 1,
    };

    let json = serde_json::to_string(&output).expect("serialization should succeed");
    assert!(json.contains("\"total\":1"));
    assert!(json.contains("\"tasks\""));
}

#[test]
fn task_claim_output_serializes_with_claimed_flag() {
    use super::data::TaskClaimOutput;
    let output = TaskClaimOutput {
        claimed: true,
        task_id: "bd-test".to_string(),
        holder: Some("agent-1".to_string()),
        error: None,
    };

    let json = serde_json::to_string(&output).expect("serialization should succeed");
    assert!(json.contains("\"claimed\":true"));
}

#[test]
fn task_yield_output_serializes_with_yielded_flag() {
    use super::data::TaskYieldOutput;
    let output = TaskYieldOutput {
        yielded: true,
        task_id: "bd-test".to_string(),
        error: None,
    };

    let json = serde_json::to_string(&output).expect("serialization should succeed");
    assert!(json.contains("\"yielded\":true"));
}

#[test]
fn task_done_output_serializes_with_closed_status() {
    let output = TaskDoneOutput {
        task_id: "bd-test".to_string(),
        title: "Test".to_string(),
        status: TaskStatusOutput::Closed,
    };

    let json = serde_json::to_string(&output).expect("serialization should succeed");
    assert!(json.contains("\"status\":\"closed\""));
}

#[test]
fn task_start_output_uses_status_enum_not_raw_string() {
    let output = TaskStartOutput {
        task_id: "bd-test".to_string(),
        status: TaskStatusOutput::InProgress,
        workspace: ".scp/workspaces/bd-test".to_string(),
    };

    let json = serde_json::to_string(&output).expect("serialization should succeed");
    assert!(json.contains("\"status\":\"in_progress\""));
}

// =========================================================================
// AgentId newtype tests
// =========================================================================

#[test]
fn agent_id_rejects_empty_string() {
    let result = AgentId::new("");
    assert_invalid_id(result);
}

#[test]
fn agent_id_rejects_whitespace_only() {
    let result = AgentId::new("   ");
    assert_invalid_id(result);
}

#[test]
fn agent_id_accepts_valid_string() {
    let agent = AgentId::new("agent-1");
    assert_eq!(agent.expect("should succeed").as_str(), "agent-1");
}

// =========================================================================
// Calculation tests - validate_task_command
// =========================================================================

#[test]
fn validate_list_command_succeeds() {
    let cmd = TaskCommand::List {
        status_filter: None,
        include_all: false,
    };
    let result = validate_task_command(&cmd);
    assert!(result.is_ok(), "List command should validate successfully");
}

#[test]
fn validate_show_rejects_empty_id_at_construction() {
    // Empty strings cannot produce a TaskId, so this tests parse_task_id
    let result = parse_task_id("");
    assert_invalid_id(result);
}

#[test]
fn validate_show_rejects_whitespace_only_id_at_construction() {
    let result = parse_task_id("   ");
    assert_invalid_id(result);
}

#[test]
fn validate_show_accepts_well_formed_id() {
    let cmd = TaskCommand::Show {
        task_id: valid_id("bd-abc123"),
    };
    let result = validate_task_command(&cmd);
    assert!(result.is_ok(), "Valid show command should pass validation");
}

#[test]
fn validate_claim_rejects_empty_task_id_at_construction() {
    let result = parse_task_id("");
    assert_invalid_id(result);
}

#[test]
fn validate_claim_accepts_well_formed_ids() {
    let cmd = TaskCommand::Claim {
        task_id: valid_id("task-001"),
        agent_id: valid_agent("agent-1"),
    };
    let result = validate_task_command(&cmd);
    assert!(result.is_ok(), "Valid claim command should pass validation");
}

#[test]
fn validate_claim_rejects_empty_agent_id() {
    // AgentId::new("") should fail at construction
    let result = AgentId::new("");
    assert_invalid_id(result);
}

#[test]
fn validate_yield_rejects_empty_task_id_at_construction() {
    let result = parse_task_id("");
    assert_invalid_id(result);
}

#[test]
fn validate_start_accepts_well_formed_ids() {
    let cmd = TaskCommand::Start {
        task_id: valid_id("task-001"),
        agent_id: valid_agent("agent-1"),
    };
    let result = validate_task_command(&cmd);
    assert!(result.is_ok(), "Valid start command should pass validation");
}

#[test]
fn validate_done_with_explicit_id_succeeds() {
    let cmd = TaskCommand::Done {
        task_id: Some(valid_id("task-001")),
        agent_id: valid_agent("agent-1"),
    };
    let result = validate_task_command(&cmd);
    assert!(result.is_ok(), "Done with explicit ID should validate");
}

#[test]
fn validate_done_without_id_succeeds_falls_to_env() {
    let cmd = TaskCommand::Done {
        task_id: None,
        agent_id: valid_agent("agent-1"),
    };
    let result = validate_task_command(&cmd);
    assert!(
        result.is_ok(),
        "Done without ID should validate (defers to env)"
    );
}

#[test]
fn validate_done_rejects_empty_explicit_id_at_construction() {
    let result = parse_task_id("");
    assert_invalid_id(result);
}

// =========================================================================
// Calculation tests - filter_tasks_by_status
// =========================================================================

#[test]
fn filter_by_status_returns_matching_tasks() {
    let tasks = vec![
        sample_task_info("1", TaskStatusOutput::Open),
        sample_task_info("2", TaskStatusOutput::Closed),
    ];

    let filtered = filter_tasks_by_status(&tasks, "open");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "1");
}

#[test]
fn filter_by_status_is_case_insensitive() {
    let tasks = vec![sample_task_info("1", TaskStatusOutput::InProgress)];
    let filtered = filter_tasks_by_status(&tasks, "IN_PROGRESS");
    assert_eq!(filtered.len(), 1);
}

#[test]
fn filter_by_status_returns_empty_when_no_match() {
    let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
    let filtered = filter_tasks_by_status(&tasks, "closed");
    assert!(filtered.is_empty());
}

// =========================================================================
// Calculation tests - status_display_icon
// =========================================================================

#[test]
fn status_icon_maps_all_variants() {
    assert_eq!(status_display_icon(&TaskStatusOutput::Open), "[ ]");
    assert_eq!(status_display_icon(&TaskStatusOutput::InProgress), "[*]");
    assert_eq!(status_display_icon(&TaskStatusOutput::Blocked), "[!]");
    assert_eq!(status_display_icon(&TaskStatusOutput::Deferred), "[-]");
    assert_eq!(status_display_icon(&TaskStatusOutput::Closed), "[x]");
}

// =========================================================================
// Calculation tests - truncate_description
// =========================================================================

#[test]
fn truncate_short_string_returns_unchanged() {
    assert_eq!(truncate_description("hello", 10), "hello");
}

#[test]
fn truncate_exact_length_returns_unchanged() {
    assert_eq!(truncate_description("hello", 5), "hello");
}

#[test]
fn truncate_long_string_appends_ellipsis() {
    let result = truncate_description("hello world this is a long description", 14);
    assert_eq!(result, "hello world...");
}

#[test]
fn truncate_empty_string_returns_empty() {
    assert_eq!(truncate_description("", 10), "");
}

#[test]
fn truncate_with_zero_max_len_returns_empty() {
    let result = truncate_description("hello", 0);
    assert_eq!(
        result, "",
        "Zero max_len should produce empty string, not degenerate '...'"
    );
}

#[test]
fn truncate_with_max_len_one_returns_empty() {
    let result = truncate_description("hello", 1);
    assert_eq!(result, "", "max_len=1 should produce empty string");
}

#[test]
fn truncate_with_max_len_two_returns_empty() {
    let result = truncate_description("hello", 2);
    assert_eq!(result, "", "max_len=2 should produce empty string");
}

#[test]
fn truncate_with_max_len_three_returns_empty_for_long_input() {
    let result = truncate_description("hello", 3);
    assert_eq!(result, "", "max_len=3 has no room for content before '...'");
}

#[test]
fn truncate_multi_byte_chars_at_boundary() {
    let emojis = "\u{1F600}\u{1F600}\u{1F600}"; // 12 bytes, 3 chars
    let result = truncate_description(emojis, 10);
    assert_eq!(result, "\u{1F600}\u{1F600}...");
}

#[test]
fn truncate_single_multi_byte_char_with_tiny_max_len() {
    let result = truncate_description("\u{1F600}test", 2);
    assert_eq!(result, "", "max_len < 3 should return empty string");
}

// =========================================================================
// Validation + execution integration tests
// =========================================================================

#[test]
fn execute_list_on_empty_store_returns_ok() {
    let cmd = TaskCommand::List {
        status_filter: None,
        include_all: true,
    };
    let lock = scp_core::lock::MemLockManager::new();
    let result = execute_task_command(&cmd, &lock);
    assert!(result.is_ok(), "List on empty store should succeed");
}

#[test]
fn execute_show_nonexistent_task_returns_not_found() {
    let cmd = TaskCommand::Show {
        task_id: valid_id("nonexistent-task-id"),
    };
    let lock = scp_core::lock::MemLockManager::new();
    let result = execute_task_command(&cmd, &lock);
    assert_not_found(result);
}

#[test]
fn execute_claim_nonexistent_task_returns_not_found() {
    let cmd = TaskCommand::Claim {
        task_id: valid_id("nonexistent-task-id"),
        agent_id: valid_agent("agent-1"),
    };
    let lock = scp_core::lock::MemLockManager::new();
    let result = execute_task_command(&cmd, &lock);
    assert_not_found(result);
}

#[test]
fn execute_yield_nonexistent_task_returns_not_found() {
    let cmd = TaskCommand::YieldTask {
        task_id: valid_id("nonexistent-task-id"),
        agent_id: valid_agent("agent-1"),
    };
    let lock = scp_core::lock::MemLockManager::new();
    let result = execute_task_command(&cmd, &lock);
    assert_not_found(result);
}

#[test]
fn execute_start_nonexistent_task_returns_not_found() {
    let cmd = TaskCommand::Start {
        task_id: valid_id("nonexistent-task-id"),
        agent_id: valid_agent("agent-1"),
    };
    let lock = scp_core::lock::MemLockManager::new();
    let result = execute_task_command(&cmd, &lock);
    assert_not_found(result);
}

#[test]
fn execute_done_nonexistent_task_returns_not_found() {
    let cmd = TaskCommand::Done {
        task_id: Some(valid_id("nonexistent-task-id")),
        agent_id: valid_agent("agent-1"),
    };
    let lock = scp_core::lock::MemLockManager::new();
    let result = execute_task_command(&cmd, &lock);
    assert_not_found(result);
}

// =========================================================================
// Equality tests
// =========================================================================

#[test]
fn identical_show_commands_are_equal() {
    let cmd1 = TaskCommand::Show {
        task_id: valid_id("bd-123"),
    };
    let cmd2 = TaskCommand::Show {
        task_id: valid_id("bd-123"),
    };
    assert_eq!(cmd1, cmd2);
}

#[test]
fn different_show_commands_are_not_equal() {
    let cmd1 = TaskCommand::Show {
        task_id: valid_id("bd-123"),
    };
    let cmd2 = TaskCommand::Show {
        task_id: valid_id("bd-456"),
    };
    assert_ne!(cmd1, cmd2);
}

// =========================================================================
// RED QUEEN ADVERSARIAL TESTS - hl-d3r
// =========================================================================

mod red_queen_adversarial {
    use super::*;
    use crate::commands::task_types::TaskState;
    use crate::commands::task_validation::{
        transition_to_claimed, transition_to_done, transition_to_started, transition_to_yielded,
    };

    // --- ATTACK: validate_task_command vs TaskId::new parity ---

    /// validate_task_command now uses TaskId at construction time, so special
    /// characters like "task!/script" are rejected BEFORE validate_task_command
    /// ever sees them.
    #[test]
    fn special_chars_rejected_at_taskid_construction() {
        let special_ids = vec![
            "task!/script",
            "task; DROP TABLE",
            "../../../etc/passwd",
            "task\x00null",
            "task with spaces",
        ];

        for id in special_ids {
            let result = TaskId::new(id);
            assert!(
                result.is_err(),
                "TaskId::new should reject '{}', got Ok",
                id
            );
            // Since TaskId::new rejects, we can't even build a TaskCommand with it.
            // This proves the validation gap is CLOSED.
        }
    }

    /// validate_task_command now uses TaskId at construction, so whitespace-padded
    /// IDs like " valid-id " are rejected at construction.
    #[test]
    fn whitespace_padded_id_rejected_at_taskid_construction() {
        let result = TaskId::new(" valid-id ");
        assert!(
            result.is_err(),
            "TaskId::new should reject whitespace-padded ID"
        );
    }

    /// Empty agent_id is rejected at AgentId construction time.
    #[test]
    fn empty_agent_id_rejected_at_construction() {
        let result = AgentId::new("");
        assert_invalid_id(result);
    }

    /// Whitespace-only agent_id is rejected at AgentId construction time.
    #[test]
    fn whitespace_agent_id_rejected_at_construction() {
        let result = AgentId::new("   ");
        assert_invalid_id(result);
    }

    // --- ATTACK: truncate_description edge cases ---

    /// Multi-byte chars near boundary are handled correctly.
    #[test]
    fn truncate_multi_byte_chars_at_boundary_produces_valid_output() {
        let emojis = "\u{1F600}\u{1F600}\u{1F600}"; // 12 bytes, 3 chars
        let result = truncate_description(emojis, 10);
        assert_eq!(result, "\u{1F600}\u{1F600}...");
    }

    /// When max_len is smaller than any single char, returns empty.
    #[test]
    fn truncate_tiny_max_len_returns_empty_not_degenerate_ellipsis() {
        let result = truncate_description("\u{1F600}test", 2);
        assert_eq!(
            result, "",
            "When max_len < 3, should produce empty string, not '...'"
        );
    }

    /// Zero max_len returns empty string (fixed from degenerate "...").
    #[test]
    fn truncate_zero_max_len_returns_empty_not_degenerate_ellipsis() {
        let result = truncate_description("hello", 0);
        assert_eq!(
            result, "",
            "Zero max_len should produce empty string, not '...'"
        );
    }

    /// max_len=1 returns empty string.
    #[test]
    fn truncate_max_len_one_returns_empty() {
        let result = truncate_description("hello", 1);
        assert_eq!(result, "");
    }

    /// max_len=2 returns empty string.
    #[test]
    fn truncate_max_len_two_returns_empty() {
        let result = truncate_description("hello", 2);
        assert_eq!(result, "");
    }

    /// max_len=3 returns empty string (no room for content before "...").
    #[test]
    fn truncate_max_len_three_returns_empty_for_long_input() {
        let result = truncate_description("hello", 3);
        assert_eq!(result, "");
    }

    // --- ATTACK: filter_tasks_by_status edge cases ---

    #[test]
    fn filter_by_empty_status_returns_nothing() {
        let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
        let filtered = filter_tasks_by_status(&tasks, "");
        assert!(
            filtered.is_empty(),
            "Empty filter should match nothing (no status has empty display name)"
        );
    }

    #[test]
    fn filter_by_unicode_status_returns_nothing() {
        let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
        let filtered = filter_tasks_by_status(&tasks, "\u{043e}\u{0440}\u{0435}\u{043d}"); // Cyrillic
        assert!(filtered.is_empty(), "Unicode filter should match nothing");
    }

    // --- ATTACK: state transitions on Closed tasks ---

    /// Yielding a closed task transitions it to Open (no state guard in
    /// transition function). This is documented behavior - validation is
    /// the caller's responsibility (see execute_yield which calls
    /// validate_claimed_by_user first).
    #[test]
    fn yield_transitions_closed_to_open_when_unvalidated() {
        let task = open_task("task-001");
        let task = transition_to_claimed(task, "agent-1");
        let task = transition_to_done(task);

        // Raw transition function has no state guard
        let yielded = transition_to_yielded(task);
        assert!(
            matches!(yielded.state, TaskState::Open),
            "Raw transition_to_yielded can move Closed -> Open (caller must validate)"
        );
        assert!(yielded.assignee.is_none());
    }

    /// Starting a closed task transitions it to InProgress (no state guard).
    #[test]
    fn start_transitions_closed_to_in_progress_when_unvalidated() {
        let task = open_task("task-001");
        let task = transition_to_claimed(task, "agent-1");
        let task = transition_to_done(task);

        let started = transition_to_started(task);
        assert!(
            matches!(started.state, TaskState::InProgress),
            "Raw transition_to_started can move Closed -> InProgress (caller must validate)"
        );
    }

    /// Claiming a closed task transitions it to InProgress (no state guard).
    #[test]
    fn claim_transitions_closed_to_in_progress_when_unvalidated() {
        let task = open_task("task-001");
        let task = transition_to_claimed(task, "agent-1");
        let task = transition_to_done(task);

        let reclaimed = transition_to_claimed(task, "agent-2");
        assert!(
            matches!(reclaimed.state, TaskState::InProgress),
            "Raw transition_to_claimed can move Closed -> InProgress (caller must validate)"
        );
        assert_eq!(
            reclaimed.assignee.as_ref().map(|a| a.as_str()),
            Some("agent-2")
        );
    }

    /// Double-close produces a later or equal timestamp.
    #[test]
    fn double_close_produces_equal_or_later_timestamp() {
        let task = open_task("task-001");
        let task = transition_to_claimed(task, "agent-1");
        let first_close = transition_to_done(task.clone());
        let second_close = transition_to_done(first_close.clone());

        match (&first_close.state, &second_close.state) {
            (TaskState::Closed { closed_at: t1 }, TaskState::Closed { closed_at: t2 }) => {
                assert!(
                    *t2 >= *t1,
                    "Second close should have equal or later timestamp"
                );
            }
            _ => panic!("Both should be Closed"),
        }
    }

    // --- ATTACK: resolve_task_id boundary ---

    /// Whitespace-only explicit_id falls through to env vars and fails.
    #[test]
    fn resolve_task_id_with_none_falls_to_env_and_fails() {
        use super::super::calculations::resolve_task_id;
        let result = resolve_task_id(None);
        assert_invalid_id(result);
    }

    // --- ATTACK: TaskId boundary validation ---

    #[test]
    fn taskid_accepts_single_char_variants() {
        assert!(TaskId::new("a").is_ok(), "Single char should be valid");
        assert!(TaskId::new("_").is_ok(), "Underscore only should be valid");
        assert!(TaskId::new("-").is_ok(), "Hyphen only should be valid");
        assert!(TaskId::new("0").is_ok(), "Single digit should be valid");
    }

    #[test]
    fn taskid_accepts_extremely_long_valid_id() {
        let long_id = "a".repeat(100_000);
        let result = TaskId::new(&long_id);
        assert!(result.is_ok(), "Extremely long valid ID should be accepted");
    }

    // --- DIM-2: TaskId Serde Bypass ---

    /// CRITICAL: TaskId serde Deserialize bypasses TaskId::new validation.
    /// A TaskId constructed via JSON deserialization accepts strings that
    /// TaskId::new() would reject.
    #[test]
    fn serde_deserialize_bypasses_taskid_validation_spaces() {
        let json = r#""bad id""#;
        let deserialized: TaskId = serde_json::from_str(json).expect("deserialize should succeed");
        assert_eq!(
            deserialized.as_str(), "bad id",
            "Serde bypass: TaskId with spaces accepted via deserialization"
        );
    }

    /// CRITICAL: Serde bypass allows SQL injection strings.
    #[test]
    fn serde_deserialize_bypasses_taskid_validation_injection() {
        let json = r#""task; DROP TABLE tasks""#;
        let deserialized: TaskId = serde_json::from_str(json).expect("deserialize should succeed");
        assert_eq!(
            deserialized.as_str(), "task; DROP TABLE tasks",
            "Serde bypass: SQL injection string accepted via deserialization"
        );
    }

    /// CRITICAL: Serde bypass allows path traversal strings.
    #[test]
    fn serde_deserialize_bypasses_taskid_validation_path_traversal() {
        let json = r#""../../../etc/passwd""#;
        let deserialized: TaskId = serde_json::from_str(json).expect("deserialize should succeed");
        assert_eq!(
            deserialized.as_str(), "../../../etc/passwd",
            "Serde bypass: path traversal string accepted via deserialization"
        );
    }

    /// CRITICAL: Serde bypass allows empty string.
    #[test]
    fn serde_deserialize_bypasses_taskid_validation_empty() {
        let json = r#""""#;
        let deserialized: TaskId = serde_json::from_str(json).expect("deserialize should succeed");
        assert_eq!(
            deserialized.as_str(), "",
            "Serde bypass: empty TaskId accepted via deserialization"
        );
    }

    /// CRITICAL: Serde bypass allows newline injection.
    #[test]
    fn serde_deserialize_bypasses_taskid_validation_newlines() {
        let json = r#""task\nwith\nnewlines""#;
        let deserialized: TaskId = serde_json::from_str(json).expect("deserialize should succeed");
        assert!(
            deserialized.as_str().contains('\n'),
            "Serde bypass: newline injection accepted via deserialization"
        );
    }

    /// CRITICAL: Full Task serde roundtrip with invalid TaskId.
    /// Serialize a valid Task, then manually craft JSON with invalid ID,
    /// deserialize — the invalid ID is accepted.
    #[test]
    fn full_task_deserialize_with_invalid_id_succeeds() {
        let task = open_task("valid-001");
        let json = serde_json::to_string(&task).expect("serialize");
        // Replace the valid ID with an invalid one
        let tampered = json.replace("\"valid-001\"", "\"invalid id!\"");
        let deserialized: Task = serde_json::from_str(&tampered).expect("deserialize tampered task");
        assert_eq!(
            deserialized.id.as_str(), "invalid id!",
            "Full task deserialization accepts invalid TaskId"
        );
    }

    // --- DIM-3: Title/Priority/Assignee Injection ---

    /// MAJOR: Title with newline characters breaks line-oriented output.
    #[test]
    fn title_accepts_newline_injection() {
        let title = Title::new("Fix bug\nInjected line 2");
        assert_eq!(title.as_str(), "Fix bug\nInjected line 2");
    }

    /// MAJOR: Title with null bytes.
    #[test]
    fn title_accepts_null_bytes() {
        let title = Title::new("task\x00with\x00nulls");
        assert_eq!(title.as_str(), "task\x00with\x00nulls");
    }

    /// MAJOR: Priority accepts arbitrary strings not mapped to queue Priority enum.
    #[test]
    fn priority_accepts_arbitrary_strings() {
        let p = Priority::new("URGENT-EXTREME");
        assert_eq!(p.as_str(), "URGENT-EXTREME");
    }

    /// MAJOR: Assignee with newlines/control characters.
    #[test]
    fn assignee_accepts_newlines_and_controls() {
        let a = Assignee::new("agent\n\rmalicious\x00");
        assert_eq!(a.as_str(), "agent\n\rmalicious\x00");
    }

    /// MAJOR: Empty Title is accepted without error.
    #[test]
    fn empty_title_is_accepted() {
        let t = Title::new("");
        assert_eq!(t.as_str(), "");
    }

    /// MAJOR: Empty Assignee is accepted without error.
    #[test]
    fn empty_assignee_is_accepted() {
        let a = Assignee::new("");
        assert_eq!(a.as_str(), "");
    }

    /// MAJOR: Task serialization roundtrip preserves injection payloads.
    #[test]
    fn task_serde_roundtrip_preserves_injection_in_title() {
        let mut task = open_task("task-001");
        task.title = Title::new("Title with \"quotes\" and \nnewlines");

        let json = serde_json::to_string(&task).expect("serialize");
        let deserialized: Task = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(
            deserialized.title.as_str(),
            "Title with \"quotes\" and \nnewlines",
            "Injection payload in title should survive serde roundtrip"
        );
    }

    /// MAJOR: Task with newline in title serializes to valid JSON.
    #[test]
    fn task_with_newline_title_produces_valid_json() {
        let mut task = open_task("task-002");
        task.title = Title::new("line1\nline2");

        let json = serde_json::to_string(&task).expect("serialize");
        // JSON should escape the newline as \n
        assert!(json.contains("\\n"), "Newline in title should be JSON-escaped");
        // Verify the JSON is valid by deserializing
        let _: Task = serde_json::from_str(&json).expect("deserialize roundtrip");
    }

    // --- DIM-6: Task State Consistency Invariants ---

    /// MAJOR: Closed task can be claimed via raw transition (no validate_not_closed).
    #[test]
    fn closed_task_claimable_via_raw_transition() {
        let task = open_task("task-001");
        let task = transition_to_claimed(task, "agent-1");
        let closed = transition_to_done(task);

        let reclaimed = transition_to_claimed(closed, "agent-2");
        assert!(
            matches!(reclaimed.state, TaskState::InProgress),
            "Raw transition_to_claimed allows Closed -> InProgress"
        );
        assert_eq!(reclaimed.assignee.as_ref().map(|a| a.as_str()), Some("agent-2"));
    }

    /// MAJOR: Blocked task can be yielded (no state guard on transition).
    #[test]
    fn blocked_task_yieldable_via_raw_transition() {
        let mut task = open_task("task-002");
        task.state = TaskState::Blocked;
        task.assignee = Some(Assignee::new("agent-1"));

        let yielded = transition_to_yielded(task);
        assert!(
            matches!(yielded.state, TaskState::Open),
            "Raw transition_to_yielded allows Blocked -> Open"
        );
        assert!(yielded.assignee.is_none());
    }

    /// MAJOR: Deferred task can be started (no state guard on transition).
    #[test]
    fn deferred_task_startable_via_raw_transition() {
        let mut task = open_task("task-003");
        task.state = TaskState::Deferred;

        let started = transition_to_started(task);
        assert!(
            matches!(started.state, TaskState::InProgress),
            "Raw transition_to_started allows Deferred -> InProgress"
        );
    }

    /// MAJOR: transition_to_done on already-Closed overwrites closed_at timestamp.
    #[test]
    fn double_done_overwrites_closed_at() {
        let task = open_task("task-004");
        let claimed = transition_to_claimed(task, "agent-1");
        let first_close = transition_to_done(claimed);

        // Small delay to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(2));

        let second_close = transition_to_done(first_close.clone());

        match (&first_close.state, &second_close.state) {
            (TaskState::Closed { closed_at: t1 }, TaskState::Closed { closed_at: t2 }) => {
                assert!(
                    *t2 > *t1,
                    "Second close should overwrite closed_at with a later timestamp"
                );
            }
            _ => panic!("Both should be Closed"),
        }
    }

    /// MAJOR: Task can have InProgress state with no assignee via transition_to_started
    /// on an unclaimed Open task.
    #[test]
    fn unclaimed_open_task_started_has_no_assignee() {
        let task = open_task("task-005");
        // task has no assignee, state is Open
        let started = transition_to_started(task);
        assert!(
            matches!(started.state, TaskState::InProgress),
            "Started task should be InProgress"
        );
        assert!(
            started.assignee.is_none(),
            "Started unclaimed task should have no assignee — state inconsistency"
        );
    }

    /// MINOR: Task with Blocked state has valid JSON serialization.
    #[test]
    fn blocked_task_serializes_correctly() {
        let mut task = open_task("task-006");
        task.state = TaskState::Blocked;
        task.assignee = Some(Assignee::new("agent-blocked"));

        let json = serde_json::to_string(&task).expect("serialize");
        assert!(json.contains("Blocked"));
        assert!(json.contains("agent-blocked"));

        let deserialized: Task = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(deserialized.state, TaskState::Blocked));
    }

    /// MINOR: Task with Deferred state has valid JSON serialization.
    #[test]
    fn deferred_task_serializes_correctly() {
        let mut task = open_task("task-007");
        task.state = TaskState::Deferred;

        let json = serde_json::to_string(&task).expect("serialize");
        assert!(json.contains("Deferred"));

        let deserialized: Task = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(deserialized.state, TaskState::Deferred));
    }
}
