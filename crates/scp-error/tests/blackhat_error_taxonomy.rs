//! Black-hat attack tests for scp-error taxonomy.
//!
//! Tests: Display non-empty for every variant, exit codes unique, numeric codes
//! in correct ranges, fix suggestions have non-empty commands, serialization roundtrips.

use serde_json;
use scp_error::{Error, ErrorCategory, ErrorFix, FixRisk};

/// Build one of every variant for exhaustive testing.
fn all_variants() -> Vec<Error> {
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
// ATTACK 1: Every error variant's Display is non-empty
// ============================================================================
#[test]
fn attack_display_non_empty_all_variants() {
    for variant in all_variants() {
        let display = variant.to_string();
        assert!(
            !display.is_empty(),
            "Display should not be empty for {:?}",
            variant
        );
    }
}

// ============================================================================
// ATTACK 2: Every error variant's exit_code is unique
// ============================================================================
#[test]
fn attack_exit_codes_unique() {
    let variants = all_variants();
    let codes: Vec<i32> = variants.iter().map(|v| v.exit_code()).collect();
    let mut seen = std::collections::HashSet::new();
    let mut duplicates = Vec::new();
    for (i, code) in codes.iter().enumerate() {
        if !seen.insert(*code) {
            duplicates.push((i, *code));
        }
    }
    assert!(
        duplicates.is_empty(),
        "Exit codes must be unique. Duplicates: {:?}",
        duplicates
    );
}

// ============================================================================
// ATTACK 3: Every error variant's numeric_code falls in correct range
// ============================================================================
#[test]
fn attack_numeric_codes_in_category_range() {
    for variant in all_variants() {
        let code = variant.numeric_code();
        let cat = variant.category();
        assert!(
            code >= cat.base() && code <= cat.max(),
            "Numeric code {} for {:?} outside category {:?} range {}-{}",
            code,
            variant.code(),
            cat,
            cat.base(),
            cat.max()
        );
    }
}

// ============================================================================
// ATTACK 4: Every error variant's numeric_code is unique
// ============================================================================
#[test]
fn attack_numeric_codes_all_unique() {
    let variants = all_variants();
    let codes: Vec<u16> = variants.iter().map(|v| v.numeric_code()).collect();
    let mut seen = std::collections::HashSet::new();
    let mut duplicates = Vec::new();
    for (i, code) in codes.iter().enumerate() {
        if !seen.insert(*code) {
            duplicates.push((i, *code));
        }
    }
    assert!(
        duplicates.is_empty(),
        "Numeric codes must be unique. Duplicates at indices: {:?}",
        duplicates
    );
}

// ============================================================================
// ATTACK 5: Fix suggestions have non-empty commands
// ============================================================================
#[test]
fn attack_fix_commands_non_empty() {
    for variant in all_variants() {
        if let Some(fix) = variant.fix() {
            assert!(
                !fix.command.is_empty(),
                "Fix command should not be empty for {:?}",
                variant
            );
            assert!(
                !fix.description.is_empty(),
                "Fix description should not be empty for {:?}",
                variant
            );
        }
    }
}

// ============================================================================
// ATTACK 6: Error serialization roundtrips for every variant
// ============================================================================
#[test]
fn attack_serialization_roundtrip_all_variants() {
    for variant in all_variants() {
        let json = serde_json::to_string(&variant).unwrap_or_else(|e| {
            panic!("Serialize failed for {:?}: {}", variant, e)
        });

        // Must be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|e| {
            panic!("Invalid JSON for {:?}: {}\nJSON: {}", variant, e, json)
        });

        // Unit variants (QueueEmpty, etc.) serialize as plain strings, not objects.
        // Tuple variants and struct variants serialize as objects or arrays.
        // Both are valid - just verify it's valid JSON.
        // FINDING: Unit variants serialize as bare strings (e.g. "QueueEmpty"),
        // while struct/tuple variants serialize differently. This means
        // deserialization must handle both formats.

        // Roundtrip: deserialize back
        let deserialized: Error = serde_json::from_str(&json).unwrap_or_else(|e| {
            panic!("Deserialize failed for {:?}: {}\nJSON: {}", variant, e, json)
        });

        // Verify roundtrip preserves code
        assert_eq!(
            variant.code(),
            deserialized.code(),
            "Code should roundtrip for {:?}",
            variant
        );

        // Verify roundtrip preserves exit_code
        assert_eq!(
            variant.exit_code(),
            deserialized.exit_code(),
            "Exit code should roundtrip for {:?}",
            variant
        );

        // Verify roundtrip preserves numeric_code
        assert_eq!(
            variant.numeric_code(),
            deserialized.numeric_code(),
            "Numeric code should roundtrip for {:?}",
            variant
        );
    }
}

