//! Heartbeat configuration and messages

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{AgentRegistryError, AgentStatus, BeadId, WorkspaceId};

// ============================================================================
// HEARTBEAT CONFIGURATION
// ============================================================================

/// Configuration for heartbeat system.
#[derive(Debug, Clone, Copy)]
pub struct HeartbeatConfig {
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval_secs: 30,
            timeout_secs: 90,
            max_retries: 0,
        }
    }
}

impl HeartbeatConfig {
    /// Calculate the cutoff time for stale detection
    #[must_use]
    pub fn stale_cutoff(&self) -> DateTime<Utc> {
        Utc::now()
            - chrono::TimeDelta::try_seconds(self.timeout_secs as i64)
                .unwrap_or(chrono::TimeDelta::zero())
    }

    /// Create a non-default configuration
    #[must_use]
    pub fn new(interval_secs: u64, timeout_secs: u64, max_retries: u32) -> Self {
        Self {
            interval_secs,
            timeout_secs,
            max_retries,
        }
    }
}

// ============================================================================
// HEARTBEAT MESSAGE
// ============================================================================

/// Heartbeat message from an agent.
///
/// Agents send heartbeat messages periodically to indicate they're alive.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub agent_id: super::AgentId,
    pub timestamp: DateTime<Utc>,
    pub status: AgentStatus,
    pub workspace_id: Option<WorkspaceId>,
    pub bead_id: Option<BeadId>,
    pub load_average: Option<f64>,
}

impl Heartbeat {
    /// Create a new heartbeat
    #[must_use]
    pub fn new(
        agent_id: super::AgentId,
        status: AgentStatus,
        workspace_id: Option<WorkspaceId>,
        bead_id: Option<BeadId>,
    ) -> Self {
        Self {
            agent_id,
            timestamp: Utc::now(),
            status,
            workspace_id,
            bead_id,
            load_average: None,
        }
    }

    /// Create with load average
    #[must_use]
    pub fn with_load_average(mut self, load_average: f64) -> Self {
        self.load_average = Some(load_average);
        self
    }
}
