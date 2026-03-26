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
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::complexity)]

use chrono::{DateTime, Utc};

/// TTL value object with validation.
///
/// Represents a valid TTL in seconds (0 means use default, max is 86400 = 24 hours).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ttl {
    seconds: u64,
}

impl Ttl {
    /// Maximum allowed TTL in seconds (24 hours).
    pub const MAX_SECS: u64 = 86400;

    /// Minimum allowed TTL in seconds (0 = use default).
    pub const MIN_SECS: u64 = 0;

    /// Create a new TTL value, returning None if invalid.
    ///
    /// Returns `None` if `seconds > 86400` or `seconds == u64::MAX` (overflow check).
    #[must_use]
    pub fn new(seconds: u64) -> Option<Self> {
        if seconds > Self::MAX_SECS || seconds == u64::MAX {
            None
        } else {
            Some(Self { seconds })
        }
    }

    /// Get the TTL seconds value.
    #[must_use]
    pub const fn seconds(&self) -> u64 {
        self.seconds
    }

    /// Check if this TTL uses the default (seconds == 0).
    #[must_use]
    pub const fn is_default(&self) -> bool {
        self.seconds == 0
    }
}

/// Type of lock operation for audit logging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockOperation {
    /// Lock acquired
    Lock,

    /// Lock released by holder
    Unlock,

    /// Lock extended via heartbeat
    Heartbeat,

    /// Double-unlock warning (same agent unlocked twice)
    DoubleUnlockWarning,
}

impl LockOperation {
    /// Convert to string representation for audit logging.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            LockOperation::Lock => "lock",
            LockOperation::Unlock => "unlock",
            LockOperation::Heartbeat => "heartbeat",
            LockOperation::DoubleUnlockWarning => "double_unlock_warning",
        }
    }
}

/// Information about an active lock.
#[derive(Debug, Clone)]
pub struct LockInfo {
    /// Session name
    pub session: String,

    /// Agent ID holding the lock
    pub agent_id: String,

    /// Lock identifier
    pub lock_id: String,

    /// When the lock was acquired
    pub acquired_at: DateTime<Utc>,

    /// When the lock expires
    pub expires_at: DateTime<Utc>,
}

/// Response returned when a lock is successfully acquired.
#[derive(Debug, Clone)]
pub struct LockResponse {
    /// Generated unique lock identifier
    /// Format: "lock-{session}-{timestamp_nanos}"
    pub lock_id: String,

    /// Session name this lock protects
    pub session: String,

    /// Agent ID that holds this lock
    pub agent_id: String,

    /// When the lock was acquired
    pub acquired_at: DateTime<Utc>,

    /// When the lock expires (acquired_at + TTL)
    pub expires_at: DateTime<Utc>,
}

/// Audit log entry for lock operations.
#[derive(Debug, Clone)]
pub struct LockAuditEntry {
    /// Session name
    pub session: String,

    /// Agent ID that performed the operation
    pub agent_id: String,

    /// Type of operation performed
    pub operation: LockOperation,

    /// When the operation occurred
    pub timestamp: DateTime<Utc>,
}

/// Current lock state for a session.
#[derive(Debug, Clone)]
pub struct LockState {
    /// Session name
    pub session: String,

    /// Agent ID holding the lock, None if no active lock
    pub holder: Option<String>,

    /// Lock expiration time, None if no active lock
    pub expires_at: Option<DateTime<Utc>>,
}
