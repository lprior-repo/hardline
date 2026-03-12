//! KIRK Contracts for Status CLI operations.
//!
//! Status provides visibility into the current state of sessions and work.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::cli_contracts::{Contract, ContractError, Invariant, Postcondition, Precondition};

// ═══════════════════════════════════════════════════════════════════════════
// STATUS INPUT/OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Input for getting status.
#[derive(Debug, Clone, Default)]
pub struct GetStatusInput {
    /// Session to get status for (defaults to current)
    pub session: Option<String>,
    /// Include detailed information
    pub detailed: bool,
    /// Output format
    pub format: Option<String>,
}

/// Input for diff output.
#[derive(Debug, Clone)]
pub struct DiffInput {
    /// Session to diff (defaults to current)
    pub session: Option<String>,
    /// Compare against specific revision
    pub base: Option<String>,
    /// Include diff stats only
    pub stat: bool,
}

/// Input for log output.
#[derive(Debug, Clone)]
pub struct LogInput {
    /// Session to log (defaults to current)
    pub session: Option<String>,
    /// Number of commits to show
    pub limit: Option<usize>,
    /// Show graph
    pub graph: bool,
}

/// Result of status query.
#[derive(Debug, Clone)]
pub struct StatusResult {
    /// Session name
    pub session: String,
    /// Session status
    pub status: String,
    /// Workspace state
    pub state: String,
    /// Current branch
    pub branch: Option<String>,
    /// Number of changed files
    pub changes: usize,
    /// Has uncommitted changes
    pub has_uncommitted: bool,
    /// Workspace path
    pub workspace_path: PathBuf,
}

/// Result of diff query.
#[derive(Debug, Clone)]
pub struct DiffResult {
    /// Files changed
    pub files: Vec<FileDiff>,
    /// Total insertions
    pub insertions: usize,
    /// Total deletions
    pub deletions: usize,
}

/// File diff information.
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// File path
    pub path: String,
    /// Status (M, A, D, R)
    pub status: String,
    /// Insertions
    pub insertions: usize,
    /// Deletions
    pub deletions: usize,
}

/// Result of log query.
#[derive(Debug, Clone)]
pub struct LogResult {
    /// Commit entries
    pub commits: Vec<CommitEntry>,
    /// Has more commits
    pub has_more: bool,
}

/// Commit entry.
#[derive(Debug, Clone)]
pub struct CommitEntry {
    /// Commit ID
    pub id: String,
    /// Commit message
    pub message: String,
    /// Author
    pub author: String,
    /// Timestamp
    pub timestamp: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// STATUS CONTRACTS
// ═══════════════════════════════════════════════════════════════════════════

/// Contracts for Status CLI operations.
pub struct StatusContracts;

impl StatusContracts {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PRECONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Precondition: session exists (if specified).
    pub const PRECOND_SESSION_EXISTS: Precondition =
        Precondition::new("session_exists", "Session must exist if specified");

    /// Precondition: inside a jj workspace.
    pub const PRECOND_IN_JJ_WORKSPACE: Precondition =
        Precondition::new("in_jj_workspace", "Must be inside a jj workspace");

    /// Precondition: output format is valid.
    pub const PRECOND_FORMAT_VALID: Precondition = Precondition::new(
        "format_valid",
        "Output format must be one of: text, json, yaml",
    );

    /// Precondition: limit is reasonable.
    pub const PRECOND_LIMIT_VALID: Precondition =
        Precondition::new("limit_valid", "Limit must be between 1 and 1000");

    /// Precondition: base revision exists.
    pub const PRECOND_BASE_EXISTS: Precondition =
        Precondition::new("base_exists", "Base revision must exist");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INVARIANTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Invariant: status is read-only (no side effects).
    pub const INV_READ_ONLY: Invariant =
        Invariant::documented("read_only", "Status operations do not modify state");

    /// Invariant: diff counts are consistent.
    pub const INV_DIFF_CONSISTENT: Invariant = Invariant::documented(
        "diff_consistent",
        "File counts match sum of individual files",
    );

    /// Invariant: log is ordered chronologically.
    pub const INV_LOG_ORDERED: Invariant =
        Invariant::documented("log_ordered", "Commits are in reverse chronological order");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // POSTCONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Postcondition: status is accurate.
    pub const POST_STATUS_ACCURATE: Postcondition =
        Postcondition::new("status_accurate", "Status reflects current workspace state");

    /// Postcondition: no state modification.
    pub const POST_NO_MODIFICATION: Postcondition =
        Postcondition::new("no_modification", "No state was modified by status query");

