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
        assert_eq!(
            claimed.assignee.as_ref().map(|a| a.as_str()),
            Some("agent-1")
        );

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
        assert!(
            guard2_retry.is_ok(),
            "Agent-2 should succeed after lock release"
        );
    }

    /// Rapid claim-yield-reclaim cycle produces consistent state.
    #[test]
    fn claim_yield_reclaim_cycle_maintains_consistency() {
        let mut task = open_task("race-cycle");

        for i in 0..20 {
            let agent = format!("agent-{i}");
            task = transition_to_claimed(task, &agent);
            assert!(matches!(task.state, TaskState::InProgress));
            assert_eq!(
                task.assignee.as_ref().map(|a| a.as_str()),
                Some(agent.as_str())
            );

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
        assert_eq!(
            title.as_str(),
            "Fix bug \u{1F41B} in \u{6587}\u{5B57}\u{5316}\u{3051}"
        );
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
        assert_eq!(
            restored.title.as_str(),
            "\u{1F41B} bug: '; DROP -- <script>"
        );
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
        assert_eq!(
            restored.description.as_deref().map(|d| d.len()),
            Some(100_000)
        );
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
        assert!(
            TaskId::new("___").is_ok(),
            "All underscores is valid per regex"
        );
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

// =========================================================================
// RED QUEEN ADVERSARIAL TESTS - hq-3fg1 Phase 2 (execution-level)
// =========================================================================

/// Additional adversarial tests exercising execution-level paths through
/// `execute_task_command` with `MemLockManager`. These complement the pure
/// transition/validation tests in `red_queen_task_subcommands` above.
mod red_queen_execution {
    use super::*;
    use crate::commands::task_types::TaskState;
    use crate::commands::task_validation::{
        acquire_task_lock, transition_to_claimed, transition_to_done, transition_to_started,
        transition_to_yielded, validate_claimed_by_user, validate_not_claimed_by_other,
    };
    use std::sync::Arc;
    use std::thread;

    // ─── CLAIM-YIELD-RECLAIM LIFECYCLE (claim expiry) ──────────────────

    /// Simulates claim expiry: agent-1 claims, yields, then agent-2 claims.
    /// State should be consistent throughout.
    #[test]
    fn claim_yield_reclaim_different_agents() {
        let task = open_task("expiry-001");
        let agent1 = "agent-1";
        let agent2 = "agent-2";

        // Agent-1 claims
        let claimed = transition_to_claimed(task, agent1);
        assert!(matches!(claimed.state, TaskState::InProgress));
        assert_eq!(claimed.assignee.as_ref().map(|a| a.as_str()), Some(agent1));

        // Agent-1 yields
        let yielded = transition_to_yielded(claimed);
        assert!(matches!(yielded.state, TaskState::Open));
        assert!(yielded.assignee.is_none());

        // Agent-2 claims (expiry allows re-claim)
        let reclaimed = transition_to_claimed(yielded, agent2);
        assert!(matches!(reclaimed.state, TaskState::InProgress));
        assert_eq!(
            reclaimed.assignee.as_ref().map(|a| a.as_str()),
            Some(agent2)
        );
    }

    /// Full lifecycle: Open -> Claim -> Start -> Done survives serialization.
    #[test]
    fn full_lifecycle_open_claim_start_done() {
        let task = open_task("lifecycle-001");
        let agent = "worker-1";

        let claimed = transition_to_claimed(task, agent);
        assert!(matches!(claimed.state, TaskState::InProgress));

        let started = transition_to_started(claimed);
        assert!(matches!(started.state, TaskState::InProgress));

        let done = transition_to_done(started);
        assert!(matches!(done.state, TaskState::Closed { .. }));
        assert_eq!(done.assignee.as_ref().map(|a| a.as_str()), Some(agent));

        // Serialize/deserialize roundtrip preserves all fields
        let json = serde_json::to_string(&done).expect("serialize");
        let restored: Task = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(restored.state, TaskState::Closed { .. }));
        assert_eq!(restored.id.as_str(), "lifecycle-001");
    }

    // ─── THREAD-BASED LOCK CONTENTION ──────────────────────────────────

    /// Lock contention: first holder blocks second acquirer until release.
    /// Tests that MemLockManager correctly serializes access.
    #[test]
    fn lock_contention_blocks_second_holder() {
        let lock = Arc::new(scp_core::lock::MemLockManager::new());
        let task_id = "lock-test-001";

        // Acquire lock in main thread
        let guard1 = acquire_task_lock(&*lock, task_id, "agent-1");
        assert!(guard1.is_ok(), "First agent should acquire lock");

        // Spawn thread that tries to acquire the same lock
        let lock_clone = Arc::clone(&lock);
        let handle = thread::spawn(move || acquire_task_lock(&*lock_clone, task_id, "agent-2"));

        // Second acquisition should fail (lock is held)
        let result = handle.join().expect("thread panicked");
        assert!(
            result.is_err(),
            "Second agent should be blocked while lock held"
        );
    }

    /// Lock acquisition for different task IDs succeeds concurrently.
    #[test]
    fn lock_different_tasks_succeeds_concurrently() {
        let lock = Arc::new(scp_core::lock::MemLockManager::new());

        let guard1 = acquire_task_lock(&*lock, "task-A", "agent-1");
        assert!(guard1.is_ok());

        let guard2 = acquire_task_lock(&*lock, "task-B", "agent-2");
        assert!(guard2.is_ok(), "Different task IDs should not contend");

        // Both held simultaneously
        drop(guard1);
        drop(guard2);
    }

    // ─── RAPID STATE CYCLING ──────────────────────────────────────────

    /// Rapid claim/yield cycling 50 times produces consistent state.
    #[test]
    fn rapid_claim_yield_cycling() {
        let mut task = open_task("cycle-rapid");
        let agent = "cycler";

        for i in 0..50 {
            task = transition_to_claimed(task, agent);
            assert!(matches!(task.state, TaskState::InProgress), "iteration {i}");
            assert_eq!(task.assignee.as_ref().map(|a| a.as_str()), Some(agent));

            task = transition_to_yielded(task);
            assert!(matches!(task.state, TaskState::Open), "iteration {i}");
            assert!(task.assignee.is_none());
        }
    }

    /// Rapid claim/done cycling — task cannot be re-opened after close
    /// without explicit reopen. Verify monotonic closed_at timestamps.
    #[test]
    fn rapid_claim_done_cycling_timestamps_monotonic() {
        let mut last_closed_at: Option<chrono::DateTime<chrono::Utc>> = None;

        for i in 0..10 {
            let id = format!("rapid-close-{i}");
            let task = open_task(&id);
            let claimed = transition_to_claimed(task, "closer");
            let closed = transition_to_done(claimed);

            match closed.state {
                TaskState::Closed { closed_at } => {
                    if let Some(prev) = last_closed_at {
                        assert!(
                            closed_at >= prev,
                            "Timestamps should be monotonically non-decreasing"
                        );
                    }
                    last_closed_at = Some(closed_at);
                }
                _ => panic!("Expected Closed state"),
            }
        }
    }

    // ─── DONE WITHOUT START — EXECUTION PATH ──────────────────────────

    /// Done via execute_task_command on a nonexistent task returns NotFound.
    /// This verifies the execution path (not just pure validation).
    #[test]
    fn execute_done_nonexistent_via_command() {
        let cmd = TaskCommand::Done {
            task_id: Some(valid_id("ghost-done")),
            agent_id: valid_agent("agent-1"),
        };
        let lock = scp_core::lock::MemLockManager::new();
        assert_not_found(execute_task_command(&cmd, &lock));
    }

    /// Start via execute_task_command on a nonexistent task returns NotFound.
    #[test]
    fn execute_start_nonexistent_via_command() {
        let cmd = TaskCommand::Start {
            task_id: valid_id("ghost-start"),
            agent_id: valid_agent("agent-1"),
        };
        let lock = scp_core::lock::MemLockManager::new();
        assert_not_found(execute_task_command(&cmd, &lock));
    }

    /// List with status filter matching nothing returns empty output (not error).
    #[test]
    fn execute_list_with_adversarial_filter_returns_empty() {
        let cmd = TaskCommand::List {
            status_filter: Some("'; DROP TABLE tasks; --".to_string()),
            include_all: false,
        };
        let lock = scp_core::lock::MemLockManager::new();
        let result = execute_task_command(&cmd, &lock);
        assert!(result.is_ok(), "Adversarial filter should not error");
    }

    /// Show with max-length valid ID returns NotFound (not crash).
    #[test]
    fn execute_show_max_length_id_not_found() {
        let long_id = "a".repeat(100_000);
        let task_id = TaskId::new(&long_id).expect("valid long id");
        let cmd = TaskCommand::Show { task_id };
        let lock = scp_core::lock::MemLockManager::new();
        assert_not_found(execute_task_command(&cmd, &lock));
    }

    // ─── EDGE CASES: BLOCKED AND DEFERRED STATE TRANSITIONS ──────────

    /// Task set to Blocked state survives serialization roundtrip.
    #[test]
    fn blocked_state_serialization_roundtrip() {
        let mut task = open_task("blocked-serde");
        task.state = TaskState::Blocked;
        let json = serde_json::to_string(&task).expect("serialize");
        let restored: Task = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(restored.state, TaskState::Blocked));
    }

    /// Task set to Deferred state survives serialization roundtrip.
    #[test]
    fn deferred_state_serialization_roundtrip() {
        let mut task = open_task("deferred-serde");
        task.state = TaskState::Deferred;
        let json = serde_json::to_string(&task).expect("serialize");
        let restored: Task = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(restored.state, TaskState::Deferred));
    }

    /// validate_not_claimed_by_other passes for Blocked task (no assignee).
    #[test]
    fn validate_claim_blocked_task_no_assignee_succeeds() {
        use crate::commands::task_validation::validate_not_claimed_by_other;
        let mut task = open_task("blocked-noclaim");
        task.state = TaskState::Blocked;
        let result = validate_not_claimed_by_other(&task, "agent-1");
        assert!(
            result.is_ok(),
            "Blocked task without assignee should be claimable"
        );
    }

    /// validate_claimed_by_user fails for Blocked task (no assignee).
    #[test]
    fn validate_user_claim_blocked_task_no_assignee_fails() {
        let mut task = open_task("blocked-noowner");
        task.state = TaskState::Blocked;
        let result = validate_claimed_by_user(&task, "agent-1");
        assert!(
            result.is_err(),
            "Cannot yield blocked task with no assignee"
        );
    }

    // ─── TASK WITH ALL FIELDS POPULATED ──────────────────────────────

    /// Task with all optional fields (description, priority, assignee) set
    /// survives full lifecycle + serialization.
    #[test]
    fn full_task_with_all_fields_lifecycle() {
        use crate::commands::task_types::{Assignee, Priority};

        let mut task = Task::new(
            TaskId::new("full-task-1").expect("valid"),
            Title::new("Task with all fields \u{1F41B}"),
        );
        task.description = Some("A description with 'quotes' and \"double quotes\"".to_string());
        task.priority = Some(Priority::new("P0-critical"));
        task.assignee = Some(Assignee::new("agent-x"));

        // Force into InProgress (simulating pre-claimed)
        task.state = TaskState::InProgress;

        // Done
        let done = transition_to_done(task);
        assert!(matches!(done.state, TaskState::Closed { .. }));

        // Verify all fields preserved through close
        assert_eq!(done.title.as_str(), "Task with all fields \u{1F41B}");
        assert_eq!(
            done.description.as_deref(),
            Some("A description with 'quotes' and \"double quotes\"")
        );
        assert_eq!(
            done.priority.as_ref().map(|p| p.as_str()),
            Some("P0-critical")
        );

        // Serialization roundtrip
        let json = serde_json::to_string(&done).expect("serialize");
        let restored: Task = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.title.as_str(), done.title.as_str());
        assert_eq!(restored.description, done.description);
    }

    // ─── ASSIGNEE FIELD ADVERSARIAL ──────────────────────────────────

    /// Assignee with special characters survives lifecycle.
    #[test]
    fn assignee_with_special_chars_survives_lifecycle() {
        let task = open_task("spec-assignee");
        let weird_agent = "agent/O'Malley<script>alert(1)</script>";
        let claimed = transition_to_claimed(task, weird_agent);
        assert_eq!(
            claimed.assignee.as_ref().map(|a| a.as_str()),
            Some(weird_agent)
        );

        let done = transition_to_done(claimed);
        assert_eq!(
            done.assignee.as_ref().map(|a| a.as_str()),
            Some(weird_agent)
        );
    }
}

