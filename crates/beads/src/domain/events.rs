//! Domain events emitted by [`BeadService`](crate::BeadService) operations.
//!
//! Each event represents a discrete state change on a bead and carries
//! a timestamp for auditability.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::value_objects::{BeadId, BeadState, BeadTitle, Priority};

/// Immutable record of a state change on a bead.
///
/// Events are produced as return values from [`BeadService`](crate::BeadService)
/// methods and can be used for event sourcing, audit logs, or notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeadEvent {
    /// A new bead was created.
    Created {
        /// ID of the created bead.
        id: BeadId,
        /// Title at creation time.
        title: BeadTitle,
        /// Timestamp of creation.
        created_at: DateTime<Utc>,
    },

    /// A bead's title was changed.
    TitleChanged {
        /// ID of the bead.
        id: BeadId,
        /// Previous title.
        old_title: BeadTitle,
        /// New title.
        new_title: BeadTitle,
        /// Timestamp of the change.
        changed_at: DateTime<Utc>,
    },

    /// A bead's state was transitioned.
    StateChanged {
        /// ID of the bead.
        id: BeadId,
        /// State before the transition.
        old_state: BeadState,
        /// State after the transition.
        new_state: BeadState,
        /// Timestamp of the transition.
        changed_at: DateTime<Utc>,
    },

    /// A bead's priority was set or changed.
    PrioritySet {
        /// ID of the bead.
        id: BeadId,
        /// The new priority.
        priority: Priority,
        /// Timestamp of the change.
        changed_at: DateTime<Utc>,
    },

    /// A bead was assigned or unassigned.
    AssigneeSet {
        /// ID of the bead.
        id: BeadId,
        /// The new assignee (`None` = unassigned).
        assignee: Option<String>,
        /// Timestamp of the change.
        changed_at: DateTime<Utc>,
    },

    /// A dependency was added to a bead.
    DependencyAdded {
        /// ID of the bead gaining the dependency.
        id: BeadId,
        /// The bead it now depends on.
        depends_on: BeadId,
        /// Timestamp of the change.
        changed_at: DateTime<Utc>,
    },

    /// A blocker was added to a bead.
    BlockerAdded {
        /// ID of the bead that was blocked.
        id: BeadId,
        /// The bead blocking it.
        blocked_by: BeadId,
        /// Timestamp of the change.
        changed_at: DateTime<Utc>,
    },

    /// A label was applied to a bead.
    Labeled {
        /// ID of the bead.
        id: BeadId,
        /// The label that was applied.
        label: String,
        /// Timestamp of the change.
        changed_at: DateTime<Utc>,
    },

    /// A bead was deleted.
    Deleted {
        /// ID of the deleted bead.
        id: BeadId,
        /// Timestamp of deletion.
        deleted_at: DateTime<Utc>,
    },
}

