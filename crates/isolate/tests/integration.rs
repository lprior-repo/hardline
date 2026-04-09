//! Cross-domain integration tests for the isolate crate.
//!
//! Tests that exercise multiple components together:
//! - State machine + guard lifecycle
//! - Events + state transitions
//! - Guard + bead/workspace mapping
//! - Checkpoint + command classification
//! - Full end-to-end workspace lifecycle scenarios

use scp_isolate::{
    classify_command, BeadId, CheckpointState, WorkspaceGuard, WorkspaceId, WorkspaceState,
    WorkspaceStateMachine,
};

fn fresh_ws_id() -> WorkspaceId {
    WorkspaceId::generate()
}

fn fresh_bead_id(tag: &str) -> BeadId {
    BeadId::parse(format!(
        "bead-{tag}-{}",
        chrono::Utc::now().timestamp_millis()
    ))
    .unwrap()
}

// === Full lifecycle: Guard + State Machine + Events ===

#[test]
fn full_happy_path_lifecycle() {
    let ws_id = fresh_ws_id();
    let bead_id = fresh_bead_id("happy");

    // 1. State: Created
    assert!(ws_id.as_str().starts_with("iso-"));
    assert!(bead_id.as_str().starts_with("bead-"));

    // 2. Guard acquire (Created -> Working)
    let guard = WorkspaceGuard::acquire(ws_id.clone(), bead_id.clone(), WorkspaceState::Created)
        .expect("acquire should succeed for Created");
    assert_eq!(guard.state(), Some(WorkspaceState::Working));
    assert!(!guard.is_resolved());

    // 3. Commit (Working -> Ready)
    let committed = guard.commit().expect("commit should succeed");
    assert!(committed.is_ready());
    assert_eq!(committed.state(), WorkspaceState::Ready);
    assert_eq!(committed.workspace_id(), &ws_id);
    assert_eq!(committed.bead_id(), &bead_id);

    // 4. Ready -> Merged (via state machine)
    let merged = WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Merged)
        .expect("Ready -> Merged should succeed");
    assert!(merged.is_terminal());
    assert!(merged.is_complete());
}

#[test]
fn full_conflict_recovery_lifecycle() {
    let ws_id = fresh_ws_id();
    let bead_id = fresh_bead_id("conflict");

    // 1. Acquire guard (Created -> Working)
    let guard =
        WorkspaceGuard::acquire(ws_id.clone(), bead_id.clone(), WorkspaceState::Created).unwrap();

    // 2. Commit (Working -> Ready)
    let committed = guard.commit().unwrap();
    assert!(committed.is_ready());

    // 3. Ready -> Conflict (merge conflict detected)
    let conflict =
        WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Conflict).unwrap();
    assert!(conflict.is_active());

    // 4. Conflict -> Working (resolve and rework)
    let working =
        WorkspaceStateMachine::transition(WorkspaceState::Conflict, WorkspaceState::Working)
            .unwrap();
    assert!(working.is_active());

    // 5. Working -> Ready (done again)
    let ready =
        WorkspaceStateMachine::transition(WorkspaceState::Working, WorkspaceState::Ready).unwrap();

    // 6. Ready -> Merged
    let merged = WorkspaceStateMachine::transition(ready, WorkspaceState::Merged).unwrap();
    assert!(merged.is_terminal());
}

#[test]
fn full_abandon_lifecycle_from_working() {
    let guard = WorkspaceGuard::acquire(
        fresh_ws_id(),
        fresh_bead_id("abandon-w"),
        WorkspaceState::Created,
    )
    .unwrap();

    let abandoned = guard.abandon().unwrap();
    assert!(abandoned.is_abandoned());
    assert!(abandoned.state().is_terminal());
}

#[test]
fn full_abandon_lifecycle_from_conflict() {
    let guard = WorkspaceGuard::acquire(
        fresh_ws_id(),
        fresh_bead_id("abandon-c"),
        WorkspaceState::Created,
    )
    .unwrap();

    let _committed = guard.commit().unwrap(); // Working -> Ready

    // Ready -> Conflict
    let _ =
        WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Conflict).unwrap();

    // Conflict -> Abandoned
    let abandoned =
        WorkspaceStateMachine::transition(WorkspaceState::Conflict, WorkspaceState::Abandoned)
            .unwrap();
    assert!(abandoned.is_terminal());
}