// ============================================================================
// ATTACK 7: context_map returns valid JSON for every variant
// ============================================================================
#[test]
fn attack_context_map_valid_json_all_variants() {
    for variant in all_variants() {
        let ctx = variant.context_map();
        assert!(
            ctx.is_some(),
            "context_map() should return Some for every variant: {:?}",
            variant.code()
        );
        let ctx = ctx.unwrap();
        assert!(
            ctx.is_object(),
            "context_map should return a JSON object for {:?}",
            variant.code()
        );
    }
}

// ============================================================================
// ATTACK 8: Error codes are SCREAMING_SNAKE_CASE
// ============================================================================
#[test]
fn attack_codes_screaming_snake() {
    for variant in all_variants() {
        let code = variant.code();
        assert!(
            !code.is_empty(),
            "Code should not be empty"
        );
        assert!(
            code.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
            "Code '{}' is not SCREAMING_SNAKE_CASE for {:?}",
            code,
            variant
        );
        assert!(
            !code.starts_with('_') && !code.ends_with('_') && !code.contains("__"),
            "Code '{}' has invalid underscore patterns",
            code
        );
    }
}

// ============================================================================
// ATTACK 9: Error category consistency
// ============================================================================
#[test]
fn attack_category_consistency() {
    // Workspace errors should map to Workspace category
    assert_eq!(Error::WorkspaceNotFound("x".into()).category(), ErrorCategory::Workspace);
    assert_eq!(Error::WorkspaceExists("x".into()).category(), ErrorCategory::Workspace);
    assert_eq!(Error::WorkspaceLocked("x".into(), "y".into()).category(), ErrorCategory::Workspace);

    // VCS errors should map to Vcs category
    assert_eq!(Error::VcsNotInitialized.category(), ErrorCategory::Vcs);
    assert_eq!(Error::BranchNotFound("x".into()).category(), ErrorCategory::Vcs);
    assert_eq!(Error::WorkingCopyDirty.category(), ErrorCategory::Vcs);

    // Stack errors should map to Stack category
    assert_eq!(Error::StackNotFound("x".into()).category(), ErrorCategory::Stack);
    assert_eq!(Error::StackCyclicDependency.category(), ErrorCategory::Stack);

    // Snapshot errors should map to Snapshot category
    assert_eq!(Error::SnapshotNotFound("x".into()).category(), ErrorCategory::Snapshot);
    assert_eq!(Error::SnapshotCorrupted("x".into()).category(), ErrorCategory::Snapshot);

    // Infrastructure errors should map to Internal category
    assert_eq!(Error::Internal("x".into()).category(), ErrorCategory::Internal);
    assert_eq!(Error::ConfigNotFound("x".into()).category(), ErrorCategory::Internal);
    assert_eq!(Error::IoError("x".into()).category(), ErrorCategory::Internal);
    assert_eq!(Error::Database("x".into()).category(), ErrorCategory::Internal);
}

// ============================================================================
// ATTACK 10: Error with empty strings doesn't panic
// ============================================================================
#[test]
fn attack_empty_strings_no_panic() {
    let errors = vec![
        Error::WorkspaceNotFound(String::new()),
        Error::SessionNotFound(String::new()),
        Error::BeadNotFound(String::new()),
        Error::InvalidBeadTitle(String::new()),
        Error::IoError(String::new()),
        Error::Internal(String::new()),
        Error::VcsConflict(String::new(), String::new()),
        Error::WorkspaceLocked(String::new(), String::new()),
        Error::ValidationFieldError {
            message: String::new(),
            field: String::new(),
            value: None,
        },
    ];

    for err in errors {
        let display = err.to_string();
        let _ = display; // Should not panic
        let _ = err.code();
        let _ = err.exit_code();
        let _ = err.numeric_code();
        let _ = err.category();
        let _ = err.context_map();
        let _ = err.is_retryable();
        let _ = err.fix();
        let _ = err.suggestion();
    }
}

