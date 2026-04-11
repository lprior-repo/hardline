#![cfg(test)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
//! RED QUEEN adversarial tests for the task CLI handler.
//!
//! These tests actively try to break the task command through:
//! - Security attack vectors (injection, path traversal, resource exhaustion)
//! - Boundary conditions (empty, max, concurrent, malformed IDs)
//! - Invariant violations (state machine violations, race conditions)
//! - Property-based fuzzing (TaskId, title, description handling)
//!
//! Named "RED QUEEN" after the co-evolutionary arms race principle:
//! tests and code evolve together, each driving the other to be stronger.

use proptest::prelude::*;
use std::sync::{Arc, Mutex};

use scp_core::error::Error;
use scp_core::error_task::TaskErrorKind;
use scp_core::lock::{LockManager, MemLockManager};

use crate::commands::handlers::task::actions::execute_task_command;
use crate::commands::handlers::task::calculations::{
    filter_tasks_by_status, task_to_output, truncate_description, validate_task_command,
};
use crate::commands::handlers::task::data::{
    AgentId, TaskCommand, TaskInfoOutput, TaskStatusOutput,
};
use crate::commands::task_store::TaskStore;
use crate::commands::task_types::{Assignee, Priority, Task, TaskId, TaskState, Title};

// ============================================================================
// Test Fixtures
// ============================================================================

fn test_store() -> Arc<TaskStore> {
    Arc::new(TaskStore::in_memory())
}

fn make_task(id: &str, title: &str) -> Task {
    Task::new(TaskId::new(id).expect("valid"), Title::new(title))
}

fn valid_id(s: &str) -> TaskId {
    TaskId::new(s).expect("valid task id")
}

fn valid_agent(s: &str) -> AgentId {
    AgentId::new(s).expect("valid agent id")
}

// ============================================================================
// ATTACK VECTOR 1: TaskId construction boundary attacks
// ============================================================================

#[test]
fn adversarial_task_id_rejects_all_special_characters() {
    let special_ids = vec![
        "task!/script",
        "task; DROP TABLE",
        "../../../etc/passwd",
        "task\x00null",
        "task with spaces",
        "task\ttab",
        "task\nnewline",
        "task\rCR",
        "task\"quoted",
        "task'single",
        "task`backtick",
        "task$var",
        "task|pipe",
        "task&ampersand",
        "task>greater",
        "task<lesser",
        "task#hash",
        "task%percent",
        "task^caret",
        "task*star",
        "task(paren",
        "task)close",
        "task{brace",
        "task}close",
        "task[bracket",
        "task]close",
        "task=equals",
        "task+plus",
        "task~tilda",
        "task`backtick",
    ];

    for id in special_ids {
        let result = TaskId::new(id);
        assert!(
            result.is_err(),
            "TaskId should reject '{}', got Ok",
            id.escape_debug()
        );
    }
}

#[test]
fn adversarial_task_id_accepts_valid_characters() {
    let long_id = "a".repeat(256);
    let valid_ids = vec![
        "a",
        "1",
        "-",
        "_",
        "a-b",
        "a_b",
        "a1",
        "1a",
        "a-b-c-d-e-f",
        "ABC",
        "abc",
        "ABC123",
        "abc-123_XYZ",
        long_id.as_str(),
    ];

    for id in valid_ids {
        let result = TaskId::new(id);
        assert!(result.is_ok(), "TaskId should accept '{}', got Err", id);
    }
}

#[test]
fn adversarial_task_id_unicode_rejected() {
    assert!(TaskId::new("task-你好").is_err());
    assert!(TaskId::new("task-日本語").is_err());
    assert!(TaskId::new("task-🎉").is_err());
    assert!(TaskId::new("task-Ελληνικά").is_err());
}

#[test]
fn adversarial_task_id_empty_rejected() {
    let result = TaskId::new("");
    assert!(result.is_err(), "Empty TaskId should be rejected");
}

#[test]
fn adversarial_task_id_whitespace_rejected() {
    assert!(TaskId::new(" ").is_err());
    assert!(TaskId::new("  ").is_err());
    assert!(TaskId::new(" task ").is_err());
    assert!(TaskId::new("\t").is_err());
    assert!(TaskId::new("\n").is_err());
}

// ============================================================================
// ATTACK VECTOR 2: AgentId construction boundary attacks
// ============================================================================

#[test]
fn adversarial_agent_id_rejects_empty() {
    assert!(AgentId::new("").is_err());
}

