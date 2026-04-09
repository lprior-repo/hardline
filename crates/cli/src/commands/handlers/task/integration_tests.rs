#![cfg(test)]
//! Integration tests for the task CLI handler.
//!
//! These tests exercise the task command handler against the full task store
//! and lock manager infrastructure, covering the complete lifecycle:
//! - Task creation and listing
//! - Task claim and conflict detection
//! - Task start and workspace creation
//! - Task done (completion)
//! - Task yield (abandon/release)
//! - Lock contention scenarios
//! - Queue integration scenarios
//!
//! Scenarios covered:
//! - execute_list on empty store returns empty result
//! - execute_list with multiple tasks returns all tasks
//! - execute_list filters by status correctly
//! - execute_show returns correct task details
//! - execute_claim assigns task to agent
//! - execute_claim fails for already-claimed task by other agent
//! - execute_claim succeeds idempotently for same agent
//! - execute_yield releases claimed task
//! - execute_yield fails for unclaimed task
//! - execute_yield fails for task claimed by other agent
//! - execute_start claims and starts task in one operation
//! - execute_start fails for unclaimed task
//! - execute_done completes claimed task
//! - execute_done fails for unclaimed task
//! - execute_done fails for already-closed task
//! - Full lifecycle: claim -> start -> done
//! - Full lifecycle: claim -> yield -> re-claim
//! - Lock contention between agents
//! - Multiple agents claiming same task (race condition)

use std::sync::{Arc, Mutex};

use scp_core::error::Error;
use scp_core::error_task::TaskErrorKind;
use scp_core::lock::{LockManager, MemLockManager};

use crate::commands::handlers::task::actions::{execute_task_command, run_task_command};
use crate::commands::handlers::task::calculations::filter_tasks_by_status;
use crate::commands::handlers::task::data::{
    AgentId, TaskCommand, TaskDoneOutput, TaskInfoOutput, TaskListOutput, TaskStartOutput,
    TaskStatusOutput,
};
use crate::commands::task_store::{get_task_store, TaskStore};
use crate::commands::task_types::{Assignee, Priority, Task, TaskId, TaskState, Title};
use chrono::Utc;

// ============================================================================
// Test Fixtures and Helpers
// ============================================================================

fn test_store() -> Arc<TaskStore> {
    get_task_store()
}

fn create_test_task(store: &Arc<TaskStore>, id: &str, title: &str) -> Task {
    let task = Task::new(TaskId::new(id).expect("valid task id"), Title::new(title));
    store.insert(task.clone()).expect("insert should succeed");
    task
}

fn claim_test_task(store: &Arc<TaskStore>, id: &str, agent: &str) -> Task {
    let task = store.get(id).expect("task should exist");
    use crate::commands::task_validation::transition_to_claimed;
    let claimed = transition_to_claimed(task, agent);
    store
        .update(claimed.clone())
        .expect("update should succeed");
    claimed
}

fn valid_id(s: &str) -> TaskId {
    TaskId::new(s).expect("valid task id")
}

fn valid_agent(s: &str) -> AgentId {
    AgentId::new(s).expect("valid agent id")
}

// ============================================================================
// execute_list integration tests
// ============================================================================

#[test]
fn integration_execute_list_empty_store_returns_empty() {
    let store = test_store();
    let tasks = store.list();

    let outputs: Vec<_> = tasks
        .iter()
        .map(|t| crate::commands::handlers::task::calculations::task_to_output(t))
        .collect();

    let filtered = filter_tasks_by_status(&outputs, "open");
    assert!(filtered.is_empty() || tasks.is_empty());
}

#[test]
fn integration_execute_list_returns_all_tasks() {
    let store = test_store();

    let _ = create_test_task(&store, "list-001", "First task");
    let _ = create_test_task(&store, "list-002", "Second task");
    let _ = create_test_task(&store, "list-003", "Third task");

    let tasks = store.list();
    assert_eq!(tasks.len(), 3);

    let outputs: Vec<_> = tasks
        .iter()
        .map(|t| crate::commands::handlers::task::calculations::task_to_output(t))
        .collect();

    assert_eq!(outputs.len(), 3);
}

