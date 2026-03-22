//! Tests for identifier types
//!
//! This module contains comprehensive tests for all identifier types.

use crate::domain::identifiers::error::IdentifierError;
use crate::domain::identifiers::{
    AbsolutePath, AgentId, BeadId, SessionId, SessionName, TaskId, WorkspaceName,
};

// ============================================================================
// SessionName Tests
// ============================================================================

#[test]
fn test_valid_session_name() {
    assert!(SessionName::parse("my-session").is_ok());
    assert!(SessionName::parse("my_session").is_ok());
    assert!(SessionName::parse("my-session-123").is_ok());
}

#[test]
fn test_session_name_trims_whitespace() {
    // Trim-then-validate: whitespace is trimmed, then validated
    let name = SessionName::parse("  my-session  ").expect("valid");
    assert_eq!(name.as_str(), "my-session");

    let name2 = SessionName::parse("\tmy-session\t").expect("valid");
    assert_eq!(name2.as_str(), "my-session");

    let name3 = SessionName::parse("\nmy-session\n").expect("valid");
    assert_eq!(name3.as_str(), "my-session");
}

#[test]
fn test_session_name_whitespace_only_is_invalid() {
    // Whitespace-only strings become empty after trimming
    let result = SessionName::parse("   ");
    assert!(result.is_err());
    assert!(matches!(result, Err(IdentifierError::Empty)));
}

#[test]
fn test_invalid_session_name_empty() {
    let result = SessionName::parse("");
    assert!(result.is_err());
    assert!(matches!(result, Err(IdentifierError::Empty)));
}

#[test]
fn test_invalid_session_name_starts_with_number() {
    let result = SessionName::parse("123-session");
    assert!(result.is_err());
}

#[test]
fn test_invalid_session_name_special_chars() {
    assert!(SessionName::parse("my.session").is_err());
    assert!(SessionName::parse("my:session").is_err());
    assert!(SessionName::parse("my session").is_err());
}

#[test]
fn test_invalid_session_name_too_long() {
    let long_name = "a".repeat(64);
    let result = SessionName::parse(&long_name);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(IdentifierError::TooLong { max: 63, .. })
    ));
}

#[test]
fn test_session_name_display() {
    match SessionName::parse("test-session") {
        Ok(name) => {
            assert_eq!(name.to_string(), "test-session");
            assert_eq!(name.as_str(), "test-session");
        }
        Err(e) => panic!("Failed to parse valid session name: {e}"),
    }
}

// ============================================================================
// AgentId Tests
// ============================================================================

#[test]
fn test_valid_agent_id() {
    assert!(AgentId::parse("agent-123").is_ok());
    assert!(AgentId::parse("agent_456").is_ok());
    assert!(AgentId::parse("agent:789").is_ok());
    assert!(AgentId::parse("agent.example").is_ok());
}

#[test]
fn test_invalid_agent_id_empty() {
    let result = AgentId::parse("");
    assert!(result.is_err());
    assert!(matches!(result, Err(IdentifierError::Empty)));
}

#[test]
fn test_invalid_agent_id_special_chars() {
    assert!(AgentId::parse("agent/123").is_err());
    assert!(AgentId::parse("agent 123").is_err());
}

#[test]
fn test_agent_id_from_process() {
    let agent = AgentId::from_process();
    let agent_str = agent.as_str();
    assert!(agent_str.starts_with("pid-"));
}

// ============================================================================
// WorkspaceName Tests
// ============================================================================

#[test]
fn test_valid_workspace_name() {
    assert!(WorkspaceName::parse("my-workspace").is_ok());
    assert!(WorkspaceName::parse("my_workspace").is_ok());
}

#[test]
fn test_invalid_workspace_name_with_path_separator() {
    assert!(WorkspaceName::parse("my/workspace").is_err());
    assert!(WorkspaceName::parse("my\\workspace").is_err());
}

#[test]
fn test_invalid_workspace_name_with_null() {
    assert!(WorkspaceName::parse("my\0workspace").is_err());
}

#[test]
fn test_invalid_workspace_name_too_long() {
    let long_name = "a".repeat(256);
    let result = WorkspaceName::parse(&long_name);
    assert!(result.is_err());
}

// ============================================================================
// TaskId Tests
// ============================================================================

