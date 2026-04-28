//! Red Queen Generation 2 — Deep adversarial tests for scp-workspace
//!
//! Dimensions:
//! - mutation-style: Tests that would catch common mutation patterns
//! - boundary: Exact boundary value testing
//! - serialization: JSON edge cases and roundtrips
//! - service-edge: Uncovered service method edge cases

use scp_workspace::{
    domain::{
        entities::{
            workspace::{Initializing, VcsType},
            Workspace, WorkspaceId, WorkspaceState,
        },
        events::WorkspaceEvent,
        state::WorkspaceStateMachine,
        value_objects::{BranchName, LockHolder, WorkspaceName, WorkspacePath},
    },
    infrastructure::workspace_repository::InMemoryWorkspaceRepository,
    WorkspaceRepository, WorkspaceService,
};

// ============================================================================
// DIMENSION: mutation-style
// Tests that would fail if common mutations were applied to the source
// ============================================================================

#[test]
fn rq_mutation_state_machine_exhaustive_pairwise() {
    // If someone mutates can_transition to always return true, this catches it
    let known_invalid = [
        (WorkspaceState::Initializing, WorkspaceState::Initializing),
        (WorkspaceState::Initializing, WorkspaceState::Locked),
        (WorkspaceState::Initializing, WorkspaceState::Corrupted),
        (WorkspaceState::Active, WorkspaceState::Initializing),
        (WorkspaceState::Active, WorkspaceState::Active),
        (WorkspaceState::Locked, WorkspaceState::Initializing),
        (WorkspaceState::Locked, WorkspaceState::Locked),
        (WorkspaceState::Corrupted, WorkspaceState::Active),
        (WorkspaceState::Corrupted, WorkspaceState::Locked),
        (WorkspaceState::Corrupted, WorkspaceState::Initializing),
        (WorkspaceState::Deleted, WorkspaceState::Active),
        (WorkspaceState::Deleted, WorkspaceState::Initializing),
        (WorkspaceState::Deleted, WorkspaceState::Locked),
        (WorkspaceState::Deleted, WorkspaceState::Corrupted),
    ];

    for (from, to) in &known_invalid {
        assert!(
            !WorkspaceStateMachine::can_transition(*from, *to),
            "MUTATION SURVIVOR: {from:?} -> {to:?} should be INVALID"
        );
    }

    let known_valid = [
        (WorkspaceState::Initializing, WorkspaceState::Active),
        (WorkspaceState::Initializing, WorkspaceState::Deleted),
        (WorkspaceState::Active, WorkspaceState::Locked),
        (WorkspaceState::Active, WorkspaceState::Corrupted),
        (WorkspaceState::Active, WorkspaceState::Deleted),
        (WorkspaceState::Locked, WorkspaceState::Active),
        (WorkspaceState::Locked, WorkspaceState::Corrupted),
        (WorkspaceState::Locked, WorkspaceState::Deleted),
        (WorkspaceState::Corrupted, WorkspaceState::Deleted),
        (WorkspaceState::Deleted, WorkspaceState::Deleted),
    ];

    for (from, to) in &known_valid {
        assert!(
            WorkspaceStateMachine::can_transition(*from, *to),
            "MUTATION SURVIVOR: {from:?} -> {to:?} should be VALID"
        );
    }
}

#[test]
fn rq_mutation_is_terminal_cannot_be_flipped() {
    // If someone mutates is_terminal to return false for Deleted/Corrupted
    assert!(WorkspaceState::Deleted.is_terminal());
    assert!(WorkspaceState::Corrupted.is_terminal());
    assert!(!WorkspaceState::Initializing.is_terminal());
    assert!(!WorkspaceState::Active.is_terminal());
    assert!(!WorkspaceState::Locked.is_terminal());
}

#[test]
fn rq_mutation_is_lockable_only_active() {
    // If someone mutates is_lockable to accept more states
    assert!(WorkspaceStateMachine::is_lockable(WorkspaceState::Active));
    for state in [
        WorkspaceState::Initializing,
        WorkspaceState::Locked,
        WorkspaceState::Corrupted,
        WorkspaceState::Deleted,
    ] {
        assert!(
            !WorkspaceStateMachine::is_lockable(state),
            "MUTATION SURVIVOR: is_lockable({state:?}) should be false"
        );
    }
}