// ============================================================================
// ATTACK 11: Error with unicode strings
// ============================================================================
#[test]
fn attack_unicode_strings() {
    let err = Error::WorkspaceNotFound("ワークスペース".into());
    assert!(err.to_string().contains("ワークスペース"));

    let err = Error::Internal("エラー発生 💥".into());
    assert!(err.to_string().contains("エラー発生"));

    let json = serde_json::to_string(&err).expect("serialize");
    assert!(json.contains("エラー発生"));
}

// ============================================================================
// ATTACK 12: Error with very long strings (>64KB)
// ============================================================================
#[test]
fn attack_very_long_strings() {
    let long_msg = "x".repeat(70000);
    let err = Error::Internal(long_msg.clone());
    assert_eq!(err.to_string().len(), long_msg.len() + "Internal error: ".len());

    // Serialization should handle long strings
    let json = serde_json::to_string(&err).expect("serialize long string");
    let deserialized: Error = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(deserialized.to_string(), err.to_string());
}

// ============================================================================
// ATTACK 13: ErrorFix serialization roundtrip
// ============================================================================
#[test]
fn attack_error_fix_serialization() {
    let fixes = vec![
        ErrorFix::new("cmd", "desc", FixRisk::Safe),
        ErrorFix::new("cmd", "desc", FixRisk::Moderate),
        ErrorFix::new("cmd", "desc", FixRisk::Dangerous),
        ErrorFix::safe("safe-cmd", "safe-desc"),
        ErrorFix::new("", "", FixRisk::Safe),       // empty strings
        ErrorFix::new("cmd\x00null", "desc\x00null", FixRisk::Dangerous), // null bytes
    ];

    for fix in fixes {
        let json = serde_json::to_string(&fix).expect("serialize fix");
        let deserialized: ErrorFix = serde_json::from_str(&json).expect("deserialize fix");
        assert_eq!(deserialized.command, fix.command);
        assert_eq!(deserialized.description, fix.description);
        assert_eq!(deserialized.risk, fix.risk);
    }
}

// ============================================================================
// ATTACK 14: ErrorCategory Display and ranges are consistent
// ============================================================================
#[test]
fn attack_category_ranges_non_overlapping() {
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

    let mut ranges: Vec<(u16, u16)> = categories.iter().map(|c| (c.base(), c.max())).collect();
    ranges.sort_by_key(|r| r.0);

    for window in ranges.windows(2) {
        assert!(
            window[0].1 < window[1].0,
            "Category ranges overlap: {:?}",
            window
        );
    }
}

// ============================================================================
// ATTACK 15: Deserialize from malicious JSON
// ============================================================================
#[test]
fn attack_deserialize_malicious_json() {
    // Try to deserialize with extra fields
    let json = r#"{"WorkspaceNotFound": "test", "extra": "field"}"#;
    let result: Result<Error, _> = serde_json::from_str(json);
    // Should succeed (serde ignores unknown fields by default for enums)
    // or fail gracefully - just verify no panic
    let _ = result;

    // Try null
    let json = r#"null"#;
    let result: Result<Error, _> = serde_json::from_str(json);
    assert!(result.is_err(), "null should fail to deserialize");

    // Try wrong type
    let json = r#"123"#;
    let result: Result<Error, _> = serde_json::from_str(json);
    assert!(result.is_err(), "number should fail to deserialize");
}

// ============================================================================
// ATTACK 16: is_retryable is correct
// ============================================================================
#[test]
fn attack_retryable_classification() {
    // Should be retryable
    assert!(Error::VcsPushFailed("x".into()).is_retryable());
    assert!(Error::VcsPullFailed("x".into()).is_retryable());
    assert!(Error::GitHubRateLimited("60s".into()).is_retryable());
    assert!(Error::LockTimeout {
        operation: "x".into(),
        timeout_ms: 1000,
        retries: 0,
    }
    .is_retryable());

    // Should NOT be retryable
    assert!(!Error::WorkspaceNotFound("x".into()).is_retryable());
    assert!(!Error::BeadNotFound("x".into()).is_retryable());
    assert!(!Error::QueueEmpty.is_retryable());
    assert!(!Error::Internal("x".into()).is_retryable());
    assert!(!Error::InvariantViolation("x".into()).is_retryable());
    assert!(!Error::GitHubAuthFailed("x".into()).is_retryable());
    assert!(!Error::SnapshotCorrupted("x".into()).is_retryable());
    assert!(!Error::WorkingCopyDirty.is_retryable());
}
