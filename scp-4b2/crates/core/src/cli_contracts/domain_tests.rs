//! Integration tests for domain types.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use crate::cli_contracts::{
        domain_types::{
            AgentId, AgentStatus, AgentType, ConfigKey, ConfigScope, FileStatus, Limit,
            NonEmptyString, OutputFormat, Priority, QueueStatus, SessionName, SessionStatus,
            TaskId, TaskPriority, TaskStatus, TimeoutSeconds,
        },
        ContractError,
    };
    use std::str::FromStr;

    // ═══════════════════════════════════════════════════════════════════════════
    // IDENTIFIER TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_session_name_valid() {
        assert!(SessionName::try_from("valid-name").is_ok());
        assert!(SessionName::try_from("Feature_Auth").is_ok());
        assert!(SessionName::try_from("a").is_ok());
        assert!(SessionName::try_from("test123").is_ok());
    }

    #[test]
    fn test_session_name_invalid() {
        assert!(matches!(
            SessionName::try_from(""),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            SessionName::try_from("1invalid"),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            SessionName::try_from("-invalid"),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            SessionName::try_from("invalid name"),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            SessionName::try_from("invalid@name"),
            Err(ContractError::InvalidInput { .. })
        ));
    }

    #[test]
    fn test_session_name_display() {
        match SessionName::try_from("test-session") {
            Ok(name) => {
                assert_eq!(name.as_str(), "test-session");
                assert_eq!(name.to_string(), "test-session");
            }
            Err(e) => panic!("Failed to parse valid session name: {e}"),
        }
    }

    #[test]
    fn test_task_id_valid() {
        assert!(TaskId::try_from("TASK-123").is_ok());
        assert!(TaskId::try_from("task-456").is_ok());
    }

    #[test]
    fn test_task_id_invalid() {
        assert!(matches!(
            TaskId::try_from(""),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            TaskId::try_from("  "),
            Err(ContractError::InvalidInput { .. })
        ));
    }

    #[test]
    fn test_agent_id_valid() {
        // AgentId is now re-exported from domain::identifiers with stricter validation
        // It accepts: alphanumeric, hyphen, underscore, dot, colon (1-128 chars)
        assert!(AgentId::try_from("agent-123").is_ok());
        assert!(AgentId::try_from("agent_456").is_ok());
        assert!(AgentId::try_from("agent:789").is_ok());
        assert!(AgentId::try_from("agent.example").is_ok());
    }

    #[test]
    fn test_agent_id_invalid() {
        // Empty string is invalid
        assert!(matches!(
            AgentId::try_from(""),
            Err(ContractError::InvalidInput { .. })
        ));
        // Whitespace only becomes empty after trim
        assert!(matches!(
            AgentId::try_from("  "),
            Err(ContractError::InvalidInput { .. })
        ));
        // Invalid characters (spaces, slashes)
        assert!(AgentId::try_from("agent 123").is_err());
        assert!(AgentId::try_from("agent/123").is_err());
    }

    #[test]
    fn test_config_key_valid() {
        assert!(ConfigKey::try_from("session.max_count").is_ok());
        assert!(ConfigKey::try_from("hooks.pre_create").is_ok());
        assert!(ConfigKey::try_from("a.b").is_ok());
    }

    #[test]
    fn test_config_key_invalid() {
        assert!(matches!(
            ConfigKey::try_from(""),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            ConfigKey::try_from("session"),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            ConfigKey::try_from("session."),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            ConfigKey::try_from(".session"),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            ConfigKey::try_from("session.max-count"),
            Err(ContractError::InvalidInput { .. })
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STATE ENUM TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_session_status_from_str() {
        match SessionStatus::from_str("creating") {
            Ok(status) => assert_eq!(status, SessionStatus::Creating),
            Err(e) => panic!("Failed to parse 'creating': {e}"),
        }
        match SessionStatus::from_str("active") {
            Ok(status) => assert_eq!(status, SessionStatus::Active),
            Err(e) => panic!("Failed to parse 'active': {e}"),
        }
        match SessionStatus::from_str("paused") {
            Ok(status) => assert_eq!(status, SessionStatus::Paused),
            Err(e) => panic!("Failed to parse 'paused': {e}"),
        }
        match SessionStatus::from_str("completed") {
            Ok(status) => assert_eq!(status, SessionStatus::Completed),
            Err(e) => panic!("Failed to parse 'completed': {e}"),
        }
        match SessionStatus::from_str("failed") {
            Ok(status) => assert_eq!(status, SessionStatus::Failed),
            Err(e) => panic!("Failed to parse 'failed': {e}"),
        }
    }

    #[test]
    fn test_session_status_invalid() {
        assert!(matches!(
            SessionStatus::from_str("pending"),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            SessionStatus::from_str("running"),
            Err(ContractError::InvalidInput { .. })
        ));
    }

    #[test]
    fn test_session_status_transitions() {
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Failed));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Completed));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Completed));

        // Invalid transitions
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));
        assert!(!SessionStatus::Completed.can_transition_to(SessionStatus::Active));
        assert!(!SessionStatus::Failed.can_transition_to(SessionStatus::Active));
    }

    #[test]
    fn test_queue_status_from_str() {
        match QueueStatus::from_str("pending") {
            Ok(status) => assert_eq!(status, QueueStatus::Pending),
            Err(e) => panic!("Failed to parse 'pending': {e}"),
        }
        match QueueStatus::from_str("processing") {
            Ok(status) => assert_eq!(status, QueueStatus::Processing),
            Err(e) => panic!("Failed to parse 'processing': {e}"),
        }
        match QueueStatus::from_str("completed") {
            Ok(status) => assert_eq!(status, QueueStatus::Completed),
            Err(e) => panic!("Failed to parse 'completed': {e}"),
        }
        match QueueStatus::from_str("failed") {
            Ok(status) => assert_eq!(status, QueueStatus::Failed),
            Err(e) => panic!("Failed to parse 'failed': {e}"),
        }
        match QueueStatus::from_str("cancelled") {
            Ok(status) => assert_eq!(status, QueueStatus::Cancelled),
            Err(e) => panic!("Failed to parse 'cancelled': {e}"),
        }
    }

    #[test]
    fn test_agent_status_from_str() {
        match AgentStatus::from_str("pending") {
            Ok(status) => assert_eq!(status, AgentStatus::Pending),
            Err(e) => panic!("Failed to parse 'pending': {e}"),
        }
        match AgentStatus::from_str("running") {
            Ok(status) => assert_eq!(status, AgentStatus::Running),
            Err(e) => panic!("Failed to parse 'running': {e}"),
        }
        match AgentStatus::from_str("timeout") {
            Ok(status) => assert_eq!(status, AgentStatus::Timeout),
            Err(e) => panic!("Failed to parse 'timeout': {e}"),
        }
    }

    #[test]
    fn test_task_status_from_str() {
        match TaskStatus::from_str("open") {
            Ok(status) => assert_eq!(status, TaskStatus::Open),
            Err(e) => panic!("Failed to parse 'open': {e}"),
        }
        match TaskStatus::from_str("in_progress") {
            Ok(status) => assert_eq!(status, TaskStatus::InProgress),
            Err(e) => panic!("Failed to parse 'in_progress': {e}"),
        }
        match TaskStatus::from_str("blocked") {
            Ok(status) => assert_eq!(status, TaskStatus::Blocked),
            Err(e) => panic!("Failed to parse 'blocked': {e}"),
        }
        match TaskStatus::from_str("closed") {
            Ok(status) => assert_eq!(status, TaskStatus::Closed),
            Err(e) => panic!("Failed to parse 'closed': {e}"),
        }
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::P0 < TaskPriority::P1);
        assert!(TaskPriority::P1 < TaskPriority::P2);
        assert!(TaskPriority::P2 < TaskPriority::P3);
        assert!(TaskPriority::P3 < TaskPriority::P4);
    }

    #[test]
    fn test_task_priority_from_str() {
        match TaskPriority::from_str("P0") {
            Ok(priority) => assert_eq!(priority, TaskPriority::P0),
            Err(e) => panic!("Failed to parse 'P0': {e}"),
        }
        match TaskPriority::from_str("P4") {
            Ok(priority) => assert_eq!(priority, TaskPriority::P4),
            Err(e) => panic!("Failed to parse 'P4': {e}"),
        }
    }

    #[test]
    fn test_config_scope_from_str() {
        match ConfigScope::from_str("local") {
            Ok(scope) => assert_eq!(scope, ConfigScope::Local),
            Err(e) => panic!("Failed to parse 'local': {e}"),
        }
        match ConfigScope::from_str("global") {
            Ok(scope) => assert_eq!(scope, ConfigScope::Global),
            Err(e) => panic!("Failed to parse 'global': {e}"),
        }
        match ConfigScope::from_str("system") {
            Ok(scope) => assert_eq!(scope, ConfigScope::System),
            Err(e) => panic!("Failed to parse 'system': {e}"),
        }
    }

    #[test]
    fn test_agent_type_from_str() {
        match AgentType::from_str("claude") {
            Ok(agent_type) => assert_eq!(agent_type, AgentType::Claude),
            Err(e) => panic!("Failed to parse 'claude': {e}"),
        }
        match AgentType::from_str("cursor") {
            Ok(agent_type) => assert_eq!(agent_type, AgentType::Cursor),
            Err(e) => panic!("Failed to parse 'cursor': {e}"),
        }
        match AgentType::from_str("aider") {
            Ok(agent_type) => assert_eq!(agent_type, AgentType::Aider),
            Err(e) => panic!("Failed to parse 'aider': {e}"),
        }
        match AgentType::from_str("copilot") {
            Ok(agent_type) => assert_eq!(agent_type, AgentType::Copilot),
            Err(e) => panic!("Failed to parse 'copilot': {e}"),
        }
    }

    #[test]
    fn test_output_format_from_str() {
        match OutputFormat::from_str("text") {
            Ok(format) => assert_eq!(format, OutputFormat::Text),
            Err(e) => panic!("Failed to parse 'text': {e}"),
        }
        match OutputFormat::from_str("json") {
            Ok(format) => assert_eq!(format, OutputFormat::Json),
            Err(e) => panic!("Failed to parse 'json': {e}"),
        }
        match OutputFormat::from_str("yaml") {
            Ok(format) => assert_eq!(format, OutputFormat::Yaml),
            Err(e) => panic!("Failed to parse 'yaml': {e}"),
        }
    }

    #[test]
    fn test_file_status_from_str() {
        match FileStatus::from_str("M") {
            Ok(status) => assert_eq!(status, FileStatus::Modified),
            Err(e) => panic!("Failed to parse 'M': {e}"),
        }
        match FileStatus::from_str("A") {
            Ok(status) => assert_eq!(status, FileStatus::Added),
            Err(e) => panic!("Failed to parse 'A': {e}"),
        }
        match FileStatus::from_str("D") {
            Ok(status) => assert_eq!(status, FileStatus::Deleted),
            Err(e) => panic!("Failed to parse 'D': {e}"),
        }
        match FileStatus::from_str("R") {
            Ok(status) => assert_eq!(status, FileStatus::Renamed),
            Err(e) => panic!("Failed to parse 'R': {e}"),
        }
        match FileStatus::from_str("?") {
            Ok(status) => assert_eq!(status, FileStatus::Untracked),
            Err(e) => panic!("Failed to parse '?': {e}"),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VALUE OBJECT TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_non_empty_string_valid() {
        assert!(NonEmptyString::try_from("valid").is_ok());
        assert!(NonEmptyString::try_from("  valid  ").is_ok());
    }

    #[test]
    fn test_non_empty_string_invalid() {
        assert!(matches!(
            NonEmptyString::try_from(""),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            NonEmptyString::try_from("   "),
            Err(ContractError::InvalidInput { .. })
        ));
    }

    #[test]
    fn test_limit_valid() {
        assert!(Limit::try_from(1).is_ok());
        assert!(Limit::try_from(500).is_ok());
        assert!(Limit::try_from(1000).is_ok());
    }

    #[test]
    fn test_limit_invalid() {
        assert!(matches!(
            Limit::try_from(0),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            Limit::try_from(1001),
            Err(ContractError::InvalidInput { .. })
        ));
    }

    #[test]
    fn test_limit_value() {
        match Limit::try_from(100) {
            Ok(limit) => assert_eq!(limit.value(), 100),
            Err(e) => panic!("Failed to create Limit: {e}"),
        }
    }

    #[test]
    fn test_priority_valid() {
        assert!(Priority::try_from(0).is_ok());
        assert!(Priority::try_from(500).is_ok());
        assert!(Priority::try_from(1000).is_ok());
    }

    #[test]
    fn test_priority_invalid() {
        assert!(matches!(
            Priority::try_from(1001),
            Err(ContractError::InvalidInput { .. })
        ));
    }

    #[test]
    fn test_timeout_valid() {
        assert!(TimeoutSeconds::try_from(1).is_ok());
        assert!(TimeoutSeconds::try_from(3600).is_ok());
        assert!(TimeoutSeconds::try_from(86400).is_ok());
    }

    #[test]
    fn test_timeout_invalid() {
        assert!(matches!(
            TimeoutSeconds::try_from(0),
            Err(ContractError::InvalidInput { .. })
        ));
        assert!(matches!(
            TimeoutSeconds::try_from(86401),
            Err(ContractError::InvalidInput { .. })
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DISPLAY TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_display_session_status() {
        assert_eq!(SessionStatus::Creating.to_string(), "creating");
        assert_eq!(SessionStatus::Active.to_string(), "active");
        assert_eq!(SessionStatus::Paused.to_string(), "paused");
        assert_eq!(SessionStatus::Completed.to_string(), "completed");
        assert_eq!(SessionStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_display_task_priority() {
        assert_eq!(TaskPriority::P0.to_string(), "P0");
        assert_eq!(TaskPriority::P1.to_string(), "P1");
        assert_eq!(TaskPriority::P2.to_string(), "P2");
    }

    #[test]
    fn test_display_agent_type() {
        assert_eq!(AgentType::Claude.to_string(), "claude");
        assert_eq!(AgentType::Cursor.to_string(), "cursor");
    }
}
