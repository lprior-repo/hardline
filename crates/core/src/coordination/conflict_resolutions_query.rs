//! Conflict resolution query operations.
//!
//! Provides async functions for querying conflict resolution records
//! from the SQLite database.

use sqlx::sqlite::SqlitePool;

pub use super::conflict_resolutions_entities::{
    validate_decider, validate_non_empty, validate_timestamp, ConflictResolution,
    ConflictResolutionError,
};
use crate::Result;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUERY OPERATIONS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Get all conflict resolutions for a session.
///
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `session` is non-empty
///
/// ## Postconditions
/// - Returns all records for given session
/// - Results ordered by `id` ascending
/// - Returns empty Vec if no matches (not an error)
///
/// # Errors
///
/// - `Error::DatabaseError` if query fails
/// - `Error::Validation` if session is empty
///
/// # Example
///
/// ```rust,no_run
/// # use sqlx::SqlitePool;
/// # use isolate_core::coordination::conflict_resolutions_query::get_conflict_resolutions;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let pool = SqlitePool::connect("sqlite:db.sqlite").await?;
/// let resolutions = get_conflict_resolutions(&pool, "my-session").await?;
/// for resolution in resolutions {
///     println!(
///         "{}: {} resolved by {}",
///         resolution.file, resolution.strategy, resolution.decider
///     );
/// }
/// # Ok(())
/// # }
/// ```
pub async fn get_conflict_resolutions(
    pool: &SqlitePool,
    session: &str,
) -> Result<Vec<ConflictResolution>> {
    validate_non_empty(session, "session").map_err(|e| crate::Error::validation_field_error(
        "session",
        format!("empty session name: {e}"),
        Some(session.to_string()),
    ))?;

    let resolutions = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE session = ? ORDER BY id",
    )
    .bind(session)
    .fetch_all(pool)
    .await
    .map_err(|e| ConflictResolutionError::QueryError {
        operation: "get_conflict_resolutions".to_string(),
        source: e.to_string(),
        recovery: "Check database connection and session name".to_string(),
    })?;

    tracing::debug!(
        "Retrieved {} conflict resolutions for session '{}'",
        resolutions.len(),
        session
    );

    Ok(resolutions)
}

/// Get conflict resolutions by decider type.
///
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `decider` is "ai" or "human"
///
/// ## Postconditions
/// - Returns all records with matching decider
/// - Results ordered by `id` ascending
/// - Returns empty Vec if no matches (not an error)
///
/// # Errors
///
/// - `Error::DatabaseError` if query fails
/// - `Error::Validation` if decider is invalid
///
/// # Example
///
/// ```rust,no_run
/// # use sqlx::SqlitePool;
/// # use isolate_core::coordination::conflict_resolutions_query::get_resolutions_by_decider;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let pool = SqlitePool::connect("sqlite:db.sqlite").await?;
/// let ai_resolutions = get_resolutions_by_decider(&pool, "ai").await?;
/// println!("AI resolved {} conflicts", ai_resolutions.len());
/// # Ok(())
/// # }
/// ```
pub async fn get_resolutions_by_decider(
    pool: &SqlitePool,
    decider: &str,
) -> Result<Vec<ConflictResolution>> {
    validate_decider(decider).map_err(|e| crate::Error::validation_field_error(
        "decider",
        format!("invalid decider '{decider}': {e}"),
        Some(decider.to_string()),
    ))?;

    let resolutions = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE decider = ? ORDER BY id",
    )
    .bind(decider)
    .fetch_all(pool)
    .await
    .map_err(|e| ConflictResolutionError::QueryError {
        operation: "get_resolutions_by_decider".to_string(),
        source: e.to_string(),
        recovery: "Check database connection".to_string(),
    })?;

    tracing::debug!(
        "Retrieved {} conflict resolutions for decider '{}'",
        resolutions.len(),
        decider
    );

    Ok(resolutions)
}

/// Get conflict resolutions within time range.
///
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `start_time` and `end_time` are valid ISO 8601 timestamps
/// - `start_time` < `end_time`
///
/// ## Postconditions
/// - Returns all records with timestamps in [`start_time`, `end_time`)
/// - Results ordered by `timestamp` ascending
/// - Returns empty Vec if no matches (not an error)
///
/// # Errors
///
/// - `Error::DatabaseError` if query fails
/// - `Error::Validation` if timestamps are invalid or range invalid
///
/// # Example
///
/// ```rust,no_run
/// # use sqlx::SqlitePool;
/// # use isolate_core::coordination::conflict_resolutions_query::get_resolutions_by_time_range;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let pool = SqlitePool::connect("sqlite:db.sqlite").await?;
/// let resolutions =
///     get_resolutions_by_time_range(&pool, "2025-02-18T00:00:00Z", "2025-02-18T23:59:59Z")
///         .await?;
/// println!("Resolved {} conflicts today", resolutions.len());
/// # Ok(())
/// # }
/// ```
pub async fn get_resolutions_by_time_range(
    pool: &SqlitePool,
    start_time: &str,
    end_time: &str,
) -> Result<Vec<ConflictResolution>> {
    validate_timestamp(start_time).map_err(|e| crate::Error::validation_field_error(
        "start_time",
        format!("invalid start_time: {e}"),
        Some(start_time.to_string()),
    ))?;

    validate_timestamp(end_time).map_err(|e| crate::Error::validation_field_error(
        "end_time",
        format!("invalid end_time: {e}"),
        Some(end_time.to_string()),
    ))?;

    // Basic validation: start_time should be before end_time
    // (This is a simple string comparison; for full ISO 8601 validation, use chrono)
    if start_time >= end_time {
        return Err(ConflictResolutionError::InvalidTimeRangeError {
            start_time: start_time.to_string(),
            end_time: end_time.to_string(),
        }
        .into());
    }

    let resolutions = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE timestamp >= ? AND timestamp < ? ORDER BY timestamp",
    )
    .bind(start_time)
    .bind(end_time)
    .fetch_all(pool)
    .await
    .map_err(|e| ConflictResolutionError::QueryError {
        operation: "get_resolutions_by_time_range".to_string(),
        source: e.to_string(),
        recovery: "Check database connection and timestamp format".to_string(),
    })?;

    tracing::debug!(
        "Retrieved {} conflict resolutions between {} and {}",
        resolutions.len(),
        start_time,
        end_time
    );

    Ok(resolutions)
}
