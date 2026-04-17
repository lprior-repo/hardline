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

    // VCS errors
    VcsNotInstalled,
    VcsCommandFailed,
    NotGitRepository,

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
    SpawnVcsCommandFailed,

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
            Self::VcsNotInstalled => "VCS_NOT_INSTALLED",
            Self::VcsCommandFailed => "VCS_COMMAND_FAILED",
            Self::NotGitRepository => "NOT_GIT_REPOSITORY",
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
            Self::SpawnVcsCommandFailed => "SPAWN_VCS_COMMAND_FAILED",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl ErrorCode {
    /// Suggest a resolution for this error code.
    ///
    /// Returns `Some` with a human-readable suggestion string when a useful
    /// resolution hint exists, or `None` when no generic suggestion applies.
    #[must_use]
    pub const fn suggest_resolution(self) -> Option<&'static str> {
        match self {
            // Session errors
            Self::SessionNotFound => {
                Some("Use 'scp session list' to see available sessions")
            }
            Self::SessionAlreadyExists => {
                Some("Use 'scp session status <name>' to switch to an existing session, or choose a different name")
            }
            Self::SessionNameInvalid => {
                Some("Session names must be 1-64 chars, start with a letter, and contain only alphanumeric, dash, underscore")
            }

            // Workspace errors
            Self::WorkspaceCreationFailed => {
                Some("Check Git is working: git status, or try: scp doctor")
            }
            Self::WorkspaceNotFound => {
                Some("Use 'scp workspace list' to see available workspaces, or 'scp doctor' to check system health")
            }

            // VCS errors
            Self::VcsNotInstalled => {
                Some("Install Git: https://git-scm.com/downloads or use your package manager")
            }
            Self::VcsCommandFailed => {
                Some("Check Git status: git status, resolve conflicts, or see: scp doctor")
            }
            Self::NotGitRepository => {
                Some("Run 'scp init' to initialize a Git repository in this directory")
            }

            // Zellij errors
            Self::ZellijNotRunning => {
                Some("Start a Zellij session: zellij attach <session> or zellij new <session>")
            }
            Self::ZellijCommandFailed => {
                Some("Ensure Zellij is installed and accessible: zellij --version")
            }

            // Config errors
            Self::ConfigNotFound => {
                Some("Run 'scp init' to create default configuration")
            }
            Self::ConfigParseError => {
                Some("Check your configuration file for syntax errors, or run 'scp config show' to inspect values")
            }
            Self::ConfigKeyNotFound => {
                Some("Check available keys with 'scp config show', or reset with 'scp config reset <key>'")
            }

            // Hook errors
            Self::HookFailed => {
                Some("Check hook scripts in .scp/hooks/, or use --no-hooks to skip")
            }
            Self::HookExecutionError => {
                Some("Verify hook script permissions (chmod +x) and shebang lines, or use --no-hooks to skip")
            }

            // State errors
            Self::StateDbCorrupted => {
                Some("Try running 'scp doctor --fix' to repair the database, or delete .scp/state.db to reset")
            }
            Self::StateDbLocked => {
                Some("Another process holds the database lock. Wait for it to finish, or check for stuck processes")
            }

            // Undo errors
            Self::ReadUndoLogFailed => {
                Some("The undo log may be corrupted. Try 'scp doctor --fix' to repair")
            }
            Self::WriteUndoLogFailed => {
                Some("Check disk space and write permissions, or try 'scp doctor --fix'")
            }

            // Spawn errors
            Self::SpawnNotOnMain => {
                Some("Switch to main branch: git checkout main")
            }
            Self::SpawnInvalidBeadStatus => {
                Some("Check bead status with: bd show <bead-id>")
            }
            Self::SpawnBeadNotFound => {
                Some("List available beads with: bd ready")
            }
            Self::SpawnWorkspaceCreationFailed => {
                Some("Check disk space and permissions, or run: scp doctor")
            }
            Self::SpawnAgentSpawnFailed => {
                Some("Check agent command is valid, or use --agent-command flag")
            }
            Self::SpawnTimeout => {
                Some("Increase timeout with --timeout flag, or check for infinite loops")
            }
            Self::SpawnMergeFailed => {
                Some("Resolve conflicts manually in workspace, or use: git merge --abort")
            }
            Self::SpawnCleanupFailed => {
                Some("Manually clean workspace: rm -rf .scp/workspaces/<bead-id>")
            }
            Self::SpawnDatabaseError => {
                Some("Run: bd sync or scp doctor --fix")
            }
            Self::SpawnVcsCommandFailed => {
                Some("Check Git is working: git status, or run: scp doctor")
            }

            // Generic errors
            Self::InvalidArgument => {
                Some("Use 'scp context' to see current state, or check command help: scp <command> --help")
            }
            Self::Unknown => {
                Some("Run 'scp doctor' to check system health and configuration")
            }
        }
    }
}

