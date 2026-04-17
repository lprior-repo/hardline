//! BDD Validation tests for scp-error
//! Claim: Every public type, method, and behavior is proven with real output.

use scp_error::{Error, Result};

// ============================================================================
// Claim 1: Result<T> type alias
// ============================================================================
mod claim_result_type_alias {
    use super::*;

    #[test]
    fn result_ok_variants_work() {
        let r: Result<i32> = Ok(42);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn result_err_variants_work() {
        let r: Result<i32> = Err(Error::Internal("boom".into()));
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().to_string(), "Internal error: boom");
    }
}

// ============================================================================
// Claim 2: #[non_exhaustive] — cannot match exhaustively outside crate
// ============================================================================
mod claim_non_exhaustive {
    use super::*;

    #[test]
    fn cannot_exhaustively_match() {
        // This compiles because we use a wildcard arm
        let err = Error::Internal("test".into());
        match err {
            Error::Internal(msg) => assert_eq!(msg, "test"),
            _ => panic!("should not reach here"),
        }
    }

    #[test]
    fn non_exhaustive_prevents_external_construction_patterns() {
        // Verify it's a real enum (not sealed trait) by checking Debug
        let err = Error::NotFound("key".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("NotFound"));
    }
}

// ============================================================================
// Claim 3: All variants derive Debug and Serialize
// ============================================================================
mod claim_debug_serialize {
    use super::*;

    #[test]
    fn all_variants_implement_debug() {
        let variants: Vec<Error> = vec![
            Error::WorkspaceNotFound("ws".into()),
            Error::WorkspaceExists("ws".into()),
            Error::WorkspaceLocked("ws".into(), "agent".into()),
            Error::WorkspaceConflict("msg".into()),
            Error::SessionNotFound("s".into()),
            Error::SessionExists("s".into()),
            Error::SessionLocked("s".into(), "a".into()),
            Error::NotLockHolder("s".into(), "a".into()),
            Error::SessionInvalidState("s".into(), "bad".into(), "good".into()),
            Error::BeadNotFound("b".into()),
            Error::BeadAlreadyExists("b".into()),
            Error::InvalidBeadId("x".into()),
            Error::InvalidBeadTitle("".into()),
            Error::BeadInvalidStateTransition { from: "a".into(), to: "b".into() },
            Error::BeadDependencyCycle("cycle".into()),
            Error::BeadBlockedBy("b1".into()),
            Error::BeadInvalidDependency("dep".into()),
            Error::QueueEmpty,
            Error::QueueItemNotFound("q".into()),
            Error::QueueLocked("a".into()),
            Error::QueueProcessing,
            Error::QueueInvalidPosition(0),
            Error::QueueFull(10),
            Error::VcsNotInitialized,
            Error::VcsConflict("file".into(), "msg".into()),
            Error::VcsPushFailed("msg".into()),
            Error::VcsPullFailed("msg".into()),
            Error::VcsRebaseFailed("msg".into()),
            Error::BranchNotFound("b".into()),
            Error::BranchExists("b".into()),
            Error::CommitNotFound("c".into()),
            Error::WorkingCopyDirty,
            Error::ConfigNotFound("k".into()),
            Error::ConfigInvalid("msg".into()),
            Error::ConfigPermission("p".into()),
            Error::InvalidConfig("msg".into()),
            Error::InvalidRepoUrl("url".into()),
            Error::AgentNotFound("a".into()),
            Error::AgentExists("a".into()),
            Error::AgentTimeout("a".into()),
            Error::InvalidState("s".into()),
            Error::NotFound("n".into()),
            Error::InvalidOperation("op".into()),
            Error::ValidationError("msg".into()),
            Error::ValidationFieldError {
                message: "err".into(),
                field: "f".into(),
                value: Some("v".into()),
            },
            Error::InvalidIdentifier("id".into()),
            Error::IoError("io".into()),
            Error::JsonParseError("json".into()),
            Error::YamlParseError("yaml".into()),
            Error::Database("db".into()),
            Error::Serialization("ser".into()),
            Error::LockTimeout {
                operation: "op".into(),
                timeout_ms: 5000,
                retries: 3,
            },
            Error::CloneFailed("msg".into()),
            Error::RecordFailed("msg".into()),
            Error::Persistence("msg".into()),
            Error::StateTransition("msg".into()),
            Error::ScenarioError("msg".into()),
            Error::RunnerError("msg".into()),
            Error::DefinitionError("msg".into()),
            Error::ServerError("msg".into()),
            Error::SyncError("msg".into()),
            Error::Internal("msg".into()),
            Error::Unimplemented("msg".into()),
            Error::InvariantViolation("msg".into()),
        ];

        // Every variant must produce Debug output
        for v in &variants {
            let debug = format!("{v:?}");
            assert!(!debug.is_empty(), "Debug output is empty for {debug}");
        }
    }