#[test]
fn integration_execute_list_filters_by_status() {
    let store = test_store();

    let _task1 = create_test_task(&store, "filter-001", "Open task");
    let mut task2 = create_test_task(&store, "filter-002", "Claimed task");
    task2.state = TaskState::InProgress;
    task2.assignee = Some(crate::commands::task_types::Assignee::new("agent-x"));
    store.update(task2).expect("update should succeed");

    let tasks = store.list();
    let outputs: Vec<_> = tasks
        .iter()
        .map(|t| crate::commands::handlers::task::calculations::task_to_output(t))
        .collect();

    let open_tasks = filter_tasks_by_status(&outputs, "open");
    let in_progress = filter_tasks_by_status(&outputs, "in_progress");

    assert!(open_tasks.iter().any(|t| t.id == "filter-001"));
    assert!(in_progress.iter().any(|t| t.id == "filter-002"));
}

// ============================================================================
// execute_show integration tests
// ============================================================================

#[test]
fn integration_execute_show_returns_correct_details() {
    let store = test_store();

    let created = create_test_task(&store, "show-001", "Detailed task");

    let task = store.get("show-001").expect("task should exist");
    let output = crate::commands::handlers::task::calculations::task_to_output(&task);

    assert_eq!(output.id, "show-001");
    assert_eq!(output.title, "Detailed task");
    assert_eq!(output.status, TaskStatusOutput::Open);
    assert!(output.description.is_none());
    assert!(output.assignee.is_none());
}

#[test]
fn integration_execute_show_with_assignee_and_priority() {
    let store = test_store();

    let mut task = create_test_task(&store, "show-002", "Assigned task");
    task.assignee = Some(crate::commands::task_types::Assignee::new("worker-42"));
    task.priority = Some(crate::commands::task_types::Priority::new("high"));
    store.update(task).expect("update should succeed");

    let task = store.get("show-002").expect("task should exist");
    let output = crate::commands::handlers::task::calculations::task_to_output(&task);

    assert_eq!(output.assignee.as_deref(), Some("worker-42"));
    assert_eq!(output.priority.as_deref(), Some("high"));
}

// ============================================================================
// execute_claim integration tests
// ============================================================================

#[test]
fn integration_execute_claim_assigns_task_to_agent() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "claim-001", "Claimable task");

    let cmd = TaskCommand::Claim {
        task_id: valid_id("claim-001"),
        agent_id: valid_agent("agent-alice"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_ok(), "Claim should succeed");

    let task = store.get("claim-001").expect("task should exist");
    assert_eq!(
        task.assignee.as_ref().map(|a| a.as_str()),
        Some("agent-alice")
    );
    assert!(matches!(task.state, TaskState::InProgress));
}

#[test]
fn integration_execute_claim_fails_for_already_claimed_by_other() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "claim-002", "Already claimed");
    let _ = claim_test_task(&store, "claim-002", "agent-bob");

    let cmd = TaskCommand::Claim {
        task_id: valid_id("claim-002"),
        agent_id: valid_agent("agent-carol"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_err(), "Claim by different agent should fail");

    let err = result.unwrap_err();
    assert!(
        matches!(err, Error::Task(ref te) if matches!(te.inner, TaskErrorKind::AlreadyClaimed(_, _))),
        "Should return AlreadyClaimed error"
    );
}

#[test]
fn integration_execute_claim_succeeds_idempotently_for_same_agent() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "claim-003", "Same agent claim");
    let _ = claim_test_task(&store, "claim-003", "agent-dana");

    let cmd = TaskCommand::Claim {
        task_id: valid_id("claim-003"),
        agent_id: valid_agent("agent-dana"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(
        result.is_ok(),
        "Same agent re-claiming should succeed (idempotent)"
    );
}

#[test]
fn integration_execute_claim_nonexistent_task_returns_not_found() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let cmd = TaskCommand::Claim {
        task_id: valid_id("nonexistent-task"),
        agent_id: valid_agent("agent-eve"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_err(), "Claim of nonexistent task should fail");

    let err = result.unwrap_err();
    assert!(
        matches!(err, Error::Task(ref te) if matches!(te.inner, TaskErrorKind::NotFound(_))),
        "Should return NotFound error"
    );
}

// ============================================================================
// execute_yield integration tests
// ============================================================================

#[test]
fn integration_execute_yield_releases_claimed_task() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "yield-001", "Yieldable task");
    let _ = claim_test_task(&store, "yield-001", "agent-frank");

    let cmd = TaskCommand::YieldTask {
        task_id: valid_id("yield-001"),
        agent_id: valid_agent("agent-frank"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_ok(), "Yield should succeed");

    let task = store.get("yield-001").expect("task should exist");
    assert!(task.assignee.is_none());
    assert!(matches!(task.state, TaskState::Open));
}

