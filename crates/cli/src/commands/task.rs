//! Task commands for SCP CLI
//!
//! Provides task management commands: list, show, claim, yield, start, done

use scp_beads::{Bead, BeadId, BeadState, BeadTitle, InMemoryBeadRepository, Result};
use scp_core::{
    error::Error,
    lock::{LockManager, LockType, MemLockManager},
    Result as CoreResult,
};
use std::sync::Arc;

/// Global task repository instance
fn get_task_repository() -> Arc<InMemoryBeadRepository> {
    Arc::new(InMemoryBeadRepository::new())
}

/// Global lock manager instance for task operations
fn get_lock_manager() -> Arc<dyn LockManager> {
    Arc::new(MemLockManager::new()) as Arc<dyn LockManager>
}

/// List all tasks (beads)
pub fn list() -> CoreResult<()> {
    let repo = get_task_repository();
    let tasks = tokio::runtime::Handle::current()
        .block_on(repo.find_all())
        .map_err(|e| Error::Database(e.to_string()))?;

    if tasks.is_empty() {
        println!("No tasks found");
        return Ok(());
    }

    println!("Tasks ({}):", tasks.len());
    for task in tasks.iter() {
        let assignee = task.assignee.as_deref().unwrap_or("-");
        let state = format!("{:?}", task.state);
        let priority = task
            .priority
            .map(|p| format!("{:?}", p))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "  {} [{}] {} - {}",
            task.id.as_str(),
            priority,
            state,
            assignee
        );
    }

    Ok(())
}

/// Show details of a specific task
pub fn show(task_id: &str) -> CoreResult<()> {
    // Validate task ID format (P1)
    let bead_id = BeadId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let repo = get_task_repository();
    let task = tokio::runtime::Handle::current()
        .block_on(repo.find(&bead_id))
        .map_err(|e| Error::Database(e.to_string()))?
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    display_task(&task);
    Ok(())
}

/// Display a task's details
fn display_task(task: &Bead) {
    println!("Task: {}", task.id.as_str());
    println!("  Title: {}", task.title.as_str());
    if let Some(desc) = &task.description {
        println!("  Description: {}", desc.as_str());
    }
    println!("  State: {:?}", task.state);
    if let Some(priority) = &task.priority {
        println!("  Priority: {:?}", priority);
    }
    println!(
        "  Assignee: {:?}",
        task.assignee.as_deref().unwrap_or("unassigned")
    );
    if !task.labels.as_slice().is_empty() {
        println!("  Labels: {:?}", task.labels.as_slice());
    }
    println!("  Created: {}", task.created_at);
    println!("  Updated: {}", task.updated_at);
}

/// Claim a task (assign to current user)
pub fn claim(task_id: &str) -> CoreResult<()> {
    // Validate task ID (P1)
    let bead_id = BeadId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let repo = get_task_repository();
    let lock = get_lock_manager();

    // Acquire lock for task (Q7)
    let lock_type = LockType::Task(task_id.to_string());
    let _guard = lock
        .acquire(lock_type, "current-user")
        .map_err(|_| Error::TaskLocked(task_id.to_string()))?;

    // Get task (P2)
    let mut task = tokio::runtime::Handle::current()
        .block_on(repo.find(&bead_id))
        .map_err(|e| Error::Database(e.to_string()))?
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    // Check if already claimed by someone else (P3)
    if let Some(assignee) = &task.assignee {
        if assignee != "current-user" {
            return Err(Error::TaskAlreadyClaimed(
                task_id.to_string(),
                assignee.clone(),
            ));
        }
    }

    // Set assignee and transition to InProgress (Q3)
    task.assignee = Some("current-user".to_string());
    let task = task
        .transition(BeadState::InProgress)
        .map_err(|e| Error::InvalidTaskStateTransition(task_id.to_string(), e.to_string()))?;

    tokio::runtime::Handle::current()
        .block_on(repo.update(&task))
        .map_err(|e| Error::Database(e.to_string()))?;

    println!("Task {} claimed", task_id);
    Ok(())
}

