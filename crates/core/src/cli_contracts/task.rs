//! KIRK Contracts for Task CLI operations.
//!
//! Tasks represent beads (work items) that can be managed through the CLI.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::cli_contracts::{Contract, ContractError, Invariant, Postcondition, Precondition};

// ═══════════════════════════════════════════════════════════════════════════
// TASK INPUT/OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Input for creating a task.
#[derive(Debug, Clone)]
pub struct CreateTaskInput {
    /// Task title
    pub title: String,
    /// Task description
    pub description: Option<String>,
    /// Task priority (P0-P4)
    pub priority: Option<String>,
    /// Task type (feature, bug, task, etc.)
    pub task_type: Option<String>,
    /// Labels/tags
    pub labels: Vec<String>,
}

/// Input for updating a task.
#[derive(Debug, Clone)]
pub struct UpdateTaskInput {
    /// Task ID
    pub task_id: String,
    /// New title (if changing)
    pub title: Option<String>,
    /// New description (if changing)
    pub description: Option<String>,
    /// New status (if changing)
    pub status: Option<String>,
}

/// Input for listing tasks.
#[derive(Debug, Clone, Default)]
pub struct ListTasksInput {
    /// Filter by status
    pub status: Option<String>,
    /// Filter by priority
    pub priority: Option<String>,
    /// Filter by label
    pub label: Option<String>,
    /// Maximum results
    pub limit: Option<usize>,
}

/// Result of task creation.
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Task ID
    pub id: String,
    /// Task title
    pub title: String,
    /// Current status
    pub status: String,
}

/// Result of task listing.
#[derive(Debug, Clone)]
pub struct TaskListResult {
    /// List of tasks
    pub tasks: Vec<TaskResult>,
    /// Total count before pagination
    pub total: usize,
}

// ═══════════════════════════════════════════════════════════════════════════
// TASK CONTRACTS
// ═══════════════════════════════════════════════════════════════════════════

/// Contracts for Task CLI operations.
pub struct TaskContracts;

impl TaskContracts {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PRECONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Precondition: task title is valid.
    pub const PRECOND_TITLE_VALID: Precondition = Precondition::new(
        "title_valid",
        "Task title must be non-empty and at most 200 characters",
    );

    /// Precondition: task ID exists.
    pub const PRECOND_TASK_EXISTS: Precondition =
        Precondition::new("task_exists", "Task must exist in the database");

    /// Precondition: priority is valid.
    pub const PRECOND_PRIORITY_VALID: Precondition = Precondition::new(
        "priority_valid",
        "Priority must be one of: P0, P1, P2, P3, P4",
    );

    /// Precondition: status is valid.
    pub const PRECOND_STATUS_VALID: Precondition = Precondition::new(
        "status_valid",
        "Status must be one of: open, in_progress, blocked, closed",
    );

    /// Precondition: limit is reasonable.
    pub const PRECOND_LIMIT_VALID: Precondition =
        Precondition::new("limit_valid", "Limit must be between 1 and 1000");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INVARIANTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Invariant: task ID is unique.
    pub const INV_TASK_ID_UNIQUE: Invariant =
        Invariant::documented("task_id_unique", "Each task has a unique identifier");

    /// Invariant: task timestamps are consistent.
    pub const INV_TIMESTAMPS_CONSISTENT: Invariant = Invariant::documented(
        "task_timestamps_consistent",
        "Updated_at must be >= created_at",
    );

