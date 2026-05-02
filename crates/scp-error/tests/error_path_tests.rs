//! Error path coverage tests for scp-error
//!
//! Tests cover:
//! - Error chaining via `From<std::io::Error>`
//! - `std::error::Error::source()` propagation
//! - Display/Debug formatting for all variants
//! - No panic in error constructors with edge cases
//! - Error constructor invariants

use std::error::Error as StdError;

use scp_error::{Error, ErrorCategory, ErrorFix, FixRisk, Result};

/// Helper: build every Error variant for exhaustive iteration.
fn all_errors() -> Vec<Error> {
    vec![
        Error::WorkspaceNotFound("ws".into()),
        Error::WorkspaceExists("ws".into()),
        Error::WorkspaceLocked("ws".into(), "agent".into()),
        Error::WorkspaceConflict("msg".into()),
        Error::SessionNotFound("s".into()),
        Error::SessionExists("s".into()),
        Error::SessionLocked("s".into(), "agent".into()),
        Error::NotLockHolder("s".into(), "agent".into()),
        Error::SessionInvalidState("s".into(), "old".into(), "new".into()),
        Error::BeadNotFound("b".into()),
        Error::BeadAlreadyExists("b".into()),
        Error::InvalidBeadId("b".into()),
        Error::InvalidBeadTitle("".into()),
        Error::BeadInvalidStateTransition {
            from: "a".into(),
            to: "b".into(),
        },
        Error::BeadDependencyCycle("b".into()),
        Error::BeadBlockedBy("b".into()),
        Error::BeadInvalidDependency("b".into()),
        Error::QueueEmpty,
        Error::QueueItemNotFound("q".into()),
        Error::QueueLocked("agent".into()),
        Error::QueueProcessing,
        Error::QueueInvalidPosition(99),
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
        Error::StackNotFound("stack".into()),
        Error::StackOrphaned("parent".into()),
        Error::StackCyclicDependency,
        Error::StackInvalidState("bad".into()),
        Error::StackPrNotFound("pr".into()),
        Error::GitHubAuthFailed("fail".into()),
        Error::GitHubTokenExpired,
        Error::GitHubRateLimited("60s".into()),
        Error::GitHubPrClosed("123".into()),
        Error::GitHubPrNotFound("123".into()),
        Error::GitHubApiError {
            status: 502,
            message: "bad gateway".into(),
        },
        Error::GitHubCiFailed(vec!["ci".into()]),
        Error::SnapshotNotFound("snap".into()),
        Error::SnapshotCorrupted("bad".into()),
        Error::SnapshotExpired("old".into()),
        Error::SnapshotLimitExceeded("max".into()),
        Error::SnapshotRestoreFailed("err".into()),
        Error::ConfigNotFound("k".into()),
        Error::ConfigInvalid("msg".into()),
        Error::ConfigPermission("k".into()),
        Error::InvalidConfig("msg".into()),
        Error::InvalidRepoUrl("url".into()),
        Error::AgentNotFound("a".into()),
        Error::AgentExists("a".into()),
        Error::AgentTimeout("a".into()),
        Error::InvalidState("msg".into()),
        Error::NotFound("res".into()),
        Error::InvalidOperation("op".into()),
        Error::ValidationError("msg".into()),
        Error::ValidationFieldError {
            message: "m".into(),
            field: "f".into(),
            value: Some("v".into()),
        },
        Error::InvalidIdentifier("id".into()),
        Error::IoError("msg".into()),
        Error::JsonParseError("msg".into()),
        Error::YamlParseError("msg".into()),
        Error::Database("msg".into()),
        Error::Serialization("msg".into()),
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
        Error::Unimplemented("feat".into()),
        Error::InvariantViolation("msg".into()),
    ]
}

// ============================================================================
// ERROR CHAINING: From<std::io::Error> conversion
// ============================================================================

mod error_chaining {
    use super::*;

