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

// =========================================================================
// Advanced Bead Aggregate Tests
// =========================================================================

mod advanced_bead_tests {
    use super::*;

    fn make_bead(id: &str, title: &str) -> Bead {
        let id = BeadId::new(id).expect("valid id");
        let title = BeadTitle::new(title).expect("valid title");
        Bead::create(id, title, None)
    }

    #[test]
    fn bead_create_with_no_description() {
        let bead = make_bead("bd-001", "No Desc");
        assert!(bead.description().is_none());
        assert!(bead.assignee().is_none());
        assert!(bead.parent().is_none());
        assert!(bead.depends_on().is_empty());
        assert!(bead.blocked_by().is_empty());
        assert!(bead.closed_at().is_none());
    }

    #[test]
    fn bead_create_default_type_is_task() {
        let bead = make_bead("bd-001", "Default Type");
        assert_eq!(bead.bead_type(), BeadType::Task);
    }

    #[test]
    fn bead_create_default_priority_is_medium() {
        let bead = make_bead("bd-001", "Default Priority");
        assert_eq!(bead.priority().as_u8(), 2);
    }

    #[test]
    fn bead_create_created_at_equals_updated_at() {
        let bead = make_bead("bd-001", "Timestamps");
        assert_eq!(bead.created_at(), bead.updated_at());
    }

    #[test]
    fn bead_duplicate_dependency_is_ignored() {
        let bead = make_bead("bd-001", "Dedup Dep");
        let dep = BeadId::new("bd-100").expect("valid");
        let with_one = bead.add_dependency(dep.clone());
        let with_two = with_one.add_dependency(dep);
        assert_eq!(with_two.depends_on().len(), 1);
    }

    #[test]
    fn bead_self_reference_dependency_is_ignored() {
        let bead = make_bead("bd-001", "Self Dep");
        let self_id = BeadId::new("bd-001").expect("valid");
        let result = bead.add_dependency(self_id);
        assert!(result.depends_on().is_empty());
    }

    #[test]
    fn bead_duplicate_blocker_is_ignored() {
        let bead = make_bead("bd-001", "Dedup Block");
        let blocker = BeadId::new("bd-200").expect("valid");
        let with_one = bead.add_blocker(blocker.clone());
        let with_two = with_one.add_blocker(blocker);
        assert_eq!(with_two.blocked_by().len(), 1);
    }

    #[test]
    fn bead_self_reference_blocker_is_ignored() {
        let bead = make_bead("bd-001", "Self Block");
        let self_id = BeadId::new("bd-001").expect("valid");
        let result = bead.add_blocker(self_id);
        assert!(result.blocked_by().is_empty());
    }

    #[test]
    fn bead_with_parent() {
        let bead = make_bead("bd-001", "Child");
        let parent = BeadId::new("bd-parent").expect("valid");
        let with_parent = bead.with_parent(parent);
        assert!(with_parent.parent().is_some());
        assert_eq!(with_parent.parent().map(|p| p.as_str()), Some("bd-parent"));
    }

    #[test]
    fn bead_multiple_dependencies() {
        let bead = make_bead("bd-001", "Multi Dep");
        let d1 = BeadId::new("bd-d1").expect("valid");
        let d2 = BeadId::new("bd-d2").expect("valid");
        let d3 = BeadId::new("bd-d3").expect("valid");
        let with_deps = bead.add_dependency(d1).add_dependency(d2).add_dependency(d3);
        assert_eq!(with_deps.depends_on().len(), 3);
    }

    #[test]
    fn bead_full_lifecycle_open_to_closed() {
        let bead = make_bead("bd-001", "Lifecycle");
        let in_progress = bead.transition(BeadState::InProgress).expect("-> InProgress");
        assert_eq!(in_progress.state(), BeadState::InProgress);

        let blocked = in_progress.transition(BeadState::Blocked).expect("-> Blocked");
        assert_eq!(blocked.state(), BeadState::Blocked);

        let resumed = blocked.transition(BeadState::InProgress).expect("-> InProgress");
        assert_eq!(resumed.state(), BeadState::InProgress);

        let deferred = resumed.transition(BeadState::Deferred).expect("-> Deferred");
        assert_eq!(deferred.state(), BeadState::Deferred);

        let resumed2 = deferred.transition(BeadState::InProgress).expect("-> InProgress");
        let closed = resumed2.transition(BeadState::Closed).expect("-> Closed");
        assert!(closed.closed_at().is_some());
    }