    /// Invariant: closed tasks have `closed_at`.
    pub const INV_CLOSED_AT_SET: Invariant = Invariant::documented(
        "closed_at_set",
        "Closed tasks must have closed_at timestamp",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // POSTCONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Postcondition: task was created.
    pub const POST_TASK_CREATED: Postcondition =
        Postcondition::new("task_created", "Task exists in database after creation");

    /// Postcondition: task was updated.
    pub const POST_TASK_UPDATED: Postcondition =
        Postcondition::new("task_updated", "Task has new values after update");

    /// Postcondition: task was closed.
    pub const POST_TASK_CLOSED: Postcondition =
        Postcondition::new("task_closed", "Task status is 'closed' after closing");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALIDATION METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Validate a task title.
    ///
    /// # Errors
    /// Returns `ContractError` if the title is invalid.
    pub fn validate_title(title: &str) -> Result<(), ContractError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(ContractError::invalid_input("title", "cannot be empty"));
        }
        if trimmed.len() > 200 {
            return Err(ContractError::invalid_input(
                "title",
                "cannot exceed 200 characters",
            ));
        }
        Ok(())
    }

    /// Validate a priority value.
    ///
    /// # Errors
    /// Returns `ContractError` if the priority is invalid.
    pub fn validate_priority(priority: &str) -> Result<(), ContractError> {
        match priority {
            "P0" | "P1" | "P2" | "P3" | "P4" => Ok(()),
            _ => Err(ContractError::invalid_input(
                "priority",
                "must be one of: P0, P1, P2, P3, P4",
            )),
        }
    }

    /// Validate a status value.
    ///
    /// # Errors
    /// Returns `ContractError` if the status is invalid.
    pub fn validate_status(status: &str) -> Result<(), ContractError> {
        match status {
            "open" | "in_progress" | "blocked" | "closed" => Ok(()),
            _ => Err(ContractError::invalid_input(
                "status",
                "must be one of: open, in_progress, blocked, closed",
            )),
        }
    }

    /// Validate a limit value.
    ///
    /// # Errors
    /// Returns `ContractError` if the limit is invalid.
    pub fn validate_limit(limit: usize) -> Result<(), ContractError> {
        if limit == 0 {
            return Err(ContractError::invalid_input("limit", "must be at least 1"));
        }
        if limit > 1000 {
            return Err(ContractError::invalid_input("limit", "cannot exceed 1000"));
        }
        Ok(())
    }
}

impl Contract<CreateTaskInput, TaskResult> for TaskContracts {
    fn preconditions(input: &CreateTaskInput) -> Result<(), ContractError> {
        Self::validate_title(&input.title)?;

        if let Some(ref priority) = input.priority {
            Self::validate_priority(priority)?;
        }

        Ok(())
    }

    fn invariants(_input: &CreateTaskInput) -> Vec<Invariant> {
        vec![Self::INV_TASK_ID_UNIQUE, Self::INV_TIMESTAMPS_CONSISTENT]
    }

    fn postconditions(input: &CreateTaskInput, result: &TaskResult) -> Result<(), ContractError> {
        if result.title != input.title {
            return Err(ContractError::PostconditionFailed {
                name: "title_matches",
                description: "Created task title must match input",
            });
        }
        if result.status != "open" {
            return Err(ContractError::PostconditionFailed {
                name: "initial_status",
                description: "New tasks must have status 'open'",
            });
        }
        Ok(())
    }
}

impl Contract<UpdateTaskInput, TaskResult> for TaskContracts {
    fn preconditions(input: &UpdateTaskInput) -> Result<(), ContractError> {
        if input.task_id.trim().is_empty() {
            return Err(ContractError::invalid_input("task_id", "cannot be empty"));
        }

        if let Some(ref title) = input.title {
            Self::validate_title(title)?;
        }

        if let Some(ref status) = input.status {
            Self::validate_status(status)?;
        }

        Ok(())
    }

    fn invariants(_input: &UpdateTaskInput) -> Vec<Invariant> {
        vec![Self::INV_TIMESTAMPS_CONSISTENT]
    }

    fn postconditions(input: &UpdateTaskInput, result: &TaskResult) -> Result<(), ContractError> {
        if result.id != input.task_id {
            return Err(ContractError::PostconditionFailed {
                name: "id_matches",
                description: "Updated task ID must match input",
            });
        }
        Ok(())
    }
}

impl Contract<ListTasksInput, TaskListResult> for TaskContracts {
    fn preconditions(input: &ListTasksInput) -> Result<(), ContractError> {
        if let Some(ref status) = input.status {
            Self::validate_status(status)?;
        }

        if let Some(ref priority) = input.priority {
            Self::validate_priority(priority)?;
        }

        if let Some(limit) = input.limit {
            Self::validate_limit(limit)?;
        }

        Ok(())
    }

    fn invariants(_input: &ListTasksInput) -> Vec<Invariant> {
        vec![]
    }

