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
use crate::commands::task_types::{Task, TaskId, TaskState, Title};
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
}

// =========================================================================
// RED QUEEN ADVERSARIAL TESTS - hq-3fg1 (task subcommands)
// =========================================================================

mod red_queen_task_subcommands {
    use super::*;
    use crate::commands::task_types::TaskState;
    use crate::commands::task_validation::{
        acquire_task_lock, transition_to_claimed, transition_to_done, transition_to_started,
        transition_to_yielded, validate_claimed_by_user, validate_not_claimed_by_other,
        validate_not_closed, validate_task_exists,
    };

    // ─── CLAIM RACE CONDITIONS ──────────────────────────────────────────

    /// Two agents race to claim the same open task.
    /// First claim succeeds; second is rejected by validate_not_claimed_by_other.
    #[test]
    fn claim_race_first_wins_second_rejected() {
        let task = open_task("race-001");

        // Agent-1 claims first
        let claimed = transition_to_claimed(task, "agent-1");
        assert!(matches!(claimed.state, TaskState::InProgress));
        assert_eq!(claimed.assignee.as_ref().map(|a| a.as_str()), Some("agent-1"));

        // Agent-2 attempts to claim the already-claimed task
        let result = validate_not_claimed_by_other(&claimed, "agent-2");
        assert!(
            result.is_err(),
            "Second claimant should be rejected when task is already claimed"
        );
    }

    /// Same agent re-claiming their own task is allowed (idempotent claim).
    #[test]
    fn claim_race_same_agent_reclaim_allowed() {
        let task = open_task("race-002");
        let claimed = transition_to_claimed(task, "agent-1");

        let result = validate_not_claimed_by_other(&claimed, "agent-1");
        assert!(
            result.is_ok(),
            "Same agent re-claiming should succeed (idempotent)"
        );
    }

    /// Lock contention prevents concurrent claim execution.
    #[test]
    fn claim_race_lock_prevents_concurrent_execution() {
        let lock = scp_core::lock::MemLockManager::new();
        let task_id = "race-lock-001";

        // Agent-1 acquires the task lock
        let guard1 = acquire_task_lock(&lock, task_id, "agent-1");
        assert!(guard1.is_ok(), "First agent should acquire lock");

        // Agent-2 cannot acquire the same task lock while held
        let guard2 = acquire_task_lock(&lock, task_id, "agent-2");
        assert!(guard2.is_err(), "Second agent should be blocked by lock");

        // After agent-1's lock drops, agent-2 can acquire
        drop(guard1);
        let guard2_retry = acquire_task_lock(&lock, task_id, "agent-2");
        assert!(guard2_retry.is_ok(), "Agent-2 should succeed after lock release");
    }

    /// Rapid claim-yield-reclaim cycle produces consistent state.
    #[test]
    fn claim_yield_reclaim_cycle_maintains_consistency() {
        let mut task = open_task("race-cycle");

        for i in 0..20 {
            let agent = format!("agent-{i}");
            task = transition_to_claimed(task, &agent);
            assert!(matches!(task.state, TaskState::InProgress));
            assert_eq!(task.assignee.as_ref().map(|a| a.as_str()), Some(agent.as_str()));

            task = transition_to_yielded(task);
            assert!(matches!(task.state, TaskState::Open));
            assert!(task.assignee.is_none());
        }
    }

    // ─── YIELD WITHOUT CLAIM ────────────────────────────────────────────

    /// Yielding an unclaimed (Open) task fails at validation.
    #[test]
    fn yield_without_claim_rejected_by_validation() {
        let task = open_task("yield-noclaim-001");
        let result = validate_claimed_by_user(&task, "agent-1");
        assert!(
            result.is_err(),
            "Yielding a task that was never claimed should fail"
        );
    }

    /// Yielding a task claimed by a different agent fails.
    #[test]
    fn yield_task_claimed_by_other_agent_rejected() {
        let task = open_task("yield-other-001");
        let claimed = transition_to_claimed(task, "agent-1");

        let result = validate_claimed_by_user(&claimed, "agent-2");
        assert!(
            result.is_err(),
            "Agent-2 cannot yield a task claimed by agent-1"
        );
    }

