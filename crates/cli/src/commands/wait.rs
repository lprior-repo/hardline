//! Wait command - blocking primitive for session conditions

use scp_core::{
    vcs::{create_backend, Workspace},
    Result,
};
use std::time::{Duration, Instant};

/// Wait modes for the wait command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitMode {
    SessionExists,
    Healthy,
    Status(String),
}

impl WaitMode {
    /// Parse wait mode from string
    pub fn parse(s: &str) -> Result<Self> {
        let lower = s.to_lowercase();
        if lower == "session-exists" {
            Ok(Self::SessionExists)
        } else if lower == "healthy" {
            Ok(Self::Healthy)
        } else if let Some(status_str) = lower.strip_prefix("status=") {
            Ok(Self::Status(status_str.to_string()))
        } else {
            Err(scp_core::Error::InvalidWaitMode(format!(
                "unknown mode: {}",
                s
            )))
        }
    }

    /// Get display string for the mode
    pub fn display(&self) -> String {
        match self {
            Self::SessionExists => "session-exists".to_string(),
            Self::Healthy => "healthy".to_string(),
            Self::Status(state) => format!("status={}", state),
        }
    }
}

/// Wait for a session condition to be met
pub fn run(
    session_name: &str,
    mode_str: &str,
    timeout_secs: Option<u64>,
    poll_interval_secs: u64,
) -> Result<()> {
    // Validate session name
    if session_name.is_empty() {
        return Err(scp_core::Error::InvalidIdentifier(
            "session name cannot be empty".to_string(),
        ));
    }

    // Parse wait mode
    let mode = WaitMode::parse(mode_str)?;

    // Validate timeout
    if let Some(0) = timeout_secs {
        return Err(scp_core::Error::ValidationError(
            "timeout must be > 0".to_string(),
        ));
    }

    // Clamp poll interval
    let poll_interval = Duration::from_secs(poll_interval_secs.clamp(1, 60));

    // Get timeout duration
    let timeout = timeout_secs.map(Duration::from_secs);

    // Wait loop
    let start = Instant::now();

    loop {
        // Check if we've hit the timeout
        if let Some(t) = timeout {
            if start.elapsed() > t {
                return Err(scp_core::Error::WaitTimeout(
                    session_name.to_string(),
                    mode.display(),
                ));
            }
        }

        // Check the condition
        match check_condition(session_name, &mode) {
            Ok(true) => {
                // Condition met!
                println!(
                    "Condition met: session '{}' is {}",
                    session_name,
                    mode.display()
                );
                return Ok(());
            }
            Ok(false) => {
                // Condition not met, continue waiting
            }
            Err(e) => {
                // Session doesn't exist yet - for session-exists mode, this is expected
                if matches!(mode, WaitMode::SessionExists) {
                    // Continue waiting
                } else {
                    return Err(e);
                }
            }
        }

        // Sleep for poll interval
        std::thread::sleep(poll_interval);
    }
}

/// Check if the wait condition is met
fn check_condition(session_name: &str, mode: &WaitMode) -> Result<bool> {
    let cwd = std::env::current_dir().map_err(|e| scp_core::Error::Io(e))?;
    let backend = create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;

    // Find the workspace
    let workspace = workspaces.iter().find(|w| w.name == session_name);

    match mode {
        WaitMode::SessionExists => Ok(workspace.is_some()),
        WaitMode::Healthy => match workspace {
            Some(_ws) => Ok(true),
            None => Ok(false),
        },
        WaitMode::Status(expected) => match workspace {
            Some(ws) => Ok(ws.branch.contains(expected)),
            None => Ok(false),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_mode_parse_session_exists() {
        let mode = WaitMode::parse("session-exists").unwrap();
        assert!(matches!(mode, WaitMode::SessionExists));
    }

    #[test]
    fn test_wait_mode_parse_healthy() {
        let mode = WaitMode::parse("healthy").unwrap();
        assert!(matches!(mode, WaitMode::Healthy));
    }

    #[test]
    fn test_wait_mode_parse_status() {
        let mode = WaitMode::parse("status=Active").unwrap();
        assert!(matches!(mode, WaitMode::Status(s) if s == "Active"));
    }

    #[test]
    fn test_wait_mode_parse_invalid() {
        let result = WaitMode::parse("invalid-mode");
        assert!(result.is_err());
    }

    #[test]
    fn test_wait_mode_display() {
        assert_eq!(WaitMode::SessionExists.display(), "session-exists");
        assert_eq!(WaitMode::Healthy.display(), "healthy");
        assert_eq!(
            WaitMode::Status("Active".to_string()).display(),
            "status=Active"
        );
    }
}