    fn postconditions(
        input: &ListTasksInput,
        result: &TaskListResult,
    ) -> Result<(), ContractError> {
        if let Some(limit) = input.limit {
            if result.tasks.len() > limit {
                return Err(ContractError::PostconditionFailed {
                    name: "limit_respected",
                    description: "Result count must not exceed limit",
                });
            }
        }
        if result.tasks.len() > result.total {
            return Err(ContractError::PostconditionFailed {
                name: "count_consistent",
                description: "Returned count must not exceed total",
            });
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_title_valid() {
        assert!(TaskContracts::validate_title("Valid title").is_ok());
        assert!(TaskContracts::validate_title("a").is_ok());
    }

    #[test]
    fn test_validate_title_empty() {
        assert!(TaskContracts::validate_title("").is_err());
        assert!(TaskContracts::validate_title("   ").is_err());
    }

    #[test]
    fn test_validate_title_too_long() {
        let long_title = "x".repeat(201);
        assert!(TaskContracts::validate_title(&long_title).is_err());
    }

    #[test]
    fn test_validate_priority_valid() {
        assert!(TaskContracts::validate_priority("P0").is_ok());
        assert!(TaskContracts::validate_priority("P1").is_ok());
        assert!(TaskContracts::validate_priority("P4").is_ok());
    }

    #[test]
    fn test_validate_priority_invalid() {
        assert!(TaskContracts::validate_priority("P5").is_err());
        assert!(TaskContracts::validate_priority("high").is_err());
    }

    #[test]
    fn test_validate_status_valid() {
        assert!(TaskContracts::validate_status("open").is_ok());
        assert!(TaskContracts::validate_status("in_progress").is_ok());
        assert!(TaskContracts::validate_status("blocked").is_ok());
        assert!(TaskContracts::validate_status("closed").is_ok());
    }

    #[test]
    fn test_validate_status_invalid() {
        assert!(TaskContracts::validate_status("pending").is_err());
        assert!(TaskContracts::validate_status("done").is_err());
    }

    #[test]
    fn test_validate_limit_valid() {
        assert!(TaskContracts::validate_limit(1).is_ok());
        assert!(TaskContracts::validate_limit(100).is_ok());
        assert!(TaskContracts::validate_limit(1000).is_ok());
    }

    #[test]
    fn test_validate_limit_invalid() {
        assert!(TaskContracts::validate_limit(0).is_err());
        assert!(TaskContracts::validate_limit(1001).is_err());
    }

    #[test]
    fn test_create_task_contract_preconditions() {
        let input = CreateTaskInput {
            title: "Test task".to_string(),
            description: None,
            priority: Some("P1".to_string()),
            task_type: None,
            labels: vec![],
        };
        assert!(TaskContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_create_task_contract_preconditions_fails() {
        let input = CreateTaskInput {
            title: String::new(),
            description: None,
            priority: Some("P99".to_string()),
            task_type: None,
            labels: vec![],
        };
        assert!(TaskContracts::preconditions(&input).is_err());
    }

    #[test]
    fn test_create_task_contract_postconditions() {
        let input = CreateTaskInput {
            title: "Test task".to_string(),
            description: None,
            priority: None,
            task_type: None,
            labels: vec![],
        };
        let result = TaskResult {
            id: "task-123".to_string(),
            title: "Test task".to_string(),
            status: "open".to_string(),
        };
        assert!(TaskContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_create_task_contract_postconditions_fails_wrong_status() {
        let input = CreateTaskInput {
            title: "Test task".to_string(),
            description: None,
            priority: None,
            task_type: None,
            labels: vec![],
        };
        let result = TaskResult {
            id: "task-123".to_string(),
            title: "Test task".to_string(),
            status: "closed".to_string(),
        };
        assert!(TaskContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_list_tasks_contract_postconditions() {
        let input = ListTasksInput {
            status: None,
            priority: None,
            label: None,
            limit: Some(10),
        };
        let result = TaskListResult {
            tasks: vec![TaskResult {
                id: "1".to_string(),
                title: "Task".to_string(),
                status: "open".to_string(),
            }],
            total: 5,
        };
        assert!(TaskContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_list_tasks_contract_postconditions_exceeds_limit() {
        let input = ListTasksInput {
            status: None,
            priority: None,
            label: None,
            limit: Some(1),
        };
        let result = TaskListResult {
            tasks: vec![
                TaskResult {
                    id: "1".to_string(),
                    title: "Task 1".to_string(),
                    status: "open".to_string(),
                },
                TaskResult {
                    id: "2".to_string(),
                    title: "Task 2".to_string(),
                    status: "open".to_string(),
                },
            ],
            total: 2,
        };
        assert!(TaskContracts::postconditions(&input, &result).is_err());
    }
}
