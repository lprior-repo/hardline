//! Data types for lock command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

/// Lock command variants (CLI subcommand representation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockCommand {
    /// Acquire a lock on a session
    Acquire {
        /// Session name to lock
        session: String,
        /// Agent ID acquiring the lock
        agent: String,
        /// Time-To-Live in seconds (optional)
        ttl: Option<u64>,
    },
    /// Release a lock on a session
    Release {
        /// Session name to unlock
        session: String,
        /// Agent ID releasing the lock
        agent: String,
    },
    /// Send a heartbeat to extend lock TTL
    Heartbeat {
        /// Session name
        session: String,
        /// Agent ID sending the heartbeat
        agent: String,
    },
    /// Show the status of a lock
    Status {
        /// Session name to check
        session: String,
    },
    /// List all active locks
    List,
    /// Force unlock a session (admin operation)
    ForceUnlock {
        /// Session name to force unlock
        session: String,
        /// Admin agent ID (for audit)
        admin: String,
    },
    /// Show detailed lock metadata
    Metadata {
        /// Session name
        session: String,
    },
}

/// Agent ID newtype - validates at construction time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentId(String);

impl AgentId {
    /// Create a new AgentId with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent ID is empty or whitespace-only.
    pub fn new(id: impl Into<String>) -> Result<Self, scp_core::error::Error> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(scp_core::error::Error::validation_error(
                "Agent ID cannot be empty",
            ));
        }
        Ok(Self(id))
    }

    /// Access the inner string as a str slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lock status for display output (serializable for JSON).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockStatus {
    /// Lock is held by an agent
    Locked,
    /// No lock is held
    Unlocked,
}

/// Lock output for CLI display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockOutput {
    /// Status of the lock operation
    pub status: LockStatus,
    /// Session name
    pub session: String,
    /// Agent holding the lock (if locked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Expiration timestamp (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Original TTL in seconds (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u64>,
    /// Remaining TTL in seconds (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_ttl: Option<u64>,
    /// Error message (if operation failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Lock metadata for detailed view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockMetadata {
    /// Session name
    pub session: String,
    /// Agent ID holding the lock
    pub agent_id: String,
    /// When the lock was acquired
    pub acquired_at: String,
    /// Original TTL in seconds
    pub ttl: u64,
    /// When the lock expires
    pub expires_at: String,
    /// Number of heartbeats sent
    pub heartbeat_count: u64,
    /// Whether the lock is expired
    pub is_expired: bool,
}

/// Lock entry for list output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEntry {
    /// Session name
    pub session: String,
    /// Agent ID holding the lock
    pub agent: String,
    /// When the lock expires
    pub expires_at: String,
    /// Whether the lock is expired
    pub is_expired: bool,
}

/// List output containing all active locks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockListOutput {
    /// Number of active locks
    pub count: usize,
    /// List of lock entries
    pub locks: Vec<LockEntry>,
    /// Whether there are any locks
    pub has_locks: bool,
}

/// Heartbeat result output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatOutput {
    /// Session name
    pub session: String,
    /// New expiration time
    pub expires_at: String,
    /// Whether the heartbeat succeeded
    pub success: bool,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Force unlock result output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceUnlockOutput {
    /// Session name
    pub session: String,
    /// Admin agent who performed the unlock
    pub admin: String,
    /// Whether the unlock succeeded
    pub success: bool,
    /// Previous holder (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_holder: Option<String>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