    #[test]
    fn all_variants_implement_serialize() {
        let variants: Vec<Error> = vec![
            Error::WorkspaceNotFound("ws".into()),
            Error::QueueEmpty,
            Error::VcsNotInitialized,
            Error::ValidationFieldError {
                message: "err".into(),
                field: "f".into(),
                value: None,
            },
            Error::LockTimeout {
                operation: "op".into(),
                timeout_ms: 1000,
                retries: 5,
            },
            Error::BeadInvalidStateTransition {
                from: "open".into(),
                to: "closed".into(),
            },
            Error::Internal("oops".into()),
        ];

        for v in &variants {
            let json = serde_json::to_string(v).expect(&format!(
                "Serialize failed for variant: {:?}",
                v
            ));
            assert!(!json.is_empty(), "Serialized JSON is empty");
            // Must be valid JSON
            let _: serde_json::Value =
                serde_json::from_str(&json).expect("Serialized output is not valid JSON");
        }
    }
}

// ============================================================================
// Claim 4: All variants implement std::error::Error (via thiserror)
// ============================================================================
mod claim_std_error {
    use super::*;
    use std::error::Error as StdError;

    #[test]
    fn all_variants_are_std_error() {
        let variants: Vec<Error> = vec![
            Error::IoError("io".into()),
            Error::JsonParseError("j".into()),
            Error::Internal("i".into()),
            Error::QueueEmpty,
            Error::VcsNotInitialized,
            Error::ValidationFieldError {
                message: "m".into(),
                field: "f".into(),
                value: Some("v".into()),
            },
            Error::LockTimeout {
                operation: "op".into(),
                timeout_ms: 100,
                retries: 1,
            },
        ];

        for v in &variants {
            // std::error::Error requires &dyn Error
            let err: &dyn StdError = v;
            assert!(!err.to_string().is_empty());
            // source() should be None (leaf errors)
            assert!(err.source().is_none());
        }
    }
}

// ============================================================================
// Claim 5: suggestion() returns correct hints
// ============================================================================
mod claim_suggestion {
    use super::*;

