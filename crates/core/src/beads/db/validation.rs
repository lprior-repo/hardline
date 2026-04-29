#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

use crate::beads::types::{BeadIssue, BeadsError, IssueStatus};

/// Validate a bead issue for insertion.
///
/// # Errors
///
/// Returns `BeadsError::ValidationFailed` if:
/// - ID is empty
/// - Title is empty
pub fn validate_bead_for_insert(issue: &BeadIssue) -> Result<(), BeadsError> {
    if issue.id.is_empty() {
        return Err(BeadsError::ValidationFailed(
            "ID cannot be empty".to_string(),
        ));
    }
    if issue.title.is_empty() {
        return Err(BeadsError::ValidationFailed(
            "Title cannot be empty".to_string(),
        ));
    }
    // Enforce invariant: status='closed' => closed_at IS NOT NULL
    // This matches the CHECK constraint in the database schema
    if issue.status == IssueStatus::Closed && issue.closed_at.is_none() {
        return Err(BeadsError::ValidationFailed(
            "closed_at must be set when status is 'closed'".to_string(),
        ));
    }
    Ok(())
}

/// Serialize optional vector as comma-separated string.
pub fn serialize_optional_vec(v: Option<&Vec<String>>) -> Option<String> {
    v.and_then(|items| {
        if items.is_empty() {
            None
        } else {
            Some(items.join(","))
        }
    })
}
