//! KIRK Contracts for Session CLI operations.
//!
//! Sessions represent parallel workspaces in jj.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::cli_contracts::{
    domain_types::{NonEmptyString, SessionName, SessionStatus},
    Contract, ContractError, Invariant, Postcondition, Precondition,
};

// ═══════════════════════════════════════════════════════════════════════════
// SESSION INPUT/OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Input for creating a session.
#[derive(Debug, Clone)]
pub struct CreateSessionInput {
    /// Session name
    pub name: SessionName,
    /// Optional parent session (for stacked sessions)
    pub parent: Option<SessionName>,
    /// Optional branch name
    pub branch: Option<NonEmptyString>,
    /// Optional deduplication key
    pub dedupe_key: Option<String>,
}

/// Input for focusing a session.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FocusSessionInput {
    /// Session name or ID
    pub session: SessionName,
}

/// Input for removing a session.
#[derive(Debug, Clone)]
pub struct RemoveSessionInput {
    /// Session name or ID
    pub session: SessionName,
    /// Force removal even with uncommitted changes
    pub force: ForceMode,
}

/// Mode for force operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceMode {
    Force,
    NoForce,
}

impl ForceMode {
    #[must_use]
    pub const fn is_force(self) -> bool {
        matches!(self, Self::Force)
    }
}

/// Input for pausing a session.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PauseSessionInput {
    /// Session name or ID
    pub session: SessionName,
}

/// Input for listing sessions.
#[derive(Debug, Clone, Default)]
pub struct ListSessionsInput {
    /// Filter by status
    pub status: Option<SessionStatus>,
    /// Include stacked sessions
    pub include_stacked: bool,
}

/// Result of session creation.
#[derive(Debug, Clone)]
pub struct SessionResult {
    /// Session ID
    pub id: String,
    /// Session name
    pub name: SessionName,
    /// Current status
    pub status: SessionStatus,
    /// Workspace path
    pub workspace_path: PathBuf,
}

/// Result of session listing.
#[derive(Debug, Clone)]
pub struct SessionListResult {
    /// List of sessions
    pub sessions: Vec<SessionResult>,
    /// Current session (if any)
    pub current: Option<SessionName>,
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
}

impl Contract<CreateSessionInput, SessionResult> for SessionContracts {
    fn preconditions(_input: &CreateSessionInput) -> Result<(), ContractError> {
        // Validation is now done at the boundary when creating SessionName
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
    fn preconditions(_input: &RemoveSessionInput) -> Result<(), ContractError> {
        // Validation is now done at the boundary when creating SessionName
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
    fn preconditions(_input: &ListSessionsInput) -> Result<(), ContractError> {
        // Validation is now done at the boundary when creating SessionStatus
        Ok(())
    }

    fn invariants(_input: &ListSessionsInput) -> Vec<Invariant> {
        vec![]
    }

    fn postconditions(
        input: &ListSessionsInput,
        result: &SessionListResult,
    ) -> Result<(), ContractError> {
        if let Some(status) = input.status {
            let all_match_status = result.sessions.iter().all(|s| s.status == status);
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

    /// Helper macro to unwrap Result in tests with context
    macro_rules! unwrap_ok {
        ($expr:expr, $msg:expr) => {
            match $expr {
                Ok(v) => v,
                Err(e) => panic!("{}: {:?}", $msg, e),
            }
        };
    }

    #[test]
    fn test_create_session_contract_preconditions() {
        let input = CreateSessionInput {
            name: unwrap_ok!(SessionName::try_from("valid-name"), "Failed to create SessionName"),
            parent: None,
            branch: None,
            dedupe_key: None,
        };
        assert!(SessionContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_create_session_contract_postconditions() {
        let input = CreateSessionInput {
            name: unwrap_ok!(SessionName::try_from("test-session"), "Failed to create SessionName"),
            parent: None,
            branch: None,
            dedupe_key: None,
        };
        let result = SessionResult {
            id: "session-123".to_string(),
            name: unwrap_ok!(SessionName::try_from("test-session"), "Failed to create SessionName"),
            status: SessionStatus::Creating,
            workspace_path: PathBuf::from("/tmp/workspace"),
        };
        assert!(SessionContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_create_session_contract_postconditions_fails_relative_path() {
        let input = CreateSessionInput {
            name: unwrap_ok!(SessionName::try_from("test-session"), "Failed to create SessionName"),
            parent: None,
            branch: None,
            dedupe_key: None,
        };
        let result = SessionResult {
            id: "session-123".to_string(),
            name: unwrap_ok!(SessionName::try_from("test-session"), "Failed to create SessionName"),
            status: SessionStatus::Creating,
            workspace_path: PathBuf::from("relative/path"),
        };
        assert!(SessionContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_list_sessions_contract_postconditions_filter() {
        let input = ListSessionsInput {
            status: Some(SessionStatus::Active),
            include_stacked: false,
        };
        let result = SessionListResult {
            sessions: vec![
                SessionResult {
                    id: "1".to_string(),
                    name: unwrap_ok!(SessionName::try_from("s1"), "Failed to create SessionName"),
                    status: SessionStatus::Active,
                    workspace_path: PathBuf::from("/tmp/1"),
                },
                SessionResult {
                    id: "2".to_string(),
                    name: unwrap_ok!(SessionName::try_from("s2"), "Failed to create SessionName"),
                    status: SessionStatus::Active,
                    workspace_path: PathBuf::from("/tmp/2"),
                },
            ],
            current: Some(unwrap_ok!(SessionName::try_from("s1"), "Failed to create SessionName")),
        };
        assert!(SessionContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_list_sessions_contract_postconditions_filter_mismatch() {
        let input = ListSessionsInput {
            status: Some(SessionStatus::Active),
            include_stacked: false,
        };
        let result = SessionListResult {
            sessions: vec![
                SessionResult {
                    id: "1".to_string(),
                    name: unwrap_ok!(SessionName::try_from("s1"), "Failed to create SessionName"),
                    status: SessionStatus::Active,
                    workspace_path: PathBuf::from("/tmp/1"),
                },
                SessionResult {
                    id: "2".to_string(),
                    name: unwrap_ok!(SessionName::try_from("s2"), "Failed to create SessionName"),
                    status: SessionStatus::Paused, // Wrong!
                    workspace_path: PathBuf::from("/tmp/2"),
                },
            ],
            current: Some(unwrap_ok!(SessionName::try_from("s1"), "Failed to create SessionName")),
        };
        assert!(SessionContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_force_mode() {
        assert!(ForceMode::Force.is_force());
        assert!(!ForceMode::NoForce.is_force());
    }
}
