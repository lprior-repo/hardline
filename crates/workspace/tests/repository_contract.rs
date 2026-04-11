//! Contract tests for the WorkspaceRepository trait.
//!
//! These tests verify behavioural invariants that **any** `WorkspaceRepository`
//! implementation must satisfy. They are parameterised over a factory function
//! so that new implementations (file-backed, SQL, etc.) can reuse the suite by
//! calling `run_contract_tests(factory)`.
//!
//! Usage from an implementation's test module:
//!
//! ```ignore
//! use scp_workspace::workspace_repository_contract_tests;
//!
//! fn my_factory() -> impl WorkspaceRepository { MyRepo::new() }
//!
//! #[test]
//! fn my_repo_satisfies_contract() {
//!     workspace_repository_contract_tests::run_contract_tests(my_factory);
//! }
//! ```

use scp_workspace::domain::entities::{Workspace, WorkspaceId, WorkspaceState};
use scp_workspace::domain::value_objects::{WorkspaceName, WorkspacePath};
use scp_workspace::error::WorkspaceError;
use scp_workspace::infrastructure::workspace_repository::WorkspaceRepository;

// ---------------------------------------------------------------------------
// Factory type
// ---------------------------------------------------------------------------

/// A function that produces a fresh, empty `WorkspaceRepository`.
///
/// Every test creates its own repo instance to guarantee isolation.
type RepoFactory = Box<dyn Fn() -> Box<dyn WorkspaceRepository>>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_workspace(name: &str) -> Workspace {
    Workspace::create(
        WorkspaceName::new(name.into()).unwrap(),
        WorkspacePath::new(format!("/tmp/{name}")).unwrap(),
    )
    .unwrap()
}

