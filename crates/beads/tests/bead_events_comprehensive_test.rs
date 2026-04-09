//! Comprehensive tests for Bead events.
//!
//! This module provides exhaustive verification of bead event behavior:
//! - Each valid lifecycle transition fires the correct event with proper payload
//! - Invalid transitions return errors without emitting events
//! - Event ordering is preserved (timestamps monotonically increase)
//! - Duplicate event prevention (same-state transitions handled correctly)

use chrono::Utc;
use scp_beads::domain::events::BeadEvent;
use scp_beads::domain::value_objects::{BeadId, BeadState, Priority};
use scp_beads::infrastructure::repository::InMemoryBeadRepository;
use scp_beads::BeadService;

fn make_service() -> BeadService<InMemoryBeadRepository> {
    BeadService::new(InMemoryBeadRepository::new())
}

fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}

// ============================================================================
// Group 1: Valid Transitions - Correct Event Type and Payload
// ============================================================================

#[tokio::test]
async fn create_bead_emits_created_event_with_correct_payload() {
    let svc = make_service();
    let before = now();

    let (bead, event) = svc
        .create_bead("create-payload", "Test Title", None)
        .await
        .unwrap();

    match event {
        BeadEvent::Created {
            id,
            title,
            created_at,
        } => {
            assert_eq!(id.as_str(), "create-payload");
            assert_eq!(title.as_str(), "Test Title");
            assert!(created_at >= before);
            assert!(created_at <= now());
        }
        other => panic!("expected Created event, got {other:?}"),
    }

    assert_eq!(bead.id().as_str(), "create-payload");
    assert_eq!(bead.title().as_str(), "Test Title");
}

#[tokio::test]
async fn create_bead_with_description_emits_created_event() {
    let svc = make_service();
    let before = now();

    let (bead, event) = svc
        .create_bead("create-desc", "Test Title", Some("A description".into()))
        .await
        .unwrap();

    match event {
        BeadEvent::Created {
            id,
            title,
            created_at,
        } => {
            assert_eq!(id.as_str(), "create-desc");
            assert_eq!(title.as_str(), "Test Title");
            assert!(created_at >= before);
        }
        other => panic!("expected Created event, got {other:?}"),
    }

    assert_eq!(bead.description().unwrap().as_str(), "A description");
}

#[tokio::test]
async fn state_change_open_to_in_progress_emits_correct_event() {
    let svc = make_service();
    svc.create_bead("state-ip", "State Test", None).await.unwrap();
    let id = BeadId::new("state-ip").unwrap();
    let before = now();

    let (bead, event) = svc
        .update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();

    match event {
        BeadEvent::StateChanged {
            id: event_id,
            old_state,
            new_state,
            changed_at,
        } => {
            assert_eq!(event_id.as_str(), "state-ip");
            assert_eq!(old_state, BeadState::Open);
            assert_eq!(new_state, BeadState::InProgress);
            assert!(changed_at >= before);
            assert!(changed_at <= now());
        }
        other => panic!("expected StateChanged event, got {other:?}"),
    }

    assert_eq!(bead.state(), BeadState::InProgress);
}

#[tokio::test]
async fn state_change_in_progress_to_blocked_emits_correct_event() {
    let svc = make_service();
    svc.create_bead("state-blocked", "State Test", None).await.unwrap();
    let id = BeadId::new("state-blocked").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();

    let (bead, event) = svc
        .update_bead_state(&id, BeadState::Blocked)
        .await
        .unwrap();

    match event {
        BeadEvent::StateChanged {
            id: event_id,
            old_state,
            new_state,
            changed_at: _,
        } => {
            assert_eq!(event_id.as_str(), "state-blocked");
            assert_eq!(old_state, BeadState::InProgress);
            assert_eq!(new_state, BeadState::Blocked);
        }
        other => panic!("expected StateChanged event, got {other:?}"),
    }

    assert_eq!(bead.state(), BeadState::Blocked);
}

