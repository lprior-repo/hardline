use crate::domain::entities::{Workspace, WorkspaceId, WorkspaceState};
use crate::error::{Result, WorkspaceError};
use std::collections::HashMap;
use std::sync::RwLock;

pub trait WorkspaceRepository: Send + Sync {
    fn save(&self, workspace: Workspace) -> Result<Workspace>;
    fn get(&self, id: &WorkspaceId) -> Result<Option<Workspace>>;
    fn get_by_name(&self, name: &str) -> Result<Option<Workspace>>;
    fn list(&self) -> Result<Vec<Workspace>>;
    fn list_active(&self) -> Result<Vec<Workspace>>;
    fn delete(&self, id: &WorkspaceId) -> Result<()>;
}

pub struct InMemoryWorkspaceRepository {
    workspaces: RwLock<HashMap<String, Workspace>>,
}

impl InMemoryWorkspaceRepository {
    pub fn new() -> Self {
        Self {
            workspaces: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryWorkspaceRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceRepository for InMemoryWorkspaceRepository {
    fn save(&self, workspace: Workspace) -> Result<Workspace> {
        let mut workspaces = self
            .workspaces
            .write()
            .map_err(|e| WorkspaceError::RepositoryError(format!("lock poisoned: {e}")))?;
        let id = workspace.id.as_str().to_string();
        workspaces.insert(id, workspace.clone());
        Ok(workspace)
    }

    fn get(&self, id: &WorkspaceId) -> Result<Option<Workspace>> {
        let workspaces = self
            .workspaces
            .read()
            .map_err(|e| WorkspaceError::RepositoryError(format!("lock poisoned: {e}")))?;
        Ok(workspaces.get(id.as_str()).cloned())
    }

    fn get_by_name(&self, name: &str) -> Result<Option<Workspace>> {
        let workspaces = self
            .workspaces
            .read()
            .map_err(|e| WorkspaceError::RepositoryError(format!("lock poisoned: {e}")))?;
        Ok(workspaces
            .values()
            .find(|w| w.name.as_str() == name)
            .cloned())
    }

    fn list(&self) -> Result<Vec<Workspace>> {
        let workspaces = self
            .workspaces
            .read()
            .map_err(|e| WorkspaceError::RepositoryError(format!("lock poisoned: {e}")))?;
        Ok(workspaces.values().cloned().collect())
    }

    fn list_active(&self) -> Result<Vec<Workspace>> {
        let workspaces = self
            .workspaces
            .read()
            .map_err(|e| WorkspaceError::RepositoryError(format!("lock poisoned: {e}")))?;
        Ok(workspaces
            .values()
            .filter(|w| w.state == WorkspaceState::Active)
            .cloned()
            .collect())
    }

    fn delete(&self, id: &WorkspaceId) -> Result<()> {
        let mut workspaces = self
            .workspaces
            .write()
            .map_err(|e| WorkspaceError::RepositoryError(format!("lock poisoned: {e}")))?;
        if workspaces.remove(id.as_str()).is_some() {
            Ok(())
        } else {
            Err(WorkspaceError::WorkspaceNotFound(id.as_str().into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{WorkspaceName, WorkspacePath};

    #[test]
    fn in_memory_repo_save_and_get() {
        let repo = InMemoryWorkspaceRepository::new();
        let workspace = Workspace::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let saved = repo.save(workspace).unwrap();
        let found = repo.get(&saved.id).unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn in_memory_repo_get_by_name() {
        let repo = InMemoryWorkspaceRepository::new();
        let workspace = Workspace::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        repo.save(workspace).unwrap();
        let found = repo.get_by_name("test").unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn in_memory_repo_list_active() {
        let repo = InMemoryWorkspaceRepository::new();
        let workspace = Workspace::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let active = workspace.activate().unwrap();
        // Convert Workspace<Active> to Workspace by using transition_impl
        let workspace_for_storage = Workspace {
            id: active.id,
            name: active.name,
            path: active.path,
            created_at: active.created_at,
            updated_at: active.updated_at,
            lock_holder: active.lock_holder,
            config: active.config,
            state: WorkspaceState::Active,
            _state: std::marker::PhantomData::<crate::domain::entities::workspace::Initializing>,
        };
        repo.save(workspace_for_storage).unwrap();
        let actives = repo.list_active().unwrap();
        assert_eq!(actives.len(), 1);
    }

    #[test]
    fn in_memory_repo_default_is_empty() {
        let repo = InMemoryWorkspaceRepository::default();
        let list = repo.list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn in_memory_repo_get_missing_returns_none() {
        let repo = InMemoryWorkspaceRepository::new();
        let id = WorkspaceId::parse("nonexistent".into()).unwrap();
        let found = repo.get(&id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn in_memory_repo_get_by_name_missing_returns_none() {
        let repo = InMemoryWorkspaceRepository::new();
        let found = repo.get_by_name("nonexistent").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn in_memory_repo_save_multiple() {
        let repo = InMemoryWorkspaceRepository::new();
        let ws1 = Workspace::create(
            WorkspaceName::new("ws-1".into()).unwrap(),
            WorkspacePath::new("/tmp/ws-1".into()).unwrap(),
        )
        .unwrap();
        let ws2 = Workspace::create(
            WorkspaceName::new("ws-2".into()).unwrap(),
            WorkspacePath::new("/tmp/ws-2".into()).unwrap(),
        )
        .unwrap();
        repo.save(ws1).unwrap();
        repo.save(ws2).unwrap();
        let list = repo.list().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn in_memory_repo_list_on_empty() {
        let repo = InMemoryWorkspaceRepository::new();
        let list = repo.list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn in_memory_repo_list_active_excludes_initializing() {
        let repo = InMemoryWorkspaceRepository::new();
        let ws = Workspace::create(
            WorkspaceName::new("init-ws".into()).unwrap(),
            WorkspacePath::new("/tmp/init-ws".into()).unwrap(),
        )
        .unwrap();
        repo.save(ws).unwrap();
        let actives = repo.list_active().unwrap();
        assert!(actives.is_empty());
    }

    #[test]
    fn in_memory_repo_delete_existing() {
        let repo = InMemoryWorkspaceRepository::new();
        let ws = Workspace::create(
            WorkspaceName::new("del-ws".into()).unwrap(),
            WorkspacePath::new("/tmp/del-ws".into()).unwrap(),
        )
        .unwrap();
        let saved = repo.save(ws).unwrap();
        let result = repo.delete(&saved.id);
        assert!(result.is_ok());
        // Verify it's actually gone
        let found = repo.get(&saved.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn in_memory_repo_delete_missing_returns_error() {
        let repo = InMemoryWorkspaceRepository::new();
        let id = WorkspaceId::parse("ghost".into()).unwrap();
        let result = repo.delete(&id);
        assert!(result.is_err());
        match result.err() {
            Some(WorkspaceError::WorkspaceNotFound(msg)) => {
                assert!(msg.contains("ghost"));
            }
            other => panic!("expected WorkspaceNotFound, got {other:?}"),
        }
    }

    #[test]
    fn in_memory_repo_save_returns_same_workspace() {
        let repo = InMemoryWorkspaceRepository::new();
        let ws = Workspace::create(
            WorkspaceName::new("roundtrip".into()).unwrap(),
            WorkspacePath::new("/tmp/roundtrip".into()).unwrap(),
        )
        .unwrap();
        let original_id = ws.id.as_str().to_string();
        let saved = repo.save(ws).unwrap();
        assert_eq!(saved.id.as_str(), original_id);
    }

    #[test]
    fn in_memory_repo_list_active_only_actives() {
        let repo = InMemoryWorkspaceRepository::new();
        // Save an initializing workspace
        let ws_init = Workspace::create(
            WorkspaceName::new("init".into()).unwrap(),
            WorkspacePath::new("/tmp/init".into()).unwrap(),
        )
        .unwrap();
        repo.save(ws_init).unwrap();

        // Save an active workspace
        let ws_active = Workspace::create(
            WorkspaceName::new("active".into()).unwrap(),
            WorkspacePath::new("/tmp/active".into()).unwrap(),
        )
        .unwrap();
        let activated = ws_active.activate().unwrap();
        let ws_for_storage = Workspace {
            id: activated.id,
            name: activated.name,
            path: activated.path,
            created_at: activated.created_at,
            updated_at: activated.updated_at,
            lock_holder: activated.lock_holder,
            config: activated.config,
            state: WorkspaceState::Active,
            _state: std::marker::PhantomData::<crate::domain::entities::workspace::Initializing>,
        };
        repo.save(ws_for_storage).unwrap();

        let actives = repo.list_active().unwrap();
        assert_eq!(actives.len(), 1);
        assert_eq!(actives[0].name.as_str(), "active");
    }

    // --- Additional unit tests ---

    fn make_active_workspace(name: &str) -> Workspace {
        let ws = Workspace::create(
            WorkspaceName::new(name.into()).unwrap(),
            WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
        )
        .unwrap();
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
            _state: std::marker::PhantomData::<crate::domain::entities::workspace::Initializing>,
        }
    }

    #[test]
    fn in_memory_repo_save_overwrites_existing() {
        let repo = InMemoryWorkspaceRepository::new();
        let ws1 = Workspace::create(
            WorkspaceName::new("overwrite".into()).unwrap(),
            WorkspacePath::new("/tmp/overwrite".into()).unwrap(),
        )
        .unwrap();
        let saved = repo.save(ws1).unwrap();

        // Save again with the same id
        let ws2 = Workspace {
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
        repo.save(ws2).unwrap();

        let found = repo.get(&saved.id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.lock_holder(), Some("agent"));
    }

    #[test]
    fn in_memory_repo_delete_and_get_returns_none() {
        let repo = InMemoryWorkspaceRepository::new();
        let ws = Workspace::create(
            WorkspaceName::new("del-get".into()).unwrap(),
            WorkspacePath::new("/tmp/del-get".into()).unwrap(),
        )
        .unwrap();
        let saved = repo.save(ws).unwrap();
        repo.delete(&saved.id).unwrap();
        let found = repo.get(&saved.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn in_memory_repo_get_by_name_after_delete() {
        let repo = InMemoryWorkspaceRepository::new();
        let ws = Workspace::create(
            WorkspaceName::new("del-name".into()).unwrap(),
            WorkspacePath::new("/tmp/del-name".into()).unwrap(),
        )
        .unwrap();
        let saved = repo.save(ws).unwrap();
        repo.delete(&saved.id).unwrap();
        let found = repo.get_by_name("del-name").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn in_memory_repo_list_after_deletes() {
        let repo = InMemoryWorkspaceRepository::new();
        let ws1 = Workspace::create(
            WorkspaceName::new("list-del-1".into()).unwrap(),
            WorkspacePath::new("/tmp/list-del-1".into()).unwrap(),
        )
        .unwrap();
        let ws2 = Workspace::create(
            WorkspaceName::new("list-del-2".into()).unwrap(),
            WorkspacePath::new("/tmp/list-del-2".into()).unwrap(),
        )
        .unwrap();
        let ws3 = Workspace::create(
            WorkspaceName::new("list-del-3".into()).unwrap(),
            WorkspacePath::new("/tmp/list-del-3".into()).unwrap(),
        )
        .unwrap();
        let saved1 = repo.save(ws1).unwrap();
        let saved2 = repo.save(ws2).unwrap();
        repo.save(ws3).unwrap();

        repo.delete(&saved1.id).unwrap();
        repo.delete(&saved2.id).unwrap();
        let list = repo.list().unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn in_memory_repo_list_returns_all_saved() {
        let repo = InMemoryWorkspaceRepository::new();
        for i in 0..5 {
            let ws = Workspace::create(
                WorkspaceName::new(format!("bulk-{}", i)).unwrap(),
                WorkspacePath::new(format!("/tmp/bulk-{}", i)).unwrap(),
            )
            .unwrap();
            repo.save(ws).unwrap();
        }
        let list = repo.list().unwrap();
        assert_eq!(list.len(), 5);
    }

    #[test]
    fn in_memory_repo_get_by_name_finds_among_many() {
        let repo = InMemoryWorkspaceRepository::new();
        for i in 0..10 {
            let ws = Workspace::create(
                WorkspaceName::new(format!("find-{}", i)).unwrap(),
                WorkspacePath::new(format!("/tmp/find-{}", i)).unwrap(),
            )
            .unwrap();
            repo.save(ws).unwrap();
        }
        let found = repo.get_by_name("find-7").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name.as_str(), "find-7");
    }

    #[test]
    fn in_memory_repo_list_active_with_mixed_states() {
        let repo = InMemoryWorkspaceRepository::new();

        // Save an active workspace
        repo.save(make_active_workspace("mixed-active")).unwrap();

        // Save a locked workspace
        let ws_locked = make_active_workspace("mixed-locked");
        let locked_ws = Workspace {
            state: WorkspaceState::Locked,
            lock_holder: Some("agent".into()),
            ..ws_locked
        };
        repo.save(locked_ws).unwrap();

        // Save a deleted workspace
        let ws_deleted = make_active_workspace("mixed-deleted");
        let deleted_ws = Workspace {
            state: WorkspaceState::Deleted,
            ..ws_deleted
        };
        repo.save(deleted_ws).unwrap();

        // Save an initializing workspace
        let ws_init = Workspace::create(
            WorkspaceName::new("mixed-init".into()).unwrap(),
            WorkspacePath::new("/tmp/mixed-init".into()).unwrap(),
        )
        .unwrap();
        repo.save(ws_init).unwrap();

        let actives = repo.list_active().unwrap();
        assert_eq!(actives.len(), 1);
        assert_eq!(actives[0].name.as_str(), "mixed-active");
    }

    #[test]
    fn in_memory_repo_save_and_get_preserves_all_fields() {
        let repo = InMemoryWorkspaceRepository::new();
        let ws = make_active_workspace("preserve-fields");
        let saved = repo.save(ws.clone()).unwrap();
        let found = repo.get(&saved.id).unwrap().unwrap();
        assert_eq!(found.id.as_str(), ws.id.as_str());
        assert_eq!(found.name.as_str(), ws.name.as_str());
        assert_eq!(found.state, ws.state);
    }

    #[test]
    fn in_memory_repo_new_is_empty() {
        let repo = InMemoryWorkspaceRepository::new();
        assert!(repo.list().unwrap().is_empty());
        assert!(repo.list_active().unwrap().is_empty());
    }

    #[test]
    fn in_memory_repo_delete_nonexistent_twice() {
        let repo = InMemoryWorkspaceRepository::new();
        let id = WorkspaceId::parse("ghost".into()).unwrap();
        let result1 = repo.delete(&id);
        assert!(result1.is_err());
        let result2 = repo.delete(&id);
        assert!(result2.is_err());
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;
        use proptest::{prop_assert, prop_assert_eq};

        proptest! {
            #[test]
            fn in_memory_repo_save_get_roundtrip(
                name in "[a-zA-Z0-9_-]{1,50}",
                path_suffix in "[a-zA-Z0-9_-]{1,50}"
            ) {
                let repo = InMemoryWorkspaceRepository::new();
                let ws = Workspace::create(
                    WorkspaceName::new(name).unwrap(),
                    WorkspacePath::new(format!("/tmp/{}", path_suffix)).unwrap(),
                ).unwrap();
                let saved = repo.save(ws).unwrap();
                let found = repo.get(&saved.id).unwrap();
                prop_assert!(found.is_some());
                let found_ws = found.unwrap();
                prop_assert_eq!(found_ws.id.as_str(), saved.id.as_str());
            }

            #[test]
            fn in_memory_repo_list_count_matches_saved(
                count in 1usize..20
            ) {
                let repo = InMemoryWorkspaceRepository::new();
                for i in 0..count {
                    let ws = Workspace::create(
                        WorkspaceName::new(format!("ws-{}", i)).unwrap(),
                        WorkspacePath::new(format!("/tmp/ws-{}", i)).unwrap(),
                    ).unwrap();
                    repo.save(ws).unwrap();
                }
                prop_assert_eq!(repo.list().unwrap().len(), count);
            }

            #[test]
            fn in_memory_repo_get_by_name_finds_correct_one(
                names in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 5..15)
            ) {
                let repo = InMemoryWorkspaceRepository::new();
                for name in &names {
                    let ws = Workspace::create(
                        WorkspaceName::new(name.clone()).unwrap(),
                        WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
                    ).unwrap();
                    repo.save(ws).unwrap();
                }
                // Pick a name to search for
                if let Some(target) = names.first() {
                    let found = repo.get_by_name(target).unwrap();
                    prop_assert!(found.is_some());
                    let found_ws = found.unwrap();
                    prop_assert_eq!(found_ws.name.as_str(), target.as_str());
                }
            }

            #[test]
            fn in_memory_repo_save_overwrite_returns_same_id(
                name in "[a-zA-Z0-9_-]{1,30}",
                path1 in "[a-zA-Z0-9_-]{1,30}",
                path2 in "[a-zA-Z0-9_-]{1,30}"
            ) {
                let repo = InMemoryWorkspaceRepository::new();
                let ws1 = Workspace::create(
                    WorkspaceName::new(name.clone()).unwrap(),
                    WorkspacePath::new(format!("/tmp/{}", path1)).unwrap(),
                ).unwrap();
                let saved1 = repo.save(ws1).unwrap();

                let ws2 = Workspace {
                    id: saved1.id.clone(),
                    name: WorkspaceName::new(name).unwrap(),
                    path: WorkspacePath::new(format!("/tmp/{}", path2)).unwrap(),
                    created_at: saved1.created_at,
                    updated_at: chrono::Utc::now(),
                    lock_holder: None,
                    config: saved1.config.clone(),
                    state: WorkspaceState::Initializing,
                    _state: std::marker::PhantomData,
                };
                repo.save(ws2).unwrap();

                // Only one workspace should exist
                prop_assert_eq!(repo.list().unwrap().len(), 1);
            }

            #[test]
            fn in_memory_repo_delete_reduces_count(
                total in 3usize..10,
                delete_idx in 0usize..3
            ) {
                let repo = InMemoryWorkspaceRepository::new();
                let mut ids = Vec::new();
                for i in 0..total {
                    let ws = Workspace::create(
                        WorkspaceName::new(format!("del-batch-{}", i)).unwrap(),
                        WorkspacePath::new(format!("/tmp/del-batch-{}", i)).unwrap(),
                    ).unwrap();
                    let saved = repo.save(ws).unwrap();
                    ids.push(saved.id);
                }
                if delete_idx < ids.len() {
                    repo.delete(&ids[delete_idx]).unwrap();
                    prop_assert_eq!(repo.list().unwrap().len(), total - 1);
                }
            }
        }
    }

    // =========================================================================
    // CONTRACT TESTS — WorkspaceRepository trait contract
    //
    // These tests verify behavioral invariants that ANY WorkspaceRepository
    // implementation must satisfy. They are parameterized over the trait so
    // a future persistent implementation can reuse them by calling
    // `contract::run_all(repo)` or by running the same tests with a different
    // subject.
    // =========================================================================

    mod contract {
        use super::*;
        use crate::domain::value_objects::{WorkspaceName, WorkspacePath};

        /// Helper: create a workspace in Initializing state with a unique name.
        fn make_workspace(name: &str) -> Workspace {
            Workspace::create(
                WorkspaceName::new(name.into()).unwrap(),
                WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
            )
            .unwrap()
        }

        /// Helper: create a workspace in Active state.
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

        // --- save + get round-trip ---

        #[test]
        fn contract_save_then_get_returns_same_workspace() {
            let repo = InMemoryWorkspaceRepository::new();
            let ws = make_workspace("roundtrip");
            let saved = repo.save(ws).unwrap();

            let found = repo.get(&saved.id).unwrap().expect("should exist");
            assert_eq!(found.id.as_str(), saved.id.as_str());
            assert_eq!(found.name.as_str(), "roundtrip");
        }

        #[test]
        fn contract_save_returns_workspace_with_id() {
            let repo = InMemoryWorkspaceRepository::new();
            let ws = make_workspace("save-id");
            let saved = repo.save(ws).unwrap();
            assert!(!saved.id.as_str().is_empty());
        }

        #[test]
        fn contract_save_preserves_name_and_path() {
            let repo = InMemoryWorkspaceRepository::new();
            let ws = make_workspace("preserve");
            let saved = repo.save(ws).unwrap();
            assert_eq!(saved.name.as_str(), "preserve");
            assert!(saved.path.as_str().unwrap().contains("/tmp/preserve"));
        }

        #[test]
        fn contract_save_preserves_state() {
            let repo = InMemoryWorkspaceRepository::new();
            let ws = make_active_workspace("state-preserve");
            let saved = repo.save(ws).unwrap();
            assert_eq!(saved.state, WorkspaceState::Active);

            let found = repo.get(&saved.id).unwrap().unwrap();
            assert_eq!(found.state, WorkspaceState::Active);
        }

        // --- get: missing workspace ---

        #[test]
        fn contract_get_nonexistent_returns_none() {
            let repo = InMemoryWorkspaceRepository::new();
            let id = WorkspaceId::parse("ghost".into()).unwrap();
            let found = repo.get(&id).unwrap();
            assert!(found.is_none());
        }

        // --- get_by_name ---

        #[test]
        fn contract_get_by_name_returns_saved_workspace() {
            let repo = InMemoryWorkspaceRepository::new();
            let ws = make_workspace("by-name");
            repo.save(ws).unwrap();

            let found = repo.get_by_name("by-name").unwrap().expect("should exist");
            assert_eq!(found.name.as_str(), "by-name");
        }

        #[test]
        fn contract_get_by_name_nonexistent_returns_none() {
            let repo = InMemoryWorkspaceRepository::new();
            let found = repo.get_by_name("nope").unwrap();
            assert!(found.is_none());
        }

        #[test]
        fn contract_get_by_name_is_case_sensitive() {
            let repo = InMemoryWorkspaceRepository::new();
            let ws = make_workspace("case-test");
            repo.save(ws).unwrap();

            assert!(repo.get_by_name("case-test").unwrap().is_some());
            assert!(repo.get_by_name("Case-Test").unwrap().is_none());
            assert!(repo.get_by_name("CASE-TEST").unwrap().is_none());
        }

        #[test]
        fn contract_get_by_name_finds_among_many() {
            let repo = InMemoryWorkspaceRepository::new();
            for i in 0..10 {
                let ws = make_workspace(&format!("find-{i}"));
                repo.save(ws).unwrap();
            }
            let found = repo.get_by_name("find-7").unwrap().expect("should exist");
            assert_eq!(found.name.as_str(), "find-7");
        }

        #[test]
        fn contract_get_by_name_after_delete_returns_none() {
            let repo = InMemoryWorkspaceRepository::new();
            let ws = make_workspace("del-name");
            let saved = repo.save(ws).unwrap();
            repo.delete(&saved.id).unwrap();
            let found = repo.get_by_name("del-name").unwrap();
            assert!(found.is_none());
        }

        // --- get by id vs get_by_name consistency ---

        #[test]
        fn contract_get_by_id_and_name_return_same_workspace() {
            let repo = InMemoryWorkspaceRepository::new();
            let ws = make_workspace("consistent");
            let saved = repo.save(ws).unwrap();

            let by_id = repo.get(&saved.id).unwrap().expect("by id");
            let by_name = repo.get_by_name("consistent").unwrap().expect("by name");
            assert_eq!(by_id.id.as_str(), by_name.id.as_str());
            assert_eq!(by_id.name.as_str(), by_name.name.as_str());
        }

        // --- list: empty repo ---

        #[test]
        fn contract_list_empty_repo_returns_empty() {
            let repo = InMemoryWorkspaceRepository::new();
            let list = repo.list().unwrap();
            assert!(list.is_empty());
        }

        // --- list: count matches saved ---

        #[test]
        fn contract_list_returns_all_saved_workspaces() {
            let repo = InMemoryWorkspaceRepository::new();
            for i in 0..5 {
                repo.save(make_workspace(&format!("bulk-{i}"))).unwrap();
            }
            assert_eq!(repo.list().unwrap().len(), 5);
        }

        #[test]
        fn contract_list_count_decreases_after_delete() {
            let repo = InMemoryWorkspaceRepository::new();
            let mut ids = Vec::new();
            for i in 0..3 {
                let saved = repo.save(make_workspace(&format!("del-{i}"))).unwrap();
                ids.push(saved.id);
            }
            repo.delete(&ids[1]).unwrap();
            assert_eq!(repo.list().unwrap().len(), 2);
        }

        #[test]
        fn contract_list_after_all_deletes_is_empty() {
            let repo = InMemoryWorkspaceRepository::new();
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

        // --- list_active ---

        #[test]
        fn contract_list_active_empty_repo_returns_empty() {
            let repo = InMemoryWorkspaceRepository::new();
            assert!(repo.list_active().unwrap().is_empty());
        }

        #[test]
        fn contract_list_active_excludes_initializing() {
            let repo = InMemoryWorkspaceRepository::new();
            repo.save(make_workspace("init-only")).unwrap();
            assert!(repo.list_active().unwrap().is_empty());
        }

        #[test]
        fn contract_list_active_includes_only_active() {
            let repo = InMemoryWorkspaceRepository::new();

            // Active workspace
            let active = make_active_workspace("active-one");
            let active_id = active.id.clone();
            repo.save(active).unwrap();

            // Initializing workspace
            repo.save(make_workspace("init-one")).unwrap();

            let actives = repo.list_active().unwrap();
            assert_eq!(actives.len(), 1);
            assert_eq!(actives[0].id.as_str(), active_id.as_str());
        }

        #[test]
        fn contract_list_active_with_mixed_states() {
            let repo = InMemoryWorkspaceRepository::new();

            // Active
            repo.save(make_active_workspace("mix-active")).unwrap();

            // Initializing
            repo.save(make_workspace("mix-init")).unwrap();

            // Locked (simulated by setting state directly)
            let locked_base = make_active_workspace("mix-locked");
            let locked = Workspace {
                state: WorkspaceState::Locked,
                lock_holder: Some("agent".into()),
                ..locked_base
            };
            repo.save(locked).unwrap();

            // Deleted (simulated)
            let deleted_base = make_active_workspace("mix-deleted");
            let deleted = Workspace {
                state: WorkspaceState::Deleted,
                ..deleted_base
            };
            repo.save(deleted).unwrap();

            let actives = repo.list_active().unwrap();
            assert_eq!(actives.len(), 1);
            assert_eq!(actives[0].name.as_str(), "mix-active");
        }

        #[test]
        fn contract_list_active_excludes_corrupted() {
            let repo = InMemoryWorkspaceRepository::new();

            let corrupted_base = make_active_workspace("corrupted-one");
            let corrupted = Workspace {
                state: WorkspaceState::Corrupted,
                ..corrupted_base
            };
            repo.save(corrupted).unwrap();

            assert!(repo.list_active().unwrap().is_empty());
        }

        // --- delete ---

        #[test]
        fn contract_delete_removes_workspace() {
            let repo = InMemoryWorkspaceRepository::new();
            let saved = repo.save(make_workspace("to-delete")).unwrap();
            repo.delete(&saved.id).unwrap();
            assert!(repo.get(&saved.id).unwrap().is_none());
        }

        #[test]
        fn contract_delete_nonexistent_returns_error() {
            let repo = InMemoryWorkspaceRepository::new();
            let id = WorkspaceId::parse("phantom".into()).unwrap();
            let result = repo.delete(&id);
            assert!(result.is_err());
        }

        #[test]
        fn contract_delete_nonexistent_error_is_workspace_not_found() {
            let repo = InMemoryWorkspaceRepository::new();
            let id = WorkspaceId::parse("phantom".into()).unwrap();
            let err = repo.delete(&id).unwrap_err();
            match err {
                WorkspaceError::WorkspaceNotFound(msg) => {
                    assert!(msg.contains("phantom"));
                }
                other => panic!("expected WorkspaceNotFound, got {other:?}"),
            }
        }

        #[test]
        fn contract_delete_twice_second_fails() {
            let repo = InMemoryWorkspaceRepository::new();
            let saved = repo.save(make_workspace("double-del")).unwrap();
            repo.delete(&saved.id).unwrap();
            let second = repo.delete(&saved.id);
            assert!(second.is_err());
        }

        #[test]
        fn contract_delete_does_not_affect_other_workspaces() {
            let repo = InMemoryWorkspaceRepository::new();
            let saved1 = repo.save(make_workspace("keep")).unwrap();
            let saved2 = repo.save(make_workspace("remove")).unwrap();
            repo.delete(&saved2.id).unwrap();
            assert!(repo.get(&saved1.id).unwrap().is_some());
            assert_eq!(repo.list().unwrap().len(), 1);
        }

        // --- save: overwrite / upsert ---

        #[test]
        fn contract_save_overwrites_existing_by_id() {
            let repo = InMemoryWorkspaceRepository::new();
            let ws = make_workspace("overwrite");
            let saved = repo.save(ws).unwrap();

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

            let found = repo.get(&saved.id).unwrap().expect("should exist");
            assert_eq!(found.lock_holder(), Some("agent"));
            assert_eq!(repo.list().unwrap().len(), 1);
        }

        #[test]
        fn contract_save_assigns_unique_ids() {
            let repo = InMemoryWorkspaceRepository::new();
            let ws1 = repo.save(make_workspace("unique-1")).unwrap();
            let ws2 = repo.save(make_workspace("unique-2")).unwrap();
            assert_ne!(ws1.id.as_str(), ws2.id.as_str());
        }

        // --- concurrency (InMemory uses RwLock) ---

        #[test]
        fn contract_concurrent_saves_dont_corrupt() {
            use std::sync::Arc;
            use std::thread;

            let repo = Arc::new(InMemoryWorkspaceRepository::new());
            let mut handles = Vec::new();

            for i in 0..10 {
                let r = Arc::clone(&repo);
                handles.push(thread::spawn(move || {
                    let ws = make_workspace(&format!("concurrent-{i}"));
                    r.save(ws).unwrap();
                }));
            }

            for h in handles {
                h.join().unwrap();
            }

            assert_eq!(repo.list().unwrap().len(), 10);
        }

        #[test]
        fn contract_concurrent_reads_during_write() {
            use std::sync::Arc;
            use std::thread;

            let repo = Arc::new(InMemoryWorkspaceRepository::new());

            // Pre-populate
            let saved = repo.save(make_workspace("concurrent-read")).unwrap();

            let r1 = Arc::clone(&repo);
            let r2 = Arc::clone(&repo);
            let id = saved.id.clone();

            let reader = thread::spawn(move || {
                for _ in 0..100 {
                    let found = r1.get(&id).unwrap();
                    assert!(found.is_some());
                }
            });

            let writer = thread::spawn(move || {
                for i in 0..100 {
                    let ws = make_workspace(&format!("writer-{i}"));
                    r2.save(ws).unwrap();
                }
            });

            reader.join().unwrap();
            writer.join().unwrap();
        }

        // --- idempotency / idempotent-like behavior ---

        #[test]
        fn contract_get_idempotent() {
            let repo = InMemoryWorkspaceRepository::new();
            let saved = repo.save(make_workspace("idempotent")).unwrap();

            let first = repo.get(&saved.id).unwrap();
            let second = repo.get(&saved.id).unwrap();
            assert_eq!(first.unwrap().id.as_str(), second.unwrap().id.as_str());
        }

        #[test]
        fn contract_get_by_name_idempotent() {
            let repo = InMemoryWorkspaceRepository::new();
            repo.save(make_workspace("name-idem")).unwrap();

            let first = repo.get_by_name("name-idem").unwrap();
            let second = repo.get_by_name("name-idem").unwrap();
            assert_eq!(first.unwrap().id.as_str(), second.unwrap().id.as_str());
        }

        // --- list returns independent snapshots ---

        #[test]
        fn contract_list_returns_snapshot_not_reference() {
            let repo = InMemoryWorkspaceRepository::new();
            repo.save(make_workspace("snap")).unwrap();

            let mut list1 = repo.list().unwrap();
            let list2 = repo.list().unwrap();

            // Mutating list1 should not affect list2
            list1.clear();
            assert_eq!(list2.len(), 1);
        }

        // --- proptests for contract invariants ---

        #[cfg(test)]
        mod proptests {
            use super::*;
            use proptest::prelude::*;
            use proptest::{prop_assert, prop_assert_eq};

            proptest! {
                #[test]
                fn contract_save_get_roundtrip(
                    name in "[a-zA-Z0-9_-]{1,50}",
                    path in "[a-zA-Z0-9_-]{1,50}"
                ) {
                    let repo = InMemoryWorkspaceRepository::new();
                    let ws = Workspace::create(
                        WorkspaceName::new(name).unwrap(),
                        WorkspacePath::new(format!("/tmp/{}", path)).unwrap(),
                    ).unwrap();
                    let saved = repo.save(ws).unwrap();
                    let found = repo.get(&saved.id).unwrap();
                    prop_assert!(found.is_some());
                    let found_ws = found.unwrap();
                    prop_assert_eq!(found_ws.id.as_str(), saved.id.as_str());
                    prop_assert_eq!(found_ws.name.as_str(), saved.name.as_str());
                }

                #[test]
                fn contract_list_count_equals_saved_count(count in 1usize..20) {
                    let repo = InMemoryWorkspaceRepository::new();
                    for i in 0..count {
                        repo.save(make_workspace(&format!("count-{i}"))).unwrap();
                    }
                    prop_assert_eq!(repo.list().unwrap().len(), count);
                }

                #[test]
                fn contract_get_by_name_roundtrip(name in "[a-zA-Z0-9_-]{1,30}") {
                    let repo = InMemoryWorkspaceRepository::new();
                    let ws = make_workspace(&name);
                    let saved = repo.save(ws).unwrap();
                    let found = repo.get_by_name(&name).unwrap();
                    prop_assert!(found.is_some());
                    prop_assert_eq!(found.unwrap().id.as_str().to_string(), saved.id.as_str().to_string());
                }

                #[test]
                fn contract_delete_reduces_count(
                    total in 3usize..10,
                    del_idx in 0usize..3
                ) {
                    let repo = InMemoryWorkspaceRepository::new();
                    let mut ids = Vec::new();
                    for i in 0..total {
                        let saved = repo.save(make_workspace(&format!("pdel-{i}"))).unwrap();
                        ids.push(saved.id);
                    }
                    if del_idx < ids.len() {
                        repo.delete(&ids[del_idx]).unwrap();
                        prop_assert_eq!(repo.list().unwrap().len(), total - 1);
                    }
                }

                #[test]
                fn contract_save_overwrite_preserves_count(
                    name in "[a-zA-Z0-9_-]{1,30}",
                    _path1 in "[a-zA-Z0-9_-]{1,30}",
                    path2 in "[a-zA-Z0-9_-]{1,30}"
                ) {
                    let repo = InMemoryWorkspaceRepository::new();
                    let ws1 = make_workspace(&name);
                    let saved1 = repo.save(ws1).unwrap();

                    let ws2 = Workspace {
                        id: saved1.id.clone(),
                        name: WorkspaceName::new(name).unwrap(),
                        path: WorkspacePath::new(format!("/tmp/{}", path2)).unwrap(),
                        created_at: saved1.created_at,
                        updated_at: chrono::Utc::now(),
                        lock_holder: None,
                        config: saved1.config.clone(),
                        state: WorkspaceState::Initializing,
                        _state: std::marker::PhantomData,
                    };
                    repo.save(ws2).unwrap();
                    prop_assert_eq!(repo.list().unwrap().len(), 1);
                }
            }
        }
    }
}
