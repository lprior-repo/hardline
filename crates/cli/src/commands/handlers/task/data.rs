//! Data types for task command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use chrono::{DateTime, Utc};
pub use scp_core::cli_contracts::domain_types::TaskStatus;
use serde::{Deserialize, Serialize};

use crate::commands::task_types::TaskId;

/// Task command variants (CLI subcommand representation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskCommand {
    /// List tasks with optional status filter
    List {
        /// Filter by task status (optional)
        status_filter: Option<String>,
        /// Include all tasks regardless of status
        include_all: bool,
    },
    /// Show details for a specific task
    Show {
        /// Task/bead ID to display
        task_id: TaskId,
    },
    /// Claim a task for the given agent
    Claim {
        /// Task/bead ID to claim
        task_id: TaskId,
        /// Agent claiming the task
        agent_id: AgentId,
    },
    /// Release (yield) a claimed task
    YieldTask {
        /// Task/bead ID to yield
        task_id: TaskId,
        /// Agent yielding the task
        agent_id: AgentId,
    },
    /// Start work on a claimed task
    Start {
        /// Task/bead ID to start
        task_id: TaskId,
        /// Agent starting the task
        agent_id: AgentId,
    },
    /// Mark a task as done (completed)
    Done {
        /// Task/bead ID to complete (uses env var if None)
        task_id: Option<TaskId>,
        /// Agent completing the task
        agent_id: AgentId,
    },
}

/// Agent ID newtype - validates at construction time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentId(String);

impl AgentId {
    /// Create a new AgentId with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent ID is empty or whitespace-only.
    pub fn new(id: impl Into<String>) -> Result<Self, scp_core::error::Error> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(scp_core::error_task::TaskErrorKind::InvalidId(
                "Agent ID cannot be empty".to_string(),
            )
            .into());
        }
        Ok(Self(id))
    }

    /// Access the inner string as a str slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Task status for display output (mirrors TaskState but serializable for JSON).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatusOutput {
    /// Task is open and unassigned
    Open,
    /// Task is in progress
    InProgress,
    /// Task is blocked
    Blocked,
    /// Task has been deferred
    Deferred,
    /// Task has been completed
    Closed,
}

impl std::fmt::Display for TaskStatusOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Blocked => write!(f, "blocked"),
            Self::Deferred => write!(f, "deferred"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

/// Single task information for display output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfoOutput {
    /// Unique task identifier
    pub id: String,
    /// Task title
    pub title: String,
    /// Current status
    pub status: TaskStatusOutput,
    /// Task description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Agent assigned to this task (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    /// Task priority (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    /// When the task was created
    pub created_at: DateTime<Utc>,
    /// When the task was last updated
    pub updated_at: DateTime<Utc>,
}

/// Result of a task list operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListOutput {
    /// List of task info outputs
    pub tasks: Vec<TaskInfoOutput>,
    /// Total count of tasks
    pub total: usize,
}

/// Result of a task claim operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskClaimOutput {
    /// Whether the claim succeeded
    pub claimed: bool,
    /// Task ID
    pub task_id: String,
    /// Agent that now holds the claim
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    /// Error message if claim failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result of a task yield operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskYieldOutput {
    /// Whether the yield succeeded
    pub yielded: bool,
    /// Task ID
    pub task_id: String,
    /// Error message if yield failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result of a task done operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDoneOutput {
    /// Task ID
    pub task_id: String,
    /// Task title
    pub title: String,
    /// Final status
    pub status: TaskStatusOutput,
}