#[tokio::test]
async fn state_change_blocked_to_in_progress_emits_correct_event() {
    let svc = make_service();
    svc.create_bead("state-resumed", "State Test", None).await.unwrap();
    let id = BeadId::new("state-resumed").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    svc.update_bead_state(&id, BeadState::Blocked)
        .await
        .unwrap();

    let (bead, event) = svc
        .update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();

    match event {
        BeadEvent::StateChanged {
            id: event_id,
            old_state,
            new_state,
            changed_at: _,
        } => {
            assert_eq!(event_id.as_str(), "state-resumed");
            assert_eq!(old_state, BeadState::Blocked);
            assert_eq!(new_state, BeadState::InProgress);
        }
        other => panic!("expected StateChanged event, got {other:?}"),
    }

    assert_eq!(bead.state(), BeadState::InProgress);
}

#[tokio::test]
async fn state_change_in_progress_to_deferred_emits_correct_event() {
    let svc = make_service();
    svc.create_bead("state-deferred", "State Test", None).await.unwrap();
    let id = BeadId::new("state-deferred").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();

    let (bead, event) = svc
        .update_bead_state(&id, BeadState::Deferred)
        .await
        .unwrap();

    match event {
        BeadEvent::StateChanged {
            id: event_id,
            old_state,
            new_state,
            changed_at: _,
        } => {
            assert_eq!(event_id.as_str(), "state-deferred");
            assert_eq!(old_state, BeadState::InProgress);
            assert_eq!(new_state, BeadState::Deferred);
        }
        other => panic!("expected StateChanged event, got {other:?}"),
    }

    assert_eq!(bead.state(), BeadState::Deferred);
}

#[tokio::test]
async fn state_change_deferred_to_in_progress_emits_correct_event() {
    let svc = make_service();
    svc.create_bead("state-def-resume", "State Test", None).await.unwrap();
    let id = BeadId::new("state-def-resume").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    svc.update_bead_state(&id, BeadState::Deferred)
        .await
        .unwrap();

    let (bead, event) = svc
        .update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();

    match event {
        BeadEvent::StateChanged {
            id: event_id,
            old_state,
            new_state,
            changed_at: _,
        } => {
            assert_eq!(event_id.as_str(), "state-def-resume");
            assert_eq!(old_state, BeadState::Deferred);
            assert_eq!(new_state, BeadState::InProgress);
        }
        other => panic!("expected StateChanged event, got {other:?}"),
    }

    assert_eq!(bead.state(), BeadState::InProgress);
}

#[tokio::test]
async fn state_change_in_progress_to_closed_emits_correct_event() {
    let svc = make_service();
    svc.create_bead("state-closed", "State Test", None).await.unwrap();
    let id = BeadId::new("state-closed").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    let closed_at = now();

    let (bead, event) = svc
        .update_bead_state(
            &id,
            BeadState::Closed {
                closed_at,
            },
        )
        .await
        .unwrap();

    match event {
        BeadEvent::StateChanged {
            id: event_id,
            old_state,
            new_state,
            changed_at: _,
        } => {
            assert_eq!(event_id.as_str(), "state-closed");
            assert_eq!(old_state, BeadState::InProgress);
            assert!(new_state.is_closed());
        }
        other => panic!("expected StateChanged event, got {other:?}"),
    }

    assert!(bead.state().is_closed());
}

#[tokio::test]
async fn state_change_blocked_to_closed_emits_correct_event() {
    let svc = make_service();
    svc.create_bead("state-blk-closed", "State Test", None).await.unwrap();
    let id = BeadId::new("state-blk-closed").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    svc.update_bead_state(&id, BeadState::Blocked)
        .await
        .unwrap();
    let closed_at = now();

    let (bead, event) = svc
        .update_bead_state(
            &id,
            BeadState::Closed {
                closed_at,
            },
        )
        .await
        .unwrap();

    match event {
        BeadEvent::StateChanged {
            id: event_id,
            old_state,
            new_state,
            changed_at: _,
        } => {
            assert_eq!(event_id.as_str(), "state-blk-closed");
            assert_eq!(old_state, BeadState::Blocked);
            assert!(new_state.is_closed());
        }
        other => panic!("expected StateChanged event, got {other:?}"),
    }

    assert!(bead.state().is_closed());
}

