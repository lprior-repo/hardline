//! Domain events for workspace isolation lifecycle.
//!
//! Events capture state changes in the isolate bounded context:
//! workspace lifecycle, session management, and agent coordination.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::WorkspaceState;

/// Event type classification for isolate domain events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Workspace lifecycle events (create, activate, complete, etc.)
    WorkspaceLifecycle,
    /// Session management events (start, end)
    Session,
    /// Agent coordination events (claim, release)
    Agent,
    /// Version control events (branch, rebase, conflict)
    Vcs,
}

impl EventType {
    /// All event type variants.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::WorkspaceLifecycle,
            Self::Session,
            Self::Agent,
            Self::Vcs,
        ]
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::WorkspaceLifecycle => "workspace_lifecycle",
            Self::Session => "session",
            Self::Agent => "agent",
            Self::Vcs => "vcs",
        })
    }
}

/// Context metadata: who/what generated the event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventContext {
    /// Workspace name this event relates to.
    pub workspace: String,
    /// Agent that triggered this event, if any.
    pub agent_id: Option<String>,
    /// Session this event occurred in, if any.
    pub session_id: Option<String>,
}

impl EventContext {
    /// Create context for a workspace with no agent or session.
    #[must_use]
    pub fn for_workspace(workspace: String) -> Self {
        Self {
            workspace,
            agent_id: None,
            session_id: None,
        }
    }

    /// Create context for a specific agent in a workspace.
    #[must_use]
    pub fn for_agent(workspace: String, agent_id: String) -> Self {
        Self {
            workspace,
            agent_id: Some(agent_id),
            session_id: None,
        }
    }

    /// Create context for a specific session in a workspace.
    #[must_use]
    pub fn for_session(workspace: String, session_id: String) -> Self {
        Self {
            workspace,
            agent_id: None,
            session_id: Some(session_id),
        }
    }

    /// Attach a session ID to this context.
    #[must_use]
    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Whether this context has an agent attached.
    #[must_use]
    pub const fn has_agent(&self) -> bool {
        self.agent_id.is_some()
    }

    /// Whether this context has a session attached.
    #[must_use]
    pub const fn has_session(&self) -> bool {
        self.session_id.is_some()
    }
}