/// Result of a task start operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStartOutput {
    /// Task ID
    pub task_id: String,
    /// Status indicator
    pub status: TaskStatusOutput,
    /// Workspace path
    pub workspace: String,
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::commands::task_types::{TaskId, TaskState};

    // ---- TaskCommand equality ----

    #[test]
    fn task_command_list_equality() {
        let a = TaskCommand::List {
            status_filter: Some("open".to_string()),
            include_all: false,
        };
        let b = TaskCommand::List {
            status_filter: Some("open".to_string()),
            include_all: false,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn task_command_list_inequality_different_filter() {
        let a = TaskCommand::List {
            status_filter: Some("open".to_string()),
            include_all: false,
        };
        let b = TaskCommand::List {
            status_filter: Some("closed".to_string()),
            include_all: false,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn task_command_different_variant_inequality() {
        assert_ne!(
            TaskCommand::List {
                status_filter: None,
                include_all: false,
            },
            TaskCommand::List {
                status_filter: None,
                include_all: true,
            }
        );
    }

    // ---- AgentId validation ----

    #[test]
    fn agent_id_valid() {
        let agent = AgentId::new("my-agent").expect("valid agent id");
        assert_eq!(agent.as_str(), "my-agent");
    }

    #[test]
    fn agent_id_empty_rejected() {
        let result = AgentId::new("");
        assert!(result.is_err());
    }

    #[test]
    fn agent_id_whitespace_only_rejected() {
        let result = AgentId::new("   ");
        assert!(result.is_err());
    }

    #[test]
    fn agent_id_with_inner_whitespace_trimmed_still_valid() {
        // AgentId only trims for validation, stores the original string
        let agent = AgentId::new(" agent-1 ").expect("valid after trim");
        assert_eq!(agent.as_str(), " agent-1 ");
    }

    #[test]
    fn agent_id_display() {
        let agent = AgentId::new("test-agent").expect("valid");
        assert_eq!(format!("{agent}"), "test-agent");
    }

    #[test]
    fn agent_id_numeric() {
        let agent = AgentId::new("agent-42").expect("valid");
        assert_eq!(agent.as_str(), "agent-42");
    }

    // ---- TaskStatusOutput display ----

    #[test]
    fn task_status_output_display_open() {
        assert_eq!(format!("{}", TaskStatusOutput::Open), "open");
    }

    #[test]
    fn task_status_output_display_in_progress() {
        assert_eq!(format!("{}", TaskStatusOutput::InProgress), "in_progress");
    }

    #[test]
    fn task_status_output_display_blocked() {
        assert_eq!(format!("{}", TaskStatusOutput::Blocked), "blocked");
    }

    #[test]
    fn task_status_output_display_deferred() {
        assert_eq!(format!("{}", TaskStatusOutput::Deferred), "deferred");
    }

    #[test]
    fn task_status_output_display_closed() {
        assert_eq!(format!("{}", TaskStatusOutput::Closed), "closed");
    }

    // ---- TaskStatusOutput serialization ----

    #[test]
    fn task_status_output_serializes_to_snake_case() {
        let json = serde_json::to_string(&TaskStatusOutput::InProgress).expect("serialize");
        assert_eq!(json, "\"in_progress\"");
    }

    // ---- TaskInfoOutput serialization ----

    #[test]
    fn task_info_output_serialization_roundtrip() {
        let info = TaskInfoOutput {
            id: "hl-0g4".to_string(),
            title: "Test task".to_string(),
            status: TaskStatusOutput::Open,
            description: Some("A description".to_string()),
            assignee: Some("agent-1".to_string()),
            priority: Some("high".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let deserialized: TaskInfoOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.id, "hl-0g4");
        assert_eq!(deserialized.title, "Test task");
        assert_eq!(deserialized.status, TaskStatusOutput::Open);
        assert_eq!(deserialized.assignee.as_deref(), Some("agent-1"));
    }

    #[test]
    fn task_info_output_skip_serializing_none() {
        let info = TaskInfoOutput {
            id: "hl-xxx".to_string(),
            title: "Minimal".to_string(),
            status: TaskStatusOutput::Closed,
            description: None,
            assignee: None,
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(!json.contains("description"));
        assert!(!json.contains("assignee"));
        assert!(!json.contains("priority"));
    }

    // ---- TaskListOutput ----

    #[test]
    fn task_list_output_construction() {
        let output = TaskListOutput {
            tasks: vec![],
            total: 0,
        };
        assert!(output.tasks.is_empty());
        assert_eq!(output.total, 0);
    }

    // ---- TaskClaimOutput ----

    #[test]
    fn task_claim_output_serialization_roundtrip() {
        let output = TaskClaimOutput {
            claimed: true,
            task_id: "hl-001".to_string(),
            holder: Some("agent-1".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: TaskClaimOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert!(deserialized.claimed);
        assert!(deserialized.error.is_none());
    }

    #[test]
    fn task_claim_output_failure_serialization() {
        let output = TaskClaimOutput {
            claimed: false,
            task_id: "hl-001".to_string(),
            holder: None,
            error: Some("already claimed by other agent".to_string()),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: TaskClaimOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert!(!deserialized.claimed);
        assert_eq!(
            deserialized.error.as_deref(),
            Some("already claimed by other agent")
        );
    }

    // ---- TaskYieldOutput ----

    #[test]
    fn task_yield_output_serialization_roundtrip() {
        let output = TaskYieldOutput {
            yielded: true,
            task_id: "hl-001".to_string(),
            error: None,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: TaskYieldOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert!(deserialized.yielded);
    }

    // ---- TaskDoneOutput ----

    #[test]
    fn task_done_output_serialization_roundtrip() {
        let output = TaskDoneOutput {
            task_id: "hl-001".to_string(),
            title: "Completed task".to_string(),
            status: TaskStatusOutput::Closed,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: TaskDoneOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.task_id, "hl-001");
        assert_eq!(deserialized.status, TaskStatusOutput::Closed);
    }

    // ---- TaskStartOutput ----

    #[test]
    fn task_start_output_serialization_roundtrip() {
        let output = TaskStartOutput {
            task_id: "hl-001".to_string(),
            status: TaskStatusOutput::InProgress,
            workspace: ".scp/workspaces/hl-001".to_string(),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: TaskStartOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.workspace, ".scp/workspaces/hl-001");
    }
}