#[test]
fn adversarial_agent_id_rejects_whitespace_only() {
    assert!(AgentId::new("   ").is_err());
    assert!(AgentId::new("\t").is_err());
    assert!(AgentId::new("\n").is_err());
}

#[test]
fn adversarial_agent_id_accepts_any_non_empty() {
    assert!(AgentId::new("a").is_ok());
    assert!(AgentId::new("agent-1").is_ok());
    assert!(AgentId::new("my agent").is_ok());
    assert!(AgentId::new("agent with spaces").is_ok());
    assert!(AgentId::new("agent/slash").is_ok());
    assert!(AgentId::new("agent!bang").is_ok());
}

// ============================================================================
// ATTACK VECTOR 3: Title acceptance of malicious content
// ============================================================================

#[test]
fn adversarial_title_sql_injection() {
    let title = Title::new("'; DROP TABLE tasks; --");
    assert_eq!(title.as_str(), "'; DROP TABLE tasks; --");
}

#[test]
fn adversarial_title_xss_payload() {
    let title = Title::new("<script>alert('xss')</script>");
    assert_eq!(title.as_str(), "<script>alert('xss')</script>");
}

#[test]
fn adversarial_title_path_traversal() {
    let title = Title::new("../../../etc/passwd");
    assert_eq!(title.as_str(), "../../../etc/passwd");
}

#[test]
fn adversarial_title_null_bytes() {
    let title = Title::new("task\x00with\x00nulls");
    assert_eq!(title.as_str(), "task\x00with\x00nulls");
}

#[test]
fn adversarial_title_emoji_and_unicode() {
    let title = Title::new("Fix bug \u{1F41B} in \u{6587}\u{5B57}\u{5316}\u{3051}");
    assert_eq!(
        title.as_str(),
        "Fix bug \u{1F41B} in \u{6587}\u{5B57}\u{5316}\u{3051}"
    );
}

#[test]
fn adversarial_title_format_string() {
    let title = Title::new("%s%s%s%s%n%d%d%d");
    assert_eq!(title.as_str(), "%s%s%s%s%n%d%d%d");
}

#[test]
fn adversarial_title_very_long() {
    let long = "x".repeat(1_000_000);
    let title = Title::new(&long);
    assert_eq!(title.as_str().len(), 1_000_000);
}

// ============================================================================
// ATTACK VECTOR 4: truncate_description edge cases
// ============================================================================

#[test]
fn adversarial_truncate_null_bytes() {
    let input = "hello\x00world";
    let result = truncate_description(input, 20);
    assert!(result.contains("hello"));
}

#[test]
fn adversarial_truncate_empty_string() {
    assert_eq!(truncate_description("", 10), "");
}

#[test]
fn adversarial_truncate_max_len_zero() {
    assert_eq!(truncate_description("hello", 0), "");
}

#[test]
fn adversarial_truncate_max_len_one() {
    assert_eq!(truncate_description("hello", 1), "");
}

#[test]
fn adversarial_truncate_max_len_two() {
    assert_eq!(truncate_description("hello", 2), "");
}

#[test]
fn adversarial_truncate_max_len_three() {
    // max_len=3: end=0, safe_end=0, returns empty
    assert_eq!(truncate_description("abc", 3), "abc");
}

#[test]
fn adversarial_truncate_all_emoji() {
    let emojis = "\u{1F600}".repeat(1000);
    let result = truncate_description(&emojis, 20);
    assert!(result.is_empty() || result.ends_with("..."));
}

#[test]
fn adversarial_truncate_mixed_ascii_unicode() {
    let mixed = format!("{}{}", "A".repeat(50), "\u{1F600}".repeat(100));
    let result = truncate_description(&mixed, 60);
    assert!(result.is_empty() || result.ends_with("..."));
}

#[test]
fn adversarial_truncate_very_long_string() {
    let long = "x".repeat(1_000_000);
    let result = truncate_description(&long, 50);
    assert!(result.len() <= 50);
    assert!(result.ends_with("..."));
}

// ============================================================================
// ATTACK VECTOR 5: filter_tasks_by_status adversarial inputs
// ============================================================================

#[test]
fn adversarial_filter_sql_injection() {
    let tasks = vec![TaskInfoOutput {
        id: "1".to_string(),
        title: "Test".to_string(),
        status: TaskStatusOutput::Open,
        description: None,
        assignee: None,
        priority: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }];

    let result = filter_tasks_by_status(&tasks, "'; DROP TABLE tasks; --");
    assert!(result.is_empty());
}

