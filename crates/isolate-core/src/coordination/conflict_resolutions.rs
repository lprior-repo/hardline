#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Conflict resolution audit trail (repository layer).
//!
//! This module provides an append-only audit log for tracking conflict
//! resolution decisions in isolate workspaces. Each record captures:
//!
//! - **Who** resolved the conflict (AI or human)
//! - **What** strategy was used
//! - **Why** the decision was made (optional reason)
//! - **When** the resolution occurred
//!
//! # Design Principles
//!
//! 1. **Append-Only**: No UPDATE or DELETE operations
//! 2. **Transparent**: Full audit trail for debugging
//! 3. **Performant**: Optimized for inserts and queries
//!
//! # Architecture
//!
//! - Infrastructure layer: `sqlx` database operations (this module)
//! - Entity types: `ConflictResolution` in `conflict_resolutions_entities.rs`
//! - Domain errors: `ConflictResolutionError` in `conflict_resolutions_entities.rs`
//!
//! # Example
//!
//! ```rust,no_run
//! use isolate_core::coordination::conflict_resolutions::*;
//! use sqlx::SqlitePool;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize schema (called during db init)
//! let pool = SqlitePool::connect("sqlite:db.sqlite").await?;
//! init_conflict_resolutions_schema(&pool).await?;
//!
//! // Record a conflict resolution
//! let resolution = ConflictResolution {
//!     id: 0, // Auto-generated
//!     timestamp: "2025-02-18T12:34:56Z".to_string(),
//!     session: "my-session".to_string(),
//!     file: "src/main.rs".to_string(),
//!     strategy: "accept_theirs".to_string(),
//!     reason: Some("Incoming changes are more recent".to_string()),
//!     confidence: Some("high".to_string()),
//!     decider: "ai".to_string(),
//! };
//! let id = insert_conflict_resolution(&pool, &resolution).await?;
//!
//! // Query resolutions for a session
//! let resolutions = get_conflict_resolutions(&pool, "my-session").await?;
//! for r in resolutions {
//!     println!("{}: {} by {}", r.file, r.strategy, r.decider);
//! }
//! # Ok(())
//! # }
//! ```

use sqlx::sqlite::SqlitePool;