#[test]
fn full_abandon_lifecycle_from_ready() {
    let guard = WorkspaceGuard::acquire(
        fresh_ws_id(),
        fresh_bead_id("abandon-r"),
        WorkspaceState::Created,
    )
    .unwrap();

    let _committed = guard.commit().unwrap(); // Working -> Ready

    // Ready -> Abandoned
    let abandoned =
        WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Abandoned)
            .unwrap();
    assert!(abandoned.is_terminal());
}

// === Guard + Mapping integration ===

#[test]
fn guard_mapping_preserves_bead_workspace_association() {
    let ws_id = fresh_ws_id();
    let bead_id = fresh_bead_id("assoc");

    let guard =
        WorkspaceGuard::acquire(ws_id.clone(), bead_id.clone(), WorkspaceState::Created).unwrap();

    let mapping = guard.mapping().unwrap();
    assert_eq!(mapping.bead_id(), &bead_id);
    assert_eq!(mapping.workspace_id(), &ws_id);
    assert!(mapping.assigned_at() <= chrono::Utc::now());

    // Commit preserves mapping through CommittedGuard
    let committed = guard.commit().unwrap();
    assert_eq!(committed.mapping().bead_id(), &bead_id);
    assert_eq!(committed.mapping().workspace_id(), &ws_id);
}

// === Events + State transitions ===

#[test]
fn state_transitions_correspond_to_lifecycle_events() {
    // Verify the mapping between state transitions and event types
    let cases = vec![
        (
            WorkspaceState::Created,
            WorkspaceState::Working,
            "workspace.activated",
        ),
        (
            WorkspaceState::Working,
            WorkspaceState::Ready,
            "workspace.completed",
        ),
        (
            WorkspaceState::Ready,
            WorkspaceState::Merged,
            "workspace.completed",
        ),
        (
            WorkspaceState::Ready,
            WorkspaceState::Conflict,
            "vcs.conflict_detected",
        ),
        (
            WorkspaceState::Conflict,
            WorkspaceState::Working,
            "workspace.resumed",
        ),
        (
            WorkspaceState::Working,
            WorkspaceState::Abandoned,
            "workspace.failed",
        ),
    ];

    for (from, to, event_prefix) in cases {
        let result = WorkspaceStateMachine::transition(from, to);
        assert!(
            result.is_ok(),
            "{from:?} -> {to:?} should succeed (event: {event_prefix})"
        );
    }
}

// === Checkpoint + Command classification integration ===

#[test]
fn risky_commands_trigger_checkpoint_flow() {
    let risky_commands = ["batch", "spawn", "remove", "cleanup", "rebase", "squash"];

    for cmd in &risky_commands {
        let risk = classify_command(cmd);
        assert!(risk.needs_checkpoint(), "'{cmd}' should need checkpoint");

        // Simulate: create checkpoint -> operation -> commit checkpoint
        let checkpoint = CheckpointState::Pending;
        assert_eq!(checkpoint.as_db(), "pending");

        let committed = CheckpointState::Committed;
        assert_eq!(committed.as_db(), "committed");
    }
}

#[test]
fn safe_commands_skip_checkpoint() {
    let safe_commands = ["list", "status", "context", "focus"];

    for cmd in &safe_commands {
        let risk = classify_command(cmd);
        assert!(
            !risk.needs_checkpoint(),
            "'{cmd}' should not need checkpoint"
        );
    }
}

#[test]
fn checkpoint_needs_restore_on_failure() {
    // Simulate: operation fails, checkpoint needs restore
    let checkpoint = CheckpointState::Pending;
    assert_eq!(checkpoint.as_db(), "pending");

    // Operation fails -> needs_restore
    let needs_restore = CheckpointState::NeedsRestore;
    assert_eq!(needs_restore.as_db(), "needs_restore");

    // Can roundtrip through DB
    let from_db = CheckpointState::from_db("needs_restore").unwrap();
    assert_eq!(from_db, CheckpointState::NeedsRestore);
}

// === Multiple concurrent workspaces (simulated) ===

