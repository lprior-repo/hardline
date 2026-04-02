//! Task types for SCP CLI
//!
//! Domain types for task management: Task, TaskState, TaskId, Title, Priority, Assignee

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use scp_core::{error::Error, error_task::TaskErrorKind};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Regex pattern for valid task IDs: alphanumeric with - or _
static TASK_ID_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_-]+$").expect("Invalid regex pattern"));

/// Task state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed { closed_at: DateTime<Utc> },
}

/// Task ID - newtype for type safety with validation at construction
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    /// Create a new TaskId with validation at parse time
    pub fn new(id: impl Into<String>) -> Result<Self, Error> {
        let id = id.into();
        if id.is_empty() {
            return Err(TaskErrorKind::InvalidId("Task ID cannot be empty".to_string()).into());
        }
        if !TASK_ID_PATTERN.is_match(&id) {
            return Err(TaskErrorKind::InvalidId(format!(
                "Task ID must be alphanumeric with - or _, got: {}",
                id
            ))
            .into());
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- TaskId validation ----

    #[test]
    fn task_id_empty_rejected() {
        let result = TaskId::new("");
        assert!(result.is_err());
    }

    #[test]
    fn task_id_whitespace_rejected() {
        let result = TaskId::new("task 123");
        assert!(result.is_err());
    }

    #[test]
    fn task_id_special_chars_rejected() {
        let result = TaskId::new("task@123!");
        assert!(result.is_err());
    }

    #[test]
    fn task_id_with_dots_rejected() {
        let result = TaskId::new("task.123");
        assert!(result.is_err());
    }

    #[test]
    fn task_id_with_slashes_rejected() {
        let result = TaskId::new("task/123");
        assert!(result.is_err());
    }

    #[test]
    fn task_id_with_colons_rejected() {
        let result = TaskId::new("task:123");
        assert!(result.is_err());
    }

    #[test]
    fn task_id_valid_simple() {
        let result = TaskId::new("task-001");
        assert!(result.is_ok());
        assert_eq!(result.expect("ok").as_str(), "task-001");
    }

    #[test]
    fn task_id_valid_underscores() {
        let result = TaskId::new("bead_123");
        assert!(result.is_ok());
        assert_eq!(result.expect("ok").as_str(), "bead_123");
    }

    #[test]
    fn task_id_valid_mixed() {
        let result = TaskId::new("ABC-123_xyz");
        assert!(result.is_ok());
    }

    #[test]
    fn task_id_valid_single_char() {
        let result = TaskId::new("a");
        assert!(result.is_ok());
    }

    #[test]
    fn task_id_valid_numeric_only() {
        let result = TaskId::new("12345");
        assert!(result.is_ok());
    }

    #[test]
    fn task_id_display() {
        let id = TaskId::new("task-001").expect("valid");
        assert_eq!(format!("{id}"), "task-001");
    }

    #[test]
    fn task_id_equality() {
        let a = TaskId::new("x").expect("ok");
        let b = TaskId::new("x").expect("ok");
        let c = TaskId::new("y").expect("ok");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn task_id_serialization_roundtrip() {
        let id = TaskId::new("task-99").expect("valid");
        let json = serde_json::to_string(&id).expect("serialize");
        let deserialized: TaskId =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.as_str(), "task-99");
    }

    // ---- Title ----

    #[test]
    fn title_new_construction() {
        let title = Title::new("My task title");
        assert_eq!(title.as_str(), "My task title");
    }

    #[test]
    fn title_empty_allowed() {
        let title = Title::new("");
        assert_eq!(title.as_str(), "");
    }

    #[test]
    fn title_with_special_chars() {
        let title = Title::new("Fix bug: crash on startup!");
        assert_eq!(title.as_str(), "Fix bug: crash on startup!");
    }

    #[test]
    fn title_display() {
        let title = Title::new("hello");
        assert_eq!(format!("{title}"), "hello");
    }

    #[test]
    fn title_serialization_roundtrip() {
        let title = Title::new("test title");
        let json = serde_json::to_string(&title).expect("serialize");
        let deserialized: Title =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.as_str(), "test title");
    }

    // ---- Priority ----

    #[test]
    fn priority_new_construction() {
        let p = Priority::new("high");
        assert_eq!(p.as_str(), "high");
    }

    #[test]
    fn priority_display() {
        let p = Priority::new("critical");
        assert_eq!(format!("{p}"), "critical");
    }

    #[test]
    fn priority_serialization_roundtrip() {
        let p = Priority::new("low");
        let json = serde_json::to_string(&p).expect("serialize");
        let deserialized: Priority =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.as_str(), "low");
    }

    // ---- Assignee ----

    #[test]
    fn assignee_new_construction() {
        let a = Assignee::new("alice");
        assert_eq!(a.as_str(), "alice");
    }

    #[test]
    fn assignee_empty_allowed() {
        let a = Assignee::new("");
        assert_eq!(a.as_str(), "");
    }

    #[test]
    fn assignee_display() {
        let a = Assignee::new("bob");
        assert_eq!(format!("{a}"), "bob");
    }

    #[test]
    fn assignee_serialization_roundtrip() {
        let a = Assignee::new("charlie");
        let json = serde_json::to_string(&a).expect("serialize");
        let deserialized: Assignee =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.as_str(), "charlie");
    }

    // ---- TaskState ----

    #[test]
    fn task_state_open_equality() {
        assert_eq!(TaskState::Open, TaskState::Open);
    }

    #[test]
    fn task_state_in_progress_equality() {
        assert_eq!(TaskState::InProgress, TaskState::InProgress);
    }

    #[test]
    fn task_state_blocked_equality() {
        assert_eq!(TaskState::Blocked, TaskState::Blocked);
    }

    #[test]
    fn task_state_deferred_equality() {
        assert_eq!(TaskState::Deferred, TaskState::Deferred);
    }

    #[test]
    fn task_state_closed_with_timestamp_equality() {
        let now = Utc::now();
        assert_eq!(
            TaskState::Closed { closed_at: now },
            TaskState::Closed { closed_at: now }
        );
    }

    #[test]
    fn task_state_closed_different_timestamps_inequality() {
        let t1 = Utc::now();
        let t2 = t1 + chrono::Duration::seconds(1);
        assert_ne!(
            TaskState::Closed { closed_at: t1 },
            TaskState::Closed { closed_at: t2 }
        );
    }

    #[test]
    fn task_state_serialization_roundtrip_open() {
        let state = TaskState::Open;
        let json = serde_json::to_string(&state).expect("serialize");
        let deserialized: TaskState =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized, TaskState::Open);
    }

    #[test]
    fn task_state_serialization_roundtrip_closed() {
        let state = TaskState::Closed {
            closed_at: Utc::now(),
        };
        let json = serde_json::to_string(&state).expect("serialize");
        let deserialized: TaskState =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert!(matches!(deserialized, TaskState::Closed { .. }));
    }

    // ---- Task ----

    #[test]
    fn task_new_defaults() {
        let before = Utc::now();
        let task = Task::new(
            TaskId::new("t-1").expect("valid"),
            Title::new("Test"),
        );
        let after = Utc::now();

        assert_eq!(task.id.as_str(), "t-1");
        assert_eq!(task.title.as_str(), "Test");
        assert!(task.description.is_none());
        assert!(matches!(task.state, TaskState::Open));
        assert!(task.priority.is_none());
        assert!(task.assignee.is_none());
        assert!(task.created_at >= before && task.created_at <= after);
        assert!(task.updated_at >= before && task.updated_at <= after);
    }

    #[test]
    fn task_serialization_roundtrip() {
        let task = Task::new(
            TaskId::new("t-1").expect("valid"),
            Title::new("Test task"),
        );
        let json = serde_json::to_string(&task).expect("serialize");
        let deserialized: Task =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.id.as_str(), "t-1");
        assert_eq!(deserialized.title.as_str(), "Test task");
        assert!(deserialized.description.is_none());
    }
}