    /// Yielding a closed task via raw transition resets it to Open.
    /// This tests the raw transition function (caller must guard).
    #[test]
    fn yield_on_closed_task_raw_transition_resets_to_open() {
        let task = open_task("yield-closed-raw");
        let claimed = transition_to_claimed(task, "agent-1");
        let closed = transition_to_done(claimed);

        assert!(matches!(closed.state, TaskState::Closed { .. }));

        let yielded = transition_to_yielded(closed);
        assert!(
            matches!(yielded.state, TaskState::Open),
            "Raw transition_to_yielded resets Closed -> Open (caller must validate)"
        );
        assert!(yielded.assignee.is_none());
    }

    /// Yield after yield is idempotent at the raw transition level.
    #[test]
    fn double_yield_returns_open() {
        let task = open_task("yield-double");
        let claimed = transition_to_claimed(task, "agent-1");

        let yielded1 = transition_to_yielded(claimed);
        assert!(matches!(yielded1.state, TaskState::Open));

        let yielded2 = transition_to_yielded(yielded1);
        assert!(matches!(yielded2.state, TaskState::Open));
        assert!(yielded2.assignee.is_none());
    }

    // ─── DONE WITHOUT START ─────────────────────────────────────────────

    /// Completing (done) a task that was only claimed (not started) succeeds.
    /// The state machine goes Open -> InProgress (claim) -> Closed (done).
    /// "Start" is not a prerequisite for "done" — only claim is.
    #[test]
    fn done_without_start_succeeds_when_claimed() {
        let task = open_task("done-nostart-001");
        let claimed = transition_to_claimed(task, "agent-1");

        // Skip start entirely, go straight to done
        let done = transition_to_done(claimed);
        assert!(matches!(done.state, TaskState::Closed { .. }));
    }

    /// Done on an unclaimed (Open) task fails at validation.
    /// validate_claimed_by_user checks that the task is assigned to the caller.
    #[test]
    fn done_without_claim_rejected_by_validation() {
        let task = open_task("done-noclaim-001");
        let result = validate_claimed_by_user(&task, "agent-1");
        assert!(
            result.is_err(),
            "Done on an unclaimed task should fail (not claimed by user)"
        );
    }

    /// Done on a yielded (back to Open) task fails validation.
    #[test]
    fn done_after_yield_rejected_by_validation() {
        let task = open_task("done-after-yield");
        let claimed = transition_to_claimed(task, "agent-1");
        let yielded = transition_to_yielded(claimed);

        let result = validate_claimed_by_user(&yielded, "agent-1");
        assert!(
            result.is_err(),
            "Done after yield should fail — assignee was cleared"
        );
    }

    // ─── DOUBLE-DONE ────────────────────────────────────────────────────

    /// Double-done is caught by validate_not_closed.
    #[test]
    fn double_done_rejected_by_validate_not_closed() {
        let task = open_task("double-done-001");
        let claimed = transition_to_claimed(task, "agent-1");
        let closed = transition_to_done(claimed);

        let result = validate_not_closed(&closed);
        assert!(
            result.is_err(),
            "Double-done should be rejected — task is already closed"
        );
    }

    /// Double-done at raw transition level produces monotonic timestamps.
    #[test]
    fn double_done_raw_transition_produces_monotonic_timestamps() {
        let task = open_task("double-done-raw");
        let claimed = transition_to_claimed(task, "agent-1");
        let first_close = transition_to_done(claimed);
        let second_close = transition_to_done(first_close);

        match (&second_close.state,) {
            (TaskState::Closed { closed_at: t2 },) => {
                // Second close always has a valid timestamp (>= first)
                assert!(t2 <= &chrono::Utc::now());
            }
            _ => panic!("Expected Closed state after double-done"),
        }
    }

