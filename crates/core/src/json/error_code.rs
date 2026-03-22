//! Error codes for machine-readable errors

use serde::{Deserialize, Serialize};

/// Error codes for machine-readable errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    // Session errors
    SessionNotFound,
    SessionAlreadyExists,
    SessionNameInvalid,

    // Workspace errors
    WorkspaceCreationFailed,
    WorkspaceNotFound,

    // JJ errors
    JjNotInstalled,
    JjCommandFailed,
    NotJjRepository,

    // Zellij errors
    ZellijNotRunning,
    ZellijCommandFailed,

    // Config errors
    ConfigNotFound,
    ConfigParseError,
    ConfigKeyNotFound,

    // Hook errors
    HookFailed,
    HookExecutionError,

    // State errors
    StateDbCorrupted,
    StateDbLocked,

    // Undo errors
    ReadUndoLogFailed,
    WriteUndoLogFailed,

    // Spawn errors
    SpawnNotOnMain,
    SpawnInvalidBeadStatus,
    SpawnBeadNotFound,
    SpawnWorkspaceCreationFailed,
    SpawnAgentSpawnFailed,
    SpawnTimeout,
    SpawnMergeFailed,
    SpawnCleanupFailed,
    SpawnDatabaseError,
    SpawnJjCommandFailed,

    // Generic errors
    InvalidArgument,
    Unknown,
}

impl ErrorCode {
    /// Get the string representation of the error code
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionNotFound => "SESSION_NOT_FOUND",
            Self::SessionAlreadyExists => "SESSION_ALREADY_EXISTS",
            Self::SessionNameInvalid => "SESSION_NAME_INVALID",
            Self::WorkspaceCreationFailed => "WORKSPACE_CREATION_FAILED",
            Self::WorkspaceNotFound => "WORKSPACE_NOT_FOUND",
            Self::JjNotInstalled => "JJ_NOT_INSTALLED",
            Self::JjCommandFailed => "JJ_COMMAND_FAILED",
            Self::NotJjRepository => "NOT_JJ_REPOSITORY",
            Self::ZellijNotRunning => "ZELLIJ_NOT_RUNNING",
            Self::ZellijCommandFailed => "ZELLIJ_COMMAND_FAILED",
            Self::ConfigNotFound => "CONFIG_NOT_FOUND",
            Self::ConfigParseError => "CONFIG_PARSE_ERROR",
            Self::ConfigKeyNotFound => "CONFIG_KEY_NOT_FOUND",
            Self::HookFailed => "HOOK_FAILED",
            Self::HookExecutionError => "HOOK_EXECUTION_ERROR",
            Self::StateDbCorrupted => "STATE_DB_CORRUPTED",
            Self::StateDbLocked => "STATE_DB_LOCKED",
            Self::ReadUndoLogFailed => "READ_UNDO_LOG_FAILED",
            Self::WriteUndoLogFailed => "WRITE_UNDO_LOG_FAILED",
            Self::SpawnNotOnMain => "SPAWN_NOT_ON_MAIN",
            Self::SpawnInvalidBeadStatus => "SPAWN_INVALID_BEAD_STATUS",
            Self::SpawnBeadNotFound => "SPAWN_BEAD_NOT_FOUND",
            Self::SpawnWorkspaceCreationFailed => "SPAWN_WORKSPACE_CREATION_FAILED",
            Self::SpawnAgentSpawnFailed => "SPAWN_AGENT_SPAWN_FAILED",
            Self::SpawnTimeout => "SPAWN_TIMEOUT",
            Self::SpawnMergeFailed => "SPAWN_MERGE_FAILED",
            Self::SpawnCleanupFailed => "SPAWN_CLEANUP_FAILED",
            Self::SpawnDatabaseError => "SPAWN_DATABASE_ERROR",
            Self::SpawnJjCommandFailed => "SPAWN_JJ_COMMAND_FAILED",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl From<ErrorCode> for String {
    fn from(code: ErrorCode) -> Self {
        code.as_str().to_string()
    }
}
