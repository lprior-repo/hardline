//! Data types for the query command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

/// Options for the query command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct QueryOptions {
    /// Type of query to execute.
    pub query_type: QueryType,
    /// Filter argument (e.g., session name).
    pub argument: Option<String>,
    /// Filter by status.
    pub status_filter: Option<String>,
    /// Filter by agent.
    pub agent_filter: Option<String>,
}

/// Types of queries supported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryType {
    /// Check if a session exists.
    SessionExists,
    /// List sessions with optional filters.
    Sessions,
    /// Get detailed session info.
    SessionInfo,
    /// Show blocked sessions.
    Blockers,
    /// Count sessions.
    SessionCount,
    /// List available commands.
    Help,
}

impl QueryType {
    /// Parse query type from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "session-exists" => Some(Self::SessionExists),
            "sessions" => Some(Self::Sessions),
            "session-info" => Some(Self::SessionInfo),
            "blockers" => Some(Self::Blockers),
            "session-count" => Some(Self::SessionCount),
            "help" | "list" => Some(Self::Help),
            _ => None,
        }
    }

    /// Get all query type names.
    pub fn all_names() -> &'static [&'static str] {
        &[
            "session-exists",
            "sessions",
            "session-info",
            "blockers",
            "session-count",
            "help",
        ]
    }
}

/// Output from the query command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOutput {
    /// Whether the query succeeded.
    pub success: bool,
    /// Query type that was executed.
    pub query_type: String,
    /// Result data (JSON-serializable).
    pub data: serde_json::Value,
}

/// Session information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session name.
    pub name: String,
    /// Session status.
    pub status: SessionStatus,
    /// Workspace path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    /// Assigned agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// Session status enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session is active and in progress.
    Active,
    /// Session is paused.
    Paused,
    /// Session is completed.
    Completed,
    /// Session was aborted.
    Aborted,
}

impl SessionStatus {
    /// Parse from string.
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "paused" => Self::Paused,
            "completed" => Self::Completed,
            "aborted" => Self::Aborted,
            _ => Self::Active,
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Aborted => "aborted",
        }
    }
}

// ============================================================================
// Pure computation functions (Tier 2)
// ============================================================================

/// Filter sessions by status.
pub fn filter_sessions_by_status<'a>(
    sessions: &'a [SessionInfo],
    status: &str,
) -> Vec<&'a SessionInfo> {
    sessions
        .iter()
        .filter(|s| s.status.as_str() == status)
        .collect()
}

/// Filter sessions by agent.
pub fn filter_sessions_by_agent<'a>(
    sessions: &'a [SessionInfo],
    agent: &str,
) -> Vec<&'a SessionInfo> {
    sessions
        .iter()
        .filter(|s| s.agent.as_deref() == Some(agent))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_type_from_str() {
        assert_eq!(
            QueryType::from_str("session-exists"),
            Some(QueryType::SessionExists)
        );
        assert_eq!(QueryType::from_str("sessions"), Some(QueryType::Sessions));
        assert_eq!(QueryType::from_str("blockers"), Some(QueryType::Blockers));
        assert_eq!(QueryType::from_str("help"), Some(QueryType::Help));
        assert_eq!(QueryType::from_str("unknown"), None);
    }

    #[test]
    fn query_type_all_names() {
        let names = QueryType::all_names();
        assert!(names.contains(&"session-exists"));
        assert!(names.contains(&"sessions"));
        assert!(names.contains(&"help"));
    }

    #[test]
    fn session_status_roundtrip() {
        assert_eq!(
            SessionStatus::from_str_lossy("active"),
            SessionStatus::Active
        );
        assert_eq!(
            SessionStatus::from_str_lossy("paused"),
            SessionStatus::Paused
        );
        assert_eq!(
            SessionStatus::from_str_lossy("completed"),
            SessionStatus::Completed
        );
        assert_eq!(
            SessionStatus::from_str_lossy("aborted"),
            SessionStatus::Aborted
        );
        assert_eq!(
            SessionStatus::from_str_lossy("unknown"),
            SessionStatus::Active
        );
    }

    #[test]
    fn session_status_as_str() {
        assert_eq!(SessionStatus::Active.as_str(), "active");
        assert_eq!(SessionStatus::Paused.as_str(), "paused");
        assert_eq!(SessionStatus::Completed.as_str(), "completed");
        assert_eq!(SessionStatus::Aborted.as_str(), "aborted");
    }

    #[test]
    fn session_info_serialization() {
        let info = SessionInfo {
            name: "test".to_string(),
            status: SessionStatus::Active,
            workspace_path: Some("/path".to_string()),
            agent: None,
            created_at: None,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"status\":\"Active\""));
    }

    #[test]
    fn query_output_serialization() {
        let output = QueryOutput {
            success: true,
            query_type: "session-exists".to_string(),
            data: serde_json::json!({"exists": false}),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn filter_sessions_by_status_active() {
        let sessions = vec![
            SessionInfo {
                name: "a".to_string(),
                status: SessionStatus::Active,
                workspace_path: None,
                agent: None,
                created_at: None,
            },
            SessionInfo {
                name: "b".to_string(),
                status: SessionStatus::Completed,
                workspace_path: None,
                agent: None,
                created_at: None,
            },
        ];
        let active = filter_sessions_by_status(&sessions, "active");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "a");
    }

    #[test]
    fn filter_sessions_by_agent_works() {
        let sessions = vec![
            SessionInfo {
                name: "a".to_string(),
                status: SessionStatus::Active,
                workspace_path: None,
                agent: Some("agent-1".to_string()),
                created_at: None,
            },
            SessionInfo {
                name: "b".to_string(),
                status: SessionStatus::Active,
                workspace_path: None,
                agent: Some("agent-2".to_string()),
                created_at: None,
            },
        ];
        let filtered = super::filter_sessions_by_agent(&sessions, "agent-1");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "a");
    }
}
