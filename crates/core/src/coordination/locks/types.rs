//! Session lock manager for agent coordination.
//!
//! Provides exclusive locking so that only one agent operates on a session at a time.
//! Locks have a TTL and can be extended via heartbeat.
//!
//! # Session Existence Validation
//!
//! The lock manager validates that a session exists in the sessions table before
//! acquiring a lock. This prevents orphaned locks from being created for
//! non-existent sessions.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use chrono::{DateTime, Utc};

/// Information about an active lock.
#[derive(Debug, Clone)]
pub struct LockInfo {
    /// The session that is locked.
    pub session: String,
    /// The agent holding the lock.
    pub agent_id: String,
    /// When the lock was acquired.
    pub acquired_at: DateTime<Utc>,
    /// When the lock expires.
    pub expires_at: DateTime<Utc>,
}

/// Response returned when a lock is successfully acquired.
#[derive(Debug, Clone)]
pub struct LockResponse {
    /// Unique lock identifier.
    pub lock_id: String,
    /// The session that was locked.
    pub session: String,
    /// The agent that acquired the lock.
    pub agent_id: String,
    /// When the lock expires.
    pub expires_at: DateTime<Utc>,
}

/// Audit log entry for lock operations.
#[derive(Debug, Clone)]
pub struct LockAuditEntry {
    /// The session that was operated on.
    pub session: String,
    /// The agent that performed the operation.
    pub agent_id: String,
    /// The operation performed (lock, unlock, `double_unlock_warning`).
    pub operation: String,
    /// When the operation occurred.
    pub timestamp: DateTime<Utc>,
}

/// Current lock state for a session.
#[derive(Debug, Clone)]
pub struct LockState {
    /// The session name.
    pub session: String,
    /// The current lock holder (if any).
    pub holder: Option<String>,
    /// When the lock expires (if locked).
    pub expires_at: Option<DateTime<Utc>>,
}
