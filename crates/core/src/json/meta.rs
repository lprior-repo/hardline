//! Response metadata structures

use serde::{Deserialize, Serialize};

/// Response metadata for debugging and tracing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseMeta {
    /// Command that generated this response
    pub command: String,
    /// Timestamp of response generation (ISO 8601)
    pub timestamp: String,
    /// Duration of command execution in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Whether this was a dry-run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
    /// Whether the operation is reversible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reversible: Option<bool>,
    /// Command to undo this operation (if reversible)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_command: Option<String>,
    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Agent ID if executed by an agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

impl ResponseMeta {
    /// Create new metadata for a command
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms: None,
            dry_run: None,
            reversible: None,
            undo_command: None,
            request_id: None,
            agent_id: None,
        }
    }

    /// Set duration
    #[must_use]
    pub const fn with_duration(self, ms: u64) -> Self {
        Self {
            command: self.command,
            timestamp: self.timestamp,
            duration_ms: Some(ms),
            dry_run: self.dry_run,
            reversible: self.reversible,
            undo_command: self.undo_command,
            request_id: self.request_id,
            agent_id: self.agent_id,
        }
    }

    /// Mark as dry run
    #[must_use]
    pub const fn as_dry_run(self) -> Self {
        Self {
            command: self.command,
            timestamp: self.timestamp,
            duration_ms: self.duration_ms,
            dry_run: Some(true),
            reversible: self.reversible,
            undo_command: self.undo_command,
            request_id: self.request_id,
            agent_id: self.agent_id,
        }
    }

    /// Mark as reversible with undo command
    #[must_use]
    pub fn with_undo(self, undo_cmd: impl Into<String>) -> Self {
        Self {
            command: self.command,
            timestamp: self.timestamp,
            duration_ms: self.duration_ms,
            dry_run: self.dry_run,
            reversible: Some(true),
            undo_command: Some(undo_cmd.into()),
            request_id: self.request_id,
            agent_id: self.agent_id,
        }
    }

    /// Set agent ID
    #[must_use]
    pub fn with_agent(self, agent_id: impl Into<String>) -> Self {
        Self {
            command: self.command,
            timestamp: self.timestamp,
            duration_ms: self.duration_ms,
            dry_run: self.dry_run,
            reversible: self.reversible,
            undo_command: self.undo_command,
            request_id: self.request_id,
            agent_id: Some(agent_id.into()),
        }
    }

    /// Set request ID
    #[must_use]
    pub fn with_request_id(self, request_id: impl Into<String>) -> Self {
        Self {
            command: self.command,
            timestamp: self.timestamp,
            duration_ms: self.duration_ms,
            dry_run: self.dry_run,
            reversible: self.reversible,
            undo_command: self.undo_command,
            request_id: Some(request_id.into()),
            agent_id: self.agent_id,
        }
    }
}
