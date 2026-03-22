//! Tests for the Bead aggregate.

use crate::domain::bead::Bead;
use crate::domain::bead_state::BeadState;
use crate::domain::bead_types::BeadType;
use crate::domain::bead_value::{BeadDescription, BeadId, BeadTitle};
use crate::domain::Priority;

#[test]
fn bead_create_sets_open_state() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);

    assert_eq!(bead.state(), BeadState::Open);
    assert!(!bead.is_blocked());
}

#[test]
fn bead_transition_to_in_progress() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);
    let in_progress = bead.transition(BeadState::InProgress).unwrap();

    assert_eq!(in_progress.state(), BeadState::InProgress);
}

#[test]
fn bead_transition_to_closed_sets_closed_at() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);
    let closed = bead.transition(BeadState::Closed).unwrap();

    assert_eq!(closed.state(), BeadState::Closed);
    assert!(closed.closed_at().is_some());
}

#[test]
fn bead_cannot_transition_from_closed() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);
    let closed = bead.transition(BeadState::Closed).unwrap();
    let result = closed.transition(BeadState::InProgress);

    assert!(result.is_err());
}

#[test]
fn bead_add_dependency() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);
    let dep_id = BeadId::new("bd-456").unwrap();
    let with_dep = bead.add_dependency(dep_id);

    assert_eq!(with_dep.depends_on().len(), 1);
}

#[test]
fn bead_add_blocker() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);
    let blocker_id = BeadId::new("bd-456").unwrap();
    let blocked = bead.add_blocker(blocker_id);

    assert!(blocked.is_blocked());
}

#[test]
fn bead_with_priority() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);
    let priority = Priority::new(1).unwrap();
    let with_prio = bead.with_priority(priority);

    assert_eq!(with_prio.priority().as_u8(), 1);
}

#[test]
fn bead_with_type() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);
    let with_type = bead.with_type(BeadType::Bug);

    assert_eq!(with_type.bead_type(), BeadType::Bug);
}

#[test]
fn bead_with_assignee() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);
    let with_assignee = bead.with_assignee("alice");

    assert_eq!(with_assignee.assignee(), Some(&"alice".to_string()));
}

#[test]
fn bead_description() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let desc = BeadDescription::new("A description").unwrap();
    let bead = Bead::create(id, title, Some(desc));

    assert!(bead.description().is_some());
}

#[test]
fn bead_can_transition_to_closed_always() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);

    // Can always transition to Closed
    assert!(bead.can_transition_to(BeadState::Closed));
}

#[test]
fn bead_cannot_transition_from_closed() {
    let id = BeadId::new("bd-123").unwrap();
    let title = BeadTitle::new("Test Bead").unwrap();
    let bead = Bead::create(id, title, None);
    let closed = bead.transition(BeadState::Closed).unwrap();

    assert!(!closed.can_transition_to(BeadState::InProgress));
}
