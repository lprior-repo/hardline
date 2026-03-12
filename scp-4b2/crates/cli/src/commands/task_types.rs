//! Task types for SCP CLI
//!
//! Domain types for task management: Task, TaskState

use chrono::{DateTime, Utc};

/// Task state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed { closed_at: DateTime<Utc> },
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(id: &str, title: &str) -> Self {
        let now = Utc::now();
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
