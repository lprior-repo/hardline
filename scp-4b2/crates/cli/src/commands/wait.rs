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

/// Validate session name is not empty
fn validate_session_name(session_name: &str) -> Result<()> {
    if session_name.is_empty() {
        Err(scp_core::Error::InvalidIdentifier(
            "session name cannot be empty".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Validate timeout value (must be Some(x) where x > 0, or None)
fn validate_timeout(timeout_secs: Option<u64>) -> Result<()> {
    if Some(0) == timeout_secs {
        Err(scp_core::Error::ValidationError(
            "timeout must be > 0".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Calculate poll interval from seconds, clamped to valid range
fn calculate_poll_interval(poll_interval_secs: u64) -> Duration {
    Duration::from_secs(poll_interval_secs.clamp(1, 60))
}

/// Convert timeout seconds to Duration
fn calculate_timeout_duration(timeout_secs: Option<u64>) -> Option<Duration> {
    timeout_secs.map(Duration::from_secs)
}

/// Check if timeout has elapsed
fn is_timeout_elapsed(start: Instant, timeout: Option<Duration>) -> bool {
    timeout.map_or(false, |t| start.elapsed() > t)
}

/// Build error for wait timeout
fn wait_timeout_error(session_name: &str, mode: &WaitMode) -> scp_core::Error {
    scp_core::Error::WaitTimeout(session_name.to_string(), mode.display())
}

/// Find workspace by name from list
fn find_workspace<'a>(workspaces: &'a [Workspace], session_name: &str) -> Option<&'a Workspace> {
    workspaces.iter().find(|w| w.name == session_name)
}

/// Evaluate wait condition based on mode
fn evaluate_wait_condition(workspace: Option<&Workspace>, mode: &WaitMode) -> bool {
    match mode {
        WaitMode::SessionExists => workspace.is_some(),
        WaitMode::Healthy => workspace.is_some(),
        WaitMode::Status(expected) => workspace.map_or(false, |ws| ws.branch.contains(expected)),
    }
}

/// Check if the wait condition is met
fn check_condition(session_name: &str, mode: &WaitMode) -> Result<bool> {
    let cwd = std::env::current_dir().map_err(scp_core::Error::Io)?;
    let backend = create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;
    let workspace = find_workspace(&workspaces, session_name);
    Ok(evaluate_wait_condition(workspace, mode))
}

/// Format success message for condition met
fn format_condition_met_message(session_name: &str, mode: &WaitMode) -> String {
    format!(
        "Condition met: session '{}' is {}",
        session_name,
        mode.display()
    )
}

/// Handle check_condition result, returning Ok(()) if condition met, Err if error
fn handle_condition_result(
    result: Result<bool>,
    session_name: &str,
    mode: &WaitMode,
) -> Result<Option<()>> {
    match result {
        Ok(true) => {
            println!("{}", format_condition_met_message(session_name, mode));
            Ok(Some(()))
        }
        Ok(false) => Ok(None),
        Err(e) if matches!(mode, WaitMode::SessionExists) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Run the wait loop until condition is met or timeout
fn run_wait_loop(
    session_name: &str,
    mode: &WaitMode,
    poll_interval: Duration,
    timeout: Option<Duration>,
) -> Result<()> {
    let start = Instant::now();

    loop {
        if is_timeout_elapsed(start, timeout) {
            return Err(wait_timeout_error(session_name, mode));
        }

        let result = check_condition(session_name, mode);
        if let Some(()) = handle_condition_result(result, session_name, mode)? {
            return Ok(());
        }

        std::thread::sleep(poll_interval);
    }
}

/// Wait for a session condition to be met
pub fn run(
    session_name: &str,
    mode_str: &str,
    timeout_secs: Option<u64>,
    poll_interval_secs: u64,
) -> Result<()> {
    validate_session_name(session_name)?;
    let mode = WaitMode::parse(mode_str)?;
    validate_timeout(timeout_secs)?;

    let poll_interval = calculate_poll_interval(poll_interval_secs);
    let timeout = calculate_timeout_duration(timeout_secs);

    run_wait_loop(session_name, &mode, poll_interval, timeout)
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

    #[test]
    fn test_validate_session_name_empty() {
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_session_name_valid() {
        let result = validate_session_name("valid");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_timeout_zero() {
        let result = validate_timeout(Some(0));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_timeout_none() {
        let result = validate_timeout(None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_poll_interval_clamp() {
        assert_eq!(calculate_poll_interval(0).as_secs(), 1);
        assert_eq!(calculate_poll_interval(30).as_secs(), 30);
        assert_eq!(calculate_poll_interval(100).as_secs(), 60);
    }

    #[test]
    fn test_evaluate_wait_condition_session_exists() {
        assert!(evaluate_wait_condition(
            Some(&Workspace::default()),
            &WaitMode::SessionExists
        ));
        assert!(!evaluate_wait_condition(None, &WaitMode::SessionExists));
    }

    #[test]
    fn test_evaluate_wait_condition_status() {
        let ws = Workspace {
            name: "test".to_string(),
            branch: "Active".to_string(),
            ..Default::default()
        };
        assert!(evaluate_wait_condition(
            Some(&ws),
            &WaitMode::Status("Active".to_string())
        ));
        assert!(!evaluate_wait_condition(
            Some(&ws),
            &WaitMode::Status("Inactive".to_string())
        ));
        assert!(!evaluate_wait_condition(
            None,
            &WaitMode::Status("Active".to_string())
        ));
    }
}
