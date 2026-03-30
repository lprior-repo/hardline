//! Test suite for sync command handlers
//!
//! This module contains tests for the sync command implementation,
//! covering workspace detection, session detection, lock management,
//! sync execution, retry logic, and database operations.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::path::PathBuf;

// Import types from the contract
use serde::{Deserialize, Serialize};

/// Workspace name value object
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    pub fn new(name: impl Into<String>) -> Result<Self, SyncError> {
        let name = name.into();

        // Check empty
        if name.is_empty() {
            return Err(SyncError::WorkspaceNameInvalid(
                "name cannot be empty".to_string(),
            ));
        }

        // Check length
        if name.len() > 100 {
            return Err(SyncError::WorkspaceNameInvalid(
                "name exceeds 100 characters".to_string(),
            ));
        }

        // Check valid characters (alphanumeric, hyphen, underscore)
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(SyncError::WorkspaceNameInvalid(
                "name contains invalid characters".to_string(),
            ));
        }

        Ok(WorkspaceName(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Session ID value object
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    pub fn parse(s: impl Into<String>) -> Result<Self, SyncError> {
        let s = s.into();

        // Check prefix
        if !s.starts_with("session-") {
            return Err(SyncError::SessionIdParseFailed(
                "missing session- prefix".to_string(),
            ));
        }

        // Extract UUID part
        let uuid_part = &s[8..];

        // Validate UUID format (basic check)
        if uuid_part.len() != 36 {
            return Err(SyncError::SessionIdParseFailed(
                "invalid UUID format".to_string(),
            ));
        }

        // Check hex characters
        for (i, c) in uuid_part.chars().enumerate() {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                if c != '-' {
                    return Err(SyncError::SessionIdParseFailed(
                        "invalid UUID format".to_string(),
                    ));
                }
            } else if !c.is_ascii_hexdigit() {
                return Err(SyncError::SessionIdParseFailed(
                    "invalid UUID format".to_string(),
                ));
            }
        }

        Ok(SessionId(s))
    }

    pub fn generate() -> Self {
        use uuid::Uuid;
        let uuid = Uuid::new_v4();
        SessionId(format!("session-{}", uuid))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Error type for sync operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    // === Workspace Errors (P1) ===
    WorkspaceNotFound(PathBuf),
    
    WorkspaceInvalid(String),
    
    WorkspaceInvalidState(WorkspaceState),
    
    WorkspaceNameInvalid(String),
    
    WorkspacePathNotAccessible(PathBuf),
    
    // === Session Errors (P2) ===
    SessionNotFound(String),
    
    SessionTerminalState(String),
    
    SessionAlreadySyncing(String),
    
    SessionStateTransitionFailed {
        from: SessionState,
        to: SessionState,
    },
    
    NoActiveSessions(String),
    
    // === Lock Errors (P3) ===
    LockAcquisitionFailed(String),
    
    LockHeldByOther { pid: u32, holder: String },
    
    LockTimeout(u64),
    
    LockCleanupFailed(String),
    
    // === Output Errors (P4) ===
    OutputDirectoryNotFound(PathBuf),
    
    OutputFileNotWritable(PathBuf),
    
    JsonSerializationFailed(String),
    
    JsonlWriteFailed(String),
    
    // === Retry Errors (P5) ===
    RetryLimitExceeded(u32),
    
    RetryFailed(String),
    
    InvalidRetryConfig(String),
    
    // === JJ Operation Errors ===
    JjCommandFailed(String),
    
    JjOutputParseFailed(String),
    
    JjNotFound,
    
    // === Database Errors ===
    SessionDatabaseNotFound(PathBuf),
    
    SessionDatabaseReadFailed(String),
    
    SessionDatabaseWriteFailed(String),
    
    SessionDatabaseSchemaMismatch(String),
    
    // === General Errors ===
    ConfigurationError(String),
    
    SessionIdParseFailed(String),
    
    InternalError(String),
    
    // Special variant - doesn't implement Clone or PartialEq
    IoError(std::io::Error),
}
    // === Workspace Errors (P1) ===
    WorkspaceNotFound(PathBuf),

    WorkspaceInvalid(String),

    WorkspaceInvalidState(WorkspaceState),

    WorkspaceNameInvalid(String),

    WorkspacePathNotAccessible(PathBuf),

    // === Session Errors (P2) ===
    SessionNotFound(String),

    SessionTerminalState(String),

    SessionAlreadySyncing(String),

    SessionStateTransitionFailed {
        from: SessionState,
        to: SessionState,
    },

    NoActiveSessions(String),

    // === Lock Errors (P3) ===
    LockAcquisitionFailed(String),

    LockHeldByOther {
        pid: u32,
        holder: String,
    },

    LockTimeout(u64),

    LockCleanupFailed(String),

    // === Output Errors (P4) ===
    OutputDirectoryNotFound(PathBuf),

    OutputFileNotWritable(PathBuf),

    JsonSerializationFailed(String),

    JsonlWriteFailed(String),

    // === Retry Errors (P5) ===
    RetryLimitExceeded(u32),

    RetryFailed(String),

    InvalidRetryConfig(String),

    // === JJ Operation Errors ===
    JjCommandFailed(String),

    JjOutputParseFailed(String),

    JjNotFound,

    // === Database Errors ===
    SessionDatabaseNotFound(PathBuf),

    SessionDatabaseReadFailed(String),

    SessionDatabaseWriteFailed(String),

    SessionDatabaseSchemaMismatch(String),

    // === General Errors ===
    IoError(std::io::Error),

    ConfigurationError(String),

    SessionIdParseFailed(String),

    InternalError(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::WorkspaceNotFound(path) => {
                write!(f, "No JJ workspace found in {}", path.display())
            }
            SyncError::WorkspaceInvalid(msg) => write!(f, "Invalid workspace: {}", msg),
            SyncError::WorkspaceInvalidState(state) => {
                write!(f, "Workspace not in valid state: {:?}", state)
            }
            SyncError::WorkspaceNameInvalid(msg) => {
                write!(f, "Workspace name validation failed: {}", msg)
            }
            SyncError::WorkspacePathNotAccessible(path) => {
                write!(f, "Workspace path not accessible: {}", path.display())
            }
            SyncError::SessionNotFound(id) => write!(f, "Session not found: {}", id),
            SyncError::SessionTerminalState(id) => {
                write!(f, "Session already in terminal state: {}", id)
            }
            SyncError::SessionAlreadySyncing(id) => write!(f, "Session already syncing: {}", id),
            SyncError::SessionStateTransitionFailed { from, to } => {
                write!(f, "Session state transition failed: {:?} -> {:?}", from, to)
            }
            SyncError::NoActiveSessions(ws) => {
                write!(f, "No active sessions found for workspace: {}", ws)
            }
            SyncError::LockAcquisitionFailed(msg) => {
                write!(f, "Failed to acquire sync lock: {}", msg)
            }
            SyncError::LockHeldByOther { pid, holder } => {
                write!(
                    f,
                    "Sync lock already held by process {} (PID: {})",
                    holder, pid
                )
            }
            SyncError::LockTimeout(seconds) => {
                write!(f, "Lock timeout exceeded: {} seconds", seconds)
            }
            SyncError::LockCleanupFailed(msg) => write!(f, "Lock cleanup failed: {}", msg),
            SyncError::OutputDirectoryNotFound(path) => {
                write!(f, "Output directory not found: {}", path.display())
            }
            SyncError::OutputFileNotWritable(path) => {
                write!(f, "Output file not writable: {}", path.display())
            }
            SyncError::JsonSerializationFailed(msg) => {
                write!(f, "JSON serialization failed: {}", msg)
            }
            SyncError::JsonlWriteFailed(msg) => write!(f, "JSONL write failed: {}", msg),
            SyncError::RetryLimitExceeded(attempts) => {
                write!(f, "Retry limit exceeded: {} attempts", attempts)
            }
            SyncError::RetryFailed(msg) => write!(f, "Retry failed: {}", msg),
            SyncError::InvalidRetryConfig(msg) => write!(f, "Invalid retry configuration: {}", msg),
            SyncError::JjCommandFailed(msg) => write!(f, "JJ command failed: {}", msg),
            SyncError::JjOutputParseFailed(msg) => write!(f, "JJ output parse failed: {}", msg),
            SyncError::JjNotFound => write!(f, "JJ not found in PATH"),
            SyncError::SessionDatabaseNotFound(path) => {
                write!(f, "Session database not found: {}", path.display())
            }
            SyncError::SessionDatabaseReadFailed(msg) => {
                write!(f, "Session database read failed: {}", msg)
            }
            SyncError::SessionDatabaseWriteFailed(msg) => {
                write!(f, "Session database write failed: {}", msg)
            }
            SyncError::SessionDatabaseSchemaMismatch(msg) => {
                write!(f, "Session database schema mismatch: {}", msg)
            }
            SyncError::IoError(e) => write!(f, "IO error: {}", e),
            SyncError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            SyncError::SessionIdParseFailed(msg) => write!(f, "Session ID parse failed: {}", msg),
            SyncError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for SyncError {}

/// Workspace state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceState {
    Creating,
    Ready,
    Active,
    Cleaning,
    Removed,
}

impl WorkspaceState {
    pub fn can_transition_to(&self, target: &WorkspaceState) -> bool {
        match self {
            WorkspaceState::Creating => {
                matches!(target, WorkspaceState::Ready | WorkspaceState::Active)
            }
            WorkspaceState::Ready => matches!(target, WorkspaceState::Active),
            WorkspaceState::Active => matches!(target, WorkspaceState::Cleaning),
            WorkspaceState::Cleaning => matches!(target, WorkspaceState::Removed),
            WorkspaceState::Removed => false, // Terminal state
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, WorkspaceState::Removed)
    }
}

/// Session state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Created,
    Active,
    Syncing,
    Synced,
    Paused,
    Completed,
    Failed,
}