/// Domain events for the isolate bounded context.
///
/// Each variant captures a specific state change in the workspace
/// lifecycle, along with relevant context metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IsolateEvent {
    // -- Workspace lifecycle --
    /// Workspace record created (clone not yet started).
    WorkspaceCreated {
        name: String,
        source: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Workspace activated (clone complete, ready for work).
    WorkspaceActivated {
        name: String,
        state: WorkspaceState,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Workspace syncing with upstream.
    WorkspaceSyncing {
        name: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Workspace sync completed.
    WorkspaceSynced {
        name: String,
        commits_rebased: usize,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Workspace paused (agent disconnected).
    WorkspacePaused {
        name: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Workspace resumed from pause.
    WorkspaceResumed {
        name: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Workspace completed successfully.
    WorkspaceCompleted {
        name: String,
        branch: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Workspace failed or abandoned.
    WorkspaceFailed {
        name: String,
        reason: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    // -- Session events --
    /// Session started in a workspace.
    SessionStarted {
        name: String,
        session_id: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Session ended.
    SessionEnded {
        name: String,
        session_id: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    // -- Agent events --
    /// Agent claimed a workspace.
    AgentClaimed {
        name: String,
        agent_id: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Agent released a workspace.
    AgentReleased {
        name: String,
        agent_id: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    // -- VCS events --
    /// Branch created in workspace.
    BranchCreated {
        name: String,
        branch: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Changes pushed from workspace.
    BranchPushed {
        name: String,
        branch: String,
        commits: usize,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Rebase completed in workspace.
    RebaseCompleted {
        name: String,
        commits: usize,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Conflict detected during sync.
    ConflictDetected {
        name: String,
        files: Vec<String>,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
    /// Conflict resolved in workspace.
    ConflictResolved {
        name: String,
        context: EventContext,
        timestamp: DateTime<Utc>,
    },
}

impl IsolateEvent {
    /// Get the event type classification.
    #[must_use]
    pub fn event_type(&self) -> EventType {
        match self {
            Self::WorkspaceCreated { .. }
            | Self::WorkspaceActivated { .. }
            | Self::WorkspaceSyncing { .. }
            | Self::WorkspaceSynced { .. }
            | Self::WorkspacePaused { .. }
            | Self::WorkspaceResumed { .. }
            | Self::WorkspaceCompleted { .. }
            | Self::WorkspaceFailed { .. } => EventType::WorkspaceLifecycle,
            Self::SessionStarted { .. } | Self::SessionEnded { .. } => EventType::Session,
            Self::AgentClaimed { .. } | Self::AgentReleased { .. } => EventType::Agent,
            Self::BranchCreated { .. }
            | Self::BranchPushed { .. }
            | Self::RebaseCompleted { .. }
            | Self::ConflictDetected { .. }
            | Self::ConflictResolved { .. } => EventType::Vcs,
        }
    }

    /// Get a dot-separated event name for logging/serialization.
    #[must_use]
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::WorkspaceCreated { .. } => "workspace.created",
            Self::WorkspaceActivated { .. } => "workspace.activated",
            Self::WorkspaceSyncing { .. } => "workspace.syncing",
            Self::WorkspaceSynced { .. } => "workspace.synced",
            Self::WorkspacePaused { .. } => "workspace.paused",
            Self::WorkspaceResumed { .. } => "workspace.resumed",
            Self::WorkspaceCompleted { .. } => "workspace.completed",
            Self::WorkspaceFailed { .. } => "workspace.failed",
            Self::SessionStarted { .. } => "session.started",
            Self::SessionEnded { .. } => "session.ended",
            Self::AgentClaimed { .. } => "agent.claimed",
            Self::AgentReleased { .. } => "agent.released",
            Self::BranchCreated { .. } => "vcs.branch_created",
            Self::BranchPushed { .. } => "vcs.branch_pushed",
            Self::RebaseCompleted { .. } => "vcs.rebase_completed",
            Self::ConflictDetected { .. } => "vcs.conflict_detected",
            Self::ConflictResolved { .. } => "vcs.conflict_resolved",
        }
    }

    /// Get the workspace name associated with this event.
    #[must_use]
    pub fn workspace(&self) -> &str {
        match self {
            Self::WorkspaceCreated { name, .. }
            | Self::WorkspaceActivated { name, .. }
            | Self::WorkspaceSyncing { name, .. }
            | Self::WorkspaceSynced { name, .. }
            | Self::WorkspacePaused { name, .. }
            | Self::WorkspaceResumed { name, .. }
            | Self::WorkspaceCompleted { name, .. }
            | Self::WorkspaceFailed { name, .. }
            | Self::SessionStarted { name, .. }
            | Self::SessionEnded { name, .. }
            | Self::AgentClaimed { name, .. }
            | Self::AgentReleased { name, .. }
            | Self::BranchCreated { name, .. }
            | Self::BranchPushed { name, .. }
            | Self::RebaseCompleted { name, .. }
            | Self::ConflictDetected { name, .. }
            | Self::ConflictResolved { name, .. } => name,
        }
    }

    /// Get the event context metadata.
    #[must_use]
    pub fn context(&self) -> &EventContext {
        match self {
            Self::WorkspaceCreated { context, .. }
            | Self::WorkspaceActivated { context, .. }
            | Self::WorkspaceSyncing { context, .. }
            | Self::WorkspaceSynced { context, .. }
            | Self::WorkspacePaused { context, .. }
            | Self::WorkspaceResumed { context, .. }
            | Self::WorkspaceCompleted { context, .. }
            | Self::WorkspaceFailed { context, .. }
            | Self::SessionStarted { context, .. }
            | Self::SessionEnded { context, .. }
            | Self::AgentClaimed { context, .. }
            | Self::AgentReleased { context, .. }
            | Self::BranchCreated { context, .. }
            | Self::BranchPushed { context, .. }
            | Self::RebaseCompleted { context, .. }
            | Self::ConflictDetected { context, .. }
            | Self::ConflictResolved { context, .. } => context,
        }
    }

    /// Get the timestamp of this event.
    #[must_use]
    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            Self::WorkspaceCreated { timestamp, .. }
            | Self::WorkspaceActivated { timestamp, .. }
            | Self::WorkspaceSyncing { timestamp, .. }
            | Self::WorkspaceSynced { timestamp, .. }
            | Self::WorkspacePaused { timestamp, .. }
            | Self::WorkspaceResumed { timestamp, .. }
            | Self::WorkspaceCompleted { timestamp, .. }
            | Self::WorkspaceFailed { timestamp, .. }
            | Self::SessionStarted { timestamp, .. }
            | Self::SessionEnded { timestamp, .. }
            | Self::AgentClaimed { timestamp, .. }
            | Self::AgentReleased { timestamp, .. }
            | Self::BranchCreated { timestamp, .. }
            | Self::BranchPushed { timestamp, .. }
            | Self::RebaseCompleted { timestamp, .. }
            | Self::ConflictDetected { timestamp, .. }
            | Self::ConflictResolved { timestamp, .. } => timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(name: &str) -> EventContext {
        EventContext::for_workspace(name.to_string())
    }

    fn agent_ctx(name: &str, agent: &str) -> EventContext {
        EventContext::for_agent(name.to_string(), agent.to_string())
    }

    fn session_ctx(name: &str, session: &str) -> EventContext {
        EventContext::for_session(name.to_string(), session.to_string())
    }

    // -- EventType tests --

    #[test]
    fn event_type_all_returns_four_variants() {
        assert_eq!(EventType::all().len(), 4);
    }

    #[test]
    fn event_type_display() {
        assert_eq!(
            EventType::WorkspaceLifecycle.to_string(),
            "workspace_lifecycle"
        );
        assert_eq!(EventType::Session.to_string(), "session");
        assert_eq!(EventType::Agent.to_string(), "agent");
        assert_eq!(EventType::Vcs.to_string(), "vcs");
    }

    // -- EventContext tests --

    #[test]
    fn context_for_workspace() {
        let c = EventContext::for_workspace("ws-1".into());
        assert_eq!(c.workspace, "ws-1");
        assert!(!c.has_agent());
        assert!(!c.has_session());
    }

    #[test]
    fn context_for_agent() {
        let c = EventContext::for_agent("ws-1".into(), "agent-42".into());
        assert!(c.has_agent());
        assert!(!c.has_session());
        assert_eq!(c.agent_id.as_deref(), Some("agent-42"));
    }

    #[test]
    fn context_for_session() {
        let c = EventContext::for_session("ws-1".into(), "sess-7".into());
        assert!(!c.has_agent());
        assert!(c.has_session());
        assert_eq!(c.session_id.as_deref(), Some("sess-7"));
    }

    #[test]
    fn context_with_session_chaining() {
        let c = EventContext::for_agent("ws".into(), "a1".into()).with_session("s1".into());
        assert!(c.has_agent());
        assert!(c.has_session());
    }

    #[test]
    fn context_serde_roundtrip() {
        let c = EventContext {
            workspace: "w".into(),
            agent_id: Some("a".into()),
            session_id: Some("s".into()),
        };
        let json = serde_json::to_string(&c).unwrap();
        let parsed: EventContext = serde_json::from_str(&json).unwrap();
        assert_eq!(c, parsed);
    }

    // -- IsolateEvent event_type() tests --

    #[test]
    fn workspace_events_classify_lifecycle() {
        let events: Vec<IsolateEvent> = vec![
            IsolateEvent::WorkspaceCreated {
                name: "w".into(),
                source: "cli".into(),
                context: ctx("w"),
                timestamp: Utc::now(),
            },
            IsolateEvent::WorkspaceActivated {
                name: "w".into(),
                state: WorkspaceState::Working,
                context: ctx("w"),
                timestamp: Utc::now(),
            },
            IsolateEvent::WorkspaceSyncing {
                name: "w".into(),
                context: ctx("w"),
                timestamp: Utc::now(),
            },
            IsolateEvent::WorkspaceSynced {
                name: "w".into(),
                commits_rebased: 3,
                context: ctx("w"),
                timestamp: Utc::now(),
            },
            IsolateEvent::WorkspacePaused {
                name: "w".into(),
                context: ctx("w"),
                timestamp: Utc::now(),
            },
            IsolateEvent::WorkspaceResumed {
                name: "w".into(),
                context: ctx("w"),
                timestamp: Utc::now(),
            },
            IsolateEvent::WorkspaceCompleted {
                name: "w".into(),
                branch: "b".into(),
                context: ctx("w"),
                timestamp: Utc::now(),
            },
            IsolateEvent::WorkspaceFailed {
                name: "w".into(),
                reason: "timeout".into(),
                context: ctx("w"),
                timestamp: Utc::now(),
            },
        ];
        for e in &events {
            assert_eq!(e.event_type(), EventType::WorkspaceLifecycle);
        }
    }

    #[test]
    fn session_events_classify_session() {
        let now = Utc::now();
        let started = IsolateEvent::SessionStarted {
            name: "w".into(),
            session_id: "s1".into(),
            context: session_ctx("w", "s1"),
            timestamp: now,
        };
        let ended = IsolateEvent::SessionEnded {
            name: "w".into(),
            session_id: "s1".into(),
            context: session_ctx("w", "s1"),
            timestamp: now,
        };
        assert_eq!(started.event_type(), EventType::Session);
        assert_eq!(ended.event_type(), EventType::Session);
    }

    #[test]
    fn agent_events_classify_agent() {
        let now = Utc::now();
        let claimed = IsolateEvent::AgentClaimed {
            name: "w".into(),
            agent_id: "a1".into(),
            context: agent_ctx("w", "a1"),
            timestamp: now,
        };
        let released = IsolateEvent::AgentReleased {
            name: "w".into(),
            agent_id: "a1".into(),
            context: agent_ctx("w", "a1"),
            timestamp: now,
        };
        assert_eq!(claimed.event_type(), EventType::Agent);
        assert_eq!(released.event_type(), EventType::Agent);
    }

    #[test]
    fn vcs_events_classify_vcs() {
        let now = Utc::now();
        let events: Vec<IsolateEvent> = vec![
            IsolateEvent::BranchCreated {
                name: "w".into(),
                branch: "b".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::BranchPushed {
                name: "w".into(),
                branch: "b".into(),
                commits: 2,
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::RebaseCompleted {
                name: "w".into(),
                commits: 5,
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::ConflictDetected {
                name: "w".into(),
                files: vec!["a.rs".into()],
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::ConflictResolved {
                name: "w".into(),
                context: ctx("w"),
                timestamp: now,
            },
        ];
        for e in &events {
            assert_eq!(e.event_type(), EventType::Vcs);
        }
    }

    // -- event_name() exhaustiveness --

    #[test]
    fn all_event_names_are_dot_separated() {
        let now = Utc::now();
        let events: Vec<IsolateEvent> = vec![
            IsolateEvent::WorkspaceCreated {
                name: "w".into(),
                source: "cli".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::WorkspaceActivated {
                name: "w".into(),
                state: WorkspaceState::Working,
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::WorkspaceSyncing {
                name: "w".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::WorkspaceSynced {
                name: "w".into(),
                commits_rebased: 0,
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::WorkspacePaused {
                name: "w".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::WorkspaceResumed {
                name: "w".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::WorkspaceCompleted {
                name: "w".into(),
                branch: "b".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::WorkspaceFailed {
                name: "w".into(),
                reason: "err".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::SessionStarted {
                name: "w".into(),
                session_id: "s".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::SessionEnded {
                name: "w".into(),
                session_id: "s".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::AgentClaimed {
                name: "w".into(),
                agent_id: "a".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::AgentReleased {
                name: "w".into(),
                agent_id: "a".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::BranchCreated {
                name: "w".into(),
                branch: "b".into(),
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::BranchPushed {
                name: "w".into(),
                branch: "b".into(),
                commits: 0,
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::RebaseCompleted {
                name: "w".into(),
                commits: 0,
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::ConflictDetected {
                name: "w".into(),
                files: vec![],
                context: ctx("w"),
                timestamp: now,
            },
            IsolateEvent::ConflictResolved {
                name: "w".into(),
                context: ctx("w"),
                timestamp: now,
            },
        ];
        for e in &events {
            assert!(
                e.event_name().contains('.'),
                "event_name '{}' should be dot-separated",
                e.event_name()
            );
        }
    }

    // -- workspace() accessor --

    #[test]
    fn workspace_accessor_returns_name() {
        let e = IsolateEvent::WorkspaceCreated {
            name: "my-ws".into(),
            source: "test".into(),
            context: ctx("my-ws"),
            timestamp: Utc::now(),
        };
        assert_eq!(e.workspace(), "my-ws");
    }

    // -- context() accessor --

    #[test]
    fn context_accessor_returns_context() {
        let c = agent_ctx("w", "a1");
        let e = IsolateEvent::AgentClaimed {
            name: "w".into(),
            agent_id: "a1".into(),
            context: c.clone(),
            timestamp: Utc::now(),
        };
        assert_eq!(e.context(), &c);
    }

    // -- timestamp() accessor --

    #[test]
    fn timestamp_accessor_returns_timestamp() {
        let ts = Utc::now();
        let e = IsolateEvent::WorkspacePaused {
            name: "w".into(),
            context: ctx("w"),
            timestamp: ts,
        };
        assert_eq!(e.timestamp(), &ts);
    }

    // -- Serde roundtrip --

    #[test]
    fn serde_roundtrip_workspace_created() {
        let e = IsolateEvent::WorkspaceCreated {
            name: "ws".into(),
            source: "cli".into(),
            context: ctx("ws"),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&e).unwrap();
        let parsed: IsolateEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, parsed);
    }

    #[test]
    fn serde_roundtrip_conflict_detected() {
        let e = IsolateEvent::ConflictDetected {
            name: "ws".into(),
            files: vec!["a.rs".into(), "b.rs".into()],
            context: ctx("ws"),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&e).unwrap();
        let parsed: IsolateEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, parsed);
    }

    #[test]
    fn serde_roundtrip_session_started() {
        let e = IsolateEvent::SessionStarted {
            name: "ws".into(),
            session_id: "sess-1".into(),
            context: session_ctx("ws", "sess-1"),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&e).unwrap();
        let parsed: IsolateEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, parsed);
    }

    #[test]
    fn serde_tagged_type_field() {
        let e = IsolateEvent::AgentClaimed {
            name: "w".into(),
            agent_id: "a".into(),
            context: ctx("w"),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"type\":\"AgentClaimed\""));
    }

    // -- Clone --

    #[test]
    fn event_is_clone() {
        let e = IsolateEvent::BranchPushed {
            name: "w".into(),
            branch: "b".into(),
            commits: 3,
            context: ctx("w"),
            timestamp: Utc::now(),
        };
        let cloned = e.clone();
        assert_eq!(e, cloned);
    }

    // -- Debug --

    #[test]
    fn event_is_debug() {
        let e = IsolateEvent::WorkspaceFailed {
            name: "w".into(),
            reason: "timeout".into(),
            context: ctx("w"),
            timestamp: Utc::now(),
        };
        let dbg = format!("{e:?}");
        assert!(dbg.contains("WorkspaceFailed"));
    }
}
