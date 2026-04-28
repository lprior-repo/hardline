//! Helper functions for hint generation
//!
//! Utility functions used across hint generation

use super::types::Hint;
use crate::types::BeadsSummary;

/// Extract session name from error message
///
/// Looks for a quoted string in the error message.
pub fn extract_session_name(error_msg: &str) -> Option<&str> {
    error_msg.split('\'').nth(1)
}

/// Generate hints for beads status
///
/// # Returns
///
/// Returns a vector of hints. The result should be used
/// as this performs analysis and generates contextual help.
#[must_use]
pub fn hints_for_beads(session_name: &str, beads: &BeadsSummary) -> Vec<Hint> {
    let mut hints = Vec::new();

    if beads.has_blockers() {
        hints.push(
            Hint::warning(format!(
                "Session '{}' has {} blocked issue(s)",
                session_name, beads.blocked
            ))
            .with_command("bv")
            .with_rationale("Resolve blockers to make progress")
            .with_context(serde_json::json!({
                "session": session_name,
                "blocked_count": beads.blocked,
            })),
        );
    }

    if beads.active() > 5 {
        hints.push(
            Hint::tip(format!(
                "Session '{}' has {} active issues - consider focusing on fewer tasks",
                session_name,
                beads.active()
            ))
            .with_rationale("Limiting work in progress improves focus"),
        );
    }

    if beads.total() == 0 {
        hints.push(
            Hint::info(format!("Session '{}' has no beads issues", session_name))
                .with_command("br new")
                .with_rationale("Track your work with beads for better organization"),
        );
    }

    hints
}