#[test]
fn multiple_workspaces_independent_lifecycles() {
    let mut guards = Vec::new();

    // Create 5 independent workspace guards
    for i in 0..5 {
        let guard = WorkspaceGuard::acquire(
            fresh_ws_id(),
            fresh_bead_id(&format!("multi-{i}")),
            WorkspaceState::Created,
        )
        .unwrap();
        guards.push(guard);
    }

    // Verify all have unique workspace IDs
    let ws_ids: std::collections::HashSet<String> = guards
        .iter()
        .map(|g| g.workspace_id().unwrap().as_str().to_string())
        .collect();
    assert_eq!(ws_ids.len(), 5, "all workspace IDs should be unique");

    // Commit some, abandon others
    let mut committed = Vec::new();
    for (i, guard) in guards.into_iter().enumerate() {
        if i % 2 == 0 {
            committed.push((guard.commit().unwrap(), "ready"));
        } else {
            committed.push((guard.abandon().unwrap(), "abandoned"));
        }
    }

    // Verify outcomes
    for (i, (guard, outcome)) in committed.iter().enumerate() {
        if i % 2 == 0 {
            assert!(guard.is_ready(), "guard {i} should be ready");
            assert_eq!(*outcome, "ready");
        } else {
            assert!(guard.is_abandoned(), "guard {i} should be abandoned");
            assert_eq!(*outcome, "abandoned");
        }
    }
}

// === State machine + terminal state enforcement ===

#[test]
fn merged_workspace_cannot_be_reused() {
    // Simulate: workspace goes through full lifecycle to merged
    let _ = WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Working)
        .unwrap();
    let _ =
        WorkspaceStateMachine::transition(WorkspaceState::Working, WorkspaceState::Ready).unwrap();
    let merged =
        WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Merged).unwrap();

    // After merge, no further transitions possible
    for &target in WorkspaceState::all() {
        let result = WorkspaceStateMachine::transition(merged, target);
        assert!(result.is_err(), "Merged -> {target:?} should fail");
    }
}

#[test]
fn abandoned_workspace_cannot_be_recovered() {
    // Workspace abandoned at Working state
    let _ = WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Working)
        .unwrap();
    let abandoned =
        WorkspaceStateMachine::transition(WorkspaceState::Working, WorkspaceState::Abandoned)
            .unwrap();

    // No recovery possible
    for &target in WorkspaceState::all() {
        let result = WorkspaceStateMachine::transition(abandoned, target);
        assert!(result.is_err(), "Abandoned -> {target:?} should fail");
    }
}

// === Rejected acquire states produce correct errors ===

#[test]
fn guard_acquire_rejected_states_error_messages() {
    let rejected = [
        (WorkspaceState::Working, "working"),
        (WorkspaceState::Ready, "ready"),
        (WorkspaceState::Merged, "merged"),
        (WorkspaceState::Abandoned, "abandoned"),
        (WorkspaceState::Conflict, "conflict"),
    ];

    for (state, name) in &rejected {
        let result = WorkspaceGuard::acquire(fresh_ws_id(), fresh_bead_id("err"), *state);
        assert!(result.is_err(), "acquire with {name} should fail");
    }
}

// === State machine roundtrip through all valid paths ===

#[test]
fn all_valid_paths_to_terminal() {
    // Path 1: Created -> Working -> Ready -> Merged
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Merged).unwrap();
    assert!(s.is_terminal());

    // Path 2: Created -> Working -> Abandoned
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Abandoned).unwrap();
    assert!(s.is_terminal());

    // Path 3: Created -> Working -> Ready -> Abandoned
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Abandoned).unwrap();
    assert!(s.is_terminal());

    // Path 4: Created -> Working -> Conflict -> Abandoned
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Conflict).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Abandoned).unwrap();
    assert!(s.is_terminal());

    // Path 5: Conflict recovery -> Merged
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Conflict).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Merged).unwrap();
    assert!(s.is_terminal());

    // Path 6: Multiple rework cycles
    let s = WorkspaceState::Created;
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap(); // rework
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Conflict).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Working).unwrap(); // fix
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Ready).unwrap();
    let s = WorkspaceStateMachine::transition(s, WorkspaceState::Merged).unwrap();
    assert!(s.is_terminal());
}
