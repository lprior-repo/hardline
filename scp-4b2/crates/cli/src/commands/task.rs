//! Task commands for SCP CLI
//!
//! Provides task management commands: list, show, claim, yield, start, done

use scp_core::{
    error::Error,
    lock::{LockGuard, LockManager, LockType, MemLockManager},
    Result as CoreResult,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Task state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed {
        closed_at: chrono::DateTime<chrono::Utc>,
    },
}

/// Task representation
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub state: TaskState,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Task {
    fn new(id: &str, title: &str) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: id.to_string(),
            title: title.to_string(),
            description: None,
            state: TaskState::Open,
            priority: None,
            assignee: None,
            created_at: now,
            updated_at: now,
        }
    }
}

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
    Arc::new(MemLockManager::new()) as Arc<dyn LockManager>
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
// Pure validation functions (extracted to reduce command size)
// ============================================================================

fn validate_task_id(task_id: &str) -> CoreResult<()> {
    if task_id.is_empty() {
        return Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()));
    }
    Ok(())
}

fn validate_task_exists(task: Option<Task>, task_id: &str) -> CoreResult<Task> {
    task.ok_or_else(|| Error::TaskNotFound(task_id.to_string()))
}

fn validate_not_claimed_by_other(task: &Task, current_user: &str) -> CoreResult<()> {
    if let Some(assignee) = &task.assignee {
        if assignee != current_user {
            return Err(Error::TaskAlreadyClaimed(task.id.clone(), assignee.clone()));
        }
    }
    Ok(())
}

fn validate_claimed_by_user(task: &Task, current_user: &str) -> CoreResult<()> {
    if task.assignee.as_deref() != Some(current_user) {
        return Err(Error::TaskNotClaimed(task.id.clone()));
    }
    Ok(())
}

fn validate_not_closed(task: &Task) -> CoreResult<()> {
    if matches!(task.state, TaskState::Closed { .. }) {
        return Err(Error::InvalidTaskStateTransition(
            task.id.clone(),
            "Task is already closed".to_string(),
        ));
    }
    Ok(())
}

fn acquire_task_lock(lock: &dyn LockManager, task_id: &str, holder: &str) -> CoreResult<LockGuard> {
    let lock_type = LockType::Task(task_id.to_string());
    lock.acquire(lock_type, holder)
        .map_err(|_| Error::TaskLocked(task_id.to_string()))
}

// ============================================================================
// State transition pure functions
// ============================================================================

fn transition_to_claimed(task: Task, user: &str) -> Task {
    let mut t = task;
    t.assignee = Some(user.to_string());
    t.state = TaskState::InProgress;
    t.updated_at = chrono::Utc::now();
    t
}

fn transition_to_yielded(task: Task) -> Task {
    let mut t = task;
    t.assignee = None;
    t.state = TaskState::Open;
    t.updated_at = chrono::Utc::now();
    t
}

fn transition_to_started(task: Task) -> Task {
    let mut t = task;
    t.state = TaskState::InProgress;
    t.updated_at = chrono::Utc::now();
    t
}

fn transition_to_done(task: Task) -> Task {
    let mut t = task;
    t.state = TaskState::Closed {
        closed_at: chrono::Utc::now(),
    };
    t.updated_at = chrono::Utc::now();
    t
}

/// List all tasks
pub fn list() -> CoreResult<()> {
    let store = get_task_store();
    let tasks = store.list();

    if tasks.is_empty() {
        // Initialize demo tasks if empty
        init_demo_tasks(&store)?;
        let tasks = store.list();
        display_tasks(&tasks);
        return Ok(());
    }

    display_tasks(&tasks);
    Ok(())
}

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

