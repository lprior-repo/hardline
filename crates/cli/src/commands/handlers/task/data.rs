//! Data types for task command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use chrono::{DateTime, Utc};
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