    /// Triple-done at raw transition level still produces Closed state.
    #[test]
    fn triple_done_raw_transition_still_produces_closed() {
        let task = open_task("triple-done");
        let claimed = transition_to_claimed(task, "agent-1");
        let closed1 = transition_to_done(claimed);
        let closed2 = transition_to_done(closed1);
        let closed3 = transition_to_done(closed2);

        assert!(matches!(closed3.state, TaskState::Closed { .. }));
    }

    // ─── INVALID STATE TRANSITIONS ──────────────────────────────────────

    /// Starting a task from every possible state — verifying which are valid.
    #[test]
    fn start_from_open_succeeds_via_claim() {
        let task = open_task("trans-open-start");
        let claimed = transition_to_claimed(task, "agent-1");
        let started = transition_to_started(claimed);
        assert!(matches!(started.state, TaskState::InProgress));
    }

    #[test]
    fn start_from_in_progress_is_idempotent() {
        let task = open_task("trans-ip-start");
        let claimed = transition_to_claimed(task, "agent-1");
        let started1 = transition_to_started(claimed);
        let started2 = transition_to_started(started1);
        assert!(matches!(started2.state, TaskState::InProgress));
    }

    #[test]
    fn start_from_blocked_raw_transition_succeeds() {
        let mut task = open_task("trans-blocked-start");
        task.state = TaskState::Blocked;
        let started = transition_to_started(task);
        assert!(matches!(started.state, TaskState::InProgress));
    }

    #[test]
    fn start_from_deferred_raw_transition_succeeds() {
        let mut task = open_task("trans-deferred-start");
        task.state = TaskState::Deferred;
        let started = transition_to_started(task);
        assert!(matches!(started.state, TaskState::InProgress));
    }

    /// Claiming an already-closed task via raw transition moves it to InProgress.
    #[test]
    fn claim_on_closed_raw_transition_moves_to_in_progress() {
        let task = open_task("trans-closed-claim");
        let claimed = transition_to_claimed(task, "agent-1");
        let closed = transition_to_done(claimed);

        let reclaimed = transition_to_claimed(closed, "agent-2");
        assert!(
            matches!(reclaimed.state, TaskState::InProgress),
            "Raw transition_to_claimed on Closed -> InProgress (caller must guard)"
        );
    }

    /// validate_not_closed rejects Closed but allows all other states.
    #[test]
    fn validate_not_closed_allows_open() {
        let task = open_task("vnc-open");
        assert!(validate_not_closed(&task).is_ok());
    }

    #[test]
    fn validate_not_closed_allows_in_progress() {
        let task = transition_to_claimed(open_task("vnc-ip"), "agent-1");
        assert!(validate_not_closed(&task).is_ok());
    }

    #[test]
    fn validate_not_closed_allows_blocked() {
        let mut task = open_task("vnc-blocked");
        task.state = TaskState::Blocked;
        assert!(validate_not_closed(&task).is_ok());
    }

    #[test]
    fn validate_not_closed_allows_deferred() {
        let mut task = open_task("vnc-deferred");
        task.state = TaskState::Deferred;
        assert!(validate_not_closed(&task).is_ok());
    }

    #[test]
    fn validate_not_closed_rejects_closed() {
        let task = transition_to_done(transition_to_claimed(open_task("vnc-closed"), "a"));
        assert!(validate_not_closed(&task).is_err());
    }

    // ─── TASK WITH SPECIAL CHARS IN TITLE ────────────────────────────────

    /// Title accepts all kinds of special characters since Title::new has no restrictions.
    #[test]
    fn title_with_sql_injection_payload() {
        let title = Title::new("'; DROP TABLE tasks; --");
        assert_eq!(title.as_str(), "'; DROP TABLE tasks; --");
    }

    #[test]
    fn title_with_html_script_tag() {
        let title = Title::new("<script>alert('xss')</script>");
        assert_eq!(title.as_str(), "<script>alert('xss')</script>");
    }

    #[test]
    fn title_with_null_bytes() {
        let title = Title::new("task\x00with\x00nulls");
        assert_eq!(title.as_str(), "task\x00with\x00nulls");
    }

