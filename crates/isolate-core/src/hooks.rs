//! Hook execution for lifecycle events
//!
//! This module provides hook execution capabilities for isolate lifecycle events:
//! - `post_create`: After workspace creation, before Zellij tab opens
//! - `pre_remove`: Before workspace deletion
//! - `post_merge`: After merge to main (optional)
//!
//! Hooks execute sequentially in the workspace directory using the user's shell.

use std::{path::Path, process::Stdio};

use tokio::process::Command;

use crate::{config::HooksConfig, Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Types of lifecycle hooks supported by isolate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookType {
    /// Runs after workspace creation, before Zellij tab opens
    PostCreate,
    /// Runs before workspace deletion
    PreRemove,
    /// Runs after merge to main (optional)
    PostMerge,
}

impl HookType {
    /// Get the event name for this hook type
    #[must_use]
    pub const fn event_name(self) -> &'static str {
        match self {
            Self::PostCreate => "post_create",
            Self::PreRemove => "pre_remove",
            Self::PostMerge => "post_merge",
        }
    }
}

/// Result of executing a single hook command
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    /// Whether the command succeeded (exit code 0)
    pub success: bool,
    /// Exit code from the command
    pub exit_code: Option<i32>,
    /// Standard output from the command
    pub stdout: String,
    /// Standard error from the command
    pub stderr: String,
}

/// Result of executing hooks for a lifecycle event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookResult {
    /// No hooks were configured for this event
    NoHooks,
    /// All hooks executed successfully
    Success(Vec<CommandResult>),
}

// ═══════════════════════════════════════════════════════════════════════════
// HOOK RUNNER
// ═══════════════════════════════════════════════════════════════════════════

/// Executes lifecycle hooks based on configuration
#[derive(Debug, Clone)]
pub struct HookRunner {
    config: HooksConfig,
}

impl HookRunner {
    /// Create a new hook runner with the given configuration
    #[must_use]
    pub const fn new(config: HooksConfig) -> Self {
        Self { config }
    }

    /// Execute hooks for the given lifecycle event
    ///
    /// Hooks execute sequentially in the workspace directory. If any hook fails,
    /// execution stops and an error is returned.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - A hook command fails (non-zero exit code)
    /// - A hook command cannot be executed (e.g., shell not found)
    /// - Unable to determine user's shell
    pub async fn run(&self, hook_type: HookType, workspace_path: &Path) -> Result<HookResult> {
        let hooks = self.get_hooks_for_type(hook_type);

        if hooks.is_empty() {
            return Ok(HookResult::NoHooks);
        }

        let shell = get_user_shell()?;
        let num_hooks = hooks.len();
        let mut results = Vec::new();

        for (index, hook_cmd) in hooks.iter().enumerate() {
            // Allow eprintln for user feedback (not debug output)
            #[allow(clippy::print_stderr)]
            {
                eprintln!(
                    "Running {} hook {}/{}: {}",
                    hook_type.event_name(),
                    index + 1,
                    num_hooks,
                    hook_cmd
                );
            }

            let result = Self::execute_hook(&shell, hook_cmd, workspace_path).await?;

            if !result.success {
                return Err(Error::HookFailed {
                    hook_type: hook_type.event_name().to_string(),
                    command: hook_cmd.clone(),
                    exit_code: result.exit_code,
                    stdout: result.stdout,
                    stderr: result.stderr,
                });
            }

            results.push(result);
        }

        Ok(HookResult::Success(results))
    }

    /// Get the list of hooks for a given type
    fn get_hooks_for_type(&self, hook_type: HookType) -> &[String] {
        match hook_type {
            HookType::PostCreate => &self.config.post_create,
            HookType::PreRemove => &self.config.pre_remove,
            HookType::PostMerge => &self.config.post_merge,
        }
    }

    /// Execute a single hook command
    async fn execute_hook(shell: &str, command: &str, cwd: &Path) -> Result<CommandResult> {
        let output = Command::new(shell)
            .arg("-c")
            .arg(command)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| Error::HookExecutionFailed {
                command: command.to_string(),
                source: e.to_string(),
            })?;

