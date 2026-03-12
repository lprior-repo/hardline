//! KIRK Contracts for Session CLI operations.
//!
//! Sessions represent parallel workspaces in jj.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::cli_contracts::{Contract, ContractError, Invariant, Postcondition, Precondition};

// ═══════════════════════════════════════════════════════════════════════════
// SESSION INPUT/OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Input for creating a session.
#[derive(Debug, Clone)]
pub struct CreateSessionInput {
    /// Session name
    pub name: String,
    /// Optional parent session (for stacked sessions)
    pub parent: Option<String>,
    /// Optional branch name
    pub branch: Option<String>,
    /// Optional deduplication key
    pub dedupe_key: Option<String>,
}

/// Input for focusing a session.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FocusSessionInput {
    /// Session name or ID
    pub session: String,
}

/// Input for removing a session.
#[derive(Debug, Clone)]
pub struct RemoveSessionInput {
    /// Session name or ID
    pub session: String,
    /// Force removal even with uncommitted changes
    pub force: bool,
}

/// Input for pausing a session.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PauseSessionInput {
    /// Session name or ID
    pub session: String,
}

/// Input for listing sessions.
#[derive(Debug, Clone, Default)]
pub struct ListSessionsInput {
    /// Filter by status
    pub status: Option<String>,
    /// Include stacked sessions
    pub include_stacked: bool,
}

/// Result of session creation.
#[derive(Debug, Clone)]
pub struct SessionResult {
    /// Session ID
    pub id: String,
    /// Session name
    pub name: String,
    /// Current status
    pub status: String,
    /// Workspace path
    pub workspace_path: PathBuf,
}

/// Result of session listing.
#[derive(Debug, Clone)]
pub struct SessionListResult {
    /// List of sessions
    pub sessions: Vec<SessionResult>,
    /// Current session (if any)
    pub current: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION CONTRACTS
// ═══════════════════════════════════════════════════════════════════════════

/// Contracts for Session CLI operations.
pub struct SessionContracts;

impl SessionContracts {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PRECONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Precondition: session name is valid.
    pub const PRECOND_NAME_VALID: Precondition = Precondition::new(
        "name_valid",
        "Session name must start with letter, contain only alphanumeric/dash/underscore, max 64 chars",
    );

    /// Precondition: session exists.
    pub const PRECOND_SESSION_EXISTS: Precondition =
        Precondition::new("session_exists", "Session must exist in the database");

    /// Precondition: session does not already exist.
    pub const PRECOND_SESSION_NOT_EXISTS: Precondition =
        Precondition::new("session_not_exists", "Session name must be unique");

    /// Precondition: parent session exists (for stacked sessions).
    pub const PRECOND_PARENT_EXISTS: Precondition = Precondition::new(
        "parent_exists",
        "Parent session must exist for stacked sessions",
    );

    /// Precondition: no uncommitted changes (for removal without force).
    pub const PRECOND_NO_UNCOMMITTED: Precondition = Precondition::new(
        "no_uncommitted",
        "Session must have no uncommitted changes (or use --force)",
    );

    /// Precondition: session is in correct state.
    pub const PRECOND_SESSION_ACTIVE: Precondition =
        Precondition::new("session_active", "Session must be in active state");

