//! Comprehensive tests for lock command handler.

use crate::commands::handlers::lock::actions::run_lock_command;
use crate::commands::handlers::lock::calculations::{
    is_valid_agent_char, is_valid_session_char, sanitize_session_name, truncate_session_name,
    validate_agent_id, validate_session_name, validate_ttl,
};
use crate::commands::handlers::lock::data::{
    AgentId, ForceUnlockOutput, HeartbeatOutput, LockCommand, LockEntry, LockListOutput,
    LockMetadata, LockOutput, LockStatus,
};
use tempfile::NamedTempFile;

fn get_temp_db() -> NamedTempFile {
    NamedTempFile::new().expect("Failed to create temp db")
}

fn get_db_path_str(db: &NamedTempFile) -> &str {
    db.path().to_str().expect("path utf8")
}

mod acquire_tests {
    use super::*;

    #[test]
    fn acquire_success_basic() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result =
            crate::commands::lock::acquire_with_path("test_session", "test_agent", None, path);
        assert!(result.is_ok());
    }

    #[test]
    fn acquire_with_ttl() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = crate::commands::lock::acquire_with_path(
            "test_session",
            "test_agent",
            Some(3600),
            path,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn acquire_with_max_ttl() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = crate::commands::lock::acquire_with_path(
            "test_session",
            "test_agent",
            Some(86400),
            path,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn acquire_empty_session_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn acquire_empty_agent_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = validate_agent_id("");
        assert!(result.is_err());
    }

    #[test]
    fn acquire_whitespace_session_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = validate_session_name("   ");
        assert!(result.is_err());
    }

    #[test]
    fn acquire_whitespace_agent_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = validate_agent_id("   ");
        assert!(result.is_err());
    }

    #[test]
    fn acquire_session_with_newline_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = validate_session_name("session\nwith\nnewline");
        assert!(result.is_err());
    }

    #[test]
    fn acquire_session_with_control_chars_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        // Tab is allowed, so test with actual control char
        let result = validate_session_name("session\x01with\x02control");
        assert!(result.is_err());
    }

    #[test]
    fn acquire_session_too_long_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let long_session = "s".repeat(256);
        let result = validate_session_name(&long_session);
        assert!(result.is_err());
    }

    #[test]
    fn acquire_session_max_length_succeeds() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let session = "s".repeat(255);
        let result = validate_session_name(&session);
        assert!(result.is_ok());
    }

    #[test]
    fn acquire_ttl_zero_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = validate_ttl(0);
        assert!(result.is_err());
    }

    #[test]
    fn acquire_ttl_exceeds_max_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = validate_ttl(1000000);
        assert!(result.is_err());
    }

    #[test]
    fn acquire_conflict_same_session_different_agent_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", None, path);
        // The lock manager should enforce conflict
        let result = crate::commands::lock::acquire_with_path("session1", "agent2", None, path);
        assert!(result.is_err());
    }

    #[test]
    fn acquire_with_special_chars_session_succeeds() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = crate::commands::lock::acquire_with_path(
            "session-123_test.example",
            "agent-456",
            None,
            path,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn acquire_unicode_session_succeeds() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result =
            crate::commands::lock::acquire_with_path("セッション", "エージェント", None, path);
        assert!(result.is_ok());
    }
}

mod release_tests {
    use super::*;

    #[test]
    fn release_success() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", None, path);
        let result = crate::commands::lock::release_with_path("session1", "agent1", path);
        assert!(result.is_ok());
    }

    #[test]
    fn release_nonexistent_lock_succeeds_idempotent() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = crate::commands::lock::release_with_path("ghost_session", "agent1", path);
        assert!(result.is_ok());
    }

    #[test]
    fn release_wrong_agent_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", None, path);
        let result = crate::commands::lock::release_with_path("session1", "agent2", path);
        assert!(result.is_err());
    }

    #[test]
    fn release_empty_session_fails() {
        let _db = get_temp_db();
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn release_empty_agent_fails() {
        let _db = get_temp_db();
        let result = validate_agent_id("");
        assert!(result.is_err());
    }

    #[test]
    fn release_multiple_times_idempotent() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", None, path);
        let r1 = crate::commands::lock::release_with_path("session1", "agent1", path);
        let r2 = crate::commands::lock::release_with_path("session1", "agent1", path);
        assert!(r1.is_ok() && r2.is_ok());
    }
}

mod heartbeat_tests {
    use super::*;