    #[test]
    fn bead_direct_close_from_open() {
        let bead = make_bead("bd-001", "Direct Close");
        assert!(bead.can_transition_to(BeadState::Closed));
        let closed = bead.transition(BeadState::Closed).expect("direct close");
        assert!(!closed.is_blocked());
    }

    #[test]
    fn bead_direct_close_from_in_progress() {
        let bead = make_bead("bd-001", "Close from IP");
        let in_progress = bead.transition(BeadState::InProgress).expect("IP");
        let closed = in_progress.transition(BeadState::Closed).expect("close");
        assert!(closed.closed_at().is_some());
    }

    #[test]
    fn bead_direct_close_from_blocked() {
        let bead = make_bead("bd-001", "Close from Blocked");
        let in_progress = bead.transition(BeadState::InProgress).expect("IP");
        let blocked = in_progress.transition(BeadState::Blocked).expect("blocked");
        let closed = blocked.transition(BeadState::Closed).expect("close");
        assert!(closed.closed_at().is_some());
    }

    #[test]
    fn bead_direct_close_from_deferred() {
        let bead = make_bead("bd-001", "Close from Deferred");
        let in_progress = bead.transition(BeadState::InProgress).expect("IP");
        let deferred = in_progress.transition(BeadState::Deferred).expect("deferred");
        let closed = deferred.transition(BeadState::Closed).expect("close");
        assert!(closed.closed_at().is_some());
    }

    #[test]
    fn bead_cannot_transition_open_to_blocked() {
        let bead = make_bead("bd-001", "No direct blocked");
        assert!(!bead.can_transition_to(BeadState::Blocked));
    }

    #[test]
    fn bead_cannot_transition_open_to_deferred() {
        let bead = make_bead("bd-001", "No direct deferred");
        assert!(!bead.can_transition_to(BeadState::Deferred));
    }

    #[test]
    fn bead_cannot_transition_open_to_closed_via_state_machine() {
        // Bead can transition to Closed from any state via bead.can_transition_to
        let bead = make_bead("bd-001", "Close check");
        assert!(bead.can_transition_to(BeadState::Closed));
    }

    #[test]
    fn bead_cannot_transition_from_closed_to_anything() {
        let bead = make_bead("bd-001", "Terminal");
        let closed = bead.transition(BeadState::Closed).expect("closed");
        assert!(!closed.can_transition_to(BeadState::Open));
        assert!(!closed.can_transition_to(BeadState::InProgress));
        assert!(!closed.can_transition_to(BeadState::Blocked));
        assert!(!closed.can_transition_to(BeadState::Deferred));
    }

    #[test]
    fn bead_blocked_to_deferred() {
        let bead = make_bead("bd-001", "Blocked to Deferred");
        let in_progress = bead.transition(BeadState::InProgress).expect("IP");
        let blocked = in_progress.transition(BeadState::Blocked).expect("blocked");
        let deferred = blocked.transition(BeadState::Deferred).expect("deferred");
        assert_eq!(deferred.state(), BeadState::Deferred);
    }

    #[test]
    fn bead_with_type_overrides() {
        let bead = make_bead("bd-001", "Type chain");
        let as_bug = bead.with_type(BeadType::Bug);
        assert_eq!(as_bug.bead_type(), BeadType::Bug);
        let as_epic = as_bug.with_type(BeadType::Epic);
        assert_eq!(as_epic.bead_type(), BeadType::Epic);
    }

    #[test]
    fn bead_serde_roundtrip() {
        let bead = make_bead("bd-001", "Serialize Me");
        let json = serde_json::to_string(&bead).expect("serialize");
        let parsed: Bead = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(bead, parsed);
    }

    #[test]
    fn bead_getters_work_after_transitions() {
        let bead = make_bead("bd-001", "Getter check");
        let in_progress = bead.transition(BeadState::InProgress).expect("IP");
        assert_eq!(in_progress.id().as_str(), "bd-001");
        assert_eq!(in_progress.title().as_str(), "Getter check");
        assert!(in_progress.description().is_none());
        assert_eq!(in_progress.bead_type(), BeadType::Task);
    }

    #[test]
    fn bead_is_blocked_false_when_empty() {
        let bead = make_bead("bd-001", "Not Blocked");
        assert!(!bead.is_blocked());
    }

    #[test]
    fn bead_is_blocked_true_after_adding_blocker() {
        let bead = make_bead("bd-001", "Blocked Now");
        let blocker = BeadId::new("bd-blocker").expect("valid");
        let blocked = bead.add_blocker(blocker);
        assert!(blocked.is_blocked());
    }

    // =========================================================================
    // Bead Invalid Transition Tests
    // =========================================================================

