//! Common test utilities and relaxed clippy settings for isolate-core integration tests
//!
//! Integration tests need relaxed clippy settings for brutal test scenarios.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    clippy::indexing_slicing,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
    // Test-specific patterns
    clippy::needless_raw_string_hashes,
    clippy::bool_assert_comparison,
    // Async and concurrency relaxations for stress tests
    clippy::await_holding_lock,
    clippy::significant_drop_tightening,
    clippy::needless_continue,
    clippy::manual_clamp,
)]

use isolate_core::Result;
use tempfile::TempDir;

/// Set up a temporary directory with an initialized jj repository
pub fn setup_test_repo() -> Result<TempDir> {
    let temp_dir = tempfile::tempdir().map_err(|e| isolate_core::Error::IoError(e.to_string()))?;

    // Initialize jj repo (using git backend for compatibility)
    let output = std::process::Command::new("jj")
        .args(["git", "init", "."])
        .current_dir(temp_dir.path())
        .output()
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to run jj git init: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(isolate_core::Error::JjCommandError {
            operation: "init test repo".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    Ok(temp_dir)
}
