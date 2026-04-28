//! Red Queen Generation 3 — Property-based + stress adversarial tests
//!
//! Dimensions:
//! - proptest-invariants: Property-based invariant verification
//! - stress-concurrent: Heavy concurrent stress testing
//! - edge-semantics: Semantic edge cases in business logic

use std::{
    collections::HashSet,
    sync::{Arc, Barrier, Mutex},
    thread,
};

use scp_workspace::{
    domain::{
        entities::{workspace::Initializing, Workspace, WorkspaceId, WorkspaceState},
        events::WorkspaceEvent,
        state::WorkspaceStateMachine,
        value_objects::{BranchName, LockHolder, WorkspaceName, WorkspacePath},
    },
    infrastructure::workspace_repository::InMemoryWorkspaceRepository,
    WorkspaceError, WorkspaceRepository, WorkspaceService,
};

// ============================================================================
// DIMENSION: proptest-invariants
// ============================================================================

#[test]
fn rq_proptest_workspace_name_reflexivity_symmetry_transitivity() {
    // Reflexivity: a == a for all valid names
    for name in &[
        "a",
        "test-name",
        "under_score",
        "MixedCase123",
        "-prefix",
        "suffix-",
        "---",
    ] {
        let n = WorkspaceName::new((*name).into()).unwrap();
        assert_eq!(n, n, "Reflexivity failed for: {name}");
    }

    // Symmetry: a == b implies b == a
    let a = WorkspaceName::new("symmetric".into()).unwrap();
    let b = WorkspaceName::new("symmetric".into()).unwrap();
    assert_eq!(a, b);
    assert_eq!(b, a);

    // Transitivity: a == b and b == c implies a == c
    let c = WorkspaceName::new("symmetric".into()).unwrap();
    assert_eq!(a, b);
    assert_eq!(b, c);
    assert_eq!(a, c);
}

#[test]
fn rq_proptest_branch_name_reflexivity_symmetry() {
    for name in &["main", "feature/x", "a", "fix/issue-123"] {
        let n = BranchName::new((*name).into()).unwrap();
        assert_eq!(n, n);
    }
    let a = BranchName::new("test".into()).unwrap();
    let b = BranchName::new("test".into()).unwrap();
    assert_eq!(a, b);
    assert_eq!(b, a);
}

#[test]
fn rq_proptest_lock_holder_reflexivity_symmetry() {
    for name in &["agent-1", "a", "system", "  "] {
        let h = LockHolder::new((*name).into()).unwrap();
        assert_eq!(h, h);
    }
    let a = LockHolder::new("test".into()).unwrap();
    let b = LockHolder::new("test".into()).unwrap();
    assert_eq!(a, b);
    assert_eq!(b, a);
}

#[test]
fn rq_proptest_workspace_id_unique_batch_1000() {
    let mut ids = HashSet::new();
    for _ in 0..1000 {
        let id = WorkspaceId::generate();
        assert!(
            ids.insert(id.as_str().to_string()),
            "Duplicate ID generated!"
        );
    }
    assert_eq!(ids.len(), 1000);
}

#[test]
fn rq_proptest_workspace_id_generate_format() {
    for _ in 0..100 {
        let id = WorkspaceId::generate();
        let s = id.as_str();
        assert!(s.starts_with("ws-"), "ID should start with 'ws-': {s}");
        // UUID format: 8-4-4-4-12 = 36 chars
        assert_eq!(s.len(), 39, "ID should be 39 chars (ws- + UUID): {s}");
    }
}

#[test]
fn rq_proptest_state_machine_idempotent_transitions() {
    // Deleted → Deleted should always be valid (idempotent delete)
    for _ in 0..10 {
        assert!(WorkspaceStateMachine::can_transition(
            WorkspaceState::Deleted,
            WorkspaceState::Deleted
        ));
    }
}

