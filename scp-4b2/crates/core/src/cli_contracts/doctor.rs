//! KIRK Contracts for Doctor CLI operations.
//!
//! Doctor provides system diagnostics and health checks.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::cli_contracts::{Contract, ContractError, Invariant, Postcondition, Precondition};

// ═══════════════════════════════════════════════════════════════════════════
// DOCTOR INPUT/OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Input for running diagnostics.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct RunDoctorInput {
    /// Specific checks to run (default: all)
    pub checks: Vec<String>,
    /// Fix issues automatically
    pub fix: bool,
    /// Verbose output
    pub verbose: bool,
}

/// Input for checking a specific component.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CheckComponentInput {
    /// Component name
    pub component: String,
    /// Attempt fix
    pub fix: bool,
}

/// Result of doctor run.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DoctorResult {
    /// Individual check results
    pub checks: Vec<CheckResult>,
    /// Overall status
    pub status: DoctorStatus,
    /// Issues that were fixed
    pub fixed: Vec<String>,
    /// Issues that could not be fixed
    pub unfixed: Vec<String>,
}

/// Result of a single check.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CheckResult {
    /// Check name
    pub name: String,
    /// Check status
    pub status: CheckStatus,
    /// Message describing the result
    pub message: String,
    /// Suggested fix (if applicable)
    pub fix: Option<String>,
}

/// Overall doctor status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorStatus {
    /// All checks passed
    Healthy,
    /// Some checks failed but can be fixed
    Fixable,
    /// Some checks failed and cannot be auto-fixed
    Unhealthy,
}

/// Individual check status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// Check passed
    Pass,
    /// Check failed, fix available
    Fail,
    /// Check failed, no fix available
    Error,
    /// Check skipped
    Skip,
    /// Check was fixed
    Fixed,
}

// ═══════════════════════════════════════════════════════════════════════════
// DOCTOR CONTRACTS
// ═══════════════════════════════════════════════════════════════════════════

/// Contracts for Doctor CLI operations.
#[allow(dead_code)]
pub struct DoctorContracts;

#[allow(dead_code)]
impl DoctorContracts {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PRECONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Precondition: check name is valid.
    pub const PRECOND_CHECK_VALID: Precondition =
        Precondition::new("check_valid", "Check name must be a known check");

    /// Precondition: not running as root (for safety).
    pub const PRECOND_NOT_ROOT: Precondition =
        Precondition::new("not_root", "Should not run as root user");

    /// Precondition: write access for fixes.
    pub const PRECOND_WRITE_ACCESS: Precondition =
        Precondition::new("write_access", "Must have write access to apply fixes");

    /// Precondition: database is accessible.
    pub const PRECOND_DB_ACCESSIBLE: Precondition =
        Precondition::new("db_accessible", "Database must be accessible for DB checks");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INVARIANTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Invariant: doctor is read-only unless --fix.
    pub const INV_READONLY_UNLESS_FIX: Invariant = Invariant::documented(
        "readonly_unless_fix",
        "Doctor does not modify state unless --fix is specified",
    );

    /// Invariant: fixes are idempotent.
    pub const INV_FIXES_IDEMPOTENT: Invariant = Invariant::documented(
        "fixes_idempotent",
        "Running fixes multiple times has same result",
    );

    /// Invariant: checks are independent.
    pub const INV_CHECKS_INDEPENDENT: Invariant =
        Invariant::documented("checks_independent", "Each check can run independently");