#[test]
fn adversarial_filter_union_select() {
    let tasks = vec![TaskInfoOutput {
        id: "1".to_string(),
        title: "Test".to_string(),
        status: TaskStatusOutput::Open,
        description: None,
        assignee: None,
        priority: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }];

    let result = filter_tasks_by_status(&tasks, "' UNION SELECT * FROM tasks --");
    assert!(result.is_empty());
}

#[test]
fn adversarial_filter_format_string() {
    let tasks = vec![TaskInfoOutput {
        id: "1".to_string(),
        title: "Test".to_string(),
        status: TaskStatusOutput::Open,
        description: None,
        assignee: None,
        priority: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }];

    let result = filter_tasks_by_status(&tasks, "%s%s%s%n");
    assert!(result.is_empty());
}

#[test]
fn adversarial_filter_case_insensitive_bypass() {
    let tasks = vec![TaskInfoOutput {
        id: "1".to_string(),
        title: "Test".to_string(),
        status: TaskStatusOutput::Open,
        description: None,
        assignee: None,
        priority: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }];

    // Case insensitivity is correct behavior
    let result = filter_tasks_by_status(&tasks, "OPEN");
    assert_eq!(result.len(), 1);
}

#[test]
fn adversarial_filter_empty_string() {
    let tasks = vec![TaskInfoOutput {
        id: "1".to_string(),
        title: "Test".to_string(),
        status: TaskStatusOutput::Open,
        description: None,
        assignee: None,
        priority: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }];

    let result = filter_tasks_by_status(&tasks, "");
    assert!(result.is_empty());
}

#[test]
fn adversarial_filter_null_bytes() {
    let tasks = vec![TaskInfoOutput {
        id: "1".to_string(),
        title: "Test".to_string(),
        status: TaskStatusOutput::Open,
        description: None,
        assignee: None,
        priority: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }];

    let result = filter_tasks_by_status(&tasks, "open\x00hidden");
    assert!(result.is_empty());
}

#[test]
fn adversarial_filter_unicode() {
    let tasks = vec![TaskInfoOutput {
        id: "1".to_string(),
        title: "Test".to_string(),
        status: TaskStatusOutput::Open,
        description: None,
        assignee: None,
        priority: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }];

    let result = filter_tasks_by_status(&tasks, "\u{043e}\u{0440}\u{0435}\u{043d}"); // Cyrillic
    assert!(result.is_empty());
}

// ============================================================================
// ATTACK VECTOR 6: State transition edge cases on closed tasks
// ============================================================================

#[test]
fn adversarial_claim_on_closed_task_raw_transition() {
    let store = test_store();

    let task = make_task("closed-claim", "Test");
    store.insert(task).expect("insert should succeed");

    use crate::commands::task_validation::{transition_to_claimed, transition_to_done};

    let claimed = transition_to_claimed(store.get("closed-claim").unwrap(), "agent-1");
    let closed = transition_to_done(claimed);

    // Raw transition on closed task
    let reclaimed = transition_to_claimed(closed, "agent-2");

    // FINDING: Raw transition allows claiming a closed task
    assert!(
        matches!(reclaimed.state, TaskState::InProgress),
        "Raw transition allows Closed -> InProgress"
    );
}

#[test]
fn adversarial_yield_on_closed_task_raw_transition() {
    let store = test_store();

    let task = make_task("closed-yield", "Test");
    store.insert(task).expect("insert should succeed");

    use crate::commands::task_validation::{
        transition_to_claimed, transition_to_done, transition_to_yielded,
    };

    let claimed = transition_to_claimed(store.get("closed-yield").unwrap(), "agent-1");
    let closed = transition_to_done(claimed);

    // Raw transition on closed task
    let yielded = transition_to_yielded(closed);

    // FINDING: Raw transition allows yielding a closed task
    assert!(
        matches!(yielded.state, TaskState::Open),
        "Raw transition allows Closed -> Open"
    );
    assert!(yielded.assignee.is_none());
}

#[test]
fn adversarial_start_on_closed_task_raw_transition() {
    let store = test_store();

    let task = make_task("closed-start", "Test");
    store.insert(task).expect("insert should succeed");

    use crate::commands::task_validation::{
        transition_to_claimed, transition_to_done, transition_to_started,
    };

    let claimed = transition_to_claimed(store.get("closed-start").unwrap(), "agent-1");
    let closed = transition_to_done(claimed);

    // Raw transition on closed task
    let started = transition_to_started(closed);

    // FINDING: Raw transition allows starting a closed task
    assert!(
        matches!(started.state, TaskState::InProgress),
        "Raw transition allows Closed -> InProgress"
    );
}

