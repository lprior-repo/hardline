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
//! This module re-exports from sibling modules in the `coordination` directory:
//! - `conflict_resolutions_schema` - Schema initialization
//! - `conflict_resolutions_insert` - Insert operations
//! - `conflict_resolutions_query` - Query operations
//! - `conflict_resolutions_error` - Error conversion
//! - `conflict_resolutions_entities` (in parent) - Entity types
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
//! use isolate_core::coordination::conflict_resolutions_entities::ConflictResolution;
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

// Re-export schema operations from sibling module
pub use super::conflict_resolutions_schema::init_conflict_resolutions_schema;

// Re-export insert operations from sibling module
pub use super::conflict_resolutions_insert::insert_conflict_resolution;

// Re-export query operations from sibling module
pub use super::conflict_resolutions_query::{
    get_conflict_resolutions, get_resolutions_by_decider, get_resolutions_by_time_range,
};

// Re-export entities for convenience
pub use super::conflict_resolutions_entities::{
    validate_decider, validate_non_empty, validate_timestamp, ConflictResolution,
    ConflictResolutionError,
};
