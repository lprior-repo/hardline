//! Wait command - blocking primitive for session conditions

use scp_core::{
    vcs::{create_backend, Workspace},
    Result,
};
use std::time::{Duration, Instant};

// ========================================================================
// Data Layer: Newtypes
// ========================================================================

/// Session name - borrowed, read-only representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionName<'a>(&'a str);

impl<'a> SessionName<'a> {
    /// Create a new SessionName from a borrowed string slice
    pub fn new(s: &'a str) -> Result<Self> {
        if s.is_empty() {
            Err(scp_core::Error::InvalidSessionName(
                "session name cannot be empty".to_string(),
            ))
        } else {
            Ok(Self(s))
        }
    }

    /// Get the inner value
    pub fn as_str(&self) -> &'a str {
        self.0
    }

    /// Convert to owned String
    pub fn to_owned(&self) -> String {
        self.0.to_string()
    }
}

impl std::fmt::Display for SessionName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ========================================================================
// Data Layer: WaitResult
// ========================================================================

/// Result of a wait operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitResult {
    /// The condition was met
    ConditionMet {
        /// The session name
        session: String,
        /// The state at the time of condition met
        state: String,
    },
    /// The wait operation timed out
    Timeout {
        /// The session name
        session: String,
        /// The mode that was being waited for
        mode: String,
    },
}

impl WaitResult {
    /// Create a ConditionMet result
    pub fn condition_met(session: String, state: String) -> Self {
        Self::ConditionMet { session, state }
    }

    /// Create a Timeout result
    pub fn timeout(session: String, mode: String) -> Self {
        Self::Timeout { session, mode }
    }
}

// ========================================================================
// Data Layer: WaitMode
// ========================================================================

/// Wait modes for the wait command
#[derive(Debug, Clone, PartialEq, Eq)]
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

// ========================================================================
// Calculations Layer: Pure functions
// ========================================================================

/// Constants for poll interval validation
pub const MIN_POLL_INTERVAL_SECS: u64 = 1;
pub const MAX_POLL_INTERVAL_SECS: u64 = 60;

/// Validate poll interval, returning clamped value
fn validate_poll_interval(secs: u64) -> Duration {
    Duration::from_secs(secs.clamp(MIN_POLL_INTERVAL_SECS, MAX_POLL_INTERVAL_SECS))
}

/// Evaluate wait condition - pure function taking validated inputs
fn evaluate_condition(workspace: Option<&Workspace>, mode: &WaitMode) -> bool {
    match mode {
        WaitMode::SessionExists => workspace.is_some(),
        WaitMode::Healthy => workspace.is_some(),
        WaitMode::Status(expected) => workspace.map_or(false, |ws| ws.branch.contains(expected)),
    }
}

/// Build timeout result if elapsed
fn build_timeout_result(session: &SessionName, mode: &WaitMode) -> WaitResult {
    WaitResult::timeout(session.to_owned(), mode.display())
}

/// Check if timeout has elapsed
fn is_timeout_elapsed(start: Instant, timeout: Option<Duration>) -> bool {
    timeout.map_or(false, |t| start.elapsed() > t)
}

/// Find workspace by name - pure function
fn find_workspace<'a>(workspaces: &'a [Workspace], name: &str) -> Option<&'a Workspace> {
    workspaces.iter().find(|w| w.name == name)
}

/// Get workspace branch name or empty string
fn workspace_branch_or_empty(workspace: Option<&Workspace>) -> String {
    workspace.map_or(String::new(), |ws| ws.branch.clone())
}

// ========================================================================
// Actions Layer: I/O operations
// ========================================================================

/// Load workspaces from filesystem - I/O operation
fn load_workspaces() -> Result<Vec<Workspace>> {
    let cwd = std::env::current_dir().map_err(scp_core::Error::Io)?;
    let backend = create_backend(&cwd)?;
    backend.list_workspaces()
}

// ========================================================================
// Main Entry Point (Actions)
// ========================================================================

