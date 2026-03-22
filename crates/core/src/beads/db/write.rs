#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

use sqlx::SqlitePool;

use crate::beads::types::{BeadIssue, BeadsError, IssueStatus, Priority};
use crate::beads::db::validation::{serialize_optional_vec, validate_bead_for_insert};

/// Insert a bead issue into the database.
///
/// # Errors
///
/// Returns `BeadsError` if:
/// - Validation fails (empty ID or title)
/// - The insert operation fails
/// - A bead with the same ID already exists (`DuplicateId`)
pub async fn insert_bead(pool: &SqlitePool, issue: &BeadIssue) -> Result<(), BeadsError> {
    // Validate input
    validate_bead_for_insert(issue)?;

    // Serialize optional fields
    let priority_str = issue
        .priority
        .map(|priority: Priority| format!("P{}", priority.to_u32()));
    let issue_type_str = issue
        .issue_type
        .as_ref()
        .map(std::string::ToString::to_string);
    let labels_str = serialize_optional_vec(issue.labels.as_ref());
    let depends_on_str = serialize_optional_vec(issue.depends_on.as_ref());
    let blocked_by_str = serialize_optional_vec(issue.blocked_by.as_ref());
    let created_at_str = issue.created_at.to_rfc3339();
    let updated_at_str = issue.updated_at.to_rfc3339();
    let closed_at_str: Option<String> = issue.closed_at.map(|dt| dt.to_rfc3339());

    // Execute insert
    let result = sqlx::query(
        "INSERT INTO issues (id, title, status, priority, type, description, labels,
                             assignee, parent, depends_on, blocked_by,
                             created_at, updated_at, closed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
    )
    .bind(&issue.id)
    .bind(&issue.title)
    .bind(issue.status.to_string())
    .bind(priority_str)
    .bind(issue_type_str)
    .bind(&issue.description)
    .bind(labels_str)
    .bind(&issue.assignee)
    .bind(&issue.parent)
    .bind(depends_on_str)
    .bind(blocked_by_str)
    .bind(&created_at_str)
    .bind(&updated_at_str)
    .bind(closed_at_str)
    .execute(pool)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("UNIQUE constraint failed") || error_msg.contains("PRIMARY KEY") {
                Err(BeadsError::DuplicateId(issue.id.clone()))
            } else {
                Err(BeadsError::InsertFailed(format!(
                    "Failed to insert bead '{}': {e}",
                    issue.id
                )))
            }
        }
    }
}

/// Delete a bead issue from the database.
///
/// # Errors
///
/// Returns `BeadsError` if:
/// - The bead with the given ID does not exist (`NotFound`)
/// - The delete operation fails (`DatabaseError`)
pub async fn delete_bead(pool: &SqlitePool, id: &str) -> Result<(), BeadsError> {
    let result = sqlx::query("DELETE FROM issues WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to delete bead '{id}': {e}")))?;

    if result.rows_affected() == 0 {
        return Err(BeadsError::NotFound(id.to_string()));
    }

    Ok(())
}

/// Update an existing bead issue in the database.
///
/// # Errors
///
/// Returns `BeadsError` if:
/// - Validation fails (empty title)
/// - The bead with the given ID does not exist (`NotFound`)
/// - The update operation fails
pub async fn update_bead(
    pool: &SqlitePool,
    id: &str,
    issue: &BeadIssue,
) -> Result<BeadIssue, BeadsError> {
    // Validate input
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

    // Serialize optional fields
    let priority_str = issue
        .priority
        .map(|priority: Priority| format!("P{}", priority.to_u32()));
    let issue_type_str = issue
        .issue_type
        .as_ref()
        .map(std::string::ToString::to_string);
    let labels_str = serialize_optional_vec(issue.labels.as_ref());
    let depends_on_str = serialize_optional_vec(issue.depends_on.as_ref());
    let blocked_by_str = serialize_optional_vec(issue.blocked_by.as_ref());
    let updated_at_str = issue.updated_at.to_rfc3339();
    let closed_at_str: Option<String> = issue.closed_at.map(|dt| dt.to_rfc3339());

    // Execute update
    let result = sqlx::query(
        "UPDATE issues SET
            title = ?1,
            status = ?2,
            priority = ?3,
            type = ?4,
            description = ?5,
            labels = ?6,
            assignee = ?7,
            parent = ?8,
            depends_on = ?9,
            blocked_by = ?10,
            updated_at = ?11,
            closed_at = ?12
         WHERE id = ?13",
    )
    .bind(&issue.title)
    .bind(issue.status.to_string())
    .bind(priority_str)
    .bind(issue_type_str)
    .bind(&issue.description)
    .bind(labels_str)
    .bind(&issue.assignee)
    .bind(&issue.parent)
    .bind(depends_on_str)
    .bind(blocked_by_str)
    .bind(&updated_at_str)
    .bind(closed_at_str)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| BeadsError::DatabaseError(format!("Failed to update bead '{id}': {e}")))?;

    // Check if row was updated
    if result.rows_affected() == 0 {
        return Err(BeadsError::NotFound(id.to_string()));
    }

    // Return the updated issue
    Ok(issue.clone())
}
