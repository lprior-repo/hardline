//! Lock manager module.
//!
//! Provides exclusive session locking for agent coordination.

pub mod helpers;
pub mod manager;
pub mod manager_lock;
pub mod manager_query;
pub mod manager_unlock;
pub mod tests_basic;
pub mod tests_concurrent;
pub mod tests_session_validation;
pub mod tests_ttl_regression;
pub mod types;

// Re-export types for convenience
pub use types::{
    LockAuditEntry, LockInfo, LockResponse, LockState,
};

// Re-export manager
pub use manager::LockManager;
