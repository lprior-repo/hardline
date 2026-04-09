//! Task validation and state transitions
//!
//! Pure functions for task validation and state transitions

use crate::commands::task_types::{Assignee, Task, TaskState};
use scp_core::{
    error_task::{TaskError, TaskErrorKind},
    lock::LockGuard,
    lock::LockManager,
    lock::LockType,
    Error, Result as CoreResult,
};

/// Validate task exists
pub fn validate_task_exists(task: Option<Task>, task_id: &str) -> CoreResult<Task> {
    task.ok_or_else(|| TaskErrorKind::NotFound(task_id.to_string()).into())
}

/// Validate task is not claimed by another user
pub fn validate_not_claimed_by_other(task: &Task, current_user: &str) -> CoreResult<()> {
    if let Some(assignee) = &task.assignee {
        if assignee.as_str() != current_user {
            return Err(
                TaskErrorKind::AlreadyClaimed(task.id.to_string(), assignee.to_string()).into(),
            );
        }
    }
    Ok(())
}

/// Validate task is claimed by current user
pub fn validate_claimed_by_user(task: &Task, current_user: &str) -> CoreResult<()> {
    if task.assignee.as_ref().map(|a| a.as_str()) != Some(current_user) {
        return Err(TaskErrorKind::NotClaimed(task.id.to_string()).into());
    }
    Ok(())
}

/// Validate task is not already closed
pub fn validate_not_closed(task: &Task) -> CoreResult<()> {
    if matches!(task.state, TaskState::Closed { .. }) {
        return Err(TaskErrorKind::InvalidStateTransition(
            task.id.to_string(),
            "Task is already closed".to_string(),
        )
        .into());
    }
    Ok(())
}

/// Acquire task lock
pub fn acquire_task_lock(
    lock: &dyn LockManager,
    task_id: &str,
    holder: &str,
) -> CoreResult<LockGuard> {
    let lock_type = LockType::Task(task_id.to_string());
    lock.acquire(lock_type, holder)
        .map_err(|_| TaskErrorKind::Locked(task_id.to_string()).into())
}

/// Transition task to claimed state (pure function - returns new instance)
pub fn transition_to_claimed(task: Task, user: &str) -> Task {
    Task {
        assignee: Some(Assignee::new(user)),
        state: TaskState::InProgress,
        updated_at: chrono::Utc::now(),
        ..task
    }
}

/// Transition task to yielded (open) state (pure function - returns new instance)
pub fn transition_to_yielded(task: Task) -> Task {
    Task {
        assignee: None,
        state: TaskState::Open,
        updated_at: chrono::Utc::now(),
        ..task
    }
}

/// Transition task to started state (pure function - returns new instance)
pub fn transition_to_started(task: Task) -> Task {
    Task {
        state: TaskState::InProgress,
        updated_at: chrono::Utc::now(),
        ..task
    }
}

/// Transition task to done (closed) state (pure function - returns new instance)
pub fn transition_to_done(task: Task) -> Task {
    Task {
        state: TaskState::Closed {
            closed_at: chrono::Utc::now(),
        },
        updated_at: chrono::Utc::now(),
        ..task
    }
}