    #[test]
    fn from_io_error_kind_notfound() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::IoError(_)));
        let display = err.to_string();
        assert!(display.contains("file not found"));
    }

    #[test]
    fn from_io_error_kind_permission_denied() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "access denied: /etc/passwd",
        );
        let err: Error = io_err.into();
        let msg = match err {
            Error::IoError(s) => s,
            _ => panic!("expected IoError"),
        };
        assert!(msg.contains("access denied"));
    }

    #[test]
    fn from_io_error_kind_connection_refused() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "port 8080 unreachable",
        );
        let err: Error = io_err.into();
        assert!(matches!(err, Error::IoError(_)));
        assert!(err.to_string().contains("port 8080 unreachable"));
    }

    #[test]
    fn from_io_error_preserves_kind_in_message() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "destination already exists",
        );
        let err: Error = io_err.into();
        let msg = match err {
            Error::IoError(s) => s,
            _ => panic!("expected IoError"),
        };
        assert!(msg.contains("destination already exists"));
    }

    #[test]
    fn io_error_roundtrip_via_result() {
        fn fallible() -> Result<String> {
            std::fs::read_to_string("/nonexistent/path/nowhere")?;
            Ok("won't reach".into())
        }
        let result = fallible();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::IoError(_)));
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn io_error_conversion_preserves_exit_code() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected EOF during read",
        );
        let err: Error = io_err.into();
        assert_eq!(err.exit_code(), 130); // IoError = 130
    }

    #[test]
    fn io_error_conversion_preserves_numeric_code() {
        let io_err =
            std::io::Error::new(std::io::ErrorKind::WriteZero, "write zero byte at offset");
        let err: Error = io_err.into();
        assert_eq!(err.numeric_code(), 9501); // IoError = 9501
    }

    #[test]
    fn io_error_conversion_preserves_code_string() {
        let io_err = std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid utf8 path");
        let err: Error = io_err.into();
        assert_eq!(err.code(), "IO_ERROR");
    }

    #[test]
    fn io_error_conversion_preserves_category() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "custom io error");
        let err: Error = io_err.into();
        assert_eq!(err.category(), ErrorCategory::Internal);
    }
}

// ============================================================================
// SOURCE ERROR PROPAGATION
// ============================================================================

mod source_propagation {
    use super::*;

    #[test]
    fn source_is_none_for_all_leaf_variants() {
        for err in all_errors() {
            let source = StdError::source(&err);
            assert!(
                source.is_none(),
                "Expected no source for {:?} (variant has no wrapped error)",
                err
            );
        }
    }

    #[test]
    fn source_chain_depth_zero() {
        let err = Error::Internal("top level error".into());
        assert!(StdError::source(&err).is_none());

        let err = Error::QueueEmpty;
        assert!(StdError::source(&err).is_none());

        let err = Error::VcsNotInitialized;
        assert!(StdError::source(&err).is_none());
    }

    #[test]
    fn source_is_none_after_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file gone");
        let err: Error = io_err.into();
        // The From implementation wraps the io error's message, not the error itself
        // So source() returns None because Error::IoError doesn't store the original
        assert!(StdError::source(&err).is_none());
    }

    #[test]
    fn provide_error_as_source_in_custom_wrapper() {
        #[derive(Debug)]
        struct WrapperError {
            inner: Error,
        }
        impl std::fmt::Display for WrapperError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "wrapper: {}", self.inner)
            }
        }
        impl StdError for WrapperError {
            fn source(&self) -> Option<&(dyn StdError + 'static)> {
                Some(&self.inner)
            }
        }

        let inner = Error::WorkspaceNotFound("my-ws".into());
        let wrapper = WrapperError { inner };
        let source = StdError::source(&wrapper);
        assert!(source.is_some());
        let source_err = source.unwrap();
        assert!(source_err.to_string().contains("my-ws"));
    }
}

// ============================================================================
// DISPLAY/DEBUG FORMATTING VERIFICATION
// ============================================================================

mod display_debug_formatting {
    use super::*;