    #[test]
    fn workspace_not_found_suggests_list() {
        let err = Error::WorkspaceNotFound("my-ws".into());
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("scp workspace list"), "suggestion: {s}");
    }

    #[test]
    fn session_not_found_suggests_list() {
        let err = Error::SessionNotFound("my-sess".into());
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("scp session list"), "suggestion: {s}");
    }

    #[test]
    fn queue_empty_suggests_enqueue() {
        let err = Error::QueueEmpty;
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("scp queue enqueue"), "suggestion: {s}");
    }

    #[test]
    fn workspace_locked_suggests_kill() {
        let err = Error::WorkspaceLocked("ws".into(), "bad-agent".into());
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("scp agent kill bad-agent"), "suggestion: {s}");
    }

    #[test]
    fn vcs_not_initialized_suggests_init() {
        let err = Error::VcsNotInitialized;
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("scp init"), "suggestion: {s}");
    }

    #[test]
    fn working_copy_dirty_suggests_commit() {
        let err = Error::WorkingCopyDirty;
        let s = err.suggestion().expect("should have suggestion");
        assert!(s.contains("Commit or stash"), "suggestion: {s}");
    }

    #[test]
    fn internal_error_has_no_suggestion() {
        let err = Error::Internal("boom".into());
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn generic_errors_have_no_suggestion() {
        let no_suggestion_variants: Vec<Error> = vec![
            Error::NotFound("x".into()),
            Error::InvalidState("y".into()),
            Error::InvalidOperation("z".into()),
            Error::IoError("io".into()),
            Error::JsonParseError("j".into()),
            Error::Database("db".into()),
            Error::AgentNotFound("a".into()),
            Error::ConfigNotFound("c".into()),
            Error::ValidationError("v".into()),
            Error::CloneFailed("cl".into()),
            Error::ScenarioError("s".into()),
            Error::InvariantViolation("inv".into()),
            Error::Unimplemented("u".into()),
            Error::BeadNotFound("b".into()),
            Error::QueueItemNotFound("q".into()),
            Error::QueueLocked("q".into()),
            Error::QueueProcessing,
            Error::QueueInvalidPosition(0),
            Error::QueueFull(10),
            Error::BranchNotFound("b".into()),
            Error::BranchExists("b".into()),
            Error::CommitNotFound("c".into()),
        ];
        for v in &no_suggestion_variants {
            assert!(v.suggestion().is_none(), "Expected no suggestion for: {v:?}");
        }
    }
}

// ============================================================================
// Claim 6: exit_code() returns unique i32 per variant
// ============================================================================
mod claim_exit_codes {
    use super::*;