#[test]
fn rq_proptest_workspace_event_equality_across_all_variants() {
    let ts = chrono::Utc::now();
    let pairs: Vec<(WorkspaceEvent, WorkspaceEvent)> = vec![
        (
            WorkspaceEvent::WorkspaceCreated {
                workspace_id: "ws-eq".into(),
                name: "test".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceCreated {
                workspace_id: "ws-eq".into(),
                name: "test".into(),
                timestamp: ts,
            },
        ),
        (
            WorkspaceEvent::WorkspaceLocked {
                workspace_id: "ws-eq".into(),
                holder: "agent".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceLocked {
                workspace_id: "ws-eq".into(),
                holder: "agent".into(),
                timestamp: ts,
            },
        ),
        (
            WorkspaceEvent::WorkspaceCorrupted {
                workspace_id: "ws-eq".into(),
                reason: "test".into(),
                timestamp: ts,
            },
            WorkspaceEvent::WorkspaceCorrupted {
                workspace_id: "ws-eq".into(),
                reason: "test".into(),
                timestamp: ts,
            },
        ),
    ];

    for (a, b) in pairs {
        assert_eq!(a, b);
    }
}

#[test]
fn rq_proptest_workspace_state_exhaustive_serialize_deserialize() {
    let all_states = [
        WorkspaceState::Initializing,
        WorkspaceState::Active,
        WorkspaceState::Locked,
        WorkspaceState::Corrupted,
        WorkspaceState::Deleted,
    ];

    // Serialize all
    let jsons: Vec<String> = all_states
        .iter()
        .map(|s| serde_json::to_string(s).unwrap())
        .collect();

    // Deserialize all
    let deserialized: Vec<WorkspaceState> = jsons
        .iter()
        .map(|j| serde_json::from_str(j).unwrap())
        .collect();

    // All should roundtrip
    for (original, rt) in all_states.iter().zip(deserialized.iter()) {
        assert_eq!(original, rt);
    }

    // All JSON strings should be unique (different states → different JSON)
    let json_set: HashSet<&str> = jsons.iter().map(|s| s.as_str()).collect();
    assert_eq!(json_set.len(), 5, "Each state should produce unique JSON");
}

// ============================================================================
// DIMENSION: stress-concurrent
// ============================================================================

#[test]
fn rq_stress_concurrent_save_100_threads() {
    let repo = Arc::new(InMemoryWorkspaceRepository::new());
    let num_threads = 100;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    for i in 0..num_threads {
        let repo_clone = Arc::clone(&repo);
        let barrier_clone = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier_clone.wait();
            let ws = Workspace::<Initializing>::create(
                WorkspaceName::new(format!("stress-{}", i)).unwrap(),
                WorkspacePath::new(format!("/tmp/rq-stress-{}", i)).unwrap(),
            )
            .unwrap();
            repo_clone.save(ws).unwrap()
        }));
    }

    let ids: Vec<String> = handles
        .into_iter()
        .map(|h| {
            h.join()
                .expect("Thread should not panic")
                .id
                .as_str()
                .to_string()
        })
        .collect();

    // All IDs should be unique
    let id_set: HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
    assert_eq!(id_set.len(), num_threads);

    // All should be retrievable
    for id in &ids {
        let parsed = WorkspaceId::parse(id.clone()).unwrap();
        let found = repo.get(&parsed).unwrap();
        assert!(found.is_some(), "Workspace {id} should be retrievable");
    }
}

