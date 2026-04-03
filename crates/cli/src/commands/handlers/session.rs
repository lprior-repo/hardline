//! Session management handlers
//!
//! Ported from hardline's session management module, adapted to use scp_core types.
//!
//! Operations:
//! - Pause: transition an active session to paused state
//! - Resume: transition a paused session back to active state
//! - Clone: duplicate a session into a new workspace

use scp_core::{output::Output, vcs, Error, Result};

/// Pause an active session.
///
/// Validates that the named session exists and is in a valid state for pausing.
///
/// # Errors
///
/// Returns an error if:
/// - Session name is empty
/// - Session (workspace) not found
/// - Pause functionality is not yet implemented
pub fn pause(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "session name cannot be empty".to_string(),
        ));
    }

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::session(format!("session '{name}' not found")));
    }

    Err(Error::unimplemented(
        "session pause: session state persistence is not yet implemented",
    ))
}

/// Resume a paused session.
///
/// Validates that the named session exists and is in a valid state for resuming.
///
/// # Errors
///
/// Returns an error if:
/// - Session name is empty
/// - Session (workspace) not found
/// - Resume functionality is not yet implemented
pub fn resume(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "session name cannot be empty".to_string(),
        ));
    }

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::session(format!("session '{name}' not found")));
    }

    Err(Error::unimplemented(
        "session resume: session state persistence is not yet implemented",
    ))
}

/// Result of a clone operation.
#[derive(Debug, Clone)]
pub struct CloneResult {
    pub success: bool,
    pub source: String,
    pub target: String,
    pub dry_run: bool,
}

