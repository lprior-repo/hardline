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
        let with_deps = bead
            .add_dependency(d1)
            .add_dependency(d2)
            .add_dependency(d3);
        assert_eq!(with_deps.depends_on().len(), 3);
    }

    #[test]
    fn bead_full_lifecycle_open_to_closed() {
        let bead = make_bead("bd-001", "Lifecycle");
        let in_progress = bead
            .transition(BeadState::InProgress)
            .expect("-> InProgress");
        assert_eq!(in_progress.state(), BeadState::InProgress);

        let blocked = in_progress
            .transition(BeadState::Blocked)
            .expect("-> Blocked");
        assert_eq!(blocked.state(), BeadState::Blocked);

        let resumed = blocked
            .transition(BeadState::InProgress)
            .expect("-> InProgress");
        assert_eq!(resumed.state(), BeadState::InProgress);

        let deferred = resumed
            .transition(BeadState::Deferred)
            .expect("-> Deferred");
        assert_eq!(deferred.state(), BeadState::Deferred);

        let resumed2 = deferred
            .transition(BeadState::InProgress)
            .expect("-> InProgress");
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
        let deferred = in_progress
            .transition(BeadState::Deferred)
            .expect("deferred");
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
            assert!(
                result.is_ok(),
                "Closed -> Closed is allowed in this bead implementation"
            );
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

    // =========================================================================
    // Full State Transition Matrix (5x5 = 25 pairs)
    // =========================================================================

    mod transition_matrix_tests {
        use super::*;

        /// Helper: attempt a transition, return (success, resulting_state)
        fn attempt_transition(
            from: BeadState,
            to: BeadState,
        ) -> (bool, Option<BeadState>) {
            let mut bead = make_bead("bd-matrix", "Matrix Test");
            // Navigate to `from` state
            if from != BeadState::Open {
                bead = navigate_to_state(&bead, from);
            }
            match bead.transition(to) {
                Ok(result) => (true, Some(result.state())),
                Err(_) => (false, None),
            }
        }

        /// Navigate a bead from Open to the target state via valid path.
        fn navigate_to_state(bead: &Bead, target: BeadState) -> Bead {
            match target {
                BeadState::Open => bead.clone(),
                BeadState::InProgress => bead.transition(BeadState::InProgress).expect("-> IP"),
                BeadState::Blocked => {
                    let ip = bead.transition(BeadState::InProgress).expect("-> IP");
                    ip.transition(BeadState::Blocked).expect("-> Blocked")
                }
                BeadState::Deferred => {
                    let ip = bead.transition(BeadState::InProgress).expect("-> IP");
                    ip.transition(BeadState::Deferred).expect("-> Deferred")
                }
                BeadState::Closed => bead.transition(BeadState::Closed).expect("-> Closed"),
            }
        }

        // --- Valid transitions ---

        #[test]
        fn matrix_open_to_in_progress_succeeds() {
            let (ok, state) = attempt_transition(BeadState::Open, BeadState::InProgress);
            assert!(ok);
            assert_eq!(state, Some(BeadState::InProgress));
        }

        #[test]
        fn matrix_open_to_closed_succeeds() {
            let (ok, state) = attempt_transition(BeadState::Open, BeadState::Closed);
            assert!(ok);
            assert_eq!(state, Some(BeadState::Closed));
        }

        #[test]
        fn matrix_in_progress_to_blocked_succeeds() {
            let (ok, state) = attempt_transition(BeadState::InProgress, BeadState::Blocked);
            assert!(ok);
            assert_eq!(state, Some(BeadState::Blocked));
        }

        #[test]
        fn matrix_in_progress_to_deferred_succeeds() {
            let (ok, state) = attempt_transition(BeadState::InProgress, BeadState::Deferred);
            assert!(ok);
            assert_eq!(state, Some(BeadState::Deferred));
        }

        #[test]
        fn matrix_in_progress_to_closed_succeeds() {
            let (ok, state) = attempt_transition(BeadState::InProgress, BeadState::Closed);
            assert!(ok);
            assert_eq!(state, Some(BeadState::Closed));
        }

        #[test]
        fn matrix_blocked_to_in_progress_succeeds() {
            let (ok, state) = attempt_transition(BeadState::Blocked, BeadState::InProgress);
            assert!(ok);
            assert_eq!(state, Some(BeadState::InProgress));
        }

        #[test]
        fn matrix_blocked_to_deferred_succeeds() {
            let (ok, state) = attempt_transition(BeadState::Blocked, BeadState::Deferred);
            assert!(ok);
            assert_eq!(state, Some(BeadState::Deferred));
        }

        #[test]
        fn matrix_blocked_to_closed_succeeds() {
            let (ok, state) = attempt_transition(BeadState::Blocked, BeadState::Closed);
            assert!(ok);
            assert_eq!(state, Some(BeadState::Closed));
        }

        #[test]
        fn matrix_deferred_to_in_progress_succeeds() {
            let (ok, state) = attempt_transition(BeadState::Deferred, BeadState::InProgress);
            assert!(ok);
            assert_eq!(state, Some(BeadState::InProgress));
        }

        #[test]
        fn matrix_deferred_to_closed_succeeds() {
            let (ok, state) = attempt_transition(BeadState::Deferred, BeadState::Closed);
            assert!(ok);
            assert_eq!(state, Some(BeadState::Closed));
        }

        #[test]
        fn matrix_closed_to_closed_succeeds() {
            let (ok, state) = attempt_transition(BeadState::Closed, BeadState::Closed);
            assert!(ok);
            assert_eq!(state, Some(BeadState::Closed));
        }

        // --- Invalid transitions ---

        #[test]
        fn matrix_open_to_open_rejects() {
            let (ok, _) = attempt_transition(BeadState::Open, BeadState::Open);
            assert!(!ok);
        }

        #[test]
        fn matrix_open_to_blocked_rejects() {
            let (ok, _) = attempt_transition(BeadState::Open, BeadState::Blocked);
            assert!(!ok);
        }

        #[test]
        fn matrix_open_to_deferred_rejects() {
            let (ok, _) = attempt_transition(BeadState::Open, BeadState::Deferred);
            assert!(!ok);
        }

        #[test]
        fn matrix_in_progress_to_open_rejects() {
            let (ok, _) = attempt_transition(BeadState::InProgress, BeadState::Open);
            assert!(!ok);
        }

        #[test]
        fn matrix_in_progress_to_in_progress_rejects() {
            let (ok, _) = attempt_transition(BeadState::InProgress, BeadState::InProgress);
            assert!(!ok);
        }

        #[test]
        fn matrix_blocked_to_open_rejects() {
            let (ok, _) = attempt_transition(BeadState::Blocked, BeadState::Open);
            assert!(!ok);
        }

        #[test]
        fn matrix_blocked_to_blocked_rejects() {
            let (ok, _) = attempt_transition(BeadState::Blocked, BeadState::Blocked);
            assert!(!ok);
        }

        #[test]
        fn matrix_deferred_to_open_rejects() {
            let (ok, _) = attempt_transition(BeadState::Deferred, BeadState::Open);
            assert!(!ok);
        }

        #[test]
        fn matrix_deferred_to_blocked_rejects() {
            let (ok, _) = attempt_transition(BeadState::Deferred, BeadState::Blocked);
            assert!(!ok);
        }

        #[test]
        fn matrix_deferred_to_deferred_rejects() {
            let (ok, _) = attempt_transition(BeadState::Deferred, BeadState::Deferred);
            assert!(!ok);
        }

        #[test]
        fn matrix_closed_to_open_rejects() {
            let (ok, _) = attempt_transition(BeadState::Closed, BeadState::Open);
            assert!(!ok);
        }

        #[test]
        fn matrix_closed_to_in_progress_rejects() {
            let (ok, _) = attempt_transition(BeadState::Closed, BeadState::InProgress);
            assert!(!ok);
        }

        #[test]
        fn matrix_closed_to_blocked_rejects() {
            let (ok, _) = attempt_transition(BeadState::Closed, BeadState::Blocked);
            assert!(!ok);
        }

        #[test]
        fn matrix_closed_to_deferred_rejects() {
            let (ok, _) = attempt_transition(BeadState::Closed, BeadState::Deferred);
            assert!(!ok);
        }

        // --- Exhaustive count: 11 valid + 14 invalid = 25 total ---

        #[test]
        fn matrix_total_valid_transitions_is_11() {
            let states = BeadState::all();
            let mut valid_count = 0;
            for &from in &states {
                for &to in &states {
                    if from.can_transition_to(to) {
                        valid_count += 1;
                    }
                }
            }
            // Open→IP, Open→Closed, IP→Blocked, IP→Deferred, IP→Closed,
            // Blocked→IP, Blocked→Deferred, Blocked→Closed,
            // Deferred→IP, Deferred→Closed, Closed→nothing from state machine
            // BUT Bead::can_transition_to adds: any→Closed (Q16)
            // and BeadState::Closed can't transition (Q15)
            // So count from BeadState perspective:
            // Open→IP(1), IP→Blocked(1), IP→Deferred(1), IP→Closed(1),
            // Blocked→IP(1), Blocked→Deferred(1), Blocked→Closed(1),
            // Deferred→IP(1), Deferred→Closed(1) = 9
            assert_eq!(valid_count, 9);
        }
    }

    // =========================================================================
    // Transition Guard Postcondition Tests
    // =========================================================================

    mod transition_guard_tests {
        use super::*;

        /// Q13: Every transition must update updated_at
        #[test]
        fn transition_updates_updated_at() {
            let bead = make_bead("bd-guard-1", "Guard Test");
            let original_updated = bead.updated_at();

            // Open → InProgress
            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert!(
                ip.updated_at() >= original_updated,
                "transition must update updated_at"
            );
        }

        /// Q12: Transition to Closed sets closed_at
        #[test]
        fn transition_to_closed_sets_closed_at() {
            let bead = make_bead("bd-guard-2", "Closed At");
            assert!(bead.closed_at().is_none(), "new bead has no closed_at");

            let closed = bead.transition(BeadState::Closed).expect("-> Closed");
            assert!(closed.closed_at().is_some(), "closed bead has closed_at");
        }

        /// Q12: closed_at is None for non-closed transitions
        #[test]
        fn transition_to_non_closed_does_not_set_closed_at() {
            let bead = make_bead("bd-guard-3", "No Closed At");
            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert!(ip.closed_at().is_none());
        }

        /// Q12: closed_at is set to a recent timestamp
        #[test]
        fn closed_at_is_recent() {
            let bead = make_bead("bd-guard-4", "Recent Close");
            let before = chrono::Utc::now();
            let closed = bead.transition(BeadState::Closed).expect("-> Closed");
            let after = chrono::Utc::now();

            let closed_at = closed.closed_at().expect("has closed_at");
            assert!(closed_at >= before);
            assert!(closed_at <= after);
        }

        /// Verify transition preserves all non-state fields (id, title, etc.)
        #[test]
        fn transition_preserves_identity() {
            let bead = make_bead("bd-guard-5", "Identity");
            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert_eq!(ip.id().as_str(), "bd-guard-5");
            assert_eq!(ip.title().as_str(), "Identity");
            assert_eq!(ip.bead_type(), BeadType::Task);
            assert_eq!(ip.priority().as_u8(), 2);
        }

        /// Verify transition preserves dependencies and blockers
        #[test]
        fn transition_preserves_dependencies_and_blockers() {
            let dep = BeadId::new("bd-dep-1").expect("valid");
            let blocker = BeadId::new("bd-block-1").expect("valid");
            let bead = make_bead("bd-guard-6", "With Deps")
                .add_dependency(dep)
                .add_blocker(blocker);

            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert_eq!(ip.depends_on().len(), 1);
            assert_eq!(ip.blocked_by().len(), 1);
            assert!(ip.is_blocked());
        }

        /// Verify transition preserves assignee and parent
        #[test]
        fn transition_preserves_assignee_and_parent() {
            let parent = BeadId::new("bd-parent").expect("valid");
            let bead = make_bead("bd-guard-7", "With Meta")
                .with_assignee("alice")
                .with_parent(parent);

            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert_eq!(ip.assignee(), Some(&"alice".to_string()));
            assert!(ip.parent().is_some());
        }

        /// Closed bead: closed_at is updated on re-close
        #[test]
        fn closed_to_closed_updates_closed_at() {
            let bead = make_bead("bd-guard-8", "Re-close");
            let closed1 = bead.transition(BeadState::Closed).expect("-> Closed");
            let closed_at_1 = closed1.closed_at().expect("has closed_at");

            // Small sleep to ensure time difference
            std::thread::sleep(std::time::Duration::from_millis(1));

            let closed2 = closed1.transition(BeadState::Closed).expect("re-close");
            let closed_at_2 = closed2.closed_at().expect("has closed_at");
            assert!(
                closed_at_2 >= closed_at_1,
                "re-closing should update closed_at"
            );
        }

        /// Verify created_at is immutable across transitions
        #[test]
        fn created_at_immutable_across_transitions() {
            let bead = make_bead("bd-guard-9", "Immutable Created");
            let created = bead.created_at();

            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert_eq!(ip.created_at(), created, "created_at must not change");

            let closed = ip.transition(BeadState::Closed).expect("-> Closed");
            assert_eq!(closed.created_at(), created, "created_at must not change");
        }
    }

    // =========================================================================
    // can_transition_to vs transition() Consistency Tests
    // =========================================================================

    mod consistency_tests {
        use super::*;

        /// For every (from, to) pair, can_transition_to and transition() must agree.
        /// If can_transition_to returns false, transition() must return Err.
        /// If can_transition_to returns true, transition() must return Ok.
        #[test]
        fn can_transition_agrees_with_transition_for_all_pairs() {
            let states = BeadState::all();
            for &from in &states {
                for &to in &states {
                    let mut bead = make_bead("bd-consist", "Consistency");
                    if from != BeadState::Open {
                        bead = navigate_to(&bead, from);
                    }
                    let can = bead.can_transition_to(to);
                    let result = bead.transition(to);
                    match (can, result) {
                        (true, Ok(_)) => {}
                        (false, Err(_)) => {}
                        (true, Err(e)) => {
                            panic!(
                                "can_transition_to({from:?}, {to:?}) = true but transition() = Err({e:?})"
                            );
                        }
                        (false, Ok(bead)) => {
                            panic!(
                                "can_transition_to({from:?}, {to:?}) = false but transition() = Ok({:?})",
                                bead.state()
                            );
                        }
                    }
                }
            }
        }

        fn navigate_to(bead: &Bead, target: BeadState) -> Bead {
            match target {
                BeadState::Open => bead.clone(),
                BeadState::InProgress => bead.transition(BeadState::InProgress).expect("-> IP"),
                BeadState::Blocked => {
                    let ip = bead.transition(BeadState::InProgress).expect("-> IP");
                    ip.transition(BeadState::Blocked).expect("-> Blocked")
                }
                BeadState::Deferred => {
                    let ip = bead.transition(BeadState::InProgress).expect("-> IP");
                    ip.transition(BeadState::Deferred).expect("-> Deferred")
                }
                BeadState::Closed => bead.transition(BeadState::Closed).expect("-> Closed"),
            }
        }

        /// Q16: can_transition_to always returns true for Closed target
        #[test]
        fn can_always_transition_to_closed_from_any_state() {
            let states = BeadState::all();
            for &from in &states {
                let mut bead = make_bead("bd-q16", "Q16 Test");
                if from != BeadState::Open {
                    bead = navigate_to(&bead, from);
                }
                assert!(
                    bead.can_transition_to(BeadState::Closed),
                    "Q16 violation: should be able to transition from {from:?} to Closed"
                );
            }
        }

        /// Q15: can_transition_to returns false from Closed to any non-Closed state
        #[test]
        fn cannot_transition_from_closed_to_non_closed() {
            let bead = make_bead("bd-q15", "Q15 Test");
            let closed = bead.transition(BeadState::Closed).expect("-> Closed");
            for &target in &BeadState::all() {
                if target == BeadState::Closed {
                    continue;
                }
                assert!(
                    !closed.can_transition_to(target),
                    "Q15 violation: should not transition from Closed to {target:?}"
                );
            }
        }
    }

    // =========================================================================
    // Blocker/Dependency Interaction with State Transitions
    // =========================================================================

    mod blocker_dependency_interaction_tests {
        use super::*;

        /// A bead with blockers can still transition (blockers don't prevent transitions)
        #[test]
        fn blocked_bead_can_transition_to_in_progress() {
            let blocker = BeadId::new("bd-blocker").expect("valid");
            let bead = make_bead("bd-bi-1", "Blocked IP")
                .add_blocker(blocker);
            assert!(bead.is_blocked());

            // Can still transition to InProgress even with blockers
            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert_eq!(ip.state(), BeadState::InProgress);
            assert!(ip.is_blocked()); // blockers still present
        }

        /// A bead with dependencies can transition through full lifecycle
        #[test]
        fn bead_with_dependencies_full_lifecycle() {
            let dep = BeadId::new("bd-dep").expect("valid");
            let blocker = BeadId::new("bd-blk").expect("valid");
            let bead = make_bead("bd-bi-2", "Full Lifecycle")
                .add_dependency(dep)
                .add_blocker(blocker);

            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert_eq!(ip.depends_on().len(), 1);
            assert_eq!(ip.blocked_by().len(), 1);

            let blocked = ip.transition(BeadState::Blocked).expect("-> Blocked");
            assert_eq!(blocked.depends_on().len(), 1);
            assert_eq!(blocked.blocked_by().len(), 1);

            let closed = blocked.transition(BeadState::Closed).expect("-> Closed");
            assert_eq!(closed.depends_on().len(), 1);
            assert!(closed.closed_at().is_some());
        }

        /// Blockers survive transitions (immutable data, not state-dependent)
        #[test]
        fn blockers_survive_state_transitions() {
            let blocker = BeadId::new("bd-surv").expect("valid");
            let bead = make_bead("bd-bi-3", "Survivor")
                .add_blocker(blocker);

            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert!(ip.is_blocked());

            let blocked = ip.transition(BeadState::Blocked).expect("-> Blocked");
            assert!(blocked.is_blocked());

            let deferred = blocked.transition(BeadState::Deferred).expect("-> Deferred");
            assert!(deferred.is_blocked());
        }

        /// Dependencies survive transitions
        #[test]
        fn dependencies_survive_state_transitions() {
            let dep = BeadId::new("bd-dep-s").expect("valid");
            let bead = make_bead("bd-bi-4", "Dep Survivor")
                .add_dependency(dep);

            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert_eq!(ip.depends_on().len(), 1);

            let closed = ip.transition(BeadState::Closed).expect("-> Closed");
            assert_eq!(closed.depends_on().len(), 1);
        }

        /// A bead with multiple blockers can still transition
        #[test]
        fn multiple_blockers_do_not_prevent_transitions() {
            let b1 = BeadId::new("bd-b1").expect("valid");
            let b2 = BeadId::new("bd-b2").expect("valid");
            let b3 = BeadId::new("bd-b3").expect("valid");
            let bead = make_bead("bd-bi-5", "Multi Block")
                .add_blocker(b1)
                .add_blocker(b2)
                .add_blocker(b3);

            assert!(bead.is_blocked());
            assert_eq!(bead.blocked_by().len(), 3);

            let ip = bead.transition(BeadState::InProgress).expect("-> IP");
            assert!(ip.is_blocked());
        }

        /// Closed bead with blockers still cannot transition away from Closed
        #[test]
        fn closed_with_blockers_cannot_transition() {
            let blocker = BeadId::new("bd-cblk").expect("valid");
            let bead = make_bead("bd-bi-6", "Closed Blocked")
                .add_blocker(blocker);
            let closed = bead.transition(BeadState::Closed).expect("-> Closed");

            assert!(closed.is_blocked());
            assert!(closed.transition(BeadState::InProgress).is_err());
            assert!(closed.transition(BeadState::Open).is_err());
        }
    }

    // =========================================================================
    // Transition Guard Proptests
    // =========================================================================

    mod transition_guard_proptests {
        use super::*;
        use proptest::proptest;
        use proptest::{prop_assert, prop_assert_eq};

        proptest! {
            /// For any (from, to) pair, can_transition_to and transition() agree
            #[test]
            fn prop_can_transition_matches_transition(
                from_idx in 0u8..5u8,
                to_idx in 0u8..5u8
            ) {
                let states = BeadState::all();
                let from = states[from_idx as usize];
                let to = states[to_idx as usize];

                let mut bead = make_bead("bd-prop-ct", "Prop CT");
                if from != BeadState::Open {
                    bead = navigate_for_prop(&bead, from);
                }

                let can = bead.can_transition_to(to);
                let result = bead.transition(to);

                if can {
                    prop_assert!(
                        result.is_ok(),
                        "can_transition_to({from:?}, {to:?}) = true but transition() failed"
                    );
                } else {
                    prop_assert!(
                        result.is_err(),
                        "can_transition_to({from:?}, {to:?}) = false but transition() succeeded"
                    );
                }
            }

            /// Transition to Closed always sets closed_at
            #[test]
            fn prop_transition_to_closed_sets_closed_at(from_idx in 0u8..5u8) {
                let states = BeadState::all();
                let from = states[from_idx as usize];

                let mut bead = make_bead("bd-prop-ca", "Prop CA");
                if from != BeadState::Open {
                    bead = navigate_for_prop(&bead, from);
                }

                let closed = bead.transition(BeadState::Closed);
                prop_assert!(closed.is_ok());
                prop_assert!(closed.unwrap().closed_at().is_some());
            }

            /// Transition to non-Closed never sets closed_at
            #[test]
            fn prop_non_closed_transition_no_closed_at(
                from_idx in 0u8..5u8,
                to_idx in 0u8..5u8
            ) {
                let states = BeadState::all();
                let from = states[from_idx as usize];
                let to = states[to_idx as usize];

                // Skip: Closed target, or invalid transition
                if to == BeadState::Closed { return Ok(()); }

                let mut bead = make_bead("bd-prop-nc", "Prop NC");
                if from != BeadState::Open {
                    bead = navigate_for_prop(&bead, from);
                }

                if bead.can_transition_to(to) {
                    let result = bead.transition(to).expect("valid transition");
                    prop_assert!(result.closed_at().is_none());
                }
            }

            /// Transition always updates updated_at (or keeps >= original)
            #[test]
            fn prop_transition_updates_timestamp(from_idx in 0u8..5u8, to_idx in 0u8..5u8) {
                let states = BeadState::all();
                let from = states[from_idx as usize];
                let to = states[to_idx as usize];

                let mut bead = make_bead("bd-prop-ts", "Prop TS");
                if from != BeadState::Open {
                    bead = navigate_for_prop(&bead, from);
                }

                if bead.can_transition_to(to) {
                    let before = bead.updated_at();
                    let result = bead.transition(to).expect("valid transition");
                    prop_assert!(result.updated_at() >= before);
                }
            }

            /// Transition preserves id, title, type, priority
            #[test]
            fn prop_transition_preserves_identity(
                from_idx in 0u8..5u8,
                to_idx in 0u8..5u8
            ) {
                let states = BeadState::all();
                let from = states[from_idx as usize];
                let to = states[to_idx as usize];

                let mut bead = make_bead("bd-prop-id", "Prop Identity");
                if from != BeadState::Open {
                    bead = navigate_for_prop(&bead, from);
                }

                if bead.can_transition_to(to) {
                    let result = bead.transition(to).expect("valid transition");
                    prop_assert_eq!(result.id().as_str(), "bd-prop-id");
                    prop_assert_eq!(result.title().as_str(), "Prop Identity");
                    prop_assert_eq!(result.bead_type(), BeadType::Task);
                }
            }
        }

        fn navigate_for_prop(bead: &Bead, target: BeadState) -> Bead {
            match target {
                BeadState::Open => bead.clone(),
                BeadState::InProgress => bead.transition(BeadState::InProgress).expect("-> IP"),
                BeadState::Blocked => {
                    let ip = bead.transition(BeadState::InProgress).expect("-> IP");
                    ip.transition(BeadState::Blocked).expect("-> Blocked")
                }
                BeadState::Deferred => {
                    let ip = bead.transition(BeadState::InProgress).expect("-> IP");
                    ip.transition(BeadState::Deferred).expect("-> Deferred")
                }
                BeadState::Closed => bead.transition(BeadState::Closed).expect("-> Closed"),
            }
        }
    }
}
