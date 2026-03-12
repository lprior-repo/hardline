//! Task validation and state transitions
//!
//! Pure functions for task validation and state transitions

use crate::commands::task_types::{Assignee, Task, TaskState};
use scp_core::{
    error::Error, lock::LockGuard, lock::LockManager, lock::LockType, Result as CoreResult,
};

/// Validate task exists
pub fn validate_task_exists(task: Option<Task>, task_id: &str) -> CoreResult<Task> {
    task.ok_or_else(|| Error::TaskNotFound(task_id.to_string()))
}

/// Validate task is not claimed by another user
pub fn validate_not_claimed_by_other(task: &Task, current_user: &str) -> CoreResult<()> {
    if let Some(assignee) = &task.assignee {
        if assignee.as_str() != current_user {
            return Err(Error::TaskAlreadyClaimed(
                task.id.to_string(),
                assignee.to_string(),
            ));
        }
    }
    Ok(())
}

/// Validate task is claimed by current user
pub fn validate_claimed_by_user(task: &Task, current_user: &str) -> CoreResult<()> {
    if task.assignee.as_ref().map(|a| a.as_str()) != Some(current_user) {
        return Err(Error::TaskNotClaimed(task.id.to_string()));
    }
    Ok(())
}

/// Validate task is not already closed
pub fn validate_not_closed(task: &Task) -> CoreResult<()> {
    if matches!(task.state, TaskState::Closed { .. }) {
        return Err(Error::InvalidTaskStateTransition(
            task.id.to_string(),
            "Task is already closed".to_string(),
        ));
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
        .map_err(|_| Error::TaskLocked(task_id.to_string()))
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

// ============================================================================
// Tests - Contract Verification and Martin-Fowler Given-When-Then
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::task_types::Title;
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

    // ========================================================================
    // Contract Verification Tests - Preconditions
    // ========================================================================

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
        assert!(matches!(result.unwrap_err(), Error::TaskNotFound(_)));
    }

    #[test]
    fn test_precondition_p3_already_claimed_prevents_claim() {
        // Given: Task claimed by "other-user"
        let task = claimed_task("task-001", "other-user");

        // When: validate_not_claimed_by_user with holder="current-user"
        let result = validate_not_claimed_by_other(&task, "current-user");

        // Then: Returns Err(TaskAlreadyClaimed)
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::TaskAlreadyClaimed(_, _)
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
        assert!(matches!(result.unwrap_err(), Error::TaskNotClaimed(_)));
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

    // ========================================================================
    // Contract Verification Tests - Postconditions
    // ========================================================================

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

    // ========================================================================
    // Contract Verification Tests - Invariants
    // ========================================================================

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
        assert!(matches!(
            result.unwrap_err(),
            Error::InvalidTaskStateTransition(_, _)
        ));
    }

    // ========================================================================
    // Happy Path Tests
    // ========================================================================

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

    // ========================================================================
    // Error Path Tests
    // ========================================================================

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
        assert!(matches!(result.unwrap_err(), Error::TaskNotFound(_)));
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
            Error::TaskAlreadyClaimed(_, _)
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
        assert!(matches!(result.unwrap_err(), Error::TaskNotClaimed(_)));
    }

    #[test]
    fn test_task_start_returns_error_for_closed_task() {
        // Given: Repository with task in Closed state
        let task = closed_task("bead-123", "current-user");

        // When: validate_not_closed
        let result = validate_not_closed(&task);

        // Then: Returns Err(InvalidTaskStateTransition)
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::InvalidTaskStateTransition(_, _)
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

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

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
}