impl From<ErrorCode> for String {
    fn from(code: ErrorCode) -> Self {
        code.as_str().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- ErrorCode enum variants have correct string representations ---

    #[test]
    fn test_error_code_as_str_session_errors() {
        assert_eq!(ErrorCode::SessionNotFound.as_str(), "SESSION_NOT_FOUND");
        assert_eq!(ErrorCode::SessionAlreadyExists.as_str(), "SESSION_ALREADY_EXISTS");
        assert_eq!(ErrorCode::SessionNameInvalid.as_str(), "SESSION_NAME_INVALID");
    }

    #[test]
    fn test_error_code_as_str_workspace_errors() {
        assert_eq!(ErrorCode::WorkspaceCreationFailed.as_str(), "WORKSPACE_CREATION_FAILED");
        assert_eq!(ErrorCode::WorkspaceNotFound.as_str(), "WORKSPACE_NOT_FOUND");
    }

    #[test]
    fn test_error_code_as_str_vcs_errors() {
        assert_eq!(ErrorCode::VcsNotInstalled.as_str(), "VCS_NOT_INSTALLED");
        assert_eq!(ErrorCode::VcsCommandFailed.as_str(), "VCS_COMMAND_FAILED");
        assert_eq!(ErrorCode::NotGitRepository.as_str(), "NOT_GIT_REPOSITORY");
    }

    #[test]
    fn test_error_code_as_str_zellij_errors() {
        assert_eq!(ErrorCode::ZellijNotRunning.as_str(), "ZELLIJ_NOT_RUNNING");
        assert_eq!(ErrorCode::ZellijCommandFailed.as_str(), "ZELLIJ_COMMAND_FAILED");
    }

    #[test]
    fn test_error_code_as_str_config_errors() {
        assert_eq!(ErrorCode::ConfigNotFound.as_str(), "CONFIG_NOT_FOUND");
        assert_eq!(ErrorCode::ConfigParseError.as_str(), "CONFIG_PARSE_ERROR");
        assert_eq!(ErrorCode::ConfigKeyNotFound.as_str(), "CONFIG_KEY_NOT_FOUND");
    }

    #[test]
    fn test_error_code_as_str_hook_errors() {
        assert_eq!(ErrorCode::HookFailed.as_str(), "HOOK_FAILED");
        assert_eq!(ErrorCode::HookExecutionError.as_str(), "HOOK_EXECUTION_ERROR");
    }

    #[test]
    fn test_error_code_as_str_state_errors() {
        assert_eq!(ErrorCode::StateDbCorrupted.as_str(), "STATE_DB_CORRUPTED");
        assert_eq!(ErrorCode::StateDbLocked.as_str(), "STATE_DB_LOCKED");
    }

    #[test]
    fn test_error_code_as_str_undo_errors() {
        assert_eq!(ErrorCode::ReadUndoLogFailed.as_str(), "READ_UNDO_LOG_FAILED");
        assert_eq!(ErrorCode::WriteUndoLogFailed.as_str(), "WRITE_UNDO_LOG_FAILED");
    }

    #[test]
    fn test_error_code_as_str_spawn_errors() {
        assert_eq!(ErrorCode::SpawnNotOnMain.as_str(), "SPAWN_NOT_ON_MAIN");
        assert_eq!(ErrorCode::SpawnInvalidBeadStatus.as_str(), "SPAWN_INVALID_BEAD_STATUS");
        assert_eq!(ErrorCode::SpawnBeadNotFound.as_str(), "SPAWN_BEAD_NOT_FOUND");
        assert_eq!(ErrorCode::SpawnWorkspaceCreationFailed.as_str(), "SPAWN_WORKSPACE_CREATION_FAILED");
        assert_eq!(ErrorCode::SpawnAgentSpawnFailed.as_str(), "SPAWN_AGENT_SPAWN_FAILED");
        assert_eq!(ErrorCode::SpawnTimeout.as_str(), "SPAWN_TIMEOUT");
        assert_eq!(ErrorCode::SpawnMergeFailed.as_str(), "SPAWN_MERGE_FAILED");
        assert_eq!(ErrorCode::SpawnCleanupFailed.as_str(), "SPAWN_CLEANUP_FAILED");
        assert_eq!(ErrorCode::SpawnDatabaseError.as_str(), "SPAWN_DATABASE_ERROR");
        assert_eq!(ErrorCode::SpawnVcsCommandFailed.as_str(), "SPAWN_VCS_COMMAND_FAILED");
    }

    #[test]
    fn test_error_code_as_str_generic_errors() {
        assert_eq!(ErrorCode::InvalidArgument.as_str(), "INVALID_ARGUMENT");
        assert_eq!(ErrorCode::Unknown.as_str(), "UNKNOWN");
    }

    // -- All as_str() values are SCREAMING_SNAKE_CASE ---

    #[test]
    fn test_all_error_codes_are_screaming_snake_case() {
        let all_codes = [
            ErrorCode::SessionNotFound,
            ErrorCode::SessionAlreadyExists,
            ErrorCode::SessionNameInvalid,
            ErrorCode::WorkspaceCreationFailed,
            ErrorCode::WorkspaceNotFound,
            ErrorCode::VcsNotInstalled,
            ErrorCode::VcsCommandFailed,
            ErrorCode::NotGitRepository,
            ErrorCode::ZellijNotRunning,
            ErrorCode::ZellijCommandFailed,
            ErrorCode::ConfigNotFound,
            ErrorCode::ConfigParseError,
            ErrorCode::ConfigKeyNotFound,
            ErrorCode::HookFailed,
            ErrorCode::HookExecutionError,
            ErrorCode::StateDbCorrupted,
            ErrorCode::StateDbLocked,
            ErrorCode::ReadUndoLogFailed,
            ErrorCode::WriteUndoLogFailed,
            ErrorCode::SpawnNotOnMain,
            ErrorCode::SpawnInvalidBeadStatus,
            ErrorCode::SpawnBeadNotFound,
            ErrorCode::SpawnWorkspaceCreationFailed,
            ErrorCode::SpawnAgentSpawnFailed,
            ErrorCode::SpawnTimeout,
            ErrorCode::SpawnMergeFailed,
            ErrorCode::SpawnCleanupFailed,
            ErrorCode::SpawnDatabaseError,
            ErrorCode::SpawnVcsCommandFailed,
            ErrorCode::InvalidArgument,
            ErrorCode::Unknown,
        ];

        for code in all_codes {
            let s = code.as_str();
            assert!(!s.is_empty(), "ErrorCode {code:?} as_str should not be empty");
            assert!(
                s.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "ErrorCode {code:?} as_str '{s}' should be SCREAMING_SNAKE_CASE"
            );
        }
    }

    // -- suggest_resolution() returns sensible suggestions for each code

    #[test]
    fn test_suggest_resolution_session_errors() {
        let s = ErrorCode::SessionNotFound.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("scp session list"));

        let s = ErrorCode::SessionAlreadyExists.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("status"));