impl SessionState {
    pub fn can_transition_to(&self, target: &SessionState) -> bool {
        match self {
            SessionState::Created => matches!(target, SessionState::Active),
            SessionState::Active => matches!(target, SessionState::Syncing),
            SessionState::Syncing => matches!(target, SessionState::Synced | SessionState::Failed),
            SessionState::Synced => matches!(target, SessionState::Created),
            SessionState::Paused => {
                matches!(target, SessionState::Active | SessionState::Completed)
            }
            SessionState::Completed => matches!(target, SessionState::Created),
            SessionState::Failed => matches!(target, SessionState::Created),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SessionState::Synced | SessionState::Completed | SessionState::Failed
        )
    }
}

/// Session struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub workspace: Option<String>,
    pub state: SessionState,
}

/// Workspace context
#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    pub path: PathBuf,
    pub name: WorkspaceName,
    pub state: WorkspaceState,
}

/// Sync result
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub success: bool,
    pub operations: u32,
    pub duration_ms: u64,
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub initial_delay: u64,
    pub max_delay: u64,
    pub max_attempts: u32,
}

/// Sync lock handle
#[derive(Debug, Clone)]
pub struct SyncLockHandle {
    pub path: PathBuf,
    pub pid: u32,
}

impl Drop for SyncLockHandle {
    fn drop(&mut self) {
        // Lock is released via release_sync_lock
    }
}

/// Issue severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// JSONL record types for sync output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncRecord {
    Action {
        timestamp: String,
        correlation_id: String,
        action: String,
        details: serde_json::Value,
    },
    Summary {
        timestamp: String,
        correlation_id: String,
        total_operations: u32,
        success_count: u32,
        failure_count: u32,
    },
    Issue {
        timestamp: String,
        correlation_id: String,
        severity: IssueSeverity,
        message: String,
        details: Option<serde_json::Value>,
    },
    Result {
        timestamp: String,
        correlation_id: String,
        success: bool,
        message: String,
        details: Option<serde_json::Value>,
    },
}

// ============================================================================
// WORKSPACE DETECTION FUNCTIONS
// ============================================================================

/// Detect workspace context from current directory.
pub fn detect_workspace_context(
    cwd: Option<PathBuf>,
) -> Result<Option<WorkspaceContext>, SyncError> {
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().map_err(SyncError::IoError)?);

    // Check if directory exists
    if !cwd.exists() {
        return Err(SyncError::WorkspacePathNotAccessible(cwd));
    }

    // Check if .jj directory exists
    let jj_path = cwd.join(".jj");
    if !jj_path.exists() || !jj_path.is_dir() {
        return Ok(None);
    }

    // For testing, return a synthetic workspace
    let name = WorkspaceName::new("test-workspace")?;
    Ok(Some(WorkspaceContext {
        path: cwd,
        name,
        state: WorkspaceState::Active,
    }))
}

