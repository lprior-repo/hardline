//! Adversarial tests for scp-error
//! Attack vectors: empty input, bad input, boundary, unicode, path traversal,
//! stress, special chars, null bytes, oversized payloads.

use scp_error::Error;
use std::sync::Arc;
use std::thread;

// ============================================================================
// Attack 1: Empty input — every string-accepting variant with empty strings
// ============================================================================
mod attack_empty_input {
    use super::*;

    #[test]
    fn workspace_not_found_empty_string() {
        let err = Error::WorkspaceNotFound("".into());
        assert_eq!(err.to_string(), "Workspace not found: ");
        assert_eq!(err.exit_code(), 10);
    }

    #[test]
    fn session_locked_empty_strings() {
        let err = Error::SessionLocked("".into(), "".into());
        assert_eq!(err.to_string(), "Session '' is locked by ''");
    }

    #[test]
    fn bead_invalid_state_transition_empty() {
        let err = Error::BeadInvalidStateTransition {
            from: "".into(),
            to: "".into(),
        };
        assert_eq!(err.to_string(), "Invalid bead state transition:  -> ");
    }

    #[test]
    fn lock_timeout_zero_values() {
        let err = Error::LockTimeout {
            operation: "".into(),
            timeout_ms: 0,
            retries: 0,
        };
        assert_eq!(
            err.to_string(),
            "Lock acquisition timeout for '' after 0ms (0 retries)"
        );
    }

    #[test]
    fn validation_field_error_all_empty() {
        let err = Error::ValidationFieldError {
            message: "".into(),
            field: "".into(),
            value: None,
        };
        assert_eq!(err.to_string(), "Validation error on '': ");
    }

    #[test]
    fn queue_invalid_position_zero() {
        let err = Error::QueueInvalidPosition(0);
        assert_eq!(err.to_string(), "Invalid queue position: 0");
    }

    #[test]
    fn queue_full_zero() {
        let err = Error::QueueFull(0);
        assert_eq!(err.to_string(), "Queue is full (max: 0)");
    }
}

// ============================================================================
// Attack 2: Bad input — special chars, control chars, null bytes
// ============================================================================
mod attack_bad_input {
    use super::*;

    #[test]
    fn newlines_in_error_messages() {
        let err = Error::Internal("line1\nline2\nline3".into());
        let msg = err.to_string();
        assert!(msg.contains("line1\nline2\nline3"));
    }

    #[test]
    fn tabs_in_error_messages() {
        let err = Error::IoError("path\twith\ttabs".into());
        let msg = err.to_string();
        assert!(msg.contains("path\twith\ttabs"));
    }

    #[test]
    fn unicode_in_error_messages() {
        let err = Error::Internal("错误 数据破損 🚨".into());
        let msg = err.to_string();
        assert!(msg.contains("错误 数据破損 🚨"));
    }

    #[test]
    fn emoji_in_workspace_name() {
        let err = Error::WorkspaceNotFound("🔥workspace".into());
        assert_eq!(err.to_string(), "Workspace not found: 🔥workspace");
    }

    #[test]
    fn null_byte_in_string() {
        let err = Error::Internal("before\0after".into());
        let msg = err.to_string();
        assert!(msg.contains("before\0after"));
    }