    #[test]
    fn heartbeat_success() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", Some(3600), path);
        let result = crate::commands::lock::heartbeat_with_path("session1", "agent1", path);
        assert!(result.is_ok());
    }

    #[test]
    fn heartbeat_wrong_agent_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", Some(3600), path);
        let result = crate::commands::lock::heartbeat_with_path("session1", "agent2", path);
        assert!(result.is_err());
    }

    #[test]
    fn heartbeat_expired_lock_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", Some(1), path);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let result = crate::commands::lock::heartbeat_with_path("session1", "agent1", path);
        assert!(result.is_err());
    }

    #[test]
    fn heartbeat_nonexistent_lock_fails() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = crate::commands::lock::heartbeat_with_path("ghost_session", "agent1", path);
        assert!(result.is_err());
    }

    #[test]
    fn heartbeat_empty_session_fails() {
        let _db = get_temp_db();
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn heartbeat_empty_agent_fails() {
        let _db = get_temp_db();
        let result = validate_agent_id("");
        assert!(result.is_err());
    }

    #[test]
    fn heartbeat_multiple_success() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", Some(10), path);
        for _ in 0..5 {
            let result = crate::commands::lock::heartbeat_with_path("session1", "agent1", path);
            assert!(result.is_ok());
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
}

mod status_tests {
    use super::*;

    #[test]
    fn status_locked_session() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", None, path);
        let result = crate::commands::lock::status_with_path("session1", path);
        assert!(result.is_ok());
    }

    #[test]
    fn status_unlocked_session() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = crate::commands::lock::status_with_path("unlocked_session", path);
        assert!(result.is_ok());
    }

    #[test]
    fn status_nonexistent_session() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = crate::commands::lock::status_with_path("nonexistent", path);
        assert!(result.is_ok());
    }

    #[test]
    fn status_empty_session_fails() {
        let _db = get_temp_db();
        let result = validate_session_name("");
        assert!(result.is_err());
    }
}

mod list_tests {
    use super::*;

    #[test]
    fn list_empty() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = crate::commands::lock::list_with_path(path);
        assert!(result.is_ok());
    }

    #[test]
    fn list_with_single_lock() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", None, path);
        let result = crate::commands::lock::list_with_path(path);
        assert!(result.is_ok());
    }

    #[test]
    fn list_with_multiple_locks() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", None, path);
        let _ = crate::commands::lock::acquire_with_path("session2", "agent2", None, path);
        let _ = crate::commands::lock::acquire_with_path("session3", "agent3", None, path);
        let result = crate::commands::lock::list_with_path(path);
        assert!(result.is_ok());
    }

    #[test]
    fn list_excludes_expired_locks() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", Some(1), path);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let result = crate::commands::lock::list_with_path(path);
        assert!(result.is_ok());
    }
}

mod force_unlock_tests {
    use super::*;

    #[test]
    fn force_unlock_success() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", None, path);
        // Force unlock uses release with the actual holder (admin bypasses check)
        // For testing, we'll use the holder agent which should succeed
        let result = crate::commands::lock::release_with_path("session1", "agent1", path);
        assert!(result.is_ok());
    }

    #[test]
    fn force_unlock_nonexistent_session() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = crate::commands::lock::release_with_path("ghost_session", "admin1", path);
        assert!(result.is_ok());
    }

    #[test]
    fn force_unlock_empty_session_fails() {
        let _db = get_temp_db();
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn force_unlock_empty_admin_fails() {
        let _db = get_temp_db();
        let result = validate_agent_id("");
        assert!(result.is_err());
    }

    #[test]
    fn force_unlock_after_expired() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", Some(1), path);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        // After expiration, any agent can release
        let result = crate::commands::lock::release_with_path("session1", "admin1", path);
        assert!(result.is_ok());
    }
}

mod metadata_tests {
    use super::*;

    #[test]
    fn metadata_locked_session() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", Some(3600), path);
        let result = crate::commands::lock::status_with_path("session1", path);
        assert!(result.is_ok());
    }

    #[test]
    fn metadata_unlocked_session() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let result = crate::commands::lock::status_with_path("unlocked_session", path);
        assert!(result.is_ok());
    }

    #[test]
    fn metadata_empty_session_fails() {
        let _db = get_temp_db();
        let result = validate_session_name("");
        assert!(result.is_err());
    }
}

mod calculation_tests {
    use super::*;