    /// Invariant: no destructive fixes without confirmation.
    pub const INV_NO_DESTRUCTIVE: Invariant = Invariant::documented(
        "no_destructive",
        "Destructive fixes require explicit confirmation",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // POSTCONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Postcondition: all requested checks were run.
    pub const POST_ALL_CHECKS_RUN: Postcondition =
        Postcondition::new("all_checks_run", "All requested checks have been executed");

    /// Postcondition: fixes were applied (if --fix).
    pub const POST_FIXES_APPLIED: Postcondition = Postcondition::new(
        "fixes_applied",
        "Applicable fixes were applied when --fix specified",
    );

    /// Postcondition: no new issues introduced.
    pub const POST_NO_NEW_ISSUES: Postcondition =
        Postcondition::new("no_new_issues", "No new issues were introduced by fixes");

    /// Postcondition: status accurately reflects state.
    pub const POST_STATUS_ACCURATE: Postcondition = Postcondition::new(
        "status_accurate",
        "Status accurately reflects check results",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // KNOWN CHECKS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// List of known doctor checks.
    #[must_use]
    pub const fn known_checks() -> &'static [(&'static str, &'static str)] {
        &[
            ("jj_installed", "Check that jj is installed and accessible"),
            ("jj_version", "Check jj version compatibility"),
            ("git_installed", "Check that git is installed"),
            ("database", "Check database integrity"),
            ("database_locked", "Check for stale database locks"),
            ("workspaces", "Check workspace integrity"),
            ("sessions", "Check session consistency"),
            ("config", "Check configuration validity"),
            ("hooks", "Check hooks are executable"),
            ("orphaned", "Check for orphaned resources"),
        ]
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALIDATION METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Validate a check name.
    ///
    /// # Errors
    /// Returns `ContractError` if the check name is unknown.
    pub fn validate_check(name: &str) -> Result<(), ContractError> {
        if name.is_empty() {
            return Err(ContractError::invalid_input("check", "cannot be empty"));
        }

        let is_known = Self::known_checks().iter().any(|(k, _)| *k == name);
        if !is_known {
            return Err(ContractError::invalid_input(
                "check",
                format!("unknown check '{name}'"),
            ));
        }
        Ok(())
    }

    /// Check if a check is a known check.
    #[must_use]
    pub fn is_known_check(name: &str) -> bool {
        Self::known_checks().iter().any(|(k, _)| *k == name)
    }
}

impl Contract<RunDoctorInput, DoctorResult> for DoctorContracts {
    fn preconditions(input: &RunDoctorInput) -> Result<(), ContractError> {
        for check in &input.checks {
            Self::validate_check(check)?;
        }
        Ok(())
    }

    fn invariants(_input: &RunDoctorInput) -> Vec<Invariant> {
        vec![
            Self::INV_READONLY_UNLESS_FIX,
            Self::INV_FIXES_IDEMPOTENT,
            Self::INV_CHECKS_INDEPENDENT,
        ]
    }

    fn postconditions(input: &RunDoctorInput, result: &DoctorResult) -> Result<(), ContractError> {
        // Verify all requested checks were run
        if input.checks.is_empty() {
            // All checks should have been run
            let expected_count = Self::known_checks().len();
            if result.checks.len() < expected_count {
                return Err(ContractError::PostconditionFailed {
                    name: "all_checks_run",
                    description: "Not all checks were executed",
                });
            }
        } else {
            // Specific checks requested
            for requested in &input.checks {
                let was_run = result.checks.iter().any(|c| &c.name == requested);
                if !was_run {
                    return Err(ContractError::PostconditionFailed {
                        name: "requested_check_run",
                        description: "A requested check was not run",
                    });
                }
            }
        }

        // Verify status is consistent with check results
        let all_passed = result
            .checks
            .iter()
            .all(|c| c.status == CheckStatus::Pass || c.status == CheckStatus::Skip);

        let expected_status = if all_passed {
            DoctorStatus::Healthy
        } else if result.fixed.len() == result.unfixed.len() + result.fixed.len() {
            // All issues were fixed
            DoctorStatus::Healthy
        } else if result.unfixed.is_empty() {
            DoctorStatus::Fixable
        } else {
            DoctorStatus::Unhealthy
        };

        if result.status != expected_status {
            return Err(ContractError::PostconditionFailed {
                name: "status_consistent",
                description: "Status does not match check results",
            });
        }

        Ok(())
    }
}

impl Contract<CheckComponentInput, CheckResult> for DoctorContracts {
    fn preconditions(input: &CheckComponentInput) -> Result<(), ContractError> {
        Self::validate_check(&input.component)
    }

    fn invariants(_input: &CheckComponentInput) -> Vec<Invariant> {
        vec![Self::INV_READONLY_UNLESS_FIX]
    }

    fn postconditions(
        input: &CheckComponentInput,
        result: &CheckResult,
    ) -> Result<(), ContractError> {
        if result.name != input.component {
            return Err(ContractError::PostconditionFailed {
                name: "name_matches",
                description: "Result name must match requested component",
            });
        }
        if result.message.is_empty() {
            return Err(ContractError::PostconditionFailed {
                name: "message_present",
                description: "Check result must have a message",
            });
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
    fn test_validate_check_valid() {
        assert!(DoctorContracts::validate_check("jj_installed").is_ok());
        assert!(DoctorContracts::validate_check("database").is_ok());
        assert!(DoctorContracts::validate_check("sessions").is_ok());
    }

    #[test]
    fn test_validate_check_empty() {
        assert!(DoctorContracts::validate_check("").is_err());
    }

    #[test]
    fn test_validate_check_unknown() {
        assert!(DoctorContracts::validate_check("unknown_check").is_err());
    }

    #[test]
    fn test_is_known_check() {
        assert!(DoctorContracts::is_known_check("jj_installed"));
        assert!(DoctorContracts::is_known_check("database"));
        assert!(!DoctorContracts::is_known_check("fake_check"));
    }

    #[test]
    fn test_run_doctor_contract_preconditions() {
        let input = RunDoctorInput {
            checks: vec!["jj_installed".to_string(), "database".to_string()],
            fix: false,
            verbose: false,
        };
        assert!(DoctorContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_run_doctor_contract_preconditions_invalid_check() {
        let input = RunDoctorInput {
            checks: vec!["invalid".to_string()],
            fix: false,
            verbose: false,
        };
        assert!(DoctorContracts::preconditions(&input).is_err());
    }

    #[test]
    fn test_run_doctor_contract_postconditions_all_passed() {
        let input = RunDoctorInput {
            checks: vec!["jj_installed".to_string()],
            fix: false,
            verbose: false,
        };
        let result = DoctorResult {
            checks: vec![CheckResult {
                name: "jj_installed".to_string(),
                status: CheckStatus::Pass,
                message: "jj is installed".to_string(),
                fix: None,
            }],
            status: DoctorStatus::Healthy,
            fixed: vec![],
            unfixed: vec![],
        };
        assert!(DoctorContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_run_doctor_contract_postconditions_missing_check() {
        let input = RunDoctorInput {
            checks: vec!["jj_installed".to_string(), "database".to_string()],
            fix: false,
            verbose: false,
        };
        let result = DoctorResult {
            checks: vec![CheckResult {
                name: "jj_installed".to_string(),
                status: CheckStatus::Pass,
                message: "jj is installed".to_string(),
                fix: None,
            }],
            status: DoctorStatus::Healthy,
            fixed: vec![],
            unfixed: vec![],
        };
        assert!(DoctorContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_run_doctor_contract_postconditions_status_mismatch() {
        let input = RunDoctorInput {
            checks: vec!["jj_installed".to_string()],
            fix: false,
            verbose: false,
        };
        let result = DoctorResult {
            checks: vec![CheckResult {
                name: "jj_installed".to_string(),
                status: CheckStatus::Fail,
                message: "jj not found".to_string(),
                fix: Some("Install jj".to_string()),
            }],
            status: DoctorStatus::Healthy, // Wrong! Should be Fixable
            fixed: vec![],
            unfixed: vec!["jj_installed".to_string()],
        };
        assert!(DoctorContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_check_component_contract_preconditions() {
        let input = CheckComponentInput {
            component: "jj_installed".to_string(),
            fix: false,
        };
        assert!(DoctorContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_check_component_contract_postconditions() {
        let input = CheckComponentInput {
            component: "jj_installed".to_string(),
            fix: false,
        };
        let result = CheckResult {
            name: "jj_installed".to_string(),
            status: CheckStatus::Pass,
            message: "jj is installed".to_string(),
            fix: None,
        };
        assert!(DoctorContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_check_component_contract_postconditions_name_mismatch() {
        let input = CheckComponentInput {
            component: "jj_installed".to_string(),
            fix: false,
        };
        let result = CheckResult {
            name: "jj_version".to_string(), // Wrong!
            status: CheckStatus::Pass,
            message: "jj is installed".to_string(),
            fix: None,
        };
        assert!(DoctorContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_check_component_contract_postconditions_empty_message() {
        let input = CheckComponentInput {
            component: "jj_installed".to_string(),
            fix: false,
        };
        let result = CheckResult {
            name: "jj_installed".to_string(),
            status: CheckStatus::Pass,
            message: String::new(), // Wrong!
            fix: None,
        };
        assert!(DoctorContracts::postconditions(&input, &result).is_err());
    }
}