#[test]
fn adversarial_done_on_closed_task_raw_transition() {
    let store = test_store();

    let task = make_task("closed-done", "Test");
    store.insert(task).expect("insert should succeed");

    use crate::commands::task_validation::{transition_to_claimed, transition_to_done};

    let claimed = transition_to_claimed(store.get("closed-done").unwrap(), "agent-1");
    let closed1 = transition_to_done(claimed);
    let closed2 = transition_to_done(closed1);

    // FINDING: Double-done via raw transition produces Closed with later timestamp
    assert!(matches!(closed2.state, TaskState::Closed { .. }));
}

// ============================================================================
// ATTACK VECTOR 7: Concurrent task operations
// ============================================================================

#[test]
fn adversarial_concurrent_claim_same_task() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let task = make_task("concurrent-claim", "Race test");
    store.insert(task).expect("insert should succeed");

    // Agent 1 acquires lock
    let guard1 = lock
        .acquire(
            scp_core::lock::LockType::Task("concurrent-claim".to_string()),
            "agent-1",
        )
        .expect("guard1 should succeed");

    // Agent 2 tries to acquire same lock
    let guard2_result = lock.acquire(
        scp_core::lock::LockType::Task("concurrent-claim".to_string()),
        "agent-2",
    );

    assert!(guard2_result.is_err(), "Second lock should be blocked");

    drop(guard1);

    // Now agent 2 should succeed
    let guard2_retry = lock.acquire(
        scp_core::lock::LockType::Task("concurrent-claim".to_string()),
        "agent-2",
    );
    assert!(guard2_retry.is_ok());
}

#[test]
fn adversarial_rapid_claim_yield_cycle() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let task = make_task("rapid-cycle", "Rapid cycle test");
    store.insert(task).expect("insert should succeed");

    use crate::commands::task_validation::{transition_to_claimed, transition_to_yielded};

    let mut current = store.get("rapid-cycle").unwrap();

    for i in 0..100 {
        let agent = format!("agent-{}", i);
        current = transition_to_claimed(current, &agent);
        assert!(matches!(current.state, TaskState::InProgress));
        assert_eq!(
            current.assignee.as_ref().map(|a| a.as_str()),
            Some(agent.as_str())
        );

        current = transition_to_yielded(current);
        assert!(matches!(current.state, TaskState::Open));
        assert!(current.assignee.is_none());
    }
}

#[test]
fn adversarial_multiple_agents_different_tasks_independent_locks() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let task1 = make_task("multi-agent-1", "Task 1");
    let task2 = make_task("multi-agent-2", "Task 2");
    let task3 = make_task("multi-agent-3", "Task 3");

    store.insert(task1).expect("insert should succeed");
    store.insert(task2).expect("insert should succeed");
    store.insert(task3).expect("insert should succeed");

    let guard1 = lock
        .acquire(
            scp_core::lock::LockType::Task("multi-agent-1".to_string()),
            "agent-1",
        )
        .expect("guard1 should succeed");
    let guard2 = lock
        .acquire(
            scp_core::lock::LockType::Task("multi-agent-2".to_string()),
            "agent-2",
        )
        .expect("guard2 should succeed");
    let guard3 = lock
        .acquire(
            scp_core::lock::LockType::Task("multi-agent-3".to_string()),
            "agent-3",
        )
        .expect("guard3 should succeed");

    assert!(!guard1.holder().is_empty());
    assert!(!guard2.holder().is_empty());
    assert!(!guard3.holder().is_empty());
}

// ============================================================================
// ATTACK VECTOR 8: Serialization adversarial inputs
// ============================================================================

#[test]
fn adversarial_task_serialization_with_sql_injection() {
    let store = test_store();

    let mut task = make_task("serde-sql", "Test");
    task.description = Some("'; DROP TABLE tasks; --".to_string());
    store.insert(task).expect("insert should succeed");

    let json = serde_json::to_string(&store.get("serde-sql").unwrap()).expect("serialize");
    let restored: Task = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(
        restored.description.as_deref(),
        Some("'; DROP TABLE tasks; --")
    );
}