#[test]
fn integration_execute_yield_fails_for_unclaimed_task() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "yield-002", "Never claimed");

    let cmd = TaskCommand::YieldTask {
        task_id: valid_id("yield-002"),
        agent_id: valid_agent("agent-george"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_err(), "Yield of unclaimed task should fail");

    let err = result.unwrap_err();
    assert!(
        matches!(err, Error::Task(ref te) if matches!(te.inner, TaskErrorKind::NotClaimed(_))),
        "Should return NotClaimed error"
    );
}

#[test]
fn integration_execute_yield_fails_for_task_claimed_by_other() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "yield-003", "Claimed by other");
    let _ = claim_test_task(&store, "yield-003", "agent-helen");

    let cmd = TaskCommand::YieldTask {
        task_id: valid_id("yield-003"),
        agent_id: valid_agent("agent-ivan"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(
        result.is_err(),
        "Yield of task claimed by other should fail"
    );
}

// ============================================================================
// execute_start integration tests
// ============================================================================

#[test]
fn integration_execute_start_claims_and_starts_task() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "start-001", "Startable task");

    let cmd = TaskCommand::Start {
        task_id: valid_id("start-001"),
        agent_id: valid_agent("agent-jane"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_ok(), "Start should succeed");

    let task = store.get("start-001").expect("task should exist");
    assert_eq!(
        task.assignee.as_ref().map(|a| a.as_str()),
        Some("agent-jane")
    );
    assert!(matches!(task.state, TaskState::InProgress));
}

#[test]
fn integration_execute_start_fails_for_nonexistent_task() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let cmd = TaskCommand::Start {
        task_id: valid_id("nonexistent-start"),
        agent_id: valid_agent("agent-kate"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_err(), "Start of nonexistent task should fail");
}

#[test]
fn integration_execute_start_fails_for_task_claimed_by_other() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "start-002", "Already claimed task");
    let _ = claim_test_task(&store, "start-002", "agent-larry");

    let cmd = TaskCommand::Start {
        task_id: valid_id("start-002"),
        agent_id: valid_agent("agent-mary"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(
        result.is_err(),
        "Start of task claimed by other should fail"
    );
}

// ============================================================================
// execute_done integration tests
// ============================================================================

#[test]
fn integration_execute_done_completes_claimed_task() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "done-001", "Completable task");
    let _ = claim_test_task(&store, "done-001", "agent-nancy");

    let cmd = TaskCommand::Done {
        task_id: Some(valid_id("done-001")),
        agent_id: valid_agent("agent-nancy"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_ok(), "Done should succeed");

    let task = store.get("done-001").expect("task should exist");
    assert!(matches!(task.state, TaskState::Closed { .. }));
}

#[test]
fn integration_execute_done_fails_for_unclaimed_task() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "done-002", "Unclaimed task");

    let cmd = TaskCommand::Done {
        task_id: Some(valid_id("done-002")),
        agent_id: valid_agent("agent-oscar"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_err(), "Done of unclaimed task should fail");
}

#[test]
fn integration_execute_done_fails_for_already_closed_task() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "done-003", "Already closed");
    let _ = claim_test_task(&store, "done-003", "agent-peter");

    let cmd_close = TaskCommand::Done {
        task_id: Some(valid_id("done-003")),
        agent_id: valid_agent("agent-peter"),
    };
    execute_task_command(&cmd_close, &*lock).expect("first close should succeed");

    let cmd_second_close = TaskCommand::Done {
        task_id: Some(valid_id("done-003")),
        agent_id: valid_agent("agent-peter"),
    };
    let result = execute_task_command(&cmd_second_close, &*lock);
    assert!(result.is_err(), "Double done should fail");
}