/// Clone a session.
///
/// Creates a new workspace forked from the source session.
///
/// # Errors
///
/// Returns an error if:
/// - Session name is empty
/// - Source workspace not found
/// - Target workspace already exists
/// - VCS fork operation fails
pub fn clone_session(source: &str, target: &str, dry_run: bool) -> Result<CloneResult> {
    if source.is_empty() {
        return Err(Error::invalid_identifier(
            "source session name cannot be empty".to_string(),
        ));
    }
    if target.is_empty() {
        return Err(Error::invalid_identifier(
            "target session name cannot be empty".to_string(),
        ));
    }

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    // Check source exists
    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == source) {
        return Err(Error::workspace_not_found(source.to_string()));
    }

    // Check target doesn't exist
    if workspaces.iter().any(|w| w.name == target) {
        return Err(Error::workspace_exists(target.to_string()));
    }

    if dry_run {
        Output::info(&format!(
            "[dry-run] Would clone '{}' to '{}'",
            source, target
        ));
        return Ok(CloneResult {
            success: true,
            source: source.to_string(),
            target: target.to_string(),
            dry_run: true,
        });
    }

    Output::info(&format!("Cloning '{}' to '{}'...", source, target));

    backend.fork_workspace(source, target)?;

    Output::success(&format!("Cloned '{}' to '{}'", source, target));
    Ok(CloneResult {
        success: true,
        source: source.to_string(),
        target: target.to_string(),
        dry_run: false,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use tempfile::TempDir;

    /// Get a known-valid directory (project root via CARGO_MANIFEST_DIR) for
    /// cwd restoration in tests that change directory.
    fn safe_restore_dir() -> std::path::PathBuf {
        std::env::var("CARGO_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                // Fallback: the process exe's parent directory is usually valid
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            })
    }

    #[test]
    fn test_pause_rejects_empty_name() {
        let result = pause("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), "INVALID_IDENTIFIER");
    }

    #[test]
    fn test_resume_rejects_empty_name() {
        let result = resume("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), "INVALID_IDENTIFIER");
    }

    #[test]
    #[serial]
    fn test_pause_returns_error_for_nonexistent_session() {
        // Run in a non-VCS directory so create_backend fails early.
        // Use a TempDir guard so the cwd is always valid during the test.
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");

        let result = pause("no-such-session");
        assert!(
            result.is_err(),
            "pause should error for non-existent session"
        );
        // In a non-VCS dir the error comes from VCS not being initialized,
        // but the key invariant is: it must not silently succeed.
        assert_ne!(
            result.unwrap_err().code(),
            "INVALID_IDENTIFIER",
            "expected VCS/session error, not identifier error"
        );

        // Restore cwd to a known-valid dir BEFORE TempDir drops (deletes dir)
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    #[serial]
    fn test_resume_returns_error_for_nonexistent_session() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");

        let result = resume("no-such-session");
        assert!(
            result.is_err(),
            "resume should error for non-existent session"
        );
        assert_ne!(
            result.unwrap_err().code(),
            "INVALID_IDENTIFIER",
            "expected VCS/session error, not identifier error"
        );

        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    #[serial]
    fn test_pause_returns_unimplemented_error() {
        // pause should never silently succeed -- it must return an error
        // until session state persistence is implemented.
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");

        let result = pause("any-session");
        assert!(result.is_err(), "pause must not be a silent no-op");

        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    #[serial]
    fn test_resume_returns_unimplemented_error() {
        // resume should never silently succeed -- it must return an error
        // until session state persistence is implemented.
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");

        let result = resume("any-session");
        assert!(result.is_err(), "resume must not be a silent no-op");

        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    // -- Clone validation tests --

    #[test]
    fn test_clone_rejects_empty_source() {
        let result = clone_session("", "target", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_clone_rejects_empty_target() {
        let result = clone_session("source", "", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_clone_result_fields() {
        let result = CloneResult {
            success: true,
            source: "src".to_string(),
            target: "dst".to_string(),
            dry_run: true,
        };
        assert_eq!(result.source, "src");
        assert_eq!(result.target, "dst");
        assert!(result.dry_run);
        assert!(result.success);
    }

    // -- Whitespace name edge cases --

    #[test]
    #[serial]
    fn test_pause_rejects_whitespace_only_name() {
        let result = pause("   \t  ");
        assert!(result.is_err());
        // Whitespace-only is not empty, so it passes the is_empty() check
        // and proceeds to VCS validation which fails in a non-VCS directory.
        // The key invariant is: it must not silently succeed.
        assert!(result.is_err(), "whitespace-only name must not succeed");
    }

    #[test]
    #[serial]
    fn test_resume_rejects_whitespace_only_name() {
        let result = resume("   \t  ");
        assert!(result.is_err(), "whitespace-only name must not succeed");
    }

    #[test]
    #[serial]
    fn test_pause_leading_trailing_whitespace() {
        // "  session  " is not empty, passes is_empty(), then VCS fails.
        let result = pause("  my-session  ");
        assert!(
            result.is_err(),
            "leading/trailing whitespace must not bypass validation"
        );
    }

    #[test]
    #[serial]
    fn test_resume_leading_trailing_whitespace() {
        let result = resume("  my-session  ");
        assert!(
            result.is_err(),
            "leading/trailing whitespace must not bypass validation"
        );
    }

    // -- Clone with same source and target --

    #[test]
    fn test_clone_same_source_and_target() {
        // Same source and target: the source workspace won't be found (or
        // if it is, target already exists). Either way, must not succeed.
        let result = clone_session("same-name", "same-name", false);
        assert!(
            result.is_err(),
            "clone with same source and target must not succeed"
        );
    }

    #[test]
    fn test_clone_same_source_and_target_dry_run() {
        let result = clone_session("same-name", "same-name", true);
        assert!(
            result.is_err(),
            "dry-run clone with same source and target must not succeed"
        );
    }

    // -- Very long session names --

    #[test]
    fn test_pause_with_very_long_name() {
        let long_name = "a".repeat(10_000);
        let result = pause(&long_name);
        // Should not panic on extremely long names; must return an error
        // (workspace won't be found).
        assert!(
            result.is_err(),
            "very long name must not cause a panic or silent success"
        );
    }

    #[test]
    fn test_resume_with_very_long_name() {
        let long_name = "b".repeat(10_000);
        let result = resume(&long_name);
        assert!(
            result.is_err(),
            "very long name must not cause a panic or silent success"
        );
    }

    #[test]
    fn test_clone_with_very_long_source_name() {
        let long_name = "c".repeat(10_000);
        let result = clone_session(&long_name, "target", false);
        assert!(
            result.is_err(),
            "very long source name must not cause a panic or silent success"
        );
    }

    #[test]
    fn test_clone_with_very_long_target_name() {
        let long_name = "d".repeat(10_000);
        let result = clone_session("source", &long_name, false);
        assert!(
            result.is_err(),
            "very long target name must not cause a panic or silent success"
        );
    }

    #[test]
    fn test_clone_with_very_long_both_names() {
        let long_source = "s".repeat(10_000);
        let long_target = "t".repeat(10_000);
        let result = clone_session(&long_source, &long_target, false);
        assert!(
            result.is_err(),
            "very long names must not cause a panic or silent success"
        );
    }

    // -- Clone with whitespace-only names --

    #[test]
    fn test_clone_whitespace_only_source() {
        let result = clone_session("   ", "target", false);
        // Whitespace-only source is not empty, but VCS backend or
        // workspace listing will reject it.
        assert!(result.is_err());
    }

    #[test]
    fn test_clone_whitespace_only_target() {
        let result = clone_session("source", "   ", false);
        // Whitespace-only target is not empty, but VCS backend or
        // workspace listing will reject it.
        assert!(result.is_err());
    }

    // -- CloneResult data type tests --

    #[test]
    fn test_clone_result_debug_contains_fields() {
        let result = CloneResult {
            success: true,
            source: "src".to_string(),
            target: "dst".to_string(),
            dry_run: false,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("src"));
        assert!(debug.contains("dst"));
        assert!(debug.contains("true") || debug.contains("false"));
    }

    #[test]
    fn test_clone_result_clone_independence() {
        let result = CloneResult {
            success: true,
            source: "original".to_string(),
            target: "target".to_string(),
            dry_run: false,
        };
        let mut cloned = result.clone();
        cloned.source = "modified".to_string();
        assert_eq!(result.source, "original", "clone should be independent");
        assert_eq!(cloned.source, "modified");
    }

    #[test]
    fn test_clone_result_field_equality() {
        let a = CloneResult {
            success: true,
            source: "s".to_string(),
            target: "t".to_string(),
            dry_run: true,
        };
        let b = CloneResult {
            success: true,
            source: "s".to_string(),
            target: "t".to_string(),
            dry_run: true,
        };
        assert_eq!(a.source, b.source);
        assert_eq!(a.target, b.target);
        assert_eq!(a.success, b.success);
        assert_eq!(a.dry_run, b.dry_run);
    }

    #[test]
    fn test_clone_result_field_inequality() {
        let a = CloneResult {
            success: true,
            source: "s".to_string(),
            target: "t".to_string(),
            dry_run: true,
        };
        let b = CloneResult {
            success: false,
            source: "s".to_string(),
            target: "t".to_string(),
            dry_run: true,
        };
        assert_ne!(a.success, b.success);
    }

    #[test]
    fn test_clone_result_differs_by_dry_run() {
        let wet = CloneResult {
            success: true,
            source: "s".to_string(),
            target: "t".to_string(),
            dry_run: false,
        };
        let dry = CloneResult {
            success: true,
            source: "s".to_string(),
            target: "t".to_string(),
            dry_run: true,
        };
        assert_ne!(wet.dry_run, dry.dry_run);
    }

    #[test]
    fn test_clone_result_failed_variant() {
        let result = CloneResult {
            success: false,
            source: "src".to_string(),
            target: "dst".to_string(),
            dry_run: false,
        };
        assert!(!result.success);
    }

    // -- Clone with special characters in names --

    #[test]
    fn test_pause_with_newline_in_name() {
        let result = pause("session\nname");
        assert!(result.is_err(), "newline in name must not succeed");
    }

    #[test]
    fn test_resume_with_newline_in_name() {
        let result = resume("session\nname");
        assert!(result.is_err(), "newline in name must not succeed");
    }

    #[test]
    fn test_clone_with_null_byte_in_source() {
        let result = clone_session("source\0", "target", false);
        assert!(result.is_err(), "null byte in name must not succeed");
    }

    #[test]
    fn test_clone_with_null_byte_in_target() {
        let result = clone_session("source", "target\0", false);
        assert!(result.is_err(), "null byte in name must not succeed");
    }

    // -- Error code verification --

    #[test]
    fn test_pause_empty_error_code_is_invalid_identifier() {
        let result = pause("");
        let err = result.expect_err("should be error");
        assert_eq!(err.code(), "INVALID_IDENTIFIER");
    }

    #[test]
    fn test_resume_empty_error_code_is_invalid_identifier() {
        let result = resume("");
        let err = result.expect_err("should be error");
        assert_eq!(err.code(), "INVALID_IDENTIFIER");
    }

    #[test]
    fn test_clone_empty_source_error_code_is_invalid_identifier() {
        let result = clone_session("", "target", false);
        let err = result.expect_err("should be error");
        assert_eq!(err.code(), "INVALID_IDENTIFIER");
    }

    #[test]
    fn test_clone_empty_target_error_code_is_invalid_identifier() {
        let result = clone_session("source", "", false);
        let err = result.expect_err("should be error");
        assert_eq!(err.code(), "INVALID_IDENTIFIER");
    }

    // -- Clone dry_run vs non-dry_run error behavior --

    #[test]
    fn test_clone_dry_run_vs_wet_same_validation() {
        // Both dry_run and non-dry_run should reject empty names identically
        let dry = clone_session("", "target", true);
        let wet = clone_session("", "target", false);
        assert_eq!(
            dry.is_err(),
            wet.is_err(),
            "dry_run and non-dry_run should have identical validation"
        );
    }

    use proptest::prelude::*;
    use proptest::proptest;
    use proptest::{prop_assert, prop_assert_eq};
    use serial_test::serial;

    proptest! {
        #[test]
        fn prop_pause_any_whitespace_or_empty_always_fails(
            name in proptest::string::string_regex("[ \t]*").unwrap()
        ) {
            let result = pause(&name);
            prop_assert!(result.is_err());
        }

        #[test]
        fn prop_resume_any_whitespace_or_empty_always_fails(
            name in proptest::string::string_regex("[ \t]*").unwrap()
        ) {
            let result = resume(&name);
            prop_assert!(result.is_err());
        }

        #[test]
        fn prop_clone_empty_source_always_fails(target in "[a-z]{1,20}", dry_run in proptest::bool::ANY) {
            let result = clone_session("", &target, dry_run);
            prop_assert!(result.is_err());
        }

        #[test]
        fn prop_clone_empty_target_always_fails(source in "[a-z]{1,20}", dry_run in proptest::bool::ANY) {
            let result = clone_session(&source, "", dry_run);
            prop_assert!(result.is_err());
        }

        #[test]
        fn prop_clone_result_clone_preserves_fields(source in "[a-z]{1,10}", target in "[a-z]{1,10}", dry_run in proptest::bool::ANY) {
            let r = CloneResult {
                success: true,
                source: source.clone(),
                target: target.clone(),
                dry_run,
            };
            let cloned = r.clone();
            prop_assert_eq!(r.source, cloned.source);
            prop_assert_eq!(r.target, cloned.target);
            prop_assert_eq!(r.success, cloned.success);
            prop_assert_eq!(r.dry_run, cloned.dry_run);
        }
    }
}