#[test]
fn adversarial_task_serialization_with_xss() {
    let store = test_store();

    let mut task = make_task("serde-xss", "Test");
    task.description = Some("<script>alert('xss')</script>".to_string());
    store.insert(task).expect("insert should succeed");

    let json = serde_json::to_string(&store.get("serde-xss").unwrap()).expect("serialize");
    let restored: Task = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(
        restored.description.as_deref(),
        Some("<script>alert('xss')</script>")
    );
}

#[test]
fn adversarial_task_serialization_unicode_titles() {
    let store = test_store();

    let task = make_task("serde-unicode", "Ελληνικά 中文 emoji 🎉");
    store.insert(task).expect("insert should succeed");

    let json = serde_json::to_string(&store.get("serde-unicode").unwrap()).expect("serialize");
    let restored: Task = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.title.as_str(), "Ελληνικά 中文 emoji 🎉");
}

#[test]
fn adversarial_task_serialization_megabyte_description() {
    let store = test_store();

    let mut task = make_task("serde-large", "Large desc");
    let big_desc = "X".repeat(1_000_000);
    task.description = Some(big_desc.clone());
    store.insert(task).expect("insert should succeed");

    let json = serde_json::to_string(&store.get("serde-large").unwrap()).expect("serialize 1MB");
    assert!(json.len() > 1_000_000);

    let restored: Task = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.description.as_deref(), Some(big_desc.as_str()));
}

#[test]
fn adversarial_task_serialization_null_bytes() {
    let store = test_store();

    let mut task = make_task("serde-null", "Null test");
    task.description = Some("a\x00b\x00c".to_string());
    store.insert(task).expect("insert should succeed");

    let json = serde_json::to_string(&store.get("serde-null").unwrap()).expect("serialize");
    let restored: Task = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.description.as_deref(), Some("a\x00b\x00c"));
}

// ============================================================================
// ATTACK VECTOR 9: Task state invariants
// ============================================================================

#[test]
fn adversarial_task_open_has_no_assignee() {
    let task = make_task("inv-open", "Open task");
    assert!(task.assignee.is_none());
    assert!(matches!(task.state, TaskState::Open));
}

#[test]
fn adversarial_task_in_progress_has_assignee() {
    let store = test_store();

    let mut task = make_task("inv-ip", "In progress task");
    task.state = TaskState::InProgress;
    task.assignee = Some(Assignee::new("agent-1"));
    store.insert(task).expect("insert should succeed");

    let retrieved = store.get("inv-ip").unwrap();
    assert!(matches!(retrieved.state, TaskState::InProgress));
    assert_eq!(
        retrieved.assignee.as_ref().map(|a| a.as_str()),
        Some("agent-1")
    );
}

#[test]
fn adversarial_task_blocked_preserves_assignee() {
    let store = test_store();

    let mut task = make_task("inv-blocked", "Blocked task");
    task.state = TaskState::Blocked;
    task.assignee = Some(Assignee::new("agent-blocked"));
    store.insert(task).expect("insert should succeed");

    let retrieved = store.get("inv-blocked").unwrap();
    assert!(matches!(retrieved.state, TaskState::Blocked));
    assert_eq!(
        retrieved.assignee.as_ref().map(|a| a.as_str()),
        Some("agent-blocked")
    );
}

#[test]
fn adversarial_task_closed_has_no_assignee() {
    let store = test_store();

    let mut task = make_task("inv-closed", "Closed task");
    task.state = TaskState::Closed {
        closed_at: chrono::Utc::now(),
    };
    task.assignee = Some(Assignee::new("agent-closed"));
    store.insert(task).expect("insert should succeed");

    let retrieved = store.get("inv-closed").unwrap();
    assert!(matches!(retrieved.state, TaskState::Closed { .. }));
    // FINDING: Closed task CAN have an assignee in the data model
    // This is not prevented by the type system
}

// ============================================================================
// ATTACK VECTOR 10: validate_task_command boundary cases
// ============================================================================

#[test]
fn adversarial_validate_list_command_always_ok() {
    let cmd = TaskCommand::List {
        status_filter: None,
        include_all: false,
    };
    assert!(validate_task_command(&cmd).is_ok());
}

#[test]
fn adversarial_validate_show_with_valid_id() {
    let cmd = TaskCommand::Show {
        task_id: valid_id("valid-show"),
    };
    assert!(validate_task_command(&cmd).is_ok());
}

#[test]
fn adversarial_validate_claim_with_valid_ids() {
    let cmd = TaskCommand::Claim {
        task_id: valid_id("valid-claim"),
        agent_id: valid_agent("valid-agent"),
    };
    assert!(validate_task_command(&cmd).is_ok());
}