    #[test]
    fn display_all_variants_non_empty() {
        for err in all_errors() {
            let display = format!("{}", err);
            assert!(
                !display.is_empty(),
                "Display must not be empty for {:?}",
                err
            );
            assert!(
                display.len() >= 4,
                "Display '{}' too short for {:?}",
                display,
                err
            );
        }
    }

    #[test]
    fn debug_all_variants_non_empty() {
        for err in all_errors() {
            let debug = format!("{:?}", err);
            assert!(!debug.is_empty(), "Debug must not be empty for {:?}", err);
        }
    }

    #[test]
    fn debug_contains_variant_name() {
        let variants = [
            (Error::QueueEmpty, "QueueEmpty"),
            (Error::VcsNotInitialized, "VcsNotInitialized"),
            (Error::WorkingCopyDirty, "WorkingCopyDirty"),
            (Error::StackCyclicDependency, "StackCyclicDependency"),
            (Error::GitHubTokenExpired, "GitHubTokenExpired"),
            (Error::Internal("x".into()), "Internal"),
        ];
        for (err, name) in variants {
            let debug = format!("{:?}", err);
            assert!(
                debug.contains(name),
                "Debug '{}' should contain '{}' for {:?}",
                debug,
                name,
                err
            );
        }
    }

    #[test]
    fn display_and_debug_differ() {
        // Display is human-readable, Debug includes variant name
        let err = Error::NotFound("resource".into());
        let display = format!("{}", err);
        let debug = format!("{:?}", err);
        assert_ne!(display, debug, "Display and Debug should differ");
        assert!(
            debug.contains("NotFound"),
            "Debug should contain variant name"
        );
    }

    #[test]
    fn display_contains_message_content() {
        let test_cases = [
            (Error::WorkspaceNotFound("my-ws".into()), "my-ws"),
            (Error::IoError("disk full".into()), "disk full"),
            (Error::Internal("secret error".into()), "secret error"),
            (
                Error::ValidationFieldError {
                    message: "required".into(),
                    field: "email".into(),
                    value: None,
                },
                "required",
            ),
        ];
        for (err, content) in test_cases {
            let display = format!("{}", err);
            assert!(
                display.contains(content),
                "Display '{}' should contain '{}' for {:?}",
                display,
                content,
                err
            );
        }
    }

    #[test]
    fn debug_preserves_all_fields() {
        let err = Error::LockTimeout {
            operation: "acquire_lock".into(),
            timeout_ms: 5000,
            retries: 3,
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("acquire_lock"));
        assert!(debug.contains("5000"));
        assert!(debug.contains("3"));
    }

    #[test]
    fn display_with_special_chars() {
        let err = Error::Internal("line1\nline2\ttab\\backslash".into());
        let display = format!("{}", err);
        assert!(display.contains("line1\nline2"));
        assert!(display.contains('\t'));
    }

    #[test]
    fn debug_with_special_chars() {
        let err = Error::IoError("null\0byte".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("null"));
    }
}

// ============================================================================
// NO PANIC IN ERROR CONSTRUCTORS
// ============================================================================

mod no_panic_constructors {
    use super::*;

    #[test]
    fn all_variants_display_does_not_panic() {
        for err in all_errors() {
            // Should not panic
            let _ = format!("{}", err);
        }
    }

    #[test]
    fn all_variants_debug_does_not_panic() {
        for err in all_errors() {
            // Should not panic
            let _ = format!("{:?}", err);
        }
    }

    #[test]
    fn all_variants_code_does_not_panic() {
        for err in all_errors() {
            // Should not panic - returns &'static str
            let _ = err.code();
        }
    }

    #[test]
    fn all_variants_exit_code_does_not_panic() {
        for err in all_errors() {
            // Should not panic - returns i32
            let _ = err.exit_code();
        }
    }

    #[test]
    fn all_variants_numeric_code_does_not_panic() {
        for err in all_errors() {
            // Should not panic - returns u16
            let _ = err.numeric_code();
        }
    }

