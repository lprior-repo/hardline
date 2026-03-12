//! Task commands for SCP CLI
//!
//! Provides task management commands: list, show, claim, yield, start, done

use crate::commands::task_types::{Task, TaskState};
use crate::commands::task_validation::{
    acquire_task_lock, transition_to_claimed, transition_to_done, transition_to_started,
    transition_to_yielded, validate_claimed_by_user, validate_not_claimed_by_other,
    validate_not_closed, validate_task_exists, validate_task_id,
};
use scp_core::{error::Error, lock::LockManager, Result as CoreResult};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Global task storage
struct TaskStore {
    tasks: RwLock<HashMap<String, Task>>,
}

impl TaskStore {
    fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
        }
    }

    fn list(&self) -> Vec<Task> {
        self.tasks
            .read()
            .map(|tasks| tasks.values().cloned().collect())
            .unwrap_or_default()
    }

    fn get(&self, id: &str) -> Option<Task> {
        self.tasks
            .read()
            .ok()
            .and_then(|tasks| tasks.get(id).cloned())
    }

    fn update(&self, task: Task) -> CoreResult<()> {
        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;
        if !tasks.contains_key(&task.id) {
            return Err(Error::TaskNotFound(task.id));
        }
        tasks.insert(task.id.clone(), task);
        Ok(())
    }

    fn insert(&self, task: Task) -> CoreResult<()> {
        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;
        if tasks.contains_key(&task.id) {
            return Err(Error::TaskAlreadyClaimed(task.id, "exists".to_string()));
        }
        tasks.insert(task.id.clone(), task);
        Ok(())
    }
}

fn get_task_store() -> Arc<TaskStore> {
    Arc::new(TaskStore::new())
}

fn get_lock_manager() -> Arc<dyn LockManager> {
    Arc::new(scp_core::lock::MemLockManager::new()) as Arc<dyn LockManager>
}

/// Initialize with some demo tasks
fn init_demo_tasks(store: &TaskStore) -> CoreResult<()> {
    let tasks = vec![
        Task::new("task-001", "Implement user authentication"),
        Task::new("task-002", "Add database migration"),
        Task::new("task-003", "Fix memory leak in worker"),
    ];
    for task in tasks {
        store.insert(task)?;
    }
    Ok(())
}

// ============================================================================
// Display functions
// ============================================================================

fn display_tasks(tasks: &[Task]) {
    if tasks.is_empty() {
        println!("No tasks found");
        return;
    }

    println!("Tasks ({}):", tasks.len());
    for task in tasks {
        let assignee = task.assignee.as_deref().unwrap_or("-");
        let state = format!("{:?}", task.state);
        let priority = task.priority.as_deref().unwrap_or("-");
        println!("  {} [{}] {} - {}", task.id, priority, state, assignee);
    }
}

fn display_task(task: &Task) {
    println!("Task: {}", task.id);
    println!("  Title: {}", task.title);
    if let Some(desc) = &task.description {
        println!("  Description: {}", desc);
    }
    println!("  State: {:?}", task.state);
    if let Some(priority) = &task.priority {
        println!("  Priority: {}", priority);
    }
    println!(
        "  Assignee: {:?}",
        task.assignee.as_deref().unwrap_or("unassigned")
    );
    println!("  Created: {}", task.created_at);
    println!("  Updated: {}", task.updated_at);
}

// ============================================================================
// Public command functions
// ============================================================================

/// List all tasks
pub fn list() -> CoreResult<()> {
    let store = get_task_store();
    let tasks = store.list();

    if tasks.is_empty() {
        init_demo_tasks(&store)?;
        let tasks = store.list();
        display_tasks(&tasks);
        return Ok(());
    }

    display_tasks(&tasks);
    Ok(())
}

/// Show details of a specific task
pub fn show(task_id: &str) -> CoreResult<()> {
    if task_id.is_empty() {
        return Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()));
    }

    let store = get_task_store();
    let task = store
        .get(task_id)
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    display_task(&task);
    Ok(())
}

/// Claim a task (assign to current user)
pub fn claim(task_id: &str) -> CoreResult<()> {
    validate_task_id(task_id)?;
    let store = get_task_store();
    let lock = get_lock_manager();
    let _guard = acquire_task_lock(&*lock, task_id, "current-user")?;

    let task = validate_task_exists(store.get(task_id), task_id)?;
    validate_not_claimed_by_other(&task, "current-user")?;

    let updated = transition_to_claimed(task, "current-user");
    store.update(updated)?;
    println!("Task {} claimed", task_id);
    Ok(())
}

/// Yield a task (release assignment)
pub fn yield_task(task_id: &str) -> CoreResult<()> {
    validate_task_id(task_id)?;
    let store = get_task_store();
    let lock = get_lock_manager();
    let _guard = acquire_task_lock(&*lock, task_id, "current-user")?;

    let task = validate_task_exists(store.get(task_id), task_id)?;
    validate_claimed_by_user(&task, "current-user")?;

    let updated = transition_to_yielded(task);
    store.update(updated)?;
    println!("Task {} yielded", task_id);
    Ok(())
}

/// Start working on a task (transition to InProgress)
pub fn start(task_id: &str) -> CoreResult<()> {
    validate_task_id(task_id)?;
    let store = get_task_store();
    let lock = get_lock_manager();
    let _guard = acquire_task_lock(&*lock, task_id, "current-user")?;

    let task = validate_task_exists(store.get(task_id), task_id)?;
    validate_claimed_by_user(&task, "current-user")?;

    let updated = transition_to_started(task);
    store.update(updated)?;
    println!("Task {} started", task_id);
    Ok(())
}

/// Complete a task (transition to Closed)
pub fn done(task_id: &str) -> CoreResult<()> {
    validate_task_id(task_id)?;
    let store = get_task_store();
    let lock = get_lock_manager();
    let _guard = acquire_task_lock(&*lock, task_id, "current-user")?;

    let task = validate_task_exists(store.get(task_id), task_id)?;
    validate_claimed_by_user(&task, "current-user")?;
    validate_not_closed(&task)?;

    let updated = transition_to_done(task);
    store.update(updated)?;
    println!("Task {} completed", task_id);
    Ok(())
}