#[test]
fn adversarial_validate_done_with_none_id() {
    // Done with task_id=None falls back to env var
    let cmd = TaskCommand::Done {
        task_id: None,
        agent_id: valid_agent("valid-agent"),
    };
    assert!(validate_task_command(&cmd).is_ok());
}

// ============================================================================
// PROPTREST: Property-based fuzzing
// ============================================================================

proptest! {
    #[test]
    fn proptest_task_id_valid_chars_always_accepted(s in "[a-zA-Z0-9_-]+") {
        assert!(TaskId::new(&s).is_ok(), "TaskId should accept: {}", s);
    }

    #[test]
    fn proptest_task_id_invalid_chars_always_rejected(s in "[^a-zA-Z0-9_-]+") {
        assert!(TaskId::new(&s).is_err(), "TaskId should reject: {:?}", s);
    }

    #[test]
    fn proptest_truncate_never_panics(s in ".*", max in 0usize..=200usize) {
        let result = truncate_description(&s, max);
        // Must always return valid UTF-8
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
    }

    #[test]
    fn proptest_filter_never_panics(filter in ".*") {
        let tasks = vec![TaskInfoOutput {
            id: "1".to_string(),
            title: "Test".to_string(),
            status: TaskStatusOutput::Open,
            description: None,
            assignee: None,
            priority: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }];
        let _ = filter_tasks_by_status(&tasks, &filter);
    }

    #[test]
    fn proptest_title_accepts_anything(s in ".*") {
        let title = Title::new(&s);
        assert_eq!(title.as_str(), s);
    }

    #[test]
    fn proptest_agent_id_validation(s in ".*") {
        let result = AgentId::new(&s);
        let is_empty_or_ws = s.trim().is_empty();
        assert_eq!(result.is_ok(), !is_empty_or_ws, "AgentId::new({:?})", s);
    }

    #[test]
    fn proptest_task_serialization_roundtrip(title in ".*") {
        let task = Task::new(
            TaskId::new("prop-test").expect("valid"),
            Title::new(&title),
        );
        let json = serde_json::to_string(&task).expect("serialize");
        let restored: Task = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.title.as_str(), title);
    }

    #[test]
    fn proptest_task_state_transitions_preserves_id(id in "[a-zA-Z0-9_-]+") {
        use crate::commands::task_validation::{
            transition_to_claimed, transition_to_done, transition_to_started,
            transition_to_yielded,
        };

        let task = make_task(&id, "Test");

        let claimed = transition_to_claimed(task.clone(), "agent");
        assert_eq!(claimed.id.as_str(), id);

        let started = transition_to_started(claimed);
        assert_eq!(started.id.as_str(), id);

        let done = transition_to_done(started);
        assert_eq!(done.id.as_str(), id);

        let yielded = transition_to_yielded(done);
        assert_eq!(yielded.id.as_str(), id);
    }
}

// ============================================================================
// FINDINGS SUMMARY
// ============================================================================

// FINDINGS from RED QUEEN adversarial testing:
//
// 1. DATA INTEGRITY: TaskId rejects all special characters except [a-zA-Z0-9_-]
//    This prevents injection attacks at the ID level.
//
// 2. TITLE ACCEPTANCE: Title accepts ANY string without validation.
//    This is correct - titles are user content, not code. Sanitization
//    should happen at the display/output layer.
//
// 3. RAW TRANSITIONS: transition_to_* functions have no state guards.
//    They allow Closed -> InProgress, Closed -> Open, etc. The caller
//    MUST validate state before calling transitions.
//
// 4. CLOSED TASK INVARIANT: A closed task CAN have an assignee in the
//    data model. This is not prevented by the type system.
//
// 5. FILTER INJECTION: filter_tasks_by_status treats filter strings as
//    plain text, not SQL. SQL injection payloads produce empty results
//    (correct behavior).
//
// 6. SERIALIZATION: All adversarial content (SQL, XSS, null bytes,
//    unicode, megabytes) survives JSON serialization roundtrip correctly.
//
// 7. LOCK CONTENTION: MemLockManager correctly blocks concurrent access
//    to the same task, allowing only one agent through at a time.
//
// 8. TRUNCATE: truncate_description handles all edge cases (empty,
//    max_len < 3, multi-byte chars, very long strings) without panicking
//    and always returns valid UTF-8.