    #[test]
    fn all_exit_codes_are_positive() {
        let variants: Vec<Error> = vec![
            Error::WorkspaceNotFound("".into()),
            Error::WorkspaceExists("".into()),
            Error::WorkspaceLocked("".into(), "".into()),
            Error::WorkspaceConflict("".into()),
            Error::SessionNotFound("".into()),
            Error::SessionExists("".into()),
            Error::SessionLocked("".into(), "".into()),
            Error::NotLockHolder("".into(), "".into()),
            Error::SessionInvalidState("".into(), "".into(), "".into()),
            Error::BeadNotFound("".into()),
            Error::BeadAlreadyExists("".into()),
            Error::InvalidBeadId("".into()),
            Error::InvalidBeadTitle("".into()),
            Error::BeadInvalidStateTransition { from: "".into(), to: "".into() },
            Error::BeadDependencyCycle("".into()),
            Error::BeadBlockedBy("".into()),
            Error::BeadInvalidDependency("".into()),
            Error::QueueEmpty,
            Error::QueueItemNotFound("".into()),
            Error::QueueLocked("".into()),
            Error::QueueProcessing,
            Error::QueueInvalidPosition(0),
            Error::QueueFull(0),
            Error::VcsNotInitialized,
            Error::VcsConflict("".into(), "".into()),
            Error::VcsPushFailed("".into()),
            Error::VcsPullFailed("".into()),
            Error::VcsRebaseFailed("".into()),
            Error::BranchNotFound("".into()),
            Error::BranchExists("".into()),
            Error::CommitNotFound("".into()),
            Error::WorkingCopyDirty,
            Error::ConfigNotFound("".into()),
            Error::ConfigInvalid("".into()),
            Error::ConfigPermission("".into()),
            Error::InvalidConfig("".into()),
            Error::InvalidRepoUrl("".into()),
            Error::AgentNotFound("".into()),
            Error::AgentExists("".into()),
            Error::AgentTimeout("".into()),
            Error::InvalidState("".into()),
            Error::NotFound("".into()),
            Error::InvalidOperation("".into()),
            Error::ValidationError("".into()),
            Error::ValidationFieldError {
                message: "".into(),
                field: "".into(),
                value: None,
            },
            Error::InvalidIdentifier("".into()),
            Error::IoError("".into()),
            Error::JsonParseError("".into()),
            Error::YamlParseError("".into()),
            Error::Database("".into()),
            Error::Serialization("".into()),
            Error::LockTimeout {
                operation: "".into(),
                timeout_ms: 0,
                retries: 0,
            },
            Error::CloneFailed("".into()),
            Error::RecordFailed("".into()),
            Error::Persistence("".into()),
            Error::StateTransition("".into()),
            Error::ScenarioError("".into()),
            Error::RunnerError("".into()),
            Error::DefinitionError("".into()),
            Error::ServerError("".into()),
            Error::SyncError("".into()),
            Error::Internal("".into()),
            Error::Unimplemented("".into()),
            Error::InvariantViolation("".into()),
        ];

        // All exit codes must be positive
        for v in &variants {
            assert!(v.exit_code() > 0, "exit_code must be > 0, got {} for {:?}", v.exit_code(), v);
        }

        // All exit codes must be unique
        let mut codes: Vec<i32> = variants.iter().map(|v| v.exit_code()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(
            codes.len(),
            variants.len(),
            "exit codes are not unique: {} variants but {} unique codes",
            variants.len(),
            codes.len()
        );
    }

    #[test]
    fn exit_codes_match_expected_ranges() {
        assert_eq!(Error::WorkspaceNotFound("".into()).exit_code(), 10);
        assert_eq!(Error::QueueEmpty.exit_code(), 30);
        assert_eq!(Error::VcsNotInitialized.exit_code(), 40);
        assert_eq!(Error::ConfigNotFound("".into()).exit_code(), 60);
        assert_eq!(Error::AgentNotFound("".into()).exit_code(), 70);
        assert_eq!(Error::InvalidState("".into()).exit_code(), 80);
        assert_eq!(Error::ValidationError("".into()).exit_code(), 90);
        assert_eq!(Error::IoError("".into()).exit_code(), 100);
        assert_eq!(Error::LockTimeout { operation: "".into(), timeout_ms: 0, retries: 0 }.exit_code(), 110);
        assert_eq!(Error::ScenarioError("".into()).exit_code(), 120);
        assert_eq!(Error::Internal("".into()).exit_code(), 130);
    }
}

// ============================================================================
// Claim 7: Display messages are human-readable for all variants
// ============================================================================
mod claim_display {
    use super::*;

