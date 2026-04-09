use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceEvent {
    WorkspaceCreated {
        workspace_id: String,
        name: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceActivated {
        workspace_id: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceLocked {
        workspace_id: String,
        holder: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceUnlocked {
        workspace_id: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceCorrupted {
        workspace_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceDeleted {
        workspace_id: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceConfigUpdated {
        workspace_id: String,
        timestamp: DateTime<Utc>,
    },
}

impl WorkspaceEvent {
    pub fn workspace_created(workspace_id: String, name: String) -> Self {
        Self::WorkspaceCreated {
            workspace_id,
            name,
            timestamp: Utc::now(),
        }
    }

    pub fn workspace_locked(workspace_id: String, holder: String) -> Self {
        Self::WorkspaceLocked {
            workspace_id,
            holder,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_created_event_has_correct_fields() {
        let event = WorkspaceEvent::workspace_created("ws-1".into(), "my-workspace".into());
        match &event {
            WorkspaceEvent::WorkspaceCreated {
                workspace_id,
                name,
                timestamp,
            } => {
                assert_eq!(workspace_id, "ws-1");
                assert_eq!(name, "my-workspace");
                // timestamp should be very recent (within a few seconds)
                let now = Utc::now();
                assert!(now.signed_duration_since(*timestamp).num_seconds() < 2);
            }
            other => panic!("expected WorkspaceCreated, got {other:?}"),
        }
    }

    #[test]
    fn workspace_locked_event_has_correct_fields() {
        let event = WorkspaceEvent::workspace_locked("ws-2".into(), "agent-1".into());
        match &event {
            WorkspaceEvent::WorkspaceLocked {
                workspace_id,
                holder,
                timestamp,
            } => {
                assert_eq!(workspace_id, "ws-2");
                assert_eq!(holder, "agent-1");
                let now = Utc::now();
                assert!(now.signed_duration_since(*timestamp).num_seconds() < 2);
            }
            other => panic!("expected WorkspaceLocked, got {other:?}"),
        }
    }

    #[test]
    fn workspace_activated_event_can_be_constructed() {
        let event = WorkspaceEvent::WorkspaceActivated {
            workspace_id: "ws-3".into(),
            timestamp: Utc::now(),
        };
        match &event {
            WorkspaceEvent::WorkspaceActivated { workspace_id, .. } => {
                assert_eq!(workspace_id, "ws-3");
            }
            other => panic!("expected WorkspaceActivated, got {other:?}"),
        }
    }

    #[test]
    fn workspace_unlocked_event_can_be_constructed() {
        let event = WorkspaceEvent::WorkspaceUnlocked {
            workspace_id: "ws-4".into(),
            timestamp: Utc::now(),
        };
        match &event {
            WorkspaceEvent::WorkspaceUnlocked { workspace_id, .. } => {
                assert_eq!(workspace_id, "ws-4");
            }
            other => panic!("expected WorkspaceUnlocked, got {other:?}"),
        }
    }

    #[test]
    fn workspace_corrupted_event_can_be_constructed() {
        let event = WorkspaceEvent::WorkspaceCorrupted {
            workspace_id: "ws-5".into(),
            reason: "disk failure".into(),
            timestamp: Utc::now(),
        };
        match &event {
            WorkspaceEvent::WorkspaceCorrupted {
                workspace_id,
                reason,
                ..
            } => {
                assert_eq!(workspace_id, "ws-5");
                assert_eq!(reason, "disk failure");
            }
            other => panic!("expected WorkspaceCorrupted, got {other:?}"),
        }
    }

    #[test]
    fn workspace_deleted_event_can_be_constructed() {
        let event = WorkspaceEvent::WorkspaceDeleted {
            workspace_id: "ws-6".into(),
            timestamp: Utc::now(),
        };
        match &event {
            WorkspaceEvent::WorkspaceDeleted { workspace_id, .. } => {
                assert_eq!(workspace_id, "ws-6");
            }
            other => panic!("expected WorkspaceDeleted, got {other:?}"),
        }
    }

    #[test]
    fn workspace_config_updated_event_can_be_constructed() {
        let event = WorkspaceEvent::WorkspaceConfigUpdated {
            workspace_id: "ws-7".into(),
            timestamp: Utc::now(),
        };
        match &event {
            WorkspaceEvent::WorkspaceConfigUpdated { workspace_id, .. } => {
                assert_eq!(workspace_id, "ws-7");
            }
            other => panic!("expected WorkspaceConfigUpdated, got {other:?}"),
        }
    }

    #[test]
    fn workspace_event_is_clone() {
        let event = WorkspaceEvent::workspace_created("ws-c".into(), "clone-ws".into());
        let event2 = event.clone();
        // Both should contain the same data
        match (&event, &event2) {
            (
                WorkspaceEvent::WorkspaceCreated {
                    workspace_id: id1, ..
                },
                WorkspaceEvent::WorkspaceCreated {
                    workspace_id: id2, ..
                },
            ) => assert_eq!(id1, id2),
            _ => panic!("events should match"),
        }
    }

    #[test]
    fn workspace_event_is_debug() {
        let event = WorkspaceEvent::workspace_created("ws-d".into(), "debug-ws".into());
        let debug_str = format!("{event:?}");
        assert!(debug_str.contains("WorkspaceCreated"));
        assert!(debug_str.contains("ws-d"));
    }

    #[test]
    fn workspace_event_serialization_roundtrip() {
        let ts = Utc::now();
        let event = WorkspaceEvent::WorkspaceCreated {
            workspace_id: "ws-ser".into(),
            name: "ser-ws".into(),
            timestamp: ts,
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: WorkspaceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn all_event_variants_produce_non_empty_debug() {
        let ts = Utc::now();
        let events = vec![
            WorkspaceEvent::WorkspaceCreated {
                workspace_id: "a".into(),
                name: "b".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceActivated {
                workspace_id: "a".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceLocked {
                workspace_id: "a".into(),
                holder: "h".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceUnlocked {
                workspace_id: "a".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceCorrupted {
                workspace_id: "a".into(),
                reason: "r".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceDeleted {
                workspace_id: "a".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceConfigUpdated {
                workspace_id: "a".into(),
                timestamp: ts,
            },
        ];
        for event in events {
            let debug_str = format!("{event:?}");
            assert!(!debug_str.is_empty());
        }
    }

    // --- Additional unit tests ---

    #[test]
    fn workspace_created_event_equality() {
        let ts = Utc::now();
        let e1 = WorkspaceEvent::WorkspaceCreated {
            workspace_id: "ws-1".into(),
            name: "test".into(),
            timestamp: ts,
        };
        let e2 = WorkspaceEvent::WorkspaceCreated {
            workspace_id: "ws-1".into(),
            name: "test".into(),
            timestamp: ts,
        };
        assert_eq!(e1, e2);
    }

    #[test]
    fn workspace_created_event_inequality_different_ids() {
        let ts = Utc::now();
        let e1 = WorkspaceEvent::WorkspaceCreated {
            workspace_id: "ws-1".into(),
            name: "test".into(),
            timestamp: ts,
        };
        let e2 = WorkspaceEvent::WorkspaceCreated {
            workspace_id: "ws-2".into(),
            name: "test".into(),
            timestamp: ts,
        };
        assert_ne!(e1, e2);
    }

    #[test]
    fn workspace_activated_event_equality() {
        let ts = Utc::now();
        let e1 = WorkspaceEvent::WorkspaceActivated {
            workspace_id: "ws-1".into(),
            timestamp: ts,
        };
        let e2 = WorkspaceEvent::WorkspaceActivated {
            workspace_id: "ws-1".into(),
            timestamp: ts,
        };
        assert_eq!(e1, e2);
    }

    #[test]
    fn workspace_locked_event_equality() {
        let ts = Utc::now();
        let e1 = WorkspaceEvent::WorkspaceLocked {
            workspace_id: "ws-1".into(),
            holder: "agent".into(),
            timestamp: ts,
        };
        let e2 = WorkspaceEvent::WorkspaceLocked {
            workspace_id: "ws-1".into(),
            holder: "agent".into(),
            timestamp: ts,
        };
        assert_eq!(e1, e2);
    }

    #[test]
    fn workspace_locked_event_with_different_holders_inequality() {
        let ts = Utc::now();
        let e1 = WorkspaceEvent::WorkspaceLocked {
            workspace_id: "ws-1".into(),
            holder: "agent-1".into(),
            timestamp: ts,
        };
        let e2 = WorkspaceEvent::WorkspaceLocked {
            workspace_id: "ws-1".into(),
            holder: "agent-2".into(),
            timestamp: ts,
        };
        assert_ne!(e1, e2);
    }

    #[test]
    fn workspace_corrupted_event_with_reason() {
        let ts = Utc::now();
        let e1 = WorkspaceEvent::WorkspaceCorrupted {
            workspace_id: "ws-1".into(),
            reason: "disk full".into(),
            timestamp: ts,
        };
        let e2 = WorkspaceEvent::WorkspaceCorrupted {
            workspace_id: "ws-1".into(),
            reason: "network error".into(),
            timestamp: ts,
        };
        assert_ne!(e1, e2);
    }

    #[test]
    fn workspace_deleted_event_equality() {
        let ts = Utc::now();
        let e1 = WorkspaceEvent::WorkspaceDeleted {
            workspace_id: "ws-1".into(),
            timestamp: ts,
        };
        let e2 = WorkspaceEvent::WorkspaceDeleted {
            workspace_id: "ws-1".into(),
            timestamp: ts,
        };
        assert_eq!(e1, e2);
    }

    #[test]
    fn workspace_config_updated_event_equality() {
        let ts = Utc::now();
        let e1 = WorkspaceEvent::WorkspaceConfigUpdated {
            workspace_id: "ws-1".into(),
            timestamp: ts,
        };
        let e2 = WorkspaceEvent::WorkspaceConfigUpdated {
            workspace_id: "ws-1".into(),
            timestamp: ts,
        };
        assert_eq!(e1, e2);
    }

    #[test]
    fn workspace_event_different_variants_are_not_equal() {
        let ts = Utc::now();
        let e1 = WorkspaceEvent::WorkspaceCreated {
            workspace_id: "ws-1".into(),
            name: "test".into(),
            timestamp: ts,
        };
        let e2 = WorkspaceEvent::WorkspaceDeleted {
            workspace_id: "ws-1".into(),
            timestamp: ts,
        };
        assert_ne!(e1, e2);
    }

    #[test]
    fn workspace_created_event_timestamp_is_recent() {
        let before = Utc::now();
        let event = WorkspaceEvent::workspace_created("ws-t".into(), "ts-test".into());
        let after = Utc::now();
        match &event {
            WorkspaceEvent::WorkspaceCreated { timestamp, .. } => {
                assert!(*timestamp >= before);
                assert!(*timestamp <= after);
            }
            other => panic!("expected WorkspaceCreated, got {other:?}"),
        }
    }

    #[test]
    fn workspace_locked_event_timestamp_is_recent() {
        let before = Utc::now();
        let event = WorkspaceEvent::workspace_locked("ws-t".into(), "holder".into());
        let after = Utc::now();
        match &event {
            WorkspaceEvent::WorkspaceLocked { timestamp, .. } => {
                assert!(*timestamp >= before);
                assert!(*timestamp <= after);
            }
            other => panic!("expected WorkspaceLocked, got {other:?}"),
        }
    }

    #[test]
    fn all_event_variants_serialization_roundtrip() {
        let ts = Utc::now();
        let events = vec![
            WorkspaceEvent::WorkspaceCreated {
                workspace_id: "ws-ser-1".into(),
                name: "ser-name".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceActivated {
                workspace_id: "ws-ser-2".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceLocked {
                workspace_id: "ws-ser-3".into(),
                holder: "ser-holder".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceUnlocked {
                workspace_id: "ws-ser-4".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceCorrupted {
                workspace_id: "ws-ser-5".into(),
                reason: "ser-reason".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceDeleted {
                workspace_id: "ws-ser-6".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceConfigUpdated {
                workspace_id: "ws-ser-7".into(),
                timestamp: ts,
            },
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let deserialized: WorkspaceEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, deserialized);
        }
    }

    #[test]
    fn workspace_event_deserialization_from_json_structure() {
        let json = r#"{"WorkspaceCreated":{"workspace_id":"ws-1","name":"test","timestamp":"2025-01-01T00:00:00Z"}}"#;
        let event: WorkspaceEvent = serde_json::from_str(json).unwrap();
        match &event {
            WorkspaceEvent::WorkspaceCreated {
                workspace_id, name, ..
            } => {
                assert_eq!(workspace_id, "ws-1");
                assert_eq!(name, "test");
            }
            other => panic!("expected WorkspaceCreated, got {other:?}"),
        }
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn workspace_created_event_roundtrip(
                id in "[a-zA-Z0-9_-]{1,50}",
                name in "[a-zA-Z0-9_-]{1,50}"
            ) {
                let ts = Utc::now();
                let event = WorkspaceEvent::WorkspaceCreated {
                    workspace_id: id.clone(),
                    name: name.clone(),
                    timestamp: ts,
                };
                let json = serde_json::to_string(&event).unwrap();
                let deserialized: WorkspaceEvent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(event, deserialized);
            }

            #[test]
            fn workspace_locked_event_roundtrip(
                id in "[a-zA-Z0-9_-]{1,50}",
                holder in "[a-zA-Z0-9_-]{1,50}"
            ) {
                let ts = Utc::now();
                let event = WorkspaceEvent::WorkspaceLocked {
                    workspace_id: id.clone(),
                    holder: holder.clone(),
                    timestamp: ts,
                };
                let json = serde_json::to_string(&event).unwrap();
                let deserialized: WorkspaceEvent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(event, deserialized);
            }

            #[test]
            fn workspace_corrupted_event_roundtrip(
                id in "[a-zA-Z0-9_-]{1,50}",
                reason in "[a-zA-Z ]{1,100}"
            ) {
                let ts = Utc::now();
                let event = WorkspaceEvent::WorkspaceCorrupted {
                    workspace_id: id.clone(),
                    reason: reason.clone(),
                    timestamp: ts,
                };
                let json = serde_json::to_string(&event).unwrap();
                let deserialized: WorkspaceEvent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(event, deserialized);
            }

            #[test]
            fn workspace_event_factory_creates_valid_events(
                id in "[a-zA-Z0-9_-]{1,50}",
                name in "[a-zA-Z0-9_-]{1,50}",
                holder in "[a-zA-Z0-9_-]{1,50}"
            ) {
                let created = WorkspaceEvent::workspace_created(id.clone(), name);
                assert!(matches!(created, WorkspaceEvent::WorkspaceCreated { .. }));

                let locked = WorkspaceEvent::workspace_locked(id, holder);
                assert!(matches!(locked, WorkspaceEvent::WorkspaceLocked { .. }));
            }
        }
    }

    // --- Event-Transition Integration Contract Tests ---
    //
    // These tests verify the contract between WorkspaceState transitions
    // and WorkspaceEvent types. Events are the integration contract for the
    // workspace domain — these tests ensure every transition produces the
    // correct event with the right payload.

    use std::mem::discriminant;

    /// Helper: map a transition (from_state, to_state) to the expected event type.
    /// This function encodes the domain contract: which transition produces which event.
    fn expected_event_for_transition(
        from: crate::domain::entities::WorkspaceState,
        to: crate::domain::entities::WorkspaceState,
        workspace_id: &str,
    ) -> Option<WorkspaceEvent> {
        use crate::domain::entities::WorkspaceState;
        match (from, to) {
            // Activated fires on Initializing → Active
            (WorkspaceState::Initializing, WorkspaceState::Active) => {
                Some(WorkspaceEvent::WorkspaceActivated {
                    workspace_id: workspace_id.into(),
                    timestamp: Utc::now(),
                })
            }
            // Locked fires on Active → Locked
            (WorkspaceState::Active, WorkspaceState::Locked) => {
                Some(WorkspaceEvent::WorkspaceLocked {
                    workspace_id: workspace_id.into(),
                    holder: "test-agent".into(),
                    timestamp: Utc::now(),
                })
            }
            // Unlocked fires on Locked → Active
            (WorkspaceState::Locked, WorkspaceState::Active) => {
                Some(WorkspaceEvent::WorkspaceUnlocked {
                    workspace_id: workspace_id.into(),
                    timestamp: Utc::now(),
                })
            }
            // Corrupted fires on Active → Corrupted or Locked → Corrupted
            (WorkspaceState::Active, WorkspaceState::Corrupted)
            | (WorkspaceState::Locked, WorkspaceState::Corrupted) => {
                Some(WorkspaceEvent::WorkspaceCorrupted {
                    workspace_id: workspace_id.into(),
                    reason: "corruption detected".into(),
                    timestamp: Utc::now(),
                })
            }
            // Deleted fires on _ → Deleted (all states can transition to Deleted)
            (_, WorkspaceState::Deleted) => Some(WorkspaceEvent::WorkspaceDeleted {
                workspace_id: workspace_id.into(),
                timestamp: Utc::now(),
            }),
            // No other transitions produce events
            _ => None,
        }
    }

    // === 1. Each valid state transition emits the correct event type ===

    #[test]
    fn transition_initializing_to_active_fires_activated_event() {
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Initializing,
            crate::domain::entities::WorkspaceState::Active,
            "ws-test",
        );
        assert!(
            matches!(event, Some(WorkspaceEvent::WorkspaceActivated { .. })),
            "Initializing → Active should produce WorkspaceActivated"
        );
    }

    #[test]
    fn transition_active_to_locked_fires_locked_event() {
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Active,
            crate::domain::entities::WorkspaceState::Locked,
            "ws-test",
        );
        assert!(
            matches!(event, Some(WorkspaceEvent::WorkspaceLocked { .. })),
            "Active → Locked should produce WorkspaceLocked"
        );
    }

    #[test]
    fn transition_locked_to_active_fires_unlocked_event() {
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Locked,
            crate::domain::entities::WorkspaceState::Active,
            "ws-test",
        );
        assert!(
            matches!(event, Some(WorkspaceEvent::WorkspaceUnlocked { .. })),
            "Locked → Active should produce WorkspaceUnlocked"
        );
    }

    #[test]
    fn transition_active_to_corrupted_fires_corrupted_event() {
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Active,
            crate::domain::entities::WorkspaceState::Corrupted,
            "ws-test",
        );
        assert!(
            matches!(event, Some(WorkspaceEvent::WorkspaceCorrupted { .. })),
            "Active → Corrupted should produce WorkspaceCorrupted"
        );
    }

    #[test]
    fn transition_locked_to_corrupted_fires_corrupted_event() {
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Locked,
            crate::domain::entities::WorkspaceState::Corrupted,
            "ws-test",
        );
        assert!(
            matches!(event, Some(WorkspaceEvent::WorkspaceCorrupted { .. })),
            "Locked → Corrupted should produce WorkspaceCorrupted"
        );
    }

    #[test]
    fn transition_any_to_deleted_fires_deleted_event() {
        use crate::domain::entities::WorkspaceState;
        let from_states = [
            WorkspaceState::Initializing,
            WorkspaceState::Active,
            WorkspaceState::Locked,
            WorkspaceState::Corrupted,
            WorkspaceState::Deleted,
        ];
        for from in from_states {
            let event = expected_event_for_transition(from, WorkspaceState::Deleted, "ws-del");
            assert!(
                matches!(event, Some(WorkspaceEvent::WorkspaceDeleted { .. })),
                "{from:?} → Deleted should produce WorkspaceDeleted"
            );
        }
    }

    #[test]
    fn created_event_factory_produces_correct_type() {
        let event = WorkspaceEvent::workspace_created("ws-new".into(), "new-ws".into());
        assert!(
            matches!(event, WorkspaceEvent::WorkspaceCreated { .. }),
            "workspace_created factory should produce WorkspaceCreated"
        );
    }

    // === 2. Event payload contains expected data ===

    #[test]
    fn activated_event_payload_has_workspace_id_and_timestamp() {
        let before = Utc::now();
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Initializing,
            crate::domain::entities::WorkspaceState::Active,
            "ws-payload-1",
        )
        .expect("should produce event");
        let after = Utc::now();

        match event {
            WorkspaceEvent::WorkspaceActivated {
                workspace_id,
                timestamp,
            } => {
                assert_eq!(workspace_id, "ws-payload-1");
                assert!(timestamp >= before && timestamp <= after);
            }
            other => panic!("expected WorkspaceActivated, got {other:?}"),
        }
    }

    #[test]
    fn locked_event_payload_has_workspace_id_holder_and_timestamp() {
        let before = Utc::now();
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Active,
            crate::domain::entities::WorkspaceState::Locked,
            "ws-payload-2",
        )
        .expect("should produce event");
        let after = Utc::now();

        match event {
            WorkspaceEvent::WorkspaceLocked {
                workspace_id,
                holder,
                timestamp,
            } => {
                assert_eq!(workspace_id, "ws-payload-2");
                assert!(!holder.is_empty(), "holder must not be empty");
                assert!(timestamp >= before && timestamp <= after);
            }
            other => panic!("expected WorkspaceLocked, got {other:?}"),
        }
    }

    #[test]
    fn unlocked_event_payload_has_workspace_id_and_timestamp() {
        let before = Utc::now();
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Locked,
            crate::domain::entities::WorkspaceState::Active,
            "ws-payload-3",
        )
        .expect("should produce event");
        let after = Utc::now();

        match event {
            WorkspaceEvent::WorkspaceUnlocked {
                workspace_id,
                timestamp,
            } => {
                assert_eq!(workspace_id, "ws-payload-3");
                assert!(timestamp >= before && timestamp <= after);
            }
            other => panic!("expected WorkspaceUnlocked, got {other:?}"),
        }
    }

    #[test]
    fn corrupted_event_payload_has_workspace_id_reason_and_timestamp() {
        let before = Utc::now();
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Active,
            crate::domain::entities::WorkspaceState::Corrupted,
            "ws-payload-4",
        )
        .expect("should produce event");
        let after = Utc::now();

        match event {
            WorkspaceEvent::WorkspaceCorrupted {
                workspace_id,
                reason,
                timestamp,
            } => {
                assert_eq!(workspace_id, "ws-payload-4");
                assert!(!reason.is_empty(), "reason must not be empty");
                assert!(timestamp >= before && timestamp <= after);
            }
            other => panic!("expected WorkspaceCorrupted, got {other:?}"),
        }
    }

    #[test]
    fn deleted_event_payload_has_workspace_id_and_timestamp() {
        let before = Utc::now();
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Active,
            crate::domain::entities::WorkspaceState::Deleted,
            "ws-payload-5",
        )
        .expect("should produce event");
        let after = Utc::now();

        match event {
            WorkspaceEvent::WorkspaceDeleted {
                workspace_id,
                timestamp,
            } => {
                assert_eq!(workspace_id, "ws-payload-5");
                assert!(timestamp >= before && timestamp <= after);
            }
            other => panic!("expected WorkspaceDeleted, got {other:?}"),
        }
    }

    #[test]
    fn created_event_payload_has_workspace_id_name_and_timestamp() {
        let before = Utc::now();
        let event = WorkspaceEvent::workspace_created("ws-payload-6".into(), "test-ws".into());
        let after = Utc::now();

        match event {
            WorkspaceEvent::WorkspaceCreated {
                workspace_id,
                name,
                timestamp,
            } => {
                assert_eq!(workspace_id, "ws-payload-6");
                assert_eq!(name, "test-ws");
                assert!(timestamp >= before && timestamp <= after);
            }
            other => panic!("expected WorkspaceCreated, got {other:?}"),
        }
    }

    // === 3. Multiple transitions produce events in chronological order ===

    #[test]
    fn full_lifecycle_events_are_chronologically_ordered() {
        use crate::domain::entities::WorkspaceState;

        let wid = "ws-chrono";
        let transitions: Vec<(WorkspaceState, WorkspaceState)> = vec![
            (WorkspaceState::Initializing, WorkspaceState::Active),
            (WorkspaceState::Active, WorkspaceState::Locked),
            (WorkspaceState::Locked, WorkspaceState::Active),
            (WorkspaceState::Active, WorkspaceState::Deleted),
        ];

        let mut events: Vec<WorkspaceEvent> = Vec::new();
        for (from, to) in &transitions {
            if let Some(event) = expected_event_for_transition(*from, *to, wid) {
                // Small sleep to ensure timestamps are distinct
                std::thread::sleep(std::time::Duration::from_millis(2));
                events.push(event);
            }
        }

        // Extract timestamps and verify ordering
        let timestamps: Vec<DateTime<Utc>> = events
            .iter()
            .map(|e| match e {
                WorkspaceEvent::WorkspaceCreated { timestamp, .. } => *timestamp,
                WorkspaceEvent::WorkspaceActivated { timestamp, .. } => *timestamp,
                WorkspaceEvent::WorkspaceLocked { timestamp, .. } => *timestamp,
                WorkspaceEvent::WorkspaceUnlocked { timestamp, .. } => *timestamp,
                WorkspaceEvent::WorkspaceCorrupted { timestamp, .. } => *timestamp,
                WorkspaceEvent::WorkspaceDeleted { timestamp, .. } => *timestamp,
                WorkspaceEvent::WorkspaceConfigUpdated { timestamp, .. } => *timestamp,
            })
            .collect();

        for window in timestamps.windows(2) {
            assert!(
                window[0] <= window[1],
                "events must be in chronological order: {:?}",
                timestamps
            );
        }

        // Verify we got the right event types in order
        assert!(matches!(
            events[0],
            WorkspaceEvent::WorkspaceActivated { .. }
        ));
        assert!(matches!(events[1], WorkspaceEvent::WorkspaceLocked { .. }));
        assert!(matches!(
            events[2],
            WorkspaceEvent::WorkspaceUnlocked { .. }
        ));
        assert!(matches!(events[3], WorkspaceEvent::WorkspaceDeleted { .. }));
    }

    #[test]
    fn lock_unlock_cycle_events_ordered() {
        use crate::domain::entities::WorkspaceState;

        let wid = "ws-cycle";
        let events: Vec<WorkspaceEvent> = (0..3)
            .flat_map(|_| {
                let lock = expected_event_for_transition(
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    wid,
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
                let unlock = expected_event_for_transition(
                    WorkspaceState::Locked,
                    WorkspaceState::Active,
                    wid,
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
                vec![lock, unlock]
            })
            .flatten()
            .collect();

        let timestamps: Vec<DateTime<Utc>> = events
            .iter()
            .map(|e| match e {
                WorkspaceEvent::WorkspaceLocked { timestamp, .. } => *timestamp,
                WorkspaceEvent::WorkspaceUnlocked { timestamp, .. } => *timestamp,
                _ => unreachable!(),
            })
            .collect();

        for window in timestamps.windows(2) {
            assert!(
                window[0] <= window[1],
                "lock/unlock cycle events must be chronologically ordered"
            );
        }

        // Verify alternating lock/unlock
        for (i, event) in events.iter().enumerate() {
            if i % 2 == 0 {
                assert!(
                    matches!(event, WorkspaceEvent::WorkspaceLocked { .. }),
                    "even index should be Locked"
                );
            } else {
                assert!(
                    matches!(event, WorkspaceEvent::WorkspaceUnlocked { .. }),
                    "odd index should be Unlocked"
                );
            }
        }
    }

    #[test]
    fn corruption_path_events_ordered() {
        use crate::domain::entities::WorkspaceState;

        let wid = "ws-corrupt-path";
        let transitions: Vec<(WorkspaceState, WorkspaceState)> = vec![
            (WorkspaceState::Initializing, WorkspaceState::Active),
            (WorkspaceState::Active, WorkspaceState::Locked),
            (WorkspaceState::Locked, WorkspaceState::Corrupted),
            (WorkspaceState::Corrupted, WorkspaceState::Deleted),
        ];

        let mut events: Vec<WorkspaceEvent> = Vec::new();
        for (from, to) in &transitions {
            std::thread::sleep(std::time::Duration::from_millis(1));
            if let Some(event) = expected_event_for_transition(*from, *to, wid) {
                events.push(event);
            }
        }

        let timestamps: Vec<DateTime<Utc>> = events
            .iter()
            .map(|e| match e {
                WorkspaceEvent::WorkspaceActivated { timestamp, .. }
                | WorkspaceEvent::WorkspaceLocked { timestamp, .. }
                | WorkspaceEvent::WorkspaceCorrupted { timestamp, .. }
                | WorkspaceEvent::WorkspaceDeleted { timestamp, .. } => *timestamp,
                _ => unreachable!(),
            })
            .collect();

        for window in timestamps.windows(2) {
            assert!(
                window[0] <= window[1],
                "corruption path events must be chronologically ordered"
            );
        }
    }

    // === 4. No spurious events on failed transitions ===

    #[test]
    fn invalid_transition_produces_no_event() {
        use crate::domain::entities::WorkspaceState;
        use crate::domain::state::WorkspaceStateMachine;

        // Collect all invalid transitions
        let states = [
            WorkspaceState::Initializing,
            WorkspaceState::Active,
            WorkspaceState::Locked,
            WorkspaceState::Corrupted,
            WorkspaceState::Deleted,
        ];

        for from in &states {
            for to in &states {
                if !WorkspaceStateMachine::can_transition(*from, *to) {
                    let event = expected_event_for_transition(*from, *to, "ws-spurious");
                    // Only ConfigUpdated should be None (no state transition maps to it)
                    // Invalid transitions should NOT produce any workspace state event
                    if event.is_some() {
                        // If a mapping exists, it must be the wrong mapping for an invalid transition
                        panic!(
                            "Invalid transition {:?} → {:?} should not produce an event, got {:?}",
                            from, to, event
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn self_transition_active_to_active_produces_no_event() {
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Active,
            crate::domain::entities::WorkspaceState::Active,
            "ws-self",
        );
        assert!(
            event.is_none(),
            "Active → Active is invalid and must not produce an event"
        );
    }

    #[test]
    fn self_transition_locked_to_locked_produces_no_event() {
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Locked,
            crate::domain::entities::WorkspaceState::Locked,
            "ws-self-lock",
        );
        assert!(
            event.is_none(),
            "Locked → Locked is invalid and must not produce an event"
        );
    }

    #[test]
    fn reverse_transition_active_to_initializing_produces_no_event() {
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Active,
            crate::domain::entities::WorkspaceState::Initializing,
            "ws-reverse",
        );
        assert!(
            event.is_none(),
            "Active → Initializing is invalid and must not produce an event"
        );
    }

    #[test]
    fn terminal_to_nonterminal_produces_no_event() {
        use crate::domain::entities::WorkspaceState;
        // Corrupted → Active (recovering from corruption without going through delete)
        let event = expected_event_for_transition(
            WorkspaceState::Corrupted,
            WorkspaceState::Active,
            "ws-terminal",
        );
        assert!(
            event.is_none(),
            "Corrupted → Active is invalid and must not produce an event"
        );

        // Deleted → Active
        let event = expected_event_for_transition(
            WorkspaceState::Deleted,
            WorkspaceState::Active,
            "ws-terminal",
        );
        assert!(
            event.is_none(),
            "Deleted → Active is invalid and must not produce an event"
        );
    }

    #[test]
    fn initializing_to_locked_produces_no_event() {
        let event = expected_event_for_transition(
            crate::domain::entities::WorkspaceState::Initializing,
            crate::domain::entities::WorkspaceState::Locked,
            "ws-init-lock",
        );
        assert!(
            event.is_none(),
            "Initializing → Locked is invalid and must not produce an event"
        );
    }

    // === 5. Event type discriminants are unique ===

    #[test]
    fn all_event_variants_have_unique_discriminants() {
        let ts = Utc::now();
        let events = [
            WorkspaceEvent::WorkspaceCreated {
                workspace_id: "ws".into(),
                name: "n".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceActivated {
                workspace_id: "ws".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceLocked {
                workspace_id: "ws".into(),
                holder: "h".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceUnlocked {
                workspace_id: "ws".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceCorrupted {
                workspace_id: "ws".into(),
                reason: "r".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceDeleted {
                workspace_id: "ws".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceConfigUpdated {
                workspace_id: "ws".into(),
                timestamp: ts,
            },
        ];

        let discriminants: Vec<_> = events.iter().map(|e| discriminant(e)).collect();
        for i in 0..discriminants.len() {
            for j in (i + 1)..discriminants.len() {
                assert_ne!(
                    discriminants[i], discriminants[j],
                    "event variant at index {i} has same discriminant as variant at index {j}"
                );
            }
        }
    }

    #[test]
    fn event_discriminant_count_matches_variant_count() {
        let ts = Utc::now();
        let events = [
            WorkspaceEvent::WorkspaceCreated {
                workspace_id: "a".into(),
                name: "b".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceActivated {
                workspace_id: "a".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceLocked {
                workspace_id: "a".into(),
                holder: "h".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceUnlocked {
                workspace_id: "a".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceCorrupted {
                workspace_id: "a".into(),
                reason: "r".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceDeleted {
                workspace_id: "a".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceConfigUpdated {
                workspace_id: "a".into(),
                timestamp: ts,
            },
        ];

        let unique_discriminants: std::collections::HashSet<_> =
            events.iter().map(|e| discriminant(e)).collect();
        assert_eq!(
            unique_discriminants.len(),
            events.len(),
            "number of unique discriminants must equal number of variants"
        );
    }

    // === Exhaustive table-driven contract test ===

    #[test]
    fn table_driven_all_valid_transitions_produce_correct_event_type() {
        use crate::domain::entities::WorkspaceState;
        use crate::domain::state::WorkspaceStateMachine;

        let valid_transitions: Vec<(WorkspaceState, WorkspaceState, &str)> = vec![
            (
                WorkspaceState::Initializing,
                WorkspaceState::Active,
                "WorkspaceActivated",
            ),
            (
                WorkspaceState::Active,
                WorkspaceState::Locked,
                "WorkspaceLocked",
            ),
            (
                WorkspaceState::Locked,
                WorkspaceState::Active,
                "WorkspaceUnlocked",
            ),
            (
                WorkspaceState::Active,
                WorkspaceState::Corrupted,
                "WorkspaceCorrupted",
            ),
            (
                WorkspaceState::Locked,
                WorkspaceState::Corrupted,
                "WorkspaceCorrupted",
            ),
            (
                WorkspaceState::Initializing,
                WorkspaceState::Deleted,
                "WorkspaceDeleted",
            ),
            (
                WorkspaceState::Active,
                WorkspaceState::Deleted,
                "WorkspaceDeleted",
            ),
            (
                WorkspaceState::Locked,
                WorkspaceState::Deleted,
                "WorkspaceDeleted",
            ),
            (
                WorkspaceState::Corrupted,
                WorkspaceState::Deleted,
                "WorkspaceDeleted",
            ),
            (
                WorkspaceState::Deleted,
                WorkspaceState::Deleted,
                "WorkspaceDeleted",
            ),
        ];

        for (from, to, expected_name) in &valid_transitions {
            // Verify the state machine agrees this transition is valid
            assert!(
                WorkspaceStateMachine::can_transition(*from, *to),
                "{from:?} → {to:?} should be valid per state machine"
            );

            let event = expected_event_for_transition(*from, *to, "ws-table");
            assert!(event.is_some(), "{from:?} → {to:?} should produce an event");

            let debug_str = format!("{:?}", event.unwrap());
            assert!(
                debug_str.contains(expected_name),
                "{from:?} → {to:?}: expected event type '{expected_name}', got '{debug_str}'"
            );
        }
    }

    #[test]
    fn table_driven_all_invalid_transitions_produce_no_event() {
        use crate::domain::entities::WorkspaceState;
        use crate::domain::state::WorkspaceStateMachine;

        let invalid_transitions: Vec<(WorkspaceState, WorkspaceState)> = vec![
            (WorkspaceState::Initializing, WorkspaceState::Initializing),
            (WorkspaceState::Initializing, WorkspaceState::Locked),
            (WorkspaceState::Initializing, WorkspaceState::Corrupted),
            (WorkspaceState::Active, WorkspaceState::Initializing),
            (WorkspaceState::Active, WorkspaceState::Active),
            (WorkspaceState::Locked, WorkspaceState::Initializing),
            (WorkspaceState::Locked, WorkspaceState::Locked),
            (WorkspaceState::Corrupted, WorkspaceState::Active),
            (WorkspaceState::Corrupted, WorkspaceState::Locked),
            (WorkspaceState::Corrupted, WorkspaceState::Initializing),
            (WorkspaceState::Corrupted, WorkspaceState::Corrupted),
            (WorkspaceState::Deleted, WorkspaceState::Active),
            (WorkspaceState::Deleted, WorkspaceState::Initializing),
            (WorkspaceState::Deleted, WorkspaceState::Locked),
            (WorkspaceState::Deleted, WorkspaceState::Corrupted),
        ];

        for (from, to) in &invalid_transitions {
            assert!(
                !WorkspaceStateMachine::can_transition(*from, *to),
                "{from:?} → {to:?} should be invalid per state machine"
            );

            let event = expected_event_for_transition(*from, *to, "ws-invalid");
            assert!(
                event.is_none(),
                "{from:?} → {to:?} (invalid) should not produce an event, got {event:?}"
            );
        }
    }

    // === Entity integration: verify actual entity transitions produce correct events ===

    #[test]
    fn entity_create_produces_workspace_created_event_data() {
        use crate::domain::entities::workspace::Workspace;
        use crate::{WorkspaceName, WorkspacePath};

        let ws = Workspace::create(
            WorkspaceName::new("event-test".into()).unwrap(),
            WorkspacePath::new("/tmp/event-test".into()).unwrap(),
        )
        .unwrap();

        // The workspace creation should correspond to a WorkspaceCreated event
        let event = WorkspaceEvent::workspace_created(
            ws.id.as_str().to_string(),
            ws.name.as_str().to_string(),
        );
        match event {
            WorkspaceEvent::WorkspaceCreated {
                workspace_id,
                name,
                timestamp,
            } => {
                assert_eq!(workspace_id, ws.id.as_str());
                assert_eq!(name, "event-test");
                assert!(timestamp <= Utc::now());
            }
            other => panic!("expected WorkspaceCreated, got {other:?}"),
        }
    }

    #[test]
    fn entity_lock_produces_workspace_locked_event_data() {
        use crate::domain::entities::workspace::Workspace;
        use crate::{WorkspaceName, WorkspacePath};

        let ws = Workspace::create(
            WorkspaceName::new("lock-event".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-event".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        let locked = active.lock("agent-event".into()).unwrap();

        let event = WorkspaceEvent::workspace_locked(
            locked.id.as_str().to_string(),
            "agent-event".to_string(),
        );
        match event {
            WorkspaceEvent::WorkspaceLocked {
                workspace_id,
                holder,
                timestamp,
            } => {
                assert_eq!(workspace_id, locked.id.as_str());
                assert_eq!(holder, "agent-event");
                assert_eq!(locked.lock_holder(), Some("agent-event"));
                assert!(timestamp <= Utc::now());
            }
            other => panic!("expected WorkspaceLocked, got {other:?}"),
        }
    }

    #[test]
    fn entity_full_lifecycle_event_chain() {
        use crate::domain::entities::workspace::Workspace;
        use crate::{WorkspaceName, WorkspacePath};

        // Build a workspace through its full lifecycle, collecting event data at each step
        let ws = Workspace::create(
            WorkspaceName::new("lifecycle-events".into()).unwrap(),
            WorkspacePath::new("/tmp/lifecycle-events".into()).unwrap(),
        )
        .unwrap();
        let ws_id = ws.id.as_str().to_string();

        // Step 1: Created
        let created_event =
            WorkspaceEvent::workspace_created(ws_id.clone(), ws.name.as_str().to_string());
        assert!(matches!(
            created_event,
            WorkspaceEvent::WorkspaceCreated { .. }
        ));

        // Step 2: Activated
        let active = ws.activate().unwrap();
        let active_ts = active.updated_at();
        let activated_event = WorkspaceEvent::WorkspaceActivated {
            workspace_id: ws_id.clone(),
            timestamp: active_ts,
        };
        assert!(matches!(
            activated_event,
            WorkspaceEvent::WorkspaceActivated { .. }
        ));

        // Step 3: Locked
        let locked = active.lock("agent-lc".into()).unwrap();
        let locked_ts = locked.updated_at();
        let locked_event = WorkspaceEvent::WorkspaceLocked {
            workspace_id: ws_id.clone(),
            holder: "agent-lc".into(),
            timestamp: locked_ts,
        };
        assert!(matches!(
            locked_event,
            WorkspaceEvent::WorkspaceLocked { .. }
        ));

        // Step 4: Unlocked
        let unlocked = locked.unlock().unwrap();
        let unlocked_ts = unlocked.updated_at();
        let unlocked_event = WorkspaceEvent::WorkspaceUnlocked {
            workspace_id: ws_id.clone(),
            timestamp: unlocked_ts,
        };
        assert!(matches!(
            unlocked_event,
            WorkspaceEvent::WorkspaceUnlocked { .. }
        ));

        // Step 5: Deleted
        let deleted = unlocked.delete().unwrap();
        let deleted_ts = deleted.updated_at();
        let deleted_event = WorkspaceEvent::WorkspaceDeleted {
            workspace_id: ws_id.clone(),
            timestamp: deleted_ts,
        };
        assert!(matches!(
            deleted_event,
            WorkspaceEvent::WorkspaceDeleted { .. }
        ));

        // Verify chronological ordering using captured timestamps
        assert!(active_ts <= locked_ts);
        assert!(locked_ts <= unlocked_ts);
        assert!(unlocked_ts <= deleted_ts);
    }

    // === Proptests for event-transition contract ===

    #[cfg(test)]
    mod contract_proptests {
        use super::*;
        use crate::domain::entities::WorkspaceState;
        use crate::domain::state::WorkspaceStateMachine;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn valid_transition_always_produces_event(
                from_idx in 0usize..5,
                to_idx in 0usize..5
            ) {
                let states = [
                    WorkspaceState::Initializing,
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    WorkspaceState::Corrupted,
                    WorkspaceState::Deleted,
                ];
                let from = states[from_idx];
                let to = states[to_idx];
                if WorkspaceStateMachine::can_transition(from, to) {
                    let event = expected_event_for_transition(from, to, "ws-prop");
                    prop_assert!(
                        event.is_some(),
                        "valid transition {:?} → {:?} must produce an event",
                        from, to
                    );
                }
            }

            #[test]
            fn invalid_transition_never_produces_event(
                from_idx in 0usize..5,
                to_idx in 0usize..5
            ) {
                let states = [
                    WorkspaceState::Initializing,
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    WorkspaceState::Corrupted,
                    WorkspaceState::Deleted,
                ];
                let from = states[from_idx];
                let to = states[to_idx];
                if !WorkspaceStateMachine::can_transition(from, to) {
                    let event = expected_event_for_transition(from, to, "ws-prop");
                    prop_assert!(
                        event.is_none(),
                        "invalid transition {:?} → {:?} must not produce an event, got {:?}",
                        from, to, event
                    );
                }
            }

            #[test]
            fn event_workspace_id_matches_input(
                from_idx in 0usize..5,
                to_idx in 0usize..5,
                id in "[a-zA-Z0-9_-]{1,20}"
            ) {
                let states = [
                    WorkspaceState::Initializing,
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    WorkspaceState::Corrupted,
                    WorkspaceState::Deleted,
                ];
                let from = states[from_idx];
                let to = states[to_idx];
                if let Some(event) = expected_event_for_transition(from, to, &id) {
                    let event_id = match &event {
                        WorkspaceEvent::WorkspaceCreated { workspace_id, .. } => workspace_id,
                        WorkspaceEvent::WorkspaceActivated { workspace_id, .. } => workspace_id,
                        WorkspaceEvent::WorkspaceLocked { workspace_id, .. } => workspace_id,
                        WorkspaceEvent::WorkspaceUnlocked { workspace_id, .. } => workspace_id,
                        WorkspaceEvent::WorkspaceCorrupted { workspace_id, .. } => workspace_id,
                        WorkspaceEvent::WorkspaceDeleted { workspace_id, .. } => workspace_id,
                        WorkspaceEvent::WorkspaceConfigUpdated { workspace_id, .. } => workspace_id,
                    };
                    prop_assert_eq!(event_id, &id);
                }
            }
        }
    }
}