/// Wait for a session condition to be met
pub fn run(
    session_name: &str,
    mode_str: &str,
    timeout_secs: Option<u64>,
    poll_interval_secs: u64,
) -> Result<WaitResult> {
    // Step 1: Validate inputs
    let (session, mode) = validate_inputs(session_name, mode_str, timeout_secs)?;

    // Step 2: Create wait loop configuration
    let poll_interval = validate_poll_interval(poll_interval_secs);
    let timeout = timeout_secs.map(Duration::from_secs);

    // Step 3: Execute wait loop
    execute_wait_loop(session, mode, poll_interval, timeout)
}

/// Validate all inputs - returns SessionName and WaitMode
fn validate_inputs(
    session_name: &str,
    mode_str: &str,
    timeout_secs: Option<u64>,
) -> Result<(SessionName, WaitMode)> {
    let session = SessionName::new(session_name)?;
    let mode = WaitMode::parse(mode_str)?;

    if timeout_secs == Some(0) {
        return Err(scp_core::Error::ValidationError(
            "timeout must be > 0".to_string(),
        ));
    }

    Ok((session, mode))
}

/// Evaluate a single poll iteration - pure function
/// Returns Some(state_string) if condition is met, None otherwise
fn evaluate_poll_iteration(
    workspaces: &[Workspace],
    session: &SessionName,
    mode: &WaitMode,
) -> Option<String> {
    find_workspace(workspaces, session.as_str()).and_then(|workspace| {
        evaluate_condition(Some(workspace), mode)
            .then(|| workspace_branch_or_empty(Some(workspace)))
    })
}

/// Execute the wait loop - main orchestration
fn execute_wait_loop(
    session: SessionName,
    mode: WaitMode,
    poll_interval: Duration,
    timeout: Option<Duration>,
) -> Result<WaitResult> {
    let start = Instant::now();

    loop {
        // Check for timeout
        if is_timeout_elapsed(start, timeout) {
            return Ok(build_timeout_result(&session, &mode));
        }

        // Load workspaces (I/O)
        let workspaces = load_workspaces()?;

        // Evaluate condition (pure)
        if let Some(state) = evaluate_poll_iteration(&workspaces, &session, &mode) {
            println!("Condition met: session '{}' is {}", session, mode.display());
            return Ok(WaitResult::condition_met(session.to_owned(), state));
        }

        // Sleep for poll interval
        std::thread::sleep(poll_interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_name_valid() {
        let session = SessionName::new("test-session").unwrap();
        assert_eq!(session.as_str(), "test-session");
    }

    #[test]
    fn test_session_name_empty_fails() {
        let result = SessionName::new("");
        assert!(result.is_err());
    }

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
    fn test_evaluate_condition_session_exists() {
        // No workspace = false
        let result = evaluate_condition(None, &WaitMode::SessionExists);
        assert!(!result);

        // With workspace = true
        let ws = Workspace {
            name: "test".to_string(),
            branch: "main".to_string(),
            path: std::path::PathBuf::new(),
        };
        let result = evaluate_condition(Some(&ws), &WaitMode::SessionExists);
        assert!(result);
    }

    #[test]
    fn test_evaluate_condition_status() {
        let ws = Workspace {
            name: "test".to_string(),
            branch: "Active".to_string(),
            path: std::path::PathBuf::new(),
        };

        // Status matches
        let result = evaluate_condition(Some(&ws), &WaitMode::Status("Active".to_string()));
        assert!(result);

        // Status doesn't match
        let result = evaluate_condition(Some(&ws), &WaitMode::Status("Inactive".to_string()));
        assert!(!result);

        // No workspace = false
        let result = evaluate_condition(None, &WaitMode::Status("Active".to_string()));
        assert!(!result);
    }

    #[test]
    fn test_wait_result_display() {
        let result = WaitResult::condition_met("test".to_string(), "main".to_string());
        assert!(matches!(result, WaitResult::ConditionMet { .. }));

        let result = WaitResult::timeout("test".to_string(), "healthy".to_string());
        assert!(matches!(result, WaitResult::Timeout { .. }));
    }
}