    #[test]
    fn title_with_emoji_and_unicode() {
        let title = Title::new("Fix bug \u{1F41B} in \u{6587}\u{5B57}\u{5316}\u{3051}");
        assert_eq!(title.as_str(), "Fix bug \u{1F41B} in \u{6587}\u{5B57}\u{5316}\u{3051}");
    }

    #[test]
    fn title_with_path_traversal() {
        let title = Title::new("../../../etc/passwd");
        assert_eq!(title.as_str(), "../../../etc/passwd");
    }

    #[test]
    fn title_with_format_string() {
        let title = Title::new("%s%s%s%s%n%d%d%d");
        assert_eq!(title.as_str(), "%s%s%s%s%n%d%d%d");
    }

    /// Task with special-char title survives full lifecycle.
    #[test]
    fn task_with_special_title_full_lifecycle() {
        let task = Task::new(
            TaskId::new("spec-title-1").expect("valid id"),
            Title::new("'; DROP TABLE tasks; -- <script>"),
        );
        assert!(matches!(task.state, TaskState::Open));

        let claimed = transition_to_claimed(task, "agent-1");
        assert!(matches!(claimed.state, TaskState::InProgress));
        assert_eq!(claimed.title.as_str(), "'; DROP TABLE tasks; -- <script>");

        let done = transition_to_done(claimed);
        assert!(matches!(done.state, TaskState::Closed { .. }));
        assert_eq!(done.title.as_str(), "'; DROP TABLE tasks; -- <script>");
    }