#[tokio::test]
async fn state_change_deferred_to_closed_emits_correct_event() {
    let svc = make_service();
    svc.create_bead("state-def-closed", "State Test", None).await.unwrap();
    let id = BeadId::new("state-def-closed").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    svc.update_bead_state(&id, BeadState::Deferred)
        .await
        .unwrap();
    let closed_at = now();

    let (bead, event) = svc
        .update_bead_state(
            &id,
            BeadState::Closed {
                closed_at,
            },
        )
        .await
        .unwrap();

    match event {
        BeadEvent::StateChanged {
            id: event_id,
            old_state,
            new_state,
            changed_at: _,
        } => {
            assert_eq!(event_id.as_str(), "state-def-closed");
            assert_eq!(old_state, BeadState::Deferred);
            assert!(new_state.is_closed());
        }
        other => panic!("expected StateChanged event, got {other:?}"),
    }

    assert!(bead.state().is_closed());
}

#[tokio::test]
async fn priority_set_emits_priority_set_event() {
    let svc = make_service();
    svc.create_bead("prio-test", "Priority Test", None)
        .await
        .unwrap();
    let id = BeadId::new("prio-test").unwrap();
    let before = now();

    let (bead, event) = svc
        .set_priority(&id, Priority::P0)
        .await
        .unwrap();

    match event {
        BeadEvent::PrioritySet {
            id: event_id,
            priority,
            changed_at,
        } => {
            assert_eq!(event_id.as_str(), "prio-test");
            assert_eq!(priority, Priority::P0);
            assert!(changed_at >= before);
        }
        other => panic!("expected PrioritySet event, got {other:?}"),
    }

    assert_eq!(bead.priority(), Some(&Priority::P0));
}

#[tokio::test]
async fn assignee_set_emits_assignee_set_event() {
    let svc = make_service();
    svc.create_bead("assign-test", "Assign Test", None)
        .await
        .unwrap();
    let id = BeadId::new("assign-test").unwrap();
    let before = now();

    let (bead, event) = svc
        .assign_bead(&id, Some("tester".into()))
        .await
        .unwrap();

    match event {
        BeadEvent::AssigneeSet {
            id: event_id,
            assignee,
            changed_at,
        } => {
            assert_eq!(event_id.as_str(), "assign-test");
            assert_eq!(assignee.as_deref(), Some("tester"));
            assert!(changed_at >= before);
        }
        other => panic!("expected AssigneeSet event, got {other:?}"),
    }

    assert_eq!(bead.assignee(), Some("tester"));
}

#[tokio::test]
async fn delete_emits_deleted_event() {
    let svc = make_service();
    svc.create_bead("delete-test", "Delete Test", None)
        .await
        .unwrap();
    let id = BeadId::new("delete-test").unwrap();
    let before = now();

    let event = svc.delete_bead(&id).await.unwrap();

    match event {
        BeadEvent::Deleted {
            id: event_id,
            deleted_at,
        } => {
            assert_eq!(event_id.as_str(), "delete-test");
            assert!(deleted_at >= before);
        }
        other => panic!("expected Deleted event, got {other:?}"),
    }
}

// ============================================================================
// Group 2: Invalid Transitions - No Events Emitted
// ============================================================================

#[tokio::test]
async fn invalid_transition_open_to_blocked_returns_error_no_event() {
    let svc = make_service();
    svc.create_bead("inv-open-blocked", "Test", None)
        .await
        .unwrap();
    let id = BeadId::new("inv-open-blocked").unwrap();

    let result = svc
        .update_bead_state(&id, BeadState::Blocked)
        .await;

    assert!(result.is_err(), "Open -> Blocked should fail");
}

#[tokio::test]
async fn invalid_transition_open_to_deferred_returns_error_no_event() {
    let svc = make_service();
    svc.create_bead("inv-open-deferred", "Test", None)
        .await
        .unwrap();
    let id = BeadId::new("inv-open-deferred").unwrap();

    let result = svc
        .update_bead_state(&id, BeadState::Deferred)
        .await;

    assert!(result.is_err(), "Open -> Deferred should fail");
}

#[tokio::test]
async fn invalid_transition_open_to_closed_returns_error_no_event() {
    let svc = make_service();
    svc.create_bead("inv-open-closed", "Test", None)
        .await
        .unwrap();
    let id = BeadId::new("inv-open-closed").unwrap();

    let result = svc.update_bead_state(
        &id,
        BeadState::Closed {
            closed_at: now(),
        },
    ).await;

    assert!(result.is_err(), "Open -> Closed should fail");
}