fn make_active_workspace(name: &str) -> Workspace {
    let ws = make_workspace(name);
    let activated = ws.activate().unwrap();
    Workspace {
        id: activated.id,
        name: activated.name,
        path: activated.path,
        created_at: activated.created_at,
        updated_at: activated.updated_at,
        lock_holder: activated.lock_holder,
        config: activated.config,
        state: WorkspaceState::Active,
        _state: std::marker::PhantomData,
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run every contract test against the repository produced by `factory`.
pub fn run_contract_tests(factory: impl Fn() -> Box<dyn WorkspaceRepository> + 'static) {
    let factory: RepoFactory = Box::new(factory);

    save_then_get_returns_identical_entity(&factory);
    save_then_get_preserves_all_fields(&factory);
    update_then_get_reflects_changes(&factory);
    delete_then_get_returns_none(&factory);
    delete_then_get_by_name_returns_none(&factory);
    list_returns_all_inserted_entities(&factory);
    list_empty_repo_returns_empty(&factory);
    save_assigns_unique_ids(&factory);
    save_overwrite_preserves_count(&factory);
    delete_nonexistent_returns_error(&factory);
    delete_twice_second_fails(&factory);
    delete_does_not_affect_others(&factory);
    get_nonexistent_returns_none(&factory);
    get_by_name_nonexistent_returns_none(&factory);
    get_by_id_and_name_consistent(&factory);
    list_active_excludes_non_active(&factory);
    list_count_decreases_after_delete(&factory);
    list_after_all_deletes_is_empty(&factory);
    get_is_idempotent(&factory);
    get_by_name_is_idempotent(&factory);
    list_returns_independent_snapshot(&factory);
    list_active_empty_repo_returns_empty(&factory);
    list_active_excludes_locked(&factory);
    list_active_excludes_corrupted(&factory);
    list_active_excludes_deleted(&factory);
    get_by_name_is_case_sensitive(&factory);
    delete_nonexistent_error_is_workspace_not_found(&factory);
    trait_is_object_safe(&factory);
    concurrent_saves_dont_corrupt(&factory);
    concurrent_reads_during_write(&factory);
}

// ---------------------------------------------------------------------------
// Contract tests
// ---------------------------------------------------------------------------

fn save_then_get_returns_identical_entity(f: &RepoFactory) {
    let repo = f();
    let ws = make_workspace("roundtrip");
    let saved = repo.save(ws).unwrap();

    let found = repo
        .get(&saved.id)
        .expect("get should not error")
        .expect("workspace should exist after save");

    assert_eq!(found.id.as_str(), saved.id.as_str());
    assert_eq!(found.name.as_str(), "roundtrip");
}

fn save_then_get_preserves_all_fields(f: &RepoFactory) {
    let repo = f();
    let ws = make_active_workspace("preserve-fields");
    let saved = repo.save(ws.clone()).unwrap();

    let found = repo
        .get(&saved.id)
        .expect("get should not error")
        .expect("workspace should exist");

    assert_eq!(found.id.as_str(), ws.id.as_str());
    assert_eq!(found.name.as_str(), ws.name.as_str());
    assert_eq!(found.state, ws.state);
}

fn update_then_get_reflects_changes(f: &RepoFactory) {
    let repo = f();
    let ws = make_workspace("update-test");
    let saved = repo.save(ws).unwrap();

    // Update by saving with the same id but different fields
    let updated = Workspace {
        id: saved.id.clone(),
        name: WorkspaceName::new("update-test".into()).unwrap(),
        path: WorkspacePath::new("/tmp/updated-path".into()).unwrap(),
        created_at: saved.created_at,
        updated_at: chrono::Utc::now(),
        lock_holder: Some("agent-1".into()),
        config: saved.config.clone(),
        state: WorkspaceState::Active,
        _state: std::marker::PhantomData,
    };
    repo.save(updated).unwrap();

    let found = repo
        .get(&saved.id)
        .expect("get should not error")
        .expect("workspace should exist");

    assert_eq!(found.lock_holder(), Some("agent-1"));
    assert_eq!(found.state, WorkspaceState::Active);
}

fn delete_then_get_returns_none(f: &RepoFactory) {
    let repo = f();
    let saved = repo.save(make_workspace("del-get")).unwrap();
    repo.delete(&saved.id).unwrap();

    let found = repo.get(&saved.id).expect("get should not error");
    assert!(found.is_none(), "workspace should not exist after delete");
}

fn delete_then_get_by_name_returns_none(f: &RepoFactory) {
    let repo = f();
    let saved = repo.save(make_workspace("del-name")).unwrap();
    repo.delete(&saved.id).unwrap();

    let found = repo
        .get_by_name("del-name")
        .expect("get_by_name should not error");
    assert!(
        found.is_none(),
        "workspace should not be findable by name after delete"
    );
}

fn list_returns_all_inserted_entities(f: &RepoFactory) {
    let repo = f();
    let count = 5;
    for i in 0..count {
        repo.save(make_workspace(&format!("bulk-{i}"))).unwrap();
    }

    let list = repo.list().expect("list should not error");
    assert_eq!(
        list.len(),
        count,
        "list should return exactly the number of inserted entities"
    );
}

fn list_empty_repo_returns_empty(f: &RepoFactory) {
    let repo = f();
    let list = repo.list().expect("list should not error");
    assert!(list.is_empty(), "empty repo should return empty list");
}

fn save_assigns_unique_ids(f: &RepoFactory) {
    let repo = f();
    let ws1 = repo.save(make_workspace("unique-1")).unwrap();
    let ws2 = repo.save(make_workspace("unique-2")).unwrap();
    assert_ne!(
        ws1.id.as_str(),
        ws2.id.as_str(),
        "each save should produce a distinct id"
    );
}

fn save_overwrite_preserves_count(f: &RepoFactory) {
    let repo = f();
    let saved = repo.save(make_workspace("overwrite")).unwrap();

    let updated = Workspace {
        id: saved.id.clone(),
        name: WorkspaceName::new("overwrite".into()).unwrap(),
        path: WorkspacePath::new("/tmp/overwrite-v2".into()).unwrap(),
        created_at: saved.created_at,
        updated_at: chrono::Utc::now(),
        lock_holder: Some("agent".into()),
        config: saved.config.clone(),
        state: WorkspaceState::Initializing,
        _state: std::marker::PhantomData,
    };
    repo.save(updated).unwrap();

    assert_eq!(
        repo.list().expect("list should not error").len(),
        1,
        "overwriting the same id should not duplicate entries"
    );
}

fn delete_nonexistent_returns_error(f: &RepoFactory) {
    let repo = f();
    let id = WorkspaceId::parse("phantom".into()).unwrap();
    let result = repo.delete(&id);
    assert!(
        result.is_err(),
        "deleting a non-existent workspace must return an error"
    );
}

fn delete_twice_second_fails(f: &RepoFactory) {
    let repo = f();
    let saved = repo.save(make_workspace("double-del")).unwrap();
    repo.delete(&saved.id).expect("first delete should succeed");

    let second = repo.delete(&saved.id);
    assert!(
        second.is_err(),
        "deleting an already-deleted workspace must fail"
    );
}

fn delete_does_not_affect_others(f: &RepoFactory) {
    let repo = f();
    let saved1 = repo.save(make_workspace("keep")).unwrap();
    let saved2 = repo.save(make_workspace("remove")).unwrap();
    repo.delete(&saved2.id).unwrap();

    assert!(
        repo.get(&saved1.id).unwrap().is_some(),
        "deleting one workspace must not affect others"
    );
    assert_eq!(repo.list().unwrap().len(), 1);
}

fn get_nonexistent_returns_none(f: &RepoFactory) {
    let repo = f();
    let id = WorkspaceId::parse("ghost".into()).unwrap();
    let found = repo.get(&id).expect("get should not error");
    assert!(found.is_none());
}

fn get_by_name_nonexistent_returns_none(f: &RepoFactory) {
    let repo = f();
    let found = repo
        .get_by_name("nope")
        .expect("get_by_name should not error");
    assert!(found.is_none());
}

fn get_by_id_and_name_consistent(f: &RepoFactory) {
    let repo = f();
    let saved = repo.save(make_workspace("consistent")).unwrap();

    let by_id = repo.get(&saved.id).unwrap().expect("by id");
    let by_name = repo.get_by_name("consistent").unwrap().expect("by name");

    assert_eq!(
        by_id.id.as_str(),
        by_name.id.as_str(),
        "get by id and get_by_name must return the same entity"
    );
}

fn list_active_excludes_non_active(f: &RepoFactory) {
    let repo = f();

    // Active
    let active = make_active_workspace("active-one");
    let active_id = active.id.clone();
    repo.save(active).unwrap();

    // Initializing
    repo.save(make_workspace("init-one")).unwrap();

    let actives = repo.list_active().expect("list_active should not error");
    assert_eq!(actives.len(), 1);
    assert_eq!(actives[0].id.as_str(), active_id.as_str());
}

fn list_count_decreases_after_delete(f: &RepoFactory) {
    let repo = f();
    let mut ids = Vec::new();
    for i in 0..3 {
        let saved = repo.save(make_workspace(&format!("del-{i}"))).unwrap();
        ids.push(saved.id);
    }
    repo.delete(&ids[1]).unwrap();
    assert_eq!(repo.list().unwrap().len(), 2);
}

fn list_after_all_deletes_is_empty(f: &RepoFactory) {
    let repo = f();
    let mut ids = Vec::new();
    for i in 0..3 {
        let saved = repo.save(make_workspace(&format!("all-del-{i}"))).unwrap();
        ids.push(saved.id);
    }
    for id in &ids {
        repo.delete(id).unwrap();
    }
    assert!(repo.list().unwrap().is_empty());
}

fn get_is_idempotent(f: &RepoFactory) {
    let repo = f();
    let saved = repo.save(make_workspace("idempotent")).unwrap();

    let first = repo.get(&saved.id).unwrap();
    let second = repo.get(&saved.id).unwrap();
    assert_eq!(first.unwrap().id.as_str(), second.unwrap().id.as_str());
}

fn list_returns_independent_snapshot(f: &RepoFactory) {
    let repo = f();
    repo.save(make_workspace("snap")).unwrap();

    let mut list1 = repo.list().unwrap();
    let list2 = repo.list().unwrap();

    list1.clear();
    assert_eq!(
        list2.len(),
        1,
        "mutating a returned list must not affect the repo"
    );
}

fn get_by_name_is_idempotent(f: &RepoFactory) {
    let repo = f();
    repo.save(make_workspace("name-idem")).unwrap();

    let first = repo
        .get_by_name("name-idem")
        .expect("get_by_name should not error");
    let second = repo
        .get_by_name("name-idem")
        .expect("get_by_name should not error");
    assert_eq!(
        first.unwrap().id.as_str(),
        second.unwrap().id.as_str(),
        "repeated get_by_name calls must return the same entity"
    );
}

fn list_active_empty_repo_returns_empty(f: &RepoFactory) {
    let repo = f();
    let actives = repo
        .list_active()
        .expect("list_active on empty repo should not error");
    assert!(
        actives.is_empty(),
        "list_active on empty repo must return empty list"
    );
}

fn list_active_excludes_locked(f: &RepoFactory) {
    let repo = f();
    let base = make_active_workspace("locked-one");
    let locked = Workspace {
        state: WorkspaceState::Locked,
        lock_holder: Some("agent".into()),
        ..base
    };
    repo.save(locked).unwrap();

    let actives = repo.list_active().expect("list_active should not error");
    assert!(
        actives.is_empty(),
        "list_active must exclude Locked workspaces"
    );
}

fn list_active_excludes_corrupted(f: &RepoFactory) {
    let repo = f();
    let base = make_active_workspace("corrupted-one");
    let corrupted = Workspace {
        state: WorkspaceState::Corrupted,
        ..base
    };
    repo.save(corrupted).unwrap();

    let actives = repo.list_active().expect("list_active should not error");
    assert!(
        actives.is_empty(),
        "list_active must exclude Corrupted workspaces"
    );
}

fn list_active_excludes_deleted(f: &RepoFactory) {
    let repo = f();
    let base = make_active_workspace("deleted-one");
    let deleted = Workspace {
        state: WorkspaceState::Deleted,
        ..base
    };
    repo.save(deleted).unwrap();

    let actives = repo.list_active().expect("list_active should not error");
    assert!(
        actives.is_empty(),
        "list_active must exclude Deleted workspaces"
    );
}

fn get_by_name_is_case_sensitive(f: &RepoFactory) {
    let repo = f();
    repo.save(make_workspace("case-test")).unwrap();

    assert!(
        repo.get_by_name("case-test")
            .expect("get_by_name should not error")
            .is_some(),
        "exact case match must find workspace"
    );
    assert!(
        repo.get_by_name("Case-Test")
            .expect("get_by_name should not error")
            .is_none(),
        "different case must not find workspace"
    );
    assert!(
        repo.get_by_name("CASE-TEST")
            .expect("get_by_name should not error")
            .is_none(),
        "all-caps must not find workspace"
    );
}

fn delete_nonexistent_error_is_workspace_not_found(f: &RepoFactory) {
    let repo = f();
    let id = WorkspaceId::parse("phantom".into()).unwrap();
    let err = repo.delete(&id).unwrap_err();
    match err {
        WorkspaceError::WorkspaceNotFound(msg) => {
            assert!(msg.contains("phantom"));
        }
        other => panic!("expected WorkspaceNotFound, got {other:?}"),
    }
}

fn concurrent_saves_dont_corrupt(f: &RepoFactory) {
    use std::sync::Arc;
    use std::thread;

    struct SharedRepo {
        inner: Box<dyn WorkspaceRepository>,
    }
    unsafe impl Send for SharedRepo {}
    unsafe impl Sync for SharedRepo {}
    impl SharedRepo {
        fn new(inner: Box<dyn WorkspaceRepository>) -> Self {
            Self { inner }
        }
    }

    let repo = Arc::new(std::sync::Mutex::new(SharedRepo::new(f())));
    let mut handles = Vec::new();

    for i in 0..10 {
        let r = Arc::clone(&repo);
        handles.push(thread::spawn(move || {
            let ws = make_workspace(&format!("concurrent-{i}"));
            r.lock().unwrap().inner.save(ws).unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let list = repo
        .lock()
        .unwrap()
        .inner
        .list()
        .expect("list should not error");
    assert_eq!(list.len(), 10, "all concurrent saves must be persisted");
}

fn concurrent_reads_during_write(f: &RepoFactory) {
    use std::sync::Arc;
    use std::thread;

    struct SharedRepo {
        inner: Box<dyn WorkspaceRepository>,
    }
    unsafe impl Send for SharedRepo {}
    unsafe impl Sync for SharedRepo {}
    impl SharedRepo {
        fn new(inner: Box<dyn WorkspaceRepository>) -> Self {
            Self { inner }
        }
    }

    let repo = Arc::new(std::sync::Mutex::new(SharedRepo::new(f())));
    let saved = repo
        .lock()
        .unwrap()
        .inner
        .save(make_workspace("concurrent-read"))
        .unwrap();
    let id = saved.id;

    let r1 = Arc::clone(&repo);
    let r2 = Arc::clone(&repo);

    let reader = thread::spawn(move || {
        for _ in 0..100 {
            let found = r1
                .lock()
                .unwrap()
                .inner
                .get(&id)
                .expect("get should not error");
            assert!(found.is_some());
        }
    });

    let writer = thread::spawn(move || {
        for i in 0..100 {
            let ws = make_workspace(&format!("writer-{i}"));
            r2.lock().unwrap().inner.save(ws).unwrap();
        }
    });

    reader.join().unwrap();
    writer.join().unwrap();
}

/// Verify the trait is object-safe (can be used as `dyn WorkspaceRepository`).
fn trait_is_object_safe(_f: &RepoFactory) {
    // This compiles only if WorkspaceRepository is object-safe.
    fn _assert_object_safe(_: &dyn WorkspaceRepository) {}
    // Also verify the factory produces a usable trait object.
    let repo: Box<dyn WorkspaceRepository> = _f();
    let _boxed: Box<dyn WorkspaceRepository> = repo;
}

// ---------------------------------------------------------------------------
// Integration test: run contract against InMemoryWorkspaceRepository
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use scp_workspace::infrastructure::workspace_repository::InMemoryWorkspaceRepository;

    #[test]
    fn in_memory_repo_satisfies_contract() {
        run_contract_tests(|| Box::new(InMemoryWorkspaceRepository::new()));
    }
}