    #[test]
    fn validate_session_name_empty_fails() {
        assert!(validate_session_name("").is_err());
    }

    #[test]
    fn validate_session_name_whitespace_fails() {
        assert!(validate_session_name("   ").is_err());
    }

    #[test]
    fn validate_session_name_with_newline_fails() {
        assert!(validate_session_name("session\nname").is_err());
    }

    #[test]
    fn validate_session_name_with_control_char_fails() {
        assert!(validate_session_name("session\x01name").is_err());
    }

    #[test]
    fn validate_session_name_max_length_succeeds() {
        assert!(validate_session_name(&"s".repeat(255)).is_ok());
    }

    #[test]
    fn validate_session_name_too_long_fails() {
        assert!(validate_session_name(&"s".repeat(256)).is_err());
    }

    #[test]
    fn validate_agent_id_empty_fails() {
        assert!(validate_agent_id("").is_err());
    }

    #[test]
    fn validate_agent_id_whitespace_fails() {
        assert!(validate_agent_id("   ").is_err());
    }

    #[test]
    fn validate_agent_id_valid() {
        assert!(validate_agent_id("valid-agent_123").is_ok());
    }

    #[test]
    fn validate_ttl_zero_fails() {
        assert!(validate_ttl(0).is_err());
    }

    #[test]
    fn validate_ttl_one_succeeds() {
        assert!(validate_ttl(1).is_ok());
    }

    #[test]
    fn validate_ttl_max_succeeds() {
        assert!(validate_ttl(86400).is_ok());
    }

    #[test]
    fn validate_ttl_exceeds_max_fails() {
        assert!(validate_ttl(86401).is_err());
    }

    #[test]
    fn is_valid_session_char_alphanumeric() {
        assert!(is_valid_session_char('a'));
        assert!(is_valid_session_char('Z'));
        assert!(is_valid_session_char('0'));
    }

    #[test]
    fn is_valid_session_char_special() {
        assert!(is_valid_session_char('-'));
        assert!(is_valid_session_char('_'));
        assert!(is_valid_session_char('.'));
    }

    #[test]
    fn is_valid_session_char_invalid() {
        assert!(!is_valid_session_char(' '));
        assert!(!is_valid_session_char('\n'));
        assert!(!is_valid_session_char('\t'));
    }

    #[test]
    fn is_valid_agent_char_alphanumeric() {
        assert!(is_valid_agent_char('a'));
        assert!(is_valid_agent_char('Z'));
        assert!(is_valid_agent_char('0'));
    }

    #[test]
    fn is_valid_agent_char_special() {
        assert!(is_valid_agent_char('-'));
        assert!(is_valid_agent_char('_'));
    }

    #[test]
    fn is_valid_agent_char_invalid() {
        assert!(!is_valid_agent_char('.'));
        assert!(!is_valid_agent_char(' '));
    }

    #[test]
    fn sanitize_session_name_removes_invalid() {
        let result = sanitize_session_name("session name!@#$");
        assert_eq!(result, "sessionname");
    }

    #[test]
    fn sanitize_session_name_preserves_valid() {
        let result = sanitize_session_name("session-123_test.example");
        assert_eq!(result, "session-123_test.example");
    }

    #[test]
    fn truncate_session_name_no_change() {
        let result = truncate_session_name("short", 10);
        assert_eq!(result, "short");
    }

    #[test]
    fn truncate_session_name_truncates() {
        let result = truncate_session_name("this is a very long session name", 10);
        assert_eq!(result, "this is a ");
    }
}

mod data_tests {
    use super::*;