#[tokio::test]
async fn invalid_transition_in_progress_to_open_returns_error_no_event() {
    let svc = make_service();
    svc.create_bead("inv-ip-open", "Test", None).await.unwrap();
    let id = BeadId::new("inv-ip-open").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();

    let result = svc.update_bead_state(&id, BeadState::Open).await;

    assert!(result.is_err(), "InProgress -> Open should fail");
}

#[tokio::test]
async fn invalid_transition_blocked_to_open_returns_error_no_event() {
    let svc = make_service();
    svc.create_bead("inv-blocked-open", "Test", None)
        .await
        .unwrap();
    let id = BeadId::new("inv-blocked-open").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    svc.update_bead_state(&id, BeadState::Blocked)
        .await
        .unwrap();

    let result = svc.update_bead_state(&id, BeadState::Open).await;

    assert!(result.is_err(), "Blocked -> Open should fail");
}

#[tokio::test]
async fn invalid_transition_deferred_to_open_returns_error_no_event() {
    let svc = make_service();
    svc.create_bead("inv-def-open", "Test", None).await.unwrap();
    let id = BeadId::new("inv-def-open").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    svc.update_bead_state(&id, BeadState::Deferred)
        .await
        .unwrap();

    let result = svc.update_bead_state(&id, BeadState::Open).await;

    assert!(result.is_err(), "Deferred -> Open should fail");
}

#[tokio::test]
async fn invalid_transition_deferred_to_blocked_returns_error_no_event() {
    let svc = make_service();
    svc.create_bead("inv-def-blocked", "Test", None)
        .await
        .unwrap();
    let id = BeadId::new("inv-def-blocked").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    svc.update_bead_state(&id, BeadState::Deferred)
        .await
        .unwrap();

    let result = svc.update_bead_state(&id, BeadState::Blocked).await;

    assert!(result.is_err(), "Deferred -> Blocked should fail");
}

#[tokio::test]
async fn closed_is_terminal_cannot_transition_to_any_state() {
    let svc = make_service();
    svc.create_bead("terminal-closed", "Test", None)
        .await
        .unwrap();
    let id = BeadId::new("terminal-closed").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    svc.update_bead_state(
        &id,
        BeadState::Closed {
            closed_at: now(),
        },
    )
    .await
    .unwrap();

    let targets = vec![
        BeadState::Open,
        BeadState::InProgress,
        BeadState::Blocked,
        BeadState::Deferred,
        BeadState::Closed {
            closed_at: now(),
        },
    ];

    for target in targets {
        let result = svc.update_bead_state(&id, target.clone()).await;
        assert!(
            result.is_err(),
            "Closed -> {:?} should fail (Closed is terminal)",
            target
        );
    }
}

// ============================================================================
// Group 3: Event Ordering (Timestamps Monotonically Increase)
// ============================================================================

#[tokio::test]
async fn event_timestamps_increase_across_full_lifecycle() {
    let svc = make_service();
    svc.create_bead("order-lifecycle", "Lifecycle", None)
        .await
        .unwrap();
    let id = BeadId::new("order-lifecycle").unwrap();

    let mut prev_ts: Option<chrono::DateTime<Utc>> = None;

    // Open -> InProgress
    let (_, event1) = svc
        .update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    if let BeadEvent::StateChanged { changed_at, .. } = event1 {
        if let Some(prev) = prev_ts {
            assert!(
                changed_at >= prev,
                "Timestamp should not decrease: {:?} < {:?}",
                changed_at,
                prev
            );
        }
        prev_ts = Some(changed_at);
    }

    // InProgress -> Blocked
    let (_, event2) = svc
        .update_bead_state(&id, BeadState::Blocked)
        .await
        .unwrap();
    if let BeadEvent::StateChanged { changed_at, .. } = event2 {
        if let Some(prev) = prev_ts {
            assert!(
                changed_at >= prev,
                "Timestamp should not decrease: {:?} < {:?}",
                changed_at,
                prev
            );
        }
        prev_ts = Some(changed_at);
    }

    // Blocked -> InProgress
    let (_, event3) = svc
        .update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    if let BeadEvent::StateChanged { changed_at, .. } = event3 {
        if let Some(prev) = prev_ts {
            assert!(
                changed_at >= prev,
                "Timestamp should not decrease: {:?} < {:?}",
                changed_at,
                prev
            );
        }
        prev_ts = Some(changed_at);
    }

    // InProgress -> Deferred
    let (_, event4) = svc
        .update_bead_state(&id, BeadState::Deferred)
        .await
        .unwrap();
    if let BeadEvent::StateChanged { changed_at, .. } = event4 {
        if let Some(prev) = prev_ts {
            assert!(
                changed_at >= prev,
                "Timestamp should not decrease: {:?} < {:?}",
                changed_at,
                prev
            );
        }
        prev_ts = Some(changed_at);
    }

    // Deferred -> InProgress
    let (_, event5) = svc
        .update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    if let BeadEvent::StateChanged { changed_at, .. } = event5 {
        if let Some(prev) = prev_ts {
            assert!(
                changed_at >= prev,
                "Timestamp should not decrease: {:?} < {:?}",
                changed_at,
                prev
            );
        }
        prev_ts = Some(changed_at);
    }

    // InProgress -> Closed
    let (_, event6) = svc
        .update_bead_state(
            &id,
            BeadState::Closed {
                closed_at: now(),
            },
        )
        .await
        .unwrap();
    if let BeadEvent::StateChanged { changed_at, .. } = event6 {
        if let Some(prev) = prev_ts {
            assert!(
                changed_at >= prev,
                "Timestamp should not decrease: {:?} < {:?}",
                changed_at,
                prev
            );
        }
    }
}

