//! Lock manager module.
//!
//! Provides exclusive session locking for agent coordination.

pub mod helpers;
pub mod manager;
pub mod manager_lock;
pub mod manager_query;
pub mod manager_unlock;
#[cfg(test)]
mod tests_basic;
#[cfg(test)]
mod tests_concurrent;
#[cfg(test)]
mod tests_session_validation;
#[cfg(test)]
mod tests_ttl_regression;
pub mod types;

// Re-export types for convenience
pub use types::{
    LockAuditEntry, LockInfo, LockResponse, LockState,
};

// Re-export manager
pub use manager::LockManager;