    mod invalid_transition_tests {
        use super::*;

        fn make_bead(id: &str, title: &str) -> Bead {
            let id = BeadId::new(id).expect("valid id");
            let title = BeadTitle::new(title).expect("valid title");
            Bead::create(id, title, None)
        }

        #[test]
        fn bead_open_to_blocked_rejects() {
            let bead = make_bead("bd-001", "No direct block");
            let result = bead.transition(BeadState::Blocked);
            assert!(result.is_err());
        }

        #[test]
        fn bead_open_to_deferred_rejects() {
            let bead = make_bead("bd-001", "No direct defer");
            let result = bead.transition(BeadState::Deferred);
            assert!(result.is_err());
        }

        #[test]
        fn bead_open_to_open_self_loop_rejects() {
            let bead = make_bead("bd-001", "No self-loop open");
            let result = bead.transition(BeadState::Open);
            assert!(result.is_err());
        }

        #[test]
        fn bead_in_progress_to_in_progress_self_loop_rejects() {
            let bead = make_bead("bd-001", "No self-loop ip");
            let in_progress = bead.transition(BeadState::InProgress).expect("ip");
            let result = in_progress.transition(BeadState::InProgress);
            assert!(result.is_err());
        }

        #[test]
        fn bead_blocked_to_open_rejects() {
            let bead = make_bead("bd-001", "Blocked -> Open");
            let ip = bead.transition(BeadState::InProgress).expect("ip");
            let blocked = ip.transition(BeadState::Blocked).expect("blocked");
            let result = blocked.transition(BeadState::Open);
            assert!(result.is_err());
        }

        #[test]
        fn bead_deferred_to_blocked_rejects() {
            let bead = make_bead("bd-001", "Deferred -> Blocked");
            let ip = bead.transition(BeadState::InProgress).expect("ip");
            let deferred = ip.transition(BeadState::Deferred).expect("deferred");
            let result = deferred.transition(BeadState::Blocked);
            assert!(result.is_err());
        }

        #[test]
        fn bead_deferred_to_open_rejects() {
            let bead = make_bead("bd-001", "Deferred -> Open");
            let ip = bead.transition(BeadState::InProgress).expect("ip");
            let deferred = ip.transition(BeadState::Deferred).expect("deferred");
            let result = deferred.transition(BeadState::Open);
            assert!(result.is_err());
        }

        #[test]
        fn bead_deferred_to_deferred_self_loop_rejects() {
            let bead = make_bead("bd-001", "No self-loop deferred");
            let ip = bead.transition(BeadState::InProgress).expect("ip");
            let deferred = ip.transition(BeadState::Deferred).expect("deferred");
            let result = deferred.transition(BeadState::Deferred);
            assert!(result.is_err());
        }

        #[test]
        fn bead_blocked_to_blocked_self_loop_rejects() {
            let bead = make_bead("bd-001", "No self-loop blocked");
            let ip = bead.transition(BeadState::InProgress).expect("ip");
            let blocked = ip.transition(BeadState::Blocked).expect("blocked");
            let result = blocked.transition(BeadState::Blocked);
            assert!(result.is_err());
        }

        #[test]
        fn bead_closed_to_closed_self_loop_succeeds() {
            // Bead's can_transition_to always returns true for transitions TO Closed,
            // and the transition() method follows the same logic (try_transition_to_closed
            // succeeds for any new_state == Closed).
            let bead = make_bead("bd-001", "Closed -> Closed allowed");
            let closed = bead.transition(BeadState::Closed).expect("closed");
            let result = closed.transition(BeadState::Closed);
            assert!(result.is_ok(), "Closed -> Closed is allowed in this bead implementation");
        }

        #[test]
        fn bead_closed_to_blocked_rejects() {
            let bead = make_bead("bd-001", "Closed -> Blocked");
            let closed = bead.transition(BeadState::Closed).expect("closed");
            let result = closed.transition(BeadState::Blocked);
            assert!(result.is_err());
        }

        #[test]
        fn bead_closed_to_deferred_rejects() {
            let bead = make_bead("bd-001", "Closed -> Deferred");
            let closed = bead.transition(BeadState::Closed).expect("closed");
            let result = closed.transition(BeadState::Deferred);
            assert!(result.is_err());
        }

        #[test]
        fn bead_closed_to_in_progress_rejects() {
            let bead = make_bead("bd-001", "Closed -> IP");
            let closed = bead.transition(BeadState::Closed).expect("closed");
            let result = closed.transition(BeadState::InProgress);
            assert!(result.is_err());
        }
    }
}
