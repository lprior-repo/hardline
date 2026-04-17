//! Red Queen: Adversarial evolutionary test suite for scp-core
//!
//! Co-evolving tests that attack domain invariants from every angle.
//! Tests are organized by attack surface:
//! - State machine transition matrices (exhaustive pairwise)
//! - Validation boundary erosion (fuzzing-style edge cases)
//! - DAG invariant preservation under mutation
//! - Error type consistency contracts
//! - Type state pattern enforcement
//! - Cross-module consistency checks

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(unused_variables)]

use proptest::prelude::*;
use proptest::prop_assert;
use proptest::prop_assert_eq;

use crate::dag::{BranchDag, BranchId};
use crate::queue::{Priority, QueueStatus};
use crate::session_state::{SessionState, SessionStateManager, StateTransition};
use crate::type_branch_state::BranchState;
use crate::type_file_change::{ChangesSummary, DiffSummary, FileChange, FileDiffStat, FileStatus};
use crate::type_session_id::SessionId;
use crate::type_session_name::SessionName;
use crate::type_session_status::{Operation, SessionStatus};
use crate::workspace_state::{WorkspaceState, WorkspaceStateFilter, WorkspaceStateTransition};

// ═══════════════════════════════════════════════════════════════════════════
// 1. SESSION STATUS STATE MACHINE — EXHAUSTIVE TRANSITION MATRIX
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rq_session_status_exhaustive_transition_matrix() {
    let all = SessionStatus::all_states();

    // Build ground-truth valid transition set from valid_next_states()
    let mut valid_set = std::collections::HashSet::new();
    for from in all.iter().copied() {
        for to in from.valid_next_states().iter().copied() {
            valid_set.insert((from, to));
        }
    }

    // Verify every pair matches can_transition_to
    for from in all.iter().copied() {
        for to in all.iter().copied() {
            let can = from.can_transition_to(to);
            let in_set = valid_set.contains(&(from, to));
            assert_eq!(
                can, in_set,
                "INCONSISTENCY: {:?} -> {:?}: can_transition_to={can}, in_valid_set={in_set}",
                from, to
            );
        }
    }

    // Specific known-valid transitions
    let valid: Vec<(SessionStatus, SessionStatus)> = vec![
        (SessionStatus::Creating, SessionStatus::Active),
        (SessionStatus::Creating, SessionStatus::Failed),
        (SessionStatus::Active, SessionStatus::Paused),
        (SessionStatus::Active, SessionStatus::Completed),
        (SessionStatus::Paused, SessionStatus::Active),
        (SessionStatus::Paused, SessionStatus::Completed),
    ];
    for (from, to) in &valid {
        assert!(from.can_transition_to(*to), "{:?} -> {:?} should be valid", from, to);
        assert!(
            from.valid_next_states().contains(to),
            "{:?} -> {:?} should be in valid_next_states",
            from, to
        );
    }

    // Specific known-invalid transitions (adversarial)
    let invalid: Vec<(SessionStatus, SessionStatus)> = vec![
        (SessionStatus::Creating, SessionStatus::Creating),
        (SessionStatus::Creating, SessionStatus::Paused),
        (SessionStatus::Creating, SessionStatus::Completed),
        (SessionStatus::Active, SessionStatus::Creating),
        (SessionStatus::Active, SessionStatus::Active),
        (SessionStatus::Active, SessionStatus::Failed),
        (SessionStatus::Paused, SessionStatus::Creating),
        (SessionStatus::Paused, SessionStatus::Paused),
        (SessionStatus::Paused, SessionStatus::Failed),
        (SessionStatus::Completed, SessionStatus::Creating),
        (SessionStatus::Completed, SessionStatus::Active),
        (SessionStatus::Completed, SessionStatus::Paused),
        (SessionStatus::Completed, SessionStatus::Failed),
        (SessionStatus::Completed, SessionStatus::Completed),
        (SessionStatus::Failed, SessionStatus::Creating),
        (SessionStatus::Failed, SessionStatus::Active),
        (SessionStatus::Failed, SessionStatus::Paused),
        (SessionStatus::Failed, SessionStatus::Completed),
        (SessionStatus::Failed, SessionStatus::Failed),
    ];
    for (from, to) in &invalid {
        assert!(
            !from.can_transition_to(*to),
            "{:?} -> {:?} should be INVALID",
            from, to
        );
    }
}

