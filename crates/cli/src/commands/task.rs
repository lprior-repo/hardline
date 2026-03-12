//! Task commands for SCP CLI
//!
//! Provides task management commands: list, show, claim, yield, start, done

use crate::commands::task_types::{Priority, Task, TaskId, TaskState, Title};
use crate::commands::task_validation::{
    acquire_task_lock, transition_to_claimed, transition_to_done, transition_to_started,
    transition_to_yielded, validate_claimed_by_user, validate_not_claimed_by_other,
    validate_not_closed, validate_task_exists,
};
use scp_core::{error::Error, lock::LockManager, Result as CoreResult};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::{Arc, RwLock};

/// Global task storage - singleton using LazyLock for thread-safe initialization
static TASK_STORE: LazyLock<Arc<TaskStore>> = LazyLock::new(|| Arc::new(TaskStore::new()));

/// Global lock manager - singleton using LazyLock
static LOCK_MANAGER: LazyLock<Arc<dyn LockManager>> =
    LazyLock::new(|| Arc::new(scp_core::lock::MemLockManager::new()) as Arc<dyn LockManager>);

/// Global in-memory task store with RwLock for interior mutability
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
            .unwrap_or_else(|_| Vec::new())
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
        if !tasks.contains_key(task.id.as_str()) {
            return Err(Error::TaskNotFound(task.id.to_string()));
        }
        tasks.insert(task.id.to_string(), task);
        Ok(())
    }

    fn insert(&self, task: Task) -> CoreResult<()> {
        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;
        if tasks.contains_key(task.id.as_str()) {
            return Err(Error::TaskAlreadyClaimed(
                task.id.to_string(),
                "exists".to_string(),
            ));
        }
        tasks.insert(task.id.to_string(), task);
        Ok(())
    }
}

/// Get the global task store singleton
fn get_task_store() -> Arc<TaskStore> {
    TASK_STORE.clone()
}

/// Get the global lock manager singleton
fn get_lock_manager() -> Arc<dyn LockManager> {
    LOCK_MANAGER.clone()
}

/// Initialize with some demo tasks
fn init_demo_tasks(store: &TaskStore) -> CoreResult<()> {
    let tasks = vec![
        Task::new(
            TaskId::new("task-001").map_err(|e| Error::InvalidTaskId(e.to_string()))?,
            Title::new("Implement user authentication"),
        ),
        Task::new(
            TaskId::new("task-002").map_err(|e| Error::InvalidTaskId(e.to_string()))?,
            Title::new("Add database migration"),
        ),
        Task::new(
            TaskId::new("task-003").map_err(|e| Error::InvalidTaskId(e.to_string()))?,
            Title::new("Fix memory leak in worker"),
        ),
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
        let assignee = task.assignee.as_ref().map(|a| a.as_str()).unwrap_or("-");
        let state = format!("{:?}", task.state);
        let priority = task.priority.as_ref().map(|p| p.as_str()).unwrap_or("-");
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
        task.assignee
            .as_ref()
            .map(|a| a.as_str())
            .unwrap_or("unassigned")
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
pub fn show(task_id: &str, _user: &str) -> CoreResult<()> {
    // Validate task ID at parse time
    let _task_id = TaskId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let store = get_task_store();
    let task = store
        .get(task_id)
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    display_task(&task);
    Ok(())
}

/// Claim a task (assign to current user)
pub fn claim(task_id: &str, user: &str) -> CoreResult<()> {
    // Validate task ID at parse time
    let _task_id = TaskId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let store = get_task_store();
    let lock = get_lock_manager();
    let _guard = acquire_task_lock(&*lock, task_id, user)?;

    let task = validate_task_exists(store.get(task_id), task_id)?;
    validate_not_claimed_by_other(&task, user)?;

    let updated = transition_to_claimed(task, user);
    store.update(updated)?;
    println!("Task {} claimed", task_id);
    Ok(())
}

/// Yield a task (release assignment)
pub fn yield_task(task_id: &str, user: &str) -> CoreResult<()> {
    // Validate task ID at parse time
    let _task_id = TaskId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let store = get_task_store();
    let lock = get_lock_manager();
    let _guard = acquire_task_lock(&*lock, task_id, user)?;

    let task = validate_task_exists(store.get(task_id), task_id)?;
    validate_claimed_by_user(&task, user)?;

    let updated = transition_to_yielded(task);
    store.update(updated)?;
    println!("Task {} yielded", task_id);
    Ok(())
}

/// Start working on a task (transition to InProgress)
pub fn start(task_id: &str, user: &str) -> CoreResult<()> {
    // Validate task ID at parse time
    let _task_id = TaskId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let store = get_task_store();
    let lock = get_lock_manager();
    let _guard = acquire_task_lock(&*lock, task_id, user)?;

    let task = validate_task_exists(store.get(task_id), task_id)?;
    validate_claimed_by_user(&task, user)?;

    let updated = transition_to_started(task);
    store.update(updated)?;
    println!("Task {} started", task_id);
    Ok(())
}

/// Complete a task (transition to Closed)
pub fn done(task_id: &str, user: &str) -> CoreResult<()> {
    // Validate task ID at parse time
    let _task_id = TaskId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let store = get_task_store();
    let lock = get_lock_manager();
    let _guard = acquire_task_lock(&*lock, task_id, user)?;

    let task = validate_task_exists(store.get(task_id), task_id)?;
    validate_claimed_by_user(&task, user)?;
    validate_not_closed(&task)?;

    let updated = transition_to_done(task);
    store.update(updated)?;
    println!("Task {} completed", task_id);
    Ok(())
}
