//! Database Schema Definitions for ADR-006
//!
//! SQLite schema with WAL durability for:
//! - Workspace state tracking
//! - Operation journal for durable execution
//! - Queue entries for merge queue
//! - Agent registry
//! - Sessions
//! - Configuration

use serde::{Deserialize, Serialize};

/// Schema version for migrations
pub const SCHEMA_VERSION: u32 = 1;

/// SQL for schema_version table
pub const SCHEMA_VERSION_TABLE: &str = r#"
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL,
    description TEXT NOT NULL
);

INSERT INTO schema_version (version, applied_at, description)
VALUES (1, datetime('now'), 'Initial schema');
"#;

/// SQL for workspaces table
pub const WORKSPACES_TABLE: &str = r#"
CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    backend TEXT NOT NULL CHECK (backend IN ('git', 'jj')),
    state TEXT NOT NULL CHECK (state IN (
        'created', 'active', 'syncing', 'paused', 'completed', 'failed'
    )),
    agent_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_accessed_at TEXT NOT NULL,
    
    UNIQUE(name),
    INDEX idx_workspaces_state (state),
    INDEX idx_workspaces_agent (agent_id)
);

CREATE TRIGGER workspaces_updated_at
AFTER UPDATE ON workspaces
BEGIN
    UPDATE workspaces SET updated_at = datetime('now') WHERE id = NEW.id;
END;
"#;

/// SQL for operations table (durable execution journal)
pub const OPERATIONS_TABLE: &str = r#"
CREATE TABLE operations (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN (
        'started', 'in_progress', 'completed', 'failed'
    )),
    current_step INTEGER NOT NULL DEFAULT 0,
    total_steps INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    final_revision INTEGER,
    error_message TEXT,
    author_id TEXT NOT NULL,
    description TEXT NOT NULL,
    
    INDEX idx_operations_workspace (workspace_id),
    INDEX idx_operations_state (state),
    INDEX idx_operations_started (started_at)
);
"#;

/// SQL for operation_steps table
pub const OPERATION_STEPS_TABLE: &str = r#"
CREATE TABLE operation_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL REFERENCES operations(id),
    step_index INTEGER NOT NULL,
    step_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'pending', 'running', 'completed', 'failed', 'skipped'
    )),
    event_revision INTEGER,
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    
    UNIQUE(operation_id, step_index),
    INDEX idx_steps_operation (operation_id)
);
"#;

/// SQL for queue_entries table (merge queue)
pub const QUEUE_ENTRIES_TABLE: &str = r#"
CREATE TABLE queue_entries (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    priority INTEGER NOT NULL CHECK (priority >= 0 AND priority <= 255),
    status TEXT NOT NULL CHECK (status IN (
        'pending', 'claimed', 'rebase', 'testing', 'ready_to_merge',
        'merging', 'merged', 'failed_retryable', 'failed_terminal', 'cancelled'
    )),
    enqueued_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    claimed_by TEXT,
    claimed_at TEXT,
    position INTEGER NOT NULL,
    
    INDEX idx_queue_priority (priority ASC, enqueued_at ASC),
    INDEX idx_queue_status (status),
    INDEX idx_queue_claimed (claimed_by)
);

CREATE TRIGGER queue_updated_at
AFTER UPDATE ON queue_entries
BEGIN
    UPDATE queue_entries SET updated_at = datetime('now') WHERE id = NEW.id;
END;
"#;

/// SQL for agents table
pub const AGENTS_TABLE: &str = r#"
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    capabilities TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'idle', 'disconnected')),
    last_heartbeat_at TEXT NOT NULL,
    registered_at TEXT NOT NULL,
    metadata TEXT,
    
    INDEX idx_agents_status (status),
    INDEX idx_agents_heartbeat (last_heartbeat_at)
);
"#;

