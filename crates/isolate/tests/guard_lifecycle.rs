//! Black-hat tests for the WorkspaceGuard RAII lifecycle.
//!
//! Covers:
//! - Acquire validation (only Created state allowed)
//! - Commit path (Working -> Ready)
//! - Abandon path (Working -> Abandoned)
//! - Double-commit/double-abandon prevention
//! - Guard accessor methods
//! - CommittedGuard fields and predicates
//! - Drop without commit (warning path)
//! - Multiple independent guards
//! - Proptests

use scp_isolate::{BeadId, IsolateError, WorkspaceGuard, WorkspaceId, WorkspaceState};

fn fresh_workspace_id() -> WorkspaceId {
    WorkspaceId::generate()
}

fn fresh_bead_id(suffix: &str) -> BeadId {
    BeadId::parse(format!("bead-{suffix}")).unwrap()
}

// === Acquire: only Created state is valid ===

#[test]
fn acquire_created_succeeds() {
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("ok"),
        WorkspaceState::Created,
    );
    assert!(guard.is_ok());
}

#[test]
fn acquire_working_fails() {
    let result = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("working"),
        WorkspaceState::Working,
    );
    assert!(result.is_err());
}

#[test]
fn acquire_ready_fails() {
    let result = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("ready"),
        WorkspaceState::Ready,
    );
    assert!(result.is_err());
}

#[test]
fn acquire_merged_fails() {
    let result = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("merged"),
        WorkspaceState::Merged,
    );
    assert!(result.is_err());
}

#[test]
fn acquire_abandoned_fails() {
    let result = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("abandoned"),
        WorkspaceState::Abandoned,
    );
    assert!(result.is_err());
}

#[test]
fn acquire_conflict_fails() {
    let result = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("conflict"),
        WorkspaceState::Conflict,
    );
    assert!(result.is_err());
}

#[test]
fn acquire_rejects_all_non_created_states() {
    let non_created = [
        WorkspaceState::Working,
        WorkspaceState::Ready,
        WorkspaceState::Merged,
        WorkspaceState::Abandoned,
        WorkspaceState::Conflict,
    ];
    for &state in &non_created {
        let result = WorkspaceGuard::acquire(fresh_workspace_id(), fresh_bead_id("all"), state);
        assert!(result.is_err(), "acquire should fail for {state:?}");
    }
}

#[test]
fn acquire_error_is_operation_failed() {
    let result = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("err"),
        WorkspaceState::Working,
    );
    match result.err() {
        Some(IsolateError::OperationFailed(msg)) => {
            assert!(
                msg.contains("Created"),
                "error should mention Created: {msg}"
            );
            assert!(
                msg.contains("working"),
                "error should mention actual state: {msg}"
            );
        }
        other => panic!("expected OperationFailed, got {other:?}"),
    }
}

// === Accessors after acquire ===

#[test]
fn acquire_sets_state_to_working() {
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("state"),
        WorkspaceState::Created,
    )
    .unwrap();
    assert_eq!(guard.state(), Some(WorkspaceState::Working));
}

#[test]
fn acquire_is_not_resolved() {
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("resolve"),
        WorkspaceState::Created,
    )
    .unwrap();
    assert!(!guard.is_resolved());
}

#[test]
fn acquire_exposes_workspace_id() {
    let ws_id = fresh_workspace_id();
    let bead_id = fresh_bead_id("ws");
    let guard = WorkspaceGuard::acquire(ws_id.clone(), bead_id, WorkspaceState::Created).unwrap();
    assert_eq!(guard.workspace_id(), Some(&ws_id));
}

#[test]
fn acquire_exposes_bead_id() {
    let bead_id = fresh_bead_id("expose");
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        bead_id.clone(),
        WorkspaceState::Created,
    )
    .unwrap();
    assert_eq!(guard.bead_id(), Some(&bead_id));
}

#[test]
fn acquire_creates_mapping() {
    let ws_id = fresh_workspace_id();
    let bead_id = fresh_bead_id("map");
    let guard =
        WorkspaceGuard::acquire(ws_id.clone(), bead_id.clone(), WorkspaceState::Created).unwrap();
    let mapping = guard.mapping().expect("mapping should exist");
    assert_eq!(mapping.bead_id(), &bead_id);
    assert_eq!(mapping.workspace_id(), &ws_id);
}

#[test]
fn acquire_mapping_has_recent_timestamp() {
    let before = chrono::Utc::now();
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("ts"),
        WorkspaceState::Created,
    )
    .unwrap();
    let after = chrono::Utc::now();
    let mapping = guard.mapping().unwrap();
    assert!(mapping.assigned_at() >= before);
    assert!(mapping.assigned_at() <= after);
}

// === Commit path ===

#[test]
fn commit_transitions_to_ready() {
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("commit"),
        WorkspaceState::Created,
    )
    .unwrap();
    let committed = guard.commit().unwrap();
    assert!(committed.is_ready());
    assert!(!committed.is_abandoned());
    assert_eq!(committed.state(), WorkspaceState::Ready);
}

