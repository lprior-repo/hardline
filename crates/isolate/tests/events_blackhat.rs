//! Black-hat tests for isolate domain events.
//!
//! Covers:
//! - All 16 IsolateEvent variants construction and accessors
//! - EventType classification for every variant
//! - EventContext construction, chaining, and predicates
//! - event_name() returns dot-separated strings
//! - Serde roundtrip for all variants
//! - Clone, Debug, PartialEq for all variants
//! - Proptests for serde roundtrip and classification

use chrono::Utc;

use scp_isolate::{EventContext, EventType, IsolateEvent, WorkspaceState};

fn ctx(name: &str) -> EventContext {
    EventContext::for_workspace(name.to_string())
}

fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}

// === EventType classification ===

#[test]
fn all_event_types_covered() {
    assert_eq!(EventType::all().len(), 4);
    assert!(EventType::all().contains(&EventType::WorkspaceLifecycle));
    assert!(EventType::all().contains(&EventType::Session));
    assert!(EventType::all().contains(&EventType::Agent));
    assert!(EventType::all().contains(&EventType::Vcs));
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

// Table-driven: every event variant maps to the correct EventType
#[test]
fn table_driven_event_type_classification() {
    let cases: Vec<(IsolateEvent, EventType)> = vec![
        (
            IsolateEvent::WorkspaceCreated {
                name: "w".into(),
                source: "cli".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::WorkspaceLifecycle,
        ),
        (
            IsolateEvent::WorkspaceActivated {
                name: "w".into(),
                state: WorkspaceState::Working,
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::WorkspaceLifecycle,
        ),
        (
            IsolateEvent::WorkspaceSyncing {
                name: "w".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::WorkspaceLifecycle,
        ),
        (
            IsolateEvent::WorkspaceSynced {
                name: "w".into(),
                commits_rebased: 0,
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::WorkspaceLifecycle,
        ),
        (
            IsolateEvent::WorkspacePaused {
                name: "w".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::WorkspaceLifecycle,
        ),
        (
            IsolateEvent::WorkspaceResumed {
                name: "w".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::WorkspaceLifecycle,
        ),
        (
            IsolateEvent::WorkspaceCompleted {
                name: "w".into(),
                branch: "b".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::WorkspaceLifecycle,
        ),
        (
            IsolateEvent::WorkspaceFailed {
                name: "w".into(),
                reason: "err".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::WorkspaceLifecycle,
        ),
        (
            IsolateEvent::SessionStarted {
                name: "w".into(),
                session_id: "s".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::Session,
        ),
        (
            IsolateEvent::SessionEnded {
                name: "w".into(),
                session_id: "s".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::Session,
        ),
        (
            IsolateEvent::AgentClaimed {
                name: "w".into(),
                agent_id: "a".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::Agent,
        ),
        (
            IsolateEvent::AgentReleased {
                name: "w".into(),
                agent_id: "a".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::Agent,
        ),
        (
            IsolateEvent::BranchCreated {
                name: "w".into(),
                branch: "b".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::Vcs,
        ),
        (
            IsolateEvent::BranchPushed {
                name: "w".into(),
                branch: "b".into(),
                commits: 1,
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::Vcs,
        ),
        (
            IsolateEvent::RebaseCompleted {
                name: "w".into(),
                commits: 1,
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::Vcs,
        ),
        (
            IsolateEvent::ConflictDetected {
                name: "w".into(),
                files: vec![],
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::Vcs,
        ),
        (
            IsolateEvent::ConflictResolved {
                name: "w".into(),
                context: ctx("w"),
                timestamp: now(),
            },
            EventType::Vcs,
        ),
    ];

    assert_eq!(cases.len(), 17, "all 17 event variants must be covered");

    for (event, expected_type) in &cases {
        assert_eq!(
            event.event_type(),
            *expected_type,
            "wrong event_type for {:?}",
            event.event_name()
        );
    }
}

// === event_name() ===

#[test]
fn all_event_names_are_dot_separated() {
    let events: Vec<IsolateEvent> = vec![
        IsolateEvent::WorkspaceCreated {
            name: "w".into(),
            source: "cli".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceActivated {
            name: "w".into(),
            state: WorkspaceState::Working,
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceSyncing {
            name: "w".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceSynced {
            name: "w".into(),
            commits_rebased: 0,
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspacePaused {
            name: "w".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceResumed {
            name: "w".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceCompleted {
            name: "w".into(),
            branch: "b".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceFailed {
            name: "w".into(),
            reason: "err".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::SessionStarted {
            name: "w".into(),
            session_id: "s".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::SessionEnded {
            name: "w".into(),
            session_id: "s".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::AgentClaimed {
            name: "w".into(),
            agent_id: "a".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::AgentReleased {
            name: "w".into(),
            agent_id: "a".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::BranchCreated {
            name: "w".into(),
            branch: "b".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::BranchPushed {
            name: "w".into(),
            branch: "b".into(),
            commits: 0,
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::RebaseCompleted {
            name: "w".into(),
            commits: 0,
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::ConflictDetected {
            name: "w".into(),
            files: vec![],
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::ConflictResolved {
            name: "w".into(),
            context: ctx("w"),
            timestamp: now(),
        },
    ];
    for event in &events {
        let name = event.event_name();
        assert!(
            name.contains('.'),
            "event_name '{name}' should be dot-separated"
        );
        assert!(!name.is_empty(), "event_name should not be empty");
    }
}

#[test]
fn event_names_are_unique() {
    use std::collections::HashSet;
    let events: Vec<IsolateEvent> = vec![
        IsolateEvent::WorkspaceCreated {
            name: "w".into(),
            source: "cli".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceActivated {
            name: "w".into(),
            state: WorkspaceState::Working,
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceSyncing {
            name: "w".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceSynced {
            name: "w".into(),
            commits_rebased: 0,
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspacePaused {
            name: "w".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceResumed {
            name: "w".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceCompleted {
            name: "w".into(),
            branch: "b".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceFailed {
            name: "w".into(),
            reason: "err".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::SessionStarted {
            name: "w".into(),
            session_id: "s".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::SessionEnded {
            name: "w".into(),
            session_id: "s".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::AgentClaimed {
            name: "w".into(),
            agent_id: "a".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::AgentReleased {
            name: "w".into(),
            agent_id: "a".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::BranchCreated {
            name: "w".into(),
            branch: "b".into(),
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::BranchPushed {
            name: "w".into(),
            branch: "b".into(),
            commits: 0,
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::RebaseCompleted {
            name: "w".into(),
            commits: 0,
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::ConflictDetected {
            name: "w".into(),
            files: vec![],
            context: ctx("w"),
            timestamp: now(),
        },
        IsolateEvent::ConflictResolved {
            name: "w".into(),
            context: ctx("w"),
            timestamp: now(),
        },
    ];
    let names: HashSet<&str> = events.iter().map(|e| e.event_name()).collect();
    assert_eq!(
        names.len(),
        events.len(),
        "all event names should be unique"
    );
}

// === workspace() accessor ===

#[test]
fn workspace_accessor_returns_name_for_all_variants() {
    let events: Vec<IsolateEvent> = vec![
        IsolateEvent::WorkspaceCreated {
            name: "ws-a".into(),
            source: "cli".into(),
            context: ctx("ws-a"),
            timestamp: now(),
        },
        IsolateEvent::WorkspaceActivated {
            name: "ws-b".into(),
            state: WorkspaceState::Working,
            context: ctx("ws-b"),
            timestamp: now(),
        },
        IsolateEvent::SessionStarted {
            name: "ws-c".into(),
            session_id: "s".into(),
            context: ctx("ws-c"),
            timestamp: now(),
        },
        IsolateEvent::AgentClaimed {
            name: "ws-d".into(),
            agent_id: "a".into(),
            context: ctx("ws-d"),
            timestamp: now(),
        },
        IsolateEvent::BranchCreated {
            name: "ws-e".into(),
            branch: "b".into(),
            context: ctx("ws-e"),
            timestamp: now(),
        },
    ];
    assert_eq!(events[0].workspace(), "ws-a");
    assert_eq!(events[1].workspace(), "ws-b");
    assert_eq!(events[2].workspace(), "ws-c");
    assert_eq!(events[3].workspace(), "ws-d");
    assert_eq!(events[4].workspace(), "ws-e");
}

// === context() accessor ===

#[test]
fn context_accessor_returns_context() {
    let c = EventContext::for_agent("w".into(), "agent-1".into());
    let event = IsolateEvent::AgentClaimed {
        name: "w".into(),
        agent_id: "a".into(),
        context: c.clone(),
        timestamp: now(),
    };
    assert_eq!(event.context(), &c);
}

// === timestamp() accessor ===

#[test]
fn timestamp_accessor_returns_timestamp() {
    let ts = now();
    let event = IsolateEvent::WorkspacePaused {
        name: "w".into(),
        context: ctx("w"),
        timestamp: ts,
    };
    assert_eq!(event.timestamp(), &ts);
}

// === EventContext ===

#[test]
fn context_for_workspace_no_agent_no_session() {
    let c = EventContext::for_workspace("ws".into());
    assert_eq!(c.workspace, "ws");
    assert!(!c.has_agent());
    assert!(!c.has_session());
}

#[test]
fn context_for_agent_has_agent() {
    let c = EventContext::for_agent("ws".into(), "agent-42".into());
    assert!(c.has_agent());
    assert!(!c.has_session());
    assert_eq!(c.agent_id.as_deref(), Some("agent-42"));
}

#[test]
fn context_for_session_has_session() {
    let c = EventContext::for_session("ws".into(), "sess-7".into());
    assert!(!c.has_agent());
    assert!(c.has_session());
    assert_eq!(c.session_id.as_deref(), Some("sess-7"));
}

#[test]
fn context_with_session_chaining() {
    let c = EventContext::for_agent("ws".into(), "a".into()).with_session("s".into());
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

#[test]
fn context_equality() {
    let a = EventContext::for_agent("ws".into(), "agent".into());
    let b = EventContext::for_agent("ws".into(), "agent".into());
    assert_eq!(a, b);
}

#[test]
fn context_inequality() {
    let a = EventContext::for_agent("ws".into(), "agent-1".into());
    let b = EventContext::for_agent("ws".into(), "agent-2".into());
    assert_ne!(a, b);
}

#[test]
fn context_debug_contains_fields() {
    let c = EventContext::for_agent("debug-ws".into(), "debug-agent".into());
    let debug = format!("{c:?}");
    assert!(debug.contains("debug-ws"));
    assert!(debug.contains("debug-agent"));
}

// === Serde roundtrip for all event variants ===

#[test]
fn serde_roundtrip_workspace_created() {
    let e = IsolateEvent::WorkspaceCreated {
        name: "ws".into(),
        source: "cli".into(),
        context: ctx("ws"),
        timestamp: now(),
    };
    let json = serde_json::to_string(&e).unwrap();
    let parsed: IsolateEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(e, parsed);
}

#[test]
fn serde_roundtrip_workspace_activated() {
    let e = IsolateEvent::WorkspaceActivated {
        name: "ws".into(),
        state: WorkspaceState::Working,
        context: ctx("ws"),
        timestamp: now(),
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
        timestamp: now(),
    };
    let json = serde_json::to_string(&e).unwrap();
    let parsed: IsolateEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(e, parsed);
}

#[test]
fn serde_roundtrip_all_variants() {
    let ts = now();
    let events: Vec<IsolateEvent> = vec![
        IsolateEvent::WorkspaceCreated {
            name: "w".into(),
            source: "cli".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::WorkspaceActivated {
            name: "w".into(),
            state: WorkspaceState::Working,
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::WorkspaceSyncing {
            name: "w".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::WorkspaceSynced {
            name: "w".into(),
            commits_rebased: 3,
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::WorkspacePaused {
            name: "w".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::WorkspaceResumed {
            name: "w".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::WorkspaceCompleted {
            name: "w".into(),
            branch: "b".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::WorkspaceFailed {
            name: "w".into(),
            reason: "err".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::SessionStarted {
            name: "w".into(),
            session_id: "s".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::SessionEnded {
            name: "w".into(),
            session_id: "s".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::AgentClaimed {
            name: "w".into(),
            agent_id: "a".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::AgentReleased {
            name: "w".into(),
            agent_id: "a".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::BranchCreated {
            name: "w".into(),
            branch: "b".into(),
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::BranchPushed {
            name: "w".into(),
            branch: "b".into(),
            commits: 5,
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::RebaseCompleted {
            name: "w".into(),
            commits: 2,
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::ConflictDetected {
            name: "w".into(),
            files: vec!["x.rs".into()],
            context: ctx("w"),
            timestamp: ts,
        },
        IsolateEvent::ConflictResolved {
            name: "w".into(),
            context: ctx("w"),
            timestamp: ts,
        },
    ];
    for event in events {
        let json = serde_json::to_string(&event).unwrap();
        let parsed: IsolateEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(
            event,
            parsed,
            "serde roundtrip failed for {}",
            event.event_name()
        );
    }
}

#[test]
fn serde_tagged_type_field() {
    let e = IsolateEvent::AgentClaimed {
        name: "w".into(),
        agent_id: "a".into(),
        context: ctx("w"),
        timestamp: now(),
    };
    let json = serde_json::to_string(&e).unwrap();
    assert!(
        json.contains("\"type\":\"AgentClaimed\""),
        "should use tagged enum: {json}"
    );
}

// === Clone ===

#[test]
fn event_clone_preserves_data() {
    let e = IsolateEvent::BranchPushed {
        name: "w".into(),
        branch: "b".into(),
        commits: 3,
        context: ctx("w"),
        timestamp: now(),
    };
    let cloned = e.clone();
    assert_eq!(e, cloned);
}

// === Debug ===

#[test]
fn event_debug_contains_variant_name() {
    let e = IsolateEvent::WorkspaceFailed {
        name: "w".into(),
        reason: "timeout".into(),
        context: ctx("w"),
        timestamp: now(),
    };
    let debug = format!("{e:?}");
    assert!(debug.contains("WorkspaceFailed"));
}

// === Proptests ===

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use proptest::prop_assert;

    proptest! {
        #[test]
        fn serde_roundtrip_workspace_created(
            name in "[a-zA-Z0-9_-]{1,20}",
            source in "[a-zA-Z0-9_-]{1,20}"
        ) {
            let e = IsolateEvent::WorkspaceCreated {
                name: name.clone(),
                source: source.clone(),
                context: ctx(&name),
                timestamp: now(),
            };
            let json = serde_json::to_string(&e).unwrap();
            let parsed: IsolateEvent = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(e, parsed);
        }

        #[test]
        fn serde_roundtrip_branch_pushed(
            name in "[a-zA-Z0-9_-]{1,20}",
            branch in "[a-zA-Z0-9_-]{1,20}",
            commits in 0usize..100
        ) {
            let e = IsolateEvent::BranchPushed {
                name: name.clone(),
                branch: branch.clone(),
                commits,
                context: ctx(&name),
                timestamp: now(),
            };
            let json = serde_json::to_string(&e).unwrap();
            let parsed: IsolateEvent = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(e, parsed);
        }

        #[test]
        fn workspace_accessor_matches_name(name in "[a-zA-Z0-9_-]{1,20}") {
            let e = IsolateEvent::WorkspaceSyncing {
                name: name.clone(),
                context: ctx(&name),
                timestamp: now(),
            };
            prop_assert_eq!(e.workspace(), &name);
        }

        #[test]
        fn event_name_is_dot_separated_for_all_variants(idx in 0usize..17) {
            let ts = now();
            let c = ctx("w");
            let events: Vec<IsolateEvent> = vec![
                IsolateEvent::WorkspaceCreated { name: "w".into(), source: "s".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::WorkspaceActivated { name: "w".into(), state: WorkspaceState::Working, context: c.clone(), timestamp: ts },
                IsolateEvent::WorkspaceSyncing { name: "w".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::WorkspaceSynced { name: "w".into(), commits_rebased: 0, context: c.clone(), timestamp: ts },
                IsolateEvent::WorkspacePaused { name: "w".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::WorkspaceResumed { name: "w".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::WorkspaceCompleted { name: "w".into(), branch: "b".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::WorkspaceFailed { name: "w".into(), reason: "r".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::SessionStarted { name: "w".into(), session_id: "s".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::SessionEnded { name: "w".into(), session_id: "s".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::AgentClaimed { name: "w".into(), agent_id: "a".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::AgentReleased { name: "w".into(), agent_id: "a".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::BranchCreated { name: "w".into(), branch: "b".into(), context: c.clone(), timestamp: ts },
                IsolateEvent::BranchPushed { name: "w".into(), branch: "b".into(), commits: 0, context: c.clone(), timestamp: ts },
                IsolateEvent::RebaseCompleted { name: "w".into(), commits: 0, context: c.clone(), timestamp: ts },
                IsolateEvent::ConflictDetected { name: "w".into(), files: vec![], context: c.clone(), timestamp: ts },
                IsolateEvent::ConflictResolved { name: "w".into(), context: c, timestamp: ts },
            ];
            let event = &events[idx % events.len()];
            prop_assert!(event.event_name().contains('.'));
        }

        #[test]
        fn context_serde_roundtrip(
            ws in "[a-zA-Z0-9_-]{1,20}",
            agent in "[a-zA-Z0-9_-]{1,20}",
            session in "[a-zA-Z0-9_-]{1,20}"
        ) {
            let c = EventContext {
                workspace: ws.clone(),
                agent_id: Some(agent.clone()),
                session_id: Some(session.clone()),
            };
            let json = serde_json::to_string(&c).unwrap();
            let parsed: EventContext = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(c, parsed);
        }
    }
}