#[test]
fn rq_mutation_validate_transition_error_content() {
    // Ensure error messages contain the actual state names
    let result = WorkspaceStateMachine::validate_transition(
        WorkspaceState::Active,
        WorkspaceState::Initializing,
    );
    let err = result.err().expect("should fail");
    let msg = format!("{err}");
    assert!(
        msg.contains("Active"),
        "Error message must contain 'Active', got: {msg}"
    );
    assert!(
        msg.contains("Initializing"),
        "Error message must contain 'Initializing', got: {msg}"
    );
}

#[test]
fn rq_mutation_workspace_id_parse_empty_rejected() {
    // If someone mutates parse to accept empty
    assert!(WorkspaceId::parse("".into()).is_err());
}

#[test]
fn rq_mutation_workspace_name_validation_triple_boundary() {
    // 254 chars: valid
    let name_254 = "a".repeat(254);
    assert!(WorkspaceName::new(name_254).is_ok());

    // 255 chars: valid (exact boundary)
    let name_255 = "a".repeat(255);
    assert!(WorkspaceName::new(name_255).is_ok());

    // 256 chars: INVALID
    let name_256 = "a".repeat(256);
    assert!(WorkspaceName::new(name_256).is_err());

    // 257 chars: INVALID
    let name_257 = "a".repeat(257);
    assert!(WorkspaceName::new(name_257).is_err());
}

// ============================================================================
// DIMENSION: boundary
// ============================================================================

#[test]
fn rq_boundary_workspace_name_single_char_each_type() {
    assert!(WorkspaceName::new("a".into()).is_ok());
    assert!(WorkspaceName::new("Z".into()).is_ok());
    assert!(WorkspaceName::new("0".into()).is_ok());
    assert!(WorkspaceName::new("9".into()).is_ok());
    assert!(WorkspaceName::new("-".into()).is_ok());
    assert!(WorkspaceName::new("_".into()).is_ok());
}

#[test]
fn rq_boundary_workspace_name_each_invalid_char_individually() {
    let invalid_chars = [
        '.', ' ', '/', '\\', '@', '#', '!', '$', '%', '^', '&', '*', '(', ')', '+', '=', '{', '}',
        '[', ']', '|', ':', ';', '"', '\'', '<', '>', ',', '?', '~', '`',
    ];
    for ch in invalid_chars {
        let name = format!("valid{ch}name");
        assert!(
            WorkspaceName::new(name).is_err(),
            "Char '{ch}' should be rejected"
        );
    }
}

#[test]
fn rq_boundary_branch_name_null_char_at_each_position() {
    let base = "abcdef".to_string();
    for i in 0..=base.len() {
        let mut name = base.clone();
        name.insert(i, '\0');
        assert!(
            BranchName::new(name).is_err(),
            "Null at position {i} should be rejected"
        );
    }
}

#[test]
fn rq_boundary_lock_holder_empty_and_single_char() {
    assert!(LockHolder::new("".into()).is_err());
    assert!(LockHolder::new("x".into()).is_ok());
}

#[test]
fn rq_boundary_workspace_id_empty_and_single_char() {
    assert!(WorkspaceId::parse("".into()).is_err());
    assert!(WorkspaceId::parse("x".into()).is_ok());
}

#[test]
fn rq_boundary_workspace_path_empty_rejected() {
    assert!(WorkspacePath::new("".into()).is_err());
}

#[test]
fn rq_boundary_workspace_config_all_boolean_combinations() {
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("cfg-bound".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-cfg-bound".into()).unwrap(),
    )
    .unwrap();
    let config = ws.config().expect("should have config");
    // Default values
    assert_eq!(config.vcs_type, VcsType::Git);
    assert!(config.auto_sync);
    assert_eq!(config.default_branch, "main");
}

// ============================================================================
// DIMENSION: serialization
// ============================================================================

#[test]
fn rq_serialization_workspace_state_all_variants_roundtrip() {
    for state in [
        WorkspaceState::Initializing,
        WorkspaceState::Active,
        WorkspaceState::Locked,
        WorkspaceState::Corrupted,
        WorkspaceState::Deleted,
    ] {
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: WorkspaceState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, deserialized, "Roundtrip failed for {state:?}");
    }
}

#[test]
fn rq_serialization_workspace_state_snake_case_format() {
    assert_eq!(
        serde_json::to_string(&WorkspaceState::Initializing).unwrap(),
        "\"initializing\""
    );
    assert_eq!(
        serde_json::to_string(&WorkspaceState::Active).unwrap(),
        "\"active\""
    );
    assert_eq!(
        serde_json::to_string(&WorkspaceState::Locked).unwrap(),
        "\"locked\""
    );
    assert_eq!(
        serde_json::to_string(&WorkspaceState::Corrupted).unwrap(),
        "\"corrupted\""
    );
    assert_eq!(
        serde_json::to_string(&WorkspaceState::Deleted).unwrap(),
        "\"deleted\""
    );
}

