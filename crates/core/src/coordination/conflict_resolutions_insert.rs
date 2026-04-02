//! Conflict resolution insert operations.
//!
//! Provides async functions for inserting conflict resolution records
//! into the SQLite database.

use sqlx::sqlite::SqlitePool;

pub use super::conflict_resolutions_entities::{
    validate_decider, validate_non_empty, validate_timestamp, ConflictResolution,
    ConflictResolutionError,
};
use crate::Result;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// INSERT OPERATIONS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Insert a conflict resolution record.
///
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `resolution.session` is valid (may check existence)
/// - `resolution.decider` is "ai" or "human"
/// - `resolution.timestamp` is valid ISO 8601
/// - `resolution.file` and `resolution.strategy` are non-empty
///
/// ## Postconditions
/// - Record inserted with auto-generated ID
/// - Returned ID matches inserted record
/// - `SELECT * FROM conflict_resolutions WHERE id = ?` returns record
///
/// # Errors
///
/// - `Error::DatabaseError` if insert fails (constraint violation, I/O error)
/// - `Error::Validation` if validation fails
///
/// # Example
///
/// ```rust,no_run
/// # use sqlx::SqlitePool;
/// # use scp_core::coordination::conflict_resolutions_insert::insert_conflict_resolution;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let pool = SqlitePool::connect("sqlite:db.sqlite").await?;
/// use scp_core::coordination::conflict_resolutions_entities::ConflictResolution;
/// let resolution = ConflictResolution {
///     id: 0,
///     timestamp: "2025-02-18T12:34:56Z".to_string(),
///     session: "my-session".to_string(),
///     file: "src/main.rs".to_string(),
///     strategy: "accept_theirs".to_string(),
///     reason: Some("Automatic resolution".to_string()),
///     confidence: Some("high".to_string()),
///     decider: "ai".to_string(),
/// };
/// let id = insert_conflict_resolution(&pool, &resolution).await?;
/// assert!(id > 0);
/// # Ok(())
/// # }
/// ```
pub async fn insert_conflict_resolution(
    pool: &SqlitePool,
    resolution: &ConflictResolution,
) -> Result<i64> {
    // Validate inputs
    validate_decider(&resolution.decider).map_err(|e| {
        crate::Error::validation_field_error(
            "decider",
            format!("invalid decider '{}': {e}", resolution.decider),
            Some(resolution.decider.clone()),
        )
    })?;

    validate_non_empty(&resolution.file, "file").map_err(|e| {
        crate::Error::validation_field_error(
            "file",
            format!("empty file path: {e}"),
            Some(resolution.file.clone()),
        )
    })?;

    validate_non_empty(&resolution.strategy, "strategy").map_err(|e| {
        crate::Error::validation_field_error(
            "strategy",
            format!("empty strategy: {e}"),
            Some(resolution.strategy.clone()),
        )
    })?;

    validate_non_empty(&resolution.session, "session").map_err(|e| {
        crate::Error::validation_field_error(
            "session",
            format!("empty session name: {e}"),
            Some(resolution.session.clone()),
        )
    })?;

    validate_timestamp(&resolution.timestamp).map_err(|e| {
        crate::Error::validation_field_error(
            "timestamp",
            format!("invalid timestamp: {e}"),
            Some(resolution.timestamp.clone()),
        )
    })?;

    // Insert record
    let result = sqlx::query(
        r"
        INSERT INTO conflict_resolutions (
            timestamp, session, file, strategy, reason, confidence, decider
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        ",
    )
    .bind(&resolution.timestamp)
    .bind(&resolution.session)
    .bind(&resolution.file)
    .bind(&resolution.strategy)
    .bind(&resolution.reason)
    .bind(&resolution.confidence)
    .bind(&resolution.decider)
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::InsertError {
        file: resolution.file.clone(),
        source: e.to_string(),
        constraint: e
            .as_database_error()
            .and_then(sqlx::error::DatabaseError::code)
            .map(|c| c.to_string()),
        recovery: "Ensure decider is 'ai' or 'human' and all required fields are non-empty"
            .to_string(),
    })?;

    let id = result.last_insert_rowid();

    // Log success
    tracing::debug!(
        "Inserted conflict resolution for file '{}' in session '{}' (id: {id}, decider: {})",
        resolution.file,
        resolution.session,
        resolution.decider
    );

    Ok(id)
}
