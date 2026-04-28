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
        use proptest::prelude::*;

        use super::*;

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
}