/// SQL for sessions table
pub const SESSIONS_TABLE: &str = r#"
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN (
        'created', 'active', 'syncing', 'synced', 'paused', 'completed', 'failed'
    )),
    bead_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    
    UNIQUE(workspace_id, name),
    INDEX idx_sessions_workspace (workspace_id),
    INDEX idx_sessions_bead (bead_id),
    INDEX idx_sessions_state (state)
);
"#;

/// SQL for config table
pub const CONFIG_TABLE: &str = r#"
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    description TEXT
);

CREATE TRIGGER config_updated_at
AFTER UPDATE ON config
BEGIN
    UPDATE config SET updated_at = datetime('now') WHERE key = NEW.key;
END;
"#;

pub fn initial_schema() -> String {
    [
        SCHEMA_VERSION_TABLE,
        WORKSPACES_TABLE,
        OPERATIONS_TABLE,
        OPERATION_STEPS_TABLE,
        QUEUE_ENTRIES_TABLE,
        AGENTS_TABLE,
        SESSIONS_TABLE,
        CONFIG_TABLE,
    ]
    .join("\n")
}

/// Workspace backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceBackend {
    Git,
    Jj,
}

impl WorkspaceBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkspaceBackend::Git => "git",
            WorkspaceBackend::Jj => "jj",
        }
    }
}

/// Workspace state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceState {
    Created,
    Active,
    Syncing,
    Paused,
    Completed,
    Failed,
}

impl WorkspaceState {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkspaceState::Created => "created",
            WorkspaceState::Active => "active",
            WorkspaceState::Syncing => "syncing",
            WorkspaceState::Paused => "paused",
            WorkspaceState::Completed => "completed",
            WorkspaceState::Failed => "failed",
        }
    }
}

/// Operation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationState {
    Started,
    InProgress,
    Completed,
    Failed,
}

impl OperationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationState::Started => "started",
            OperationState::InProgress => "in_progress",
            OperationState::Completed => "completed",
            OperationState::Failed => "failed",
        }
    }
}

/// Operation step status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StepStatus::Pending => "pending",
            StepStatus::Running => "running",
            StepStatus::Completed => "completed",
            StepStatus::Failed => "failed",
            StepStatus::Skipped => "skipped",
        }
    }
}

/// Queue entry status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    Pending,
    Claimed,
    Rebase,
    Testing,
    ReadyToMerge,
    Merging,
    Merged,
    FailedRetryable,
    FailedTerminal,
    Cancelled,
}

impl QueueStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueueStatus::Pending => "pending",
            QueueStatus::Claimed => "claimed",
            QueueStatus::Rebase => "rebase",
            QueueStatus::Testing => "testing",
            QueueStatus::ReadyToMerge => "ready_to_merge",
            QueueStatus::Merging => "merging",
            QueueStatus::Merged => "merged",
            QueueStatus::FailedRetryable => "failed_retryable",
            QueueStatus::FailedTerminal => "failed_terminal",
            QueueStatus::Cancelled => "cancelled",
        }
    }
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Active,
    Idle,
    Disconnected,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Active => "active",
            AgentStatus::Idle => "idle",
            AgentStatus::Disconnected => "disconnected",
        }
    }
}

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Created,
    Active,
    Syncing,
    Synced,
    Paused,
    Completed,
    Failed,
}

