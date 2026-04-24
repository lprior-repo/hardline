//! Action functions for task command handler (Tier 3).
//!
//! I/O operations that use scp_core. All validation is delegated to Tier 2.

use crate::commands::task_store::get_task_store;
use crate::commands::task_types::TaskId;
use crate::commands::task_validation::{
    acquire_task_lock, transition_to_claimed, transition_to_done, transition_to_started,
    transition_to_yielded, validate_claimed_by_user, validate_not_claimed_by_other,
    validate_not_closed, validate_task_exists,
};
use scp_core::error::Error;
use scp_core::error_task::TaskErrorKind;
use scp_core::lock::LockManager;
use scp_core::output::Output;

use super::calculations::{
    filter_tasks_by_status, resolve_task_id, status_display_icon, task_to_output,
    truncate_description, validate_task_command,
};
use super::data::{TaskCommand, TaskListOutput};

/// Execute a validated task command.
///
/// Dispatches to the appropriate subcommand handler after validation.
///
/// # Errors
///
/// Returns errors from validation failure or subcommand execution.
pub fn execute_task_command(cmd: &TaskCommand, lock: &dyn LockManager) -> scp_core::Result<()> {
    // TIER 2: Validate before any I/O
    validate_task_command(cmd)?;

    // TIER 3: Dispatch to subcommand handler
    match cmd {
        TaskCommand::List {
            status_filter,
            include_all,
        } => execute_list(status_filter.as_deref(), *include_all),
        TaskCommand::Show { task_id } => execute_show(task_id),
        TaskCommand::Claim { task_id, agent_id } => execute_claim(task_id, agent_id, lock),
        TaskCommand::YieldTask { task_id, agent_id } => execute_yield(task_id, agent_id, lock),
        TaskCommand::Start { task_id, agent_id } => execute_start(task_id, agent_id, lock),
        TaskCommand::Done { task_id, agent_id } => {
            let resolved_id = resolve_task_id(task_id.as_ref())?;
            execute_done(&resolved_id, agent_id, lock)
        }
    }
}

/// Execute the task list subcommand.
fn execute_list(status_filter: Option<&str>, include_all: bool) -> scp_core::Result<()> {
    let store = get_task_store()?;
    let tasks = store.list();

    let outputs: Vec<_> = tasks.iter().map(task_to_output).collect();

    let filtered = if include_all {
        outputs
    } else if let Some(filter) = status_filter {
        filter_tasks_by_status(&outputs, filter)
    } else {
        outputs
    };

    let result = TaskListOutput {
        total: filtered.len(),
        tasks: filtered,
    };

    if result.tasks.is_empty() {
        Output::info("No tasks found.");
        return Ok(());
    }

    Output::info(&format!("Tasks ({} total):", result.total));
    for task in &result.tasks {
        let icon = status_display_icon(&task.status);
        Output::info(&format!("  {} {} - {}", icon, task.id, task.title));
        if let Some(ref desc) = task.description {
            let truncated = truncate_description(desc, 60);
            if !truncated.is_empty() {
                Output::info(&format!("      {truncated}"));
            }
        }
    }

    Ok(())
}

/// Execute the task show subcommand.
fn execute_show(task_id: &TaskId) -> scp_core::Result<()> {
    let store = get_task_store()?;
    let task = store
        .get(task_id.as_str())
        .ok_or_else(|| Error::from(TaskErrorKind::NotFound(task_id.to_string())))?;

    let output = task_to_output(&task);

    Output::info(&format!("Task: {}", output.id));
    Output::info(&format!("  Title: {}", output.title));
    Output::info(&format!("  Status: {}", output.status));
    if let Some(ref desc) = output.description {
        Output::info(&format!("  Description: {desc}"));
    }
    if let Some(ref priority) = output.priority {
        Output::info(&format!("  Priority: {priority}"));
    }
    if let Some(ref assignee) = output.assignee {
        Output::info(&format!("  Assignee: {assignee}"));
    }

    Ok(())
}

/// Execute the task claim subcommand.
fn execute_claim(
    task_id: &TaskId,
    agent_id: &super::data::AgentId,
    lock: &dyn LockManager,
) -> scp_core::Result<()> {
    let store = get_task_store()?;
    let _guard = acquire_task_lock(lock, task_id.as_str(), agent_id.as_str())?;

    let task = validate_task_exists(store.get(task_id.as_str()), task_id.as_str())?;
    validate_not_claimed_by_other(&task, agent_id.as_str())?;

    let updated = transition_to_claimed(task, agent_id.as_str());
    store.update(updated)?;

    Output::success(&format!("Claimed task '{}'", task_id));
    Ok(())
}

/// Execute the task yield subcommand.
fn execute_yield(
    task_id: &TaskId,
    agent_id: &super::data::AgentId,
    lock: &dyn LockManager,
) -> scp_core::Result<()> {
    let store = get_task_store()?;
    let _guard = acquire_task_lock(lock, task_id.as_str(), agent_id.as_str())?;

    let task = validate_task_exists(store.get(task_id.as_str()), task_id.as_str())?;
    validate_claimed_by_user(&task, agent_id.as_str())?;

    let updated = transition_to_yielded(task);
    store.update(updated)?;

    Output::success(&format!("Yielded task '{}'", task_id));
    Ok(())
}

/// Execute the task start subcommand.
fn execute_start(
    task_id: &TaskId,
    agent_id: &super::data::AgentId,
    lock: &dyn LockManager,
) -> scp_core::Result<()> {
    let store = get_task_store()?;
    let _guard = acquire_task_lock(lock, task_id.as_str(), agent_id.as_str())?;

    // First claim the task
    let task = validate_task_exists(store.get(task_id.as_str()), task_id.as_str())?;
    validate_not_claimed_by_other(&task, agent_id.as_str())?;

    let claimed = transition_to_claimed(task, agent_id.as_str());
    let started = transition_to_started(claimed);
    store.update(started)?;

    let workspace = format!(".scp/workspaces/{task_id}");
    Output::success(&format!("Started task '{task_id}'"));
    Output::info(&format!("  Workspace: {workspace}"));

    Ok(())
}

/// Execute the task done subcommand.
fn execute_done(
    task_id: &TaskId,
    agent_id: &super::data::AgentId,
    lock: &dyn LockManager,
) -> scp_core::Result<()> {
    let store = get_task_store()?;
    let _guard = acquire_task_lock(lock, task_id.as_str(), agent_id.as_str())?;

    let task = validate_task_exists(store.get(task_id.as_str()), task_id.as_str())?;
    validate_claimed_by_user(&task, agent_id.as_str())?;
    validate_not_closed(&task)?;

    let updated = transition_to_done(task);
    store.update(updated)?;

    let output = task_to_output(
        &store
            .get(task_id.as_str())
            .ok_or_else(|| Error::internal("Task disappeared after completion"))?,
    );

    Output::success(&format!("Completed task '{task_id}'"));
    Output::info(&format!("  Title: {}", output.title));
    Output::info(&format!("  Status: {}", output.status));

    Ok(())
}

/// High-level entry point for CLI task command dispatch.
///
/// Constructs internal types from raw CLI arguments and delegates to
/// `execute_task_command`.
///
/// # Errors
///
/// Propagates any validation or execution errors.
pub fn run_task_command(cmd: &TaskCommand) -> scp_core::Result<()> {
    let lock = scp_core::lock::MemLockManager::new();
    execute_task_command(cmd, &lock)
}