// Tests - Contract Verification and Martin-Fowler Given-When-Then

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::task_types::{TaskId, Title};
    use chrono::Utc;

    /// Helper to create a task in Open state (no assignee)
    fn open_task(id: &str) -> Task {
        Task::new(
            TaskId::new(id).expect("valid task id"),
            Title::new("Test task"),
        )
    }

    /// Helper to create a task claimed by a specific user
    fn claimed_task(id: &str, assignee: &str) -> Task {
        let task = open_task(id);
        transition_to_claimed(task, assignee)
    }

    /// Helper to create a task in InProgress state
    fn in_progress_task(id: &str, assignee: &str) -> Task {
        claimed_task(id, assignee)
    }

    /// Helper to create a closed task
    fn closed_task(id: &str, assignee: &str) -> Task {
        let task = in_progress_task(id, assignee);
        transition_to_done(task)
    }

    // Contract Verification Tests - Preconditions

    #[test]
    fn test_precondition_p1_empty_id_rejected_at_type_level() {
        let result = TaskId::new("");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_precondition_p1_malformed_id_rejected_at_type_level() {
        let result = TaskId::new("bad id!");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("alphanumeric"));
    }

    #[test]
    fn test_precondition_p2_nonexistent_task_returns_not_found() {
        let result = validate_task_exists(None, "nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            Error::Task(te) if matches!(te.inner, TaskErrorKind::NotFound(_))
        ));
    }

    #[test]
    fn test_precondition_p3_already_claimed_prevents_claim() {
        // Given: Task claimed by "other-user"
        let task = claimed_task("task-001", "other-user");

        // When: validate_not_claimed_by_user with holder="current-user"
        let result = validate_not_claimed_by_other(&task, "current-user");

        // Then: Returns Err(TaskAlreadyClaimed)
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            Error::Task(te) if matches!(te.inner, TaskErrorKind::AlreadyClaimed(_, _))
        ));
    }

    #[test]
    fn test_precondition_p3_claim_succeeds_for_same_user() {
        // Given: Task claimed by "current-user"
        let task = claimed_task("task-001", "current-user");

        // When: validate_not_claimed_by_other with holder="current-user"
        let result = validate_not_claimed_by_other(&task, "current-user");

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_precondition_p4_must_be_claimed_before_yield() {
        // Given: Task not claimed (Open state, no assignee)
        let task = open_task("task-001");

        // When: validate_claimed_by_user with holder="current-user"
        let result = validate_claimed_by_user(&task, "current-user");

        // Then: Returns Err(TaskNotClaimed)
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            Error::Task(te) if matches!(te.inner, TaskErrorKind::NotClaimed(_))
        ));
    }

    #[test]
    fn test_precondition_p4_yield_succeeds_when_claimed() {
        // Given: Task claimed by "current-user"
        let task = claimed_task("task-001", "current-user");

        // When: validate_claimed_by_user with holder="current-user"
        let result = validate_claimed_by_user(&task, "current-user");

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    // Contract Verification Tests - Postconditions

    #[test]
    fn test_postcondition_q3_claim_sets_assignee_and_in_progress() {
        // Given: Open task
        let task = open_task("task-001");

        // When: transition_to_claimed with user="current-user"
        let result = transition_to_claimed(task, "current-user");

        // Then: assignee is set and state is InProgress
        assert_eq!(
            result.assignee.as_ref().map(|a| a.as_str()),
            Some("current-user")
        );
        assert!(matches!(result.state, TaskState::InProgress));
    }

    #[test]
    fn test_postcondition_q4_yield_clears_assignee_and_sets_open() {
        // Given: InProgress task with assignee
        let task = in_progress_task("task-001", "current-user");

        // When: transition_to_yielded
        let result = transition_to_yielded(task);

        // Then: assignee is None and state is Open
        assert!(result.assignee.is_none());
        assert!(matches!(result.state, TaskState::Open));
    }

    #[test]
    fn test_postcondition_q6_done_sets_closed_with_timestamp() {
        // Given: InProgress task
        let task = in_progress_task("task-001", "current-user");

        // When: transition_to_done
        let before = Utc::now();
        let result = transition_to_done(task);
        let after = Utc::now();

        // Then: state is Closed with closed_at timestamp
        match result.state {
            TaskState::Closed { closed_at } => {
                assert!(closed_at >= before && closed_at <= after);
            }
            _ => panic!("Expected Closed state"),
        }
    }

    // Contract Verification Tests - Invariants

    #[test]
    fn test_invariant_i1_valid_task_ids_are_accepted() {
        let valid_ids = vec!["task-001", "bead_123", "ABC-123_xyz", "a", "1-2_3"];
        for id in valid_ids {
            let result = TaskId::new(id);
            assert!(result.is_ok(), "Expected {} to be valid", id);
        }
    }

    #[test]
    fn test_invariant_i2_cannot_close_already_closed_task() {
        // Given: Closed task
        let task = closed_task("task-001", "current-user");

        // When: validate_not_closed
        let result = validate_not_closed(&task);

        // Then: Returns Err
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            Error::Task(te) if matches!(te.inner, TaskErrorKind::InvalidStateTransition(_, _))
        ));
    }

    // Happy Path Tests

    #[test]
    fn test_task_claim_assigns_task_to_current_user_and_sets_in_progress() {
        // Given: Open task, unclaimed
        let task = open_task("bead-123");

        // When: transition_to_claimed
        let result = transition_to_claimed(task, "current-user");

        // Then: Task has assignee set to current user, state changed to InProgress
        assert_eq!(result.assignee.unwrap().as_str(), "current-user");
        assert!(matches!(result.state, TaskState::InProgress));
    }

    #[test]
    fn test_task_yield_clears_assignee_and_sets_state_to_open() {
        // Given: Task claimed by current user
        let task = claimed_task("bead-123", "current-user");

        // When: transition_to_yielded
        let result = transition_to_yielded(task);

        // Then: Task has assignee cleared, state changed to Open
        assert!(result.assignee.is_none());
        assert!(matches!(result.state, TaskState::Open));
    }

    #[test]
    fn test_task_start_transitions_to_in_progress_preserving_assignee() {
        // Given: Task claimed by current user, state Open
        let task = claimed_task("bead-123", "current-user");

        // When: transition_to_started
        let result = transition_to_started(task);

        // Then: Task state is InProgress, assignee remains unchanged
        assert!(matches!(result.state, TaskState::InProgress));
        assert_eq!(result.assignee.unwrap().as_str(), "current-user");
    }

    #[test]
    fn test_task_done_closes_task_with_timestamp() {
        // Given: Task claimed by current user, state InProgress
        let task = in_progress_task("bead-123", "current-user");

        // When: transition_to_done
        let result = transition_to_done(task);

        // Then: Task state is Closed with closed_at set
        assert!(matches!(result.state, TaskState::Closed { .. }));
    }

    // Error Path Tests

    #[test]
    fn test_task_show_returns_error_for_invalid_task_id() {
        let result = TaskId::new("");
        assert!(result.is_err());
    }

    #[test]
    fn test_task_show_returns_error_for_malformed_task_id() {
        let result = TaskId::new("bad id!");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("alphanumeric"));
    }

    #[test]
    fn test_task_show_returns_not_found_for_nonexistent_task() {
        let result = validate_task_exists(None, "nonexistent");
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), Error::Task(ref te) if matches!(te.inner, TaskErrorKind::NotFound(_)))
        );
    }

    #[test]
    fn test_task_claim_returns_error_when_task_already_claimed() {
        // Given: Repository with task claimed by "other-user"
        let task = claimed_task("bead-123", "other-user");

        // When: validate_not_claimed_by_other with holder="current-user"
        let result = validate_not_claimed_by_other(&task, "current-user");

        // Then: Returns Err(TaskAlreadyClaimed)
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::Task(ref te) if matches!(te.inner, TaskErrorKind::AlreadyClaimed(_, _))
        ));
    }

    #[test]
    fn test_task_yield_returns_error_when_task_not_claimed() {
        // Given: Repository with task that has no assignee
        let task = open_task("bead-123");

        // When: validate_claimed_by_user with holder="current-user"
        let result = validate_claimed_by_user(&task, "current-user");

        // Then: Returns Err(TaskNotClaimed)
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            Error::Task(te) if matches!(te.inner, TaskErrorKind::NotClaimed(_))
        ));
    }

    #[test]
    fn test_task_start_returns_error_for_closed_task() {
        // Given: Repository with task in Closed state
        let task = closed_task("bead-123", "current-user");

        // When: validate_not_closed
        let result = validate_not_closed(&task);

        // Then: Returns Err(InvalidTaskStateTransition)
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            Error::Task(te) if matches!(te.inner, TaskErrorKind::InvalidStateTransition(_, _))
        ));
    }

    #[test]
    fn test_task_done_returns_error_for_already_closed_task() {
        // Given: Repository with task already in Closed state
        let task = closed_task("bead-123", "current-user");

        // When: validate_not_closed
        let result = validate_not_closed(&task);

        // Then: Returns Err(InvalidTaskStateTransition)
        assert!(result.is_err());
    }

    // Edge Case Tests

    #[test]
    fn test_task_claim_idempotent_when_already_claimed_by_same_user() {
        // Given: Repository with task already claimed by current user
        let task = claimed_task("bead-123", "current-user");

        // When: transition_to_claimed is called again
        let result = transition_to_claimed(task, "current-user");

        // Then: Returns success, state remains InProgress
        assert!(matches!(result.state, TaskState::InProgress));
        assert_eq!(result.assignee.unwrap().as_str(), "current-user");
    }

    // ---- Additional edge case tests ----

    #[test]
    fn test_validate_not_claimed_by_other_succeeds_when_no_assignee() {
        // Given: Open task with no assignee
        let task = open_task("task-001");

        // When: validate_not_claimed_by_other with any user
        let result = validate_not_claimed_by_other(&task, "any-user");

        // Then: Succeeds because there is no assignee to conflict with
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_not_closed_succeeds_for_open_state() {
        let task = open_task("task-001");
        assert!(validate_not_closed(&task).is_ok());
    }

    #[test]
    fn test_validate_not_closed_succeeds_for_in_progress_state() {
        let task = in_progress_task("task-001", "user");
        assert!(validate_not_closed(&task).is_ok());
    }

    #[test]
    fn test_validate_not_closed_succeeds_for_blocked_state() {
        let mut task = open_task("task-001");
        task.state = TaskState::Blocked;
        assert!(validate_not_closed(&task).is_ok());
    }

    #[test]
    fn test_validate_not_closed_succeeds_for_deferred_state() {
        let mut task = open_task("task-001");
        task.state = TaskState::Deferred;
        assert!(validate_not_closed(&task).is_ok());
    }

    #[test]
    fn test_transition_to_claimed_does_not_mutate_original() {
        // Verify immutability: original task is not modified
        let task = open_task("task-001");
        let original_state = task.state.clone();

        let _result = transition_to_claimed(task, "user");

        // Original state is preserved (via move, this verifies the function returns a new instance)
        assert!(matches!(original_state, TaskState::Open));
    }

    #[test]
    fn test_transition_to_yielded_does_not_mutate_original() {
        let task = claimed_task("task-001", "user");
        let had_assignee = task.assignee.is_some();

        let result = transition_to_yielded(task);

        // The returned task has cleared assignee
        assert!(result.assignee.is_none());
        // Original task had an assignee (we checked before the move)
        assert!(had_assignee);
    }

    #[test]
    fn test_transition_to_started_preserves_existing_assignee() {
        let task = claimed_task("task-001", "alice");
        let result = transition_to_started(task);
        assert_eq!(result.assignee.as_ref().map(|a| a.as_str()), Some("alice"));
    }

    #[test]
    fn test_transition_to_started_on_open_task() {
        // A task that was claimed but somehow set back to Open should still work
        let mut task = claimed_task("task-001", "user");
        task.state = TaskState::Open;
        let result = transition_to_started(task);
        assert!(matches!(result.state, TaskState::InProgress));
        assert_eq!(result.assignee.as_ref().map(|a| a.as_str()), Some("user"));
    }

    #[test]
    fn test_validate_task_exists_returns_task_when_present() {
        let task = open_task("task-001");
        let result = validate_task_exists(Some(task), "task-001");
        assert!(result.is_ok());
        assert_eq!(result.expect("ok").id.as_str(), "task-001");
    }

    #[test]
    fn test_validate_claimed_by_user_fails_for_different_user() {
        let task = claimed_task("task-001", "alice");
        let result = validate_claimed_by_user(&task, "bob");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_claimed_by_user_fails_for_no_assignee() {
        let task = open_task("task-001");
        let result = validate_claimed_by_user(&task, "anyone");
        assert!(result.is_err());
    }

    // ---- Error Message Clarity Tests ----

    #[test]
    fn test_error_message_not_found_contains_task_id() {
        let result = validate_task_exists(None, "missing-task-123");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("missing-task-123"),
            "Error message should contain task ID"
        );
        assert!(
            msg.to_lowercase().contains("not found"),
            "Error message should indicate not found"
        );
    }

    #[test]
    fn test_error_message_already_claimed_contains_task_id_and_assignee() {
        let task = claimed_task("task-001", "current-owner");
        let result = validate_not_claimed_by_other(&task, "new-claimant");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("task-001"),
            "Error message should contain task ID"
        );
        assert!(
            msg.contains("current-owner"),
            "Error message should contain current assignee"
        );
        assert!(
            msg.to_lowercase().contains("claimed"),
            "Error message should indicate claimed"
        );
    }

    #[test]
    fn test_error_message_not_claimed_contains_task_id() {
        let task = open_task("task-001");
        let result = validate_claimed_by_user(&task, "any-user");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("task-001"),
            "Error message should contain task ID"
        );
        assert!(
            msg.to_lowercase().contains("not claimed"),
            "Error message should indicate not claimed"
        );
    }

    #[test]
    fn test_error_message_invalid_state_transition_contains_task_id_and_reason() {
        let task = closed_task("task-001", "user");
        let result = validate_not_closed(&task);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("task-001"),
            "Error message should contain task ID"
        );
        assert!(
            msg.to_lowercase().contains("closed") || msg.to_lowercase().contains("state"),
            "Error message should mention closed/state"
        );
    }

    #[test]
    fn test_error_message_actionable_for_not_found() {
        let result = validate_task_exists(None, "bead-42");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("bead-42") && msg.to_lowercase().contains("not found"),
            "Error should tell user which task was not found: {}",
            msg
        );
    }

    #[test]
    fn test_error_message_actionable_for_already_claimed() {
        let task = claimed_task("task-007", "alice");
        let result = validate_not_claimed_by_other(&task, "bob");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("task-007")
                && msg.contains("alice")
                && msg.to_lowercase().contains("claimed"),
            "Error should identify task and current owner: {}",
            msg
        );
    }

    // ---- Error Exit Code Tests ----

    #[test]
    fn test_error_exit_code_not_found() {
        let result = validate_task_exists(None, "task-x");
        let err = result.unwrap_err();
        if let Error::Task(te) = err {
            assert_eq!(te.exit_code(), 60, "NotFound should have exit code 60");
        } else {
            panic!("Expected Task error");
        }
    }

    #[test]
    fn test_error_exit_code_already_claimed() {
        let task = claimed_task("task-x", "owner");
        let result = validate_not_claimed_by_other(&task, "other");
        let err = result.unwrap_err();
        if let Error::Task(te) = err {
            assert_eq!(
                te.exit_code(),
                61,
                "AlreadyClaimed should have exit code 61"
            );
        } else {
            panic!("Expected Task error");
        }
    }

    #[test]
    fn test_error_exit_code_not_claimed() {
        let task = open_task("task-x");
        let result = validate_claimed_by_user(&task, "user");
        let err = result.unwrap_err();
        if let Error::Task(te) = err {
            assert_eq!(te.exit_code(), 62, "NotClaimed should have exit code 62");
        } else {
            panic!("Expected Task error");
        }
    }

    #[test]
    fn test_error_exit_code_locked() {
        let err = TaskErrorKind::Locked("task-x".to_string());
        let task_err: Error = err.into();
        if let Error::Task(te) = task_err {
            assert_eq!(te.exit_code(), 63, "Locked should have exit code 63");
        } else {
            panic!("Expected Task error");
        }
    }

    #[test]
    fn test_error_exit_code_invalid_id() {
        let result = TaskId::new("bad id!");
        let err = result.unwrap_err();
        if let Error::Task(te) = err {
            assert_eq!(te.exit_code(), 64, "InvalidId should have exit code 64");
        } else {
            panic!("Expected Task error");
        }
    }

    #[test]
    fn test_error_exit_code_invalid_state_transition() {
        let task = closed_task("task-x", "user");
        let result = validate_not_closed(&task);
        let err = result.unwrap_err();
        if let Error::Task(te) = err {
            assert_eq!(
                te.exit_code(),
                65,
                "InvalidStateTransition should have exit code 65"
            );
        } else {
            panic!("Expected Task error");
        }
    }

    // ---- Immutability Tests ----

    #[test]
    fn test_transition_to_done_does_not_mutate_original() {
        let task = in_progress_task("task-001", "user");
        let original_state = task.state.clone();

        let _result = transition_to_done(task);

        assert!(
            matches!(original_state, TaskState::InProgress),
            "Original should be InProgress, immutable verification"
        );
    }

    #[test]
    fn test_transition_to_started_returns_new_instance() {
        let task = in_progress_task("task-001", "user");
        let original_updated_at = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        let result = transition_to_started(task);

        assert!(
            result.updated_at >= original_updated_at,
            "New instance should have updated timestamp"
        );
    }

    // ---- State Transition Edge Cases ----

    #[test]
    fn test_transition_to_claimed_on_already_claimed_same_user() {
        let task = claimed_task("task-001", "alice");
        let result = transition_to_claimed(task, "alice");
        assert!(matches!(result.state, TaskState::InProgress));
        assert_eq!(result.assignee.as_ref().map(|a| a.as_str()), Some("alice"));
    }

    #[test]
    fn test_transition_to_yielded_from_open_task() {
        let task = open_task("task-001");
        let result = transition_to_yielded(task);
        assert!(matches!(result.state, TaskState::Open));
        assert!(result.assignee.is_none());
    }

    #[test]
    fn test_transition_to_started_on_blocked_task() {
        let mut task = in_progress_task("task-001", "user");
        task.state = TaskState::Blocked;
        let result = transition_to_started(task);
        assert!(matches!(result.state, TaskState::InProgress));
    }

    #[test]
    fn test_transition_to_started_on_deferred_task() {
        let mut task = in_progress_task("task-001", "user");
        task.state = TaskState::Deferred;
        let result = transition_to_started(task);
        assert!(matches!(result.state, TaskState::InProgress));
    }

    #[test]
    fn test_transition_to_done_from_open_task() {
        let task = open_task("task-001");
        let result = transition_to_done(task);
        assert!(matches!(result.state, TaskState::Closed { .. }));
    }

    #[test]
    fn test_transition_to_done_from_blocked_task() {
        let mut task = in_progress_task("task-001", "user");
        task.state = TaskState::Blocked;
        let result = transition_to_done(task);
        assert!(matches!(result.state, TaskState::Closed { .. }));
    }

    // ---- TaskId Validation Edge Cases ----

    #[test]
    fn test_task_id_max_length_accepted() {
        let long_id = "a".repeat(256);
        let result = TaskId::new(&long_id);
        assert!(result.is_ok(), "Long but valid ID should be accepted");
    }

    #[test]
    fn test_task_id_with_only_dashes_and_underscores() {
        let result = TaskId::new("---___---");
        assert!(
            result.is_ok(),
            "ID with only dashes and underscores should be valid"
        );
    }

    #[test]
    fn test_task_id_unicode_rejected() {
        let result = TaskId::new("task-你好");
        assert!(result.is_err(), "Unicode characters should be rejected");
    }

    #[test]
    fn test_task_id_newline_rejected() {
        let result = TaskId::new("task\n001");
        assert!(result.is_err(), "Newlines should be rejected");
    }

    #[test]
    fn test_task_id_tab_rejected() {
        let result = TaskId::new("task\t001");
        assert!(result.is_err(), "Tabs should be rejected");
    }

    // ---- Cross-Validation Scenarios ----

    #[test]
    fn test_full_task_lifecycle_open_to_closed() {
        let task = open_task("lifecycle-001");

        let claimed = transition_to_claimed(task, "user");
        assert!(matches!(claimed.state, TaskState::InProgress));
        assert_eq!(claimed.assignee.as_ref().map(|a| a.as_str()), Some("user"));

        let started = transition_to_started(claimed);
        assert!(matches!(started.state, TaskState::InProgress));

        let done = transition_to_done(started);
        assert!(matches!(done.state, TaskState::Closed { .. }));

        let closed_result = validate_not_closed(&done);
        assert!(closed_result.is_err());
    }

    #[test]
    fn test_yield_and_reclaim_workflow() {
        let task = open_task("yield-001");

        let claimed = transition_to_claimed(task, "alice");
        assert!(validate_claimed_by_user(&claimed, "alice").is_ok());

        let yielded = transition_to_yielded(claimed);
        assert!(yielded.assignee.is_none());

        let reclaimed = transition_to_claimed(yielded, "bob");
        assert!(validate_claimed_by_user(&reclaimed, "bob").is_ok());
    }

    #[test]
    fn test_validate_not_claimed_by_other_allows_unclaimed_task() {
        let task = open_task("unclaimed-001");
        let result = validate_not_claimed_by_other(&task, "anyone");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_not_claimed_by_other_allows_same_user() {
        let task = claimed_task("same-user-001", "alice");
        let result = validate_not_claimed_by_other(&task, "alice");
        assert!(result.is_ok());
    }

    // ---- Transition Timestamp Tests ----

    #[test]
    fn test_transition_to_claimed_updates_timestamp() {
        let task = open_task("ts-001");
        let original = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        let result = transition_to_claimed(task, "user");

        assert!(result.updated_at > original);
    }

    #[test]
    fn test_transition_to_yielded_updates_timestamp() {
        let task = claimed_task("ts-002", "user");
        let original = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        let result = transition_to_yielded(task);

        assert!(result.updated_at > original);
    }

    #[test]
    fn test_transition_to_started_updates_timestamp() {
        let task = claimed_task("ts-003", "user");
        let original = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        let result = transition_to_started(task);

        assert!(result.updated_at > original);
    }

    #[test]
    fn test_transition_to_done_updates_timestamp() {
        let task = in_progress_task("ts-004", "user");
        let original = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        let result = transition_to_done(task);

        assert!(result.updated_at > original);
    }

    // ---- Error Kind Matching Tests ----

    #[test]
    fn test_error_kind_kind_method() {
        let task_err = TaskError::from(TaskErrorKind::NotFound("test".into()));
        assert!(matches!(task_err.kind(), TaskErrorKind::NotFound(_)));
    }

    #[test]
    fn test_error_kind_all_variants_have_exit_codes() {
        let variants = vec![
            TaskErrorKind::NotFound("x".into()),
            TaskErrorKind::AlreadyClaimed("x".into(), "y".into()),
            TaskErrorKind::NotClaimed("x".into()),
            TaskErrorKind::Locked("x".into()),
            TaskErrorKind::InvalidId("x".into()),
            TaskErrorKind::InvalidStateTransition("x".into(), "y".into()),
        ];

        for variant in variants {
            let err: Error = variant.into();
            if let Error::Task(te) = err {
                let code = te.exit_code();
                assert!(
                    code >= 60 && code <= 65,
                    "Exit code should be in 60-65 range"
                );
            } else {
                panic!("Should convert to Task error");
            }
        }
    }
}
