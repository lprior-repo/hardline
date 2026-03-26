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
//! This module was split into smaller submodules to improve maintainability:
//! - [`conflict_resolutions_schema`] - Schema initialization
//! - [`conflict_resolutions_insert`] - Insert operations
//! - [`conflict_resolutions_queries`] - Query operations
//! - [`conflict_resolutions_error_convert`] - Error conversion
//! - [`conflict_resolutions_entities`] - Entity types and errors
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
//! let record = ConflictResolutionRecord::new(
//!     "workspace-123",
//!     ConflictResolutionStrategy::Auto,
//!     Some("Auto-resolved based on strategy".to_string()),
//! );
//! insert_conflict_resolution(&pool, &record).await?;
//!
//! let resolutions = query_conflict_resolutions(&pool, "workspace-123").await?;
//! println!("Found {} resolutions", resolutions.len());
//! # Ok(())
//! # }
//! ```

pub use super::conflict_resolutions_insert::insert_conflict_resolution;
pub use super::conflict_resolutions_query::{
    get_conflict_resolutions, get_resolutions_by_decider, get_resolutions_by_time_range,
};
pub use super::conflict_resolutions_schema::init_conflict_resolutions_schema;