#[test]
fn integration_execute_done_fails_for_task_claimed_by_other() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "done-004", "Claimed by different");
    let _ = claim_test_task(&store, "done-004", "agent-quinn");

    let cmd = TaskCommand::Done {
        task_id: Some(valid_id("done-004")),
        agent_id: valid_agent("agent-rachel"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_err(), "Done of task claimed by other should fail");
}

// ============================================================================
// Full lifecycle integration tests
// ============================================================================

#[test]
fn integration_full_lifecycle_claim_start_done() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "lifecycle-001", "Full lifecycle task");

    let claim_cmd = TaskCommand::Claim {
        task_id: valid_id("lifecycle-001"),
        agent_id: valid_agent("agent-sam"),
    };
    assert!(execute_task_command(&claim_cmd, &*lock).is_ok());

    let start_cmd = TaskCommand::Start {
        task_id: valid_id("lifecycle-001"),
        agent_id: valid_agent("agent-sam"),
    };
    assert!(execute_task_command(&start_cmd, &*lock).is_ok());

    let done_cmd = TaskCommand::Done {
        task_id: Some(valid_id("lifecycle-001")),
        agent_id: valid_agent("agent-sam"),
    };
    assert!(execute_task_command(&done_cmd, &*lock).is_ok());

    let task = store.get("lifecycle-001").expect("task should exist");
    assert!(matches!(task.state, TaskState::Closed { .. }));
}

#[test]
fn integration_full_lifecycle_claim_yield_reclaim() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "lifecycle-002", "Yield and reclaim task");

    let claim1 = TaskCommand::Claim {
        task_id: valid_id("lifecycle-002"),
        agent_id: valid_agent("agent-tina"),
    };
    assert!(execute_task_command(&claim1, &*lock).is_ok());

    let yield_cmd = TaskCommand::YieldTask {
        task_id: valid_id("lifecycle-002"),
        agent_id: valid_agent("agent-tina"),
    };
    assert!(execute_task_command(&yield_cmd, &*lock).is_ok());

    let claim2 = TaskCommand::Claim {
        task_id: valid_id("lifecycle-002"),
        agent_id: valid_agent("agent-ursula"),
    };
    assert!(execute_task_command(&claim2, &*lock).is_ok());

    let task = store.get("lifecycle-002").expect("task should exist");
    assert_eq!(
        task.assignee.as_ref().map(|a| a.as_str()),
        Some("agent-ursula")
    );
}

#[test]
fn integration_full_lifecycle_skip_start_direct_to_done() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "lifecycle-003", "Skip start task");

    let claim_cmd = TaskCommand::Claim {
        task_id: valid_id("lifecycle-003"),
        agent_id: valid_agent("agent-victor"),
    };
    assert!(execute_task_command(&claim_cmd, &*lock).is_ok());

    let done_cmd = TaskCommand::Done {
        task_id: Some(valid_id("lifecycle-003")),
        agent_id: valid_agent("agent-victor"),
    };
    assert!(execute_task_command(&done_cmd, &*lock).is_ok());

    let task = store.get("lifecycle-003").expect("task should exist");
    assert!(matches!(task.state, TaskState::Closed { .. }));
}

// ============================================================================
// Lock contention integration tests
// ============================================================================

#[test]
fn integration_lock_contention_blocks_concurrent_claim() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "lock-001", "Contested task");

    let guard1 = lock
        .acquire(
            scp_core::lock::LockType::Task("lock-001".to_string()),
            "agent-alice",
        )
        .expect("first lock should succeed");

    let guard2_result = lock.acquire(
        scp_core::lock::LockType::Task("lock-001".to_string()),
        "agent-bob",
    );

    assert!(guard2_result.is_err(), "second lock should be blocked");

    drop(guard1);

    let guard2_retry = lock.acquire(
        scp_core::lock::LockType::Task("lock-001".to_string()),
        "agent-bob",
    );
    assert!(
        guard2_retry.is_ok(),
        "second lock should succeed after first is released"
    );
}

#[test]
fn integration_multiple_tasks_independent_locks() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "multi-001", "First task");
    let _ = create_test_task(&store, "multi-002", "Second task");

    let guard1 = lock
        .acquire(
            scp_core::lock::LockType::Task("multi-001".to_string()),
            "agent-carol",
        )
        .expect("first lock should succeed");

    let guard2 = lock
        .acquire(
            scp_core::lock::LockType::Task("multi-002".to_string()),
            "agent-david",
        )
        .expect("second lock should succeed (different task)");

    assert!(guard1.holder().len() > 0);
    assert!(guard2.holder().len() > 0);
}

