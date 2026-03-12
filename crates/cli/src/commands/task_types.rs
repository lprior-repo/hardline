//! Task types for SCP CLI
//!
//! Domain types for task management: Task, TaskState, TaskId, Title, Priority, Assignee

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use scp_core::error::Error;
use std::fmt;

/// Regex pattern for valid task IDs: alphanumeric with - or _
static TASK_ID_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_-]+$").expect("Invalid regex pattern"));

/// Task state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed { closed_at: DateTime<Utc> },
}

/// Task ID - newtype for type safety with validation at construction
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(String);

impl TaskId {
    /// Create a new TaskId with validation at parse time
    pub fn new(id: impl Into<String>) -> Result<Self, Error> {
        let id = id.into();
        if id.is_empty() {
            return Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()));
        }
        if !TASK_ID_PATTERN.is_match(&id) {
            return Err(Error::InvalidTaskId(format!(
                "Task ID must be alphanumeric with - or _, got: {}",
                id
            )));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Title - newtype for task titles
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title(String);

impl Title {
    pub fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Priority - newtype for task priority
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Priority(String);

impl Priority {
    pub fn new(priority: impl Into<String>) -> Self {
        Self(priority.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Assignee - newtype for task assignee
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignee(String);

impl Assignee {
    pub fn new(assignee: impl Into<String>) -> Self {
        Self(assignee.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Assignee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Task representation
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub title: Title,
    pub description: Option<String>,
    pub state: TaskState,
    pub priority: Option<Priority>,
    pub assignee: Option<Assignee>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(id: impl Into<TaskId>, title: impl Into<Title>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            state: TaskState::Open,
            priority: None,
            assignee: None,
            created_at: now,
            updated_at: now,
        }
    }
}