pub use super::conflict_resolutions_entities::{
    validate_decider, validate_non_empty, validate_timestamp, ConflictResolution,
    ConflictResolutionError,
};
use crate::Result;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SCHEMA INITIALIZATION
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Initialize `conflict_resolutions` table schema.
///
/// This function is called during database initialization to create
/// the `conflict_resolutions` table and its indexes.
///
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `sessions` table exists (dependency)
///
/// ## Postconditions
/// - `conflict_resolutions` table exists
/// - All indexes created
/// - Function is idempotent (safe to call multiple times)
///
/// # Errors
///
/// Returns `Error::DatabaseError` if table creation fails.
///
/// # Example
///
/// ```rust,no_run
/// # use sqlx::SqlitePool;
/// # use isolate_core::coordination::conflict_resolutions::init_conflict_resolutions_schema;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = SqlitePool::connect("sqlite:db.sqlite").await?;
/// init_conflict_resolutions_schema(&pool).await?;
/// # Ok(())
/// # }
/// ```
pub async fn init_conflict_resolutions_schema(pool: &SqlitePool) -> Result<()> {
    // Create conflict_resolutions table
    let create_table = sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS conflict_resolutions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            session TEXT NOT NULL,
            file TEXT NOT NULL,
            strategy TEXT NOT NULL,
            reason TEXT,
            confidence TEXT,
            decider TEXT NOT NULL CHECK(decider IN ('ai', 'human'))
        )
        ",
    )
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::SchemaInitializationError {
        operation: "CREATE TABLE conflict_resolutions".to_string(),
        source: e.to_string(),
        recovery: "Check database permissions and connection".to_string(),
    })?;

    // Create indexes
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_session ON conflict_resolutions(session)",
    )
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::SchemaInitializationError {
        operation: "CREATE INDEX idx_conflict_resolutions_session".to_string(),
        source: e.to_string(),
        recovery: "Check database permissions".to_string(),
    })?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_timestamp ON conflict_resolutions(timestamp)",
    )
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::SchemaInitializationError {
        operation: "CREATE INDEX idx_conflict_resolutions_timestamp".to_string(),
        source: e.to_string(),
        recovery: "Check database permissions".to_string(),
    })?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_decider ON conflict_resolutions(decider)",
    )
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::SchemaInitializationError {
        operation: "CREATE INDEX idx_conflict_resolutions_decider".to_string(),
        source: e.to_string(),
        recovery: "Check database permissions".to_string(),
    })?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_session_timestamp ON conflict_resolutions(session, timestamp)",
    )
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::SchemaInitializationError {
        operation: "CREATE INDEX idx_conflict_resolutions_session_timestamp".to_string(),
        source: e.to_string(),
        recovery: "Check database permissions".to_string(),
    })?;

    // Log success
    tracing::debug!(
        "Initialized conflict_resolutions schema (rows_affected: {})",
        create_table.rows_affected()
    );

    Ok(())
}

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
/// # use isolate_core::coordination::conflict_resolutions::{insert_conflict_resolution, ConflictResolution};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let pool = SqlitePool::connect("sqlite:db.sqlite").await?;
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
    validate_decider(&resolution.decider).map_err(|e| crate::Error::ValidationError {
        message: format!("invalid decider '{}': {e}", resolution.decider),
        field: Some("decider".to_string()),
        value: Some(resolution.decider.clone()),
        constraints: vec!["ai".to_string(), "human".to_string()],
    })?;

    validate_non_empty(&resolution.file, "file").map_err(|e| crate::Error::ValidationError {
        message: format!("empty file path: {e}"),
        field: Some("file".to_string()),
        value: Some(resolution.file.clone()),
        constraints: vec!["non-empty".to_string()],
    })?;

    validate_non_empty(&resolution.strategy, "strategy").map_err(|e| {
        crate::Error::ValidationError {
            message: format!("empty strategy: {e}"),
            field: Some("strategy".to_string()),
            value: Some(resolution.strategy.clone()),
            constraints: vec!["non-empty".to_string()],
        }
    })?;

    validate_non_empty(&resolution.session, "session").map_err(|e| {
        crate::Error::ValidationError {
            message: format!("empty session name: {e}"),
            field: Some("session".to_string()),
            value: Some(resolution.session.clone()),
            constraints: vec!["non-empty".to_string()],
        }
    })?;

    validate_timestamp(&resolution.timestamp).map_err(|e| crate::Error::ValidationError {
        message: format!("invalid timestamp: {e}"),
        field: Some("timestamp".to_string()),
        value: Some(resolution.timestamp.clone()),
        constraints: vec!["ISO 8601".to_string()],
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
/// # use isolate_core::coordination::conflict_resolutions::get_conflict_resolutions;
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
    validate_non_empty(session, "session").map_err(|e| crate::Error::ValidationError {
        message: format!("empty session name: {e}"),
        field: Some("session".to_string()),
        value: Some(session.to_string()),
        constraints: vec!["non-empty".to_string()],
    })?;

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
/// # use isolate_core::coordination::conflict_resolutions::get_resolutions_by_decider;
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
    validate_decider(decider).map_err(|e| crate::Error::ValidationError {
        message: format!("invalid decider '{decider}': {e}"),
        field: Some("decider".to_string()),
        value: Some(decider.to_string()),
        constraints: vec!["ai".to_string(), "human".to_string()],
    })?;

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
/// # use isolate_core::coordination::conflict_resolutions::get_resolutions_by_time_range;
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
    validate_timestamp(start_time).map_err(|e| crate::Error::ValidationError {
        message: format!("invalid start_time: {e}"),
        field: Some("start_time".to_string()),
        value: Some(start_time.to_string()),
        constraints: vec!["ISO 8601".to_string()],
    })?;

    validate_timestamp(end_time).map_err(|e| crate::Error::ValidationError {
        message: format!("invalid end_time: {e}"),
        field: Some("end_time".to_string()),
        value: Some(end_time.to_string()),
        constraints: vec!["ISO 8601".to_string()],
    })?;

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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ERROR CONVERSION
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl From<ConflictResolutionError> for crate::Error {
    fn from(err: ConflictResolutionError) -> Self {
        match err {
            ConflictResolutionError::SchemaInitializationError {
                operation, source, ..
            } => Self::DatabaseError(format!(
                "Schema initialization failed for '{operation}': {source}"
            )),
            ConflictResolutionError::InsertError { file, source, .. } => Self::DatabaseError(
                format!("Failed to insert conflict resolution for '{file}': {source}"),
            ),
            ConflictResolutionError::QueryError {
                operation, source, ..
            } => Self::DatabaseError(format!("Failed to execute query '{operation}': {source}")),
            ConflictResolutionError::InvalidDeciderError { decider, .. } => Self::ValidationError {
                message: format!("invalid decider '{decider}': must be 'ai' or 'human'"),
                field: Some("decider".to_string()),
                value: Some(decider),
                constraints: vec!["ai".to_string(), "human".to_string()],
            },
            ConflictResolutionError::InvalidTimestampError { timestamp, .. } => {
                Self::ValidationError {
                    message: format!("invalid timestamp '{timestamp}': must be ISO 8601 format"),
                    field: Some("timestamp".to_string()),
                    value: Some(timestamp),
                    constraints: vec!["ISO 8601".to_string()],
                }
            }
            ConflictResolutionError::EmptyFieldError { field } => Self::ValidationError {
                message: format!("empty required field '{field}'"),
                field: Some(field),
                value: Some(String::new()),
                constraints: vec!["non-empty".to_string()],
            },
            ConflictResolutionError::InvalidTimeRangeError {
                start_time,
                end_time,
            } => Self::ValidationError {
                message: format!(
                    "invalid time range: start_time '{start_time}' >= end_time '{end_time}'"
                ),
                field: Some("time_range".to_string()),
                value: Some(format!("{start_time}..{end_time}")),
                constraints: vec!["start_time < end_time".to_string()],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests are in conflict_resolutions_tests.rs
    // This module contains only unit tests for pure functions

    #[test]
    fn test_validate_decider_ai() {
        assert_eq!(validate_decider("ai"), Ok(()));
    }

    #[test]
    fn test_validate_decider_human() {
        assert_eq!(validate_decider("human"), Ok(()));
    }

    #[test]
    fn test_validate_decider_invalid() {
        let result = validate_decider("robot");
        assert!(result.is_err());
        match result {
            Err(ConflictResolutionError::InvalidDeciderError { decider, .. }) => {
                assert_eq!(decider, "robot");
            }
            _ => panic!("Expected InvalidDeciderError"),
        }
    }

    #[test]
    fn test_validate_timestamp_valid() {
        assert_eq!(validate_timestamp("2025-02-18T12:34:56Z"), Ok(()));
    }

    #[test]
    fn test_validate_timestamp_empty() {
        let result = validate_timestamp("");
        assert!(result.is_err());
    }
}