impl SessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionState::Created => "created",
            SessionState::Active => "active",
            SessionState::Syncing => "syncing",
            SessionState::Synced => "synced",
            SessionState::Paused => "paused",
            SessionState::Completed => "completed",
            SessionState::Failed => "failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_backend_as_str() {
        assert_eq!(WorkspaceBackend::Git.as_str(), "git");
        assert_eq!(WorkspaceBackend::Jj.as_str(), "jj");
    }

    #[test]
    fn test_workspace_state_as_str() {
        assert_eq!(WorkspaceState::Created.as_str(), "created");
        assert_eq!(WorkspaceState::Active.as_str(), "active");
        assert_eq!(WorkspaceState::Syncing.as_str(), "syncing");
        assert_eq!(WorkspaceState::Paused.as_str(), "paused");
        assert_eq!(WorkspaceState::Completed.as_str(), "completed");
        assert_eq!(WorkspaceState::Failed.as_str(), "failed");
    }

    #[test]
    fn test_operation_state_as_str() {
        assert_eq!(OperationState::Started.as_str(), "started");
        assert_eq!(OperationState::InProgress.as_str(), "in_progress");
        assert_eq!(OperationState::Completed.as_str(), "completed");
        assert_eq!(OperationState::Failed.as_str(), "failed");
    }

    #[test]
    fn test_step_status_as_str() {
        assert_eq!(StepStatus::Pending.as_str(), "pending");
        assert_eq!(StepStatus::Running.as_str(), "running");
        assert_eq!(StepStatus::Completed.as_str(), "completed");
        assert_eq!(StepStatus::Failed.as_str(), "failed");
        assert_eq!(StepStatus::Skipped.as_str(), "skipped");
    }

    #[test]
    fn test_queue_status_as_str() {
        assert_eq!(QueueStatus::Pending.as_str(), "pending");
        assert_eq!(QueueStatus::Claimed.as_str(), "claimed");
        assert_eq!(QueueStatus::Merged.as_str(), "merged");
        assert_eq!(QueueStatus::FailedTerminal.as_str(), "failed_terminal");
    }

    #[test]
    fn test_agent_status_as_str() {
        assert_eq!(AgentStatus::Active.as_str(), "active");
        assert_eq!(AgentStatus::Idle.as_str(), "idle");
        assert_eq!(AgentStatus::Disconnected.as_str(), "disconnected");
    }

    #[test]
    fn test_session_state_as_str() {
        assert_eq!(SessionState::Created.as_str(), "created");
        assert_eq!(SessionState::Active.as_str(), "active");
        assert_eq!(SessionState::Synced.as_str(), "synced");
        assert_eq!(SessionState::Failed.as_str(), "failed");
    }

    #[test]
    fn test_schema_version() {
        assert_eq!(SCHEMA_VERSION, 1);
    }

    // =========================================================================
    // initial_schema() function
    // =========================================================================

    #[test]
    fn given_initial_schema_when_called_then_contains_all_tables() {
        let schema = initial_schema();
        assert!(schema.contains("CREATE TABLE schema_version"));
        assert!(schema.contains("CREATE TABLE workspaces"));
        assert!(schema.contains("CREATE TABLE operations"));
        assert!(schema.contains("CREATE TABLE operation_steps"));
        assert!(schema.contains("CREATE TABLE queue_entries"));
        assert!(schema.contains("CREATE TABLE agents"));
        assert!(schema.contains("CREATE TABLE sessions"));
        assert!(schema.contains("CREATE TABLE config"));
    }

    #[test]
    fn given_initial_schema_when_called_then_contains_triggers() {
        let schema = initial_schema();
        assert!(schema.contains("workspaces_updated_at"));
        assert!(schema.contains("queue_updated_at"));
        assert!(schema.contains("config_updated_at"));
    }

    #[test]
    fn given_initial_schema_when_called_then_contains_indexes() {
        let schema = initial_schema();
        assert!(schema.contains("idx_workspaces_state"));
        assert!(schema.contains("idx_operations_workspace"));
        assert!(schema.contains("idx_queue_priority"));
        assert!(schema.contains("idx_agents_status"));
        assert!(schema.contains("idx_sessions_workspace"));
    }

    #[test]
    fn given_initial_schema_when_called_then_not_empty() {
        let schema = initial_schema();
        assert!(!schema.is_empty());
        assert!(schema.len() > 100);
    }

    // =========================================================================
    // Serialization roundtrips for enums
    // =========================================================================

    #[test]
    fn given_workspace_backend_when_serialized_then_roundtrips() {
        for variant in [WorkspaceBackend::Git, WorkspaceBackend::Jj] {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: WorkspaceBackend = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn given_workspace_backend_when_serialized_then_lowercase() {
        assert_eq!(
            serde_json::to_string(&WorkspaceBackend::Git).unwrap(),
            "\"git\""
        );
        assert_eq!(
            serde_json::to_string(&WorkspaceBackend::Jj).unwrap(),
            "\"jj\""
        );
    }

    #[test]
    fn given_workspace_state_when_serialized_then_roundtrips() {
        for variant in [
            WorkspaceState::Created,
            WorkspaceState::Active,
            WorkspaceState::Syncing,
            WorkspaceState::Paused,
            WorkspaceState::Completed,
            WorkspaceState::Failed,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: WorkspaceState = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn given_operation_state_when_serialized_then_roundtrips() {
        for variant in [
            OperationState::Started,
            OperationState::InProgress,
            OperationState::Completed,
            OperationState::Failed,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: OperationState = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn given_operation_state_in_progress_when_serialized_then_snake_case() {
        let json = serde_json::to_string(&OperationState::InProgress).unwrap();
        assert_eq!(json, "\"in_progress\"");
    }

    #[test]
    fn given_step_status_when_serialized_then_roundtrips() {
        for variant in [
            StepStatus::Pending,
            StepStatus::Running,
            StepStatus::Completed,
            StepStatus::Failed,
            StepStatus::Skipped,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: StepStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn given_queue_status_when_serialized_then_roundtrips() {
        for variant in [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebase,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::Merged,
            QueueStatus::FailedRetryable,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: QueueStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn given_queue_status_ready_to_merge_when_serialized_then_snake_case() {
        let json = serde_json::to_string(&QueueStatus::ReadyToMerge).unwrap();
        assert_eq!(json, "\"ready_to_merge\"");
    }

    #[test]
    fn given_queue_status_failed_retryable_when_serialized_then_snake_case() {
        let json = serde_json::to_string(&QueueStatus::FailedRetryable).unwrap();
        assert_eq!(json, "\"failed_retryable\"");
    }

    #[test]
    fn given_agent_status_when_serialized_then_roundtrips() {
        for variant in [
            AgentStatus::Active,
            AgentStatus::Idle,
            AgentStatus::Disconnected,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: AgentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn given_session_state_when_serialized_then_roundtrips() {
        for variant in [
            SessionState::Created,
            SessionState::Active,
            SessionState::Syncing,
            SessionState::Synced,
            SessionState::Paused,
            SessionState::Completed,
            SessionState::Failed,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: SessionState = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    // =========================================================================
    // Debug formatting for enums
    // =========================================================================

    #[test]
    fn given_workspace_backend_when_debug_then_contains_variant() {
        assert!(format!("{:?}", WorkspaceBackend::Git).contains("Git"));
        assert!(format!("{:?}", WorkspaceBackend::Jj).contains("Jj"));
    }

    #[test]
    fn given_workspace_state_when_debug_then_contains_variant() {
        assert!(format!("{:?}", WorkspaceState::Active).contains("Active"));
    }

    #[test]
    fn given_operation_state_when_debug_then_contains_variant() {
        assert!(format!("{:?}", OperationState::InProgress).contains("InProgress"));
    }

    #[test]
    fn given_step_status_when_debug_then_contains_variant() {
        assert!(format!("{:?}", StepStatus::Pending).contains("Pending"));
    }

    #[test]
    fn given_queue_status_when_debug_then_contains_variant() {
        assert!(format!("{:?}", QueueStatus::Merged).contains("Merged"));
    }

    #[test]
    fn given_agent_status_when_debug_then_contains_variant() {
        assert!(format!("{:?}", AgentStatus::Disconnected).contains("Disconnected"));
    }

    #[test]
    fn given_session_state_when_debug_then_contains_variant() {
        assert!(format!("{:?}", SessionState::Synced).contains("Synced"));
    }

    // =========================================================================
    // as_str() exhaustiveness - ensure all variants return non-empty
    // =========================================================================

    #[test]
    fn given_all_workspace_backends_when_as_str_then_non_empty() {
        for variant in [WorkspaceBackend::Git, WorkspaceBackend::Jj] {
            assert!(!variant.as_str().is_empty());
        }
    }

    #[test]
    fn given_all_workspace_states_when_as_str_then_non_empty() {
        for variant in [
            WorkspaceState::Created,
            WorkspaceState::Active,
            WorkspaceState::Syncing,
            WorkspaceState::Paused,
            WorkspaceState::Completed,
            WorkspaceState::Failed,
        ] {
            assert!(!variant.as_str().is_empty());
        }
    }

    #[test]
    fn given_all_operation_states_when_as_str_then_non_empty() {
        for variant in [
            OperationState::Started,
            OperationState::InProgress,
            OperationState::Completed,
            OperationState::Failed,
        ] {
            assert!(!variant.as_str().is_empty());
        }
    }

    #[test]
    fn given_all_step_statuses_when_as_str_then_non_empty() {
        for variant in [
            StepStatus::Pending,
            StepStatus::Running,
            StepStatus::Completed,
            StepStatus::Failed,
            StepStatus::Skipped,
        ] {
            assert!(!variant.as_str().is_empty());
        }
    }

    #[test]
    fn given_all_queue_statuses_when_as_str_then_non_empty() {
        for variant in [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebase,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::Merged,
            QueueStatus::FailedRetryable,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ] {
            assert!(!variant.as_str().is_empty());
        }
    }

    #[test]
    fn given_all_agent_statuses_when_as_str_then_non_empty() {
        for variant in [
            AgentStatus::Active,
            AgentStatus::Idle,
            AgentStatus::Disconnected,
        ] {
            assert!(!variant.as_str().is_empty());
        }
    }

    #[test]
    fn given_all_session_states_when_as_str_then_non_empty() {
        for variant in [
            SessionState::Created,
            SessionState::Active,
            SessionState::Syncing,
            SessionState::Synced,
            SessionState::Paused,
            SessionState::Completed,
            SessionState::Failed,
        ] {
            assert!(!variant.as_str().is_empty());
        }
    }

    // =========================================================================
    // Copy trait for all enums (they derive Copy)
    // =========================================================================

    #[test]
    fn given_workspace_backend_when_copied_then_same() {
        let a = WorkspaceBackend::Git;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn given_queue_status_when_copied_then_same() {
        let a = QueueStatus::Merged;
        let b = a;
        assert_eq!(a, b);
    }

    // =========================================================================
    // Schema SQL constants - smoke checks
    // =========================================================================

    #[test]
    fn given_schema_version_table_when_sql_then_valid() {
        assert!(SCHEMA_VERSION_TABLE.contains("CREATE TABLE schema_version"));
        assert!(SCHEMA_VERSION_TABLE.contains("INSERT INTO schema_version"));
    }

    #[test]
    fn given_workspaces_table_when_sql_then_has_constraints() {
        assert!(WORKSPACES_TABLE.contains("UNIQUE(name)"));
        assert!(WORKSPACES_TABLE.contains("CHECK (backend IN"));
        assert!(WORKSPACES_TABLE.contains("REFERENCES"));
    }

    #[test]
    fn given_queue_entries_table_when_sql_then_has_constraints() {
        assert!(QUEUE_ENTRIES_TABLE.contains("CHECK (priority >= 0 AND priority <= 255)"));
        assert!(QUEUE_ENTRIES_TABLE.contains("CHECK (status IN"));
    }

    // =========================================================================
    // Eq / PartialEq for enums
    // =========================================================================

    #[test]
    fn given_same_enum_variant_then_equal() {
        assert_eq!(WorkspaceBackend::Git, WorkspaceBackend::Git);
        assert_eq!(QueueStatus::Pending, QueueStatus::Pending);
        assert_eq!(AgentStatus::Idle, AgentStatus::Idle);
    }

    #[test]
    fn given_different_enum_variant_then_not_equal() {
        assert_ne!(WorkspaceBackend::Git, WorkspaceBackend::Jj);
        assert_ne!(QueueStatus::Pending, QueueStatus::Merged);
        assert_ne!(AgentStatus::Active, AgentStatus::Idle);
    }
}