#[test]
fn rq_stress_concurrent_read_write_delete_50_threads() {
    let repo = Arc::new(InMemoryWorkspaceRepository::new());
    let saved_ids = Arc::new(Mutex::new(Vec::new()));

    // Pre-populate 50 workspaces
    for i in 0..50 {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new(format!("rw-stress-{}", i)).unwrap(),
            WorkspacePath::new(format!("/tmp/rq-rw-stress-{}", i)).unwrap(),
        )
        .unwrap();
        let saved = repo.save(ws).unwrap();
        saved_ids.lock().unwrap().push(saved.id);
    }

    let num_threads = 50;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    // Half readers, half deleters
    for i in 0..num_threads {
        let repo_clone = Arc::clone(&repo);
        let barrier_clone = Arc::clone(&barrier);
        let ids_clone = Arc::clone(&saved_ids);

        handles.push(thread::spawn(move || {
            barrier_clone.wait();
            let ids = ids_clone.lock().unwrap();
            if i % 2 == 0 {
                // Reader: list all workspaces
                let list = repo_clone.list().unwrap();
                assert!(!list.is_empty());
            } else {
                // Deleter: delete one workspace
                if let Some(id) = ids.get(i / 2) {
                    let _ = repo_clone.delete(id); // may fail if already deleted
                }
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Thread should not panic");
    }
}

#[test]
fn rq_stress_lock_unlock_1000_cycles_single_thread() {
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("stress-cycles".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-stress-cycles".into()).unwrap(),
    )
    .unwrap();
    let active = ws.activate().unwrap();
    let id = active.id.as_str().to_string();

    let mut current = active;
    for i in 0..1000 {
        let locked = current.lock(format!("agent-{}", i)).unwrap();
        assert_eq!(locked.id.as_str(), id);
        let unlocked = locked.unlock().unwrap();
        assert_eq!(unlocked.id.as_str(), id);
        current = unlocked;
    }
}

// ============================================================================
// DIMENSION: edge-semantics
// ============================================================================

#[test]
fn rq_edge_deleted_workspace_no_further_methods() {
    // Workspace<Deleted> has no methods — this is a compile-time check
    // The struct exists but is a dead end
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("dead-end".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-dead-end".into()).unwrap(),
    )
    .unwrap();
    let deleted = ws.delete().unwrap();
    assert_eq!(deleted.state, WorkspaceState::Deleted);
    // Only accessors remain available on Workspace<Deleted>
    assert!(deleted.is_terminal());
    assert!(!deleted.is_active());
    assert!(!deleted.is_locked());
    assert_eq!(deleted.created_at(), deleted.created_at());
}

#[test]
fn rq_edge_corrupted_workspace_only_delete_allowed() {
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("corrupt-edge".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-corrupt-edge".into()).unwrap(),
    )
    .unwrap();
    let active = ws.activate().unwrap();
    let corrupted = active.mark_corrupted().unwrap();
    assert!(corrupted.is_terminal());
    // Only delete() is available on Workspace<Corrupted>
    let deleted = corrupted.delete().unwrap();
    assert_eq!(deleted.state, WorkspaceState::Deleted);
}

#[test]
fn rq_edge_active_workspace_all_transitions_available() {
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("active-edge".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-active-edge".into()).unwrap(),
    )
    .unwrap();
    let active = ws.activate().unwrap();

    // Can lock
    let locked = active.lock("agent".into()).unwrap();
    assert!(locked.is_locked());

    // Can mark corrupted
    let ws2 = Workspace::<Initializing>::create(
        WorkspaceName::new("active-corrupt".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-active-corrupt".into()).unwrap(),
    )
    .unwrap();
    let active2 = ws2.activate().unwrap();
    let corrupted = active2.mark_corrupted().unwrap();
    assert!(corrupted.is_terminal());

    // Can delete
    let ws3 = Workspace::<Initializing>::create(
        WorkspaceName::new("active-delete".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-active-delete".into()).unwrap(),
    )
    .unwrap();
    let active3 = ws3.activate().unwrap();
    let deleted = active3.delete().unwrap();
    assert_eq!(deleted.state, WorkspaceState::Deleted);
}

#[test]
fn rq_edge_locked_workspace_all_transitions_available() {
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("locked-edge".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-locked-edge".into()).unwrap(),
    )
    .unwrap();
    let active = ws.activate().unwrap();
    let locked = active.lock("holder".into()).unwrap();

    // Can unlock
    let unlocked = locked.unlock().unwrap();
    assert!(unlocked.is_active());

    // Can mark corrupted
    let ws2 = Workspace::<Initializing>::create(
        WorkspaceName::new("locked-corrupt".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-locked-corrupt".into()).unwrap(),
    )
    .unwrap();
    let active2 = ws2.activate().unwrap();
    let locked2 = active2.lock("h".into()).unwrap();
    let corrupted = locked2.mark_corrupted().unwrap();
    assert!(corrupted.is_terminal());

    // Can delete
    let ws3 = Workspace::<Initializing>::create(
        WorkspaceName::new("locked-delete".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-locked-delete".into()).unwrap(),
    )
    .unwrap();
    let active3 = ws3.activate().unwrap();
    let locked3 = active3.lock("h".into()).unwrap();
    let deleted = locked3.delete().unwrap();
    assert_eq!(deleted.state, WorkspaceState::Deleted);
}

#[test]
fn rq_edge_initializing_workspace_limited_transitions() {
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("init-edge".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-init-edge".into()).unwrap(),
    )
    .unwrap();

    // Can activate
    let ws2 = Workspace::<Initializing>::create(
        WorkspaceName::new("init-activate".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-init-activate".into()).unwrap(),
    )
    .unwrap();
    let active = ws2.activate().unwrap();
    assert!(active.is_active());

    // Can delete
    let deleted = ws.delete().unwrap();
    assert_eq!(deleted.state, WorkspaceState::Deleted);

    // Cannot lock (no lock method on Workspace<Initializing>)
    // Cannot unlock, mark_corrupted (only activate and delete)
}

#[test]
fn rq_edge_workspace_timestamp_monotonicity() {
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("ts-mono".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-ts-mono".into()).unwrap(),
    )
    .unwrap();
    let created_at = ws.created_at();
    let updated_at = ws.updated_at();
    assert!(updated_at >= created_at);

    std::thread::sleep(std::time::Duration::from_millis(5));

    let active = Workspace::<Initializing>::create(
        WorkspaceName::new("ts-mono-2".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-ts-mono-2".into()).unwrap(),
    )
    .unwrap();
    let activated = active.activate().unwrap();
    assert!(activated.updated_at() >= activated.created_at());
    assert!(activated.created_at() >= created_at);
}

#[test]
fn rq_edge_workspace_preserves_identity_through_all_transitions() {
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("identity".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-identity".into()).unwrap(),
    )
    .unwrap();
    let id = ws.id.as_str().to_string();
    let name = ws.name().as_str().to_string();
    let created_at = ws.created_at();

    let active = ws.activate().unwrap();
    assert_eq!(active.id.as_str(), id);
    assert_eq!(active.name().as_str(), name);
    assert_eq!(active.created_at(), created_at);

    let locked = active.lock("holder".into()).unwrap();
    assert_eq!(locked.id.as_str(), id);
    assert_eq!(locked.name().as_str(), name);
    assert_eq!(locked.created_at(), created_at);

    let unlocked = locked.unlock().unwrap();
    assert_eq!(unlocked.id.as_str(), id);
    assert_eq!(unlocked.name().as_str(), name);
    assert_eq!(unlocked.created_at(), created_at);

    let deleted = unlocked.delete().unwrap();
    assert_eq!(deleted.id.as_str(), id);
    assert_eq!(deleted.name().as_str(), name);
    assert_eq!(deleted.created_at(), created_at);
}

#[test]
fn rq_edge_error_send_sync_static() {
    // All error types must be Send + Sync + 'static for use in async contexts
    fn assert_traits<T: Send + Sync + std::error::Error + 'static>() {}
    assert_traits::<WorkspaceError>();
}