#[test]
fn test_valid_task_id() {
    assert!(TaskId::parse("bd-abc123").is_ok());
    assert!(TaskId::parse("bd-ABC123DEF456").is_ok());
    assert!(TaskId::parse("bd-1234567890abcdef").is_ok());
}

#[test]
fn test_invalid_task_id_no_prefix() {
    let result = TaskId::parse("abc123");
    assert!(result.is_err());
    assert!(matches!(result, Err(IdentifierError::InvalidPrefix { .. })));
}

#[test]
fn test_invalid_task_id_empty() {
    let result = TaskId::parse("");
    assert!(result.is_err());
}

#[test]
fn test_invalid_task_id_no_hex() {
    assert!(TaskId::parse("bd-xyz").is_err());
    assert!(TaskId::parse("bd-123-456").is_err());
}

#[test]
fn test_task_id_display() {
    match TaskId::parse("bd-abc123") {
        Ok(task) => {
            assert_eq!(task.to_string(), "bd-abc123");
            assert_eq!(task.as_str(), "bd-abc123");
        }
        Err(e) => panic!("Failed to parse valid task ID: {e}"),
    }
}

// ============================================================================
// BeadId Alias Tests
// ============================================================================

#[test]
fn test_bead_id_is_task_id() {
    match (BeadId::parse("bd-abc123"), TaskId::parse("bd-abc123")) {
        (Ok(bead), Ok(task)) => {
            assert_eq!(bead.as_str(), task.as_str());
        }
        (Err(e), _) => panic!("Failed to parse bead ID: {e}"),
        (_, Err(e)) => panic!("Failed to parse task ID: {e}"),
    }
}

// ============================================================================
// SessionId Tests
// ============================================================================

#[test]
fn test_valid_session_id() {
    assert!(SessionId::parse("session-abc123").is_ok());
    assert!(SessionId::parse("sess-123").is_ok());
    assert!(SessionId::parse("SESSION_ABC").is_ok());
}

#[test]
fn test_invalid_session_id_empty() {
    let result = SessionId::parse("");
    assert!(result.is_err());
    assert!(matches!(result, Err(IdentifierError::Empty)));
}

#[test]
fn test_invalid_session_id_non_ascii() {
    let result = SessionId::parse("session-abc-日本語");
    assert!(result.is_err());
}

#[test]
fn test_session_id_display() {
    match SessionId::parse("session-abc123") {
        Ok(id) => {
            assert_eq!(id.to_string(), "session-abc123");
            assert_eq!(id.as_str(), "session-abc123");
        }
        Err(e) => panic!("Failed to parse valid session ID: {e}"),
    }
}

// ============================================================================
// AbsolutePath Tests
// ============================================================================

#[test]
fn test_valid_absolute_path() {
    assert!(AbsolutePath::parse("/home/user").is_ok());
    assert!(AbsolutePath::parse("/tmp/workspace").is_ok());
    assert!(AbsolutePath::parse("/").is_ok());
}

#[test]
fn test_invalid_absolute_path_empty() {
    let result = AbsolutePath::parse("");
    assert!(result.is_err());
}

#[test]
fn test_invalid_absolute_path_relative() {
    let result = AbsolutePath::parse("relative/path");
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(IdentifierError::NotAbsolutePath { .. })
    ));
}

#[test]
fn test_invalid_absolute_path_null_bytes() {
    let result = AbsolutePath::parse("/path\0with\0nulls");
    assert!(result.is_err());
    assert!(matches!(result, Err(IdentifierError::NullBytesInPath)));
}

#[test]
fn test_absolute_path_display() {
    match AbsolutePath::parse("/home/user/workspace") {
        Ok(path) => {
            assert_eq!(path.to_string(), "/home/user/workspace");
            assert_eq!(path.as_str(), "/home/user/workspace");
        }
        Err(e) => panic!("Failed to parse valid absolute path: {e}"),
    }
}

#[test]
fn test_absolute_path_to_path_buf() {
    match AbsolutePath::parse("/home/user/workspace") {
        Ok(path) => {
            let path_buf = path.to_path_buf();
            assert_eq!(path_buf, std::path::PathBuf::from("/home/user/workspace"));
        }
        Err(e) => panic!("Failed to parse valid absolute path: {e}"),
    }
}