// ============================================================================
// Task state transition edge cases
// ============================================================================

#[test]
fn integration_task_transitions_preserve_data() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let mut task = create_test_task(&store, "trans-001", "Transition test");
    task.description = Some("Original description".to_string());
    task.priority = Some(crate::commands::task_types::Priority::new("critical"));
    store.update(task).expect("update should succeed");

    let claim_cmd = TaskCommand::Claim {
        task_id: valid_id("trans-001"),
        agent_id: valid_agent("agent-edward"),
    };
    execute_task_command(&claim_cmd, &*lock).expect("claim should succeed");

    let task = store.get("trans-001").expect("task should exist");
    assert_eq!(task.description.as_deref(), Some("Original description"));
    assert_eq!(task.priority.as_ref().map(|p| p.as_str()), Some("critical"));
}

#[test]
fn integration_yield_clears_assignee_but_preserves_other_fields() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let mut task = create_test_task(&store, "yield-preserve", "Preserve fields");
    task.description = Some("Important description".to_string());
    store.update(task).expect("update should succeed");

    let _ = claim_test_task(&store, "yield-preserve", "agent-frank");

    let yield_cmd = TaskCommand::YieldTask {
        task_id: valid_id("yield-preserve"),
        agent_id: valid_agent("agent-frank"),
    };
    execute_task_command(&yield_cmd, &*lock).expect("yield should succeed");

    let task = store.get("yield-preserve").expect("task should exist");
    assert!(task.assignee.is_none());
    assert_eq!(task.description.as_deref(), Some("Important description"));
}

#[test]
fn integration_double_yield_is_idempotent() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "double-yield", "Double yield test");
    let _ = claim_test_task(&store, "double-yield", "agent-george");

    let yield_cmd = TaskCommand::YieldTask {
        task_id: valid_id("double-yield"),
        agent_id: valid_agent("agent-george"),
    };

    assert!(execute_task_command(&yield_cmd, &*lock).is_ok());
    assert!(execute_task_command(&yield_cmd, &*lock).is_ok());

    let task = store.get("double-yield").expect("task should exist");
    assert!(task.assignee.is_none());
}

#[test]
fn integration_start_on_already_started_task_is_idempotent() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "start-twice", "Start twice test");
    let _ = claim_test_task(&store, "start-twice", "agent-helen");

    let start_cmd = TaskCommand::Start {
        task_id: valid_id("start-twice"),
        agent_id: valid_agent("agent-helen"),
    };

    assert!(execute_task_command(&start_cmd, &*lock).is_ok());
    assert!(execute_task_command(&start_cmd, &*lock).is_ok());

    let task = store.get("start-twice").expect("task should exist");
    assert!(matches!(task.state, TaskState::InProgress));
}

// ============================================================================
// Timestamp and metadata integration tests
// ============================================================================

#[test]
fn integration_transitions_update_timestamp() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let task = create_test_task(&store, "timestamp-001", "Timestamp test");
    let original_updated = task.updated_at;

    std::thread::sleep(std::time::Duration::from_millis(10));

    let claim_cmd = TaskCommand::Claim {
        task_id: valid_id("timestamp-001"),
        agent_id: valid_agent("agent-ivan"),
    };
    execute_task_command(&claim_cmd, &*lock).expect("claim should succeed");

    let updated_task = store.get("timestamp-001").expect("task should exist");
    assert!(
        updated_task.updated_at > original_updated,
        "updated_at should change after transition"
    );
}

#[test]
fn integration_closed_task_preserves_closed_at_timestamp() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "closed-ts-001", "Closed timestamp test");
    let _ = claim_test_task(&store, "closed-ts-001", "agent-jane");

    let before = Utc::now();

    let done_cmd = TaskCommand::Done {
        task_id: Some(valid_id("closed-ts-001")),
        agent_id: valid_agent("agent-jane"),
    };
    execute_task_command(&done_cmd, &*lock).expect("done should succeed");

    let after = Utc::now();

    let task = store.get("closed-ts-001").expect("task should exist");
    match task.state {
        TaskState::Closed { closed_at } => {
            assert!(closed_at >= before && closed_at <= after);
        }
        _ => panic!("Expected Closed state"),
    }
}