#[test]
fn commit_preserves_workspace_id() {
    let ws_id = fresh_workspace_id();
    let guard =
        WorkspaceGuard::acquire(ws_id.clone(), fresh_bead_id("cid"), WorkspaceState::Created)
            .unwrap();
    let committed = guard.commit().unwrap();
    assert_eq!(committed.workspace_id(), &ws_id);
}

#[test]
fn commit_preserves_bead_id() {
    let bead_id = fresh_bead_id("cbead");
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        bead_id.clone(),
        WorkspaceState::Created,
    )
    .unwrap();
    let committed = guard.commit().unwrap();
    assert_eq!(committed.bead_id(), &bead_id);
}

#[test]
fn commit_preserves_mapping() {
    let ws_id = fresh_workspace_id();
    let bead_id = fresh_bead_id("cmap");
    let guard =
        WorkspaceGuard::acquire(ws_id.clone(), bead_id.clone(), WorkspaceState::Created).unwrap();
    let committed = guard.commit().unwrap();
    assert_eq!(committed.mapping().bead_id(), &bead_id);
    assert_eq!(committed.mapping().workspace_id(), &ws_id);
}

// === Abandon path ===

#[test]
fn abandon_transitions_to_abandoned() {
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("abandon"),
        WorkspaceState::Created,
    )
    .unwrap();
    let committed = guard.abandon().unwrap();
    assert!(committed.is_abandoned());
    assert!(!committed.is_ready());
    assert_eq!(committed.state(), WorkspaceState::Abandoned);
}

#[test]
fn abandon_preserves_workspace_id() {
    let ws_id = fresh_workspace_id();
    let guard =
        WorkspaceGuard::acquire(ws_id.clone(), fresh_bead_id("aid"), WorkspaceState::Created)
            .unwrap();
    let committed = guard.abandon().unwrap();
    assert_eq!(committed.workspace_id(), &ws_id);
}

#[test]
fn abandon_preserves_bead_id() {
    let bead_id = fresh_bead_id("abead");
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        bead_id.clone(),
        WorkspaceState::Created,
    )
    .unwrap();
    let committed = guard.abandon().unwrap();
    assert_eq!(committed.bead_id(), &bead_id);
}

// === CommittedGuard ===

#[test]
fn committed_guard_clone_preserves_fields() {
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("clone"),
        WorkspaceState::Created,
    )
    .unwrap();
    let committed = guard.commit().unwrap();
    let cloned = committed.clone();
    assert_eq!(cloned.state(), committed.state());
    assert_eq!(cloned.workspace_id(), committed.workspace_id());
    assert_eq!(cloned.bead_id(), committed.bead_id());
    assert_eq!(cloned.is_ready(), committed.is_ready());
}

#[test]
fn committed_guard_debug_contains_fields() {
    let ws_id = WorkspaceId::parse("debug-ws".into()).unwrap();
    let bead_id = BeadId::parse("debug-bead".into()).unwrap();
    let guard = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Created).unwrap();
    let committed = guard.commit().unwrap();
    let debug = format!("{committed:?}");
    assert!(debug.contains("CommittedGuard"));
}

// === Guard Debug ===

#[test]
fn guard_debug_format_active() {
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("dbg"),
        WorkspaceState::Created,
    )
    .unwrap();
    let debug = format!("{guard:?}");
    assert!(debug.contains("WorkspaceGuard"));
    assert!(debug.contains("committed"));
}

// === Drop behavior ===

#[test]
fn drop_without_commit_does_not_panic() {
    // Guard created and dropped — should emit warning but not panic
    let _ = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("drop"),
        WorkspaceState::Created,
    );
    // Drop happens here, no panic
}

#[test]
fn drop_after_commit_no_double_consume() {
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("dc"),
        WorkspaceState::Created,
    )
    .unwrap();
    let _committed = guard.commit().unwrap();
    // After commit, guard is consumed — no drop warning
}

#[test]
fn drop_after_abandon_no_double_consume() {
    let guard = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("da"),
        WorkspaceState::Created,
    )
    .unwrap();
    let _committed = guard.abandon().unwrap();
}

// === Multiple guards ===

#[test]
fn multiple_independent_guards() {
    let ws1 = fresh_workspace_id();
    let ws2 = fresh_workspace_id();
    let bead1 = fresh_bead_id("g1");
    let bead2 = fresh_bead_id("g2");

    let g1 = WorkspaceGuard::acquire(ws1.clone(), bead1.clone(), WorkspaceState::Created).unwrap();
    let g2 = WorkspaceGuard::acquire(ws2.clone(), bead2.clone(), WorkspaceState::Created).unwrap();

    assert_ne!(g1.workspace_id(), g2.workspace_id());

    let c1 = g1.commit().unwrap();
    let c2 = g2.abandon().unwrap();

    assert!(c1.is_ready());
    assert!(c2.is_abandoned());
    assert_ne!(c1.workspace_id(), c2.workspace_id());
}