    /// Precondition: max sessions limit not exceeded.
    pub const PRECOND_MAX_SESSIONS: Precondition = Precondition::new(
        "max_sessions",
        "Number of sessions must not exceed configured maximum",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INVARIANTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Invariant: session name is unique.
    pub const INV_NAME_UNIQUE: Invariant = Invariant::documented(
        "name_unique",
        "Session names must be unique across all sessions",
    );

    /// Invariant: workspace path is absolute.
    pub const INV_PATH_ABSOLUTE: Invariant =
        Invariant::documented("path_absolute", "Workspace path must be an absolute path");

    /// Invariant: no circular stacking.
    pub const INV_NO_CYCLES: Invariant =
        Invariant::documented("no_cycles", "Session stacking must not form cycles");

    /// Invariant: timestamps are consistent.
    pub const INV_TIMESTAMPS_CONSISTENT: Invariant = Invariant::documented(
        "session_timestamps_consistent",
        "updated_at >= created_at for all sessions",
    );

    /// Invariant: valid state transitions.
    pub const INV_VALID_TRANSITIONS: Invariant = Invariant::documented(
        "valid_transitions",
        "Session state transitions must follow the defined state machine",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // POSTCONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Postcondition: session was created.
    pub const POST_SESSION_CREATED: Postcondition = Postcondition::new(
        "session_created",
        "Session exists in database with status 'creating'",
    );

    /// Postcondition: session was removed.
    pub const POST_SESSION_REMOVED: Postcondition =
        Postcondition::new("session_removed", "Session no longer exists in database");

    /// Postcondition: workspace was focused.
    pub const POST_WORKSPACE_FOCUSED: Postcondition = Postcondition::new(
        "workspace_focused",
        "Current working directory is session workspace",
    );

    /// Postcondition: session was paused.
    pub const POST_SESSION_PAUSED: Postcondition =
        Postcondition::new("session_paused", "Session status is 'paused'");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALIDATION METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Validate a session name.
    ///
    /// # Errors
    /// Returns `ContractError` if the name is invalid.
    pub fn validate_name(name: &str) -> Result<(), ContractError> {
        if name.is_empty() {
            return Err(ContractError::invalid_input("name", "cannot be empty"));
        }
        if name.len() > 64 {
            return Err(ContractError::invalid_input(
                "name",
                "cannot exceed 64 characters",
            ));
        }
        let first_char = name.chars().next();
        let starts_with_letter = first_char.is_some_and(|c| c.is_ascii_alphabetic());
        if !starts_with_letter {
            return Err(ContractError::invalid_input(
                "name",
                "must start with a letter (a-z, A-Z)",
            ));
        }
        let valid_chars = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !valid_chars {
            return Err(ContractError::invalid_input(
                "name",
                "can only contain alphanumeric characters, dashes, and underscores",
            ));
        }
        Ok(())
    }

    /// Validate a session status.
    ///
    /// # Errors
    /// Returns `ContractError` if the status is invalid.
    pub fn validate_status(status: &str) -> Result<(), ContractError> {
        match status {
            "creating" | "active" | "paused" | "completed" | "failed" => Ok(()),
            _ => Err(ContractError::invalid_input(
                "status",
                "must be one of: creating, active, paused, completed, failed",
            )),
        }
    }

    /// Check if a state transition is valid.
    #[must_use]
    pub fn is_valid_transition(from: &str, to: &str) -> bool {
        matches!(
            (from, to),
            ("creating", "active" | "failed")
                | ("active", "paused" | "completed")
                | ("paused", "active" | "completed")
        )
    }
}

impl Contract<CreateSessionInput, SessionResult> for SessionContracts {
    fn preconditions(input: &CreateSessionInput) -> Result<(), ContractError> {
        Self::validate_name(&input.name)?;

        if let Some(ref parent) = input.parent {
            Self::validate_name(parent)?;
        }

        if let Some(ref branch) = input.branch {
            if branch.is_empty() {
                return Err(ContractError::invalid_input(
                    "branch",
                    "cannot be empty if provided",
                ));
            }
        }

        Ok(())
    }

    fn invariants(_input: &CreateSessionInput) -> Vec<Invariant> {
        vec![
            Self::INV_NAME_UNIQUE,
            Self::INV_PATH_ABSOLUTE,
            Self::INV_NO_CYCLES,
            Self::INV_TIMESTAMPS_CONSISTENT,
        ]
    }

    fn postconditions(
        input: &CreateSessionInput,
        result: &SessionResult,
    ) -> Result<(), ContractError> {
        if result.name != input.name {
            return Err(ContractError::PostconditionFailed {
                name: "name_matches",
                description: "Created session name must match input",
            });
        }
        if !result.workspace_path.is_absolute() {
            return Err(ContractError::PostconditionFailed {
                name: "path_absolute",
                description: "Workspace path must be absolute",
            });
        }
        Ok(())
    }
}

impl Contract<RemoveSessionInput, ()> for SessionContracts {
    fn preconditions(input: &RemoveSessionInput) -> Result<(), ContractError> {
        if input.session.trim().is_empty() {
            return Err(ContractError::invalid_input("session", "cannot be empty"));
        }
        Ok(())
    }

    fn invariants(_input: &RemoveSessionInput) -> Vec<Invariant> {
        vec![]
    }

    fn postconditions(_input: &RemoveSessionInput, _result: &()) -> Result<(), ContractError> {
        // Session should no longer exist - verified by caller
        Ok(())
    }
}

impl Contract<ListSessionsInput, SessionListResult> for SessionContracts {
    fn preconditions(input: &ListSessionsInput) -> Result<(), ContractError> {
        if let Some(ref status) = input.status {
            Self::validate_status(status)?;
        }
        Ok(())
    }

    fn invariants(_input: &ListSessionsInput) -> Vec<Invariant> {
        vec![]
    }

    fn postconditions(
        input: &ListSessionsInput,
        result: &SessionListResult,
    ) -> Result<(), ContractError> {
        if let Some(ref status) = input.status {
            let all_match_status = result.sessions.iter().all(|s| &s.status == status);
            if !all_match_status {
                return Err(ContractError::PostconditionFailed {
                    name: "status_filter",
                    description: "All returned sessions must match the status filter",
                });
            }
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(SessionContracts::validate_name("valid-name").is_ok());
        assert!(SessionContracts::validate_name("Feature_Auth").is_ok());
        assert!(SessionContracts::validate_name("a").is_ok());
        assert!(SessionContracts::validate_name("test123").is_ok());
    }

    #[test]
    fn test_validate_name_empty() {
        assert!(SessionContracts::validate_name("").is_err());
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "a".repeat(65);
        assert!(SessionContracts::validate_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_name_invalid_start() {
        assert!(SessionContracts::validate_name("1invalid").is_err());
        assert!(SessionContracts::validate_name("-invalid").is_err());
        assert!(SessionContracts::validate_name("_invalid").is_err());
    }

    #[test]
    fn test_validate_name_invalid_chars() {
        assert!(SessionContracts::validate_name("invalid name").is_err());
        assert!(SessionContracts::validate_name("invalid@name").is_err());
    }

    #[test]
    fn test_validate_status_valid() {
        assert!(SessionContracts::validate_status("creating").is_ok());
        assert!(SessionContracts::validate_status("active").is_ok());
        assert!(SessionContracts::validate_status("paused").is_ok());
        assert!(SessionContracts::validate_status("completed").is_ok());
        assert!(SessionContracts::validate_status("failed").is_ok());
    }

    #[test]
    fn test_validate_status_invalid() {
        assert!(SessionContracts::validate_status("pending").is_err());
        assert!(SessionContracts::validate_status("running").is_err());
    }

    #[test]
    fn test_is_valid_transition() {
        assert!(SessionContracts::is_valid_transition("creating", "active"));
        assert!(SessionContracts::is_valid_transition("creating", "failed"));
        assert!(SessionContracts::is_valid_transition("active", "paused"));
        assert!(SessionContracts::is_valid_transition("active", "completed"));
        assert!(SessionContracts::is_valid_transition("paused", "active"));
        assert!(SessionContracts::is_valid_transition("paused", "completed"));

        assert!(!SessionContracts::is_valid_transition("creating", "paused"));
        assert!(!SessionContracts::is_valid_transition(
            "completed",
            "active"
        ));
        assert!(!SessionContracts::is_valid_transition("failed", "active"));
    }

    #[test]
    fn test_create_session_contract_preconditions() {
        let input = CreateSessionInput {
            name: "valid-name".to_string(),
            parent: None,
            branch: None,
            dedupe_key: None,
        };
        assert!(SessionContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_create_session_contract_preconditions_fails() {
        let input = CreateSessionInput {
            name: "1invalid".to_string(),
            parent: None,
            branch: None,
            dedupe_key: None,
        };
        assert!(SessionContracts::preconditions(&input).is_err());
    }

    #[test]
    fn test_create_session_contract_postconditions() {
        let input = CreateSessionInput {
            name: "test-session".to_string(),
            parent: None,
            branch: None,
            dedupe_key: None,
        };
        let result = SessionResult {
            id: "session-123".to_string(),
            name: "test-session".to_string(),
            status: "creating".to_string(),
            workspace_path: PathBuf::from("/tmp/workspace"),
        };
        assert!(SessionContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_create_session_contract_postconditions_fails_relative_path() {
        let input = CreateSessionInput {
            name: "test-session".to_string(),
            parent: None,
            branch: None,
            dedupe_key: None,
        };
        let result = SessionResult {
            id: "session-123".to_string(),
            name: "test-session".to_string(),
            status: "creating".to_string(),
            workspace_path: PathBuf::from("relative/path"),
        };
        assert!(SessionContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_list_sessions_contract_postconditions_filter() {
        let input = ListSessionsInput {
            status: Some("active".to_string()),
            include_stacked: false,
        };
        let result = SessionListResult {
            sessions: vec![
                SessionResult {
                    id: "1".to_string(),
                    name: "s1".to_string(),
                    status: "active".to_string(),
                    workspace_path: PathBuf::from("/tmp/1"),
                },
                SessionResult {
                    id: "2".to_string(),
                    name: "s2".to_string(),
                    status: "active".to_string(),
                    workspace_path: PathBuf::from("/tmp/2"),
                },
            ],
            current: Some("s1".to_string()),
        };
        assert!(SessionContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_list_sessions_contract_postconditions_filter_mismatch() {
        let input = ListSessionsInput {
            status: Some("active".to_string()),
            include_stacked: false,
        };
        let result = SessionListResult {
            sessions: vec![
                SessionResult {
                    id: "1".to_string(),
                    name: "s1".to_string(),
                    status: "active".to_string(),
                    workspace_path: PathBuf::from("/tmp/1"),
                },
                SessionResult {
                    id: "2".to_string(),
                    name: "s2".to_string(),
                    status: "paused".to_string(),
                    workspace_path: PathBuf::from("/tmp/2"),
                },
            ],
            current: Some("s1".to_string()),
        };
        assert!(SessionContracts::postconditions(&input, &result).is_err());
    }
}