/// Show details of a specific task
pub fn show(task_id: &str) -> CoreResult<()> {
    // Validate task ID format (P1)
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

/// Display a task's details
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

/// Claim a task (assign to current user)
pub fn claim(task_id: &str) -> CoreResult<()> {
    // Validate task ID (P1)
    if task_id.is_empty() {
        return Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()));
    }

    let store = get_task_store();
    let lock = get_lock_manager();

    // Acquire lock for task (Q7)
    let lock_type = LockType::Task(task_id.to_string());
    let _guard = lock
        .acquire(lock_type, "current-user")
        .map_err(|_| Error::TaskLocked(task_id.to_string()))?;

    // Get task (P2)
    let mut task = store
        .get(task_id)
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
    task.state = TaskState::InProgress;
    task.updated_at = chrono::Utc::now();

    store.update(task)?;

    println!("Task {} claimed", task_id);
    Ok(())
}

/// Yield a task (release assignment)
pub fn yield_task(task_id: &str) -> CoreResult<()> {
    // Validate task ID (P1)
    if task_id.is_empty() {
        return Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()));
    }

    let store = get_task_store();
    let lock = get_lock_manager();

    // Acquire lock (Q7)
    let lock_type = LockType::Task(task_id.to_string());
    let _guard = lock
        .acquire(lock_type, "current-user")
        .map_err(|_| Error::TaskLocked(task_id.to_string()))?;

    // Get task (P2)
    let mut task = store
        .get(task_id)
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    // Check if claimed by current user (P4)
    if task.assignee.as_deref() != Some("current-user") {
        return Err(Error::TaskNotClaimed(task_id.to_string()));
    }

    // Clear assignee and set state to Open (Q4)
    task.assignee = None;
    task.state = TaskState::Open;
    task.updated_at = chrono::Utc::now();

    store.update(task)?;

    println!("Task {} yielded", task_id);
    Ok(())
}

/// Start working on a task (transition to InProgress)
pub fn start(task_id: &str) -> CoreResult<()> {
    // Validate task ID (P1)
    if task_id.is_empty() {
        return Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()));
    }

    let store = get_task_store();
    let lock = get_lock_manager();

    // Acquire lock (Q7)
    let lock_type = LockType::Task(task_id.to_string());
    let _guard = lock
        .acquire(lock_type, "current-user")
        .map_err(|_| Error::TaskLocked(task_id.to_string()))?;

    // Get task (P2)
    let mut task = store
        .get(task_id)
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    // Check if claimed by current user (P4)
    if task.assignee.as_deref() != Some("current-user") {
        return Err(Error::TaskNotClaimed(task_id.to_string()));
    }

    // Transition to InProgress (Q5)
    task.state = TaskState::InProgress;
    task.updated_at = chrono::Utc::now();

    store.update(task)?;

    println!("Task {} started", task_id);
    Ok(())
}

/// Complete a task (transition to Closed)
pub fn done(task_id: &str) -> CoreResult<()> {
    // Validate task ID (P1)
    if task_id.is_empty() {
        return Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()));
    }

    let store = get_task_store();
    let lock = get_lock_manager();

    // Acquire lock (Q7)
    let lock_type = LockType::Task(task_id.to_string());
    let _guard = lock
        .acquire(lock_type, "current-user")
        .map_err(|_| Error::TaskLocked(task_id.to_string()))?;

    // Get task (P2)
    let mut task = store
        .get(task_id)
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    // Check if claimed by current user (P4)
    if task.assignee.as_deref() != Some("current-user") {
        return Err(Error::TaskNotClaimed(task_id.to_string()));
    }

    // Check state allows transition (P5)
    if matches!(task.state, TaskState::Closed { .. }) {
        return Err(Error::InvalidTaskStateTransition(
            task_id.to_string(),
            "Task is already closed".to_string(),
        ));
    }

    // Transition to Closed (Q6)
    task.state = TaskState::Closed {
        closed_at: chrono::Utc::now(),
    };
    task.updated_at = chrono::Utc::now();

    store.update(task)?;

    println!("Task {} completed", task_id);
    Ok(())
}
