#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::beads::types::{BeadIssue, BeadsError, IssueStatus, Priority};

/// Parse a datetime string from RFC3339 format.
///
/// # Errors
///
/// Returns `BeadsError::QueryFailed` if the string is missing or invalid.
pub fn parse_datetime(datetime_str: Option<String>) -> Result<DateTime<Utc>, BeadsError> {
    datetime_str
        .ok_or_else(|| BeadsError::QueryFailed("Missing required datetime field".to_string()))
        .and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| BeadsError::QueryFailed(format!("Invalid datetime format '{s}': {e}")))
        })
}

/// Parse a status string into `IssueStatus`.
///
/// # Errors
///
/// Returns `BeadsError::QueryFailed` if the status string is invalid.
pub fn parse_status(status_str: &str) -> Result<IssueStatus, BeadsError> {
    status_str.parse().map_err(|_| {
        BeadsError::QueryFailed(format!("Invalid status value '{status_str}'. Expected one of: open, in_progress, done, cancelled"))
    })
}

/// Parse a single row from the beads database into a `BeadIssue`
///
/// # Errors
///
/// Returns `BeadsError` if any required field is missing or malformed
pub fn parse_bead_row(row: &sqlx::sqlite::SqliteRow) -> Result<BeadIssue, BeadsError> {
    let status_str: String = row
        .try_get("status")
        .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'status' error: {e}")))?;
    let status = parse_status(&status_str)?;

    let priority_str: Option<String> = row.try_get("priority").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'priority' error: {e}"))
    })?;
    let priority = priority_str
        .and_then(|p: String| p.strip_prefix('P').and_then(|n| n.parse::<u32>().ok()))
        .and_then(Priority::from_u32);

    let issue_type_str: Option<String> = row
        .try_get("type")
        .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'type' error: {e}")))?;
    let issue_type = issue_type_str.and_then(|s: String| s.parse().ok());

    let labels_str: Option<String> = row
        .try_get("labels")
        .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'labels' error: {e}")))?;
    let labels =
        labels_str.map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

    let depends_on_str: Option<String> = row.try_get("depends_on").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'depends_on' error: {e}"))
    })?;
    let depends_on =
        depends_on_str.map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

    let blocked_by_str: Option<String> = row.try_get("blocked_by").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'blocked_by' error: {e}"))
    })?;
    let blocked_by =
        blocked_by_str.map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

    // Required datetime fields - fail if missing or invalid
    let created_at_str: Option<String> = row.try_get("created_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'created_at' error: {e}"))
    })?;
    let created_at = parse_datetime(created_at_str)?;

    let updated_at_str: Option<String> = row.try_get("updated_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'updated_at' error: {e}"))
    })?;
    let updated_at = parse_datetime(updated_at_str)?;

    // Optional datetime field
    let closed_at_str: Option<String> = row.try_get("closed_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'closed_at' error: {e}"))
    })?;
    let closed_at: Option<DateTime<Utc>> = closed_at_str
        .map(|s: String| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| BeadsError::QueryFailed(format!("Invalid closed_at datetime: {e}")))
        })
        .transpose()?;

    Ok(BeadIssue {
        id: row
            .try_get("id")
            .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'id' error: {e}")))?,
        title: row.try_get("title").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'title' error: {e}"))
        })?,
        status,
        priority,
        issue_type,
        description: row.try_get("description").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'description' error: {e}"))
        })?,
        labels,
        assignee: row.try_get("assignee").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'assignee' error: {e}"))
        })?,
        parent: row.try_get("parent").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'parent' error: {e}"))
        })?,
        depends_on,
        blocked_by,
        created_at,
        updated_at,
        closed_at,
    })
}