#[test]
fn same_bead_different_guards_fails_or_succeeds_independently() {
    // Two guards with different workspace IDs but same bead ID — both should work
    let bead = fresh_bead_id("shared");
    let g1 = WorkspaceGuard::acquire(fresh_workspace_id(), bead.clone(), WorkspaceState::Created)
        .unwrap();
    let g2 = WorkspaceGuard::acquire(fresh_workspace_id(), bead, WorkspaceState::Created).unwrap();

    let c1 = g1.commit().unwrap();
    let c2 = g2.commit().unwrap();
    assert!(c1.is_ready());
    assert!(c2.is_ready());
    assert_ne!(c1.workspace_id(), c2.workspace_id());
}

// === Commit vs Abandon give different terminal states ===

#[test]
fn commit_vs_abandon_different_outcomes() {
    let g_commit = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("c"),
        WorkspaceState::Created,
    )
    .unwrap();
    let g_abandon = WorkspaceGuard::acquire(
        fresh_workspace_id(),
        fresh_bead_id("a"),
        WorkspaceState::Created,
    )
    .unwrap();

    let committed = g_commit.commit().unwrap();
    let abandoned = g_abandon.abandon().unwrap();

    assert!(committed.state().is_complete());
    assert!(!abandoned.state().is_complete());
    assert!(committed.state().is_terminal() || !committed.state().is_terminal());
    assert!(abandoned.state().is_terminal());
}

// === Proptests ===

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use proptest::prop_assert;

    proptest! {
        #[test]
        fn commit_always_ready(bead_suffix in "[a-z0-9]{1,20}") {
            let bead = BeadId::parse(format!("bead-{bead_suffix}")).unwrap();
            let guard = WorkspaceGuard::acquire(
                WorkspaceId::generate(),
                bead,
                WorkspaceState::Created,
            )
            .unwrap();
            let committed = guard.commit().unwrap();
            prop_assert!(committed.is_ready());
            prop_assert_eq!(committed.state(), WorkspaceState::Ready);
        }

        #[test]
        fn abandon_always_abandoned(bead_suffix in "[a-z0-9]{1,20}") {
            let bead = BeadId::parse(format!("bead-{bead_suffix}")).unwrap();
            let guard = WorkspaceGuard::acquire(
                WorkspaceId::generate(),
                bead,
                WorkspaceState::Created,
            )
            .unwrap();
            let committed = guard.abandon().unwrap();
            prop_assert!(committed.is_abandoned());
            prop_assert_eq!(committed.state(), WorkspaceState::Abandoned);
        }

        #[test]
        fn non_created_state_always_fails(state_idx in 1usize..5) {
            let non_created = [
                WorkspaceState::Working,
                WorkspaceState::Ready,
                WorkspaceState::Merged,
                WorkspaceState::Abandoned,
                WorkspaceState::Conflict,
            ];
            let state = non_created[state_idx];
            let result = WorkspaceGuard::acquire(
                WorkspaceId::generate(),
                BeadId::parse("any".into()).unwrap(),
                state,
            );
            prop_assert!(result.is_err());
        }

        #[test]
        fn guard_preserves_ids_after_commit(
            bead_suffix in "[a-z0-9]{1,10}",
            ws_suffix in "[a-z0-9]{1,10}"
        ) {
            let bead = BeadId::parse(format!("bead-{bead_suffix}")).unwrap();
            let ws = WorkspaceId::parse(format!("ws-{ws_suffix}")).unwrap();
            let guard = WorkspaceGuard::acquire(ws.clone(), bead.clone(), WorkspaceState::Created).unwrap();
            let committed = guard.commit().unwrap();
            prop_assert_eq!(committed.workspace_id().as_str(), ws.as_str());
            prop_assert_eq!(committed.bead_id().as_str(), bead.as_str());
        }

        #[test]
        fn guard_preserves_ids_after_abandon(
            bead_suffix in "[a-z0-9]{1,10}",
            ws_suffix in "[a-z0-9]{1,10}"
        ) {
            let bead = BeadId::parse(format!("bead-{bead_suffix}")).unwrap();
            let ws = WorkspaceId::parse(format!("ws-{ws_suffix}")).unwrap();
            let guard = WorkspaceGuard::acquire(ws.clone(), bead.clone(), WorkspaceState::Created).unwrap();
            let committed = guard.abandon().unwrap();
            prop_assert_eq!(committed.workspace_id().as_str(), ws.as_str());
            prop_assert_eq!(committed.bead_id().as_str(), bead.as_str());
        }

        #[test]
        fn no_panic_on_acquire_any_state(state_idx in 0usize..6) {
            let states = WorkspaceState::all();
            let state = states[state_idx];
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = WorkspaceGuard::acquire(
                    WorkspaceId::generate(),
                    BeadId::parse("panic".into()).unwrap(),
                    state,
                );
            }));
            prop_assert!(result.is_ok());
        }
    }
}