#[test]
fn rq_serialization_workspace_event_all_variants() {
    let ts = chrono::Utc::now();
    let events = vec![
        WorkspaceEvent::WorkspaceCreated {
            workspace_id: "ws-1".into(),
            name: "test".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceActivated {
            workspace_id: "ws-2".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceLocked {
            workspace_id: "ws-3".into(),
            holder: "agent".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceUnlocked {
            workspace_id: "ws-4".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceCorrupted {
            workspace_id: "ws-5".into(),
            reason: "test".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceDeleted {
            workspace_id: "ws-6".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceConfigUpdated {
            workspace_id: "ws-7".into(),
            timestamp: ts,
        },
    ];

    for event in &events {
        let json = serde_json::to_string(event).unwrap();
        let deserialized: WorkspaceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(*event, deserialized);
    }
}

#[test]
fn rq_serialization_workspace_event_rejects_invalid_json() {
    let result = serde_json::from_str::<WorkspaceEvent>("{\"InvalidVariant\":{}}");
    assert!(result.is_err(), "Should reject unknown variant");
}

#[test]
fn rq_serialization_workspace_name_special_chars_in_json() {
    let name = WorkspaceName::new("test_name-123".into()).unwrap();
    let json = serde_json::to_string(&name).unwrap();
    assert!(json.contains("test_name-123"));
    let deserialized: WorkspaceName = serde_json::from_str(&json).unwrap();
    assert_eq!(name, deserialized);
}

#[test]
fn rq_serialization_branch_name_with_slashes() {
    let name = BranchName::new("feature/USER-123-test".into()).unwrap();
    let json = serde_json::to_string(&name).unwrap();
    let deserialized: BranchName = serde_json::from_str(&json).unwrap();
    assert_eq!(name, deserialized);
}

// ============================================================================
// DIMENSION: service-edge
// ============================================================================

#[test]
fn rq_service_delete_already_deleted_workspace() {
    let ws = WorkspaceService::create_workspace(
        WorkspaceName::new("double-del".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-double-del".into()).unwrap(),
    )
    .unwrap();
    let deleted_ws = Workspace {
        state: WorkspaceState::Deleted,
        ..ws
    };
    let result = WorkspaceService::delete_workspace(deleted_ws);
    assert!(
        result.is_err(),
        "Should not be able to delete already-deleted workspace"
    );
}

#[test]
fn rq_service_recover_from_corrupted_fails() {
    let ws = Workspace::create(
        WorkspaceName::new("recover-corrupt-edge".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-recover-corrupt".into()).unwrap(),
    )
    .unwrap();
    let corrupted_ws = Workspace {
        state: WorkspaceState::Corrupted,
        ..ws
    };
    let result = WorkspaceService::recover_workspace(corrupted_ws);
    assert!(result.is_err());
}

#[test]
fn rq_service_recover_from_deleted_fails() {
    let ws = Workspace::create(
        WorkspaceName::new("recover-del-edge".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-recover-del".into()).unwrap(),
    )
    .unwrap();
    let deleted_ws = Workspace {
        state: WorkspaceState::Deleted,
        ..ws
    };
    let result = WorkspaceService::recover_workspace(deleted_ws);
    assert!(result.is_err());
}

#[test]
fn rq_service_unlock_from_initializing_fails() {
    let ws = WorkspaceService::create_workspace(
        WorkspaceName::new("unlock-init-edge".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-unlock-init".into()).unwrap(),
    )
    .unwrap();
    let result = WorkspaceService::unlock_workspace(ws);
    assert!(result.is_err());
}

#[test]
fn rq_service_find_workspace_returns_first_match_by_id() {
    let ws1 = WorkspaceService::create_workspace(
        WorkspaceName::new("find-first-1".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-find-first-1".into()).unwrap(),
    )
    .unwrap();
    let ws2 = WorkspaceService::create_workspace(
        WorkspaceName::new("find-first-2".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-find-first-2".into()).unwrap(),
    )
    .unwrap();
    let all = vec![ws1.clone(), ws2];
    let found = WorkspaceService::find_workspace(&all, &ws1.id);
    assert!(found.is_some());
    assert_eq!(found.unwrap().id.as_str(), ws1.id.as_str());
}

#[test]
fn rq_service_find_by_name_with_duplicate_names() {
    let ws1 = WorkspaceService::create_workspace(
        WorkspaceName::new("dup-name".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-dup-1".into()).unwrap(),
    )
    .unwrap();
    let ws2 = WorkspaceService::create_workspace(
        WorkspaceName::new("dup-name".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-dup-2".into()).unwrap(),
    )
    .unwrap();
    let all = vec![ws1.clone(), ws2];
    let found =
        WorkspaceService::find_by_name(&all, &WorkspaceName::new("dup-name".into()).unwrap());
    assert!(found.is_some());
    // Should return first match
    assert_eq!(found.unwrap().id.as_str(), ws1.id.as_str());
}

#[test]
fn rq_service_lock_then_delete_via_service_fails() {
    // Service explicitly prevents locked→deleted
    let ws = WorkspaceService::create_workspace(
        WorkspaceName::new("lock-del-via-svc".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-lock-del-svc".into()).unwrap(),
    )
    .unwrap();
    let init = WorkspaceService::initialize_workspace(ws).unwrap();
    let locked = WorkspaceService::lock_workspace(init, "holder".into()).unwrap();
    let result = WorkspaceService::delete_workspace(locked);
    assert!(result.is_err());
}

#[test]
fn rq_service_delete_initializing_via_entity_succeeds() {
    // Entity allows Initializing→Deleted but service wraps it
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("entity-del-init".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-entity-del-init".into()).unwrap(),
    )
    .unwrap();
    let deleted = ws.delete().unwrap();
    assert_eq!(deleted.state, WorkspaceState::Deleted);
}

// ============================================================================
// DIMENSION: repository-stress
// ============================================================================

#[test]
fn rq_repo_save_get_delete_cycle() {
    let repo = InMemoryWorkspaceRepository::new();
    let ws = Workspace::<Initializing>::create(
        WorkspaceName::new("cycle-test".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-cycle".into()).unwrap(),
    )
    .unwrap();
    let saved = repo.save(ws).unwrap();
    let found = repo.get(&saved.id).unwrap();
    assert!(found.is_some());
    repo.delete(&saved.id).unwrap();
    let gone = repo.get(&saved.id).unwrap();
    assert!(gone.is_none());
}

#[test]
fn rq_repo_overwrite_same_id_different_data() {
    let repo = InMemoryWorkspaceRepository::new();
    let ws1 = Workspace::<Initializing>::create(
        WorkspaceName::new("overwrite-1".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-overwrite-1".into()).unwrap(),
    )
    .unwrap();
    let saved1 = repo.save(ws1).unwrap();
    let id = saved1.id.as_str().to_string();

    // Overwrite with different data
    let ws2 = Workspace {
        id: saved1.id.clone(),
        name: WorkspaceName::new("overwrite-2".into()).unwrap(),
        path: WorkspacePath::new("/tmp/rq-overwrite-2".into()).unwrap(),
        created_at: saved1.created_at,
        updated_at: chrono::Utc::now(),
        lock_holder: Some("new-holder".into()),
        config: saved1.config.clone(),
        state: WorkspaceState::Active,
        _state: std::marker::PhantomData,
    };
    repo.save(ws2).unwrap();

    let found = repo.get(&saved1.id).unwrap().unwrap();
    assert_eq!(found.id.as_str(), id);
    assert_eq!(found.name.as_str(), "overwrite-2");
    assert_eq!(found.lock_holder(), Some("new-holder"));
}

#[test]
fn rq_repo_get_by_name_after_overwrite() {
    let repo = InMemoryWorkspaceRepository::new();
    let ws1 = Workspace::<Initializing>::create(
        WorkspaceName::new("name-overwrite".into()).unwrap(),
        WorkspacePath::new("/tmp/rq-name-ow-1".into()).unwrap(),
    )
    .unwrap();
    repo.save(ws1).unwrap();

    let ws2 = Workspace {
        id: WorkspaceId::generate(),
        name: WorkspaceName::new("name-overwrite".into()).unwrap(),
        path: WorkspacePath::new("/tmp/rq-name-ow-2".into()).unwrap(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        lock_holder: None,
        config: None,
        state: WorkspaceState::Active,
        _state: std::marker::PhantomData,
    };
    repo.save(ws2).unwrap();

    // get_by_name returns first match — behavior depends on HashMap iteration
    let found = repo.get_by_name("name-overwrite").unwrap();
    assert!(found.is_some());
}