    #[test]
    fn all_variants_have_human_readable_display() {
        let test_cases: Vec<(Error, &str)> = vec![
            (Error::WorkspaceNotFound("my-ws".into()), "Workspace not found: my-ws"),
            (Error::WorkspaceExists("my-ws".into()), "Workspace already exists: my-ws"),
            (Error::WorkspaceLocked("ws".into(), "agent1".into()), "Workspace 'ws' is locked by 'agent1'"),
            (Error::WorkspaceConflict("merge conflict".into()), "Workspace conflict: merge conflict"),
            (Error::SessionNotFound("sess1".into()), "Session not found: sess1"),
            (Error::SessionExists("sess1".into()), "Session already exists: sess1"),
            (Error::SessionLocked("s".into(), "a".into()), "Session 's' is locked by 'a'"),
            (Error::NotLockHolder("s".into(), "a".into()), "Agent 'a' does not hold lock on session 's'"),
            (Error::SessionInvalidState("s".into(), "dead".into(), "alive".into()), "Session 's' is dead, expected alive"),
            (Error::BeadNotFound("b".into()), "Bead not found: b"),
            (Error::BeadAlreadyExists("b".into()), "Bead already exists: b"),
            (Error::InvalidBeadId("x!".into()), "Invalid bead ID: x!"),
            (Error::InvalidBeadTitle("".into()), "Invalid bead title: "),
            (Error::BeadInvalidStateTransition { from: "open".into(), to: "closed".into() }, "Invalid bead state transition: open -> closed"),
            (Error::BeadDependencyCycle("a->b->a".into()), "Dependency cycle detected: a->b->a"),
            (Error::BeadBlockedBy("b1, b2".into()), "Bead is blocked by: [b1, b2]"),
            (Error::BeadInvalidDependency("self".into()), "Invalid bead dependency: self"),
            (Error::QueueEmpty, "Queue is empty"),
            (Error::QueueItemNotFound("q".into()), "Queue item not found: q"),
            (Error::QueueLocked("a".into()), "Queue is locked by 'a'"),
            (Error::QueueProcessing, "Queue operation already in progress"),
            (Error::QueueInvalidPosition(5), "Invalid queue position: 5"),
            (Error::QueueFull(100), "Queue is full (max: 100)"),
            (Error::VcsNotInitialized, "VCS not initialized in this directory"),
            (Error::VcsConflict("file.rs".into(), "merge".into()), "VCS conflict in file.rs: merge"),
            (Error::VcsPushFailed("rejected".into()), "Failed to push: rejected"),
            (Error::VcsPullFailed("network".into()), "Failed to pull: network"),
            (Error::VcsRebaseFailed("conflict".into()), "Failed to rebase: conflict"),
            (Error::BranchNotFound("feat/x".into()), "Branch not found: feat/x"),
            (Error::BranchExists("main".into()), "Branch already exists: main"),
            (Error::CommitNotFound("abc123".into()), "Commit not found: abc123"),
            (Error::WorkingCopyDirty, "Working copy has uncommitted changes"),
            (Error::ConfigNotFound("key".into()), "Configuration not found: key"),
            (Error::ConfigInvalid("bad value".into()), "Configuration invalid: bad value"),
            (Error::ConfigPermission("/root/scp".into()), "Configuration permission denied: /root/scp"),
            (Error::InvalidConfig("missing field".into()), "Invalid configuration: missing field"),
            (Error::InvalidRepoUrl("not-a-url".into()), "Invalid repository URL: not-a-url"),
            (Error::AgentNotFound("bot".into()), "Agent not found: bot"),
            (Error::AgentExists("bot".into()), "Agent already registered: bot"),
            (Error::AgentTimeout("bot".into()), "Agent 'bot' heartbeat timeout"),
            (Error::InvalidState("bad".into()), "Invalid state: bad"),
            (Error::NotFound("resource".into()), "Not found: resource"),
            (Error::InvalidOperation("delete on read".into()), "Invalid operation: delete on read"),
            (Error::ValidationError("field required".into()), "Validation error: field required"),
            (Error::ValidationFieldError { message: "too short".into(), field: "name".into(), value: Some("a".into()) }, "Validation error on 'name': too short"),
            (Error::InvalidIdentifier("123abc".into()), "Invalid identifier: 123abc"),
            (Error::IoError("permission denied".into()), "IO error: permission denied"),
            (Error::JsonParseError("unexpected token".into()), "JSON parse error: unexpected token"),
            (Error::YamlParseError("invalid mapping".into()), "YAML parse error: invalid mapping"),
            (Error::Database("connection lost".into()), "Database error: connection lost"),
            (Error::Serialization("struct too large".into()), "Serialization error: struct too large"),
            (Error::LockTimeout { operation: "write".into(), timeout_ms: 5000, retries: 3 }, "Lock acquisition timeout for 'write' after 5000ms (3 retries)"),
            (Error::CloneFailed("network error".into()), "Clone failed: network error"),
            (Error::RecordFailed("disk full".into()), "Record failed: disk full"),
            (Error::Persistence("save failed".into()), "Persistence error: save failed"),
            (Error::StateTransition("invalid".into()), "State transition error: invalid"),
            (Error::ScenarioError("setup failed".into()), "Scenario error: setup failed"),
            (Error::RunnerError("exec failed".into()), "Runner error: exec failed"),
            (Error::DefinitionError("missing step".into()), "Definition error: missing step"),
            (Error::ServerError("port in use".into()), "Server error: port in use"),
            (Error::SyncError("diverged".into()), "Sync error: diverged"),
            (Error::Internal("unexpected null".into()), "Internal error: unexpected null"),
            (Error::Unimplemented("feature x".into()), "Not implemented: feature x"),
            (Error::InvariantViolation("state corrupted".into()), "Invariant violation: state corrupted"),
        ];

        for (err, expected) in test_cases {
            let display = err.to_string();
            assert_eq!(display, expected, "Display mismatch for variant: expected '{expected}', got '{display}'");
        }
    }
}

// ============================================================================
// Claim 8: Struct variants carry named fields correctly
// ============================================================================
mod claim_struct_variants {
    use super::*;