    #[test]
    fn all_variants_category_does_not_panic() {
        for err in all_errors() {
            // Should not panic - returns ErrorCategory
            let _ = err.category();
        }
    }

    #[test]
    fn all_variants_suggestion_does_not_panic() {
        for err in all_errors() {
            // Should not panic - returns Option<String>
            let _ = err.suggestion();
        }
    }

    #[test]
    fn all_variants_fix_does_not_panic() {
        for err in all_errors() {
            // Should not panic - returns Option<ErrorFix>
            let _ = err.fix();
        }
    }

    #[test]
    fn all_variants_context_map_does_not_panic() {
        for err in all_errors() {
            // Should not panic - returns Option<serde_json::Value>
            let _ = err.context_map();
        }
    }

    #[test]
    fn all_variants_is_retryable_does_not_panic() {
        for err in all_errors() {
            // Should not panic - returns bool
            let _ = err.is_retryable();
        }
    }

    #[test]
    fn empty_string_constructors_all_variants() {
        let errors_with_strings = vec![
            Error::WorkspaceNotFound(String::new()),
            Error::WorkspaceExists(String::new()),
            Error::WorkspaceLocked(String::new(), String::new()),
            Error::WorkspaceConflict(String::new()),
            Error::SessionNotFound(String::new()),
            Error::SessionExists(String::new()),
            Error::SessionLocked(String::new(), String::new()),
            Error::NotLockHolder(String::new(), String::new()),
            Error::SessionInvalidState(String::new(), String::new(), String::new()),
            Error::BeadNotFound(String::new()),
            Error::BeadAlreadyExists(String::new()),
            Error::InvalidBeadId(String::new()),
            Error::InvalidBeadTitle(String::new()),
            Error::BeadDependencyCycle(String::new()),
            Error::BeadBlockedBy(String::new()),
            Error::BeadInvalidDependency(String::new()),
            Error::QueueItemNotFound(String::new()),
            Error::QueueLocked(String::new()),
            Error::VcsConflict(String::new(), String::new()),
            Error::VcsPushFailed(String::new()),
            Error::VcsPullFailed(String::new()),
            Error::VcsRebaseFailed(String::new()),
            Error::BranchNotFound(String::new()),
            Error::BranchExists(String::new()),
            Error::CommitNotFound(String::new()),
            Error::StackNotFound(String::new()),
            Error::StackOrphaned(String::new()),
            Error::StackInvalidState(String::new()),
            Error::StackPrNotFound(String::new()),
            Error::GitHubAuthFailed(String::new()),
            Error::GitHubRateLimited(String::new()),
            Error::GitHubPrClosed(String::new()),
            Error::GitHubPrNotFound(String::new()),
            Error::GitHubApiError {
                status: 0,
                message: String::new(),
            },
            Error::GitHubCiFailed(vec![]),
            Error::SnapshotNotFound(String::new()),
            Error::SnapshotCorrupted(String::new()),
            Error::SnapshotExpired(String::new()),
            Error::SnapshotLimitExceeded(String::new()),
            Error::SnapshotRestoreFailed(String::new()),
            Error::ConfigNotFound(String::new()),
            Error::ConfigInvalid(String::new()),
            Error::ConfigPermission(String::new()),
            Error::InvalidConfig(String::new()),
            Error::InvalidRepoUrl(String::new()),
            Error::AgentNotFound(String::new()),
            Error::AgentExists(String::new()),
            Error::AgentTimeout(String::new()),
            Error::InvalidState(String::new()),
            Error::NotFound(String::new()),
            Error::InvalidOperation(String::new()),
            Error::ValidationError(String::new()),
            Error::ValidationFieldError {
                message: String::new(),
                field: String::new(),
                value: None,
            },
            Error::InvalidIdentifier(String::new()),
            Error::IoError(String::new()),
            Error::JsonParseError(String::new()),
            Error::YamlParseError(String::new()),
            Error::Database(String::new()),
            Error::Serialization(String::new()),
            Error::LockTimeout {
                operation: String::new(),
                timeout_ms: 0,
                retries: 0,
            },
            Error::CloneFailed(String::new()),
            Error::RecordFailed(String::new()),
            Error::Persistence(String::new()),
            Error::StateTransition(String::new()),
            Error::ScenarioError(String::new()),
            Error::RunnerError(String::new()),
            Error::DefinitionError(String::new()),
            Error::ServerError(String::new()),
            Error::SyncError(String::new()),
            Error::Internal(String::new()),
            Error::Unimplemented(String::new()),
            Error::InvariantViolation(String::new()),
        ];

        for err in errors_with_strings {
            // All methods should work without panic on empty strings
            let _ = err.to_string();
            let _ = format!("{:?}", err);
            let _ = err.code();
            let _ = err.exit_code();
            let _ = err.numeric_code();
            let _ = err.category();
            let _ = err.suggestion();
            let _ = err.fix();
            let _ = err.context_map();
            let _ = err.is_retryable();
        }
    }