    /// Postcondition: diff matches actual changes.
    pub const POST_DIFF_ACCURATE: Postcondition =
        Postcondition::new("diff_accurate", "Diff reflects actual file differences");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALIDATION METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Validate an output format.
    ///
    /// # Errors
    /// Returns `ContractError` if the format is invalid.
    pub fn validate_format(format: &str) -> Result<(), ContractError> {
        match format {
            "text" | "json" | "yaml" => Ok(()),
            _ => Err(ContractError::invalid_input(
                "format",
                "must be one of: text, json, yaml",
            )),
        }
    }

    /// Validate a limit value.
    ///
    /// # Errors
    /// Returns `ContractError` if the limit is invalid.
    pub fn validate_limit(limit: usize) -> Result<(), ContractError> {
        if limit == 0 {
            return Err(ContractError::invalid_input("limit", "must be at least 1"));
        }
        if limit > 1000 {
            return Err(ContractError::invalid_input("limit", "cannot exceed 1000"));
        }
        Ok(())
    }

    /// Validate a file status.
    ///
    /// # Errors
    /// Returns `ContractError` if the status is invalid.
    pub fn validate_file_status(status: &str) -> Result<(), ContractError> {
        match status {
            "M" | "A" | "D" | "R" | "?" => Ok(()),
            _ => Err(ContractError::invalid_input(
                "status",
                "must be one of: M, A, D, R, ?",
            )),
        }
    }
}

impl Contract<GetStatusInput, StatusResult> for StatusContracts {
    fn preconditions(input: &GetStatusInput) -> Result<(), ContractError> {
        if let Some(ref session) = input.session {
            if session.trim().is_empty() {
                return Err(ContractError::invalid_input(
                    "session",
                    "cannot be empty if provided",
                ));
            }
        }

        if let Some(ref format) = input.format {
            Self::validate_format(format)?;
        }

        Ok(())
    }

    fn invariants(_input: &GetStatusInput) -> Vec<Invariant> {
        vec![Self::INV_READ_ONLY]
    }

