//! Snapshot tests for CLI contract JSON serialization.
//!
//! These tests verify that CLI contract types serialize correctly to JSON
//! for command output and API responses.

use scp_core::cli_contracts::{
    AgentListResult, AgentResult, ContractError, CreateSessionInput, CreateTaskInput,
    ListSessionsInput, ListTasksInput, RemoveSessionInput, SessionListResult, SessionResult,
    StatusResult, TaskListResult, TaskResult,
};
use std::path::PathBuf;

#[test]
fn test_contract_error_precondition_failed_json() {
    let error = ContractError::PreconditionFailed {
        name: "session_exists",
        description: "Session must exist before removal",
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("contract_error_precondition", json);
}

#[test]
fn test_contract_error_invariant_violation_json() {
    let error = ContractError::InvariantViolation {
        name: "session_name_unique",
        description: "Session names must be unique",
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("contract_error_invariant", json);
}

#[test]
fn test_contract_error_postcondition_failed_json() {
    let error = ContractError::PostconditionFailed {
        name: "session_created",
        description: "Session should exist after creation",
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("contract_error_postcondition", json);
}

#[test]
fn test_contract_error_not_found_json() {
    let error = ContractError::NotFound {
        resource_type: "Session",
        identifier: "my-session".into(),
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("contract_error_not_found", json);
}

#[test]
fn test_contract_error_invalid_state_transition_json() {
    let error = ContractError::InvalidStateTransition {
        from: "completed".into(),
        to: "active".into(),
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("contract_error_invalid_transition", json);
}

#[test]
fn test_contract_error_concurrent_modification_json() {
    let error = ContractError::ConcurrentModification {
        description: "Session was modified by another process".into(),
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("contract_error_concurrent", json);
}

#[test]
fn test_session_result_json() {
    let result = SessionResult {
        id: "session-123".into(),
        name: "test-session".into(),
        status: "active".into(),
        workspace_path: PathBuf::from("/tmp/workspace"),
    };
    let json = serde_json::to_string(&result).unwrap();
    insta::assert_snapshot!("session_result", json);
}

#[test]
fn test_session_list_result_json() {
    let result = SessionListResult {
        sessions: vec![
            SessionResult {
                id: "1".into(),
                name: "session-1".into(),
                status: "active".into(),
                workspace_path: PathBuf::from("/tmp/s1"),
            },
            SessionResult {
                id: "2".into(),
                name: "session-2".into(),
                status: "paused".into(),
                workspace_path: PathBuf::from("/tmp/s2"),
            },
        ],
        current: Some("session-1".into()),
    };
    let json = serde_json::to_string(&result).unwrap();
    insta::assert_snapshot!("session_list_result", json);
}

#[test]
fn test_task_result_json() {
    let result = TaskResult {
        id: "task-456".into(),
        title: "Implement feature X".into(),
        status: "open".into(),
    };
    let json = serde_json::to_string(&result).unwrap();
    insta::assert_snapshot!("task_result", json);
}

#[test]
fn test_task_list_result_json() {
    let result = TaskListResult {
        tasks: vec![
            TaskResult {
                id: "1".into(),
                title: "Task 1".into(),
                status: "open".into(),
            },
            TaskResult {
                id: "2".into(),
                title: "Task 2".into(),
                status: "in_progress".into(),
            },
        ],
        total: 10,
    };
    let json = serde_json::to_string(&result).unwrap();
    insta::assert_snapshot!("task_list_result", json);
}

#[test]
fn test_agent_result_json() {
    let result = AgentResult {
        id: "agent-789".into(),
        agent_type: "claude".into(),
        session: "test-session".into(),
        status: "running".into(),
        pid: Some(12345),
    };
    let json = serde_json::to_string(&result).unwrap();
    insta::assert_snapshot!("agent_result", json);
}

#[test]
fn test_agent_list_result_json() {
    let result = AgentListResult {
        agents: vec![
            AgentResult {
                id: "1".into(),
                agent_type: "claude".into(),
                session: "s1".into(),
                status: "running".into(),
                pid: Some(100),
            },
            AgentResult {
                id: "2".into(),
                agent_type: "cursor".into(),
                session: "s2".into(),
                status: "completed".into(),
                pid: None,
            },
        ],
        total: 2,
    };
    let json = serde_json::to_string(&result).unwrap();
    insta::assert_snapshot!("agent_list_result", json);
}

#[test]
fn test_status_result_json() {
    let result = StatusResult {
        session: "test-session".into(),
        status: "active".into(),
        state: "working".into(),
        branch: Some("main".into()),
        changes: 5,
        has_uncommitted: true,
        workspace_path: PathBuf::from("/tmp/workspace"),
    };
    let json = serde_json::to_string(&result).unwrap();
    insta::assert_snapshot!("status_result", json);
}

#[test]
fn test_status_result_without_branch_json() {
    let result = StatusResult {
        session: "test-session".into(),
        status: "active".into(),
        state: "clean".into(),
        branch: None,
        changes: 0,
        has_uncommitted: false,
        workspace_path: PathBuf::from("/tmp/workspace"),
    };
    let json = serde_json::to_string(&result).unwrap();
    insta::assert_snapshot!("status_result_no_branch", json);
}

#[test]
fn test_create_task_input_json() {
    let input = CreateTaskInput {
        title: "New task".into(),
        description: Some("Task description".into()),
        priority: Some("P1".into()),
        task_type: Some("feature".into()),
        labels: vec!["api".into(), "urgent".into()],
    };
    let json = serde_json::to_string(&input).unwrap();
    insta::assert_snapshot!("create_task_input", json);
}

#[test]
fn test_list_tasks_input_json() {
    let input = ListTasksInput {
        status: Some("open".into()),
        priority: Some("P0".into()),
        label: Some("bug".into()),
        limit: Some(50),
    };
    let json = serde_json::to_string(&input).unwrap();
    insta::assert_snapshot!("list_tasks_input", json);
}

#[test]
fn test_list_sessions_input_json() {
    let input = ListSessionsInput {
        status: Some("active".into()),
        include_stacked: true,
    };
    let json = serde_json::to_string(&input).unwrap();
    insta::assert_snapshot!("list_sessions_input", json);
}