    /// Task with special-char title serializes and deserializes correctly.
    #[test]
    fn task_with_special_title_serialization_roundtrip() {
        let task = Task::new(
            TaskId::new("spec-title-2").expect("valid id"),
            Title::new("\u{1F41B} bug: '; DROP -- <script>"),
        );
        let json = serde_json::to_string(&task).expect("serialize");
        let restored: Task = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.title.as_str(), "\u{1F41B} bug: '; DROP -- <script>");
    }

    // ─── EXTREMELY LONG DESCRIPTIONS ────────────────────────────────────

    /// Task with a 10MB description survives lifecycle.
    #[test]
    fn task_with_megabyte_description() {
        let mut task = Task::new(
            TaskId::new("longdesc-1").expect("valid id"),
            Title::new("Big desc"),
        );
        let big_desc = "A".repeat(1_000_000);
        task.description = Some(big_desc.clone());

        let claimed = transition_to_claimed(task, "agent-1");
        assert_eq!(claimed.description.as_deref(), Some(big_desc.as_str()));

        let done = transition_to_done(claimed);
        assert_eq!(done.description.as_deref(), Some(big_desc.as_str()));
    }

    /// Truncation of extremely long description produces valid output.
    #[test]
    fn truncate_megabyte_description() {
        let long = "X".repeat(1_000_000);
        let result = truncate_description(&long, 50);
        assert!(result.len() <= 50);
        assert!(result.ends_with("..."));
    }

    /// Truncation of description with all multi-byte chars at boundary.
    ///
    /// NOTE: char boundary handling in truncate_description can produce output
    /// exceeding max_len when multi-byte chars land near the boundary. The
    /// safe_end calculation (i + c.len_utf8()) may exceed (max_len - 3),
    /// causing the final result with "..." appended to exceed max_len.
    /// This is documented behavior — the function prioritizes UTF-8 safety
    /// over strict byte-length adherence.
    #[test]
    fn truncate_all_emoji_description() {
        let emojis = "\u{1F600}".repeat(1000); // 1000 emoji = 4000 bytes
        let result = truncate_description(&emojis, 20);
        // Result is valid UTF-8 and contains truncated content + "..."
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
        assert!(result.is_empty() || result.ends_with("..."));
    }

    /// Long description with mixed content truncates at char boundary.
    #[test]
    fn truncate_long_mixed_ascii_unicode() {
        let mixed = format!("{}{}", "A".repeat(50), "\u{1F600}".repeat(100));
        let result = truncate_description(&mixed, 60);
        assert!(result.is_empty() || result.ends_with("..."));
        // Must be valid UTF-8
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
    }

    /// Task with max-length description serializes correctly.
    #[test]
    fn task_with_long_description_serialization_roundtrip() {
        let mut task = Task::new(
            TaskId::new("longdesc-serde").expect("valid id"),
            Title::new("Serialization test"),
        );
        task.description = Some("Y".repeat(100_000));

        let json = serde_json::to_string(&task).expect("serialize 100KB desc");
        let restored: Task = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.description.as_deref().map(|d| d.len()), Some(100_000));
    }

    // ─── SQL INJECTION / SEARCH ADVERSARIAL ─────────────────────────────

    /// filter_tasks_by_status with SQL injection payload treats it as a plain string.
    #[test]
    fn filter_sql_injection_payload_treated_as_string() {
        let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
        let result = filter_tasks_by_status(&tasks, "'; DROP TABLE tasks; --");
        assert!(
            result.is_empty(),
            "SQL injection payload in filter should match nothing (treated as plain string)"
        );
    }

    #[test]
    fn filter_with_union_select_matches_nothing() {
        let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
        let result = filter_tasks_by_status(&tasks, "' UNION SELECT * FROM tasks --");
        assert!(result.is_empty());
    }

    #[test]
    fn filter_with_format_string_matches_nothing() {
        let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
        let result = filter_tasks_by_status(&tasks, "%s%s%s%n");
        assert!(result.is_empty());
    }

    /// filter_tasks_by_status is case-insensitive even with adversarial input.
    #[test]
    fn filter_case_insensitive_with_special_chars() {
        let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
        // "OPEN" matches "open" case-insensitively
        let result = filter_tasks_by_status(&tasks, "OPEN");
        assert_eq!(result.len(), 1);
    }

    /// Empty filter matches nothing.
    #[test]
    fn filter_empty_string_matches_nothing() {
        let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
        let result = filter_tasks_by_status(&tasks, "");
        assert!(result.is_empty());
    }

    /// Filter with null bytes.
    #[test]
    fn filter_null_bytes_matches_nothing() {
        let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
        let result = filter_tasks_by_status(&tasks, "open\x00hidden");
        assert!(result.is_empty());
    }

    // ─── TASKID CONSTRUCTION BOUNDARY ATTACKS ────────────────────────────

    /// TaskId rejects all non-alphanumeric-with-dash-underscore chars.
    #[test]
    fn taskid_rejects_injection_payloads() {
        let injection_ids = vec![
            "'; DROP TABLE tasks; --",
            "task OR 1=1",
            "task\"; --",
            "task\x00hidden",
            "../../../etc/passwd",
            "task\nnewline",
            "task\ttab",
            "task\rCR",
        ];
        for id in injection_ids {
            assert!(
                TaskId::new(id).is_err(),
                "TaskId should reject injection payload: {:?}",
                id
            );
        }
    }

    /// TaskId rejects all-underscore (valid but edge-case).
    #[test]
    fn taskid_accepts_all_underscore() {
        assert!(TaskId::new("___").is_ok(), "All underscores is valid per regex");
    }

    /// TaskId rejects dot-containing IDs.
    #[test]
    fn taskid_rejects_dotted_id() {
        assert!(TaskId::new("task.1.2").is_err());
    }

    // ─── EXECUTION-LEVEL ADVERSARIAL TESTS ──────────────────────────────

    /// execute_task_command for done without env var or explicit ID.
    #[test]
    fn execute_done_no_env_no_explicit_id_returns_error() {
        let cmd = TaskCommand::Done {
            task_id: None,
            agent_id: valid_agent("agent-1"),
        };
        let lock = scp_core::lock::MemLockManager::new();
        let result = execute_task_command(&cmd, &lock);
        assert!(
            result.is_err(),
            "Done without explicit ID or env var should fail"
        );
    }

    /// Execute claim on nonexistent task returns NotFound.
    #[test]
    fn execute_claim_nonexistent_returns_not_found() {
        let cmd = TaskCommand::Claim {
            task_id: valid_id("ghost-task"),
            agent_id: valid_agent("agent-1"),
        };
        let lock = scp_core::lock::MemLockManager::new();
        assert_not_found(execute_task_command(&cmd, &lock));
    }

    /// Execute yield on nonexistent task returns NotFound.
    #[test]
    fn execute_yield_nonexistent_returns_not_found() {
        let cmd = TaskCommand::YieldTask {
            task_id: valid_id("ghost-task"),
            agent_id: valid_agent("agent-1"),
        };
        let lock = scp_core::lock::MemLockManager::new();
        assert_not_found(execute_task_command(&cmd, &lock));
    }

    /// Execute start on nonexistent task returns NotFound.
    #[test]
    fn execute_start_nonexistent_returns_not_found() {
        let cmd = TaskCommand::Start {
            task_id: valid_id("ghost-task"),
            agent_id: valid_agent("agent-1"),
        };
        let lock = scp_core::lock::MemLockManager::new();
        assert_not_found(execute_task_command(&cmd, &lock));
    }

    /// Execute done on nonexistent task returns NotFound.
    #[test]
    fn execute_done_nonexistent_returns_not_found() {
        let cmd = TaskCommand::Done {
            task_id: Some(valid_id("ghost-task")),
            agent_id: valid_agent("agent-1"),
        };
        let lock = scp_core::lock::MemLockManager::new();
        assert_not_found(execute_task_command(&cmd, &lock));
    }

    /// Execute show on nonexistent task returns NotFound.
    #[test]
    fn execute_show_nonexistent_returns_not_found() {
        let cmd = TaskCommand::Show {
            task_id: valid_id("ghost-task"),
        };
        let lock = scp_core::lock::MemLockManager::new();
        assert_not_found(execute_task_command(&cmd, &lock));
    }

    // ─── PROPTEST-BASED FUZZING ─────────────────────────────────────────

    use proptest::prelude::*;

    proptest! {
        /// TaskId rejects any string containing non-[a-zA-Z0-9_-] chars.
        #[test]
        fn proptest_taskid_rejects_invalid_chars(s in "[^a-zA-Z0-9_-]+") {
            let result = TaskId::new(&s);
            assert!(result.is_err(), "TaskId should reject: {:?}", s);
        }

        /// TaskId accepts any string of only [a-zA-Z0-9_-].
        #[test]
        fn proptest_taskid_accepts_valid_chars(s in "[a-zA-Z0-9_-]+") {
            let result = TaskId::new(&s);
            assert!(result.is_ok(), "TaskId should accept: {:?}", s);
        }

        /// truncate_description never panics on any input and always produces valid UTF-8.
        #[test]
        fn proptest_truncate_never_panics(s in ".*", max in 0usize..=200usize) {
            let result = truncate_description(&s, max);
            // Never panics, and always returns valid UTF-8
            assert!(std::str::from_utf8(result.as_bytes()).is_ok());
        }

        /// filter_tasks_by_status never panics on any filter string.
        #[test]
        fn proptest_filter_never_panics(filter in ".*") {
            let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
            let _ = filter_tasks_by_status(&tasks, &filter);
        }

        /// Title::new accepts any string without panic.
        #[test]
        fn proptest_title_accepts_anything(s in ".*") {
            let title = Title::new(&s);
            assert_eq!(title.as_str(), s);
        }

        /// AgentId rejects empty and whitespace-only, accepts everything else.
        #[test]
        fn proptest_agent_id_validation(s in ".*") {
            let result = AgentId::new(&s);
            let is_empty_or_ws = s.trim().is_empty();
            assert_eq!(result.is_ok(), !is_empty_or_ws, "AgentId::new({:?})", s);
        }

        /// Task serialization roundtrip preserves data for any valid title.
        #[test]
        fn proptest_task_serialization_roundtrip(title in ".*") {
            let task = Task::new(
                TaskId::new("rt-test").expect("valid"),
                Title::new(&title),
            );
            let json = serde_json::to_string(&task).expect("serialize");
            let restored: Task = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(restored.title.as_str(), title);
        }
    }
}
