//! Session sync domain logic - Data → Calculations → Actions
//!
//! This module implements the sync operation for sessions with:
//! - Preconditions: session exists, status Active/Failed, workspace clean (or allowed)
//! - Postconditions: rebased onto main, conflicts reported, status transitions
//! - Errors: `SessionNotFound`, `InvalidSessionStatus`, `DirtyWorkspace`, `Conflict`,
//!   `RebaseFailure`
//!
//! # Architecture
//!
//! - **Data**: `SessionSyncInput`, `SessionSyncResult`, `SyncError` types
//! - **Calculations**: Pure validation and state transition functions
//! - **Actions**: Async JJ operations wrapped with proper error handling

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

// Submodules
mod data;
mod error;
mod validation;

// Re-exports - data types
pub use data::{
    PreconditionCheck, SessionSyncInput, SessionSyncResult, WorkspaceCleanStatus,
};

// Re-exports - error types
pub use error::SyncError;

// Re-exports - validation functions
pub use validation::{
    create_sync_result, determine_workspace_status, has_conflicts_in_output,
    parse_rebase_output, validate_sync_preconditions,
};
