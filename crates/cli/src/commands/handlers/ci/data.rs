//! Data types for the CI command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

/// Options for the CI check command.
#[derive(Debug, Clone)]
pub struct CiCheckOptions {
    /// Show all tracked branches (not just current).
    pub all: bool,
    /// Show current stack branches.
    pub stack: bool,
    /// Output as JSON.
    pub json: bool,
    /// Compact display mode.
    pub verbose: bool,
}

/// Options for the CI watch command.
#[derive(Debug, Clone)]
pub struct CiWatchOptions {
    /// Show all tracked branches (not just current).
    pub all: bool,
    /// Show current stack branches.
    pub stack: bool,
    /// Output as JSON.
    pub json: bool,
    /// Compact display mode.
    pub verbose: bool,
    /// Poll interval in seconds.
    pub interval: u64,
}

/// Subcommands for the CI command.
#[derive(Debug, Clone)]
pub enum CiSubcommand {
    /// Check CI status once.
    Check(CiCheckOptions),
    /// Watch CI status until complete.
    Watch(CiWatchOptions),
}

/// Output of a CI check command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiCheckOutput {
    /// Number of branches checked.
    pub branches_checked: usize,
    /// Number of branches with passing CI.
    pub passing: usize,
    /// Number of branches with failing CI.
    pub failing: usize,
    /// Number of branches with running CI.
    pub running: usize,
    /// Number of branches with no CI.
    pub no_ci: usize,
}

impl CiCheckOutput {
    /// All CI is complete (no pending/running).
    pub fn is_complete(&self) -> bool {
        self.running == 0
    }

    /// Any branch has failures.
    pub fn has_failures(&self) -> bool {
        self.failing > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_check_output_all_passing() {
        let output = CiCheckOutput {
            branches_checked: 3,
            passing: 3,
            failing: 0,
            running: 0,
            no_ci: 0,
        };
        assert!(output.is_complete());
        assert!(!output.has_failures());
    }

    #[test]
    fn ci_check_output_has_running() {
        let output = CiCheckOutput {
            branches_checked: 2,
            passing: 1,
            failing: 0,
            running: 1,
            no_ci: 0,
        };
        assert!(!output.is_complete());
    }

    #[test]
    fn ci_check_output_has_failures() {
        let output = CiCheckOutput {
            branches_checked: 2,
            passing: 1,
            failing: 1,
            running: 0,
            no_ci: 0,
        };
        assert!(output.is_complete());
        assert!(output.has_failures());
    }

    #[test]
    fn ci_check_output_serialization() {
        let output = CiCheckOutput {
            branches_checked: 5,
            passing: 2,
            failing: 1,
            running: 1,
            no_ci: 1,
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("5"));
        assert!(json.contains("2"));
        assert!(json.contains("1"));
    }
}