    #[test]
    fn max_usize_constructors() {
        let errors = vec![
            Error::QueueInvalidPosition(usize::MAX),
            Error::QueueFull(usize::MAX),
        ];
        for err in errors {
            let _ = err.to_string();
            let _ = err.exit_code();
        }
    }

    #[test]
    fn max_u64_constructors() {
        let err = Error::LockTimeout {
            operation: "max_timeout".into(),
            timeout_ms: u64::MAX,
            retries: usize::MAX,
        };
        let _ = err.to_string();
        let _ = err.exit_code();
        let _ = err.numeric_code();
    }

    #[test]
    fn all_category_base_max_pairs_valid() {
        // Every ErrorCategory base() must be <= max()
        let categories = [
            ErrorCategory::Workspace,
            ErrorCategory::Session,
            ErrorCategory::Bead,
            ErrorCategory::Queue,
            ErrorCategory::Vcs,
            ErrorCategory::Stack,
            ErrorCategory::GitHub,
            ErrorCategory::Snapshot,
            ErrorCategory::Internal,
        ];
        for cat in categories {
            assert!(
                cat.base() <= cat.max(),
                "Category {:?} base() {} must be <= max() {}",
                cat,
                cat.base(),
                cat.max()
            );
        }
    }

    #[test]
    fn error_fix_safe_constructor() {
        let fix = ErrorFix::safe("cmd", "desc");
        assert_eq!(fix.risk, FixRisk::Safe);
        assert_eq!(fix.command, "cmd");
        assert_eq!(fix.description, "desc");
    }

    #[test]
    fn error_fix_new_all_risk_levels() {
        for risk in [FixRisk::Safe, FixRisk::Moderate, FixRisk::Dangerous] {
            let fix = ErrorFix::new("cmd", "desc", risk);
            assert_eq!(fix.risk, risk);
        }
    }

    #[test]
    fn error_fix_with_empty_strings() {
        let fix = ErrorFix::new("", "", FixRisk::Safe);
        assert_eq!(fix.command, "");
        assert_eq!(fix.description, "");
        assert_eq!(fix.risk, FixRisk::Safe);
    }
}

// ============================================================================
// RESULT TYPE VERIFICATION
// ============================================================================

mod result_type_verification {
    use super::*;

    #[test]
    fn result_ok_maps_correctly() {
        let r: Result<i32> = Ok(42);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn result_err_maps_correctly() {
        let r: Result<i32> = Err(Error::Internal("boom".into()));
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().to_string(), "Internal error: boom");
    }

    #[test]
    fn result_err_with_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let r: Result<String> = Err(io_err.into());
        assert!(r.is_err());
        let err = r.unwrap_err();
        assert!(matches!(err, Error::IoError(_)));
    }

    #[test]
    fn result_chaining_with_question_mark() {
        fn inner() -> Result<()> {
            std::fs::read_to_string("/nonexistent")?;
            Ok(())
        }
        let result = inner();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::IoError(_)));
    }
}

