//! Action functions for the prune command handler (Tier 3).
//!
//! I/O operations that discover and remove invalid session records.
//! Currently a honest stub: returns an error indicating the command
//! is not yet fully implemented, rather than lying about success.

use scp_core::{Error, Result};

use super::data::{PruneMode, PruneOptions, PruneOutput};

/// Execute the prune command with the given options.
///
/// Dispatches based on `PruneMode`: dry-run shows what would be pruned,
/// confirm skips prompts, interactive asks the user.
///
/// # Errors
///
/// Returns `Error::invalid_state` until the prune command is fully wired
/// to the session database.
pub fn run_prune(options: &PruneOptions) -> Result<PruneOutput> {
    match options.mode {
        PruneMode::DryRun => run_prune_dry_run(options),
        PruneMode::Confirm | PruneMode::Interactive => {
            // TODO: Wire to session database once the prune command is
            // integrated into the CLI dispatch. The full implementation
            // will scan .scp/ for orphaned worktrees and stale lock
            // files, then optionally prompt for confirmation.
            Err(Error::invalid_state(
                "Prune command is not yet implemented. \
                 Session database integration is required to scan for \
                 orphaned worktrees and stale lock files.",
            ))
        }
    }
}

/// Execute a dry-run prune, reporting what would be removed.
///
/// # Errors
///
/// Returns `Error::invalid_state` until the prune command is fully wired
/// to the session database.
fn run_prune_dry_run(_options: &PruneOptions) -> Result<PruneOutput> {
    // TODO: Wire to session database once integrated.
    // Will list sessions with missing workspace directories without deleting.
    Err(Error::invalid_state(
        "Prune dry-run is not yet implemented. \
         Session database integration is required to scan for \
         orphaned worktrees and stale lock files.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- run_prune returns not-yet-implemented error --

    #[test]
    fn run_prune_interactive_returns_not_implemented() {
        let opts = PruneOptions {
            mode: PruneMode::Interactive,
        };
        let result = run_prune(&opts);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not yet implemented"),
            "Expected not-yet-implemented message, got: {err_msg}"
        );
    }

    #[test]
    fn run_prune_confirm_returns_not_implemented() {
        let opts = PruneOptions {
            mode: PruneMode::Confirm,
        };
        let result = run_prune(&opts);
        assert!(result.is_err());
    }

    #[test]
    fn run_prune_dry_run_mode_returns_not_implemented() {
        let opts = PruneOptions {
            mode: PruneMode::DryRun,
        };
        let result = run_prune(&opts);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not yet implemented"),
            "Expected not-yet-implemented message, got: {err_msg}"
        );
    }

    #[test]
    fn error_message_mentions_session_database() {
        let opts = PruneOptions {
            mode: PruneMode::Interactive,
        };
        let result = run_prune(&opts);
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Session database"),
            "Error should mention session database requirement, got: {err_msg}"
        );
    }
}
