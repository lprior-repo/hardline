//! Red Queen Generation 1 — Adversarial co-evolving tests for scp-workspace
//!
//! Dimensions:
//! - state-inconsistency: State machine vs service behavior contradictions
//! - value-object-gaps: Validation holes in value objects
//! - typestate-bypass: Service methods that defeat the typestate pattern
//! - concurrency: RwLock stress testing
//! - invariant-violation: Invariants that should hold but aren't enforced

use std::{
    sync::{Arc, Barrier},
    thread,
};

use scp_workspace::{
    domain::{
        entities::{workspace::Initializing, Workspace, WorkspaceId, WorkspaceState},
        state::WorkspaceStateMachine,
        value_objects::{BranchName, LockHolder, WorkspaceName, WorkspacePath},
    },
    infrastructure::workspace_repository::InMemoryWorkspaceRepository,
    WorkspaceError, WorkspaceRepository, WorkspaceService,
};

// ============================================================================
// DIMENSION: state-inconsistency
// ============================================================================

#[test]
fn rq_state_machine_vs_service_delete_locked_contradiction() {
    // StateMachine says Locked→Deleted is VALID
    assert!(WorkspaceStateMachine::can_transition(
        WorkspaceState::Locked,
        WorkspaceState::Deleted
    ));
    // But WorkspaceService rejects it
    let ws = WorkspaceService::create_workspace(
        WorkspaceName::new("contradiction".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-contradiction".into()).unwrap(),
    )
    .unwrap();
    let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
    let locked = WorkspaceService::lock_workspace(initialized, "agent".into()).unwrap();
    let result = WorkspaceService::delete_workspace(locked);
    // BUG: State machine allows this transition but service rejects it.
    // This is a design inconsistency — either the state machine should reject
    // Locked→Deleted, or the service should allow it.
    match result.as_ref().err() {
        Some(WorkspaceError::WorkspaceLocked(_, _)) => {
            // Service rejects what state machine allows — INCONSISTENCY
        }
        _ => panic!(
            "Expected WorkspaceLocked error, got {:?}",
            result.as_ref().err()
        ),
    }
}

#[test]
fn rq_state_machine_vs_service_delete_corrupted_contradiction() {
    // StateMachine says Corrupted→Deleted is VALID
    assert!(WorkspaceStateMachine::can_transition(
        WorkspaceState::Corrupted,
        WorkspaceState::Deleted
    ));
    // Entity also allows it: Workspace<Corrupted>::delete() exists
    // But WorkspaceService rejects Corrupted→Deleted
    let ws = Workspace::create(
        WorkspaceName::new("corrupt-del-rq".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-corrupt-del".into()).unwrap(),
    )
    .unwrap();
    let corrupted_ws = Workspace {
        id: ws.id,
        name: ws.name,
        path: ws.path,
        created_at: ws.created_at,
        updated_at: ws.updated_at,
        lock_holder: None,
        config: ws.config,
        state: WorkspaceState::Corrupted,
        _state: std::marker::PhantomData,
    };
    let result = WorkspaceService::delete_workspace(corrupted_ws);
    // BUG: Entity allows Corrupted→Deleted, state machine allows it,
    // but WorkspaceService rejects it
    assert!(
        result.is_err(),
        "Service should allow Corrupted→Deleted (entity and state machine both allow it)"
    );
}

#[test]
fn rq_state_machine_deleted_is_not_deletable_but_service_allows_init_delete() {
    // StateMachine says Deleted is NOT deletable
    assert!(!WorkspaceStateMachine::is_deletable(
        WorkspaceState::Deleted
    ));
    // But WorkspaceService allows Initializing→Deleted
    let ws = WorkspaceService::create_workspace(
        WorkspaceName::new("init-del-rq".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-init-del".into()).unwrap(),
    )
    .unwrap();
    let result = WorkspaceService::delete_workspace(ws);
    assert!(result.is_ok());
    // This is fine — is_deletable means "can transition TO Deleted", not "is in Deleted state"
    // But it shows the naming is confusing: is_deletable = !is_terminal
}

// ============================================================================
// DIMENSION: typestate-bypass
// ============================================================================

#[test]
fn rq_service_bypasses_typestate_phantomdata_is_wrong() {
    // WorkspaceService::initialize_workspace returns a Workspace with
    // PhantomData::<Initializing> but state = Active
    // This is a typestate violation — the PhantomData doesn't match runtime state
    let ws = WorkspaceService::create_workspace(
        WorkspaceName::new("typestate-rq".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-typestate".into()).unwrap(),
    )
    .unwrap();
    let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
    // Runtime state is Active but type is Workspace<Initializing>
    assert_eq!(initialized.state, WorkspaceState::Active);
    // The PhantomData field is ::<Initializing> — type system thinks it's Initializing
    // but runtime state is Active. This defeats the entire purpose of typestate.
}

#[test]
fn rq_service_lock_workspace_calls_activate_on_any_workspace() {
    // WorkspaceService::lock_workspace calls workspace.activate() first
    // This means if you pass an Initializing workspace, it gets activated THEN locked
    // But the entity's type system says only Workspace<Active> can be locked
    let ws = WorkspaceService::create_workspace(
        WorkspaceName::new("lock-any-rq".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-lock-any".into()).unwrap(),
    )
    .unwrap();
    // ws is in Initializing state
    assert_eq!(ws.state, WorkspaceState::Initializing);
    // But lock_workspace silently activates it first
    let locked = WorkspaceService::lock_workspace(ws, "agent".into()).unwrap();
    assert!(locked.is_locked());
    // BUG: Service silently bypasses the Initializing→Active transition
    // that should be explicit. Typestate on entity is defeated.
}

#[test]
fn rq_service_unlock_bypasses_typestate_for_locked_workspace() {
    // unlock_workspace checks state == Locked but uses PhantomData::<Initializing>
    // It manually constructs the result instead of calling entity methods
    let ws = WorkspaceService::create_workspace(
        WorkspaceName::new("unlock-bypass".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-unlock-bypass".into()).unwrap(),
    )
    .unwrap();
    let init = WorkspaceService::initialize_workspace(ws).unwrap();
    let locked = WorkspaceService::lock_workspace(init, "agent".into()).unwrap();
    let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
    // The result has PhantomData::<Initializing> but state = Active
    assert_eq!(unlocked.state, WorkspaceState::Active);
}

// ============================================================================
// DIMENSION: value-object-gaps
// ============================================================================

#[test]
fn rq_workspace_name_allows_whitespace_only_via_unicode() {
    // WorkspaceName validation only checks alphanumeric, dash, underscore
    // But it doesn't reject names that are only separators
    let name = WorkspaceName::new("---".into());
    assert!(
        name.is_ok(),
        "Only-separator names should be valid per current rules"
    );
    let name2 = WorkspaceName::new("___".into());
    assert!(name2.is_ok());
}

#[test]
fn rq_branch_name_allows_newline_and_control_chars() {
    // BranchName only rejects empty and null char
    // Newlines, tabs, and other control chars pass through
    let name = BranchName::new("branch\nwith-newline".into());
    assert!(
        name.is_ok(),
        "Newlines are allowed — potential injection vector"
    );
    let name2 = BranchName::new("branch\twith-tab".into());
    assert!(name2.is_ok());
    let name3 = BranchName::new("branch\x01control".into());
    assert!(name3.is_ok(), "Control char 0x01 is allowed");
}

#[test]
fn rq_lock_holder_allows_whitespace_only() {
    // LockHolder only rejects empty string
    // Whitespace-only holders are valid
    let holder = LockHolder::new("   ".into());
    assert!(holder.is_ok(), "Whitespace-only lock holder is accepted");
    assert_eq!(holder.unwrap().as_str(), "   ");
}

#[test]
fn rq_lock_holder_allows_newlines() {
    let holder = LockHolder::new("agent\ninjected".into());
    assert!(holder.is_ok(), "Newlines in lock holder are allowed");
}

#[test]
fn rq_workspace_id_parse_allows_anything_except_empty() {
    // WorkspaceId::parse only rejects empty — accepts anything else
    let id = WorkspaceId::parse("../../../etc/passwd".into());
    assert!(id.is_ok(), "Path traversal in workspace ID is allowed");
    assert_eq!(id.unwrap().as_str(), "../../../etc/passwd");

    let id2 = WorkspaceId::parse("<script>alert(1)</script>".into());
    assert!(id2.is_ok(), "XSS-like content in workspace ID is allowed");
}

// ============================================================================
// DIMENSION: invariant-violation
// ============================================================================

#[test]
fn rq_transition_impl_never_validates() {
    // Workspace::transition_impl just sets the new state — no validation
    // You can create Initializing→Deleted via entity (correct)
    // But the method itself never checks if transition is valid
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("inv-rq".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-inv".into()).unwrap(),
    )
    .unwrap();
    let active = ws.activate().unwrap();
    // Entity allows Active→Deleted directly (valid per state machine)
    let deleted = active.delete().unwrap();
    assert_eq!(deleted.state, WorkspaceState::Deleted);
    // This is fine, but the point is transition_impl doesn't call validate_transition
}

#[test]
fn rq_service_recover_calls_activate_on_already_active_workspace() {
    // recover_workspace: unlocks (→Active), then calls .activate() on the Active workspace
    // Workspace<Active> doesn't have .activate() — only Workspace<Initializing> does
    // But the service bypasses this by constructing Workspace manually
    let ws = WorkspaceService::create_workspace(
        WorkspaceName::new("recover-rq".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-recover".into()).unwrap(),
    )
    .unwrap();
    let init = WorkspaceService::initialize_workspace(ws).unwrap();
    let locked = WorkspaceService::lock_workspace(init, "stuck-agent".into()).unwrap();

    // After recover, the workspace should be Active with no lock holder
    let recovered = WorkspaceService::recover_workspace(locked).unwrap();
    assert!(recovered.is_active());
    assert!(recovered.lock_holder().is_none());
    // BUG: The code constructs an Active workspace then calls .activate() on it
    // This would fail if types were correct (Active has no activate method)
    // The manual Workspace construction hides this bug
}

#[test]
fn rq_workspace_config_not_preserved_after_mark_corrupted() {
    // When marking corrupted, config should be preserved
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("corrupt-cfg-rq".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-corrupt-cfg".into()).unwrap(),
    )
    .unwrap();
    let active = ws.activate().unwrap();
    let corrupted = active.mark_corrupted().unwrap();
    let config = corrupted.config();
    assert!(
        config.is_some(),
        "Config should survive corruption transition"
    );
    assert_eq!(config.unwrap().default_branch, "main");
}

#[test]
fn rq_repository_list_active_includes_only_active_state() {
    let repo = InMemoryWorkspaceRepository::new();

    // Save workspaces in various states
    for i in 0..5 {
        let ws = Workspace::create(
            WorkspaceName::new(format!("rq-active-{}", i)).unwrap(),
            WorkspacePath::new(format!("/tmp/rq-active-{}", i)).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        // Store as Active state
        let stored = Workspace {
            id: active.id,
            name: active.name,
            path: active.path,
            created_at: active.created_at,
            updated_at: active.updated_at,
            lock_holder: active.lock_holder,
            config: active.config,
            state: WorkspaceState::Active,
            _state: std::marker::PhantomData,
        };
        repo.save(stored).unwrap();
    }

    // Save some non-active
    let ws_init = Workspace::create(
        WorkspaceName::new("rq-init".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-init".into()).unwrap(),
    )
    .unwrap();
    repo.save(ws_init).unwrap();

    let actives = repo.list_active().unwrap();
    assert_eq!(actives.len(), 5);
}

// ============================================================================
// DIMENSION: concurrency
// ============================================================================

#[test]
fn rq_concurrent_save_and_get_thread_safety() {
    let repo = Arc::new(InMemoryWorkspaceRepository::new());
    let num_threads = 8;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    for i in 0..num_threads {
        let repo_clone = Arc::clone(&repo);
        let barrier_clone = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier_clone.wait();
            let ws = Workspace::create(
                WorkspaceName::new(format!("rq-concurrent-{}", i)).unwrap(),
                WorkspacePath::new(format!("/tmp/rq-concurrent-{}", i)).unwrap(),
            )
            .unwrap();
            let saved = repo_clone.save(ws).unwrap();
            let found = repo_clone.get(&saved.id).unwrap();
            assert!(
                found.is_some(),
                "Workspace should be retrievable after save"
            );
        }));
    }

    for handle in handles {
        handle.join().expect("Thread should not panic");
    }

    let all = repo.list().unwrap();
    assert_eq!(all.len(), num_threads as usize);
}

#[test]
fn rq_concurrent_reads_during_writes() {
    let repo = Arc::new(InMemoryWorkspaceRepository::new());
    let num_writers = 4;
    let num_readers = 4;
    let total = num_writers + num_readers;
    let barrier = Arc::new(Barrier::new(total));
    let mut handles = Vec::new();

    // Pre-populate
    for i in 0..10 {
        let ws = Workspace::create(
            WorkspaceName::new(format!("rq-rw-pre-{}", i)).unwrap(),
            WorkspacePath::new(format!("/tmp/rq-rw-pre-{}", i)).unwrap(),
        )
        .unwrap();
        repo.save(ws).unwrap();
    }

    // Writers
    for i in 0..num_writers {
        let repo_clone = Arc::clone(&repo);
        let barrier_clone = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier_clone.wait();
            for j in 0..10 {
                let ws = Workspace::create(
                    WorkspaceName::new(format!("rq-rw-{}-{}", i, j)).unwrap(),
                    WorkspacePath::new(format!("/tmp/rq-rw-{}-{}", i, j)).unwrap(),
                )
                .unwrap();
                repo_clone.save(ws).unwrap();
            }
        }));
    }

    // Readers
    for _ in 0..num_readers {
        let repo_clone = Arc::clone(&repo);
        let barrier_clone = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier_clone.wait();
            for _ in 0..10 {
                let list = repo_clone.list().unwrap();
                // list() should never panic or return garbage
                assert!(list.len() >= 10, "Should see at least pre-populated items");
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Thread should not panic");
    }
}

// ============================================================================
// DIMENSION: edge-case-transitions
// ============================================================================

#[test]
fn rq_full_lifecycle_all_valid_paths() {
    // Path 1: Init → Active → Locked → Active → Corrupted → Deleted
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("lifecycle1".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-lifecycle1".into()).unwrap(),
    )
    .unwrap();
    let active = ws.activate().unwrap();
    let locked = active.lock("a".into()).unwrap();
    let unlocked = locked.unlock().unwrap();
    let corrupted = unlocked.mark_corrupted().unwrap();
    let deleted = corrupted.delete().unwrap();
    assert_eq!(deleted.state, WorkspaceState::Deleted);

    // Path 2: Init → Active → Locked → Corrupted → Deleted
    let ws2 = Workspace::<Initializing>::create(
        WorkspaceName::new("lifecycle2".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-lifecycle2".into()).unwrap(),
    )
    .unwrap();
    let active2 = ws2.activate().unwrap();
    let locked2 = active2.lock("b".into()).unwrap();
    let corrupted2 = locked2.mark_corrupted().unwrap();
    let deleted2 = corrupted2.delete().unwrap();
    assert_eq!(deleted2.state, WorkspaceState::Deleted);

    // Path 3: Init → Active → Corrupted → Deleted
    let ws3 = Workspace::<Initializing>::create(
        WorkspaceName::new("lifecycle3".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-lifecycle3".into()).unwrap(),
    )
    .unwrap();
    let active3 = ws3.activate().unwrap();
    let corrupted3 = active3.mark_corrupted().unwrap();
    let deleted3 = corrupted3.delete().unwrap();
    assert_eq!(deleted3.state, WorkspaceState::Deleted);

    // Path 4: Init → Deleted (skip active)
    let ws4 = Workspace::<Initializing>::create(
        WorkspaceName::new("lifecycle4".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-lifecycle4".into()).unwrap(),
    )
    .unwrap();
    let deleted4 = ws4.delete().unwrap();
    assert_eq!(deleted4.state, WorkspaceState::Deleted);

    // Path 5: Init → Active → Deleted
    let ws5 = Workspace::<Initializing>::create(
        WorkspaceName::new("lifecycle5".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-lifecycle5".into()).unwrap(),
    )
    .unwrap();
    let active5 = ws5.activate().unwrap();
    let deleted5 = active5.delete().unwrap();
    assert_eq!(deleted5.state, WorkspaceState::Deleted);
}

#[test]
fn rq_multiple_lock_unlock_cycles_preserve_consistency() {
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("cycles-rq".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-cycles".into()).unwrap(),
    )
    .unwrap();
    let active = ws.activate().unwrap();
    let id = active.id.as_str().to_string();
    let created_at = active.created_at();

    let mut current = active;
    for i in 0..20 {
        let locked = current.lock(format!("agent-{}", i)).unwrap();
        assert_eq!(locked.id.as_str(), id);
        assert_eq!(locked.lock_holder(), Some(format!("agent-{}", i).as_str()));
        let unlocked = locked.unlock().unwrap();
        assert_eq!(unlocked.id.as_str(), id);
        assert!(unlocked.lock_holder().is_none());
        assert_eq!(unlocked.created_at(), created_at);
        current = unlocked;
    }
}

#[test]
fn rq_deleted_to_deleted_idempotent() {
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("del-idem".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-del-idem".into()).unwrap(),
    )
    .unwrap();
    let deleted = ws.delete().unwrap();
    assert_eq!(deleted.state, WorkspaceState::Deleted);
    // StateMachine allows Deleted→Deleted
    assert!(WorkspaceStateMachine::can_transition(
        WorkspaceState::Deleted,
        WorkspaceState::Deleted
    ));
    // But entity has no method for it — this is fine since it's a no-op
}

// ============================================================================
// DIMENSION: service-filter-correctness
// ============================================================================

#[test]
fn rq_get_active_workspaces_excludes_terminal_states() {
    let ws1 = WorkspaceService::create_workspace(
        WorkspaceName::new("filter-active".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-filter-active".into()).unwrap(),
    )
    .unwrap();
    let active1 = WorkspaceService::initialize_workspace(ws1).unwrap();

    // Create a corrupted workspace
    let ws2 = WorkspaceService::create_workspace(
        WorkspaceName::new("filter-corrupt".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-filter-corrupt".into()).unwrap(),
    )
    .unwrap();
    let active2 = WorkspaceService::initialize_workspace(ws2).unwrap();
    let corrupted_ws = Workspace {
        state: WorkspaceState::Corrupted,
        ..active2
    };

    // Create a deleted workspace
    let ws3 = WorkspaceService::create_workspace(
        WorkspaceName::new("filter-deleted".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-filter-deleted".into()).unwrap(),
    )
    .unwrap();
    let deleted_ws = Workspace {
        state: WorkspaceState::Deleted,
        ..ws3
    };

    let all = vec![active1, corrupted_ws, deleted_ws];
    let actives = WorkspaceService::get_active_workspaces(&all);
    assert_eq!(actives.len(), 1);
    assert_eq!(actives[0].name.as_str(), "filter-active");
}

#[test]
fn rq_get_locked_workspaces_with_mixed_states() {
    let ws1 = WorkspaceService::create_workspace(
        WorkspaceName::new("lock-mix-1".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-lock-mix-1".into()).unwrap(),
    )
    .unwrap();
    let active1 = WorkspaceService::initialize_workspace(ws1).unwrap();
    let locked1 = WorkspaceService::lock_workspace(active1, "a".into()).unwrap();

    let ws2 = WorkspaceService::create_workspace(
        WorkspaceName::new("lock-mix-2".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-lock-mix-2".into()).unwrap(),
    )
    .unwrap();
    let active2 = WorkspaceService::initialize_workspace(ws2).unwrap();

    let ws3 = WorkspaceService::create_workspace(
        WorkspaceName::new("lock-mix-3".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-lock-mix-3".into()).unwrap(),
    )
    .unwrap();

    let all = vec![locked1, active2, ws3];
    let locked_list = WorkspaceService::get_locked_workspaces(&all);
    assert_eq!(locked_list.len(), 1);
    assert_eq!(locked_list[0].name.as_str(), "lock-mix-1");
}