        let s = ErrorCode::SessionNameInvalid.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("alphanumeric"));
    }

    #[test]
    fn test_suggest_resolution_workspace_errors() {
        let s = ErrorCode::WorkspaceCreationFailed.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("git status") || s.unwrap().contains("doctor"));

        let s = ErrorCode::WorkspaceNotFound.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("workspace list"));
    }

    #[test]
    fn test_suggest_resolution_vcs_errors() {
        let s = ErrorCode::VcsNotInstalled.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("Install Git"));

        let s = ErrorCode::VcsCommandFailed.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("git status") || s.unwrap().contains("doctor"));

        let s = ErrorCode::NotGitRepository.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("scp init"));
    }

    #[test]
    fn test_suggest_resolution_zellij_errors() {
        let s = ErrorCode::ZellijNotRunning.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("zellij"));

        let s = ErrorCode::ZellijCommandFailed.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("zellij") || s.unwrap().contains("version"));
    }

    #[test]
    fn test_suggest_resolution_config_errors() {
        let s = ErrorCode::ConfigNotFound.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("scp init"));

        let s = ErrorCode::ConfigParseError.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("syntax") || s.unwrap().contains("config show"));

        let s = ErrorCode::ConfigKeyNotFound.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("config show") || s.unwrap().contains("config reset"));
    }

    #[test]
    fn test_suggest_resolution_hook_errors() {
        let s = ErrorCode::HookFailed.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("hooks") || s.unwrap().contains("--no-hooks"));

        let s = ErrorCode::HookExecutionError.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("permissions") || s.unwrap().contains("chmod"));
    }

    #[test]
    fn test_suggest_resolution_state_errors() {
        let s = ErrorCode::StateDbCorrupted.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("doctor") || s.unwrap().contains("fix"));

        let s = ErrorCode::StateDbLocked.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("lock") || s.unwrap().contains("process"));
    }

    #[test]
    fn test_suggest_resolution_undo_errors() {
        let s = ErrorCode::ReadUndoLogFailed.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("doctor") || s.unwrap().contains("fix"));

        let s = ErrorCode::WriteUndoLogFailed.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("disk") || s.unwrap().contains("permissions"));
    }

    #[test]
    fn test_suggest_resolution_spawn_errors() {
        assert!(ErrorCode::SpawnNotOnMain.suggest_resolution().is_some());
        assert!(ErrorCode::SpawnInvalidBeadStatus.suggest_resolution().is_some());
        assert!(ErrorCode::SpawnBeadNotFound.suggest_resolution().is_some());
        assert!(ErrorCode::SpawnWorkspaceCreationFailed.suggest_resolution().is_some());
        assert!(ErrorCode::SpawnAgentSpawnFailed.suggest_resolution().is_some());
        assert!(ErrorCode::SpawnTimeout.suggest_resolution().is_some());
        assert!(ErrorCode::SpawnMergeFailed.suggest_resolution().is_some());
        assert!(ErrorCode::SpawnCleanupFailed.suggest_resolution().is_some());
        assert!(ErrorCode::SpawnDatabaseError.suggest_resolution().is_some());
        assert!(ErrorCode::SpawnVcsCommandFailed.suggest_resolution().is_some());
    }

    #[test]
    fn test_suggest_resolution_generic_errors() {
        let s = ErrorCode::InvalidArgument.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("--help") || s.unwrap().contains("context"));

        let s = ErrorCode::Unknown.suggest_resolution();
        assert!(s.is_some());
        assert!(s.unwrap().contains("doctor"));
    }

    // -- All variants return Some from suggest_resolution ---

    #[test]
    fn test_all_error_codes_have_suggestions() {
        let all_codes = [
            ErrorCode::SessionNotFound,
            ErrorCode::SessionAlreadyExists,
            ErrorCode::SessionNameInvalid,
            ErrorCode::WorkspaceCreationFailed,
            ErrorCode::WorkspaceNotFound,
            ErrorCode::VcsNotInstalled,
            ErrorCode::VcsCommandFailed,
            ErrorCode::NotGitRepository,
            ErrorCode::ZellijNotRunning,
            ErrorCode::ZellijCommandFailed,
            ErrorCode::ConfigNotFound,
            ErrorCode::ConfigParseError,
            ErrorCode::ConfigKeyNotFound,
            ErrorCode::HookFailed,
            ErrorCode::HookExecutionError,
            ErrorCode::StateDbCorrupted,
            ErrorCode::StateDbLocked,
            ErrorCode::ReadUndoLogFailed,
            ErrorCode::WriteUndoLogFailed,
            ErrorCode::SpawnNotOnMain,
            ErrorCode::SpawnInvalidBeadStatus,
            ErrorCode::SpawnBeadNotFound,
            ErrorCode::SpawnWorkspaceCreationFailed,
            ErrorCode::SpawnAgentSpawnFailed,
            ErrorCode::SpawnTimeout,
            ErrorCode::SpawnMergeFailed,
            ErrorCode::SpawnCleanupFailed,
            ErrorCode::SpawnDatabaseError,
            ErrorCode::SpawnVcsCommandFailed,
            ErrorCode::InvalidArgument,
            ErrorCode::Unknown,
        ];

        for code in all_codes {
            assert!(
                code.suggest_resolution().is_some(),
                "ErrorCode {code:?} should have a suggestion"
            );
        }
    }

    // -- From<ErrorCode> for String ---

    #[test]
    fn test_error_code_into_string() {
        let s: String = ErrorCode::SessionNotFound.into();
        assert_eq!(s, "SESSION_NOT_FOUND");

        let s: String = ErrorCode::Unknown.into();
        assert_eq!(s, "UNKNOWN");
    }

    // -- Serialization / Deserialization round-trip ---

    #[test]
    fn test_error_code_serde_round_trip() {
        let code = ErrorCode::SessionNotFound;
        let json = serde_json::to_string(&code).expect("serialize");
        let deserialized: ErrorCode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(code, deserialized);
    }

    #[test]
    fn test_error_code_serde_deserialize_from_string() {
        let code: ErrorCode =
            serde_json::from_str("\"InvalidArgument\"").expect("deserialize");
        assert_eq!(code, ErrorCode::InvalidArgument);
    }

    #[test]
    fn test_error_code_serde_serializes_as_enum_name() {
        let code = ErrorCode::SessionNotFound;
        let json = serde_json::to_string(&code).expect("serialize");
        assert_eq!(json, "\"SessionNotFound\"");
    }
}