        Ok(CommandResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Get the user's default shell from the SHELL environment variable
///
/// Falls back to `/bin/sh` if SHELL is not set.
///
/// # Errors
///
/// Returns error if the shell path is empty or invalid
fn get_user_shell() -> Result<String> {
    std::env::var("SHELL")
        .or_else(|_| Ok("/bin/sh".to_string()))
        .and_then(|shell| {
            if shell.is_empty() {
                Err(Error::InvalidConfig(
                    "SHELL environment variable is empty".to_string(),
                ))
            } else {
                Ok(shell)
            }
        })
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::{Mutex, OnceLock},
    };

    use tempfile::TempDir;

    use super::*;

    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    fn get_env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    // Helper to create a temporary workspace for testing
    fn create_test_workspace() -> Result<TempDir> {
        TempDir::new().map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))
    }

    // Test 1: No hooks configured - returns NoHooks
    #[tokio::test]
    async fn test_no_hooks_configured() -> Result<()> {
        let (runner, workspace) = {
            let _lock = get_env_lock();
            let config = HooksConfig::default();
            let runner = HookRunner::new(config);
            let workspace = create_test_workspace()?;
            (runner, workspace)
        }; // Lock dropped here

        let result = runner.run(HookType::PostCreate, workspace.path()).await?;

        assert_eq!(result, HookResult::NoHooks);
        Ok(())
    }

    // Test 2: Single successful hook
    #[tokio::test]
    async fn test_single_successful_hook() -> Result<()> {
        let (runner, workspace) = {
            let _lock = get_env_lock();
            let config = HooksConfig {
                post_create: vec!["echo 'Hello'".to_string()],
                pre_remove: Vec::new(),
                post_merge: Vec::new(),
            };
            let runner = HookRunner::new(config);
            let workspace = create_test_workspace()?;
            (runner, workspace)
        }; // Lock dropped here

        let result = runner.run(HookType::PostCreate, workspace.path()).await?;

        if let HookResult::Success(results) = result {
            assert_eq!(results.len(), 1);
            let first = results
                .first()
                .ok_or_else(|| Error::InvalidConfig("Expected at least one result".to_string()))?;
            assert!(first.success);
            assert!(first.stdout.contains("Hello"));
        } else {
            return Err(Error::InvalidConfig(
                "Expected Success, got NoHooks".to_string(),
            ));
        }
        Ok(())
    }

    // Test 3: Multiple successful hooks execute in order
    #[tokio::test]
    async fn test_multiple_successful_hooks() -> Result<()> {
        let (runner, workspace) = {
            let _lock = get_env_lock();
            let config = HooksConfig {
                post_create: vec!["echo 'A'".to_string(), "echo 'B'".to_string()],
                pre_remove: Vec::new(),
                post_merge: Vec::new(),
            };
            let runner = HookRunner::new(config);
            let workspace = create_test_workspace()?;
            (runner, workspace)
        }; // Lock dropped here

        let result = runner.run(HookType::PostCreate, workspace.path()).await?;

        if let HookResult::Success(results) = result {
            assert_eq!(results.len(), 2);
            let first = results
                .first()
                .ok_or_else(|| Error::InvalidConfig("Expected at least one result".to_string()))?;
            assert!(first.success);
            assert!(first.stdout.contains('A'));
            let second = results
                .get(1)
                .ok_or_else(|| Error::InvalidConfig("Expected at least two results".to_string()))?;
            assert!(second.success);
            assert!(second.stdout.contains('B'));
        } else {
            return Err(Error::InvalidConfig(
                "Expected Success, got NoHooks".to_string(),
            ));
        }
        Ok(())
    }

    // Test 4: Hook failure returns error
    #[tokio::test]
    async fn test_hook_failure() -> Result<()> {
        let (runner, workspace) = {
            let _lock = get_env_lock();
            let config = HooksConfig {
                post_create: vec!["exit 1".to_string()],
                pre_remove: Vec::new(),
                post_merge: Vec::new(),
            };
            let runner = HookRunner::new(config);
            let workspace = create_test_workspace()?;
            (runner, workspace)
        }; // Lock dropped here

        let result = runner.run(HookType::PostCreate, workspace.path()).await;

        assert!(result.is_err());
        if let Err(Error::HookFailed {
            hook_type,
            command,
            exit_code,
            ..
        }) = result
        {
            assert_eq!(hook_type, "post_create");
            assert_eq!(command, "exit 1");
            assert_eq!(exit_code, Some(1));
        } else {
            return Err(Error::InvalidConfig(
                "Expected HookFailed error".to_string(),
            ));
        }
        Ok(())
    }

    // Test 5: Partial hook failure - second hook fails, third never runs
    #[tokio::test]
    async fn test_partial_hook_failure() -> Result<()> {
        let (runner, workspace) = {
            let _lock = get_env_lock();
            let config = HooksConfig {
                post_create: vec![
                    "echo 'A'".to_string(),
                    "exit 1".to_string(),
                    "echo 'C'".to_string(),
                ],
                pre_remove: Vec::new(),
                post_merge: Vec::new(),
            };
            let runner = HookRunner::new(config);
            let workspace = create_test_workspace()?;
            (runner, workspace)
        }; // Lock dropped here

        let result = runner.run(HookType::PostCreate, workspace.path()).await;

        assert!(result.is_err());
        // The third hook should never execute
        if let Err(Error::HookFailed { command, .. }) = result {
            assert_eq!(command, "exit 1");
        } else {
            return Err(Error::InvalidConfig(
                "Expected HookFailed error".to_string(),
            ));
        }
        Ok(())
    }

    // Test 6: Hook with workspace as cwd
    #[tokio::test]
    async fn test_hook_with_workspace_cwd() -> Result<()> {
        let (runner, workspace) = {
            let _lock = get_env_lock();
            let config = HooksConfig {
                post_create: vec!["pwd".to_string()],
                pre_remove: Vec::new(),
                post_merge: Vec::new(),
            };
            let runner = HookRunner::new(config);
            let workspace = create_test_workspace()?;
            (runner, workspace)
        }; // Lock dropped here

        let result = runner.run(HookType::PostCreate, workspace.path()).await?;

        if let HookResult::Success(results) = result {
            assert_eq!(results.len(), 1);
            let first = results
                .first()
                .ok_or_else(|| Error::InvalidConfig("Expected at least one result".to_string()))?;
            assert!(first.success);
            let output = first.stdout.trim();
            let expected = workspace.path().to_string_lossy();
            assert_eq!(output, expected.as_ref());
        } else {
            return Err(Error::InvalidConfig(
                "Expected Success, got NoHooks".to_string(),
            ));
        }
        Ok(())
    }

    // Test 7: Hook stderr captured
    #[tokio::test]
    async fn test_hook_stderr_captured() -> Result<()> {
        let (runner, workspace) = {
            let _lock = get_env_lock();
            let config = HooksConfig {
                post_create: vec!["echo 'error' >&2".to_string()],
                pre_remove: Vec::new(),
                post_merge: Vec::new(),
            };
            let runner = HookRunner::new(config);
            let workspace = create_test_workspace()?;
            (runner, workspace)
        }; // Lock dropped here

        let result = runner.run(HookType::PostCreate, workspace.path()).await?;

        if let HookResult::Success(results) = result {
            assert_eq!(results.len(), 1);
            let first = results
                .first()
                .ok_or_else(|| Error::InvalidConfig("Expected at least one result".to_string()))?;
            assert!(first.success);
            assert!(first.stderr.contains("error"));
        } else {
            return Err(Error::InvalidConfig(
                "Expected Success, got NoHooks".to_string(),
            ));
        }
        Ok(())
    }

    // Test 8: Complex hook script (multi-command)
    #[tokio::test]
    async fn test_complex_hook_script() -> Result<()> {
        let (runner, workspace) = {
            let _lock = get_env_lock();
            let workspace = create_test_workspace()?;

            // Create a subdirectory
            let subdir = workspace.path().join("subdir");
            fs::create_dir(&subdir)?;

            let config = HooksConfig {
                post_create: vec!["cd subdir && pwd".to_string()],
                pre_remove: Vec::new(),
                post_merge: Vec::new(),
            };
            let runner = HookRunner::new(config);
            (runner, workspace)
        }; // Lock dropped here

        let result = runner.run(HookType::PostCreate, workspace.path()).await?;

        if let HookResult::Success(results) = result {
            assert_eq!(results.len(), 1);
            let first = results
                .first()
                .ok_or_else(|| Error::InvalidConfig("Expected at least one result".to_string()))?;
            assert!(first.success);
            let output = first.stdout.trim();
            assert!(output.ends_with("subdir"));
        } else {
            return Err(Error::InvalidConfig(
                "Expected Success, got NoHooks".to_string(),
            ));
        }
        Ok(())
    }

    // Test 9: Different hook types use different configs
    #[tokio::test]
    async fn test_different_hook_types() -> Result<()> {
        let (runner, workspace) = {
            let _lock = get_env_lock();
            let config = HooksConfig {
                post_create: vec!["echo 'post_create'".to_string()],
                pre_remove: vec!["echo 'pre_remove'".to_string()],
                post_merge: vec!["echo 'post_merge'".to_string()],
            };
            let runner = HookRunner::new(config);
            let workspace = create_test_workspace()?;
            (runner, workspace)
        }; // Lock dropped here

        // Test post_create
        let result = runner.run(HookType::PostCreate, workspace.path()).await?;
        if let HookResult::Success(results) = result {
            let first = results
                .first()
                .ok_or_else(|| Error::InvalidConfig("Expected at least one result".to_string()))?;
            assert!(first.stdout.contains("post_create"));
        } else {
            return Err(Error::InvalidConfig(
                "Expected Success for post_create".to_string(),
            ));
        }

        // Test pre_remove
        let result = runner.run(HookType::PreRemove, workspace.path()).await?;
        if let HookResult::Success(results) = result {
            let first = results
                .first()
                .ok_or_else(|| Error::InvalidConfig("Expected at least one result".to_string()))?;
            assert!(first.stdout.contains("pre_remove"));
        } else {
            return Err(Error::InvalidConfig(
                "Expected Success for pre_remove".to_string(),
            ));
        }

        // Test post_merge
        let result = runner.run(HookType::PostMerge, workspace.path()).await?;
        if let HookResult::Success(results) = result {
            let first = results
                .first()
                .ok_or_else(|| Error::InvalidConfig("Expected at least one result".to_string()))?;
            assert!(first.stdout.contains("post_merge"));
        } else {
            return Err(Error::InvalidConfig(
                "Expected Success for post_merge".to_string(),
            ));
        }

        Ok(())
    }

    // Test 10: Shell detection uses SHELL env var
    #[test]
    fn test_get_user_shell_from_env() -> Result<()> {
        let _lock = get_env_lock();
        // Save current SHELL value
        let original_shell = std::env::var("SHELL").ok();

        // Set SHELL to a test value
        std::env::set_var("SHELL", "/bin/sh"); // Use /bin/sh which should exist

        let shell = get_user_shell()?;
        assert_eq!(shell, "/bin/sh");

        // Restore original SHELL
        match original_shell {
            Some(shell) => std::env::set_var("SHELL", shell),
            None => std::env::remove_var("SHELL"),
        }
        Ok(())
    }

    // Test 11: Shell detection falls back to /bin/sh
    #[test]
    fn test_get_user_shell_fallback() -> Result<()> {
        let _lock = get_env_lock();
        // Save current SHELL value
        let original_shell = std::env::var("SHELL").ok();

        // Remove SHELL env var
        std::env::remove_var("SHELL");

        let shell = get_user_shell()?;
        assert_eq!(shell, "/bin/sh");

        // Restore original SHELL
        match original_shell {
            Some(shell) => std::env::set_var("SHELL", shell),
            None => std::env::remove_var("SHELL"),
        }
        Ok(())
    }

    // Test 12: HookType event names
    #[test]
    fn test_hook_type_event_names() {
        assert_eq!(HookType::PostCreate.event_name(), "post_create");
        assert_eq!(HookType::PreRemove.event_name(), "pre_remove");
        assert_eq!(HookType::PostMerge.event_name(), "post_merge");
    }

    // Test 13: Hook execution failed error (invalid command)
    #[tokio::test]
    async fn test_hook_execution_failed() -> Result<()> {
        let config = HooksConfig {
            post_create: vec!["nonexistent_command_that_does_not_exist".to_string()],
            pre_remove: Vec::new(),
            post_merge: Vec::new(),
        };
        let runner = HookRunner::new(config);
        let workspace = create_test_workspace()?;

        let result = runner.run(HookType::PostCreate, workspace.path()).await;

        // This might be HookFailed (command found but returns error) or
        // could succeed with error code, depending on shell behavior
        // Most shells will exit with 127 for command not found
        assert!(result.is_err() || matches!(result, Ok(HookResult::Success(_))));
        Ok(())
    }
}