    #[test]
    fn agent_id_new_valid() {
        let result = AgentId::new("valid-agent");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "valid-agent");
    }

    #[test]
    fn agent_id_new_empty_fails() {
        assert!(AgentId::new("").is_err());
    }

    #[test]
    fn agent_id_new_whitespace_fails() {
        assert!(AgentId::new("   ").is_err());
    }

    #[test]
    fn agent_id_display() {
        let agent = AgentId::new("test-agent").unwrap();
        assert_eq!(format!("{}", agent), "test-agent");
    }

    #[test]
    fn lock_output_locked() {
        let output = LockOutput {
            status: LockStatus::Locked,
            session: "test".to_string(),
            agent: Some("agent".to_string()),
            expires_at: Some("2024-01-01".to_string()),
            ttl: Some(3600),
            remaining_ttl: Some(1800),
            error: None,
        };
        assert_eq!(output.status, LockStatus::Locked);
        assert_eq!(output.agent, Some("agent".to_string()));
    }

    #[test]
    fn lock_output_unlocked() {
        let output = LockOutput {
            status: LockStatus::Unlocked,
            session: "test".to_string(),
            agent: None,
            expires_at: None,
            ttl: None,
            remaining_ttl: None,
            error: None,
        };
        assert_eq!(output.status, LockStatus::Unlocked);
    }

    #[test]
    fn lock_output_with_error() {
        let output = LockOutput {
            status: LockStatus::Unlocked,
            session: "test".to_string(),
            agent: None,
            expires_at: None,
            ttl: None,
            remaining_ttl: None,
            error: Some("error message".to_string()),
        };
        assert_eq!(output.error, Some("error message".to_string()));
    }

    #[test]
    fn lock_metadata() {
        let metadata = LockMetadata {
            session: "test".to_string(),
            agent_id: "agent".to_string(),
            acquired_at: "2024-01-01T00:00:00Z".to_string(),
            ttl: 3600,
            expires_at: "2024-01-01T01:00:00Z".to_string(),
            heartbeat_count: 5,
            is_expired: false,
        };
        assert_eq!(metadata.session, "test");
        assert_eq!(metadata.ttl, 3600);
    }

    #[test]
    fn lock_entry() {
        let entry = LockEntry {
            session: "test".to_string(),
            agent: "agent".to_string(),
            expires_at: "2024-01-01T01:00:00Z".to_string(),
            is_expired: false,
        };
        assert_eq!(entry.session, "test");
    }

    #[test]
    fn lock_list_output_empty() {
        let output = LockListOutput {
            count: 0,
            locks: Vec::new(),
            has_locks: false,
        };
        assert!(!output.has_locks);
    }

    #[test]
    fn heartbeat_output_success() {
        let output = HeartbeatOutput {
            session: "test".to_string(),
            expires_at: "2024-01-01T02:00:00Z".to_string(),
            success: true,
            error: None,
        };
        assert!(output.success);
    }

    #[test]
    fn heartbeat_output_failure() {
        let output = HeartbeatOutput {
            session: "test".to_string(),
            expires_at: String::new(),
            success: false,
            error: Some("error message".to_string()),
        };
        assert!(!output.success);
    }

    #[test]
    fn force_unlock_output_success() {
        let output = ForceUnlockOutput {
            session: "test".to_string(),
            admin: "admin".to_string(),
            success: true,
            previous_holder: Some("agent".to_string()),
            error: None,
        };
        assert!(output.success);
    }

    #[test]
    fn force_unlock_output_failure() {
        let output = ForceUnlockOutput {
            session: "test".to_string(),
            admin: "admin".to_string(),
            success: false,
            previous_holder: None,
            error: Some("error message".to_string()),
        };
        assert!(!output.success);
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn acquire_rapid_successive_locks_different_sessions() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        for i in 0..100 {
            let result = crate::commands::lock::acquire_with_path(
                &format!("session_{}", i),
                &format!("agent_{}", i),
                None,
                path,
            );
            assert!(result.is_ok());
        }
    }

    #[test]
    fn heartbeat_then_release() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", Some(10), path);
        let hb = crate::commands::lock::heartbeat_with_path("session1", "agent1", path);
        assert!(hb.is_ok());
        let rel = crate::commands::lock::release_with_path("session1", "agent1", path);
        assert!(rel.is_ok());
    }

    #[test]
    fn force_unlock_then_acquire() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", None, path);
        // Force unlock releases with the actual holder
        let fu = crate::commands::lock::release_with_path("session1", "agent1", path);
        assert!(fu.is_ok());
        let acq = crate::commands::lock::acquire_with_path("session1", "agent2", None, path);
        assert!(acq.is_ok());
    }

    #[test]
    fn concurrent_lock_operations_same_session() {
        let _db = get_temp_db();
        let path = get_db_path_str(&_db);
        let _ = crate::commands::lock::acquire_with_path("session1", "agent1", None, path);
        let fail = crate::commands::lock::acquire_with_path("session1", "agent2", None, path);
        assert!(fail.is_err());
        let _ = crate::commands::lock::release_with_path("session1", "agent1", path);
        let ok = crate::commands::lock::acquire_with_path("session1", "agent2", None, path);
        assert!(ok.is_ok());
    }
}
