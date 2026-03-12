//! Task validation and state transitions
//!
//! Pure functions for task validation and state transitions

use crate::commands::task_types::{Task, TaskState};
use scp_core::{
    error::Error, lock::LockGuard, lock::LockManager, lock::LockType, Result as CoreResult,
};

/// Validate that task ID is not empty
pub fn validate_task_id(task_id: &str) -> CoreResult<()> {
    if task_id.is_empty() {
        return Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()));
    }
    Ok(())
}

/// Validate task exists
pub fn validate_task_exists(task: Option<Task>, task_id: &str) -> CoreResult<Task> {
    task.ok_or_else(|| Error::TaskNotFound(task_id.to_string()))
}

/// Validate task is not claimed by another user
pub fn validate_not_claimed_by_other(task: &Task, current_user: &str) -> CoreResult<()> {
    if let Some(assignee) = &task.assignee {
        if assignee != current_user {
            return Err(Error::TaskAlreadyClaimed(task.id.clone(), assignee.clone()));
        }
    }
    Ok(())
}

/// Validate task is claimed by current user
pub fn validate_claimed_by_user(task: &Task, current_user: &str) -> CoreResult<()> {
    if task.assignee.as_deref() != Some(current_user) {
        return Err(Error::TaskNotClaimed(task.id.clone()));
    }
    Ok(())
}

/// Validate task is not already closed
pub fn validate_not_closed(task: &Task) -> CoreResult<()> {
    if matches!(task.state, TaskState::Closed { .. }) {
        return Err(Error::InvalidTaskStateTransition(
            task.id.clone(),
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

/// Transition task to claimed state
pub fn transition_to_claimed(task: Task, user: &str) -> Task {
    let mut t = task;
    t.assignee = Some(user.to_string());
    t.state = TaskState::InProgress;
    t.updated_at = chrono::Utc::now();
    t
}

/// Transition task to yielded (open) state
pub fn transition_to_yielded(task: Task) -> Task {
    let mut t = task;
    t.assignee = None;
    t.state = TaskState::Open;
    t.updated_at = chrono::Utc::now();
    t
}

/// Transition task to started state
pub fn transition_to_started(task: Task) -> Task {
    let mut t = task;
    t.state = TaskState::InProgress;
    t.updated_at = chrono::Utc::now();
    t
}

/// Transition task to done (closed) state
pub fn transition_to_done(task: Task) -> Task {
    let mut t = task;
    t.state = TaskState::Closed {
        closed_at: chrono::Utc::now(),
    };
    t.updated_at = chrono::Utc::now();
    t
}
