//! Common test utilities for isolate integration tests
//!
//! This module provides testing infrastructure adapted from isolate for hardline.

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

use std::{path::PathBuf, sync::Arc};

use scp_isolate::Result;
use serde_json::Value as JsonValue;
use tempfile::TempDir;

/// Result of a command execution
///
/// # Performance Note
///
/// Uses String instead of Cow<str> for simplicity in test code.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Whether the command succeeded
    pub success: bool,
    /// Exit code (if available)
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
}

impl CommandResult {
    /// Assert that the command succeeded
    #[inline]
    pub fn assert_success(&self) {
        assert!(
            self.success,
            "Command failed\nExit code: {:?}\nStdout: {}\nStderr: {}",
            self.exit_code, self.stdout, self.stderr
        );
    }

    /// Assert that stdout contains a string
    #[inline]
    pub fn assert_stdout_contains(&self, s: &str) {
        assert!(
            self.stdout.contains(s),
            "Stdout should contain '{}'\nGot: {}",
            s,
            self.stdout
        );
    }

    /// Assert that stderr contains a string
    #[inline]
    pub fn assert_stderr_contains(&self, s: &str) {
        assert!(
            self.stderr.contains(s),
            "Stderr should contain '{}'\nGot: {}",
            s,
            self.stderr
        );
    }
}

/// Test harness for integration tests
///
/// Provides a clean temporary environment with a JJ repository
/// and utilities to execute isolate commands.
///
/// Note: Since hardline's isolate is a library crate (scp_isolate), this harness
/// uses the library API rather than CLI commands.
pub struct TestHarness {
    /// Temporary directory for the test (kept for automatic cleanup on drop)
    _temp_dir: TempDir,
    /// Path to the JJ repository root
    pub repo_path: PathBuf,
    /// Current working directory for commands (defaults to `repo_path`)
    pub current_dir: PathBuf,
}

impl TestHarness {
    /// Create a new test harness with a fresh JJ repository
    ///
    /// # Errors
    ///
    /// Returns an error if jj is not available or repository initialization fails.
    pub fn new() -> Result<Self> {
        let temp_dir =
            tempfile::tempdir().map_err(|e| scp_isolate::IsolateError::IoError(e.to_string()))?;

        let repo_path = temp_dir.path().to_path_buf();

        // Initialize JJ repository
        let output = std::process::Command::new("jj")
            .args(["git", "init", "."])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| {
                scp_isolate::IsolateError::IoError(format!("Failed to run jj git init: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(scp_isolate::IsolateError::OperationFailed(format!(
                "jj git init failed: {}",
                stderr
            )));
        }

        // Create an initial commit
        std::fs::write(repo_path.join("README.md"), "# Test Repository\n").map_err(|e| {
            scp_isolate::IsolateError::IoError(format!("Failed to create README: {e}"))
        })?;

        let output = std::process::Command::new("jj")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| {
                scp_isolate::IsolateError::IoError(format!("Failed to run jj commit: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(scp_isolate::IsolateError::OperationFailed(format!(
                "jj commit failed: {}",
                stderr
            )));
        }

        // Create main bookmark
        let output = std::process::Command::new("jj")
            .args(["bookmark", "create", "main"])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| {
                scp_isolate::IsolateError::IoError(format!("Failed to create bookmark: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(scp_isolate::IsolateError::OperationFailed(format!(
                "jj bookmark create main failed: {}",
                stderr
            )));
        }

        Ok(Self {
            _temp_dir: temp_dir,
            repo_path: repo_path.clone(),
            current_dir: repo_path,
        })
    }

    /// Try to create a new test harness, returning None if initialization fails
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Get the .isolate directory path
    #[must_use]
    pub fn isolate_dir(&self) -> PathBuf {
        self.repo_path.join(".isolate")
    }

    /// Get the workspace path for a session
    #[must_use]
    pub fn workspace_path(&self, session: &str) -> PathBuf {
        self.repo_path.join("workspaces").join(session)
    }

    /// Get the workspaces directory
    #[must_use]
    pub fn workspaces_dir(&self) -> PathBuf {
        self.repo_path.join("workspaces")
    }

    /// Run jj in the test repository
    pub fn jj_in_dir(&self, dir: &PathBuf, args: &[&str]) -> CommandResult {
        let output = std::process::Command::new("jj")
            .args(args)
            .current_dir(dir)
            .output();

        match output {
            Ok(output) => CommandResult {
                success: output.status.success(),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            },
            Err(e) => CommandResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Failed to execute jj: {e}"),
            },
        }
    }

    /// Run jj in the test repository and assert success
    pub fn jj_in_dir_assert_success(&self, dir: &PathBuf, args: &[&str]) {
        let result = self.jj_in_dir(dir, args);
        result.assert_success();
    }
}

/// Parse JSONL output from CLI commands, returning all parsed JSON lines.
///
/// JSONL format has one JSON object per line. This function filters empty lines
/// and parses each remaining line as JSON.
pub fn parse_jsonl_output(s: &str) -> Result<Vec<JsonValue>, serde_json::Error> {
    s.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && trimmed.starts_with('{')
        })
        .map(serde_json::from_str)
        .collect()
}

/// Get the first JSON line from JSONL output that matches a given variant type.
pub fn find_jsonl_line_by_type<'a>(
    lines: &'a [JsonValue],
    type_name: &str,
) -> Option<&'a JsonValue> {
    lines.iter().find(|line| line.get(type_name).is_some())
}

/// Get all JSON lines from JSONL output that match a given variant type.
pub fn filter_jsonl_lines_by_type<'a>(
    lines: &'a [JsonValue],
    type_name: &'a str,
) -> impl Iterator<Item = &'a JsonValue> {
    lines
        .iter()
        .filter(move |line| line.get(type_name).is_some())
}

/// Session test context that holds state for each scenario
///
/// Uses Arc<Mutex<>> for thread-safe sharing across async steps.
pub struct SessionTestContext {
    /// The test harness for running commands
    pub harness: TestHarness,
    /// Track the last session name for assertions
    pub last_session: Arc<tokio::sync::Mutex<Option<String>>>,
    /// Track the last operation result
    pub last_result: Arc<tokio::sync::Mutex<Option<CommandResult>>>,
    /// Track created sessions for cleanup
    pub created_sessions: Arc<tokio::sync::Mutex<Vec<String>>>,
}

impl SessionTestContext {
    /// Create a new session test context
    pub fn new() -> Result<Self> {
        let harness = TestHarness::new()?;
        Ok(Self {
            harness,
            last_session: Arc::new(tokio::sync::Mutex::new(None)),
            last_result: Arc::new(tokio::sync::Mutex::new(None)),
            created_sessions: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        })
    }

    /// Try to create a new context, returning None if initialization fails
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Initialize the Isolate database
    pub fn init_isolate(&self) -> Result<()> {
        // Create the .isolate directory to simulate initialization
        let isolate_dir = self.harness.isolate_dir();
        std::fs::create_dir_all(&isolate_dir).map_err(|e| {
            scp_isolate::IsolateError::IoError(format!(
                "Failed to create .isolate directory: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// Store a session name for later cleanup
    pub async fn track_session(&self, name: &str) {
        self.created_sessions.lock().await.push(name.to_string());
        *self.last_session.lock().await = Some(name.to_string());
    }
}