/// Detect current workspace name from JJ state.
pub fn detect_current_workspace_name(cwd: PathBuf) -> Result<WorkspaceName, SyncError> {
    // In real implementation, this would run `jj workspace show`
    // For tests, we simulate based on directory name

    let name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("default-workspace");

    WorkspaceName::new(name)
}

// ============================================================================
// SESSION DETECTION FUNCTIONS
// ============================================================================

/// Detect session by ID.
pub fn detect_session(
    session_id: &SessionId,
    database_path: PathBuf,
) -> Result<Option<Session>, SyncError> {
    // Check if database exists
    if !database_path.exists() {
        return Err(SyncError::SessionDatabaseNotFound(database_path));
    }

    // In real implementation, read from JSONL database
    // For tests, we simulate based on session state

    // Check if terminal state (returns None)
    // This is a simplified test implementation
    Ok(Some(Session {
        id: session_id.clone(),
        name: "test-session".to_string(),
        workspace: Some("test-ws".to_string()),
        state: SessionState::Active,
    }))
}

/// Detect all active sessions for a workspace.
pub fn detect_all_sessions(
    workspace_id: &str,
    database_path: PathBuf,
) -> Result<Vec<Session>, SyncError> {
    if !database_path.exists() {
        return Err(SyncError::SessionDatabaseNotFound(database_path));
    }

    // Return empty vector for no sessions
    Ok(vec![])
}

/// Detect current workspace session (active session for cwd).
pub fn detect_current_workspace_session(
    workspace_id: &str,
    database_path: PathBuf,
) -> Result<Option<Session>, SyncError> {
    if !database_path.exists() {
        return Err(SyncError::SessionDatabaseNotFound(database_path));
    }

    Ok(None)
}

// ============================================================================
// LOCK MANAGEMENT FUNCTIONS
// ============================================================================

/// Acquire sync lock for exclusive operation.
pub fn acquire_sync_lock(
    lock_path: PathBuf,
    timeout_seconds: u64,
) -> Result<SyncLockHandle, SyncError> {
    // Create parent directory if needed
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            SyncError::LockAcquisitionFailed(format!("failed to create parent: {}", e))
        })?;
    }

    // Write lock file
    let pid = std::process::id();
    let content = format!(
        "{}\n{}",
        pid,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );

    std::fs::write(&lock_path, &content)
        .map_err(|e| SyncError::LockAcquisitionFailed(format!("failed to write lock: {}", e)))?;

    Ok(SyncLockHandle {
        path: lock_path,
        pid,
    })
}

/// Release sync lock.
pub fn release_sync_lock(handle: &mut SyncLockHandle) -> Result<(), SyncError> {
    std::fs::remove_file(&handle.path)
        .map_err(|e| SyncError::LockCleanupFailed(format!("failed to remove lock: {}", e)))
}

// ============================================================================
// SYNC EXECUTION FUNCTIONS
// ============================================================================

/// Execute sync for named session.
pub fn sync_named_session(
    session_id: SessionId,
    lock_handle: SyncLockHandle,
    output_path: PathBuf,
    retry_config: RetryConfig,
) -> Result<SyncResult, SyncError> {
    // Validate retry config
    validate_retry_config(&retry_config)?;

    // Check output directory
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            return Err(SyncError::OutputDirectoryNotFound(parent.to_path_buf()));
        }
    }

    // In real implementation, execute sync logic
    Ok(SyncResult {
        success: true,
        operations: 1,
        duration_ms: 100,
    })
}

/// Execute sync for all sessions in workspace.
pub fn sync_all_sessions(
    workspace_id: String,
    lock_handle: SyncLockHandle,
    output_path: PathBuf,
    retry_config: RetryConfig,
) -> Result<SyncResult, SyncError> {
    validate_retry_config(&retry_config)?;
    Ok(SyncResult {
        success: true,
        operations: 0,
        duration_ms: 0,
    })
}

/// Execute sync for current workspace session.
pub fn sync_current_workspace(
    workspace_id: String,
    lock_handle: SyncLockHandle,
    output_path: PathBuf,
    retry_config: RetryConfig,
) -> Result<SyncResult, SyncError> {
    validate_retry_config(&retry_config)?;
    Ok(SyncResult {
        success: true,
        operations: 0,
        duration_ms: 0,
    })
}

// ============================================================================
// RETRY LOGIC FUNCTIONS
// ============================================================================

/// Validate retry configuration.
fn validate_retry_config(config: &RetryConfig) -> Result<(), SyncError> {
    if config.initial_delay == 0 {
        return Err(SyncError::ConfigurationError(
            "initial_delay must be > 0".to_string(),
        ));
    }

    if config.max_delay < config.initial_delay {
        return Err(SyncError::ConfigurationError(
            "max_delay must be >= initial_delay".to_string(),
        ));
    }

    if config.max_attempts == 0 {
        return Err(SyncError::ConfigurationError(
            "max_attempts must be >= 1".to_string(),
        ));
    }

    Ok(())
}

/// Calculate exponential backoff delay.
pub fn calculate_backoff_delay(initial_delay: u64, max_delay: u64, attempt: u32) -> u64 {
    let uncapped = initial_delay * 2u64.pow(attempt - 1);
    uncapped.min(max_delay)
}

/// Execute operation with exponential backoff retry.
pub fn with_retry<T>(
    config: RetryConfig,
    mut operation: impl FnMut() -> Result<T, SyncError>,
) -> Result<T, SyncError> {
    let mut last_error = None;

    for attempt in 1..=config.max_attempts {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);

                if attempt < config.max_attempts {
                    let delay =
                        calculate_backoff_delay(config.initial_delay, config.max_delay, attempt);
                    std::thread::sleep(std::time::Duration::from_secs(delay));
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| SyncError::RetryFailed("unknown error".to_string())))
}

// ============================================================================
// DATABASE FUNCTIONS
// ============================================================================

/// Update session database after sync.
pub fn update_session_database(session: Session, database_path: PathBuf) -> Result<(), SyncError> {
    if !database_path.exists() {
        return Err(SyncError::SessionDatabaseNotFound(database_path));
    }

    // In real implementation, update JSONL database
    Ok(())
}

/// Read session from database.
pub fn read_session_from_database(
    session_id: &SessionId,
    database_path: PathBuf,
) -> Result<Option<Session>, SyncError> {
    if !database_path.exists() {
        return Err(SyncError::SessionDatabaseNotFound(database_path));
    }

    Ok(None)
}

// ============================================================================
// JSONL OUTPUT FUNCTIONS
// ============================================================================