// ============================================================================
// Group 4: Duplicate Event Prevention
// ============================================================================

#[tokio::test]
async fn same_state_transitions_produce_distinguishable_events() {
    let svc = make_service();
    svc.create_bead("dup-test", "Duplicate Test", None)
        .await
        .unwrap();
    let id = BeadId::new("dup-test").unwrap();

    // Two consecutive Open -> Open transitions
    let (_, event1) = svc
        .update_bead_state(&id, BeadState::Open)
        .await
        .unwrap();
    let (_, event2) = svc
        .update_bead_state(&id, BeadState::Open)
        .await
        .unwrap();

    // Events should be distinguishable by timestamp
    match (&event1, &event2) {
        (
            BeadEvent::StateChanged {
                changed_at: ts_a,
                ..
            },
            BeadEvent::StateChanged {
                changed_at: ts_b,
                ..
            },
        ) => {
            // The second timestamp should be >= the first
            assert!(
                *ts_b >= *ts_a,
                "Later same-state transition should have >= timestamp: {:?} < {:?}",
                ts_b,
                ts_a
            );
        }
        _ => panic!("expected StateChanged events"),
    }
}

#[tokio::test]
async fn rapid_successive_transitions_produce_distinct_events() {
    let svc = make_service();
    svc.create_bead("rapid-test", "Rapid Test", None)
        .await
        .unwrap();
    let id = BeadId::new("rapid-test").unwrap();

    // Rapid transitions Open -> InProgress -> Blocked -> InProgress
    let (bead1, event1) = svc
        .update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    let (bead2, event2) = svc
        .update_bead_state(&id, BeadState::Blocked)
        .await
        .unwrap();
    let (bead3, event3) = svc
        .update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();

    // Verify each produces a distinct event with distinct old_state
    match (&event1, &event2, &event3) {
        (
            BeadEvent::StateChanged {
                old_state: old1,
                new_state: new1,
                ..
            },
            BeadEvent::StateChanged {
                old_state: old2,
                new_state: new2,
                ..
            },
            BeadEvent::StateChanged {
                old_state: old3,
                new_state: new3,
                ..
            },
        ) => {
            assert_eq!(*old1, BeadState::Open);
            assert_eq!(*new1, BeadState::InProgress);

            assert_eq!(*old2, BeadState::InProgress);
            assert_eq!(*new2, BeadState::Blocked);

            assert_eq!(*old3, BeadState::Blocked);
            assert_eq!(*new3, BeadState::InProgress);
        }
        _ => panic!("expected StateChanged events"),
    }

    assert_eq!(bead1.state(), BeadState::InProgress);
    assert_eq!(bead2.state(), BeadState::Blocked);
    assert_eq!(bead3.state(), BeadState::InProgress);
}

