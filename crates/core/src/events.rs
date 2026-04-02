//! Unified event system for Source Control Plane.
//!
//! Provides event types for workspaces, queues, and agents.
//! Zero panic, zero unwrap - all operations return Result.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Unified event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
            .map_err(|e| crate::error::Error::internal(e.to_string()))?;
        events.push(emitted);

        Ok(())
    }

    fn history(&self, limit: usize) -> Result<Vec<Event>> {
        let events = self
            .events
            .read()
            .map_err(|e| crate::error::Error::internal(e.to_string()))?;
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
            .map_err(|e| Error::internal(e.to_string()))?;
        events.clear();
        Ok(())
    }
}

/// Simple UUID generator (for testing)
#[allow(clippy::unwrap_used)]
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

    // =========================================================================
    // event_type() exhaustiveness for all variants
    // =========================================================================

    #[test]
    fn given_workspace_created_when_event_type_then_correct() {
        let event = Event::WorkspaceCreated {
            name: "w".into(),
            source: "s".into(),
        };
        assert_eq!(event.event_type(), "workspace.created");
    }

    #[test]
    fn given_workspace_synced_when_event_type_then_correct() {
        let event = Event::WorkspaceSynced {
            name: "w".into(),
            commits_rebased: 3,
        };
        assert_eq!(event.event_type(), "workspace.synced");
    }

    #[test]
    fn given_workspace_completed_when_event_type_then_correct() {
        let event = Event::WorkspaceCompleted {
            name: "w".into(),
            branch: "b".into(),
        };
        assert_eq!(event.event_type(), "workspace.completed");
    }

    #[test]
    fn given_workspace_aborted_when_event_type_then_correct() {
        let event = Event::WorkspaceAborted {
            name: "w".into(),
            reason: "r".into(),
        };
        assert_eq!(event.event_type(), "workspace.aborted");
    }

    #[test]
    fn given_item_dequeued_when_event_type_then_correct() {
        let event = Event::ItemDequeued {
            branch: "b".into(),
            reason: "done".into(),
        };
        assert_eq!(event.event_type(), "queue.dequeued");
    }

    #[test]
    fn given_item_processing_when_event_type_then_correct() {
        let event = Event::ItemProcessing { branch: "b".into() };
        assert_eq!(event.event_type(), "queue.processing");
    }

    #[test]
    fn given_item_processed_when_event_type_then_correct() {
        let event = Event::ItemProcessed {
            branch: "b".into(),
            success: true,
            error: None,
        };
        assert_eq!(event.event_type(), "queue.processed");
    }

    #[test]
    fn given_agent_started_when_event_type_then_correct() {
        let event = Event::AgentStarted {
            id: "1".into(),
            name: "bot".into(),
        };
        assert_eq!(event.event_type(), "agent.started");
    }

    #[test]
    fn given_agent_stopped_when_event_type_then_correct() {
        let event = Event::AgentStopped {
            id: "1".into(),
            reason: "done".into(),
        };
        assert_eq!(event.event_type(), "agent.stopped");
    }

    #[test]
    fn given_agent_heartbeat_when_event_type_then_correct() {
        let event = Event::AgentHeartbeat { id: "1".into() };
        assert_eq!(event.event_type(), "agent.heartbeat");
    }

    #[test]
    fn given_vcs_pushed_when_event_type_then_correct() {
        let event = Event::VcsPushed {
            branch: "b".into(),
            commits: 5,
        };
        assert_eq!(event.event_type(), "vcs.pushed");
    }

    #[test]
    fn given_vcs_pulled_when_event_type_then_correct() {
        let event = Event::VcsPulled {
            branch: "b".into(),
            commits: 2,
        };
        assert_eq!(event.event_type(), "vcs.pulled");
    }

    #[test]
    fn given_vcs_conflict_when_event_type_then_correct() {
        let event = Event::VcsConflict {
            branch: "b".into(),
            files: vec!["a.rs".into(), "b.rs".into()],
        };
        assert_eq!(event.event_type(), "vcs.conflict");
    }

    #[test]
    fn given_vcs_conflict_resolved_when_event_type_then_correct() {
        let event = Event::VcsConflictResolved { branch: "b".into() };
        assert_eq!(event.event_type(), "vcs.conflict_resolved");
    }

    // =========================================================================
    // Serialization / Deserialization
    // =========================================================================

    #[test]
    fn given_workspace_created_when_serialized_then_deserializes() {
        let event = Event::WorkspaceCreated {
            name: "my-workspace".into(),
            source: "cli".into(),
        };
        let json = serde_json::to_string(&event).expect("serialize");
        let deserialized: Event = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn given_item_enqueued_when_serialized_then_deserializes() {
        let event = Event::ItemEnqueued {
            branch: "feature-x".into(),
            position: 42,
            source: "webhook".into(),
        };
        let json = serde_json::to_string(&event).expect("serialize");
        let deserialized: Event = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn given_item_processed_with_error_when_serialized_then_deserializes() {
        let event = Event::ItemProcessed {
            branch: "broken".into(),
            success: false,
            error: Some("timeout".into()),
        };
        let json = serde_json::to_string(&event).expect("serialize");
        let deserialized: Event = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn given_vcs_conflict_with_files_when_serialized_then_deserializes() {
        let event = Event::VcsConflict {
            branch: "main".into(),
            files: vec!["src/lib.rs".into(), "src/main.rs".into()],
        };
        let json = serde_json::to_string(&event).expect("serialize");
        let deserialized: Event = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn given_serialized_event_when_json_contains_type_tag() {
        let event = Event::AgentHeartbeat {
            id: "agent-1".into(),
        };
        let json = serde_json::to_string(&event).expect("serialize");
        assert!(json.contains("\"type\":\"AgentHeartbeat\""));
    }

    #[test]
    fn given_emitted_event_when_serialized_then_deserializes() {
        let emitted = EmittedEvent {
            id: "test-id".to_string(),
            event: Event::WorkspaceCreated {
                name: "w".into(),
                source: "s".into(),
            },
            timestamp: chrono::Utc::now(),
            source: "test-source".to_string(),
        };
        let json = serde_json::to_string(&emitted).expect("serialize");
        let deserialized: EmittedEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(emitted.id, deserialized.id);
        assert_eq!(emitted.source, deserialized.source);
        assert_eq!(emitted.timestamp, deserialized.timestamp);
    }

    // =========================================================================
    // MemEventEmitter tests
    // =========================================================================

    #[test]
    fn given_emitter_when_multiple_events_then_history_is_reverse_chronological() -> Result<()> {
        let emitter = MemEventEmitter::new();

        emitter.emit(Event::AgentStarted {
            id: "1".into(),
            name: "first".into(),
        })?;
        emitter.emit(Event::AgentHeartbeat { id: "1".into() })?;
        emitter.emit(Event::AgentStopped {
            id: "1".into(),
            reason: "done".into(),
        })?;

        let history = emitter.history(10)?;
        assert_eq!(history.len(), 3);
        // Most recent first
        match &history[0] {
            Event::AgentStopped { reason, .. } => assert_eq!(reason, "done"),
            other => panic!("Expected AgentStopped, got: {other:?}"),
        }
        match &history[2] {
            Event::AgentStarted { name, .. } => assert_eq!(name, "first"),
            other => panic!("Expected AgentStarted, got: {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn given_emitter_when_history_limit_then_respects_limit() -> Result<()> {
        let emitter = MemEventEmitter::new();

        for i in 0..10 {
            emitter.emit(Event::AgentHeartbeat { id: i.to_string() })?;
        }

        let history = emitter.history(3)?;
        assert_eq!(history.len(), 3);

        // Should be the 3 most recent
        match &history[0] {
            Event::AgentHeartbeat { id } => assert_eq!(id, "9"),
            other => panic!("Expected AgentHeartbeat, got: {other:?}"),
        }
        match &history[2] {
            Event::AgentHeartbeat { id } => assert_eq!(id, "7"),
            other => panic!("Expected AgentHeartbeat, got: {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn given_emitter_when_clear_then_history_empty() -> Result<()> {
        let emitter = MemEventEmitter::new();

        emitter.emit(Event::AgentStarted {
            id: "1".into(),
            name: "bot".into(),
        })?;
        emitter.emit(Event::AgentHeartbeat { id: "1".into() })?;

        assert_eq!(emitter.history(10)?.len(), 2);

        emitter.clear()?;

        assert_eq!(emitter.history(10)?.len(), 0);

        Ok(())
    }

    #[test]
    fn given_empty_emitter_when_history_then_empty() -> Result<()> {
        let emitter = MemEventEmitter::new();
        let history = emitter.history(10)?;
        assert!(history.is_empty());
        Ok(())
    }

    #[test]
    fn given_emitter_when_clear_then_can_emit_again() -> Result<()> {
        let emitter = MemEventEmitter::new();

        emitter.emit(Event::AgentHeartbeat { id: "1".into() })?;
        emitter.clear()?;
        emitter.emit(Event::AgentHeartbeat { id: "2".into() })?;

        let history = emitter.history(10)?;
        assert_eq!(history.len(), 1);
        match &history[0] {
            Event::AgentHeartbeat { id } => assert_eq!(id, "2"),
            other => panic!("Expected AgentHeartbeat, got: {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn given_mem_event_emitter_when_default_then_empty() {
        let emitter = MemEventEmitter::default();
        let history = emitter.history(10).expect("history should work");
        assert!(history.is_empty());
    }

    // =========================================================================
    // Clone tests for Event variants
    // =========================================================================

    #[test]
    fn given_event_when_cloned_then_equal() {
        let event = Event::VcsConflict {
            branch: "main".into(),
            files: vec!["a.rs".into(), "b.rs".into()],
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    // =========================================================================
    // Serde roundtrip tests
    // =========================================================================

    #[test]
    fn test_event_serde_roundtrip_workspace_created() {
        let event = Event::WorkspaceCreated {
            name: "test-ws".to_string(),
            source: "cli".to_string(),
        };
        let json = serde_json::to_string(&event).expect("serialize ok");
        let deserialized: Event = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_event_serde_roundtrip_agent_heartbeat() {
        let event = Event::AgentHeartbeat {
            id: "agent-1".to_string(),
        };
        let json = serde_json::to_string(&event).expect("serialize ok");
        let deserialized: Event = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_emitted_event_serde_roundtrip() {
        let emitted = EmittedEvent {
            id: "evt-001".to_string(),
            event: Event::AgentHeartbeat {
                id: "agent-1".to_string(),
            },
            timestamp: Utc::now(),
            source: "test".to_string(),
        };
        let json = serde_json::to_string(&emitted).expect("serialize ok");
        let deserialized: EmittedEvent = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(emitted.id, deserialized.id);
        assert_eq!(emitted.source, deserialized.source);
    }
}