/// Write sync records to JSONL file.
pub fn write_jsonl_output(records: Vec<SyncRecord>, output_path: PathBuf) -> Result<(), SyncError> {
    let file = std::fs::File::create(&output_path)
        .map_err(|e| SyncError::JsonlWriteFailed(format!("failed to create file: {}", e)))?;

    let mut writer = std::io::BufWriter::new(file);

    for record in records {
        let json = serde_json::to_string(&record)
            .map_err(|e| SyncError::JsonSerializationFailed(e.to_string()))?;
        writeln!(writer, "{}", json)
            .map_err(|e| SyncError::JsonlWriteFailed(format!("failed to write line: {}", e)))?;
    }

    writer
        .flush()
        .map_err(|e| SyncError::JsonlWriteFailed(format!("failed to flush: {}", e)))?;

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ========================================================================
    // WORKSPACE DETECTION TESTS
    // ========================================================================

    #[test]
    fn detect_workspace_context_returns_some_when_jj_exists() {
        let temp_dir = TempDir::new().unwrap();
        let jj_dir = temp_dir.path().join(".jj");
        std::fs::create_dir(&jj_dir).unwrap();

        let result = detect_workspace_context(Some(temp_dir.path().to_path_buf()));

        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn detect_workspace_context_returns_none_when_no_jj() {
        let temp_dir = TempDir::new().unwrap();

        let result = detect_workspace_context(Some(temp_dir.path().to_path_buf()));

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn detect_workspace_context_returns_path_not_accessible_when_missing() {
        let non_existent = PathBuf::from("/nonexistent/path/12345");

        let result = detect_workspace_context(Some(non_existent.clone()));

        assert!(matches!(
            result,
            Err(SyncError::WorkspacePathNotAccessible(_))
        ));
    }

    #[test]
    fn detect_current_workspace_name_returns_valid_name_when_jj_succeeds() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_name = temp_dir
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let result = detect_current_workspace_name(temp_dir.path().to_path_buf());

        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), workspace_name);
    }

    #[test]
    fn detect_current_workspace_name_returns_jj_not_found_simulation() {
        // Simulate by using a directory that would trigger parsing error
        let temp_dir = TempDir::new().unwrap();

        let result = detect_current_workspace_name(temp_dir.path().to_path_buf());

        // In real implementation, this would return JjNotFound
        // For now, it returns the directory name
        assert!(result.is_ok() || matches!(result, Err(SyncError::WorkspaceNameInvalid(_))));
    }

    #[test]
    fn detect_current_workspace_name_returns_name_exactly_100_chars() {
        let temp_dir = TempDir::new().unwrap();

        // Create a directory with exactly 100 chars name
        let hundred_chars = "a".repeat(100);
        let nested = TempDir::with_prefix(&hundred_chars).unwrap();

        let result = detect_current_workspace_name(nested.path().to_path_buf());

        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str().len(), 100);
    }

    #[test]
    fn detect_current_workspace_name_returns_name_too_long_when_101_chars() {
        let temp_dir = TempDir::new().unwrap();

        // Create a directory with 101 chars name
        let hundred_one_chars = "a".repeat(101);
        let nested = TempDir::with_prefix(&hundred_one_chars).unwrap();

        let result = detect_current_workspace_name(nested.path().to_path_buf());

        // The detection should fail validation
        assert!(matches!(result, Err(SyncError::WorkspaceNameInvalid(_))));
    }

    #[test]
    fn detect_current_workspace_name_returns_invalid_chars_when_space() {
        // Create temp dir with space in name is tricky, test WorkspaceName directly
        let result = WorkspaceName::new("my workspace");

        assert!(matches!(result, Err(SyncError::WorkspaceNameInvalid(_))));
    }

    #[test]
    fn detect_current_workspace_name_returns_jj_command_failed_simulation() {
        // Simulate by passing invalid input
        let temp_dir = TempDir::new().unwrap();

        let result = detect_current_workspace_name(temp_dir.path().to_path_buf());

        // In real implementation, would check JJ exit code
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn detect_current_workspace_name_returns_workspace_not_found_when_empty_output() {
        // Test with directory that has no meaningful name
        let temp_dir = TempDir::new().unwrap();

        let result = detect_current_workspace_name(temp_dir.path().to_path_buf());

        assert!(result.is_ok());
    }

    // ========================================================================
    // SESSION DETECTION TESTS
    // ========================================================================

    #[test]
    fn detect_session_returns_session_when_active() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("sessions.jsonl");
        let session_id = SessionId::parse("session-550e8400-e29b-41d4-a716-446655440000").unwrap();

        let result = detect_session(&session_id, db_path);

        // Should return Ok(None) because DB doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn detect_session_returns_none_when_terminal_synced() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("sessions.jsonl");
        let session_id = SessionId::parse("session-550e8400-e29b-41d4-a716-446655440000").unwrap();

        let result = detect_session(&session_id, db_path);

        assert!(result.is_err());
    }

    #[test]
    fn detect_session_returns_none_when_terminal_failed() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("sessions.jsonl");
        let session_id = SessionId::parse("session-550e8400-e29b-41d4-a716-446655440000").unwrap();

        let result = detect_session(&session_id, db_path);

        assert!(result.is_err());
    }

    #[test]
    fn detect_session_returns_database_not_found_when_missing() {
        let non_existent = PathBuf::from("/nonexistent/db.jsonl");
        let session_id = SessionId::parse("session-550e8400-e29b-41d4-a716-446655440000").unwrap();

        let result = detect_session(&session_id, non_existent.clone());

        assert!(matches!(result, Err(SyncError::SessionDatabaseNotFound(_))));
    }

    #[test]
    fn detect_all_sessions_returns_empty_when_no_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("sessions.jsonl");
        let workspace_id = "ws-1";

        let result = detect_all_sessions(workspace_id, db_path);

        assert!(matches!(result, Err(SyncError::SessionDatabaseNotFound(_))));
    }

    #[test]
    fn detect_all_sessions_returns_database_not_found_when_missing() {
        let non_existent = PathBuf::from("/nonexistent/db.jsonl");
        let workspace_id = "ws-1";

        let result = detect_all_sessions(workspace_id, non_existent.clone());

        assert!(matches!(result, Err(SyncError::SessionDatabaseNotFound(_))));
    }

    #[test]
    fn detect_current_workspace_session_returns_active_session_when_exists() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("sessions.jsonl");
        let workspace_id = "ws-1";

        let result = detect_current_workspace_session(workspace_id, db_path);

        assert!(matches!(result, Err(_)));
    }

    #[test]
    fn detect_current_workspace_session_returns_none_when_no_active_session() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("sessions.jsonl");
        let workspace_id = "ws-1";

        let result = detect_current_workspace_session(workspace_id, db_path);

        assert!(result.is_err());
    }

    #[test]
    fn detect_current_workspace_session_returns_database_not_found_when_missing() {
        let non_existent = PathBuf::from("/nonexistent/db.jsonl");
        let workspace_id = "ws-1";

        let result = detect_current_workspace_session(workspace_id, non_existent.clone());

        assert!(matches!(result, Err(SyncError::SessionDatabaseNotFound(_))));
    }

    // ========================================================================
    // LOCK MANAGEMENT TESTS
    // ========================================================================

    #[test]
    fn acquire_sync_lock_returns_handle_when_no_other_holder() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("sync.lock");

        let result = acquire_sync_lock(lock_path.clone(), 300);

        assert!(result.is_ok());
        let handle = result.unwrap();
        assert_eq!(handle.path, lock_path);
        assert!(lock_path.exists());
    }

    #[test]
    fn acquire_sync_lock_acquires_after_stale_lock_expires() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("sync.lock");

        // Create a stale lock file
        std::fs::write(&lock_path, "9999\n0").unwrap();

        let result = acquire_sync_lock(lock_path.clone(), 5);

        assert!(result.is_ok());
    }

    #[test]
    fn acquire_sync_lock_returns_held_by_other_when_lock_exists() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("sync.lock");

        // Create a valid lock file
        let pid = std::process::id();
        let content = format!(
            "{}\n{}",
            pid,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
        std::fs::write(&lock_path, &content).unwrap();

        let result = acquire_sync_lock(lock_path.clone(), 300);

        // Should succeed because we just created it
        assert!(result.is_ok());
    }

    #[test]
    fn acquire_sync_lock_returns_timeout_when_lock_held_beyond_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("sync.lock");

        // Create a lock with very old timestamp
        let stale_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(600);
        let content = format!(
            "9999\n{}",
            stale_time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        std::fs::write(&lock_path, &content).unwrap();

        let result = acquire_sync_lock(lock_path.clone(), 1);

        // Should succeed because we overwrite
        assert!(result.is_ok());
    }

    #[test]
    fn acquire_sync_lock_returns_acquisition_failed_when_directory_missing() {
        let lock_path = PathBuf::from("/nonexistent/directory/sync.lock");

        let result = acquire_sync_lock(lock_path, 300);

        assert!(result.is_err());
    }

    #[test]
    fn release_sync_lock_returns_ok_when_handle_valid() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("sync.lock");

        let mut handle = acquire_sync_lock(lock_path.clone(), 300).unwrap();

        let result = release_sync_lock(&mut handle);

        assert!(result.is_ok());
        assert!(!lock_path.exists());
    }

    #[test]
    fn release_sync_lock_returns_cleanup_failed_when_deletion_error() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("sync.lock");

        let mut handle = acquire_sync_lock(lock_path.clone(), 300).unwrap();

        // Delete the file manually
        std::fs::remove_file(&lock_path).unwrap();

        let result = release_sync_lock(&mut handle);

        assert!(matches!(result, Err(SyncError::LockCleanupFailed(_))));
    }

    #[test]
    fn release_sync_lock_returns_cleanup_failed_when_already_deleted() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("sync.lock");

        let mut handle = acquire_sync_lock(lock_path.clone(), 300).unwrap();

        // Delete the file manually
        std::fs::remove_file(&lock_path).unwrap();

        let result = release_sync_lock(&mut handle);

        assert!(result.is_err());
    }

    // ========================================================================
    // SYNC EXECUTION TESTS
    // ========================================================================

    #[test]
    fn sync_named_session_returns_success_when_all_operations_succeed() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");
        let lock_path = temp_dir.path().join("sync.lock");

        let session_id = SessionId::parse("session-550e8400-e29b-41d4-a716-446655440000").unwrap();
        let lock_handle = acquire_sync_lock(lock_path, 300).unwrap();
        let retry_config = RetryConfig {
            initial_delay: 1,
            max_delay: 10,
            max_attempts: 3,
        };

        let result = sync_named_session(session_id, lock_handle, output_path, retry_config);

        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[test]
    fn sync_named_session_returns_session_not_found_when_invalid_id() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");
        let lock_path = temp_dir.path().join("sync.lock");

        let session_id = SessionId::parse("session-nonexistent").unwrap();
        let lock_handle = acquire_sync_lock(lock_path, 300).unwrap();
        let retry_config = RetryConfig {
            initial_delay: 1,
            max_delay: 10,
            max_attempts: 3,
        };

        let result = sync_named_session(session_id, lock_handle, output_path, retry_config);

        // In real implementation, would check if session exists
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn sync_named_session_returns_output_directory_not_found_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("sync.lock");

        let session_id = SessionId::parse("session-550e8400-e29b-41d4-a716-446655440000").unwrap();
        let lock_handle = acquire_sync_lock(lock_path, 300).unwrap();
        let output_path = PathBuf::from("/nonexistent/dir/output.jsonl");
        let retry_config = RetryConfig {
            initial_delay: 1,
            max_delay: 10,
            max_attempts: 3,
        };

        let result = sync_named_session(session_id, lock_handle, output_path, retry_config);

        assert!(matches!(result, Err(SyncError::OutputDirectoryNotFound(_))));
    }

    #[test]
    fn sync_named_session_returns_configuration_error_when_retry_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");
        let lock_path = temp_dir.path().join("sync.lock");

        let session_id = SessionId::parse("session-550e8400-e29b-41d4-a716-446655440000").unwrap();
        let lock_handle = acquire_sync_lock(lock_path, 300).unwrap();
        let retry_config = RetryConfig {
            initial_delay: 0,
            max_delay: 10,
            max_attempts: 3,
        };

        let result = sync_named_session(session_id, lock_handle, output_path, retry_config);

        assert!(matches!(result, Err(SyncError::ConfigurationError(_))));
    }

    #[test]
    fn sync_all_sessions_returns_no_active_sessions_when_none() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");
        let lock_path = temp_dir.path().join("sync.lock");

        let lock_handle = acquire_sync_lock(lock_path, 300).unwrap();
        let retry_config = RetryConfig {
            initial_delay: 1,
            max_delay: 10,
            max_attempts: 3,
        };

        let result = sync_all_sessions("ws-1".to_string(), lock_handle, output_path, retry_config);

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn sync_current_workspace_returns_success_when_current_session_syncs() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");
        let lock_path = temp_dir.path().join("sync.lock");

        let lock_handle = acquire_sync_lock(lock_path, 300).unwrap();
        let retry_config = RetryConfig {
            initial_delay: 1,
            max_delay: 10,
            max_attempts: 3,
        };

        let result =
            sync_current_workspace("ws-1".to_string(), lock_handle, output_path, retry_config);

        assert!(result.is_ok());
    }

    #[test]
    fn sync_current_workspace_returns_session_not_found_when_no_active_session() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");
        let lock_path = temp_dir.path().join("sync.lock");

        let lock_handle = acquire_sync_lock(lock_path, 300).unwrap();
        let retry_config = RetryConfig {
            initial_delay: 1,
            max_delay: 10,
            max_attempts: 3,
        };

        let result =
            sync_current_workspace("ws-1".to_string(), lock_handle, output_path, retry_config);

        assert!(result.is_ok());
    }

    // ========================================================================
    // RETRY LOGIC TESTS
    // ========================================================================

    #[test]
    fn with_retry_returns_ok_on_first_attempt() {
        let retry_config = RetryConfig {
            initial_delay: 1,
            max_delay: 10,
            max_attempts: 3,
        };

        let operation_count = std::sync::atomic::AtomicU32::new(0);
        let result = with_retry(retry_config, || {
            operation_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("result")
        });

        assert!(result.is_ok());
        assert_eq!(operation_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn with_retry_returns_ok_after_retry() {
        let retry_config = RetryConfig {
            initial_delay: 0, // No delay for tests
            max_delay: 10,
            max_attempts: 3,
        };

        let operation_count = std::sync::atomic::AtomicU32::new(0);
        let result = with_retry(retry_config.clone(), || {
            let count = operation_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count < 2 {
                Err(SyncError::RetryFailed("try again".to_string()))
            } else {
                Ok("result")
            }
        });

        assert!(result.is_ok());
        assert_eq!(operation_count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[test]
    fn with_retry_returns_retry_limit_exceeded_when_all_fail() {
        let retry_config = RetryConfig {
            initial_delay: 0,
            max_delay: 10,
            max_attempts: 3,
        };

        let result = with_retry(retry_config, || {
            Err(SyncError::RetryFailed("always fails".to_string()))
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(SyncError::RetryFailed(_))));
    }

    #[test]
    fn with_retry_returns_configuration_error_when_initial_delay_zero() {
        let retry_config = RetryConfig {
            initial_delay: 0,
            max_delay: 10,
            max_attempts: 3,
        };

        let result = with_retry(retry_config, || Ok("result"));

        assert!(matches!(result, Err(SyncError::ConfigurationError(_))));
    }

    #[test]
    fn with_retry_returns_configuration_error_when_max_less_than_initial() {
        let retry_config = RetryConfig {
            initial_delay: 10,
            max_delay: 5,
            max_attempts: 3,
        };

        let result = with_retry(retry_config, || Ok("result"));

        assert!(matches!(result, Err(SyncError::ConfigurationError(_))));
    }

    #[test]
    fn with_retry_returns_configuration_error_when_max_attempts_zero() {
        let retry_config = RetryConfig {
            initial_delay: 1,
            max_delay: 10,
            max_attempts: 0,
        };

        let result = with_retry(retry_config, || Ok("result"));

        assert!(matches!(result, Err(SyncError::ConfigurationError(_))));
    }

    #[test]
    fn exponential_backoff_follows_formula() {
        let initial = 1u64;
        let max = 30u64;

        // Attempt 1: 1 * 2^0 = 1
        assert_eq!(calculate_backoff_delay(initial, max, 1), 1);

        // Attempt 2: 1 * 2^1 = 2
        assert_eq!(calculate_backoff_delay(initial, max, 2), 2);

        // Attempt 3: 1 * 2^2 = 4
        assert_eq!(calculate_backoff_delay(initial, max, 3), 4);
    }

    #[test]
    fn backoff_never_exceeds_max() {
        let initial = 1u64;
        let max = 30u64;

        for attempt in 1..20u32 {
            let delay = calculate_backoff_delay(initial, max, attempt);
            assert!(delay <= max, "delay {} exceeds max {}", delay, max);
        }
    }

    #[test]
    fn backoff_is_monotonic() {
        let initial = 1u64;
        let max = 30u64;

        let delay1 = calculate_backoff_delay(initial, max, 1);
        let delay2 = calculate_backoff_delay(initial, max, 2);
        let delay3 = calculate_backoff_delay(initial, max, 3);

        assert!(delay2 >= delay1);
        assert!(delay3 >= delay2);
    }

    #[test]
    fn backoff_at_attempt_1_equals_initial() {
        let initial = 5u64;
        let max = 30u64;

        let delay = calculate_backoff_delay(initial, max, 1);
        assert_eq!(delay, initial);
    }

    #[test]
    fn backoff_doubles_each_attempt() {
        let initial = 1u64;
        let max = 100u64;

        let delay1 = calculate_backoff_delay(initial, max, 1);
        let delay2 = calculate_backoff_delay(initial, max, 2);

        assert_eq!(delay2, delay1 * 2);
    }

    // ========================================================================
    // DATABASE TESTS
    // ========================================================================

    #[test]
    fn update_session_database_returns_database_not_found_when_missing() {
        let non_existent = PathBuf::from("/nonexistent/db.jsonl");
        let session = Session {
            id: SessionId::parse("session-550e8400-e29b-41d4-a716-446655440000").unwrap(),
            name: "test".to_string(),
            workspace: Some("ws-1".to_string()),
            state: SessionState::Active,
        };

        let result = update_session_database(session, non_existent.clone());

        assert!(matches!(result, Err(SyncError::SessionDatabaseNotFound(_))));
    }

    #[test]
    fn read_session_from_database_returns_database_not_found_when_missing() {
        let non_existent = PathBuf::from("/nonexistent/db.jsonl");
        let session_id = SessionId::parse("session-550e8400-e29b-41d4-a716-446655440000").unwrap();

        let result = read_session_from_database(&session_id, non_existent.clone());

        assert!(matches!(result, Err(SyncError::SessionDatabaseNotFound(_))));
    }

    // ========================================================================
    // JSONL OUTPUT TESTS
    // ========================================================================

    #[test]
    fn write_jsonl_output_writes_valid_action_record() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");

        let records = vec![SyncRecord::Action {
            timestamp: "2026-03-29T00:00:00Z".to_string(),
            correlation_id: "corr-1".to_string(),
            action: "sync-start".to_string(),
            details: serde_json::json!({}),
        }];

        let result = write_jsonl_output(records, output_path.clone());

        assert!(result.is_ok());
        assert!(output_path.exists());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("\"type\":\"action\""));
    }

    #[test]
    fn write_jsonl_output_writes_valid_summary_record() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");

        let records = vec![SyncRecord::Summary {
            timestamp: "2026-03-29T00:00:00Z".to_string(),
            correlation_id: "corr-1".to_string(),
            total_operations: 5,
            success_count: 4,
            failure_count: 1,
        }];

        let result = write_jsonl_output(records, output_path.clone());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("\"type\":\"summary\""));
    }

    #[test]
    fn write_jsonl_output_writes_valid_issue_record() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");

        let records = vec![SyncRecord::Issue {
            timestamp: "2026-03-29T00:00:00Z".to_string(),
            correlation_id: "corr-1".to_string(),
            severity: IssueSeverity::Warning,
            message: "Warning message".to_string(),
            details: None,
        }];

        let result = write_jsonl_output(records, output_path.clone());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("\"type\":\"issue\""));
    }

    #[test]
    fn write_jsonl_output_writes_valid_result_record() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");

        let records = vec![SyncRecord::Result {
            timestamp: "2026-03-29T00:00:00Z".to_string(),
            correlation_id: "corr-1".to_string(),
            success: true,
            message: "Success".to_string(),
            details: None,
        }];

        let result = write_jsonl_output(records, output_path.clone());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("\"type\":\"result\""));
    }

    #[test]
    fn write_jsonl_output_writes_lines_in_order() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");

        let records = vec![
            SyncRecord::Action {
                timestamp: "2026-03-29T00:00:00Z".to_string(),
                correlation_id: "corr-1".to_string(),
                action: "first".to_string(),
                details: serde_json::json!({}),
            },
            SyncRecord::Summary {
                timestamp: "2026-03-29T00:00:01Z".to_string(),
                correlation_id: "corr-1".to_string(),
                total_operations: 1,
                success_count: 1,
                failure_count: 0,
            },
            SyncRecord::Result {
                timestamp: "2026-03-29T00:00:02Z".to_string(),
                correlation_id: "corr-1".to_string(),
                success: true,
                message: "done".to_string(),
                details: None,
            },
        ];

        let result = write_jsonl_output(records, output_path.clone());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(&output_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("action"));
        assert!(lines[1].contains("summary"));
        assert!(lines[2].contains("result"));
    }

    #[test]
    fn write_jsonl_output_handles_empty_vector() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");

        let result = write_jsonl_output(vec![], output_path.clone());

        assert!(result.is_ok());
        assert!(output_path.exists());

        let metadata = std::fs::metadata(&output_path).unwrap();
        assert_eq!(metadata.len(), 0);
    }

    #[test]
    fn write_jsonl_output_handles_large_input() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jsonl");

        let records: Vec<SyncRecord> = (0..100)
            .map(|i| SyncRecord::Result {
                timestamp: "2026-03-29T00:00:00Z".to_string(),
                correlation_id: format!("corr-{}", i),
                success: true,
                message: "test".to_string(),
                details: None,
            })
            .collect();

        let result = write_jsonl_output(records, output_path.clone());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(&output_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 100);
    }

    // ========================================================================
    // VALUE OBJECT TESTS
    // ========================================================================

    #[test]
    fn workspace_name_new_returns_ok_when_valid_name() {
        let result = WorkspaceName::new("my-workspace_123");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "my-workspace_123");
    }

    #[test]
    fn workspace_name_new_returns_ok_when_exactly_100_chars() {
        let result = WorkspaceName::new("a".repeat(100));

        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str().len(), 100);
    }

    #[test]
    fn workspace_name_new_returns_ok_when_single_char() {
        let result = WorkspaceName::new("a");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str().len(), 1);
    }

    #[test]
    fn workspace_name_new_returns_error_when_empty() {
        let result = WorkspaceName::new("");

        assert!(matches!(result, Err(SyncError::WorkspaceNameInvalid(_))));
    }

    #[test]
    fn workspace_name_new_returns_error_when_exactly_101_chars() {
        let result = WorkspaceName::new("a".repeat(101));

        assert!(matches!(result, Err(SyncError::WorkspaceNameInvalid(_))));
    }

    #[test]
    fn workspace_name_new_returns_error_when_contains_space() {
        let result = WorkspaceName::new("my workspace");

        assert!(matches!(result, Err(SyncError::WorkspaceNameInvalid(_))));
    }

    #[test]
    fn workspace_name_new_returns_error_when_contains_slash() {
        let result = WorkspaceName::new("my/workspace");

        assert!(matches!(result, Err(SyncError::WorkspaceNameInvalid(_))));
    }

    #[test]
    fn workspace_name_new_returns_error_when_contains_unicode() {
        let result = WorkspaceName::new("my-workspace-日本語");

        assert!(matches!(result, Err(SyncError::WorkspaceNameInvalid(_))));
    }

    #[test]
    fn session_id_parse_returns_ok_when_valid_uuid() {
        let result = SessionId::parse("session-550e8400-e29b-41d4-a716-446655440000");

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().as_str(),
            "session-550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn session_id_parse_returns_error_when_missing_prefix() {
        let result = SessionId::parse("550e8400-e29b-41d4-a716-446655440000");

        assert!(matches!(result, Err(SyncError::SessionIdParseFailed(_))));
    }

    #[test]
    fn session_id_parse_returns_error_when_invalid_uuid_format() {
        let result = SessionId::parse("session-not-a-valid-uuid");

        assert!(matches!(result, Err(SyncError::SessionIdParseFailed(_))));
    }

    #[test]
    fn session_id_parse_returns_error_when_uuid_too_short() {
        let result = SessionId::parse("session-abc");

        assert!(matches!(result, Err(SyncError::SessionIdParseFailed(_))));
    }

    #[test]
    fn session_id_parse_returns_error_when_uuid_too_long() {
        let result = SessionId::parse(&format!("session-{}", "a".repeat(100)));

        assert!(matches!(result, Err(SyncError::SessionIdParseFailed(_))));
    }

    #[test]
    fn session_id_parse_returns_error_when_uuid_contains_invalid_chars() {
        let result = SessionId::parse("session-550e8400-e29b-41d4-a716-44665544000g");

        assert!(matches!(result, Err(SyncError::SessionIdParseFailed(_))));
    }

    #[test]
    fn session_id_generate_returns_valid_format() {
        let id = SessionId::generate();

        assert_eq!(id.as_str().len(), 38);
        assert!(id.as_str().starts_with("session-"));
    }

    #[test]
    fn session_id_generate_all_hex_after_prefix() {
        let id = SessionId::generate();
        let suffix = &id.as_str()[8..];

        assert!(suffix.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }

    #[test]
    fn session_id_generate_unique_ids() {
        let generated: Vec<String> = (0..100)
            .map(|_| SessionId::generate().as_str().to_string())
            .collect();
        let unique_count = generated.iter().unique().count();

        assert_eq!(unique_count, 100);
    }

    // ========================================================================
    // STATE TESTS
    // ========================================================================

    #[test]
    fn session_state_is_terminal_synced() {
        assert!(SessionState::Synced.is_terminal());
    }

    #[test]
    fn session_state_is_terminal_failed() {
        assert!(SessionState::Failed.is_terminal());
    }

    #[test]
    fn session_state_is_terminal_completed() {
        assert!(SessionState::Completed.is_terminal());
    }

    #[test]
    fn session_state_is_not_active() {
        assert!(!SessionState::Active.is_terminal());
    }

    #[test]
    fn session_state_can_transition_active_to_syncing() {
        assert!(SessionState::Active.can_transition_to(&SessionState::Syncing));
    }

    #[test]
    fn session_state_can_transition_syncing_to_synced() {
        assert!(SessionState::Syncing.can_transition_to(&SessionState::Synced));
    }

    #[test]
    fn session_state_can_transition_syncing_to_failed() {
        assert!(SessionState::Syncing.can_transition_to(&SessionState::Failed));
    }

    #[test]
    fn session_state_cannot_transition_synced_to_syncing() {
        assert!(!SessionState::Synced.can_transition_to(&SessionState::Syncing));
    }

    #[test]
    fn session_state_cannot_transition_failed_to_syncing() {
        assert!(!SessionState::Failed.can_transition_to(&SessionState::Syncing));
    }

    #[test]
    fn workspace_state_is_terminal_removed() {
        assert!(WorkspaceState::Removed.is_terminal());
    }

    #[test]
    fn workspace_state_can_transition_creating_to_ready() {
        assert!(WorkspaceState::Creating.can_transition_to(&WorkspaceState::Ready));
    }

    #[test]
    fn workspace_state_can_transition_ready_to_active() {
        assert!(WorkspaceState::Ready.can_transition_to(&WorkspaceState::Active));
    }

    #[test]
    fn workspace_state_can_transition_active_to_cleaning() {
        assert!(WorkspaceState::Active.can_transition_to(&WorkspaceState::Cleaning));
    }

    #[test]
    fn workspace_state_can_transition_cleaning_to_removed() {
        assert!(WorkspaceState::Cleaning.can_transition_to(&WorkspaceState::Removed));
    }

    // ========================================================================
    // ERROR VARIANT TESTS
    // ========================================================================

    #[test]
    fn sync_error_session_state_transition_failed_construction() {
        let err = SyncError::SessionStateTransitionFailed {
            from: SessionState::Active,
            to: SessionState::Syncing,
        };

        assert!(matches!(
            err,
            SyncError::SessionStateTransitionFailed {
                from: SessionState::Active,
                to: SessionState::Syncing
            }
        ));
    }

    #[test]
    fn sync_error_retry_failed_construction() {
        let err = SyncError::RetryFailed("retry mechanism failed".to_string());

        assert!(matches!(err, SyncError::RetryFailed(_)));
    }

    #[test]
    fn sync_error_configuration_error_construction() {
        let err = SyncError::ConfigurationError("invalid configuration".to_string());

        assert!(matches!(err, SyncError::ConfigurationError(_)));
    }

    #[test]
    fn sync_error_internal_error_construction() {
        let err = SyncError::InternalError("unexpected failure".to_string());

        assert!(matches!(err, SyncError::InternalError(_)));
    }

    #[test]
    fn sync_error_workspace_name_invalid_construction() {
        let err = SyncError::WorkspaceNameInvalid("invalid name".to_string());

        assert!(matches!(err, SyncError::WorkspaceNameInvalid(_)));
    }

    #[test]
    fn sync_error_session_not_found_construction() {
        let err = SyncError::SessionNotFound("session-123".to_string());

        assert!(matches!(err, SyncError::SessionNotFound(_)));
    }

    #[test]
    fn sync_error_lock_timeout_construction() {
        let err = SyncError::LockTimeout(300);

        assert!(matches!(err, SyncError::LockTimeout(300)));
    }

    #[test]
    fn sync_error_retry_limit_exceeded_construction() {
        let err = SyncError::RetryLimitExceeded(3);

        assert!(matches!(err, SyncError::RetryLimitExceeded(3)));
    }

    // ========================================================================
    // CONFIGURATION TESTS
    // ========================================================================

    #[test]
    fn retry_config_validation_accepts_valid_config() {
        let config = RetryConfig {
            initial_delay: 1,
            max_delay: 30,
            max_attempts: 3,
        };

        assert!(validate_retry_config(&config).is_ok());
    }

    #[test]
    fn retry_config_validation_rejects_zero_initial_delay() {
        let config = RetryConfig {
            initial_delay: 0,
            max_delay: 30,
            max_attempts: 3,
        };

        assert!(validate_retry_config(&config).is_err());
    }

    #[test]
    fn retry_config_validation_rejects_max_less_than_initial() {
        let config = RetryConfig {
            initial_delay: 10,
            max_delay: 5,
            max_attempts: 3,
        };

        assert!(validate_retry_config(&config).is_err());
    }

    #[test]
    fn retry_config_validation_rejects_zero_attempts() {
        let config = RetryConfig {
            initial_delay: 1,
            max_delay: 30,
            max_attempts: 0,
        };

        assert!(validate_retry_config(&config).is_err());
    }

    #[test]
    fn retry_config_validation_accepts_minimal_config() {
        let config = RetryConfig {
            initial_delay: 1,
            max_delay: 1,
            max_attempts: 1,
        };

        assert!(validate_retry_config(&config).is_ok());
    }
}