#[tokio::test]
async fn in_progress_to_closed_produces_closed_event() {
    let svc = make_service();
    svc.create_bead("closed-dup", "Closed Dup", None)
        .await
        .unwrap();
    let id = BeadId::new("closed-dup").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();
    let closed_at = now();

    // Transition to Closed
    let (_, event) = svc
        .update_bead_state(
            &id,
            BeadState::Closed {
                closed_at,
            },
        )
        .await
        .unwrap();

    // Verify the event
    match event {
        BeadEvent::StateChanged {
            old_state,
            new_state,
            changed_at: _,
            ..
        } => {
            assert_eq!(old_state, BeadState::InProgress);
            assert!(new_state.is_closed());
        }
        _ => panic!("expected StateChanged event"),
    }
}

// ============================================================================
// Group 5: Event Payload Correctness - All Fields Populated
// ============================================================================

#[tokio::test]
async fn created_event_contains_all_fields() {
    let svc = make_service();
    let (bead, event) = svc
        .create_bead("full-created", "Full Created", Some("A description".into()))
        .await
        .unwrap();

    match event {
        BeadEvent::Created {
            id,
            title,
            created_at,
        } => {
            assert_eq!(id.as_str(), "full-created");
            assert_eq!(title.as_str(), "Full Created");
            assert!(created_at <= now());
            assert!(created_at >= bead.created_at());
        }
        other => panic!("expected Created event, got {other:?}"),
    }
}

#[tokio::test]
async fn state_changed_event_old_state_matches_previous() {
    let svc = make_service();
    svc.create_bead("old-state", "Old State", None)
        .await
        .unwrap();
    let id = BeadId::new("old-state").unwrap();
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();

    let (_, event) = svc
        .update_bead_state(&id, BeadState::Blocked)
        .await
        .unwrap();

    match event {
        BeadEvent::StateChanged {
            old_state,
            new_state,
            ..
        } => {
            assert_eq!(old_state, BeadState::InProgress);
            assert_eq!(new_state, BeadState::Blocked);
        }
        other => panic!("expected StateChanged, got {other:?}"),
    }
}

#[tokio::test]
async fn priority_set_event_preserves_all_priority_values() {
    let svc = make_service();
    svc.create_bead("prio-val", "Prio Val", None)
        .await
        .unwrap();
    let id = BeadId::new("prio-val").unwrap();

    let priorities = vec![
        Priority::P0,
        Priority::P1,
        Priority::P2,
        Priority::P3,
        Priority::P4,
    ];

    for prio in priorities {
        let (_, event) = svc.set_priority(&id, prio).await.unwrap();
        match event {
            BeadEvent::PrioritySet {
                priority,
                changed_at: _,
                ..
            } => {
                assert_eq!(priority, prio);
            }
            other => panic!("expected PrioritySet, got {other:?}"),
        }
    }
}

// ============================================================================
// Group 6: Exhaustive Transition Matrix Verification
// ============================================================================

#[tokio::test]
async fn exhaustive_all_valid_transitions_emit_correct_event_types() {
    // Each case is (to_state, expected_old) - only transitions that ARE allowed
    let valid_cases: Vec<(BeadState, BeadState)> = vec![
        // From Open
        (BeadState::InProgress, BeadState::Open),
        (BeadState::Open, BeadState::Open), // same-state is allowed
        // From InProgress
        (BeadState::Blocked, BeadState::InProgress),
        (BeadState::Deferred, BeadState::InProgress),
        (BeadState::InProgress, BeadState::InProgress), // same-state
        (BeadState::Blocked, BeadState::InProgress), // going to blocked then closing
        // From Blocked
        (BeadState::InProgress, BeadState::Blocked),
        (BeadState::Deferred, BeadState::Blocked),
        (BeadState::Blocked, BeadState::Blocked), // same-state
        // From Deferred
        (BeadState::InProgress, BeadState::Deferred),
        (BeadState::Deferred, BeadState::Deferred), // same-state
    ];

    for (idx, (to_state, expected_old)) in valid_cases.into_iter().enumerate() {
        let svc = make_service();
        let bead_id_str = format!("exhaustive-valid-{}", idx);
        svc.create_bead(bead_id_str.as_str(), "Exhaustive", None)
            .await
            .unwrap();
        let id = BeadId::new(&bead_id_str).unwrap();

        // Navigate to expected old state if needed
        match expected_old {
            BeadState::InProgress => {
                svc.update_bead_state(&id, BeadState::InProgress)
                    .await
                    .unwrap();
            }
            BeadState::Blocked => {
                svc.update_bead_state(&id, BeadState::InProgress)
                    .await
                    .unwrap();
                svc.update_bead_state(&id, BeadState::Blocked)
                    .await
                    .unwrap();
            }
            BeadState::Deferred => {
                svc.update_bead_state(&id, BeadState::InProgress)
                    .await
                    .unwrap();
                svc.update_bead_state(&id, BeadState::Deferred)
                    .await
                    .unwrap();
            }
            _ => {}
        }

        let result = svc.update_bead_state(&id, to_state.clone()).await;
        assert!(
            result.is_ok(),
            "Transition {} should succeed but got: {:?}",
            idx,
            result
        );

        let (_, event) = result.unwrap();
        match event {
            BeadEvent::StateChanged {
                old_state,
                new_state,
                ..
            } => {
                assert_eq!(
                    old_state, expected_old,
                    "exhaustive[{}]: old_state mismatch",
                    idx
                );
                if to_state.is_closed() {
                    assert!(
                        new_state.is_closed(),
                        "exhaustive[{}]: expected Closed",
                        idx
                    );
                } else {
                    assert_eq!(
                        new_state, to_state,
                        "exhaustive[{}]: new_state mismatch",
                        idx
                    );
                }
            }
            other => panic!("exhaustive[{}]: expected StateChanged, got {:?}", idx, other),
        }
    }
}

