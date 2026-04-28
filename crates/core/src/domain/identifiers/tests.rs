//! Tests for identifier types
//!
//! This module contains comprehensive tests for all identifier types.

use crate::domain::identifiers::{
    error::IdentifierError, AbsolutePath, AgentId, BeadId, SessionId, SessionName, TaskId,
    WorkspaceName,
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

// ============================================================================
// SessionName - Extended Tests
// ============================================================================

#[test]
fn test_session_name_single_letter() {
    assert!(SessionName::parse("a").is_ok());
    assert!(SessionName::parse("Z").is_ok());
}

#[test]
fn test_session_name_max_length_exact() {
    let name = "a".repeat(63);
    assert!(SessionName::parse(&name).is_ok());
}

#[test]
fn test_session_name_into_string() {
    let name = SessionName::parse("test-name").expect("valid");
    assert_eq!(name.into_string(), "test-name");
}

#[test]
fn test_session_name_as_ref_str() {
    let name = SessionName::parse("hello").expect("valid");
    let s: &str = name.as_ref();
    assert_eq!(s, "hello");
}

#[test]
fn test_session_name_from_str_trait() {
    use std::str::FromStr;
    let name = SessionName::from_str("my-session");
    assert!(name.is_ok());
}

#[test]
fn test_session_name_try_from_string() {
    let name = SessionName::try_from(String::from("abc"));
    assert!(name.is_ok());
}

#[test]
fn test_session_name_try_from_str() {
    let name = SessionName::try_from("abc");
    assert!(name.is_ok());
}

#[test]
fn test_session_name_into_string_conversion() {
    let name = SessionName::parse("convert-me").expect("valid");
    let s: String = name.clone().into();
    assert_eq!(s, "convert-me");
}

#[test]
fn test_session_name_max_length_constant() {
    assert_eq!(SessionName::MAX_LENGTH, 63);
}

#[test]
fn test_session_name_hash() {
    use std::collections::HashSet;
    let a = SessionName::parse("duplicate").expect("valid");
    let b = SessionName::parse("duplicate").expect("valid");
    let set: HashSet<SessionName> = [a, b].into_iter().collect();
    assert_eq!(set.len(), 1);
}

#[test]
fn test_session_name_clone() {
    let name = SessionName::parse("original").expect("valid");
    let cloned = name.clone();
    assert_eq!(name, cloned);
}

// ============================================================================
// SessionId - Extended Tests
// ============================================================================

#[test]
fn test_session_id_into_string() {
    let id = SessionId::parse("my-id-123").expect("valid");
    assert_eq!(id.into_string(), "my-id-123");
}

#[test]
fn test_session_id_as_ref_str() {
    let id = SessionId::parse("ref-test").expect("valid");
    let s: &str = id.as_ref();
    assert_eq!(s, "ref-test");
}

#[test]
fn test_session_id_try_from_string() {
    let id = SessionId::try_from(String::from("abc-123"));
    assert!(id.is_ok());
}

#[test]
fn test_session_id_try_from_str() {
    let id = SessionId::try_from("abc-123");
    assert!(id.is_ok());
}

#[test]
fn test_session_id_hash() {
    use std::collections::HashSet;
    let a = SessionId::parse("same").expect("valid");
    let b = SessionId::parse("same").expect("valid");
    let set: HashSet<SessionId> = [a, b].into_iter().collect();
    assert_eq!(set.len(), 1);
}

#[test]
fn test_session_id_clone() {
    let id = SessionId::parse("original").expect("valid");
    let cloned = id.clone();
    assert_eq!(id, cloned);
}

#[test]
fn test_session_id_with_underscore_is_valid() {
    // Domain SessionId only requires non-empty + ASCII, so underscores are fine
    assert!(SessionId::parse("abc_def").is_ok());
}

#[test]
fn test_session_id_with_slash_is_valid() {
    // Domain SessionId only requires non-empty + ASCII, so slashes are fine
    assert!(SessionId::parse("abc/def").is_ok());
}

// ============================================================================
// AgentId - Extended Tests
// ============================================================================

#[test]
fn test_agent_id_into_string() {
    let id = AgentId::parse("agent-123").expect("valid");
    assert_eq!(id.into_string(), "agent-123");
}

#[test]
fn test_agent_id_display() {
    let id = AgentId::parse("my-agent").expect("valid");
    assert_eq!(format!("{id}"), "my-agent");
}

#[test]
fn test_agent_id_as_ref_str() {
    let id = AgentId::parse("ref-agent").expect("valid");
    let s: &str = id.as_ref();
    assert_eq!(s, "ref-agent");
}

#[test]
fn test_agent_id_try_from_string() {
    let id = AgentId::try_from(String::from("agent-test"));
    assert!(id.is_ok());
}

#[test]
fn test_agent_id_try_from_str() {
    let id = AgentId::try_from("agent-test");
    assert!(id.is_ok());
}

#[test]
fn test_agent_id_hash() {
    use std::collections::HashSet;
    let a = AgentId::parse("agent-x").expect("valid");
    let b = AgentId::parse("agent-x").expect("valid");
    let set: HashSet<AgentId> = [a, b].into_iter().collect();
    assert_eq!(set.len(), 1);
}

#[test]
fn test_agent_id_clone() {
    let id = AgentId::parse("agent-clone").expect("valid");
    let cloned = id.clone();
    assert_eq!(id, cloned);
}

#[test]
fn test_agent_id_empty_rejected() {
    assert!(AgentId::parse("").is_err());
}

#[test]
fn test_agent_id_too_long() {
    let long_id = "a".repeat(129);
    assert!(AgentId::parse(&long_id).is_err());
}

#[test]
fn test_agent_id_max_length_boundary() {
    let id = "a".repeat(128);
    assert!(AgentId::parse(&id).is_ok());
}

#[test]
fn test_agent_id_with_slash_rejected() {
    assert!(AgentId::parse("agent/test").is_err());
}

#[test]
fn test_agent_id_with_backslash_rejected() {
    assert!(AgentId::parse("agent\\test").is_err());
}

// ============================================================================
// WorkspaceName - Extended Tests
// ============================================================================

#[test]
fn test_workspace_name_into_string() {
    let name = WorkspaceName::parse("my-ws").expect("valid");
    assert_eq!(name.into_string(), "my-ws");
}

#[test]
fn test_workspace_name_display() {
    let name = WorkspaceName::parse("my-workspace").expect("valid");
    assert_eq!(format!("{name}"), "my-workspace");
}

#[test]
fn test_workspace_name_as_ref_str() {
    let name = WorkspaceName::parse("ref-ws").expect("valid");
    let s: &str = name.as_ref();
    assert_eq!(s, "ref-ws");
}

#[test]
fn test_workspace_name_try_from_string() {
    let name = WorkspaceName::try_from(String::from("ws-test"));
    assert!(name.is_ok());
}

#[test]
fn test_workspace_name_try_from_str() {
    let name = WorkspaceName::try_from("ws-test");
    assert!(name.is_ok());
}

#[test]
fn test_workspace_name_hash() {
    use std::collections::HashSet;
    let a = WorkspaceName::parse("dup-ws").expect("valid");
    let b = WorkspaceName::parse("dup-ws").expect("valid");
    let set: HashSet<WorkspaceName> = [a, b].into_iter().collect();
    assert_eq!(set.len(), 1);
}

#[test]
fn test_workspace_name_clone() {
    let name = WorkspaceName::parse("ws-clone").expect("valid");
    let cloned = name.clone();
    assert_eq!(name, cloned);
}

#[test]
fn test_workspace_name_empty_rejected() {
    assert!(WorkspaceName::parse("").is_err());
}

#[test]
fn test_workspace_name_max_length_boundary() {
    let name = "a".repeat(255);
    assert!(WorkspaceName::parse(&name).is_ok());
}

#[test]
fn test_workspace_name_over_max_length() {
    let name = "a".repeat(256);
    assert!(WorkspaceName::parse(&name).is_err());
}

#[test]
fn test_workspace_name_with_spaces_ok() {
    // Workspace names only reject path separators and nulls, spaces are allowed
    assert!(WorkspaceName::parse("my workspace").is_ok());
}

// ============================================================================
// TaskId / BeadId - Extended Tests
// ============================================================================

#[test]
fn test_task_id_into_string() {
    let id = TaskId::parse("bd-abc123").expect("valid");
    assert_eq!(id.into_string(), "bd-abc123");
}

#[test]
fn test_task_id_as_ref_str() {
    let id = TaskId::parse("bd-abc").expect("valid");
    let s: &str = id.as_ref();
    assert_eq!(s, "bd-abc");
}

#[test]
fn test_task_id_try_from_string() {
    let id = TaskId::try_from(String::from("bd-deadbeef"));
    assert!(id.is_ok());
}

#[test]
fn test_task_id_try_from_str() {
    let id = TaskId::try_from("bd-deadbeef");
    assert!(id.is_ok());
}

#[test]
fn test_task_id_hash() {
    use std::collections::HashSet;
    let a = TaskId::parse("bd-abc").expect("valid");
    let b = TaskId::parse("bd-abc").expect("valid");
    let set: HashSet<TaskId> = [a, b].into_iter().collect();
    assert_eq!(set.len(), 1);
}

#[test]
fn test_task_id_clone() {
    let id = TaskId::parse("bd-abcdef").expect("valid");
    let cloned = id.clone();
    assert_eq!(id, cloned);
}

#[test]
fn test_task_id_prefix_only_rejected() {
    assert!(TaskId::parse("bd-").is_err());
}

#[test]
fn test_task_id_empty_hex_rejected() {
    assert!(TaskId::parse("bd-").is_err());
}

#[test]
fn test_bead_id_into_string() {
    let id = BeadId::parse("bd-bead123").expect("valid");
    assert_eq!(id.into_string(), "bd-bead123");
}

#[test]
fn test_bead_id_as_ref_str() {
    let id = BeadId::parse("bd-abcdef").expect("valid");
    let s: &str = id.as_ref();
    assert_eq!(s, "bd-abcdef");
}

#[test]
fn test_bead_id_clone() {
    let id = BeadId::parse("bd-abcdef").expect("valid");
    let cloned = id.clone();
    assert_eq!(id, cloned);
}

#[test]
fn test_bead_id_try_from_string() {
    let id = BeadId::try_from(String::from("bd-abcdef"));
    assert!(id.is_ok());
}

#[test]
fn test_bead_id_try_from_str() {
    let id = BeadId::try_from("bd-abcdef");
    assert!(id.is_ok());
}

// ============================================================================
// AbsolutePath - Extended Tests
// ============================================================================

#[test]
fn test_absolute_path_into_string() {
    let path = AbsolutePath::parse("/home/user").expect("valid");
    assert_eq!(path.into_string(), "/home/user");
}

#[test]
fn test_absolute_path_as_ref_str() {
    let path = AbsolutePath::parse("/tmp/test").expect("valid");
    let s: &str = path.as_ref();
    assert_eq!(s, "/tmp/test");
}

#[test]
fn test_absolute_path_try_from_string() {
    let path = AbsolutePath::try_from(String::from("/home/user"));
    assert!(path.is_ok());
}

#[test]
fn test_absolute_path_try_from_str() {
    let path = AbsolutePath::try_from("/home/user");
    assert!(path.is_ok());
}

#[test]
fn test_absolute_path_hash() {
    use std::collections::HashSet;
    let a = AbsolutePath::parse("/same").expect("valid");
    let b = AbsolutePath::parse("/same").expect("valid");
    let set: HashSet<AbsolutePath> = [a, b].into_iter().collect();
    assert_eq!(set.len(), 1);
}

#[test]
fn test_absolute_path_clone() {
    let path = AbsolutePath::parse("/clone/me").expect("valid");
    let cloned = path.clone();
    assert_eq!(path, cloned);
}

#[test]
fn test_absolute_path_display_method() {
    let path = AbsolutePath::parse("/display/test").expect("valid");
    assert_eq!(format!("{}", path.display()), "/display/test");
}

#[test]
fn test_absolute_path_root() {
    let path = AbsolutePath::parse("/").expect("valid");
    assert_eq!(path.as_str(), "/");
}

#[test]
fn test_absolute_path_trailing_slash() {
    let path = AbsolutePath::parse("/home/user/").expect("valid");
    assert_eq!(path.as_str(), "/home/user/");
}

// ============================================================================
// IdentifierError - Extended Tests
// ============================================================================

#[test]
fn test_identifier_error_empty() {
    let err = IdentifierError::empty();
    assert!(err.is_empty());
    assert_eq!(format!("{err}"), "identifier cannot be empty");
}

#[test]
fn test_identifier_error_too_long() {
    let err = IdentifierError::too_long(63, 100);
    assert!(err.is_too_long());
    assert!(matches!(
        err,
        IdentifierError::TooLong {
            max: 63,
            actual: 100
        }
    ));
    assert!(format!("{err}").contains("100 characters"));
    assert!(format!("{err}").contains("max 63"));
}

#[test]
fn test_identifier_error_invalid_characters() {
    let err = IdentifierError::invalid_characters("bad chars found");
    assert!(err.is_invalid_characters());
    assert!(format!("{err}").contains("bad chars found"));
}

#[test]
fn test_identifier_error_invalid_format() {
    let err = IdentifierError::invalid_format("bad format");
    assert!(err.is_invalid_format());
    assert!(format!("{err}").contains("bad format"));
}

#[test]
fn test_identifier_error_invalid_start() {
    let err = IdentifierError::invalid_start('a');
    assert!(format!("{err}").contains("must start with a letter"));
}

#[test]
fn test_identifier_error_invalid_prefix() {
    let err = IdentifierError::invalid_prefix("bd-", "abc123");
    assert!(
        matches!(err, IdentifierError::InvalidPrefix { prefix: "bd-", value } if value == "abc123")
    );
}

#[test]
fn test_identifier_error_invalid_hex() {
    let err = IdentifierError::invalid_hex("bd-xyz");
    assert!(format!("{err}").contains("bd-xyz"));
}

#[test]
fn test_identifier_error_not_absolute_path() {
    let err = IdentifierError::not_absolute_path("relative/path");
    assert!(format!("{err}").contains("relative/path"));
}

#[test]
fn test_identifier_error_equality() {
    let a = IdentifierError::empty();
    let b = IdentifierError::empty();
    assert_eq!(a, b);
}

#[test]
fn test_identifier_error_clone() {
    let err = IdentifierError::too_long(10, 20);
    let cloned = err.clone();
    assert_eq!(err, cloned);
}
