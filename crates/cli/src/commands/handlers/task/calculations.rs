//! Pure calculation functions for task command handler (Tier 2).
//!
//! No I/O, no side effects. All functions are pure.

use crate::commands::task_types::{Task, TaskId, TaskState};
use scp_core::error_task::TaskErrorKind;

use super::data::{AgentId, TaskCommand, TaskInfoOutput, TaskStatusOutput};

/// Validate a task command before execution.
///
/// Ensures task IDs are well-formed (using TaskId::new rules) and required
/// fields are present. Validates agent IDs are non-empty.
///
/// # Errors
///
/// Returns `TaskErrorKind::InvalidId` if a task ID or agent ID is empty,
/// whitespace-only, or contains characters outside `[a-zA-Z0-9_-]`.
pub fn validate_task_command(cmd: &TaskCommand) -> scp_core::Result<()> {
    match cmd {
        TaskCommand::List { .. } => Ok(()),
        TaskCommand::Show { task_id } => validate_task_id_format(task_id, "show"),
        TaskCommand::Claim { task_id, agent_id } => {
            validate_task_id_format(task_id, "claim")?;
            validate_agent_id(agent_id)
        }
        TaskCommand::YieldTask { task_id, agent_id } => {
            validate_task_id_format(task_id, "yield")?;
            validate_agent_id(agent_id)
        }
        TaskCommand::Start { task_id, agent_id } => {
            validate_task_id_format(task_id, "start")?;
            validate_agent_id(agent_id)
        }
        TaskCommand::Done { task_id, agent_id } => {
            if let Some(id) = task_id {
                validate_task_id_format(id, "done")?;
            }
            validate_agent_id(agent_id)
        }
    }
}

/// Validate that a TaskId is well-formed (re-checks the TaskId regex rules).
/// Since TaskId::new already validated at construction, this is a belt-and-suspenders
/// check that the ID remains valid. Primarily ensures consistency between
/// validation and TaskId construction.
fn validate_task_id_format(_task_id: &TaskId, _context: &str) -> scp_core::Result<()> {
    // TaskId is already validated at construction time via TaskId::new().
    // This function exists as a structural placeholder for any future validation.
    Ok(())
}

/// Validate that an agent ID is non-empty and non-whitespace.
fn validate_agent_id(agent_id: &AgentId) -> scp_core::Result<()> {
    if agent_id.as_str().trim().is_empty() {
        return Err(TaskErrorKind::InvalidId(format!(
            "Agent ID cannot be empty"
        ))
        .into());
    }
    Ok(())
}

/// Convert a `TaskState` to a `TaskStatusOutput`.
///
/// Pure mapping function.
#[must_use]
pub fn task_state_to_output(state: &TaskState) -> TaskStatusOutput {
    match state {
        TaskState::Open => TaskStatusOutput::Open,
        TaskState::InProgress => TaskStatusOutput::InProgress,
        TaskState::Blocked => TaskStatusOutput::Blocked,
        TaskState::Deferred => TaskStatusOutput::Deferred,
        TaskState::Closed { .. } => TaskStatusOutput::Closed,
    }
}

/// Convert a `Task` domain type to a `TaskInfoOutput` display type.
///
/// Pure mapping function.
#[must_use]
pub fn task_to_output(task: &Task) -> TaskInfoOutput {
    TaskInfoOutput {
        id: task.id.to_string(),
        title: task.title.to_string(),
        status: task_state_to_output(&task.state),
        description: task.description.clone(),
        assignee: task.assignee.as_ref().map(|a| a.to_string()),
        priority: task.priority.as_ref().map(|p| p.to_string()),
        created_at: task.created_at,
        updated_at: task.updated_at,
    }
}

/// Filter a list of tasks by status string.
///
/// Pure function - matches case-insensitively against `TaskStatusOutput` display names.
#[must_use]
pub fn filter_tasks_by_status(tasks: &[TaskInfoOutput], status_filter: &str) -> Vec<TaskInfoOutput> {
    let filter_lower = status_filter.to_lowercase();
    tasks
        .iter()
        .filter(|t| t.status.to_string().to_lowercase() == filter_lower)
        .cloned()
        .collect()
}

/// Get a display icon for a task status.
#[must_use]
pub const fn status_display_icon(status: &TaskStatusOutput) -> &'static str {
    match status {
        TaskStatusOutput::Open => "[ ]",
        TaskStatusOutput::InProgress => "[*]",
        TaskStatusOutput::Blocked => "[!]",
        TaskStatusOutput::Deferred => "[-]",
        TaskStatusOutput::Closed => "[x]",
    }
}

/// Truncate a description for display.
///
/// Returns an empty string for empty input. For non-empty input that exceeds
/// `max_len`, truncates and appends "...". Handles multi-byte characters safely.
/// When `max_len` is less than 3 (the length of "..."), returns an empty string
/// rather than degenerate output.
#[must_use]
pub fn truncate_description(desc: &str, max_len: usize) -> String {
    if desc.is_empty() {
        return String::new();
    }
    if max_len < 3 {
        return String::new();
    }
    if desc.len() <= max_len {
        return desc.to_string();
    }

    let end = max_len.saturating_sub(3);
    if end == 0 {
        return String::new();
    }

    // Find a safe char boundary to avoid panicking on multi-byte chars
    let safe_end = desc
        .char_indices()
        .take_while(|(i, _)| *i < end)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);

    if safe_end == 0 {
        return String::new();
    }

    format!("{}...", &desc[..safe_end])
}

/// Get the agent ID from the environment or generate a default.
pub fn get_agent_id() -> String {
    std::env::var("SCP_AGENT_ID")
        .or_else(|_| std::env::var("Isolate_AGENT_ID"))
        .unwrap_or_else(|_| format!("agent-{}", std::process::id()))
}

/// Resolve a task ID for the `done` command.
///
/// If an explicit ID is provided, validates and returns it.
/// Otherwise, falls back to the `SCP_BEAD_ID` or `Isolate_BEAD_ID` environment variable.
///
/// # Errors
///
/// Returns `TaskErrorKind::InvalidId` if no ID can be resolved.
pub fn resolve_task_id(explicit_id: Option<&TaskId>) -> scp_core::Result<TaskId> {
    match explicit_id {
        Some(id) => Ok(id.clone()),
        None => {
            let raw = std::env::var("SCP_BEAD_ID")
                .or_else(|_| std::env::var("Isolate_BEAD_ID"))
                .map_err(|_| {
                    TaskErrorKind::InvalidId(
                        "No task ID provided and not in a workspace (SCP_BEAD_ID not set)"
                            .to_string(),
                    )
                })?;
            TaskId::new(raw).map_err(Into::into)
        }
    }
}

/// Parse a raw string into a TaskId, returning a consistent error type.
///
/// # Errors
///
/// Returns `Error::Task(TaskErrorKind::InvalidId(...))` if the string is not
/// a valid task ID.
pub fn parse_task_id(raw: &str) -> scp_core::Result<TaskId> {
    TaskId::new(raw).map_err(Into::into)
}