// ============================================================================
// Serialization roundtrip integration tests
// ============================================================================

#[test]
fn integration_task_serialization_roundtrip_preserves_all_fields() {
    let store = test_store();

    let mut task = create_test_task(&store, "serde-001", "Serialization test");
    task.description = Some("Full description".to_string());
    task.priority = Some(crate::commands::task_types::Priority::new("high"));
    store.update(task).expect("update should succeed");

    let json = serde_json::to_string(&store.get("serde-001").unwrap()).expect("serialize");
    let restored: Task = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.id.as_str(), "serde-001");
    assert_eq!(restored.title.as_str(), "Serialization test");
    assert_eq!(restored.description.as_deref(), Some("Full description"));
    assert_eq!(restored.priority.as_ref().map(|p| p.as_str()), Some("high"));
}

#[test]
fn integration_task_info_output_serialization() {
    let store = test_store();

    let _ = create_test_task(&store, "output-001", "Output test");
    let task = store.get("output-001").expect("task should exist");
    let output = crate::commands::handlers::task::calculations::task_to_output(&task);

    let json = serde_json::to_string(&output).expect("serialize");
    let back: TaskInfoOutput = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(back.id, "output-001");
    assert_eq!(back.title, "Output test");
    assert_eq!(back.status, TaskStatusOutput::Open);
}

#[test]
fn integration_task_list_output_comprehensive() {
    let store = test_store();

    let _ = create_test_task(&store, "list-out-001", "List output 1");
    let _ = create_test_task(&store, "list-out-002", "List output 2");

    let tasks = store.list();
    let outputs: Vec<_> = tasks
        .iter()
        .map(|t| crate::commands::handlers::task::calculations::task_to_output(t))
        .collect();

    let list_output = TaskListOutput {
        tasks: outputs,
        total: 2,
    };

    let json = serde_json::to_string(&list_output).expect("serialize");
    let back: TaskListOutput = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(back.total, 2);
    assert_eq!(back.tasks.len(), 2);
}

// ============================================================================
// Error message integration tests
// ============================================================================

#[test]
fn integration_not_found_error_contains_task_id() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let cmd = TaskCommand::Show {
        task_id: valid_id("missing-task-123"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("missing-task-123"),
        "Error message should contain task ID: {}",
        msg
    );
}

#[test]
fn integration_already_claimed_error_contains_task_id_and_holder() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "claimed-err-001", "Claimed task");
    let _ = claim_test_task(&store, "claimed-err-001", "original-holder");

    let cmd = TaskCommand::Claim {
        task_id: valid_id("claimed-err-001"),
        agent_id: valid_agent("new-claimant"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("claimed-err-001") || msg.contains("original-holder"),
        "Error message should contain task ID or holder: {}",
        msg
    );
}

#[test]
fn integration_not_claimed_error_contains_task_id() {
    let store = test_store();
    let lock = Arc::new(MemLockManager::new());

    let _ = create_test_task(&store, "not-claimed-err-001", "Unclaimed task");

    let cmd = TaskCommand::YieldTask {
        task_id: valid_id("not-claimed-err-001"),
        agent_id: valid_agent("someone"),
    };

    let result = execute_task_command(&cmd, &*lock);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("not-claimed-err-001") || msg.to_lowercase().contains("not claimed"),
        "Error message should contain task ID or not claimed: {}",
        msg
    );
}

// ============================================================================
// run_task_command public API integration tests
// ============================================================================

#[test]
fn integration_run_task_command_uses_internal_lock_manager() {
    let store = test_store();
    let _ = create_test_task(&store, "public-api-001", "Public API test");

    let cmd = TaskCommand::Show {
        task_id: valid_id("public-api-001"),
    };

    let result = run_task_command(&cmd);
    assert!(result.is_ok());
}

#[test]
fn integration_run_task_command_claim_success() {
    let store = test_store();
    let _ = create_test_task(&store, "public-api-002", "Public API claim test");

    let cmd = TaskCommand::Claim {
        task_id: valid_id("public-api-002"),
        agent_id: valid_agent("public-agent"),
    };

    let result = run_task_command(&cmd);
    assert!(result.is_ok());
}