// ============================================================================
// INVARIANT VERIFICATION
// ============================================================================

mod invariant_verification {
    use super::*;

    #[test]
    fn all_exit_codes_positive() {
        for err in all_errors() {
            assert!(
                err.exit_code() > 0,
                "exit_code must be positive for {:?}, got {}",
                err,
                err.exit_code()
            );
        }
    }

    #[test]
    fn all_exit_codes_unique() {
        let codes: Vec<i32> = all_errors().iter().map(|e| e.exit_code()).collect();
        let mut sorted = codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(codes.len(), sorted.len(), "All exit codes must be unique");
    }

    #[test]
    fn all_numeric_codes_unique() {
        let codes: Vec<u16> = all_errors().iter().map(|e| e.numeric_code()).collect();
        let mut sorted = codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            codes.len(),
            sorted.len(),
            "All numeric codes must be unique"
        );
    }

    #[test]
    fn all_codes_are_screaming_snake_case() {
        for err in all_errors() {
            let code = err.code();
            assert!(
                code.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "code '{}' must be SCREAMING_SNAKE_CASE for {:?}",
                code,
                err
            );
            assert!(
                !code.starts_with('_') && !code.ends_with('_') && !code.contains("__"),
                "code '{}' has invalid underscore pattern for {:?}",
                code,
                err
            );
        }
    }

    #[test]
    fn all_numeric_codes_in_category_range() {
        for err in all_errors() {
            let code = err.numeric_code();
            let cat = err.category();
            assert!(
                code >= cat.base() && code <= cat.max(),
                "numeric_code {} for {:?} outside category {:?} range {}-{}",
                code,
                err,
                cat,
                cat.base(),
                cat.max()
            );
        }
    }

    #[test]
    fn suggestion_returns_string_for_supported_variants() {
        let with_suggestion = vec![
            Error::WorkspaceNotFound("ws".into()),
            Error::SessionNotFound("s".into()),
            Error::QueueEmpty,
            Error::WorkspaceLocked("ws".into(), "holder".into()),
            Error::VcsNotInitialized,
            Error::WorkingCopyDirty,
        ];
        for err in with_suggestion {
            let sug = err.suggestion();
            assert!(
                sug.is_some() && !sug.unwrap().is_empty(),
                "Expected non-empty suggestion for {:?}",
                err
            );
        }
    }

    #[test]
    fn suggestion_returns_none_for_unsupported_variants() {
        let without_suggestion = vec![
            Error::Internal("msg".into()),
            Error::BeadNotFound("b".into()),
            Error::IoError("msg".into()),
            Error::QueueItemNotFound("q".into()),
        ];
        for err in without_suggestion {
            assert!(
                err.suggestion().is_none(),
                "Expected no suggestion for {:?}",
                err
            );
        }
    }

    #[test]
    fn fix_returns_error_fix_for_supported_variants() {
        let with_fix = vec![
            Error::WorkspaceNotFound("ws".into()),
            Error::SessionNotFound("s".into()),
            Error::QueueEmpty,
            Error::WorkspaceLocked("ws".into(), "holder".into()),
            Error::VcsNotInitialized,
            Error::WorkingCopyDirty,
        ];
        for err in with_fix {
            let fix = err.fix();
            assert!(fix.is_some(), "Expected fix for {:?}", err);
            let fix = fix.unwrap();
            assert!(!fix.command.is_empty(), "Fix command must be non-empty");
        }
    }

    #[test]
    fn context_map_returns_json_object_for_all_variants() {
        for err in all_errors() {
            let ctx = err.context_map();
            assert!(
                ctx.is_some(),
                "context_map() must return Some for {:?}",
                err
            );
            let ctx = ctx.unwrap();
            assert!(
                ctx.is_object(),
                "context_map() must return JSON object for {:?}, got {}",
                err,
                ctx
            );
        }
    }
}
