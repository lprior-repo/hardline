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

// Re-export data types
pub use crate::session_sync_data::{
    PreconditionCheck, SessionSyncInput, SessionSyncResult, WorkspaceCleanStatus,
};

// Re-export error types
pub use crate::session_sync_errors::SyncError;

// Re-export calculation functions
pub use crate::session_sync_calculations::{
    create_sync_result, determine_workspace_status, has_conflicts_in_output,
    parse_rebase_output, validate_sync_preconditions,
};

// Tests
#[cfg(test)]
mod tests {
    include!("session_sync_tests.rs");
}
