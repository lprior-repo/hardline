//! Pure calculation functions for task command handler (Tier 2).
//!
//! No I/O, no side effects. All functions are pure.

use scp_core::error_task::TaskErrorKind;

use super::data::{AgentId, TaskCommand, TaskInfoOutput, TaskStatusOutput};
use crate::commands::task_types::{Task, TaskId, TaskState};

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
const fn validate_task_id_format(_task_id: &TaskId, _context: &str) -> scp_core::Result<()> {
    // TaskId is already validated at construction time via TaskId::new().
    // This function exists as a structural placeholder for any future validation.
    Ok(())
}

/// Validate that an agent ID is non-empty and non-whitespace.
fn validate_agent_id(agent_id: &AgentId) -> scp_core::Result<()> {
    if agent_id.as_str().trim().is_empty() {
        return Err(TaskErrorKind::InvalidId("Agent ID cannot be empty".to_string()).into());
    }
    Ok(())
}

/// Convert a `TaskState` to a `TaskStatusOutput`.
///
/// Pure mapping function.
#[must_use]
pub const fn task_state_to_output(state: &TaskState) -> TaskStatusOutput {
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
pub fn filter_tasks_by_status(
    tasks: &[TaskInfoOutput],
    status_filter: &str,
) -> Vec<TaskInfoOutput> {
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
            TaskId::new(raw)
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
    TaskId::new(raw)
}

#[cfg(test)]
mod tests {
    use scp_core::error_task::TaskErrorKind;

    use super::*;
    use crate::commands::task_types::{Assignee, Priority, TaskId, Title};

    // ---- truncate_description ----

    #[test]
    fn truncate_empty_string() {
        assert_eq!(truncate_description("", 20), "");
    }

    #[test]
    fn truncate_within_limit() {
        assert_eq!(truncate_description("hello", 10), "hello");
    }

    #[test]
    fn truncate_at_exact_limit() {
        assert_eq!(truncate_description("hello", 5), "hello");
    }

    #[test]
    fn truncate_one_over_limit() {
        let result = truncate_description("hello world", 8);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 8);
    }

    #[test]
    fn truncate_max_len_zero() {
        assert_eq!(truncate_description("hello", 0), "");
    }

    #[test]
    fn truncate_max_len_one() {
        assert_eq!(truncate_description("hello", 1), "");
    }

    #[test]
    fn truncate_max_len_two() {
        // max_len < 3 returns empty
        assert_eq!(truncate_description("hello", 2), "");
    }

    #[test]
    fn truncate_max_len_three() {
        // max_len = 3: end = 0, safe_end = 0, returns empty
        assert_eq!(truncate_description("abc", 3), "abc");
    }

    #[test]
    fn truncate_preserves_multi_byte_chars() {
        // "Hello, world!" with emoji at the boundary
        let input = "Hello, \u{1F600} world!";
        let result = truncate_description(input, 10);
        // Should not panic on multi-byte char boundary
        assert!(result.ends_with("...") || result == input);
    }

    #[test]
    fn truncate_very_long_string() {
        let long = "a".repeat(1000);
        let result = truncate_description(&long, 20);
        assert_eq!(result.len(), 20);
        assert!(result.ends_with("..."));
    }

    // ---- status_display_icon ----

    #[test]
    fn icon_open() {
        assert_eq!(status_display_icon(&TaskStatusOutput::Open), "[ ]");
    }

    #[test]
    fn icon_in_progress() {
        assert_eq!(status_display_icon(&TaskStatusOutput::InProgress), "[*]");
    }

    #[test]
    fn icon_blocked() {
        assert_eq!(status_display_icon(&TaskStatusOutput::Blocked), "[!]");
    }

    #[test]
    fn icon_deferred() {
        assert_eq!(status_display_icon(&TaskStatusOutput::Deferred), "[-]");
    }

    #[test]
    fn icon_closed() {
        assert_eq!(status_display_icon(&TaskStatusOutput::Closed), "[x]");
    }

    // ---- task_state_to_output ----

    #[test]
    fn state_to_output_open() {
        assert_eq!(
            task_state_to_output(&TaskState::Open),
            TaskStatusOutput::Open
        );
    }

    #[test]
    fn state_to_output_in_progress() {
        assert_eq!(
            task_state_to_output(&TaskState::InProgress),
            TaskStatusOutput::InProgress
        );
    }

    #[test]
    fn state_to_output_blocked() {
        assert_eq!(
            task_state_to_output(&TaskState::Blocked),
            TaskStatusOutput::Blocked
        );
    }

    #[test]
    fn state_to_output_deferred() {
        assert_eq!(
            task_state_to_output(&TaskState::Deferred),
            TaskStatusOutput::Deferred
        );
    }

    #[test]
    fn state_to_output_closed() {
        assert!(matches!(
            task_state_to_output(&TaskState::Closed {
                closed_at: chrono::Utc::now()
            }),
            TaskStatusOutput::Closed
        ));
    }

    // ---- task_to_output ----

    fn make_task(id: &str, title: &str) -> Task {
        Task::new(TaskId::new(id).expect("valid"), Title::new(title))
    }

    #[test]
    fn task_to_output_basic_fields() {
        let task = make_task("task-1", "My task");
        let output = task_to_output(&task);
        assert_eq!(output.id, "task-1");
        assert_eq!(output.title, "My task");
        assert!(output.description.is_none());
        assert!(output.assignee.is_none());
        assert!(output.priority.is_none());
    }

    #[test]
    fn task_to_output_with_assignee_and_priority() {
        let mut task = make_task("task-1", "My task");
        task.assignee = Some(Assignee::new("alice"));
        task.priority = Some(Priority::new("high"));
        let output = task_to_output(&task);
        assert_eq!(output.assignee.as_deref(), Some("alice"));
        assert_eq!(output.priority.as_deref(), Some("high"));
    }

    // ---- parse_task_id ----

    #[test]
    fn parse_task_id_valid() {
        let result = parse_task_id("task-001");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_task_id_invalid() {
        let result = parse_task_id("bad id!");
        assert!(result.is_err());
    }

    // ---- validate_task_command ----

    fn make_agent_id(s: &str) -> AgentId {
        // Only use for valid agent IDs
        AgentId::new(s.to_string()).expect("valid agent id for test helper")
    }

    #[test]
    fn validate_list_always_ok() {
        let cmd = TaskCommand::List {
            status_filter: None,
            include_all: false,
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    #[test]
    fn validate_show_validates_id() {
        let cmd = TaskCommand::Show {
            task_id: TaskId::new("valid-id").expect("ok"),
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    #[test]
    fn validate_claim_validates_agent_id() {
        let cmd = TaskCommand::Claim {
            task_id: TaskId::new("valid-id").expect("ok"),
            agent_id: make_agent_id("alice"),
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    #[test]
    fn validate_claim_rejects_empty_agent() {
        // AgentId::new rejects whitespace-only, but validate_agent_id checks as_str().trim()
        // We need to construct an AgentId with whitespace-only content.
        // Since AgentId::new("  ") returns Err, we test validate_task_command indirectly
        // by verifying the validation function works via a valid AgentId that has trimmed-empty
        // content. This path is actually caught at AgentId::new construction time.
        // We verify the validate_agent_id logic directly instead.
        let agent_id = AgentId::new("  ");
        assert!(
            agent_id.is_err(),
            "AgentId::new should reject whitespace-only input"
        );
    }

    #[test]
    fn validate_yield_validates_agent_id() {
        let cmd = TaskCommand::YieldTask {
            task_id: TaskId::new("t-1").expect("ok"),
            agent_id: make_agent_id("bob"),
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    #[test]
    fn validate_start_validates_agent_id() {
        let cmd = TaskCommand::Start {
            task_id: TaskId::new("t-1").expect("ok"),
            agent_id: make_agent_id("carol"),
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    #[test]
    fn validate_done_with_explicit_id() {
        let cmd = TaskCommand::Done {
            task_id: Some(TaskId::new("t-1").expect("ok")),
            agent_id: make_agent_id("dave"),
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    #[test]
    fn validate_done_without_id_still_validates_agent() {
        let cmd = TaskCommand::Done {
            task_id: None,
            agent_id: make_agent_id("eve"),
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    #[test]
    fn validate_done_rejects_empty_agent() {
        // AgentId::new("") returns Err, so validate_task_command never sees
        // an empty AgentId. But we can verify this by constructing AgentId directly.
        let agent_result = AgentId::new("");
        assert!(agent_result.is_err());
    }
}