    #[test]
    fn backslashes_and_quotes() {
        let err = Error::ConfigInvalid(r#"key = "value\"escaped""#.into());
        let msg = err.to_string();
        assert!(msg.contains(r#"key = "value\"escaped""#));
    }

    #[test]
    fn ansi_escape_sequences() {
        let err = Error::RunnerError("\x1b[31mRed error\x1b[0m".into());
        let msg = err.to_string();
        assert!(msg.contains("\x1b[31mRed error\x1b[0m"));
    }
}

// ============================================================================
// Attack 3: Boundary — huge strings, max values
// ============================================================================
mod attack_boundary {
    use super::*;

    #[test]
    fn very_long_error_message() {
        let long_msg = "x".repeat(100_000);
        let err = Error::Internal(long_msg.clone());
        let msg = err.to_string();
        assert_eq!(msg.len(), 100_000 + 16); // "Internal error: " prefix
    }

    #[test]
    fn huge_queue_position() {
        let err = Error::QueueInvalidPosition(usize::MAX);
        assert_eq!(
            err.to_string(),
            format!("Invalid queue position: {}", usize::MAX)
        );
    }

    #[test]
    fn huge_queue_full_max() {
        let err = Error::QueueFull(usize::MAX);
        assert_eq!(
            err.to_string(),
            format!("Queue is full (max: {})", usize::MAX)
        );
    }

    #[test]
    fn lock_timeout_max_values() {
        let err = Error::LockTimeout {
            operation: "op".into(),
            timeout_ms: u64::MAX,
            retries: usize::MAX,
        };
        let msg = err.to_string();
        assert!(msg.contains(&u64::MAX.to_string()));
        assert!(msg.contains(&usize::MAX.to_string()));
    }

    #[test]
    fn many_beads_blocked() {
        let many = (0..1000)
            .map(|i| format!("bead-{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let err = Error::BeadBlockedBy(many);
        let msg = err.to_string();
        assert!(msg.contains("bead-0"));
        assert!(msg.contains("bead-999"));
    }

    #[test]
    fn serialize_huge_error() {
        let long = "x".repeat(50_000);
        let err = Error::Internal(long);
        let json = serde_json::to_string(&err).expect("should serialize huge error");
        assert!(json.len() > 50_000);
    }
}

// ============================================================================
// Attack 4: Path traversal in string fields
// ============================================================================
mod attack_path_traversal {
    use super::*;

    #[test]
    fn path_traversal_in_workspace_name() {
        let err = Error::WorkspaceNotFound("../../etc/passwd".into());
        let msg = err.to_string();
        assert!(msg.contains("../../etc/passwd"));
    }

    #[test]
    fn path_traversal_in_config_path() {
        let err = Error::ConfigPermission("/etc/shadow".into());
        let msg = err.to_string();
        assert!(msg.contains("/etc/shadow"));
    }

    #[test]
    fn path_traversal_in_repo_url() {
        let err = Error::InvalidRepoUrl("file:///etc/passwd".into());
        let msg = err.to_string();
        assert!(msg.contains("file:///etc/passwd"));
    }

    #[test]
    fn null_byte_in_path() {
        let err = Error::IoError("/tmp/file\0.exe".into());
        let msg = err.to_string();
        assert!(msg.contains("/tmp/file\0.exe"));
    }
}

// ============================================================================
// Attack 5: Wrong state — unusual variant usage patterns
// ============================================================================
mod attack_wrong_state {
    use super::*;

    #[test]
    fn double_call_suggestion() {
        let err = Error::WorkspaceNotFound("ws".into());
        // Calling suggestion twice should return same result
        assert_eq!(err.suggestion(), err.suggestion());
    }

    #[test]
    fn double_call_exit_code() {
        let err = Error::QueueEmpty;
        // exit_code is const fn — should always return same value
        assert_eq!(err.exit_code(), err.exit_code());
    }

    #[test]
    fn display_after_debug() {
        let err = Error::Internal("test".into());
        let _ = format!("{err:?}"); // Debug first
        let display = format!("{err}"); // Then Display — should still work
        assert_eq!(display, "Internal error: test");
    }

    #[test]
    fn clone_semantics() {
        let err = Error::CloneFailed("repo".into());
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
        assert_eq!(err.exit_code(), cloned.exit_code());
    }
}

// ============================================================================
// Attack 6: Stress — concurrent access, many threads
// ============================================================================
mod attack_stress {
    use super::*;

    #[test]
    fn concurrent_error_creation() {
        let errors: Arc<Vec<Error>> = Arc::new(vec![
            Error::Internal("stress".into()),
            Error::QueueEmpty,
            Error::VcsNotInitialized,
            Error::WorkspaceNotFound("ws".into()),
            Error::LockTimeout {
                operation: "lock".into(),
                timeout_ms: 1000,
                retries: 3,
            },
        ]);

        let handles: Vec<_> = (0..100)
            .map(|i| {
                let errs = Arc::clone(&errors);
                thread::spawn(move || {
                    let err = &errs[i % errs.len()];
                    let _ = err.to_string();
                    let _ = err.exit_code();
                    let _ = err.suggestion();
                    let _ = serde_json::to_string(err);
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread should not panic");
        }
    }

    #[test]
    fn concurrent_serialization() {
        let err = Arc::new(Error::LockTimeout {
            operation: "concurrent-op".into(),
            timeout_ms: 5000,
            retries: 10,
        });

        let handles: Vec<_> = (0..50)
            .map(|_| {
                let e = Arc::clone(&err);
                thread::spawn(move || serde_json::to_string(&*e).unwrap())
            })
            .collect();

        for h in handles {
            let json = h.join().unwrap();
            assert!(json.contains("concurrent-op"));
        }
    }
}

// ============================================================================
// Attack 7: Serialization edge cases
// ============================================================================
mod attack_serialization {
    use super::*;

    #[test]
    fn round_trip_json() {
        let original = Error::LockTimeout {
            operation: "test-op".into(),
            timeout_ms: 3000,
            retries: 5,
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Error = serde_json::from_str(&json).unwrap();
        assert_eq!(original.to_string(), parsed.to_string());
        assert_eq!(original.exit_code(), parsed.exit_code());
    }

    #[test]
    fn round_trip_json_simple_variant() {
        let original = Error::QueueEmpty;
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Error = serde_json::from_str(&json).unwrap();
        assert_eq!(original.to_string(), parsed.to_string());
    }

    #[test]
    fn round_trip_json_tuple_variant() {
        let original = Error::WorkspaceNotFound("my-ws".into());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Error = serde_json::from_str(&json).unwrap();
        assert_eq!(original.to_string(), parsed.to_string());
    }

    #[test]
    fn round_trip_json_validation_field_error() {
        let original = Error::ValidationFieldError {
            message: "required".into(),
            field: "email".into(),
            value: Some("test@example.com".into()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Error = serde_json::from_str(&json).unwrap();
        assert_eq!(original.to_string(), parsed.to_string());
    }

    #[test]
    fn yaml_serialization() {
        let err = Error::VcsConflict("src/main.rs".into(), "merge conflict".into());
        let yaml = serde_yaml::to_string(&err).unwrap();
        assert!(!yaml.is_empty());
    }

    #[test]
    fn json_with_special_characters_roundtrip() {
        let original = Error::Internal("hello \"world\" \n\t\r\\".into());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Error = serde_json::from_str(&json).unwrap();
        assert_eq!(original.to_string(), parsed.to_string());
    }

    #[test]
    fn json_with_unicode_roundtrip() {
        let original = Error::AgentNotFound("日本語エージェント 🤖".into());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Error = serde_json::from_str(&json).unwrap();
        assert_eq!(original.to_string(), parsed.to_string());
    }
}