/// Yield a task (release assignment)
pub fn yield_task(task_id: &str) -> CoreResult<()> {
    // Validate task ID (P1)
    let bead_id = BeadId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let repo = get_task_repository();
    let lock = get_lock_manager();

    // Acquire lock (Q7)
    let lock_type = LockType::Task(task_id.to_string());
    let _guard = lock
        .acquire(lock_type, "current-user")
        .map_err(|_| Error::TaskLocked(task_id.to_string()))?;

    // Get task (P2)
    let mut task = tokio::runtime::Handle::current()
        .block_on(repo.find(&bead_id))
        .map_err(|e| Error::Database(e.to_string()))?
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    // Check if claimed by current user (P4)
    if task.assignee.as_deref() != Some("current-user") {
        return Err(Error::TaskNotClaimed(task_id.to_string()));
    }

    // Clear assignee and set state to Open (Q4)
    task.assignee = None;
    let task = task
        .transition(BeadState::Open)
        .map_err(|e| Error::InvalidTaskStateTransition(task_id.to_string(), e.to_string()))?;

    tokio::runtime::Handle::current()
        .block_on(repo.update(&task))
        .map_err(|e| Error::Database(e.to_string()))?;

    println!("Task {} yielded", task_id);
    Ok(())
}

/// Start working on a task (transition to InProgress)
pub fn start(task_id: &str) -> CoreResult<()> {
    // Validate task ID (P1)
    let bead_id = BeadId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let repo = get_task_repository();
    let lock = get_lock_manager();

    // Acquire lock (Q7)
    let lock_type = LockType::Task(task_id.to_string());
    let _guard = lock
        .acquire(lock_type, "current-user")
        .map_err(|_| Error::TaskLocked(task_id.to_string()))?;

    // Get task (P2)
    let mut task = tokio::runtime::Handle::current()
        .block_on(repo.find(&bead_id))
        .map_err(|e| Error::Database(e.to_string()))?
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    // Check if claimed by current user (P4)
    if task.assignee.as_deref() != Some("current-user") {
        return Err(Error::TaskNotClaimed(task_id.to_string()));
    }

    // Transition to InProgress (Q5)
    let task = task
        .transition(BeadState::InProgress)
        .map_err(|e| Error::InvalidTaskStateTransition(task_id.to_string(), e.to_string()))?;

    tokio::runtime::Handle::current()
        .block_on(repo.update(&task))
        .map_err(|e| Error::Database(e.to_string()))?;

    println!("Task {} started", task_id);
    Ok(())
}

/// Complete a task (transition to Closed)
pub fn done(task_id: &str) -> CoreResult<()> {
    // Validate task ID (P1)
    let bead_id = BeadId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let repo = get_task_repository();
    let lock = get_lock_manager();

    // Acquire lock (Q7)
    let lock_type = LockType::Task(task_id.to_string());
    let _guard = lock
        .acquire(lock_type, "current-user")
        .map_err(|_| Error::TaskLocked(task_id.to_string()))?;

    // Get task (P2)
    let mut task = tokio::runtime::Handle::current()
        .block_on(repo.find(&bead_id))
        .map_err(|e| Error::Database(e.to_string()))?
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    // Check if claimed by current user (P4)
    if task.assignee.as_deref() != Some("current-user") {
        return Err(Error::TaskNotClaimed(task_id.to_string()));
    }

    // Check state allows transition (P5)
    if task.state.is_closed() {
        return Err(Error::InvalidTaskStateTransition(
            task_id.to_string(),
            "Task is already closed".to_string(),
        ));
    }

    // Transition to Closed (Q6)
    let task = task
        .transition(BeadState::Closed {
            closed_at: chrono::Utc::now(),
        })
        .map_err(|e| Error::InvalidTaskStateTransition(task_id.to_string(), e.to_string()))?;

    tokio::runtime::Handle::current()
        .block_on(repo.update(&task))
        .map_err(|e| Error::Database(e.to_string()))?;

    println!("Task {} completed", task_id);
    Ok(())
}