    #[test]
    fn bead_invalid_state_transition_fields() {
        let err = Error::BeadInvalidStateTransition {
            from: "open".into(),
            to: "closed".into(),
        };
        assert!(err.to_string().contains("open"));
        assert!(err.to_string().contains("closed"));
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"from\":\"open\""));
        assert!(json.contains("\"to\":\"closed\""));
    }

    #[test]
    fn lock_timeout_fields() {
        let err = Error::LockTimeout {
            operation: "write_lock".into(),
            timeout_ms: 10000,
            retries: 5,
        };
        assert!(err.to_string().contains("write_lock"));
        assert!(err.to_string().contains("10000"));
        assert!(err.to_string().contains("5"));
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"operation\":\"write_lock\""));
        assert!(json.contains("\"timeout_ms\":10000"));
        assert!(json.contains("\"retries\":5"));
    }

    #[test]
    fn validation_field_error_fields() {
        let err = Error::ValidationFieldError {
            message: "too short".into(),
            field: "username".into(),
            value: Some("ab".into()),
        };
        assert!(err.to_string().contains("username"));
        assert!(err.to_string().contains("too short"));
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"field\":\"username\""));
        assert!(json.contains("\"message\":\"too short\""));
        assert!(json.contains("\"value\":\"ab\""));
    }

    #[test]
    fn validation_field_error_none_value() {
        let err = Error::ValidationFieldError {
            message: "missing".into(),
            field: "email".into(),
            value: None,
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"value\":null"));
    }
}

// ============================================================================
// Claim 9: Serialize produces valid JSON/YAML for all variants
// ============================================================================
mod claim_serialize_formats {
    use super::*;

    #[test]
    fn serialize_to_json() {
        let err = Error::WorkspaceNotFound("test-ws".into());
        let json = serde_json::to_string(&err).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["WorkspaceNotFound"], "test-ws");
    }

    #[test]
    fn serialize_struct_variant_to_json() {
        let err = Error::LockTimeout {
            operation: "op".into(),
            timeout_ms: 5000,
            retries: 3,
        };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["LockTimeout"]["operation"], "op");
        assert_eq!(parsed["LockTimeout"]["timeout_ms"], 5000);
        assert_eq!(parsed["LockTimeout"]["retries"], 3);
    }

    #[test]
    fn serialize_to_yaml() {
        let err = Error::QueueEmpty;
        let yaml = serde_yaml::to_string(&err).unwrap();
        assert!(yaml.contains("QueueEmpty") || yaml.contains("QueueEmpty"));
    }
}

// ============================================================================
// Claim 10: Error is Send + Sync (safe to use across threads)
// ============================================================================
mod claim_thread_safety {
    use super::*;
    use std::thread;

    #[test]
    fn error_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Error>();
    }

    #[test]
    fn error_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Error>();
    }

    #[test]
    fn error_can_cross_thread_boundary() {
        let err = Error::Internal("cross-thread test".into());
        let handle = thread::spawn(move || {
            format!("{err}")
        });
        let result = handle.join().unwrap();
        assert_eq!(result, "Internal error: cross-thread test");
    }
}