impl BeadEvent {
    /// Returns the ID of the bead this event relates to.
    #[must_use]
    pub const fn id(&self) -> &BeadId {
        match self {
            Self::Created { id, .. } => id,
            Self::TitleChanged { id, .. } => id,
            Self::StateChanged { id, .. } => id,
            Self::PrioritySet { id, .. } => id,
            Self::AssigneeSet { id, .. } => id,
            Self::DependencyAdded { id, .. } => id,
            Self::BlockerAdded { id, .. } => id,
            Self::Labeled { id, .. } => id,
            Self::Deleted { id, .. } => id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{BeadState, Priority};

    fn test_id() -> BeadId {
        BeadId::new("evt-1").unwrap()
    }

    fn test_title() -> BeadTitle {
        BeadTitle::new("Test Title").unwrap()
    }

    #[test]
    fn created_event_returns_correct_id() {
        let event = BeadEvent::Created {
            id: test_id(),
            title: test_title(),
            created_at: Utc::now(),
        };
        assert_eq!(event.id().as_str(), "evt-1");
    }

    #[test]
    fn title_changed_event_returns_correct_id() {
        let event = BeadEvent::TitleChanged {
            id: test_id(),
            old_title: test_title(),
            new_title: BeadTitle::new("New Title").unwrap(),
            changed_at: Utc::now(),
        };
        assert_eq!(event.id().as_str(), "evt-1");
    }

    #[test]
    fn state_changed_event_returns_correct_id() {
        let event = BeadEvent::StateChanged {
            id: test_id(),
            old_state: BeadState::Open,
            new_state: BeadState::InProgress,
            changed_at: Utc::now(),
        };
        assert_eq!(event.id().as_str(), "evt-1");
    }

    #[test]
    fn priority_set_event_returns_correct_id() {
        let event = BeadEvent::PrioritySet {
            id: test_id(),
            priority: Priority::P1,
            changed_at: Utc::now(),
        };
        assert_eq!(event.id().as_str(), "evt-1");
    }

    #[test]
    fn assignee_set_event_returns_correct_id() {
        let event = BeadEvent::AssigneeSet {
            id: test_id(),
            assignee: Some("alice".into()),
            changed_at: Utc::now(),
        };
        assert_eq!(event.id().as_str(), "evt-1");
    }

    #[test]
    fn assignee_set_event_with_none_assignee() {
        let event = BeadEvent::AssigneeSet {
            id: test_id(),
            assignee: None,
            changed_at: Utc::now(),
        };
        assert_eq!(event.id().as_str(), "evt-1");
    }

    #[test]
    fn dependency_added_event_returns_correct_id() {
        let dep_id = BeadId::new("dep-1").unwrap();
        let event = BeadEvent::DependencyAdded {
            id: test_id(),
            depends_on: dep_id,
            changed_at: Utc::now(),
        };
        assert_eq!(event.id().as_str(), "evt-1");
    }

    #[test]
    fn blocker_added_event_returns_correct_id() {
        let blocker_id = BeadId::new("blocker-1").unwrap();
        let event = BeadEvent::BlockerAdded {
            id: test_id(),
            blocked_by: blocker_id,
            changed_at: Utc::now(),
        };
        assert_eq!(event.id().as_str(), "evt-1");
    }

    #[test]
    fn labeled_event_returns_correct_id() {
        let event = BeadEvent::Labeled {
            id: test_id(),
            label: "urgent".into(),
            changed_at: Utc::now(),
        };
        assert_eq!(event.id().as_str(), "evt-1");
    }

    #[test]
    fn deleted_event_returns_correct_id() {
        let event = BeadEvent::Deleted {
            id: test_id(),
            deleted_at: Utc::now(),
        };
        assert_eq!(event.id().as_str(), "evt-1");
    }

    #[test]
    fn serde_roundtrip_created() {
        let event = BeadEvent::Created {
            id: test_id(),
            title: test_title(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_state_changed() {
        let event = BeadEvent::StateChanged {
            id: test_id(),
            old_state: BeadState::Open,
            new_state: BeadState::InProgress,
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_deleted() {
        let event = BeadEvent::Deleted {
            id: test_id(),
            deleted_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_priority_set() {
        let event = BeadEvent::PrioritySet {
            id: test_id(),
            priority: Priority::P3,
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_dependency_added() {
        let dep_id = BeadId::new("dep-99").unwrap();
        let event = BeadEvent::DependencyAdded {
            id: test_id(),
            depends_on: dep_id,
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_assignee_set_none() {
        let event = BeadEvent::AssigneeSet {
            id: test_id(),
            assignee: None,
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn event_is_debug() {
        let event = BeadEvent::Created {
            id: test_id(),
            title: test_title(),
            created_at: Utc::now(),
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("Created"));
    }

    #[test]
    fn event_is_clone() {
        let event = BeadEvent::Created {
            id: test_id(),
            title: test_title(),
            created_at: Utc::now(),
        };
        let cloned = event.clone();
        assert_eq!(event.id().as_str(), cloned.id().as_str());
    }

    #[test]
    fn serde_roundtrip_title_changed() {
        let event = BeadEvent::TitleChanged {
            id: test_id(),
            old_title: test_title(),
            new_title: BeadTitle::new("Updated Title").unwrap(),
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_labeled() {
        let event = BeadEvent::Labeled {
            id: test_id(),
            label: "critical".into(),
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_blocker_added() {
        let blocker_id = BeadId::new("blk-1").unwrap();
        let event = BeadEvent::BlockerAdded {
            id: test_id(),
            blocked_by: blocker_id,
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_assignee_set_some() {
        let event = BeadEvent::AssigneeSet {
            id: test_id(),
            assignee: Some("bob".into()),
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn all_event_variants_produce_id() {
        let dep_id = BeadId::new("d-1").unwrap();
        let now = Utc::now();

        assert!(matches!(
            BeadEvent::Created {
                id: test_id(),
                title: test_title(),
                created_at: now
            }
            .id()
            .as_str(),
            "evt-1"
        ));
        assert!(matches!(
            BeadEvent::TitleChanged {
                id: test_id(),
                old_title: test_title(),
                new_title: test_title(),
                changed_at: now
            }
            .id()
            .as_str(),
            "evt-1"
        ));
        assert!(matches!(
            BeadEvent::StateChanged {
                id: test_id(),
                old_state: BeadState::Open,
                new_state: BeadState::InProgress,
                changed_at: now
            }
            .id()
            .as_str(),
            "evt-1"
        ));
        assert!(matches!(
            BeadEvent::PrioritySet {
                id: test_id(),
                priority: Priority::P1,
                changed_at: now
            }
            .id()
            .as_str(),
            "evt-1"
        ));
        assert!(matches!(
            BeadEvent::AssigneeSet {
                id: test_id(),
                assignee: None,
                changed_at: now
            }
            .id()
            .as_str(),
            "evt-1"
        ));
        assert!(matches!(
            BeadEvent::DependencyAdded {
                id: test_id(),
                depends_on: dep_id.clone(),
                changed_at: now
            }
            .id()
            .as_str(),
            "evt-1"
        ));
        assert!(matches!(
            BeadEvent::BlockerAdded {
                id: test_id(),
                blocked_by: dep_id,
                changed_at: now
            }
            .id()
            .as_str(),
            "evt-1"
        ));
        assert!(matches!(
            BeadEvent::Labeled {
                id: test_id(),
                label: "x".into(),
                changed_at: now
            }
            .id()
            .as_str(),
            "evt-1"
        ));
        assert!(matches!(
            BeadEvent::Deleted {
                id: test_id(),
                deleted_at: now
            }
            .id()
            .as_str(),
            "evt-1"
        ));
    }

    // ── Equality tests ───────────────────────────────────────────────────────

    #[test]
    fn created_event_equality() {
        let now = Utc::now();
        let e1 = BeadEvent::Created {
            id: test_id(),
            title: test_title(),
            created_at: now,
        };
        let e2 = BeadEvent::Created {
            id: test_id(),
            title: test_title(),
            created_at: now,
        };
        assert_eq!(e1, e2);
    }

    #[test]
    fn created_event_inequality_different_title() {
        let now = Utc::now();
        let e1 = BeadEvent::Created {
            id: test_id(),
            title: test_title(),
            created_at: now,
        };
        let e2 = BeadEvent::Created {
            id: test_id(),
            title: BeadTitle::new("Different").unwrap(),
            created_at: now,
        };
        assert_ne!(e1, e2);
    }

    #[test]
    fn created_event_inequality_different_id() {
        let now = Utc::now();
        let e1 = BeadEvent::Created {
            id: test_id(),
            title: test_title(),
            created_at: now,
        };
        let e2 = BeadEvent::Created {
            id: BeadId::new("other").unwrap(),
            title: test_title(),
            created_at: now,
        };
        assert_ne!(e1, e2);
    }

    #[test]
    fn state_changed_equality() {
        let now = Utc::now();
        let e1 = BeadEvent::StateChanged {
            id: test_id(),
            old_state: BeadState::Open,
            new_state: BeadState::InProgress,
            changed_at: now,
        };
        let e2 = BeadEvent::StateChanged {
            id: test_id(),
            old_state: BeadState::Open,
            new_state: BeadState::InProgress,
            changed_at: now,
        };
        assert_eq!(e1, e2);
    }

    #[test]
    fn state_changed_inequality_different_states() {
        let now = Utc::now();
        let e1 = BeadEvent::StateChanged {
            id: test_id(),
            old_state: BeadState::Open,
            new_state: BeadState::InProgress,
            changed_at: now,
        };
        let e2 = BeadEvent::StateChanged {
            id: test_id(),
            old_state: BeadState::Open,
            new_state: BeadState::Blocked,
            changed_at: now,
        };
        assert_ne!(e1, e2);
    }

    #[test]
    fn different_event_variants_are_not_equal() {
        let now = Utc::now();
        let created = BeadEvent::Created {
            id: test_id(),
            title: test_title(),
            created_at: now,
        };
        let deleted = BeadEvent::Deleted {
            id: test_id(),
            deleted_at: now,
        };
        assert_ne!(created, deleted);
    }

    // ── Debug formatting for all variants ────────────────────────────────────

    #[test]
    fn title_changed_is_debug() {
        let event = BeadEvent::TitleChanged {
            id: test_id(),
            old_title: test_title(),
            new_title: BeadTitle::new("New").unwrap(),
            changed_at: Utc::now(),
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("TitleChanged"));
    }

    #[test]
    fn state_changed_is_debug() {
        let event = BeadEvent::StateChanged {
            id: test_id(),
            old_state: BeadState::Open,
            new_state: BeadState::InProgress,
            changed_at: Utc::now(),
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("StateChanged"));
    }

    #[test]
    fn priority_set_is_debug() {
        let event = BeadEvent::PrioritySet {
            id: test_id(),
            priority: Priority::P0,
            changed_at: Utc::now(),
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("PrioritySet"));
    }

    #[test]
    fn assignee_set_is_debug() {
        let event = BeadEvent::AssigneeSet {
            id: test_id(),
            assignee: Some("alice".into()),
            changed_at: Utc::now(),
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("AssigneeSet"));
    }

    #[test]
    fn dependency_added_is_debug() {
        let event = BeadEvent::DependencyAdded {
            id: test_id(),
            depends_on: BeadId::new("dep").unwrap(),
            changed_at: Utc::now(),
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("DependencyAdded"));
    }

    #[test]
    fn blocker_added_is_debug() {
        let event = BeadEvent::BlockerAdded {
            id: test_id(),
            blocked_by: BeadId::new("blk").unwrap(),
            changed_at: Utc::now(),
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("BlockerAdded"));
    }

    #[test]
    fn labeled_is_debug() {
        let event = BeadEvent::Labeled {
            id: test_id(),
            label: "critical".into(),
            changed_at: Utc::now(),
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("Labeled"));
    }

    #[test]
    fn deleted_is_debug() {
        let event = BeadEvent::Deleted {
            id: test_id(),
            deleted_at: Utc::now(),
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("Deleted"));
    }

    // ── Serde roundtrip for all variants ─────────────────────────────────────

    #[test]
    fn serde_roundtrip_assignee_set_some_with_empty_string() {
        let event = BeadEvent::AssigneeSet {
            id: test_id(),
            assignee: Some(String::new()),
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
        match parsed {
            BeadEvent::AssigneeSet { assignee, .. } => {
                assert_eq!(assignee, Some(String::new()));
            }
            other => panic!("expected AssigneeSet, got {other:?}"),
        }
    }

    #[test]
    fn serde_roundtrip_state_changed_with_closed_state() {
        let event = BeadEvent::StateChanged {
            id: test_id(),
            old_state: BeadState::InProgress,
            new_state: BeadState::Closed {
                closed_at: Utc::now(),
            },
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_state_changed_same_state() {
        let event = BeadEvent::StateChanged {
            id: test_id(),
            old_state: BeadState::Open,
            new_state: BeadState::Open,
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_labeled_empty_string() {
        let event = BeadEvent::Labeled {
            id: test_id(),
            label: String::new(),
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn serde_roundtrip_labeled_long_label() {
        let long_label = "a".repeat(500);
        let event = BeadEvent::Labeled {
            id: test_id(),
            label: long_label.clone(),
            changed_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BeadEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
        match parsed {
            BeadEvent::Labeled { label, .. } => assert_eq!(label, long_label),
            other => panic!("expected Labeled, got {other:?}"),
        }
    }

    // ── Clone tests for all variants ─────────────────────────────────────────

    #[test]
    fn title_changed_is_clone() {
        let event = BeadEvent::TitleChanged {
            id: test_id(),
            old_title: test_title(),
            new_title: BeadTitle::new("Updated").unwrap(),
            changed_at: Utc::now(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn state_changed_is_clone() {
        let event = BeadEvent::StateChanged {
            id: test_id(),
            old_state: BeadState::Open,
            new_state: BeadState::InProgress,
            changed_at: Utc::now(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn priority_set_is_clone() {
        let event = BeadEvent::PrioritySet {
            id: test_id(),
            priority: Priority::P2,
            changed_at: Utc::now(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn assignee_set_is_clone() {
        let event = BeadEvent::AssigneeSet {
            id: test_id(),
            assignee: Some("alice".into()),
            changed_at: Utc::now(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn dependency_added_is_clone() {
        let event = BeadEvent::DependencyAdded {
            id: test_id(),
            depends_on: BeadId::new("dep-x").unwrap(),
            changed_at: Utc::now(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn blocker_added_is_clone() {
        let event = BeadEvent::BlockerAdded {
            id: test_id(),
            blocked_by: BeadId::new("blk-x").unwrap(),
            changed_at: Utc::now(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn labeled_is_clone() {
        let event = BeadEvent::Labeled {
            id: test_id(),
            label: "test-label".into(),
            changed_at: Utc::now(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn deleted_is_clone() {
        let event = BeadEvent::Deleted {
            id: test_id(),
            deleted_at: Utc::now(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    // ── Id accessor with different IDs ───────────────────────────────────────

    #[test]
    fn id_returns_different_values_for_different_events() {
        let id1 = BeadId::new("first").unwrap();
        let id2 = BeadId::new("second").unwrap();
        let e1 = BeadEvent::Created {
            id: id1,
            title: test_title(),
            created_at: Utc::now(),
        };
        let e2 = BeadEvent::Deleted {
            id: id2,
            deleted_at: Utc::now(),
        };
        assert_ne!(e1.id().as_str(), e2.id().as_str());
    }

    #[test]
    fn created_event_title_preserved() {
        let now = Utc::now();
        let title = BeadTitle::new("My Bead Title").unwrap();
        let event = BeadEvent::Created {
            id: test_id(),
            title: title.clone(),
            created_at: now,
        };
        if let BeadEvent::Created {
            title: evt_title, ..
        } = event
        {
            assert_eq!(evt_title, title);
        } else {
            panic!("expected Created variant");
        }
    }

    #[test]
    fn title_changed_event_preserves_both_titles() {
        let old = BeadTitle::new("Old Title").unwrap();
        let new = BeadTitle::new("New Title").unwrap();
        let event = BeadEvent::TitleChanged {
            id: test_id(),
            old_title: old.clone(),
            new_title: new.clone(),
            changed_at: Utc::now(),
        };
        if let BeadEvent::TitleChanged {
            old_title,
            new_title,
            ..
        } = event
        {
            assert_eq!(old_title, old);
            assert_eq!(new_title, new);
        } else {
            panic!("expected TitleChanged variant");
        }
    }
}
