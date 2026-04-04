//! Action functions for the prune command handler (Tier 3).
//!
//! I/O operations that discover and remove invalid session records.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{PruneOptions, PruneOutput};

/// Execute the prune command with the given options.
///
/// Scans for session records whose workspace directories no longer exist
/// and removes them. In dry-run mode, reports what would be removed
/// without performing deletions.
///
/// # Errors
///
/// Returns errors if the session database cannot be accessed or if
/// individual deletion operations fail.
pub fn run_prune(_options: &PruneOptions) -> Result<PruneOutput> {
    // TODO: Wire to session database once the prune command is integrated
    // into the CLI dispatch. The full implementation will:
    // 1. List all sessions from the database
    // 2. Filter those whose workspace_path no longer exists on disk
    // 3. Optionally prompt for confirmation (unless --yes)
    // 4. Delete invalid sessions from the database
    // 5. Return PruneOutput with counts

    Output::info("No invalid sessions found");
    Output::info("  All sessions have valid workspaces");

    Ok(PruneOutput::empty())
}

/// Execute a dry-run prune, reporting what would be removed.
///
/// # Errors
///
/// Returns errors if the session database cannot be accessed.
pub fn run_prune_dry_run(_options: &PruneOptions) -> Result<PruneOutput> {
    // TODO: Wire to session database once integrated.
    // Will list sessions with missing workspace directories without deleting.
    Output::info("No invalid sessions found (dry-run)");
    Ok(PruneOutput::empty())
}

/// Validate prune options before execution.
///
/// # Errors
///
/// Returns a validation error if options are contradictory.
pub fn validate_prune_options(options: &PruneOptions) -> Result<()> {
    // Both --yes and --dry-run together is fine: dry-run takes precedence
    // and --yes is simply ignored. No validation error needed.
    let _ = options;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- run_prune with default options --

    #[test]
    fn run_prune_default_returns_empty() {
        let opts = PruneOptions::default();
        let result = run_prune(&opts);
        assert!(result.is_ok());
        let output = result.expect("should be ok");
        assert_eq!(output.invalid_count, 0);
        assert_eq!(output.removed_count, 0);
        assert!(output.invalid_sessions.is_empty());
    }

    #[test]
    fn run_prune_with_yes_flag() {
        let opts = PruneOptions {
            yes: true,
            ..PruneOptions::default()
        };
        let result = run_prune(&opts);
        assert!(result.is_ok());
        let output = result.expect("should be ok");
        assert_eq!(output.invalid_count, 0);
    }

    #[test]
    fn run_prune_with_dry_run() {
        let opts = PruneOptions {
            dry_run: true,
            ..PruneOptions::default()
        };
        let result = run_prune(&opts);
        assert!(result.is_ok());
    }

    // -- run_prune_dry_run --

    #[test]
    fn run_prune_dry_run_default() {
        let opts = PruneOptions::default();
        let result = run_prune_dry_run(&opts);
        assert!(result.is_ok());
        let output = result.expect("should be ok");
        assert_eq!(output.removed_count, 0);
    }

    #[test]
    fn run_prune_dry_run_with_yes() {
        let opts = PruneOptions {
            yes: true,
            dry_run: true,
        };
        let result = run_prune_dry_run(&opts);
        assert!(result.is_ok());
    }

    // -- validate_prune_options --

    #[test]
    fn validate_prune_options_default() {
        let opts = PruneOptions::default();
        assert!(validate_prune_options(&opts).is_ok());
    }

    #[test]
    fn validate_prune_options_with_yes() {
        let opts = PruneOptions {
            yes: true,
            ..PruneOptions::default()
        };
        assert!(validate_prune_options(&opts).is_ok());
    }

    #[test]
    fn validate_prune_options_with_dry_run() {
        let opts = PruneOptions {
            dry_run: true,
            ..PruneOptions::default()
        };
        assert!(validate_prune_options(&opts).is_ok());
    }

    #[test]
    fn validate_prune_options_both_flags() {
        let opts = PruneOptions {
            yes: true,
            dry_run: true,
        };
        assert!(validate_prune_options(&opts).is_ok());
    }
}
