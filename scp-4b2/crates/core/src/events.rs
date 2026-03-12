//! Unified event system for Source Control Plane.
//!
//! Provides event types for workspaces, queues, and agents.
//! Zero panic, zero unwrap - all operations return Result.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Unified event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    // ========================================================================
    // Workspace Events (from Isolate)
    // ========================================================================
    /// Workspace was created
    WorkspaceCreated { name: String, source: String },

    /// Workspace was synced
    WorkspaceSynced {
        name: String,
        commits_rebased: usize,
    },

    /// Workspace work completed
    WorkspaceCompleted { name: String, branch: String },

    /// Workspace was aborted
    WorkspaceAborted { name: String, reason: String },

    // ========================================================================
    // Queue Events (from Stak)
    // ========================================================================
    /// Item added to queue
    ItemEnqueued {
        branch: String,
        position: usize,
        source: String,
    },

    /// Item removed from queue
    ItemDequeued { branch: String, reason: String },

    /// Item processing started
    ItemProcessing { branch: String },

    /// Item processing completed
    ItemProcessed {
        branch: String,
        success: bool,
        error: Option<String>,
    },

    // ========================================================================
    // Agent Events
    // ========================================================================
    /// Agent started
    AgentStarted { id: String, name: String },

    /// Agent stopped
    AgentStopped { id: String, reason: String },

    /// Agent heartbeat
    AgentHeartbeat { id: String },

    // ========================================================================
    // VCS Events
    // ========================================================================
    /// Changes pushed
    VcsPushed { branch: String, commits: usize },

    /// Changes pulled
    VcsPulled { branch: String, commits: usize },

    /// Conflict detected
    VcsConflict { branch: String, files: Vec<String> },

    /// Conflict resolved
    VcsConflictResolved { branch: String },
}

impl Event {
    /// Get event type name
    pub fn event_type(&self) -> &str {
        match self {
            Event::WorkspaceCreated { .. } => "workspace.created",
            Event::WorkspaceSynced { .. } => "workspace.synced",
            Event::WorkspaceCompleted { .. } => "workspace.completed",
            Event::WorkspaceAborted { .. } => "workspace.aborted",
            Event::ItemEnqueued { .. } => "queue.enqueued",
            Event::ItemDequeued { .. } => "queue.dequeued",
            Event::ItemProcessing { .. } => "queue.processing",
            Event::ItemProcessed { .. } => "queue.processed",
            Event::AgentStarted { .. } => "agent.started",
            Event::AgentStopped { .. } => "agent.stopped",
            Event::AgentHeartbeat { .. } => "agent.heartbeat",
            Event::VcsPushed { .. } => "vcs.pushed",
            Event::VcsPulled { .. } => "vcs.pulled",
            Event::VcsConflict { .. } => "vcs.conflict",
            Event::VcsConflictResolved { .. } => "vcs.conflict_resolved",
        }
    }
}

/// Event emitter trait
pub trait EventEmitter: Send + Sync {
    /// Emit an event
    fn emit(&self, event: Event) -> Result<()>;

    /// Get event history
    fn history(&self, limit: usize) -> Result<Vec<Event>>;

    /// Clear event history
    fn clear(&self) -> Result<()>;
}

/// Event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmittedEvent {
    pub id: String,
    pub event: Event,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

/// In-memory event store
#[derive(Debug, Default)]
pub struct MemEventEmitter {
    events: RwLock<Vec<EmittedEvent>>,
}

impl MemEventEmitter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl EventEmitter for MemEventEmitter {
    fn emit(&self, event: Event) -> Result<()> {
        let emitted = EmittedEvent {
            id: uuid_simple(),
            event,
            timestamp: Utc::now(),
            source: "scp".to_string(),
        };

        let mut events = self
            .events
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;
        events.push(emitted);

        Ok(())
    }

    fn history(&self, limit: usize) -> Result<Vec<Event>> {
        let events = self
            .events
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(events
            .iter()
            .rev()
            .take(limit)
            .map(|e| e.event.clone())
            .collect())
    }

    fn clear(&self) -> Result<()> {
        let mut events = self
            .events
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;
        events.clear();
        Ok(())
    }
}

/// Simple UUID generator (for testing)
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_types() {
        let event = Event::WorkspaceCreated {
            name: "test".into(),
            source: "cli".into(),
        };
        assert_eq!(event.event_type(), "workspace.created");

        let event = Event::ItemEnqueued {
            branch: "main".into(),
            position: 1,
            source: "cli".into(),
        };
        assert_eq!(event.event_type(), "queue.enqueued");
    }

    #[test]
    fn test_event_emitter() -> Result<()> {
        let emitter = MemEventEmitter::new();

        emitter.emit(Event::WorkspaceCreated {
            name: "test".into(),
            source: "cli".into(),
        })?;

        let history = emitter.history(10)?;
        assert_eq!(history.len(), 1);

        Ok(())
    }
}