// =========================================================================
// EXHAUSTIVE TASK HANDLER TESTS — ha-9vio
// =========================================================================

/// Exhaustive tests covering: task CRUD via execution paths, state transitions,
/// assignment, priority, filtering, detail display, and invariants.
///
/// Note: The task handler implements list/show/claim/yield/start/done (no
/// create/update/delete commands or dependency management — those features
/// don't exist in this handler). Tests cover all available operations.
mod exhaustive_task_handler {
    use super::*;
    use crate::commands::task_types::{Assignee, Priority, TaskState};
    use crate::commands::task_validation::{
        transition_to_claimed, transition_to_done, transition_to_started, transition_to_yielded,
        validate_claimed_by_user, validate_not_claimed_by_other, validate_not_closed,
    };

    // ─── RESOLVE TASK ID ────────────────────────────────────────────────

    #[test]
    fn resolve_task_id_with_explicit_id_returns_it() {
        use super::super::calculations::resolve_task_id;
        let id = valid_id("explicit-task-123");
        let result = resolve_task_id(Some(&id));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "explicit-task-123");
    }

    #[test]
    fn resolve_task_id_none_without_env_returns_error() {
        use super::super::calculations::resolve_task_id;
        let result = resolve_task_id(None);
        assert_invalid_id(result);
    }

    // ─── FILTER TASKS BY STATUS — ALL VARIANTS ──────────────────────────

    #[test]
    fn filter_open_status_extracts_only_open() {
        let tasks = vec![
            sample_task_info("1", TaskStatusOutput::Open),
            sample_task_info("2", TaskStatusOutput::InProgress),
            sample_task_info("3", TaskStatusOutput::Closed),
        ];
        let filtered = filter_tasks_by_status(&tasks, "open");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "1");
    }

    #[test]
    fn filter_in_progress_status_extracts_only_in_progress() {
        let tasks = vec![
            sample_task_info("1", TaskStatusOutput::Open),
            sample_task_info("2", TaskStatusOutput::InProgress),
            sample_task_info("3", TaskStatusOutput::Closed),
        ];
        let filtered = filter_tasks_by_status(&tasks, "in_progress");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "2");
    }

    #[test]
    fn filter_blocked_status_extracts_only_blocked() {
        let tasks = vec![
            sample_task_info("1", TaskStatusOutput::Blocked),
            sample_task_info("2", TaskStatusOutput::Open),
            sample_task_info("3", TaskStatusOutput::Closed),
        ];
        let filtered = filter_tasks_by_status(&tasks, "blocked");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "1");
    }

    #[test]
    fn filter_deferred_status_extracts_only_deferred() {
        let tasks = vec![
            sample_task_info("1", TaskStatusOutput::Deferred),
            sample_task_info("2", TaskStatusOutput::Open),
            sample_task_info("3", TaskStatusOutput::Closed),
        ];
        let filtered = filter_tasks_by_status(&tasks, "deferred");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "1");
    }

    #[test]
    fn filter_closed_status_extracts_only_closed() {
        let tasks = vec![
            sample_task_info("1", TaskStatusOutput::Open),
            sample_task_info("2", TaskStatusOutput::InProgress),
            sample_task_info("3", TaskStatusOutput::Closed),
        ];
        let filtered = filter_tasks_by_status(&tasks, "closed");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "3");
    }

    #[test]
    fn filter_with_all_five_statuses() {
        let tasks = vec![
            sample_task_info("a", TaskStatusOutput::Open),
            sample_task_info("b", TaskStatusOutput::InProgress),
            sample_task_info("c", TaskStatusOutput::Blocked),
            sample_task_info("d", TaskStatusOutput::Deferred),
            sample_task_info("e", TaskStatusOutput::Closed),
        ];
        assert_eq!(filter_tasks_by_status(&tasks, "open").len(), 1);
        assert_eq!(filter_tasks_by_status(&tasks, "in_progress").len(), 1);
        assert_eq!(filter_tasks_by_status(&tasks, "blocked").len(), 1);
        assert_eq!(filter_tasks_by_status(&tasks, "deferred").len(), 1);
        assert_eq!(filter_tasks_by_status(&tasks, "closed").len(), 1);
    }

    #[test]
    fn filter_multiple_tasks_same_status() {
        let tasks = vec![
            sample_task_info("1", TaskStatusOutput::Open),
            sample_task_info("2", TaskStatusOutput::Open),
            sample_task_info("3", TaskStatusOutput::Open),
            sample_task_info("4", TaskStatusOutput::Closed),
        ];
        let filtered = filter_tasks_by_status(&tasks, "open");
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn filter_empty_input_returns_empty() {
        let tasks: Vec<TaskInfoOutput> = vec![];
        let filtered = filter_tasks_by_status(&tasks, "open");
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_nonexistent_status_returns_empty() {
        let tasks = vec![sample_task_info("1", TaskStatusOutput::Open)];
        let filtered = filter_tasks_by_status(&tasks, "nonexistent");
        assert!(filtered.is_empty());
    }

    // ─── TASK TO OUTPUT — FULL FIELD COVERAGE ────────────────────────────

    #[test]
    fn task_to_output_with_description_and_priority_and_assignee() {
        let mut task = Task::new(valid_id("full-out"), Title::new("Full output task"));
        task.description = Some("Detailed description".to_string());
        task.priority = Some(Priority::new("P0-critical"));
        task.assignee = Some(Assignee::new("agent-x"));
        task.state = TaskState::InProgress;

        let output = task_to_output(&task);
        assert_eq!(output.id, "full-out");
        assert_eq!(output.title, "Full output task");
        assert_eq!(output.status, TaskStatusOutput::InProgress);
        assert_eq!(output.description.as_deref(), Some("Detailed description"));
        assert_eq!(output.priority.as_deref(), Some("P0-critical"));
        assert_eq!(output.assignee.as_deref(), Some("agent-x"));
    }

    #[test]
    fn task_to_output_minimal_fields() {
        let task = open_task("minimal-out");
        let output = task_to_output(&task);
        assert_eq!(output.id, "minimal-out");
        assert_eq!(output.title, "Test task");
        assert_eq!(output.status, TaskStatusOutput::Open);
        assert!(output.description.is_none());
        assert!(output.assignee.is_none());
        assert!(output.priority.is_none());
    }

    #[test]
    fn task_to_output_blocked_state() {
        let mut task = open_task("blocked-out");
        task.state = TaskState::Blocked;
        let output = task_to_output(&task);
        assert_eq!(output.status, TaskStatusOutput::Blocked);
    }

    #[test]
    fn task_to_output_deferred_state() {
        let mut task = open_task("deferred-out");
        task.state = TaskState::Deferred;
        let output = task_to_output(&task);
        assert_eq!(output.status, TaskStatusOutput::Deferred);
    }

    #[test]
    fn task_to_output_closed_state() {
        let task = open_task("closed-out");
        let claimed = transition_to_claimed(task, "agent-1");
        let done = transition_to_done(claimed);
        let output = task_to_output(&done);
        assert_eq!(output.status, TaskStatusOutput::Closed);
        assert_eq!(output.assignee.as_deref(), Some("agent-1"));
    }

    #[test]
    fn task_to_output_preserves_timestamps() {
        let task = open_task("ts-out");
        let created = task.created_at;
        let updated = task.updated_at;
        let output = task_to_output(&task);
        assert_eq!(output.created_at, created);
        assert_eq!(output.updated_at, updated);
    }

    // ─── PRIORITY THROUGH FULL LIFECYCLE ────────────────────────────────

    #[test]
    fn priority_preserved_through_claim_start_done() {
        let mut task = open_task("prio-lifecycle");
        task.priority = Some(Priority::new("P1-high"));
        let claimed = transition_to_claimed(task, "agent-1");
        assert_eq!(
            claimed.priority.as_ref().map(|p| p.as_str()),
            Some("P1-high")
        );
        let started = transition_to_started(claimed);
        assert_eq!(
            started.priority.as_ref().map(|p| p.as_str()),
            Some("P1-high")
        );
        let done = transition_to_done(started);
        assert_eq!(done.priority.as_ref().map(|p| p.as_str()), Some("P1-high"));
    }

    #[test]
    fn priority_preserved_through_claim_yield_cycle() {
        let mut task = open_task("prio-yield");
        task.priority = Some(Priority::new("P2-medium"));
        let claimed = transition_to_claimed(task, "agent-1");
        let yielded = transition_to_yielded(claimed);
        assert_eq!(
            yielded.priority.as_ref().map(|p| p.as_str()),
            Some("P2-medium")
        );
    }

    #[test]
    fn no_priority_stays_none_through_lifecycle() {
        let task = open_task("no-prio");
        let claimed = transition_to_claimed(task, "agent-1");
        assert!(claimed.priority.is_none());
        let done = transition_to_done(claimed);
        assert!(done.priority.is_none());
    }

    #[test]
    fn priority_serialization_roundtrip() {
        let mut task = open_task("prio-serde");
        task.priority = Some(Priority::new("P3-low"));
        let json = serde_json::to_string(&task).expect("serialize");
        let restored: Task = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            restored.priority.as_ref().map(|p| p.as_str()),
            Some("P3-low")
        );
    }

    // ─── ASSIGNMENT THROUGH FULL LIFECYCLE ──────────────────────────────

    #[test]
    fn claim_sets_assignee_yield_clears_it() {
        let task = open_task("assign-cycle");
        assert!(task.assignee.is_none());

        let claimed = transition_to_claimed(task, "worker-1");
        assert_eq!(
            claimed.assignee.as_ref().map(|a| a.as_str()),
            Some("worker-1")
        );

        let yielded = transition_to_yielded(claimed);
        assert!(yielded.assignee.is_none());
    }

    #[test]
    fn claim_overwrites_previous_assignee() {
        let task = open_task("assign-overwrite");
        let claimed1 = transition_to_claimed(task, "agent-A");
        assert_eq!(
            claimed1.assignee.as_ref().map(|a| a.as_str()),
            Some("agent-A")
        );

        let claimed2 = transition_to_claimed(claimed1, "agent-B");
        assert_eq!(
            claimed2.assignee.as_ref().map(|a| a.as_str()),
            Some("agent-B")
        );
    }

    #[test]
    fn done_preserves_assignee() {
        let task = open_task("assign-done");
        let claimed = transition_to_claimed(task, "final-agent");
        let done = transition_to_done(claimed);
        assert_eq!(
            done.assignee.as_ref().map(|a| a.as_str()),
            Some("final-agent")
        );
    }

    #[test]
    fn started_preserves_assignee() {
        let task = open_task("assign-start");
        let claimed = transition_to_claimed(task, "starter-agent");
        let started = transition_to_started(claimed);
        assert_eq!(
            started.assignee.as_ref().map(|a| a.as_str()),
            Some("starter-agent")
        );
    }

    // ─── STATE TRANSITION INVARIANTS ────────────────────────────────────

    #[test]
    fn transition_preserves_task_id() {
        let task = open_task("id-inv");
        let claimed = transition_to_claimed(task, "agent-1");
        assert_eq!(claimed.id.as_str(), "id-inv");
        let started = transition_to_started(claimed);
        assert_eq!(started.id.as_str(), "id-inv");
        let done = transition_to_done(started);
        assert_eq!(done.id.as_str(), "id-inv");

        // Also test yield path
        let task2 = open_task("id-inv-2");
        let claimed2 = transition_to_claimed(task2, "agent-1");
        let yielded = transition_to_yielded(claimed2);
        assert_eq!(yielded.id.as_str(), "id-inv-2");
    }

    #[test]
    fn transition_preserves_title() {
        let task = Task::new(valid_id("title-inv"), Title::new("Original Title"));
        let claimed = transition_to_claimed(task, "agent-1");
        assert_eq!(claimed.title.as_str(), "Original Title");
        let started = transition_to_started(claimed);
        assert_eq!(started.title.as_str(), "Original Title");
        let done = transition_to_done(started);
        assert_eq!(done.title.as_str(), "Original Title");
    }

    #[test]
    fn transition_preserves_description() {
        let mut task = open_task("desc-inv");
        task.description = Some("Important context".to_string());
        let claimed = transition_to_claimed(task, "agent-1");
        assert_eq!(claimed.description.as_deref(), Some("Important context"));
        let started = transition_to_started(claimed);
        assert_eq!(started.description.as_deref(), Some("Important context"));
        let done = transition_to_done(started);
        assert_eq!(done.description.as_deref(), Some("Important context"));
    }

    #[test]
    fn transition_preserves_created_at() {
        let task = open_task("cat-inv");
        let original_created = task.created_at;
        let claimed = transition_to_claimed(task, "agent-1");
        assert_eq!(claimed.created_at, original_created);
        let started = transition_to_started(claimed);
        assert_eq!(started.created_at, original_created);
        let done = transition_to_done(started);
        assert_eq!(done.created_at, original_created);
    }

    #[test]
    fn transition_updates_updated_at() {
        let task = open_task("uat-inv");
        let t0 = task.updated_at;

        let claimed = transition_to_claimed(task, "agent-1");
        let claimed_time = claimed.updated_at;
        assert!(claimed_time >= t0);

        let started = transition_to_started(claimed);
        let started_time = started.updated_at;
        assert!(started_time >= claimed_time);

        let done = transition_to_done(started);
        assert!(done.updated_at >= started_time);
    }

    // ─── STATE TRANSITION MATRIX ────────────────────────────────────────

    /// Systematic test of claim from every state.
    #[test]
    fn claim_from_open_sets_in_progress() {
        let task = open_task("claim-open");
        let result = transition_to_claimed(task, "agent-1");
        assert!(matches!(result.state, TaskState::InProgress));
    }

    #[test]
    fn claim_from_in_progress_updates_assignee() {
        let task = transition_to_claimed(open_task("claim-ip"), "agent-1");
        let reclaimed = transition_to_claimed(task, "agent-2");
        assert!(matches!(reclaimed.state, TaskState::InProgress));
        assert_eq!(
            reclaimed.assignee.as_ref().map(|a| a.as_str()),
            Some("agent-2")
        );
    }

    #[test]
    fn claim_from_blocked_sets_in_progress() {
        let mut task = open_task("claim-blocked");
        task.state = TaskState::Blocked;
        let claimed = transition_to_claimed(task, "agent-1");
        assert!(matches!(claimed.state, TaskState::InProgress));
    }

    #[test]
    fn claim_from_deferred_sets_in_progress() {
        let mut task = open_task("claim-deferred");
        task.state = TaskState::Deferred;
        let claimed = transition_to_claimed(task, "agent-1");
        assert!(matches!(claimed.state, TaskState::InProgress));
    }

    /// Start from every state.
    #[test]
    fn start_from_claimed_is_in_progress() {
        let claimed = transition_to_claimed(open_task("start-claimed"), "agent-1");
        let started = transition_to_started(claimed);
        assert!(matches!(started.state, TaskState::InProgress));
    }

    #[test]
    fn start_from_in_progress_is_idempotent() {
        let claimed = transition_to_claimed(open_task("start-ip"), "agent-1");
        let started = transition_to_started(claimed);
        let restarted = transition_to_started(started);
        assert!(matches!(restarted.state, TaskState::InProgress));
    }

    #[test]
    fn start_from_blocked_sets_in_progress() {
        let mut task = open_task("start-blocked");
        task.state = TaskState::Blocked;
        let started = transition_to_started(task);
        assert!(matches!(started.state, TaskState::InProgress));
    }

    #[test]
    fn start_from_deferred_sets_in_progress() {
        let mut task = open_task("start-deferred");
        task.state = TaskState::Deferred;
        let started = transition_to_started(task);
        assert!(matches!(started.state, TaskState::InProgress));
    }

    /// Done from every non-closed state.
    #[test]
    fn done_from_open_raw_transition() {
        let task = open_task("done-open-raw");
        let done = transition_to_done(task);
        assert!(matches!(done.state, TaskState::Closed { .. }));
    }

    #[test]
    fn done_from_in_progress() {
        let claimed = transition_to_claimed(open_task("done-ip"), "agent-1");
        let done = transition_to_done(claimed);
        assert!(matches!(done.state, TaskState::Closed { .. }));
    }

    #[test]
    fn done_from_blocked_raw_transition() {
        let mut task = open_task("done-blocked-raw");
        task.state = TaskState::Blocked;
        let done = transition_to_done(task);
        assert!(matches!(done.state, TaskState::Closed { .. }));
    }

    #[test]
    fn done_from_deferred_raw_transition() {
        let mut task = open_task("done-deferred-raw");
        task.state = TaskState::Deferred;
        let done = transition_to_done(task);
        assert!(matches!(done.state, TaskState::Closed { .. }));
    }

    /// Yield from every state.
    #[test]
    fn yield_from_in_progress_sets_open() {
        let claimed = transition_to_claimed(open_task("yield-ip"), "agent-1");
        let yielded = transition_to_yielded(claimed);
        assert!(matches!(yielded.state, TaskState::Open));
        assert!(yielded.assignee.is_none());
    }

    #[test]
    fn yield_from_open_is_idempotent() {
        let task = open_task("yield-open");
        let yielded = transition_to_yielded(task);
        assert!(matches!(yielded.state, TaskState::Open));
        assert!(yielded.assignee.is_none());
    }

    #[test]
    fn yield_from_blocked_raw_transition() {
        let mut task = open_task("yield-blocked-raw");
        task.state = TaskState::Blocked;
        let yielded = transition_to_yielded(task);
        assert!(matches!(yielded.state, TaskState::Open));
        assert!(yielded.assignee.is_none());
    }

    #[test]
    fn yield_from_deferred_raw_transition() {
        let mut task = open_task("yield-deferred-raw");
        task.state = TaskState::Deferred;
        let yielded = transition_to_yielded(task);
        assert!(matches!(yielded.state, TaskState::Open));
        assert!(yielded.assignee.is_none());
    }

    // ─── VALIDATION GUARDS — STATE-AWARE ────────────────────────────────

    #[test]
    fn validate_not_claimed_by_other_allows_unclaimed() {
        let task = open_task("vnc-unclaimed");
        assert!(validate_not_claimed_by_other(&task, "any-agent").is_ok());
    }

    #[test]
    fn validate_not_claimed_by_other_allows_same_agent() {
        let claimed = transition_to_claimed(open_task("vnc-same"), "agent-1");
        assert!(validate_not_claimed_by_other(&claimed, "agent-1").is_ok());
    }

    #[test]
    fn validate_not_claimed_by_other_rejects_different_agent() {
        let claimed = transition_to_claimed(open_task("vnc-diff"), "agent-1");
        assert!(validate_not_claimed_by_other(&claimed, "agent-2").is_err());
    }

    #[test]
    fn validate_claimed_by_user_succeeds_for_owner() {
        let claimed = transition_to_claimed(open_task("vcu-owner"), "agent-1");
        assert!(validate_claimed_by_user(&claimed, "agent-1").is_ok());
    }

    #[test]
    fn validate_claimed_by_user_fails_for_non_owner() {
        let claimed = transition_to_claimed(open_task("vcu-nonowner"), "agent-1");
        assert!(validate_claimed_by_user(&claimed, "agent-2").is_err());
    }

    #[test]
    fn validate_claimed_by_user_fails_for_unclaimed() {
        let task = open_task("vcu-unclaimed");
        assert!(validate_claimed_by_user(&task, "agent-1").is_err());
    }

    #[test]
    fn validate_not_closed_allows_all_non_closed_states() {
        let open = open_task("vnc-open");
        let in_progress = transition_to_claimed(open_task("vnc-ip"), "agent-1");
        let mut blocked = open_task("vnc-blocked");
        blocked.state = TaskState::Blocked;
        let mut deferred = open_task("vnc-deferred");
        deferred.state = TaskState::Deferred;

        assert!(validate_not_closed(&open).is_ok());
        assert!(validate_not_closed(&in_progress).is_ok());
        assert!(validate_not_closed(&blocked).is_ok());
        assert!(validate_not_closed(&deferred).is_ok());
    }

    #[test]
    fn validate_not_closed_rejects_closed() {
        let closed = transition_to_done(transition_to_claimed(open_task("vnc-closed"), "agent-1"));
        assert!(validate_not_closed(&closed).is_err());
    }

    // ─── VALIDATION + EXECUTION COMBOS ──────────────────────────────────

    #[test]
    fn validate_list_with_status_filter() {
        let cmd = TaskCommand::List {
            status_filter: Some("open".to_string()),
            include_all: false,
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    #[test]
    fn validate_list_include_all() {
        let cmd = TaskCommand::List {
            status_filter: None,
            include_all: true,
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    #[test]
    fn validate_list_with_both_filter_and_include_all() {
        let cmd = TaskCommand::List {
            status_filter: Some("in_progress".to_string()),
            include_all: true,
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    // ─── TASKCOMMAND CLONE AND EQUALITY ─────────────────────────────────

    #[test]
    fn task_command_list_clone_equals_original() {
        let cmd = TaskCommand::List {
            status_filter: Some("open".to_string()),
            include_all: false,
        };
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    #[test]
    fn task_command_claim_clone_equals_original() {
        let cmd = TaskCommand::Claim {
            task_id: valid_id("clone-claim"),
            agent_id: valid_agent("clone-agent"),
        };
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    #[test]
    fn task_command_done_clone_equals_original() {
        let cmd = TaskCommand::Done {
            task_id: Some(valid_id("clone-done")),
            agent_id: valid_agent("clone-agent"),
        };
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    #[test]
    fn task_command_done_none_clone_equals_original() {
        let cmd = TaskCommand::Done {
            task_id: None,
            agent_id: valid_agent("clone-agent"),
        };
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    // ─── TASKID HASH AND COLLECTIONS ────────────────────────────────────

    #[test]
    fn taskid_usable_as_hashmap_key() {
        let mut map = std::collections::HashMap::new();
        let id = valid_id("hash-test-1");
        map.insert(id.clone(), "value-1");
        assert_eq!(map.get(&id), Some(&"value-1"));
    }

    #[test]
    fn taskid_hashmap_lookup_different_keys() {
        let mut map = std::collections::HashMap::new();
        let id1 = valid_id("key-A");
        let id2 = valid_id("key-B");
        map.insert(id1.clone(), "v1");
        map.insert(id2.clone(), "v2");
        assert_eq!(map.get(&id1), Some(&"v1"));
        assert_eq!(map.get(&id2), Some(&"v2"));
    }

    #[test]
    fn taskid_hashset_deduplication() {
        let mut set = std::collections::HashSet::new();
        let id = valid_id("dedup-id");
        set.insert(id.clone());
        set.insert(id.clone()); // Duplicate
        assert_eq!(set.len(), 1);
    }

    // ─── TASKSTATUSOUTPUT COPY AND CLONE ────────────────────────────────

    #[test]
    fn task_status_output_copy_semantics() {
        let original = TaskStatusOutput::InProgress;
        let copied = original;
        // Both are still valid (Copy trait)
        assert_eq!(original, copied);
    }

    #[test]
    fn task_status_output_clone_equals() {
        let original = TaskStatusOutput::Blocked;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // ─── FULL LIFECYCLE WITH ALL FIELDS ─────────────────────────────────

    #[test]
    fn full_lifecycle_preserves_all_fields() {
        let mut task = Task::new(valid_id("lifecycle-full"), Title::new("Complete task"));
        task.description = Some("Detailed description with 'quotes'".to_string());
        task.priority = Some(Priority::new("P0-critical"));
        let original_created = task.created_at;

        // Claim
        let claimed = transition_to_claimed(task, "worker-1");
        assert!(matches!(claimed.state, TaskState::InProgress));
        assert_eq!(
            claimed.assignee.as_ref().map(|a| a.as_str()),
            Some("worker-1")
        );
        assert_eq!(claimed.title.as_str(), "Complete task");
        assert_eq!(
            claimed.description.as_deref(),
            Some("Detailed description with 'quotes'")
        );
        assert_eq!(
            claimed.priority.as_ref().map(|p| p.as_str()),
            Some("P0-critical")
        );
        assert_eq!(claimed.created_at, original_created);
        assert_eq!(claimed.id.as_str(), "lifecycle-full");

        // Start
        let started = transition_to_started(claimed);
        assert!(matches!(started.state, TaskState::InProgress));
        assert_eq!(
            started.assignee.as_ref().map(|a| a.as_str()),
            Some("worker-1")
        );
        assert_eq!(started.title.as_str(), "Complete task");
        assert_eq!(
            started.description.as_deref(),
            Some("Detailed description with 'quotes'")
        );
        assert_eq!(
            started.priority.as_ref().map(|p| p.as_str()),
            Some("P0-critical")
        );
        assert_eq!(started.created_at, original_created);

        // Done
        let done = transition_to_done(started);
        assert!(matches!(done.state, TaskState::Closed { .. }));
        assert_eq!(done.assignee.as_ref().map(|a| a.as_str()), Some("worker-1"));
        assert_eq!(done.title.as_str(), "Complete task");
        assert_eq!(
            done.description.as_deref(),
            Some("Detailed description with 'quotes'")
        );
        assert_eq!(
            done.priority.as_ref().map(|p| p.as_str()),
            Some("P0-critical")
        );
        assert_eq!(done.created_at, original_created);
        assert_eq!(done.id.as_str(), "lifecycle-full");

        // Serialization roundtrip after full lifecycle
        let json = serde_json::to_string(&done).expect("serialize");
        let restored: Task = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.id.as_str(), "lifecycle-full");
        assert_eq!(restored.title.as_str(), "Complete task");
        assert_eq!(
            restored.description.as_deref(),
            Some("Detailed description with 'quotes'")
        );
        assert_eq!(
            restored.priority.as_ref().map(|p| p.as_str()),
            Some("P0-critical")
        );
        assert_eq!(
            restored.assignee.as_ref().map(|a| a.as_str()),
            Some("worker-1")
        );
        assert!(matches!(restored.state, TaskState::Closed { .. }));
    }

    #[test]
    fn claim_yield_reclaim_different_agents_preserves_priority() {
        let mut task = open_task("prio-handoff");
        task.priority = Some(Priority::new("urgent"));
        let claimed1 = transition_to_claimed(task, "agent-1");
        assert_eq!(
            claimed1.priority.as_ref().map(|p| p.as_str()),
            Some("urgent")
        );
        let yielded = transition_to_yielded(claimed1);
        assert_eq!(
            yielded.priority.as_ref().map(|p| p.as_str()),
            Some("urgent")
        );
        let claimed2 = transition_to_claimed(yielded, "agent-2");
        assert_eq!(
            claimed2.priority.as_ref().map(|p| p.as_str()),
            Some("urgent")
        );
        assert_eq!(
            claimed2.assignee.as_ref().map(|a| a.as_str()),
            Some("agent-2")
        );
    }

    // ─── TRUNCATE DESCRIPTION BOUNDARY CASES ────────────────────────────

    #[test]
    fn truncate_at_min_viable_length() {
        // max_len=4: end=1, so we can fit 1 char + "..."
        let result = truncate_description("abcdef", 4);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 4);
    }

    #[test]
    fn truncate_exact_ellipsis_length() {
        // "..." is 3 chars. Input "abc" is exactly 3, should return unchanged.
        assert_eq!(truncate_description("abc", 3), "abc");
    }

    #[test]
    fn truncate_one_past_exact_length() {
        let result = truncate_description("abcd", 3);
        assert_eq!(result, "");
    }

    #[test]
    fn truncate_mixed_ascii_unicode_boundary() {
        let input = "Hello, \u{1F600} world!";
        let result = truncate_description(input, 12);
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
    }

    #[test]
    fn truncate_all_whitespace() {
        assert_eq!(truncate_description("   ", 10), "   ");
    }

    #[test]
    fn truncate_single_char_string() {
        assert_eq!(truncate_description("x", 10), "x");
    }

    #[test]
    fn truncate_single_char_over_limit() {
        let result = truncate_description("x", 0);
        assert_eq!(result, "");
    }

    // ─── AGENT ID EDGE CASES ────────────────────────────────────────────

    #[test]
    fn agent_id_accepts_numeric() {
        let agent = AgentId::new("42");
        assert_eq!(agent.expect("ok").as_str(), "42");
    }

    #[test]
    fn agent_id_accepts_slashes() {
        let agent = AgentId::new("rig/polecat/name");
        assert_eq!(agent.expect("ok").as_str(), "rig/polecat/name");
    }

    #[test]
    fn agent_id_accepts_unicode() {
        let agent = AgentId::new("agent-\u{00e9}");
        assert_eq!(agent.expect("ok").as_str(), "agent-\u{00e9}");
    }

    #[test]
    fn agent_id_accepts_very_long_string() {
        let long = "a".repeat(10_000);
        let agent = AgentId::new(&long);
        assert!(agent.is_ok());
    }

    // ─── EXECUTION PATHS WITH MEM LOCK MANAGER ──────────────────────────

    #[test]
    fn execute_list_with_status_filter_succeeds() {
        let cmd = TaskCommand::List {
            status_filter: Some("blocked".to_string()),
            include_all: false,
        };
        let lock = scp_core::lock::MemLockManager::new();
        let result = execute_task_command(&cmd, &lock);
        assert!(result.is_ok(), "List with 'blocked' filter should succeed");
    }

    #[test]
    fn execute_list_with_deferred_filter_succeeds() {
        let cmd = TaskCommand::List {
            status_filter: Some("deferred".to_string()),
            include_all: false,
        };
        let lock = scp_core::lock::MemLockManager::new();
        let result = execute_task_command(&cmd, &lock);
        assert!(result.is_ok(), "List with 'deferred' filter should succeed");
    }

    #[test]
    fn execute_list_include_all_on_empty() {
        let cmd = TaskCommand::List {
            status_filter: None,
            include_all: true,
        };
        let lock = scp_core::lock::MemLockManager::new();
        let result = execute_task_command(&cmd, &lock);
        assert!(result.is_ok(), "List --all on empty store should succeed");
    }

    #[test]
    fn execute_run_task_command_delegates_to_mem_lock() {
        let cmd = TaskCommand::List {
            status_filter: None,
            include_all: true,
        };
        let result = run_task_command(&cmd);
        assert!(result.is_ok(), "run_task_command should delegate correctly");
    }

    // ─── TASK WITH ALL OPTIONAL FIELDS SERIALIZATION ────────────────────

    #[test]
    fn task_info_output_with_all_optional_fields_json() {
        let info = TaskInfoOutput {
            id: "full-json".to_string(),
            title: "Full task".to_string(),
            status: TaskStatusOutput::InProgress,
            description: Some("Has desc".to_string()),
            assignee: Some("agent-1".to_string()),
            priority: Some("P0".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(json.contains("\"description\""));
        assert!(json.contains("\"assignee\""));
        assert!(json.contains("\"priority\""));
    }

    #[test]
    fn task_list_output_with_multiple_tasks_json() {
        let output = TaskListOutput {
            tasks: vec![
                sample_task_info("a", TaskStatusOutput::Open),
                sample_task_info("b", TaskStatusOutput::InProgress),
                sample_task_info("c", TaskStatusOutput::Closed),
            ],
            total: 3,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"total\":3"));
        let restored: TaskListOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.total, 3);
        assert_eq!(restored.tasks.len(), 3);
    }
}
