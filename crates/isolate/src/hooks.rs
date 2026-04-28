//! Workflow hooks - Run commands on success or failure
//!
//! Provides --on-success and --on-failure hooks for command execution.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::Result;

/// Hook configuration
#[derive(Debug, Clone, Default)]
pub struct HooksConfig {
    /// Command to run on success
    pub on_success: Option<String>,
    /// Command to run on failure
    pub on_failure: Option<String>,
}

/// Result of running a hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// Which hook was run
    pub hook: String,
    /// Whether the hook succeeded
    pub success: bool,
    /// Hook command that was run
    pub command: String,
    /// Output from the hook
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Error from the hook if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl HooksConfig {
    /// Validate a callback command string is non-empty after trimming whitespace
    fn validate_command(cmd: &str) -> Result<&str> {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            Err(crate::IsolateError::OperationFailed(
                "callback command cannot be empty or whitespace-only".to_string(),
            ))
        } else {
            Ok(trimmed)
        }
    }

    /// Create a new hooks config from command line args
    ///
    /// Returns an error if either command is empty or contains only whitespace.
    pub fn from_args(on_success: Option<String>, on_failure: Option<String>) -> Result<Self> {
        let validated_on_success = match on_success {
            Some(cmd) => Some(Self::validate_command(&cmd)?.to_string()),
            None => None,
        };

        let validated_on_failure = match on_failure {
            Some(cmd) => Some(Self::validate_command(&cmd)?.to_string()),
            None => None,
        };

        Ok(Self {
            on_success: validated_on_success,
            on_failure: validated_on_failure,
        })
    }

    /// Check if any hooks are configured
    #[must_use]
    pub const fn has_hooks(&self) -> bool {
        self.on_success.is_some() || self.on_failure.is_some()
    }

    /// Run the appropriate hook based on result
    pub async fn run_hook(&self, success: bool) -> Option<HookResult> {
        let (hook_name, hook_cmd) = if success {
            ("on_success", &self.on_success)
        } else {
            ("on_failure", &self.on_failure)
        };

        if let Some(cmd) = hook_cmd.as_ref() {
            Some(run_hook_command(hook_name, cmd))
        } else {
            None
        }
    }
}

/// Run a hook command and print output for visibility
fn run_hook_command(hook_name: &str, command: &str) -> HookResult {
    // Run the command through the shell (synchronous)
    let result = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", command]).output()
    } else {
        Command::new("sh").args(["-c", command]).output()
    };

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Print hook execution output for visibility
            eprintln!("Hook [{hook_name}]: {command}");
            if !stdout.is_empty() {
                eprintln!("Hook stdout:\n{stdout}");
            }
            if !stderr.is_empty() {
                eprintln!("Hook stderr:\n{stderr}");
            }

            let exit_code_msg = output.status.code().map_or_else(
                || "terminated by signal".to_string(),
                |code| format!("exited with code: {code}"),
            );

            if output.status.success() {
                HookResult {
                    hook: hook_name.to_string(),
                    success: true,
                    command: command.to_string(),
                    output: if stdout.is_empty() { None } else { Some(stdout) },
                    error: None,
                }
            } else {
                HookResult {
                    hook: hook_name.to_string(),
                    success: false,
                    command: command.to_string(),
                    output: if stdout.is_empty() { None } else { Some(stdout) },
                    error: Some(if stderr.is_empty() {
                        exit_code_msg
                    } else {
                        stderr
                    }),
                }
            }
        }
        Err(e) => {
            eprintln!("Hook [{hook_name}] failed to execute: {e}");
            HookResult {
                hook: hook_name.to_string(),
                success: false,
                command: command.to_string(),
                output: None,
                error: Some(format!("Failed to execute hook: {e}")),
            }
        }
    }
}

/// Wrapper to execute a command with hooks
#[allow(dead_code)]
pub async fn with_hooks<F, Fut>(hooks: &HooksConfig, f: F) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let result = f().await;
    let success = result.is_ok();

    // Run the appropriate hook and print results
    if let Some(hook_result) = hooks.run_hook(success).await {
        // Return error if hook failed
        if !hook_result.success {
            let error_msg = hook_result
                .error
                .as_ref()
                .map_or("unknown error", |msg| msg.as_str());
            return Err(crate::IsolateError::OperationFailed(format!(
                "Hook '{}' failed: {}",
                hook_result.hook,
                error_msg
            )));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooks_config_default() {
        let config = HooksConfig::default();
        assert!(!config.has_hooks());
    }

    #[test]
    fn test_hooks_config_with_success() {
        let config = HooksConfig::from_args(Some("echo success".to_string()), None).unwrap();
        assert!(config.has_hooks());
    }

    #[test]
    fn test_hooks_config_with_failure() {
        let config = HooksConfig::from_args(None, Some("echo failed".to_string())).unwrap();
        assert!(config.has_hooks());
    }

    #[test]
    fn test_hooks_config_rejects_empty_success() {
        let result = HooksConfig::from_args(Some(String::new()), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_hooks_config_rejects_whitespace_only_success() {
        let result = HooksConfig::from_args(Some("   ".to_string()), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_hooks_config_accepts_whitespace_padded_command() {
        let config = HooksConfig::from_args(Some("  echo test  ".to_string()), None).unwrap();
        assert_eq!(config.on_success, Some("echo test".to_string()));
    }

    #[test]
    fn test_hook_result_serialization() -> Result<()> {
        let result = HookResult {
            hook: "on_success".to_string(),
            success: true,
            command: "echo test".to_string(),
            output: Some("test\n".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"hook\":\"on_success\""));
        Ok(())
    }

    #[test]
    fn test_run_hook_echo() {
        let result = run_hook_command("test", "echo hello");
        assert!(result.success);
        assert!(result.output.is_some());
    }

    #[test]
    fn test_run_hook_false() {
        let result = run_hook_command("test", "false");
        assert!(!result.success);
    }
}
