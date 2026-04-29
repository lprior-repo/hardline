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

    let priority = parse_priority(row)?;
    let issue_type = parse_optional_string_field(row, "type")?.and_then(|s| s.parse().ok());
    let labels = parse_comma_separated_field(row, "labels")?;
    let depends_on = parse_comma_separated_field(row, "depends_on")?;
    let blocked_by = parse_comma_separated_field(row, "blocked_by")?;

    let created_at_str: Option<String> = row.try_get("created_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'created_at' error: {e}"))
    })?;
    let created_at = parse_datetime(created_at_str)?;

    let updated_at_str: Option<String> = row.try_get("updated_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'updated_at' error: {e}"))
    })?;
    let updated_at = parse_datetime(updated_at_str)?;

    let closed_at = parse_optional_datetime(row, "closed_at")?;

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

/// Parse the priority field from a row.
fn parse_priority(row: &sqlx::sqlite::SqliteRow) -> Result<Option<Priority>, BeadsError> {
    let priority_str: Option<String> = row.try_get("priority").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'priority' error: {e}"))
    })?;
    Ok(priority_str
        .and_then(|p: String| p.strip_prefix('P').and_then(|n| n.parse::<u32>().ok()))
        .and_then(Priority::from_u32))
}

/// Parse an optional string field from a row.
fn parse_optional_string_field(
    row: &sqlx::sqlite::SqliteRow,
    field: &str,
) -> Result<Option<String>, BeadsError> {
    row.try_get(field)
        .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field '{field}' error: {e}")))
}

/// Parse a comma-separated optional string field from a row.
fn parse_comma_separated_field(
    row: &sqlx::sqlite::SqliteRow,
    field: &str,
) -> Result<Option<Vec<String>>, BeadsError> {
    let value = parse_optional_string_field(row, field)?;
    Ok(value.map(|s| s.split(',').map(String::from).collect()))
}

/// Parse an optional datetime field from a row.
fn parse_optional_datetime(
    row: &sqlx::sqlite::SqliteRow,
    field: &str,
) -> Result<Option<DateTime<Utc>>, BeadsError> {
    let value = parse_optional_string_field(row, field)?;
    value
        .map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| BeadsError::QueryFailed(format!("Invalid {field} datetime: {e}")))
        })
        .transpose()
}