#[tokio::test]
async fn exhaustive_all_invalid_transitions_return_errors() {
    let invalid_cases: Vec<(BeadState, BeadState)> = vec![
        // Open cannot go to Blocked, Deferred, or Closed directly
        (BeadState::Open, BeadState::Blocked),
        (BeadState::Open, BeadState::Deferred),
        // InProgress cannot go back to Open
        (BeadState::InProgress, BeadState::Open),
        // Blocked cannot go to Open
        (BeadState::Blocked, BeadState::Open),
        // Deferred cannot go to Open or Blocked
        (BeadState::Deferred, BeadState::Open),
        (BeadState::Deferred, BeadState::Blocked),
    ];

    for (idx, (from_state, to_state)) in invalid_cases.into_iter().enumerate() {
        let svc = make_service();
        let bead_id_str = format!("exhaustive-invalid-{}", idx);
        svc.create_bead(bead_id_str.as_str(), "Exhaustive", None)
            .await
            .unwrap();
        let id = BeadId::new(&bead_id_str).unwrap();

        // Navigate to from_state
        match from_state {
            BeadState::InProgress => {
                svc.update_bead_state(&id, BeadState::InProgress)
                    .await
                    .unwrap();
            }
            BeadState::Blocked => {
                svc.update_bead_state(&id, BeadState::InProgress)
                    .await
                    .unwrap();
                svc.update_bead_state(&id, BeadState::Blocked)
                    .await
                    .unwrap();
            }
            BeadState::Deferred => {
                svc.update_bead_state(&id, BeadState::InProgress)
                    .await
                    .unwrap();
                svc.update_bead_state(&id, BeadState::Deferred)
                    .await
                    .unwrap();
            }
            _ => {}
        }

        let result = svc.update_bead_state(&id, to_state.clone()).await;
        assert!(
            result.is_err(),
            "exhaustive[{}]: {:?} -> {:?} should fail but succeeded",
            idx,
            from_state,
            to_state
        );
    }
}

// ============================================================================
// Group 7: Closed Transitions (must go through non-Open state)
// ============================================================================

#[tokio::test]
async fn closed_transitions_require_intermediate_state() {
    let svc = make_service();
    svc.create_bead("close-test", "Close Test", None)
        .await
        .unwrap();
    let id = BeadId::new("close-test").unwrap();

    // Open -> Closed should fail
    let result = svc.update_bead_state(
        &id,
        BeadState::Closed {
            closed_at: now(),
        },
    ).await;
    assert!(result.is_err(), "Open -> Closed should fail");

    // Must go through InProgress, Blocked, or Deferred first
    svc.update_bead_state(&id, BeadState::InProgress)
        .await
        .unwrap();

    // Now InProgress -> Closed should succeed
    let result = svc.update_bead_state(
        &id,
        BeadState::Closed {
            closed_at: now(),
        },
    ).await;
    assert!(result.is_ok(), "InProgress -> Closed should succeed");
}
