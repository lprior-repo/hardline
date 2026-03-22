//! Task commands for SCP CLI
//!
//! Provides task management commands: list, show, claim, yield, start, done

use crate::commands::task_store::{get_task_store, init_demo_tasks};
use crate::commands::task_types::TaskId;
use crate::commands::task_validation::{
    acquire_task_lock, transition_to_claimed, transition_to_done, transition_to_started,
    transition_to_yielded, validate_claimed_by_user, validate_not_claimed_by_other,
    validate_not_closed, validate_task_exists,
};
use scp_core::{error::Error, lock::LockManager, Result as CoreResult};
use std::sync::Arc;
use std::sync::LazyLock;

static LOCK_MANAGER: LazyLock<Arc<dyn LockManager>> =
    LazyLock::new(|| Arc::new(scp_core::lock::MemLockManager::new()) as Arc<dyn LockManager>);

fn get_lock_manager() -> Arc<dyn LockManager> {
    LOCK_MANAGER.clone()
}

fn display_tasks(tasks: &[impl TaskDisplay]) {
    if tasks.is_empty() {
        println!("No tasks found");
        return;
    }

    println!("Tasks ({}):", tasks.len());
    for task in tasks {
        task.display();
    }
}

fn display_task(task: &impl TaskDisplay) {
    task.display_detailed();
}

trait TaskDisplay {
    fn display(&self);
    fn display_detailed(&self);
}

impl TaskDisplay for crate::commands::task_types::Task {
    fn display(&self) {
        let assignee = self.assignee.as_ref().map(|a| a.as_str()).unwrap_or("-");
        let state = format!("{:?}", self.state);
        let priority = self.priority.as_ref().map(|p| p.as_str()).unwrap_or("-");
        println!("  {} [{}] {} - {}", self.id, priority, state, assignee);
    }

    fn display_detailed(&self) {
        println!("Task: {}", self.id);
        println!("  Title: {}", self.title);
        if let Some(desc) = &self.description {
            println!("  Description: {}", desc);
        }
        println!("  State: {:?}", self.state);
        if let Some(priority) = &self.priority {
            println!("  Priority: {}", priority);
        }
        println!(
            "  Assignee: {:?}",
            self.assignee
                .as_ref()
                .map(|a| a.as_str())
                .unwrap_or("unassigned")
        );
        println!("  Created: {}", self.created_at);
        println!("  Updated: {}", self.updated_at);
    }
}

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

pub fn show(task_id: &str, _user: &str) -> CoreResult<()> {
    let _task_id = TaskId::new(task_id).map_err(|e| Error::InvalidTaskId(e.to_string()))?;

    let store = get_task_store();
    let task = store
        .get(task_id)
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;

    display_task(&task);
    Ok(())
}

pub fn claim(task_id: &str, user: &str) -> CoreResult<()> {
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

pub fn yield_task(task_id: &str, user: &str) -> CoreResult<()> {
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

pub fn start(task_id: &str, user: &str) -> CoreResult<()> {
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

pub fn done(task_id: &str, user: &str) -> CoreResult<()> {
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
