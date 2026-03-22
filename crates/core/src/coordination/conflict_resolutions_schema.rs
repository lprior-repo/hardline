#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Conflict resolution schema initialization.
//!
//! This module provides the `init_conflict_resolutions_schema` function
//! for creating the `conflict_resolutions` table and its indexes.
//!
//! # Design Principles
//!
//! 1. **Append-Only**: No UPDATE or DELETE operations
//! 2. **Transparent**: Full audit trail for debugging
//! 3. **Performant**: Optimized for inserts and queries
//!
//! # Architecture
//!
//! - Infrastructure layer: `sqlx` database operations
//! - Entity types: `ConflictResolution` in `conflict_resolutions_entities.rs`
//! - Domain errors: `ConflictResolutionError` in `conflict_resolutions_entities.rs`

use sqlx::sqlite::SqlitePool;

pub use super::conflict_resolutions_entities::{
    ConflictResolution, ConflictResolutionError,
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