#[test]
fn rq_session_status_no_self_transitions_any_state() {
    for state in SessionStatus::all_states().iter().copied() {
        assert!(
            !state.can_transition_to(state),
            "Self-transition MUST be rejected for {:?}",
            state
        );
    }
}

#[test]
fn rq_session_status_operations_exhaustive() {
    let all_ops = [Operation::Status, Operation::Diff, Operation::Focus, Operation::Remove];

    for status in SessionStatus::all_states().iter().copied() {
        let allowed = status.allowed_operations();
        for &op in &all_ops {
            let via_allows = status.allows_operation(op);
            let in_list = allowed.contains(&op);
            assert_eq!(
                via_allows, in_list,
                "INCONSISTENCY: {:?} allows {:?}: allows_op={via_allows}, in_list={in_list}",
                status, op
            );
        }
    }
}

#[test]
fn rq_session_status_terminal_states_have_no_valid_transitions() {
    for state in SessionStatus::all_states().iter().copied() {
        if state.is_terminal() {
            assert!(
                state.valid_next_states().is_empty(),
                "Terminal state {:?} should have no valid transitions",
                state
            );
            for next in SessionStatus::all_states().iter().copied() {
                assert!(
                    !state.can_transition_to(next),
                    "Terminal state {:?} should reject transition to {:?}",
                    state, next
                );
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. WORKSPACE STATE MACHINE — EXHAUSTIVE TRANSITION MATRIX
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rq_workspace_state_exhaustive_transition_matrix() {
    let all = WorkspaceState::all();

    for from in all.iter().copied() {
        let valid_nexts = from.valid_next_states();
        for to in all.iter().copied() {
            let can = from.can_transition_to(to);
            let in_list = valid_nexts.contains(&to);
            assert_eq!(
                can, in_list,
                "INCONSISTENCY: {:?} -> {:?}: can_transition_to={can}, in_list={in_list}",
                from, to
            );

            let transition = WorkspaceStateTransition::new(from, to, "test");
            let result = transition.validate();
            if can {
                assert!(result.is_ok(), "{:?} -> {:?} should validate", from, to);
            } else {
                assert!(result.is_err(), "{:?} -> {:?} should NOT validate", from, to);
            }
        }
    }
}

#[test]
fn rq_workspace_state_no_self_transitions() {
    for state in WorkspaceState::all().iter().copied() {
        assert!(
            !state.can_transition_to(state),
            "Self-transition MUST be rejected for {:?}",
            state
        );
    }
}

#[test]
fn rq_workspace_state_non_terminal_has_transitions() {
    for state in WorkspaceState::all().iter().copied() {
        if !state.is_terminal() {
            assert!(
                !state.valid_next_states().is_empty(),
                "Non-terminal state {:?} must have at least one valid transition",
                state
            );
        }
    }
}

#[test]
fn rq_workspace_state_active_complete_terminal_partition() {
    for state in WorkspaceState::all().iter().copied() {
        if state.is_active() {
            assert!(!state.is_complete(), "{:?} is both active AND complete", state);
        }
        if state.is_terminal() {
            assert!(!state.is_active(), "{:?} is both terminal AND active", state);
            assert!(
                !WorkspaceStateFilter::NonTerminal.matches(state),
                "{:?} is both terminal AND non-terminal",
                state
            );
        }
    }
}

#[test]
fn rq_workspace_state_filter_completeness() {
    for state in WorkspaceState::all().iter().copied() {
        assert!(WorkspaceStateFilter::All.matches(state));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. SESSION STATE (TYPE STATE PATTERN) — EXHAUSTIVE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rq_session_state_exhaustive_transition_matrix() {
    let all = SessionState::all_states();

    for from in all.iter().copied() {
        let valid_nexts = from.valid_next_states();
        for to in all.iter().copied() {
            let can = from.can_transition_to(to);
            let in_list = valid_nexts.contains(&to);
            assert_eq!(
                can, in_list,
                "INCONSISTENCY: {:?} -> {:?}: can={can}, in_list={in_list}",
                from, to
            );
        }
    }

    assert!(SessionState::Created.can_transition_to(SessionState::Active));
    assert!(SessionState::Created.can_transition_to(SessionState::Failed));
    assert!(SessionState::Active.can_transition_to(SessionState::Syncing));
    assert!(SessionState::Active.can_transition_to(SessionState::Paused));
    assert!(SessionState::Active.can_transition_to(SessionState::Completed));
    assert!(SessionState::Syncing.can_transition_to(SessionState::Synced));
    assert!(SessionState::Syncing.can_transition_to(SessionState::Failed));
    assert!(SessionState::Synced.can_transition_to(SessionState::Active));
    assert!(SessionState::Synced.can_transition_to(SessionState::Paused));
    assert!(SessionState::Synced.can_transition_to(SessionState::Completed));
    assert!(SessionState::Paused.can_transition_to(SessionState::Active));
    assert!(SessionState::Paused.can_transition_to(SessionState::Completed));
    assert!(SessionState::Completed.can_transition_to(SessionState::Created));
    assert!(SessionState::Failed.can_transition_to(SessionState::Created));
}

#[test]
fn rq_session_state_transition_validate_consistency() {
    for from in SessionState::all_states().iter().copied() {
        for to in SessionState::all_states() {
            let transition = StateTransition::new(from, *to, "test");
            let result = transition.validate();
            if from.can_transition_to(*to) {
                assert!(result.is_ok(), "{:?} -> {:?} should validate", from, to);
            } else {
                assert!(result.is_err(), "{:?} -> {:?} should NOT validate", from, to);
            }
        }
    }
}

#[test]
fn rq_session_state_terminal_states_allow_restart() {
    for state in [SessionState::Completed, SessionState::Failed] {
        assert!(state.is_terminal(), "{:?} should be terminal", state);
        assert!(
            state.can_transition_to(SessionState::Created),
            "{:?} should allow restart to Created",
            state
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. SESSION NAME VALIDATION — ADVERSARIAL BOUNDARY
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rq_session_name_boundary_63_vs_64() {
    let exactly_63 = "a".repeat(63);
    assert!(SessionName::parse(&exactly_63).is_ok(), "63 chars MUST be valid");

    let exactly_64 = "a".repeat(64);
    assert!(SessionName::parse(&exactly_64).is_err(), "64 chars MUST be invalid");
}

#[test]
fn rq_session_name_whitespace_trimming_invariant() {
    let internal_space = SessionName::parse("a b");
    assert!(internal_space.is_err(), "Internal spaces must be rejected");

    let padded_valid = SessionName::parse("  ab  ").unwrap();
    assert_eq!(padded_valid.as_str(), "ab");
}

#[test]
fn rq_session_name_from_string_bypass_is_unsafe() {
    let bypassed = SessionName::from("123-invalid".to_string());
    assert!(
        SessionName::parse("123-invalid").is_err(),
        "parse() must reject this"
    );
    assert_eq!(bypassed.as_str(), "123-invalid");
}

#[test]
fn rq_session_name_ascii_only_no_unicode_letters() {
    assert!(SessionName::parse("café").is_err());
    assert!(SessionName::parse("naïve").is_err());
    assert!(SessionName::parse("über").is_err());
    assert!(SessionName::parse("test\u{0660}").is_err());
}

proptest! {
    #[test]
    fn rq_session_name_parse_idempotent(s in "[a-zA-Z][a-zA-Z0-9_-]{0,62}") {
        let first = SessionName::parse(s.clone()).expect("valid");
        let second = SessionName::parse(first.as_str()).expect("still valid");
        prop_assert_eq!(first, second);
    }

    #[test]
    fn rq_session_name_never_valid_with_null_byte(s in ".*") {
        if s.contains('\0') {
            prop_assert!(SessionName::parse(s).is_err(), "null bytes must be rejected");
        }
    }

    #[test]
    fn rq_session_name_control_chars_rejected(s in "[\x00-\x1f\x7f]+") {
        prop_assert!(SessionName::parse(s).is_err(), "control chars must be rejected");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. SESSION ID VALIDATION — ADVERSARIAL
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rq_session_id_no_max_length_restriction() {
    let very_long = "a".repeat(100_000);
    assert!(SessionId::parse(&very_long).is_ok(), "SessionId has no max length");
}

#[test]
fn rq_session_id_single_hyphen_valid() {
    assert!(SessionId::parse("-").is_ok(), "single hyphen must be valid");
}

#[test]
fn rq_session_id_all_hyphens_valid() {
    assert!(SessionId::parse("---").is_ok());
    assert!(SessionId::parse(&"-".repeat(1000)).is_ok());
}

#[test]
fn rq_session_id_start_with_digit_valid() {
    assert!(SessionId::parse("123abc").is_ok());
    assert!(SessionId::parse("0").is_ok());
}

proptest! {
    #[test]
    fn rq_session_id_alnum_hyphen_always_valid(s in "[a-zA-Z0-9-]{1,1000}") {
        prop_assert!(SessionId::parse(s).is_ok());
    }

    #[test]
    fn rq_session_id_with_underscore_always_invalid(s in "[a-zA-Z0-9_]+") {
        if s.contains('_') && !s.is_empty() {
            prop_assert!(SessionId::parse(&s).is_err());
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. DAG INVARIANT PRESERVATION UNDER MUTATION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rq_dag_trunk_always_exists() {
    let dag = BranchDag::new();
    assert!(dag.contains(&BranchId::new("trunk")));
    assert_eq!(dag.len(), 1);
    assert!(dag.is_empty());
}

#[test]
fn rq_dag_self_parent_rejected() {
    let mut dag = BranchDag::new();
    let trunk = BranchId::new("trunk");
    let result = dag.add_branch(trunk.clone(), vec![trunk]);
    assert!(result.is_err(), "Self-parent must be rejected");
}

#[test]
fn rq_dag_cycle_detection_depth_2() {
    let mut dag = BranchDag::new();
    let trunk = BranchId::new("trunk");
    let x = BranchId::new("x");
    let y = BranchId::new("y");
    dag.add_branch(x.clone(), vec![trunk.clone()]).unwrap();
    dag.add_branch(y.clone(), vec![x.clone()]).unwrap();
    // y is a descendant of x. Making x a child of y creates a cycle.
    let result = dag.add_branch(x.clone(), vec![y.clone()]);
    assert!(result.is_err(), "Adding descendant as parent must create cycle error");
}

#[test]
fn rq_dag_cycle_detection_depth_3() {
    let mut dag = BranchDag::new();
    let trunk = BranchId::new("trunk");
    let a = BranchId::new("a");
    let b = BranchId::new("b");
    let c = BranchId::new("c");
    let d = BranchId::new("d");

    dag.add_branch(a.clone(), vec![trunk.clone()]).unwrap();
    dag.add_branch(b.clone(), vec![a.clone()]).unwrap();
    dag.add_branch(c.clone(), vec![b.clone()]).unwrap();
    dag.add_branch(d.clone(), vec![c.clone()]).unwrap();

    // a is ancestor of d. Making a a child of d creates cycle.
    let result = dag.add_branch(a.clone(), vec![d.clone()]);
    assert!(result.is_err(), "Long cycle must be detected");
}

#[test]
fn rq_dag_non_trunk_must_have_parent() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("orphan"), vec![]);
    assert!(result.is_err(), "Non-trunk branch without parent must be rejected");
}

#[test]
fn rq_dag_duplicate_branch_rejected() {
    let mut dag = BranchDag::new();
    let trunk = BranchId::new("trunk");
    let a = BranchId::new("a");
    dag.add_branch(a.clone(), vec![trunk.clone()]).unwrap();
    let result = dag.add_branch(a, vec![trunk]);
    assert!(result.is_err(), "Duplicate branch must be rejected");
}

#[test]
fn rq_dag_invalid_parent_rejected() {
    let mut dag = BranchDag::new();
    let ghost = BranchId::new("nonexistent");
    let result = dag.add_branch(BranchId::new("a"), vec![ghost]);
    assert!(result.is_err(), "Non-existent parent must be rejected");
}

#[test]
fn rq_dag_remove_with_descendants_rejected() {
    let mut dag = BranchDag::new();
    let trunk = BranchId::new("trunk");
    let a = BranchId::new("a");
    let b = BranchId::new("b");
    dag.add_branch(a.clone(), vec![trunk.clone()]).unwrap();
    dag.add_branch(b.clone(), vec![a.clone()]).unwrap();

    let result = dag.remove_branch(a.clone());
    assert!(result.is_err(), "Removing branch with descendants must fail");

    let result = dag.remove_branch(b.clone());
    assert!(result.is_ok(), "Leaf branch should be removable");

    let result = dag.remove_branch(a);
    assert!(result.is_ok(), "Branch should be removable after children removed");
}

#[test]
fn rq_dag_parent_child_consistency_after_add_remove() {
    let mut dag = BranchDag::new();
    let trunk = BranchId::new("trunk");
    let a = BranchId::new("a");
    let b = BranchId::new("b");

    dag.add_branch(a.clone(), vec![trunk.clone()]).unwrap();
    dag.add_branch(b.clone(), vec![a.clone()]).unwrap();

    assert!(dag.parents[&a].contains(&trunk));
    assert!(dag.children[&trunk].contains(&a));
    assert!(dag.parents[&b].contains(&a));
    assert!(dag.children[&a].contains(&b));

    dag.remove_branch(b.clone()).unwrap();
    assert!(!dag.branches.contains(&b));
    assert!(!dag.children[&a].contains(&b));

    dag.remove_branch(a.clone()).unwrap();
    assert!(!dag.branches.contains(&a));
    assert!(!dag.children[&trunk].contains(&a));
}

#[test]
fn rq_dag_ancestors_reach_trunk() {
    let mut dag = BranchDag::new();
    let trunk = BranchId::new("trunk");
    let a = BranchId::new("a");
    let b = BranchId::new("b");
    let c = BranchId::new("c");

    dag.add_branch(a.clone(), vec![trunk.clone()]).unwrap();
    dag.add_branch(b.clone(), vec![a.clone()]).unwrap();
    dag.add_branch(c.clone(), vec![b.clone()]).unwrap();

    let c_ancestors = dag.ancestors(&c).unwrap();
    assert!(c_ancestors.contains(&b), "c's ancestors must include b");
    assert!(c_ancestors.contains(&a), "c's ancestors must include a");
    assert!(c_ancestors.contains(&trunk), "c's ancestors must include trunk");
}

#[test]
fn rq_dag_descendants_transitive() {
    let mut dag = BranchDag::new();
    let trunk = BranchId::new("trunk");
    let a = BranchId::new("a");
    let b = BranchId::new("b");

    dag.add_branch(a.clone(), vec![trunk.clone()]).unwrap();
    dag.add_branch(b.clone(), vec![trunk.clone()]).unwrap();

    let trunk_descendants = dag.descendants(&trunk).unwrap();
    assert!(trunk_descendants.contains(&a), "trunk descendants must include a");
    assert!(trunk_descendants.contains(&b), "trunk descendants must include b");
}

#[test]
fn rq_dag_multiple_parents() {
    let mut dag = BranchDag::new();
    let trunk = BranchId::new("trunk");
    let a = BranchId::new("a");
    let b = BranchId::new("b");
    let merge = BranchId::new("merge");

    dag.add_branch(a.clone(), vec![trunk.clone()]).unwrap();
    dag.add_branch(b.clone(), vec![trunk.clone()]).unwrap();
    dag.add_branch(merge.clone(), vec![a.clone(), b.clone()]).unwrap();

    let merge_ancestors = dag.ancestors(&merge).unwrap();
    assert!(merge_ancestors.contains(&a));
    assert!(merge_ancestors.contains(&b));
    assert!(merge_ancestors.contains(&trunk));

    assert!(dag.children[&a].contains(&merge));
    assert!(dag.children[&b].contains(&merge));
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. ERROR TYPE CONSISTENCY — EVERY VARIANT HAS CODE + CONTEXT
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rq_error_code_is_screaming_snake_case() {
    use crate::error::Error;
    let all_errors: Vec<Error> = vec![
        Error::workspace_not_found("w"),
        Error::workspace_exists("w"),
        Error::workspace_locked("w", "h"),
        Error::workspace_conflict("m"),
        Error::session("s"),
        Error::session_exists("s"),
        Error::session_locked("s", "h"),
        Error::not_lock_holder("s", "a"),
        Error::session_invalid_state("s", "a", "b"),
        Error::queue_empty(),
        Error::queue_item_not_found("i"),
        Error::queue_locked("h"),
        Error::queue_processing(),
        Error::queue_invalid_position(1),
        Error::queue_full(10),
        Error::vcs_not_initialized(),
        Error::vcs_conflict("r", "m"),
        Error::vcs_push_failed("m"),
        Error::vcs_pull_failed("m"),
        Error::vcs_rebase_failed("m"),
        Error::branch_not_found("b"),
        Error::branch_exists("b"),
        Error::commit_not_found("c"),
        Error::working_copy_dirty(),
        Error::vcs_commit_failed("m"),
        Error::vcs_checkout_failed("m"),
        Error::vcs_diff_failed("m"),
        Error::config_not_found("m"),
        Error::config_invalid("m"),
        Error::config_permission("m"),
        Error::agent_not_found("a"),
        Error::agent_exists("a"),
        Error::io_error("m"),
        Error::database("m"),
        Error::invalid_state("m"),
        Error::not_found("m"),
        Error::validation_error("m"),
        Error::validation_field_error("f", "m", None),
        Error::invalid_identifier("m"),
        Error::internal("m"),
        Error::unimplemented("m"),
        Error::batch_empty(),
        Error::batch_command_failed("m"),
        Error::batch_rollback_failed("m"),
        Error::batch_size_exceeded(10),
        Error::checkpoint_error("m"),
    ];

    for err in &all_errors {
        let code = err.code();
        assert!(
            !code.is_empty(),
            "Error code must not be empty: {:?}",
            err
        );
        assert!(
            code.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()),
            "Error code must be SCREAMING_SNAKE_CASE: {}",
            code
        );
    }
}

#[test]
fn rq_error_exit_code_never_zero() {
    use crate::error::Error;
    let all_errors: Vec<Error> = vec![
        Error::workspace_not_found("w"),
        Error::session("s"),
        Error::queue_empty(),
        Error::vcs_not_initialized(),
        Error::config_not_found("m"),
        Error::agent_not_found("a"),
        Error::io_error("m"),
        Error::invalid_state("m"),
        Error::internal("m"),
        Error::batch_empty(),
    ];
    for err in &all_errors {
        assert_ne!(
            err.exit_code(), 0,
            "Error exit code must be non-zero: {:?}",
            err
        );
    }
}

#[test]
fn rq_error_context_map_always_some() {
    use crate::error::Error;
    let all_errors: Vec<Error> = vec![
        Error::workspace_not_found("w"),
        Error::workspace_exists("w"),
        Error::workspace_locked("w", "h"),
        Error::workspace_conflict("m"),
        Error::session("s"),
        Error::session_exists("s"),
        Error::session_locked("s", "h"),
        Error::not_lock_holder("s", "a"),
        Error::session_invalid_state("s", "a", "b"),
        Error::queue_empty(),
        Error::queue_item_not_found("i"),
        Error::queue_locked("h"),
        Error::queue_processing(),
        Error::queue_invalid_position(1),
        Error::queue_full(10),
        Error::vcs_not_initialized(),
        Error::vcs_conflict("r", "m"),
        Error::branch_not_found("b"),
        Error::working_copy_dirty(),
        Error::config_not_found("m"),
        Error::config_invalid("m"),
        Error::agent_not_found("a"),
        Error::agent_exists("a"),
        Error::io_error("m"),
        Error::database("m"),
        Error::invalid_state("m"),
        Error::not_found("m"),
        Error::validation_error("m"),
        Error::validation_field_error("f", "m", Some("v".into())),
        Error::invalid_identifier("m"),
        Error::internal("m"),
        Error::unimplemented("m"),
        Error::batch_empty(),
        Error::batch_command_failed("m"),
        Error::checkpoint_error("m"),
    ];
    for err in &all_errors {
        assert!(
            err.context_map().is_some(),
            "context_map() must return Some for {:?}",
            err
        );
    }
}

#[test]
fn rq_error_display_never_empty() {
    use crate::error::Error;
    let all_errors: Vec<Error> = vec![
        Error::workspace_not_found("w"),
        Error::session("s"),
        Error::queue_empty(),
        Error::vcs_not_initialized(),
        Error::config_not_found("m"),
        Error::agent_not_found("a"),
        Error::io_error("m"),
        Error::invalid_state("m"),
        Error::internal("m"),
        Error::batch_empty(),
        Error::not_found("m"),
    ];
    for err in &all_errors {
        let display = err.to_string();
        assert!(
            !display.is_empty(),
            "Error Display must not be empty for {:?}",
            err
        );
        assert!(
            display.len() > 3,
            "Error Display too short: '{}' for {:?}",
            display, err
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 8. SESSION STATE MANAGER — TYPE STATE PATTERN
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rq_session_manager_full_lifecycle_created_to_completed() {
    let manager = SessionStateManager::new("test");
    assert_eq!(manager.current_state(), SessionState::Created);

    let active = manager.activate("go").unwrap();
    assert_eq!(active.current_state(), SessionState::Active);

    let completed = active.complete("done").unwrap();
    assert_eq!(completed.current_state(), SessionState::Completed);
    assert_eq!(completed.history().len(), 2);
}

#[test]
fn rq_session_manager_full_lifecycle_with_sync() {
    let manager = SessionStateManager::new("test");

    let active = manager.activate("go").unwrap();
    let syncing = active.sync("syncing").unwrap();
    assert_eq!(syncing.current_state(), SessionState::Syncing);

    let synced = syncing.sync_complete("done").unwrap();
    assert_eq!(synced.current_state(), SessionState::Synced);

    let completed = synced.complete("done").unwrap();
    assert_eq!(completed.current_state(), SessionState::Completed);
    assert_eq!(completed.history().len(), 4);
}

#[test]
fn rq_session_manager_pause_resume_cycle() {
    let manager = SessionStateManager::new("test");

    let active = manager.activate("go").unwrap();
    let paused = active.pause("break").unwrap();
    assert_eq!(paused.current_state(), SessionState::Paused);

    let resumed = paused.resume("back").unwrap();
    assert_eq!(resumed.current_state(), SessionState::Active);

    let _completed = resumed.complete("done").unwrap();
}

#[test]
fn rq_session_manager_fail_retry_cycle() {
    let manager = SessionStateManager::new("test");

    let failed = manager.fail("error").unwrap();
    assert_eq!(failed.current_state(), SessionState::Failed);

    let retried = failed.retry("try again").unwrap();
    assert_eq!(retried.current_state(), SessionState::Created);
    assert_eq!(retried.history().len(), 2);
}

#[test]
fn rq_session_manager_restart_from_completed() {
    let manager = SessionStateManager::new("test");

    let active = manager.activate("go").unwrap();
    let completed = active.complete("done").unwrap();
    let restarted = completed.restart("redo").unwrap();
    assert_eq!(restarted.current_state(), SessionState::Created);
}

#[test]
fn rq_session_manager_synced_pause_then_complete() {
    let manager = SessionStateManager::new("test");

    let active = manager.activate("go").unwrap();
    let syncing = active.sync("sync").unwrap();
    let synced = syncing.sync_complete("done").unwrap();
    let paused = synced.pause("pause").unwrap();
    assert_eq!(paused.current_state(), SessionState::Paused);

    let completed = paused.complete("done").unwrap();
    assert_eq!(completed.current_state(), SessionState::Completed);
}

#[test]
fn rq_session_manager_history_preserves_order() {
    let manager = SessionStateManager::new("test");

    let active = manager.activate("go").unwrap();
    let syncing = active.sync("sync").unwrap();
    let synced = syncing.sync_complete("done").unwrap();

    let history = synced.history();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].from, SessionState::Created);
    assert_eq!(history[0].to, SessionState::Active);
    assert_eq!(history[1].from, SessionState::Active);
    assert_eq!(history[1].to, SessionState::Syncing);
    assert_eq!(history[2].from, SessionState::Syncing);
    assert_eq!(history[2].to, SessionState::Synced);
}

#[test]
fn rq_session_manager_metadata_preserved_through_transitions() {
    let manager = SessionStateManager::new("test");
    let mut active = manager.activate("go").unwrap();
    active.set_metadata("key", "value");

    let completed = active.complete("done").unwrap();
    assert_eq!(completed.metadata().get("key"), Some(&"value".to_string()));
    assert_eq!(completed.session_id(), "test");
}

// ═══════════════════════════════════════════════════════════════════════════
// 9. CROSS-MODULE CONSISTENCY
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rq_session_status_and_session_state_both_have_terminal_states() {
    let ss_terminals: Vec<_> = SessionStatus::all_states()
        .iter()
        .filter(|s| s.is_terminal())
        .copied()
        .collect();
    assert!(!ss_terminals.is_empty(), "SessionStatus must have terminal states");

    let state_terminals: Vec<_> = SessionState::all_states()
        .iter()
        .filter(|s| s.is_terminal())
        .copied()
        .collect();
    assert!(!state_terminals.is_empty(), "SessionState must have terminal states");
}

#[test]
fn rq_workspace_state_fromstr_matches_display() {
    use std::str::FromStr;
    for state in WorkspaceState::all().iter().copied() {
        let display = state.to_string();
        let parsed = WorkspaceState::from_str(&display).unwrap();
        assert_eq!(state, parsed, "FromStr(Display) roundtrip failed for {:?}", state);
    }
}

#[test]
fn rq_branch_state_detached_vs_empty_string_disambiguation() {
    let detached: BranchState = serde_json::from_str("\"detached\"").unwrap();
    assert!(detached.is_detached());

    let empty: BranchState = serde_json::from_str("\"\"").unwrap();
    assert!(!empty.is_detached());
    assert_eq!(empty.branch_name(), Some(""));
}

#[test]
fn rq_priority_ordering_critical_beats_high_beats_normal_beats_low() {
    assert!(Priority::Critical < Priority::High);
    assert!(Priority::High < Priority::Normal);
    assert!(Priority::Normal < Priority::Low);
    assert!(Priority::Critical < Priority::Low);
}

#[test]
fn rq_queue_status_serde_roundtrip_all_variants() {
    for status in [
        QueueStatus::Pending,
        QueueStatus::Processing,
        QueueStatus::Completed,
        QueueStatus::Failed,
        QueueStatus::Cancelled,
    ] {
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: QueueStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 10. PROPTEST — FUZZ-DRIVEN INVARIANT ATTACKS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #[test]
    fn rq_session_name_valid_then_parse_always_succeeds(
        first in "[a-zA-Z]",
        rest in "[a-zA-Z0-9_-]{0,62}"
    ) {
        let name = format!("{first}{rest}");
        let parsed = SessionName::parse(name.clone()).unwrap();
        let reparsed = SessionName::parse(parsed.as_str()).unwrap();
        prop_assert_eq!(parsed, reparsed);
    }

    #[test]
    fn rq_workspace_state_filter_matches_exhaustive(state_idx in 0..6usize) {
        let all = WorkspaceState::all();
        let state = all[state_idx];

        let is_terminal = WorkspaceStateFilter::Terminal.matches(state);
        let is_non_terminal = WorkspaceStateFilter::NonTerminal.matches(state);
        prop_assert_ne!(is_terminal, is_non_terminal,
            "Terminal and NonTerminal must be complements for {:?}",
            state);
    }

    #[test]
    fn rq_session_id_never_empty_after_parse(s in "[a-zA-Z0-9-]{1,100}") {
        let id = SessionId::parse(s).unwrap();
        prop_assert!(!id.as_str().is_empty());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 11. FILE CHANGE / DIFF SUMMARY INVARIANT ATTACKS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rq_diff_summary_mismatch_always_fails() {
    use std::path::PathBuf;

    let mismatch = DiffSummary {
        insertions: 10,
        deletions: 5,
        files_changed: 99,
        files: vec![FileDiffStat {
            path: PathBuf::from("f.rs"),
            insertions: 10,
            deletions: 5,
            status: FileStatus::Modified,
        }],
    };
    assert!(mismatch.validate().is_err(), "Mismatched files_changed must fail");

    let ins_mismatch = DiffSummary {
        insertions: 100,
        deletions: 5,
        files_changed: 1,
        files: vec![FileDiffStat {
            path: PathBuf::from("f.rs"),
            insertions: 10,
            deletions: 5,
            status: FileStatus::Modified,
        }],
    };
    // Domain does NOT validate insertion sum consistency — only files_changed count.
    assert!(ins_mismatch.validate().is_ok(), "Domain trusts caller for insertion sums");
}

#[test]
fn rq_file_change_renamed_requires_old_path() {
    use std::path::PathBuf;

    let no_old = FileChange {
        path: PathBuf::from("new.rs"),
        status: FileStatus::Renamed,
        old_path: None,
    };
    assert!(no_old.validate().is_err(), "Renamed without old_path must fail");

    let with_old = FileChange {
        path: PathBuf::from("new.rs"),
        status: FileStatus::Renamed,
        old_path: Some(PathBuf::from("old.rs")),
    };
    assert!(with_old.validate().is_ok());
}

#[test]
fn rq_changes_summary_total_excludes_untracked() {
    let only_untracked = ChangesSummary {
        modified: 0, added: 0, deleted: 0, renamed: 0, untracked: 100,
    };
    assert_eq!(only_untracked.total(), 0, "total() must exclude untracked");
    assert!(!only_untracked.has_changes(), "has_changes() must be false for only untracked");
    assert!(!only_untracked.has_tracked_changes());
}

#[test]
fn rq_changes_summary_all_tracked_counts() {
    let s = ChangesSummary {
        modified: 3, added: 2, deleted: 1, renamed: 4, untracked: 10,
    };
    assert_eq!(s.total(), 10);
    assert!(s.has_tracked_changes());
    assert!(s.has_changes());
}