    fn postconditions(_input: &GetStatusInput, result: &StatusResult) -> Result<(), ContractError> {
        if result.session.trim().is_empty() {
            return Err(ContractError::PostconditionFailed {
                name: "session_set",
                description: "Status result must have a session name",
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

impl Contract<DiffInput, DiffResult> for StatusContracts {
    fn preconditions(input: &DiffInput) -> Result<(), ContractError> {
        if let Some(ref session) = input.session {
            if session.trim().is_empty() {
                return Err(ContractError::invalid_input(
                    "session",
                    "cannot be empty if provided",
                ));
            }
        }

        if let Some(ref base) = input.base {
            if base.trim().is_empty() {
                return Err(ContractError::invalid_input(
                    "base",
                    "cannot be empty if provided",
                ));
            }
        }

        Ok(())
    }

    fn invariants(_input: &DiffInput) -> Vec<Invariant> {
        vec![Self::INV_READ_ONLY, Self::INV_DIFF_CONSISTENT]
    }

    fn postconditions(_input: &DiffInput, result: &DiffResult) -> Result<(), ContractError> {
        // Verify diff counts are consistent
        let total_insertions: usize = result.files.iter().map(|f| f.insertions).sum();
        let total_deletions: usize = result.files.iter().map(|f| f.deletions).sum();

        if total_insertions != result.insertions {
            return Err(ContractError::PostconditionFailed {
                name: "insertions_match",
                description: "Total insertions must match sum of file insertions",
            });
        }
        if total_deletions != result.deletions {
            return Err(ContractError::PostconditionFailed {
                name: "deletions_match",
                description: "Total deletions must match sum of file deletions",
            });
        }
        Ok(())
    }
}

impl Contract<LogInput, LogResult> for StatusContracts {
    fn preconditions(input: &LogInput) -> Result<(), ContractError> {
        if let Some(ref session) = input.session {
            if session.trim().is_empty() {
                return Err(ContractError::invalid_input(
                    "session",
                    "cannot be empty if provided",
                ));
            }
        }

        if let Some(limit) = input.limit {
            Self::validate_limit(limit)?;
        }

        Ok(())
    }

    fn invariants(_input: &LogInput) -> Vec<Invariant> {
        vec![Self::INV_READ_ONLY, Self::INV_LOG_ORDERED]
    }

    fn postconditions(input: &LogInput, result: &LogResult) -> Result<(), ContractError> {
        // Verify limit is respected
        if let Some(limit) = input.limit {
            if result.commits.len() > limit {
                return Err(ContractError::PostconditionFailed {
                    name: "limit_respected",
                    description: "Number of commits must not exceed limit",
                });
            }
        }

        // Verify commits have required fields
        for commit in &result.commits {
            if commit.id.trim().is_empty() {
                return Err(ContractError::PostconditionFailed {
                    name: "commit_id_set",
                    description: "Each commit must have an ID",
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
    fn test_validate_format_valid() {
        assert!(StatusContracts::validate_format("text").is_ok());
        assert!(StatusContracts::validate_format("json").is_ok());
        assert!(StatusContracts::validate_format("yaml").is_ok());
    }

    #[test]
    fn test_validate_format_invalid() {
        assert!(StatusContracts::validate_format("xml").is_err());
        assert!(StatusContracts::validate_format("toml").is_err());
    }

    #[test]
    fn test_validate_limit_valid() {
        assert!(StatusContracts::validate_limit(1).is_ok());
        assert!(StatusContracts::validate_limit(100).is_ok());
        assert!(StatusContracts::validate_limit(1000).is_ok());
    }

    #[test]
    fn test_validate_limit_invalid() {
        assert!(StatusContracts::validate_limit(0).is_err());
        assert!(StatusContracts::validate_limit(1001).is_err());
    }

    #[test]
    fn test_validate_file_status_valid() {
        assert!(StatusContracts::validate_file_status("M").is_ok());
        assert!(StatusContracts::validate_file_status("A").is_ok());
        assert!(StatusContracts::validate_file_status("D").is_ok());
        assert!(StatusContracts::validate_file_status("R").is_ok());
        assert!(StatusContracts::validate_file_status("?").is_ok());
    }

    #[test]
    fn test_validate_file_status_invalid() {
        assert!(StatusContracts::validate_file_status("C").is_err());
        assert!(StatusContracts::validate_file_status("X").is_err());
    }

    #[test]
    fn test_get_status_contract_preconditions() {
        let input = GetStatusInput {
            session: Some("test".to_string()),
            detailed: false,
            format: Some("json".to_string()),
        };
        assert!(StatusContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_get_status_contract_postconditions() {
        let input = GetStatusInput::default();
        let result = StatusResult {
            session: "test".to_string(),
            status: "active".to_string(),
            state: "working".to_string(),
            branch: Some("main".to_string()),
            changes: 5,
            has_uncommitted: true,
            workspace_path: PathBuf::from("/tmp/test"),
        };
        assert!(StatusContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_get_status_contract_postconditions_relative_path() {
        let input = GetStatusInput::default();
        let result = StatusResult {
            session: "test".to_string(),
            status: "active".to_string(),
            state: "working".to_string(),
            branch: Some("main".to_string()),
            changes: 5,
            has_uncommitted: true,
            workspace_path: PathBuf::from("relative/path"),
        };
        assert!(StatusContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_diff_contract_postconditions_consistent() {
        let input = DiffInput {
            session: None,
            base: None,
            stat: false,
        };
        let result = DiffResult {
            files: vec![
                FileDiff {
                    path: "file1.txt".to_string(),
                    status: "M".to_string(),
                    insertions: 5,
                    deletions: 2,
                },
                FileDiff {
                    path: "file2.txt".to_string(),
                    status: "A".to_string(),
                    insertions: 10,
                    deletions: 0,
                },
            ],
            insertions: 15,
            deletions: 2,
        };
        assert!(StatusContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_diff_contract_postconditions_inconsistent() {
        let input = DiffInput {
            session: None,
            base: None,
            stat: false,
        };
        let result = DiffResult {
            files: vec![FileDiff {
                path: "file1.txt".to_string(),
                status: "M".to_string(),
                insertions: 5,
                deletions: 2,
            }],
            insertions: 100, // Wrong!
            deletions: 2,
        };
        assert!(StatusContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_log_contract_postconditions_limit() {
        let input = LogInput {
            session: None,
            limit: Some(2),
            graph: false,
        };
        let result = LogResult {
            commits: vec![
                CommitEntry {
                    id: "abc123".to_string(),
                    message: "First".to_string(),
                    author: "Alice".to_string(),
                    timestamp: "2024-01-01".to_string(),
                },
                CommitEntry {
                    id: "def456".to_string(),
                    message: "Second".to_string(),
                    author: "Bob".to_string(),
                    timestamp: "2024-01-02".to_string(),
                },
            ],
            has_more: true,
        };
        assert!(StatusContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_log_contract_postconditions_exceeds_limit() {
        let input = LogInput {
            session: None,
            limit: Some(1),
            graph: false,
        };
        let result = LogResult {
            commits: vec![
                CommitEntry {
                    id: "abc123".to_string(),
                    message: "First".to_string(),
                    author: "Alice".to_string(),
                    timestamp: "2024-01-01".to_string(),
                },
                CommitEntry {
                    id: "def456".to_string(),
                    message: "Second".to_string(),
                    author: "Bob".to_string(),
                    timestamp: "2024-01-02".to_string(),
                },
            ],
            has_more: true,
        };
        assert!(StatusContracts::postconditions(&input, &result).is_err());
    }
}
