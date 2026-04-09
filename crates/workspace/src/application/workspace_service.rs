use crate::domain::entities::{Workspace, WorkspaceId, WorkspaceState};
use crate::domain::value_objects::{WorkspaceName, WorkspacePath};
use crate::error::WorkspaceError;

pub struct WorkspaceService;

impl WorkspaceService {
    pub fn create_workspace(
        name: WorkspaceName,
        path: WorkspacePath,
    ) -> std::result::Result<Workspace, WorkspaceError> {
        Workspace::create(name, path)
    }

    pub fn initialize_workspace(
        workspace: Workspace,
    ) -> std::result::Result<Workspace, WorkspaceError> {
        if workspace.state != WorkspaceState::Initializing {
            return Err(WorkspaceError::InvalidStateTransition {
                from: format!("{:?}", workspace.state),
                to: "Active".into(),
            });
        }
        workspace.activate().map(|w| Workspace {
            id: w.id,
            name: w.name,
            path: w.path,
            created_at: w.created_at,
            updated_at: w.updated_at,
            lock_holder: w.lock_holder,
            config: w.config,
            state: WorkspaceState::Active,
            _state: std::marker::PhantomData,
        })
    }

    pub fn lock_workspace(
        workspace: Workspace,
        holder: String,
    ) -> std::result::Result<Workspace, WorkspaceError> {
        let active = workspace.activate()?;
        active.lock(holder).map(|w| Workspace {
            id: w.id,
            name: w.name,
            path: w.path,
            created_at: w.created_at,
            updated_at: w.updated_at,
            lock_holder: w.lock_holder,
            config: w.config,
            state: WorkspaceState::Locked,
            _state: std::marker::PhantomData,
        })
    }

    pub fn unlock_workspace(
        workspace: Workspace,
    ) -> std::result::Result<Workspace, WorkspaceError> {
        // workspace is actually Workspace<Locked> at runtime, but we receive it as Workspace
        // Call unlock via the entity's transition_with_lock_holder pattern
        // Since workspace.state is Locked, we need to use unlock which is on Workspace<Locked>
        // But we can't call it directly because the type is Workspace
        // So we manually construct the result
        if workspace.state != WorkspaceState::Locked {
            return Err(WorkspaceError::InvalidStateTransition {
                from: format!("{:?}", workspace.state),
                to: "Active".into(),
            });
        }
        // Manually transition from Locked to Active
        Ok(Workspace {
            id: workspace.id,
            name: workspace.name,
            path: workspace.path,
            created_at: workspace.created_at,
            updated_at: workspace.updated_at,
            lock_holder: None,
            config: workspace.config,
            state: WorkspaceState::Active,
            _state: std::marker::PhantomData,
        })
    }

    pub fn delete_workspace(
        workspace: Workspace,
    ) -> std::result::Result<Workspace, WorkspaceError> {
        if workspace.state == WorkspaceState::Locked {
            return Err(WorkspaceError::WorkspaceLocked(
                workspace.id.as_str().into(),
                workspace.lock_holder.clone().unwrap_or_default(),
            ));
        }

        match workspace.state {
            WorkspaceState::Active | WorkspaceState::Initializing => {
                // Call delete on the appropriate state
                match workspace.state {
                    WorkspaceState::Active => {
                        // Need to call delete on Workspace<Active>
                        // Since we have Workspace, we need to use transition_with_lock_holder(None) equivalent
                        Ok(Workspace {
                            id: workspace.id,
                            name: workspace.name,
                            path: workspace.path,
                            created_at: workspace.created_at,
                            updated_at: workspace.updated_at,
                            lock_holder: workspace.lock_holder,
                            config: workspace.config,
                            state: WorkspaceState::Deleted,
                            _state: std::marker::PhantomData,
                        })
                    }
                    WorkspaceState::Initializing => Ok(Workspace {
                        id: workspace.id,
                        name: workspace.name,
                        path: workspace.path,
                        created_at: workspace.created_at,
                        updated_at: workspace.updated_at,
                        lock_holder: workspace.lock_holder,
                        config: workspace.config,
                        state: WorkspaceState::Deleted,
                        _state: std::marker::PhantomData,
                    }),
                    _ => unreachable!(),
                }
            }
            _ => Err(WorkspaceError::InvalidStateTransition {
                from: format!("{:?}", workspace.state),
                to: "Deleted".into(),
            }),
        }
    }

    pub fn recover_workspace(
        workspace: Workspace,
    ) -> std::result::Result<Workspace, WorkspaceError> {
        if workspace.state == WorkspaceState::Locked {
            // Unlock first, then activate
            let unlocked = Workspace {
                id: workspace.id,
                name: workspace.name,
                path: workspace.path,
                created_at: workspace.created_at,
                updated_at: workspace.updated_at,
                lock_holder: None,
                config: workspace.config,
                state: WorkspaceState::Active,
                _state: std::marker::PhantomData,
            };
            // Now activate
            unlocked.activate().map(|w| Workspace {
                id: w.id,
                name: w.name,
                path: w.path,
                created_at: w.created_at,
                updated_at: w.updated_at,
                lock_holder: w.lock_holder,
                config: w.config,
                state: WorkspaceState::Active,
                _state: std::marker::PhantomData,
            })
        } else {
            Err(WorkspaceError::InvalidStateTransition {
                from: format!("{:?}", workspace.state),
                to: "Recoverable".into(),
            })
        }
    }

    pub fn get_active_workspaces(workspaces: &[Workspace]) -> Vec<&Workspace> {
        workspaces
            .iter()
            .filter(|w| w.state == WorkspaceState::Active)
            .collect()
    }

    pub fn get_locked_workspaces(workspaces: &[Workspace]) -> Vec<&Workspace> {
        workspaces
            .iter()
            .filter(|w| w.state == WorkspaceState::Locked)
            .collect()
    }

    pub fn find_workspace<'a>(
        workspaces: &'a [Workspace],
        workspace_id: &WorkspaceId,
    ) -> Option<&'a Workspace> {
        workspaces.iter().find(|w| &w.id == workspace_id)
    }

    pub fn find_by_name<'a>(
        workspaces: &'a [Workspace],
        name: &WorkspaceName,
    ) -> Option<&'a Workspace> {
        workspaces.iter().find(|w| &w.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_service_create() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        assert_eq!(workspace.state, WorkspaceState::Initializing);
    }

    #[test]
    fn workspace_service_initialize() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(workspace).unwrap();
        assert_eq!(initialized.state, WorkspaceState::Active);
    }

    #[test]
    fn workspace_service_lock_and_unlock() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(workspace).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "agent-1".into()).unwrap();
        assert!(locked.is_locked());
        let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
        assert!(unlocked.is_active());
    }

    #[test]
    fn workspace_service_delete_fails_when_locked() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(workspace).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "agent-1".into()).unwrap();
        let result = WorkspaceService::delete_workspace(locked);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_service_delete_succeeds_when_active() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("del-active".into()).unwrap(),
            WorkspacePath::new("/tmp/del-active".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(workspace).unwrap();
        let deleted = WorkspaceService::delete_workspace(initialized).unwrap();
        assert_eq!(deleted.state, WorkspaceState::Deleted);
    }

    #[test]
    fn workspace_service_delete_succeeds_when_initializing() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("del-init".into()).unwrap(),
            WorkspacePath::new("/tmp/del-init".into()).unwrap(),
        )
        .unwrap();
        let deleted = WorkspaceService::delete_workspace(workspace).unwrap();
        assert_eq!(deleted.state, WorkspaceState::Deleted);
    }

    #[test]
    fn workspace_service_delete_fails_when_corrupted() {
        // Create a corrupted workspace by manually constructing it
        let workspace = Workspace::create(
            WorkspaceName::new("del-corrupt".into()).unwrap(),
            WorkspacePath::new("/tmp/del-corrupt".into()).unwrap(),
        )
        .unwrap();
        let ws_for_delete = Workspace {
            id: workspace.id,
            name: workspace.name,
            path: workspace.path,
            created_at: workspace.created_at,
            updated_at: workspace.updated_at,
            lock_holder: None,
            config: workspace.config,
            state: WorkspaceState::Corrupted,
            _state: std::marker::PhantomData,
        };
        let result = WorkspaceService::delete_workspace(ws_for_delete);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_service_unlock_fails_when_not_locked() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("unlock-active".into()).unwrap(),
            WorkspacePath::new("/tmp/unlock-active".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(workspace).unwrap();
        let result = WorkspaceService::unlock_workspace(initialized);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_service_lock_sets_lock_holder() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("lock-holder".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-holder".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(workspace).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "agent-99".into()).unwrap();
        assert_eq!(locked.lock_holder(), Some("agent-99"));
    }

    #[test]
    fn workspace_service_unlock_clears_lock_holder() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("unlock-clear".into()).unwrap(),
            WorkspacePath::new("/tmp/unlock-clear".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(workspace).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "agent-50".into()).unwrap();
        let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
        assert!(unlocked.lock_holder().is_none());
    }

    #[test]
    fn workspace_service_get_active_workspaces() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("active-1".into()).unwrap(),
            WorkspacePath::new("/tmp/active-1".into()).unwrap(),
        )
        .unwrap();
        let active1 = WorkspaceService::initialize_workspace(ws1).unwrap();

        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("init-2".into()).unwrap(),
            WorkspacePath::new("/tmp/init-2".into()).unwrap(),
        )
        .unwrap();

        let all = vec![active1.clone(), ws2];
        let active_list = WorkspaceService::get_active_workspaces(&all);
        assert_eq!(active_list.len(), 1);
        assert_eq!(active_list[0].name.as_str(), "active-1");
    }

    #[test]
    fn workspace_service_get_locked_workspaces() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("lock-1".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-1".into()).unwrap(),
        )
        .unwrap();
        let active1 = WorkspaceService::initialize_workspace(ws1).unwrap();
        let locked1 = WorkspaceService::lock_workspace(active1, "a1".into()).unwrap();

        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("active-2".into()).unwrap(),
            WorkspacePath::new("/tmp/active-2".into()).unwrap(),
        )
        .unwrap();
        let active2 = WorkspaceService::initialize_workspace(ws2).unwrap();

        let all = vec![locked1.clone(), active2];
        let locked_list = WorkspaceService::get_locked_workspaces(&all);
        assert_eq!(locked_list.len(), 1);
        assert_eq!(locked_list[0].name.as_str(), "lock-1");
    }

    #[test]
    fn workspace_service_find_workspace_by_id() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("find-1".into()).unwrap(),
            WorkspacePath::new("/tmp/find-1".into()).unwrap(),
        )
        .unwrap();
        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("find-2".into()).unwrap(),
            WorkspacePath::new("/tmp/find-2".into()).unwrap(),
        )
        .unwrap();
        let all = vec![ws1.clone(), ws2];
        let found = WorkspaceService::find_workspace(&all, &ws1.id).unwrap();
        assert_eq!(found.name.as_str(), "find-1");
    }

    #[test]
    fn workspace_service_find_workspace_missing() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("find-miss".into()).unwrap(),
            WorkspacePath::new("/tmp/find-miss".into()).unwrap(),
        )
        .unwrap();
        let all = vec![ws1];
        let found = WorkspaceService::find_workspace(
            &all,
            &WorkspaceId::parse("nonexistent".into()).unwrap(),
        );
        assert!(found.is_none());
    }

    #[test]
    fn workspace_service_find_by_name() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("name-search".into()).unwrap(),
            WorkspacePath::new("/tmp/name-search".into()).unwrap(),
        )
        .unwrap();
        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("other".into()).unwrap(),
            WorkspacePath::new("/tmp/other".into()).unwrap(),
        )
        .unwrap();
        let all = vec![ws1.clone(), ws2];
        let found = WorkspaceService::find_by_name(
            &all,
            &WorkspaceName::new("name-search".into()).unwrap(),
        );
        assert!(found.is_some());
        assert_eq!(found.unwrap().id.as_str(), ws1.id.as_str());
    }

    #[test]
    fn workspace_service_find_by_name_missing() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("name-miss".into()).unwrap(),
            WorkspacePath::new("/tmp/name-miss".into()).unwrap(),
        )
        .unwrap();
        let all = vec![ws1];
        let found =
            WorkspaceService::find_by_name(&all, &WorkspaceName::new("ghost".into()).unwrap());
        assert!(found.is_none());
    }

    #[test]
    fn workspace_service_create_with_different_names() {
        for name in &["alpha", "beta-1", "gamma_2", "Delta3"] {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new((*name).into()).unwrap(),
                WorkspacePath::new("/tmp/test".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.name.as_str(), *name);
        }
    }

    // --- recover_workspace ---

    #[test]
    fn workspace_service_recover_locked_workspace() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("recover".into()).unwrap(),
            WorkspacePath::new("/tmp/recover".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "agent-1".into()).unwrap();

        let recovered = WorkspaceService::recover_workspace(locked).unwrap();
        assert!(recovered.is_active());
        assert!(recovered.lock_holder().is_none());
    }

    #[test]
    fn workspace_service_recover_fails_when_not_locked() {
        // Active workspace is not locked, so recovery should fail
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("recover-active".into()).unwrap(),
            WorkspacePath::new("/tmp/recover-active".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();

        let result = WorkspaceService::recover_workspace(initialized);
        assert!(result.is_err());
        match result.err() {
            Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                assert_eq!(from, "Active");
                assert_eq!(to, "Recoverable");
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[test]
    fn workspace_service_recover_fails_when_initializing() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("recover-init".into()).unwrap(),
            WorkspacePath::new("/tmp/recover-init".into()).unwrap(),
        )
        .unwrap();
        let result = WorkspaceService::recover_workspace(ws);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_service_recover_preserves_workspace_id_and_name() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("recover-preserve".into()).unwrap(),
            WorkspacePath::new("/tmp/recover-preserve".into()).unwrap(),
        )
        .unwrap();
        let id_before = ws.id.as_str().to_string();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "agent-1".into()).unwrap();

        let recovered = WorkspaceService::recover_workspace(locked).unwrap();
        assert_eq!(recovered.id.as_str(), id_before);
        assert_eq!(recovered.name.as_str(), "recover-preserve");
    }

    // --- delete_workspace edge cases ---

    #[test]
    fn workspace_service_delete_fails_when_already_deleted() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("del-deleted".into()).unwrap(),
            WorkspacePath::new("/tmp/del-deleted".into()).unwrap(),
        )
        .unwrap();
        let ws_deleted = Workspace {
            id: ws.id,
            name: ws.name,
            path: ws.path,
            created_at: ws.created_at,
            updated_at: ws.updated_at,
            lock_holder: None,
            config: ws.config,
            state: WorkspaceState::Deleted,
            _state: std::marker::PhantomData,
        };
        let result = WorkspaceService::delete_workspace(ws_deleted);
        assert!(result.is_err());
    }

    // --- unlock_workspace edge cases ---

    #[test]
    fn workspace_service_unlock_fails_when_initializing() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("unlock-init".into()).unwrap(),
            WorkspacePath::new("/tmp/unlock-init".into()).unwrap(),
        )
        .unwrap();
        let result = WorkspaceService::unlock_workspace(ws);
        assert!(result.is_err());
        match result.err() {
            Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                assert_eq!(from, "Initializing");
                assert_eq!(to, "Active");
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    // --- filter helpers on empty slice ---

    #[test]
    fn workspace_service_get_active_workspaces_empty() {
        let list = WorkspaceService::get_active_workspaces(&[]);
        assert!(list.is_empty());
    }

    #[test]
    fn workspace_service_get_locked_workspaces_empty() {
        let list = WorkspaceService::get_locked_workspaces(&[]);
        assert!(list.is_empty());
    }

    #[test]
    fn workspace_service_find_workspace_empty() {
        let found =
            WorkspaceService::find_workspace(&[], &WorkspaceId::parse("any".into()).unwrap());
        assert!(found.is_none());
    }

    #[test]
    fn workspace_service_find_by_name_empty() {
        let found = WorkspaceService::find_by_name(&[], &WorkspaceName::new("any".into()).unwrap());
        assert!(found.is_none());
    }

    // --- initialize_workspace edge cases ---

    #[test]
    fn workspace_service_initialize_updates_updated_at() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("init-ts".into()).unwrap(),
            WorkspacePath::new("/tmp/init-ts".into()).unwrap(),
        )
        .unwrap();
        let created_at = ws.created_at();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        assert!(initialized.updated_at() >= created_at);
    }

    #[test]
    fn workspace_service_initialize_preserves_id() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("init-id".into()).unwrap(),
            WorkspacePath::new("/tmp/init-id".into()).unwrap(),
        )
        .unwrap();
        let id = ws.id.as_str().to_string();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        assert_eq!(initialized.id.as_str(), id);
    }

    #[test]
    fn workspace_service_initialize_preserves_config() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("init-cfg".into()).unwrap(),
            WorkspacePath::new("/tmp/init-cfg".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let config = initialized.config().expect("should have config");
        assert_eq!(config.default_branch, "main");
        assert!(config.auto_sync);
    }

    // --- Additional unit tests ---

    #[test]
    fn workspace_service_create_preserves_name() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("name-preserve".into()).unwrap(),
            WorkspacePath::new("/tmp/name-preserve".into()).unwrap(),
        )
        .unwrap();
        assert_eq!(ws.name().as_str(), "name-preserve");
    }

    #[test]
    fn workspace_service_create_preserves_path() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("path-preserve".into()).unwrap(),
            WorkspacePath::new("/tmp/path-preserve".into()).unwrap(),
        )
        .unwrap();
        assert!(ws.path().as_str().unwrap().contains("/tmp/path-preserve"));
    }

    #[test]
    fn workspace_service_create_generates_unique_ids() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("id-1".into()).unwrap(),
            WorkspacePath::new("/tmp/id-1".into()).unwrap(),
        )
        .unwrap();
        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("id-2".into()).unwrap(),
            WorkspacePath::new("/tmp/id-2".into()).unwrap(),
        )
        .unwrap();
        assert_ne!(ws1.id.as_str(), ws2.id.as_str());
    }

    #[test]
    fn workspace_service_initialize_preserves_name() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("init-name".into()).unwrap(),
            WorkspacePath::new("/tmp/init-name".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        assert_eq!(initialized.name().as_str(), "init-name");
    }

    #[test]
    fn workspace_service_lock_preserves_id() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("lock-id".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-id".into()).unwrap(),
        )
        .unwrap();
        let id = ws.id.as_str().to_string();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "agent".into()).unwrap();
        assert_eq!(locked.id.as_str(), id);
    }

    #[test]
    fn workspace_service_unlock_preserves_name() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("unlock-name".into()).unwrap(),
            WorkspacePath::new("/tmp/unlock-name".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "agent".into()).unwrap();
        let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
        assert_eq!(unlocked.name().as_str(), "unlock-name");
    }

    #[test]
    fn workspace_service_delete_initializing_succeeds() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("del-init".into()).unwrap(),
            WorkspacePath::new("/tmp/del-init".into()).unwrap(),
        )
        .unwrap();
        let deleted = WorkspaceService::delete_workspace(ws).unwrap();
        assert_eq!(deleted.state, WorkspaceState::Deleted);
        assert!(deleted.is_terminal());
    }

    #[test]
    fn workspace_service_delete_active_succeeds() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("del-active2".into()).unwrap(),
            WorkspacePath::new("/tmp/del-active2".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let deleted = WorkspaceService::delete_workspace(initialized).unwrap();
        assert_eq!(deleted.state, WorkspaceState::Deleted);
    }

    #[test]
    fn workspace_service_delete_locked_returns_workspace_locked_error() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("del-locked".into()).unwrap(),
            WorkspacePath::new("/tmp/del-locked".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "agent-1".into()).unwrap();
        let result = WorkspaceService::delete_workspace(locked);
        match result.err() {
            Some(WorkspaceError::WorkspaceLocked(id, holder)) => {
                assert!(id.contains("del-locked") || !id.is_empty());
                assert_eq!(holder, "agent-1");
            }
            other => panic!("expected WorkspaceLocked, got {other:?}"),
        }
    }

    #[test]
    fn workspace_service_delete_preserves_created_at() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("del-ts".into()).unwrap(),
            WorkspacePath::new("/tmp/del-ts".into()).unwrap(),
        )
        .unwrap();
        let created_at = ws.created_at();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let deleted = WorkspaceService::delete_workspace(initialized).unwrap();
        assert_eq!(deleted.created_at(), created_at);
    }

    #[test]
    fn workspace_service_recover_locked_clears_lock_holder() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("recover-clear".into()).unwrap(),
            WorkspacePath::new("/tmp/recover-clear".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "old-agent".into()).unwrap();
        let recovered = WorkspaceService::recover_workspace(locked).unwrap();
        assert!(recovered.lock_holder().is_none());
        assert!(recovered.is_active());
    }

    #[test]
    fn workspace_service_recover_preserves_config() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("recover-cfg".into()).unwrap(),
            WorkspacePath::new("/tmp/recover-cfg".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let locked = WorkspaceService::lock_workspace(initialized, "agent".into()).unwrap();
        let recovered = WorkspaceService::recover_workspace(locked).unwrap();
        let config = recovered.config().expect("should have config");
        assert_eq!(config.default_branch, "main");
    }

    #[test]
    fn workspace_service_recover_fails_when_corrupted() {
        let ws = Workspace::create(
            WorkspaceName::new("recover-corrupt".into()).unwrap(),
            WorkspacePath::new("/tmp/recover-corrupt".into()).unwrap(),
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
        let result = WorkspaceService::recover_workspace(corrupted_ws);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_service_get_active_workspaces_multiple() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("active-a".into()).unwrap(),
            WorkspacePath::new("/tmp/active-a".into()).unwrap(),
        )
        .unwrap();
        let active_a = WorkspaceService::initialize_workspace(ws1).unwrap();

        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("active-b".into()).unwrap(),
            WorkspacePath::new("/tmp/active-b".into()).unwrap(),
        )
        .unwrap();
        let active_b = WorkspaceService::initialize_workspace(ws2).unwrap();

        let ws3 = WorkspaceService::create_workspace(
            WorkspaceName::new("init-c".into()).unwrap(),
            WorkspacePath::new("/tmp/init-c".into()).unwrap(),
        )
        .unwrap();

        let all = vec![active_a.clone(), active_b.clone(), ws3];
        let active_list = WorkspaceService::get_active_workspaces(&all);
        assert_eq!(active_list.len(), 2);
    }

    #[test]
    fn workspace_service_get_locked_workspaces_multiple() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("lock-a".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-a".into()).unwrap(),
        )
        .unwrap();
        let active1 = WorkspaceService::initialize_workspace(ws1).unwrap();
        let locked1 = WorkspaceService::lock_workspace(active1, "a1".into()).unwrap();

        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("lock-b".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-b".into()).unwrap(),
        )
        .unwrap();
        let active2 = WorkspaceService::initialize_workspace(ws2).unwrap();
        let locked2 = WorkspaceService::lock_workspace(active2, "a2".into()).unwrap();

        let all = vec![locked1.clone(), locked2.clone()];
        let locked_list = WorkspaceService::get_locked_workspaces(&all);
        assert_eq!(locked_list.len(), 2);
    }

    #[test]
    fn workspace_service_find_by_name_returns_first_match() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("dup".into()).unwrap(),
            WorkspacePath::new("/tmp/dup-1".into()).unwrap(),
        )
        .unwrap();
        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("other".into()).unwrap(),
            WorkspacePath::new("/tmp/other".into()).unwrap(),
        )
        .unwrap();
        let all = vec![ws1.clone(), ws2];
        let found =
            WorkspaceService::find_by_name(&all, &WorkspaceName::new("dup".into()).unwrap());
        assert!(found.is_some());
        assert_eq!(found.unwrap().id.as_str(), ws1.id.as_str());
    }

    #[test]
    fn workspace_service_get_active_workspaces_no_actives() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("init-a".into()).unwrap(),
            WorkspacePath::new("/tmp/init-a".into()).unwrap(),
        )
        .unwrap();
        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("init-b".into()).unwrap(),
            WorkspacePath::new("/tmp/init-b".into()).unwrap(),
        )
        .unwrap();
        let all = vec![ws1, ws2];
        let active_list = WorkspaceService::get_active_workspaces(&all);
        assert!(active_list.is_empty());
    }

    #[test]
    fn workspace_service_get_locked_workspaces_no_locked() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("active-x".into()).unwrap(),
            WorkspacePath::new("/tmp/active-x".into()).unwrap(),
        )
        .unwrap();
        let active = WorkspaceService::initialize_workspace(ws1).unwrap();
        let all = vec![active];
        let locked_list = WorkspaceService::get_locked_workspaces(&all);
        assert!(locked_list.is_empty());
    }

    #[test]
    fn workspace_service_unlock_fails_when_deleted() {
        let ws = Workspace {
            id: WorkspaceId::parse("del-unlock".into()).unwrap(),
            name: WorkspaceName::new("del-unlock".into()).unwrap(),
            path: WorkspacePath::new("/tmp/del-unlock".into()).unwrap(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            lock_holder: None,
            config: None,
            state: WorkspaceState::Deleted,
            _state: std::marker::PhantomData,
        };
        let result = WorkspaceService::unlock_workspace(ws);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_service_unlock_fails_when_corrupted() {
        let ws = Workspace {
            id: WorkspaceId::parse("corrupt-unlock".into()).unwrap(),
            name: WorkspaceName::new("corrupt-unlock".into()).unwrap(),
            path: WorkspacePath::new("/tmp/corrupt-unlock".into()).unwrap(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            lock_holder: None,
            config: None,
            state: WorkspaceState::Corrupted,
            _state: std::marker::PhantomData,
        };
        let result = WorkspaceService::unlock_workspace(ws);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_service_full_lifecycle_with_recover() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("full-cycle".into()).unwrap(),
            WorkspacePath::new("/tmp/full-cycle".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        assert!(initialized.is_active());

        let locked = WorkspaceService::lock_workspace(initialized, "agent-stuck".into()).unwrap();
        assert!(locked.is_locked());

        let recovered = WorkspaceService::recover_workspace(locked).unwrap();
        assert!(recovered.is_active());

        let deleted = WorkspaceService::delete_workspace(recovered).unwrap();
        assert!(deleted.is_terminal());
    }

    // --- Exhaustive query method tests (ha-49p) ---

    mod query_exhaustive {
        use super::*;

        fn make_active(name: &str) -> Workspace {
            WorkspaceService::create_workspace(
                WorkspaceName::new(name.into()).unwrap(),
                WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
            )
            .and_then(WorkspaceService::initialize_workspace)
            .unwrap()
        }

        fn make_locked(name: &str, holder: &str) -> Workspace {
            WorkspaceService::lock_workspace(make_active(name), holder.into()).unwrap()
        }

        fn make_corrupted(name: &str) -> Workspace {
            let ws = make_active(name);
            Workspace {
                state: WorkspaceState::Corrupted,
                ..ws
            }
        }

        fn make_deleted(name: &str) -> Workspace {
            let ws = make_active(name);
            Workspace {
                state: WorkspaceState::Deleted,
                ..ws
            }
        }

        fn make_initializing(name: &str) -> Workspace {
            WorkspaceService::create_workspace(
                WorkspaceName::new(name.into()).unwrap(),
                WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
            )
            .unwrap()
        }

        // ── get_active_workspaces ──

        #[test]
        fn get_active_single_active() {
            let ws = make_active("solo-active");
            let all = [ws];
            let result = WorkspaceService::get_active_workspaces(&all);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].name.as_str(), "solo-active");
        }

        #[test]
        fn get_active_excludes_initializing() {
            let ws = make_initializing("init-ws");
            let all = [ws];
            let result = WorkspaceService::get_active_workspaces(&all);
            assert!(result.is_empty());
        }

        #[test]
        fn get_active_excludes_locked() {
            let ws = make_locked("locked-ws", "agent-1");
            let all = [ws];
            let result = WorkspaceService::get_active_workspaces(&all);
            assert!(result.is_empty());
        }

        #[test]
        fn get_active_excludes_corrupted() {
            let ws = make_corrupted("corrupt-ws");
            let all = [ws];
            let result = WorkspaceService::get_active_workspaces(&all);
            assert!(result.is_empty());
        }

        #[test]
        fn get_active_excludes_deleted() {
            let ws = make_deleted("deleted-ws");
            let all = [ws];
            let result = WorkspaceService::get_active_workspaces(&all);
            assert!(result.is_empty());
        }

        #[test]
        fn get_active_mixed_states_returns_only_active() {
            let all: Vec<Workspace> = vec![
                make_active("active-1"),
                make_initializing("init-1"),
                make_locked("locked-1", "a"),
                make_corrupted("corrupt-1"),
                make_deleted("deleted-1"),
                make_active("active-2"),
            ];
            let result = WorkspaceService::get_active_workspaces(&all);
            assert_eq!(result.len(), 2);
            let names: Vec<&str> = result.iter().map(|w| w.name.as_str()).collect();
            assert!(names.contains(&"active-1"));
            assert!(names.contains(&"active-2"));
        }

        #[test]
        fn get_active_many_actives() {
            let all: Vec<Workspace> = (0..20)
                .map(|i| make_active(&format!("active-{}", i)))
                .collect();
            let result = WorkspaceService::get_active_workspaces(&all);
            assert_eq!(result.len(), 20);
        }

        #[test]
        fn get_active_empty_slice() {
            let result = WorkspaceService::get_active_workspaces(&[]);
            assert!(result.is_empty());
        }

        #[test]
        fn get_active_does_not_mutate_input() {
            let all: Vec<Workspace> = vec![make_active("a1"), make_locked("l1", "agent")];
            let pre_len = all.len();
            let _ = WorkspaceService::get_active_workspaces(&all);
            assert_eq!(all.len(), pre_len);
            assert_eq!(all[0].name.as_str(), "a1");
            assert_eq!(all[1].name.as_str(), "l1");
        }

        #[test]
        fn get_active_consistent_across_calls() {
            let all: Vec<Workspace> = vec![make_active("c1"), make_initializing("c2")];
            let first = WorkspaceService::get_active_workspaces(&all);
            let second = WorkspaceService::get_active_workspaces(&all);
            assert_eq!(first.len(), second.len());
            assert_eq!(first[0].id.as_str(), second[0].id.as_str());
        }

        #[test]
        fn get_active_preserves_order() {
            let all: Vec<Workspace> = vec![
                make_active("first"),
                make_active("second"),
                make_active("third"),
            ];
            let result = WorkspaceService::get_active_workspaces(&all);
            assert_eq!(result[0].name.as_str(), "first");
            assert_eq!(result[1].name.as_str(), "second");
            assert_eq!(result[2].name.as_str(), "third");
        }

        // ── get_locked_workspaces ──

        #[test]
        fn get_locked_single_locked() {
            let ws = make_locked("solo-locked", "holder");
            let all = [ws];
            let result = WorkspaceService::get_locked_workspaces(&all);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].name.as_str(), "solo-locked");
            assert_eq!(result[0].lock_holder(), Some("holder"));
        }

        #[test]
        fn get_locked_excludes_active() {
            let ws = make_active("active-ws");
            let all = [ws];
            let result = WorkspaceService::get_locked_workspaces(&all);
            assert!(result.is_empty());
        }

        #[test]
        fn get_locked_excludes_initializing() {
            let ws = make_initializing("init-ws");
            let all = [ws];
            let result = WorkspaceService::get_locked_workspaces(&all);
            assert!(result.is_empty());
        }

        #[test]
        fn get_locked_excludes_corrupted() {
            let ws = make_corrupted("corrupt-ws");
            let all = [ws];
            let result = WorkspaceService::get_locked_workspaces(&all);
            assert!(result.is_empty());
        }

        #[test]
        fn get_locked_excludes_deleted() {
            let ws = make_deleted("deleted-ws");
            let all = [ws];
            let result = WorkspaceService::get_locked_workspaces(&all);
            assert!(result.is_empty());
        }

        #[test]
        fn get_locked_mixed_states_returns_only_locked() {
            let all: Vec<Workspace> = vec![
                make_locked("locked-1", "a"),
                make_active("active-1"),
                make_initializing("init-1"),
                make_corrupted("corrupt-1"),
                make_deleted("deleted-1"),
                make_locked("locked-2", "b"),
            ];
            let result = WorkspaceService::get_locked_workspaces(&all);
            assert_eq!(result.len(), 2);
            let names: Vec<&str> = result.iter().map(|w| w.name.as_str()).collect();
            assert!(names.contains(&"locked-1"));
            assert!(names.contains(&"locked-2"));
        }

        #[test]
        fn get_locked_many_locked() {
            let all: Vec<Workspace> = (0..20)
                .map(|i| make_locked(&format!("locked-{}", i), &format!("agent-{}", i)))
                .collect();
            let result = WorkspaceService::get_locked_workspaces(&all);
            assert_eq!(result.len(), 20);
        }

        #[test]
        fn get_locked_empty_slice() {
            let result = WorkspaceService::get_locked_workspaces(&[]);
            assert!(result.is_empty());
        }

        #[test]
        fn get_locked_does_not_mutate_input() {
            let all: Vec<Workspace> = vec![make_locked("l1", "agent"), make_active("a1")];
            let pre_len = all.len();
            let _ = WorkspaceService::get_locked_workspaces(&all);
            assert_eq!(all.len(), pre_len);
            assert_eq!(all[0].name.as_str(), "l1");
            assert_eq!(all[1].name.as_str(), "a1");
        }

        #[test]
        fn get_locked_consistent_across_calls() {
            let all: Vec<Workspace> = vec![make_locked("c1", "a"), make_active("c2")];
            let first = WorkspaceService::get_locked_workspaces(&all);
            let second = WorkspaceService::get_locked_workspaces(&all);
            assert_eq!(first.len(), second.len());
            assert_eq!(first[0].id.as_str(), second[0].id.as_str());
        }

        #[test]
        fn get_locked_preserves_order() {
            let all: Vec<Workspace> = vec![
                make_locked("first", "a"),
                make_locked("second", "b"),
                make_locked("third", "c"),
            ];
            let result = WorkspaceService::get_locked_workspaces(&all);
            assert_eq!(result[0].name.as_str(), "first");
            assert_eq!(result[1].name.as_str(), "second");
            assert_eq!(result[2].name.as_str(), "third");
        }

        // ── find_workspace ──

        #[test]
        fn find_workspace_found() {
            let ws = make_active("find-me");
            let all = vec![ws.clone()];
            let found = WorkspaceService::find_workspace(&all, &ws.id);
            assert!(found.is_some());
            assert_eq!(found.unwrap().id.as_str(), ws.id.as_str());
        }

        #[test]
        fn find_workspace_not_found() {
            let ws = make_active("find-me");
            let all = vec![ws];
            let ghost_id = WorkspaceId::parse("ws-nonexistent".into()).unwrap();
            let found = WorkspaceService::find_workspace(&all, &ghost_id);
            assert!(found.is_none());
        }

        #[test]
        fn find_workspace_empty_slice() {
            let ghost_id = WorkspaceId::parse("ws-ghost".into()).unwrap();
            let found = WorkspaceService::find_workspace(&[], &ghost_id);
            assert!(found.is_none());
        }

        #[test]
        fn find_workspace_among_many() {
            let ws0 = make_active("ws-0");
            let ws1 = make_active("ws-1");
            let ws2 = make_active("ws-2");
            let target_id = ws1.id.clone();
            let all = vec![ws0, ws1, ws2];
            let found = WorkspaceService::find_workspace(&all, &target_id);
            assert!(found.is_some());
            assert_eq!(found.unwrap().name.as_str(), "ws-1");
        }

        #[test]
        fn find_workspace_returns_correct_reference() {
            let ws = make_active("ref-test");
            let all = vec![ws.clone()];
            let found = WorkspaceService::find_workspace(&all, &ws.id).unwrap();
            assert!(std::ptr::eq(found, &all[0]));
        }

        #[test]
        fn find_workspace_does_not_mutate_input() {
            let ws = make_active("no-mut");
            let all = vec![ws.clone()];
            let id = ws.id.clone();
            let _ = WorkspaceService::find_workspace(&all, &id);
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].name.as_str(), "no-mut");
        }

        #[test]
        fn find_workspace_consistent_across_calls() {
            let ws = make_active("consistent");
            let all = vec![ws.clone()];
            let first = WorkspaceService::find_workspace(&all, &ws.id);
            let second = WorkspaceService::find_workspace(&all, &ws.id);
            assert_eq!(first.is_some(), second.is_some());
            assert_eq!(first.unwrap().id.as_str(), second.unwrap().id.as_str());
        }

        #[test]
        fn find_workspace_finds_in_mixed_states() {
            let active = make_active("active-find");
            let locked = make_locked("locked-find", "a");
            let target_id = locked.id.clone();
            let all = vec![active, locked];
            let found = WorkspaceService::find_workspace(&all, &target_id);
            assert!(found.is_some());
            assert_eq!(found.unwrap().name.as_str(), "locked-find");
        }

        // ── find_by_name ──

        #[test]
        fn find_by_name_found() {
            let ws = make_active("unique-name");
            let all = vec![ws.clone()];
            let found = WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("unique-name".into()).unwrap(),
            );
            assert!(found.is_some());
            assert_eq!(found.unwrap().id.as_str(), ws.id.as_str());
        }

        #[test]
        fn find_by_name_not_found() {
            let ws = make_active("existing");
            let all = vec![ws];
            let found = WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("nonexistent".into()).unwrap(),
            );
            assert!(found.is_none());
        }

        #[test]
        fn find_by_name_empty_slice() {
            let found = WorkspaceService::find_by_name(
                &[],
                &WorkspaceName::new("anything".into()).unwrap(),
            );
            assert!(found.is_none());
        }

        #[test]
        fn find_by_name_returns_first_match_when_duplicates() {
            let ws1 = WorkspaceService::create_workspace(
                WorkspaceName::new("dup-name".into()).unwrap(),
                WorkspacePath::new("/tmp/dup-1".into()).unwrap(),
            )
            .unwrap();
            let ws2 = WorkspaceService::create_workspace(
                WorkspaceName::new("dup-name".into()).unwrap(),
                WorkspacePath::new("/tmp/dup-2".into()).unwrap(),
            )
            .unwrap();
            let first_id = ws1.id.as_str().to_string();
            let all = vec![ws1, ws2];
            let found = WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("dup-name".into()).unwrap(),
            );
            assert!(found.is_some());
            assert_eq!(found.unwrap().id.as_str(), first_id);
        }

        #[test]
        fn find_by_name_case_sensitive() {
            let ws = make_active("case-test");
            let all = vec![ws];
            assert!(WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("case-test".into()).unwrap(),
            )
            .is_some());
            assert!(WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("Case-Test".into()).unwrap(),
            )
            .is_none());
            assert!(WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("CASE-TEST".into()).unwrap(),
            )
            .is_none());
        }

        #[test]
        fn find_by_name_among_many() {
            let all: Vec<Workspace> = (0..10).map(|i| make_active(&format!("ws-{}", i))).collect();
            let found =
                WorkspaceService::find_by_name(&all, &WorkspaceName::new("ws-7".into()).unwrap());
            assert!(found.is_some());
            assert_eq!(found.unwrap().name.as_str(), "ws-7");
        }

        #[test]
        fn find_by_name_returns_correct_reference() {
            let ws = make_active("ref-name");
            let all = vec![ws.clone()];
            let found = WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("ref-name".into()).unwrap(),
            )
            .unwrap();
            assert!(std::ptr::eq(found, &all[0]));
        }

        #[test]
        fn find_by_name_does_not_mutate_input() {
            let ws = make_active("no-mut-name");
            let all = vec![ws];
            let name = WorkspaceName::new("no-mut-name".into()).unwrap();
            let _ = WorkspaceService::find_by_name(&all, &name);
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].name.as_str(), "no-mut-name");
        }

        #[test]
        fn find_by_name_consistent_across_calls() {
            let ws = make_active("idem-name");
            let all = vec![ws.clone()];
            let name = WorkspaceName::new("idem-name".into()).unwrap();
            let first = WorkspaceService::find_by_name(&all, &name);
            let second = WorkspaceService::find_by_name(&all, &name);
            assert_eq!(first.is_some(), second.is_some());
            assert_eq!(first.unwrap().id.as_str(), second.unwrap().id.as_str());
        }

        #[test]
        fn find_by_name_finds_in_mixed_states() {
            let active = make_active("active-name");
            let locked = make_locked("locked-name", "a");
            let all = vec![active, locked];
            let found = WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("locked-name".into()).unwrap(),
            );
            assert!(found.is_some());
            assert_eq!(found.unwrap().name.as_str(), "locked-name");
        }

        #[test]
        fn find_by_name_duplicate_names_returns_first_occurrence() {
            let ws1 = make_active("dup");
            let first_id = ws1.id.as_str().to_string();
            let ws2 = make_active("dup");
            let ws3 = make_active("dup");
            let all = vec![ws1, ws2, ws3];
            let found =
                WorkspaceService::find_by_name(&all, &WorkspaceName::new("dup".into()).unwrap());
            assert!(found.is_some());
            assert_eq!(found.unwrap().id.as_str(), first_id);
        }

        // ── Cross-method consistency ──

        #[test]
        fn get_active_and_get_locked_are_disjoint() {
            let all: Vec<Workspace> = vec![
                make_active("a1"),
                make_locked("l1", "h1"),
                make_active("a2"),
                make_locked("l2", "h2"),
            ];
            let actives = WorkspaceService::get_active_workspaces(&all);
            let locked = WorkspaceService::get_locked_workspaces(&all);
            let active_ids: std::collections::HashSet<&str> =
                actives.iter().map(|w| w.id.as_str()).collect();
            let locked_ids: std::collections::HashSet<&str> =
                locked.iter().map(|w| w.id.as_str()).collect();
            assert!(active_ids.is_disjoint(&locked_ids));
        }

        #[test]
        fn find_by_id_and_find_by_name_return_same_workspace() {
            let ws = make_active("consistent-ws");
            let all = vec![ws.clone()];
            let by_id = WorkspaceService::find_workspace(&all, &ws.id).unwrap();
            let by_name = WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("consistent-ws".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(by_id.id.as_str(), by_name.id.as_str());
            assert!(std::ptr::eq(by_id, by_name));
        }

        #[test]
        fn all_queries_on_large_mixed_set() {
            let mut all: Vec<Workspace> = Vec::new();
            for i in 0..5 {
                all.push(make_active(&format!("active-{}", i)));
                all.push(make_locked(&format!("locked-{}", i), &format!("h{}", i)));
                all.push(make_initializing(&format!("init-{}", i)));
                all.push(make_corrupted(&format!("corrupt-{}", i)));
                all.push(make_deleted(&format!("deleted-{}", i)));
            }
            assert_eq!(all.len(), 25);
            assert_eq!(WorkspaceService::get_active_workspaces(&all).len(), 5);
            assert_eq!(WorkspaceService::get_locked_workspaces(&all).len(), 5);
            assert!(WorkspaceService::find_workspace(&all, &all[0].id).is_some());
            assert!(WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("active-0".into()).unwrap()
            )
            .is_some());
            assert!(WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("nonexistent".into()).unwrap()
            )
            .is_none());
        }
    }

    // --- Exhaustive initialize_workspace tests (ha-o0o) ---

    mod initialize_workspace_exhaustive {
        use super::*;

        fn make_active_ws(name: &str) -> Workspace {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new(name.into()).unwrap(),
                WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
            )
            .unwrap();
            WorkspaceService::initialize_workspace(ws).unwrap()
        }

        fn make_corrupted_ws(name: &str) -> Workspace {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new(name.into()).unwrap(),
                WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
            )
            .unwrap();
            let active = WorkspaceService::initialize_workspace(ws).unwrap();
            Workspace {
                state: WorkspaceState::Corrupted,
                ..active
            }
        }

        fn make_locked_ws(name: &str, holder: &str) -> Workspace {
            let active = make_active_ws(name);
            WorkspaceService::lock_workspace(active, holder.into()).unwrap()
        }

        fn make_deleted_ws(name: &str) -> Workspace {
            let active = make_active_ws(name);
            WorkspaceService::delete_workspace(active).unwrap()
        }

        // === Happy path: Initializing → Active ===

        #[test]
        fn init_happy_path_transitions_to_active() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("happy".into()).unwrap(),
                WorkspacePath::new("/tmp/happy".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.state, WorkspaceState::Initializing);

            let result = WorkspaceService::initialize_workspace(ws);
            assert!(result.is_ok());
            let initialized = result.unwrap();
            assert_eq!(initialized.state, WorkspaceState::Active);
            assert!(initialized.is_active());
        }

        #[test]
        fn init_happy_path_preserves_id() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("happy-id".into()).unwrap(),
                WorkspacePath::new("/tmp/happy-id".into()).unwrap(),
            )
            .unwrap();
            let id = ws.id.as_str().to_string();
            let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
            assert_eq!(initialized.id.as_str(), id);
        }

        #[test]
        fn init_happy_path_preserves_name() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("happy-name".into()).unwrap(),
                WorkspacePath::new("/tmp/happy-name".into()).unwrap(),
            )
            .unwrap();
            let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
            assert_eq!(initialized.name.as_str(), "happy-name");
        }

        #[test]
        fn init_happy_path_preserves_path() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("happy-path".into()).unwrap(),
                WorkspacePath::new("/tmp/happy-path".into()).unwrap(),
            )
            .unwrap();
            let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
            assert!(initialized
                .path
                .as_str()
                .unwrap()
                .contains("/tmp/happy-path"));
        }

        #[test]
        fn init_happy_path_preserves_config() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("happy-cfg".into()).unwrap(),
                WorkspacePath::new("/tmp/happy-cfg".into()).unwrap(),
            )
            .unwrap();
            let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
            let config = initialized.config().expect("should have config");
            assert_eq!(config.default_branch, "main");
            assert!(config.auto_sync);
            use crate::domain::entities::workspace::VcsType;
            assert_eq!(config.vcs_type, VcsType::Git);
        }

        #[test]
        fn init_happy_path_preserves_created_at() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("happy-ts".into()).unwrap(),
                WorkspacePath::new("/tmp/happy-ts".into()).unwrap(),
            )
            .unwrap();
            let created_at = ws.created_at();
            let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
            assert_eq!(initialized.created_at(), created_at);
        }

        #[test]
        fn init_happy_path_updates_updated_at() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("happy-uts".into()).unwrap(),
                WorkspacePath::new("/tmp/happy-uts".into()).unwrap(),
            )
            .unwrap();
            let created_at = ws.created_at();
            std::thread::sleep(std::time::Duration::from_millis(2));
            let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
            assert!(initialized.updated_at() >= created_at);
        }

        #[test]
        fn init_happy_path_no_lock_holder() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("happy-lock".into()).unwrap(),
                WorkspacePath::new("/tmp/happy-lock".into()).unwrap(),
            )
            .unwrap();
            let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
            assert!(initialized.lock_holder().is_none());
        }

        #[test]
        fn init_happy_path_not_locked() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("happy-nl".into()).unwrap(),
                WorkspacePath::new("/tmp/happy-nl".into()).unwrap(),
            )
            .unwrap();
            let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
            assert!(!initialized.is_locked());
        }

        #[test]
        fn init_happy_path_not_terminal() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("happy-nt".into()).unwrap(),
                WorkspacePath::new("/tmp/happy-nt".into()).unwrap(),
            )
            .unwrap();
            let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
            assert!(!initialized.is_terminal());
        }

        #[test]
        fn init_happy_path_different_names_all_succeed() {
            let names = vec![
                "alpha".to_string(),
                "beta-1".to_string(),
                "gamma_2".to_string(),
                "Delta3".to_string(),
                "x".repeat(50),
            ];
            for name in &names {
                let ws = WorkspaceService::create_workspace(
                    WorkspaceName::new(name.clone()).unwrap(),
                    WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
                )
                .unwrap();
                let result = WorkspaceService::initialize_workspace(ws);
                assert!(result.is_ok(), "should succeed for name '{}'", name);
                assert_eq!(result.unwrap().state, WorkspaceState::Active);
            }
        }

        // === Already initialized: Active state returns error ===

        #[test]
        fn init_rejects_active_workspace() {
            let active = make_active_ws("already-active");
            let result = WorkspaceService::initialize_workspace(active);
            assert!(result.is_err());
        }

        #[test]
        fn init_rejects_active_error_is_invalid_state_transition() {
            let active = make_active_ws("active-err");
            let result = WorkspaceService::initialize_workspace(active);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Active");
                    assert_eq!(to, "Active");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn init_rejects_active_preserves_workspace_identity() {
            let active = make_active_ws("active-id");
            let id_before = active.id.as_str().to_string();
            let name_before = active.name.as_str().to_string();
            let result = WorkspaceService::initialize_workspace(active);
            assert!(result.is_err());
            let _ = (id_before, name_before);
        }

        // === Corrupted state: cannot be initialized ===

        #[test]
        fn init_rejects_corrupted_workspace() {
            let corrupted = make_corrupted_ws("corrupt-init");
            let result = WorkspaceService::initialize_workspace(corrupted);
            assert!(result.is_err());
        }

        #[test]
        fn init_corrupted_error_is_invalid_state_transition() {
            let corrupted = make_corrupted_ws("corrupt-err");
            let result = WorkspaceService::initialize_workspace(corrupted);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Corrupted");
                    assert_eq!(to, "Active");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn init_corrupted_must_use_recover() {
            let corrupted = make_corrupted_ws("corrupt-recover");
            let result = WorkspaceService::initialize_workspace(corrupted.clone());
            assert!(
                result.is_err(),
                "Corrupted workspace must not be initializable — use recover_workspace instead"
            );
        }

        // === Locked state: cannot be initialized ===

        #[test]
        fn init_rejects_locked_workspace() {
            let locked = make_locked_ws("locked-init", "agent-1");
            let result = WorkspaceService::initialize_workspace(locked);
            assert!(result.is_err());
        }

        #[test]
        fn init_locked_error_is_invalid_state_transition() {
            let locked = make_locked_ws("locked-err", "agent-2");
            let result = WorkspaceService::initialize_workspace(locked);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Locked");
                    assert_eq!(to, "Active");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        // === Deleted state: cannot be initialized ===

        #[test]
        fn init_rejects_deleted_workspace() {
            let deleted = make_deleted_ws("deleted-init");
            let result = WorkspaceService::initialize_workspace(deleted);
            assert!(result.is_err());
        }

        #[test]
        fn init_deleted_error_is_invalid_state_transition() {
            let deleted = make_deleted_ws("deleted-err");
            let result = WorkspaceService::initialize_workspace(deleted);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Deleted");
                    assert_eq!(to, "Active");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        // === Table-driven: only Initializing is accepted ===

        #[test]
        fn table_driven_only_initializing_succeeds() {
            let cases: Vec<(&str, WorkspaceState, bool)> = vec![
                ("Initializing", WorkspaceState::Initializing, true),
                ("Active", WorkspaceState::Active, false),
                ("Locked", WorkspaceState::Locked, false),
                ("Corrupted", WorkspaceState::Corrupted, false),
                ("Deleted", WorkspaceState::Deleted, false),
            ];

            for (label, state, expect_ok) in cases {
                let ws = WorkspaceService::create_workspace(
                    WorkspaceName::new(format!("tbl-{}", label).into()).unwrap(),
                    WorkspacePath::new(format!("/tmp/tbl-{}", label)).unwrap(),
                )
                .unwrap();
                let ws_with_state = Workspace { state, ..ws };
                let result = WorkspaceService::initialize_workspace(ws_with_state);
                assert_eq!(
                    result.is_ok(),
                    expect_ok,
                    "state={:?} ({}): expected ok={}, got ok={}",
                    state,
                    label,
                    expect_ok,
                    result.is_ok()
                );
                if expect_ok {
                    assert_eq!(result.unwrap().state, WorkspaceState::Active);
                }
            }
        }

        // === Error message format ===

        #[test]
        fn init_error_message_contains_from_and_to_states() {
            let active = make_active_ws("err-msg");
            let result = WorkspaceService::initialize_workspace(active);
            let err = result.err().expect("should be error");
            let msg = format!("{err}");
            assert!(
                msg.contains("Active"),
                "error message should contain 'Active': {msg}"
            );
            assert!(
                msg.contains("Invalid state transition"),
                "error message should contain 'Invalid state transition': {msg}"
            );
        }

        // === Idempotency: double init fails ===

        #[test]
        fn init_double_initialize_fails() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("double".into()).unwrap(),
                WorkspacePath::new("/tmp/double".into()).unwrap(),
            )
            .unwrap();
            let first = WorkspaceService::initialize_workspace(ws).unwrap();
            assert!(first.is_active());

            let second = WorkspaceService::initialize_workspace(first);
            assert!(second.is_err(), "second initialize should fail");
        }

        // === Lifecycle: init then delete works ===

        #[test]
        fn init_then_delete_succeeds() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("init-del".into()).unwrap(),
                WorkspacePath::new("/tmp/init-del".into()).unwrap(),
            )
            .unwrap();
            let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
            let deleted = WorkspaceService::delete_workspace(initialized).unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);
        }

        // === Lifecycle: init → lock → unlock → delete ===

        #[test]
        fn init_full_lifecycle() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("lifecycle".into()).unwrap(),
                WorkspacePath::new("/tmp/lifecycle".into()).unwrap(),
            )
            .unwrap();
            let active = WorkspaceService::initialize_workspace(ws).unwrap();
            assert!(active.is_active());

            let locked = WorkspaceService::lock_workspace(active, "agent-1".into()).unwrap();
            assert!(locked.is_locked());

            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(unlocked.is_active());

            let deleted = WorkspaceService::delete_workspace(unlocked).unwrap();
            assert!(deleted.is_terminal());
        }

        // === Multiple independent initializations ===

        #[test]
        fn init_multiple_workspaces_independently() {
            let mut initialized = Vec::new();
            for i in 0..10 {
                let ws = WorkspaceService::create_workspace(
                    WorkspaceName::new(format!("batch-{}", i)).unwrap(),
                    WorkspacePath::new(format!("/tmp/batch-{}", i)).unwrap(),
                )
                .unwrap();
                let init = WorkspaceService::initialize_workspace(ws).unwrap();
                assert_eq!(init.state, WorkspaceState::Active);
                initialized.push(init);
            }
            let ids: std::collections::HashSet<&str> =
                initialized.iter().map(|w| w.id.as_str()).collect();
            assert_eq!(ids.len(), 10, "all workspace IDs must be unique");
        }
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;
        use proptest::{prop_assert, prop_assert_eq};

        proptest! {
            #[test]
            fn workspace_service_create_always_succeeds_for_valid_inputs(
                name in "[a-zA-Z0-9_-]{1,100}",
                path_suffix in "[a-zA-Z0-9_-]{1,50}"
            ) {
                let name = WorkspaceName::new(name).unwrap();
                let path = WorkspacePath::new(format!("/tmp/{}", path_suffix)).unwrap();
                let ws = WorkspaceService::create_workspace(name, path);
                prop_assert!(ws.is_ok());
                let ws = ws.unwrap();
                prop_assert_eq!(ws.state, WorkspaceState::Initializing);
            }

            #[test]
            fn workspace_service_find_workspace_roundtrip(
                name in "[a-zA-Z0-9_-]{1,50}"
            ) {
                let name = WorkspaceName::new(name.clone()).unwrap();
                let ws = WorkspaceService::create_workspace(
                    name.clone(),
                    WorkspacePath::new("/tmp/test".into()).unwrap(),
                ).unwrap();
                let all = vec![ws.clone()];
                let found = WorkspaceService::find_workspace(&all, &ws.id);
                prop_assert!(found.is_some());
                prop_assert_eq!(found.unwrap().name.as_str(), name.as_str());
            }

            #[test]
            fn workspace_service_find_by_name_roundtrip(
                name in "[a-zA-Z0-9_-]{1,50}"
            ) {
                let name = WorkspaceName::new(name.clone()).unwrap();
                let ws = WorkspaceService::create_workspace(
                    name.clone(),
                    WorkspacePath::new("/tmp/test".into()).unwrap(),
                ).unwrap();
                let all = vec![ws];
                let found = WorkspaceService::find_by_name(&all, &name);
                prop_assert!(found.is_some());
            }

            #[test]
            fn workspace_service_create_generates_unique_ids_batch(
                names in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 10..50)
            ) {
                let mut ids = std::collections::HashSet::new();
                for name in &names {
                    let ws = WorkspaceService::create_workspace(
                        WorkspaceName::new(name.clone()).unwrap(),
                        WorkspacePath::new("/tmp/test".into()).unwrap(),
                    ).unwrap();
                    ids.insert(ws.id.as_str().to_string());
                }
                prop_assert_eq!(ids.len(), names.len());
            }

            #[test]
            fn workspace_service_filter_actives_only_returns_actives(
                active_count in 0usize..10,
                init_count in 0usize..10
            ) {
                let mut all: Vec<Workspace> = Vec::new();
                for i in 0..active_count {
                    let ws = WorkspaceService::create_workspace(
                        WorkspaceName::new(format!("active-{}", i)).unwrap(),
                        WorkspacePath::new("/tmp/test".into()).unwrap(),
                    ).unwrap();
                    let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
                    all.push(initialized);
                }
                for i in 0..init_count {
                    let ws = WorkspaceService::create_workspace(
                        WorkspaceName::new(format!("init-{}", i)).unwrap(),
                        WorkspacePath::new("/tmp/test".into()).unwrap(),
                    ).unwrap();
                    all.push(ws);
                }
                let actives = WorkspaceService::get_active_workspaces(&all);
                prop_assert_eq!(actives.len(), active_count);
            }
        }
    }

    // =============================================================================
    // create_workspace exhaustive tests (ha-5ka)
    // =============================================================================

    mod create_workspace_exhaustive {
        use super::*;
        use crate::domain::entities::workspace::VcsType;

        // ── Happy path ──────────────────────────────────────────────────────────

        #[test]
        fn create_workspace_happy_path_valid_name_and_path() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("test-workspace".into()).unwrap(),
                WorkspacePath::new("/tmp/test-workspace".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.state, WorkspaceState::Initializing);
            assert_eq!(ws.name.as_str(), "test-workspace");
            assert!(ws.path.as_str().unwrap().contains("test-workspace"));
        }

        #[test]
        fn create_workspace_produces_init_state() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("init-check".into()).unwrap(),
                WorkspacePath::new("/tmp/init-check".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.state, WorkspaceState::Initializing);
            assert!(!ws.is_active());
            assert!(!ws.is_locked());
            assert!(!ws.is_terminal());
        }

        #[test]
        fn create_workspace_generates_unique_id() {
            let ws1 = WorkspaceService::create_workspace(
                WorkspaceName::new("unique-1".into()).unwrap(),
                WorkspacePath::new("/tmp/unique-1".into()).unwrap(),
            )
            .unwrap();
            let ws2 = WorkspaceService::create_workspace(
                WorkspaceName::new("unique-2".into()).unwrap(),
                WorkspacePath::new("/tmp/unique-2".into()).unwrap(),
            )
            .unwrap();
            assert_ne!(ws1.id.as_str(), ws2.id.as_str());
        }

        #[test]
        fn create_workspace_sets_created_at_and_updated_at_equal() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("timestamps".into()).unwrap(),
                WorkspacePath::new("/tmp/timestamps".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.created_at, ws.updated_at);
        }

        #[test]
        fn create_workspace_has_no_lock_holder() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("no-lock".into()).unwrap(),
                WorkspacePath::new("/tmp/no-lock".into()).unwrap(),
            )
            .unwrap();
            assert!(ws.lock_holder().is_none());
        }

        #[test]
        fn create_workspace_has_default_config() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("config-check".into()).unwrap(),
                WorkspacePath::new("/tmp/config-check".into()).unwrap(),
            )
            .unwrap();
            let config = ws.config().expect("should have config");
            assert_eq!(config.default_branch, "main");
            assert!(config.auto_sync);
        }

        #[test]
        fn create_workspace_preserves_name_exactly() {
            let name_cases = vec!["simple", "with-dash", "with_underscore", "WithCaps123"];
            for name in name_cases {
                let ws = WorkspaceService::create_workspace(
                    WorkspaceName::new(name.into()).unwrap(),
                    WorkspacePath::new("/tmp/test".into()).unwrap(),
                )
                .unwrap();
                assert_eq!(ws.name.as_str(), name);
            }
        }

        #[test]
        fn create_workspace_accepts_root_path() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("root-path".into()).unwrap(),
                WorkspacePath::new("/".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.state, WorkspaceState::Initializing);
        }

        // ── Empty name rejection ─────────────────────────────────────────────────
        // Note: WorkspaceService::create_workspace takes WorkspaceName directly,
        // so invalid names fail at value object construction BEFORE calling the service.
        // These tests verify the value object rejects empty names.

        #[test]
        fn workspace_name_empty_is_rejected() {
            let name_result = WorkspaceName::new("".into());
            assert!(name_result.is_err());
            match name_result.err().unwrap() {
                WorkspaceError::InvalidWorkspaceName(msg) => {
                    assert!(msg.contains("empty"));
                }
                other => panic!("expected InvalidWorkspaceName, got {:?}", other),
            }
        }

        // ── Name too long rejection ──────────────────────────────────────────────

        #[test]
        fn create_workspace_rejects_name_too_long() {
            let long_name = "a".repeat(256);
            let result = WorkspaceName::new(long_name.into());
            assert!(result.is_err());
        }

        #[test]
        fn create_workspace_accepts_name_at_max_length() {
            let max_name = "a".repeat(255);
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new(max_name.into()).unwrap(),
                WorkspacePath::new("/tmp/test".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.name.as_str().len(), 255);
        }

        // ── Name with invalid characters ────────────────────────────────────────

        #[test]
        fn create_workspace_rejects_name_with_space() {
            let result = WorkspaceName::new("my workspace".into());
            assert!(result.is_err());
        }

        #[test]
        fn create_workspace_rejects_name_with_dot() {
            let result = WorkspaceName::new("my.workspace".into());
            assert!(result.is_err());
        }

        #[test]
        fn create_workspace_rejects_name_with_slash() {
            let result = WorkspaceName::new("my/workspace".into());
            assert!(result.is_err());
        }

        #[test]
        fn create_workspace_rejects_name_with_colon() {
            let result = WorkspaceName::new("my:workspace".into());
            assert!(result.is_err());
        }

        #[test]
        fn create_workspace_rejects_name_with_backslash() {
            let result = WorkspaceName::new("my\\workspace".into());
            assert!(result.is_err());
        }

        #[test]
        fn create_workspace_rejects_name_with_tab() {
            let result = WorkspaceName::new("my\tworkspace".into());
            assert!(result.is_err());
        }

        #[test]
        fn create_workspace_rejects_name_with_newline() {
            let result = WorkspaceName::new("my\nworkspace".into());
            assert!(result.is_err());
        }

        #[test]
        fn create_workspace_accepts_name_with_hyphen_and_underscore() {
            let valid_names = vec![
                "a-b",
                "a_b",
                "a-b-c",
                "a_b_c",
                "-leading",
                "trailing-",
                "_leading",
                "trailing_",
            ];
            for name in valid_names {
                let result = WorkspaceService::create_workspace(
                    WorkspaceName::new(name.into()).unwrap(),
                    WorkspacePath::new("/tmp/test".into()).unwrap(),
                );
                assert!(result.is_ok(), "should accept: {}", name);
            }
        }

        // ── Empty path rejection ────────────────────────────────────────────────

        #[test]
        fn create_workspace_rejects_empty_path() {
            let result = WorkspacePath::new("".into());
            assert!(result.is_err());
        }

        #[test]
        fn create_workspace_rejects_empty_path_error_type() {
            let result = WorkspacePath::new("".into());
            match result.err().unwrap() {
                WorkspaceError::InvalidWorkspacePath(msg) => {
                    assert!(msg.contains("empty"));
                }
                other => panic!("expected InvalidWorkspacePath, got {:?}", other),
            }
        }

        // ── Path traversal behavior ────────────────────────────────────────────

        #[test]
        fn create_workspace_accepts_path_with_dot_segments() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("dot-path".into()).unwrap(),
                WorkspacePath::new("/tmp/./subdir/../other".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.state, WorkspaceState::Initializing);
        }

        #[test]
        fn create_workspace_accepts_relative_path() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("relative-path".into()).unwrap(),
                WorkspacePath::new("relative/workspace".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.state, WorkspaceState::Initializing);
            assert!(ws.path.as_path().is_absolute());
        }

        // ── Path existence note ────────────────────────────────────────────────
        // WorkspacePath does NOT validate path existence - non-existent paths
        // are accepted. This is by design: the path may not exist yet when
        // the workspace is created, and will be validated at activation time.

        #[test]
        fn create_workspace_accepts_nonexistent_path() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("nonexistent".into()).unwrap(),
                WorkspacePath::new("/tmp/this-path-does-not-exist-xyz123".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.state, WorkspaceState::Initializing);
            assert!(!ws.path.exists());
        }

        // ── Multiple workspaces with same path (note: stateless service) ─────────
        // The WorkspaceService::create_workspace is stateless and does NOT
        // check for duplicate names or path conflicts. This check must be
        // done at a higher application layer that manages workspace collections.

        #[test]
        fn create_workspace_allows_duplicate_names_different_ids() {
            let ws1 = WorkspaceService::create_workspace(
                WorkspaceName::new("same-name".into()).unwrap(),
                WorkspacePath::new("/tmp/path-1".into()).unwrap(),
            )
            .unwrap();
            let ws2 = WorkspaceService::create_workspace(
                WorkspaceName::new("same-name".into()).unwrap(),
                WorkspacePath::new("/tmp/path-2".into()).unwrap(),
            )
            .unwrap();
            // Both succeed because the service is stateless
            assert_ne!(ws1.id.as_str(), ws2.id.as_str());
            assert_eq!(ws1.name.as_str(), ws2.name.as_str());
        }

        #[test]
        fn create_workspace_allows_same_path_different_names() {
            let ws1 = WorkspaceService::create_workspace(
                WorkspaceName::new("name-1".into()).unwrap(),
                WorkspacePath::new("/tmp/same-path".into()).unwrap(),
            )
            .unwrap();
            let ws2 = WorkspaceService::create_workspace(
                WorkspaceName::new("name-2".into()).unwrap(),
                WorkspacePath::new("/tmp/same-path".into()).unwrap(),
            )
            .unwrap();
            // Both succeed - path conflict detection is at repository level
            assert_ne!(ws1.name.as_str(), ws2.name.as_str());
        }

        // ── Single character and edge case names ────────────────────────────────

        #[test]
        fn create_workspace_accepts_single_char_name() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("a".into()).unwrap(),
                WorkspacePath::new("/tmp/test".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.name.as_str(), "a");
        }

        #[test]
        fn create_workspace_accepts_numeric_name() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("12345".into()).unwrap(),
                WorkspacePath::new("/tmp/test".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.name.as_str(), "12345");
        }

        // ── Timestamp behavior ─────────────────────────────────────────────────

        #[test]
        fn create_workspace_timestamps_are_chronological() {
            let ws1 = WorkspaceService::create_workspace(
                WorkspaceName::new("first".into()).unwrap(),
                WorkspacePath::new("/tmp/first".into()).unwrap(),
            )
            .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1));
            let ws2 = WorkspaceService::create_workspace(
                WorkspaceName::new("second".into()).unwrap(),
                WorkspacePath::new("/tmp/second".into()).unwrap(),
            )
            .unwrap();
            assert!(ws2.created_at >= ws1.created_at);
        }

        // ── Config preservation ───────────────────────────────────────────────

        #[test]
        fn create_workspace_config_has_git_vcs_type() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("vcs-check".into()).unwrap(),
                WorkspacePath::new("/tmp/vcs-check".into()).unwrap(),
            )
            .unwrap();
            let config = ws.config().expect("should have config");
            assert_eq!(config.vcs_type, VcsType::Git);
        }

        // ── State machine integration ─────────────────────────────────────────

        #[test]
        fn create_workspace_state_can_be_activated() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("to-activate".into()).unwrap(),
                WorkspacePath::new("/tmp/to-activate".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.state, WorkspaceState::Initializing);
            let activated = WorkspaceService::initialize_workspace(ws).unwrap();
            assert_eq!(activated.state, WorkspaceState::Active);
        }

        #[test]
        fn create_workspace_state_can_be_deleted_directly() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("to-delete".into()).unwrap(),
                WorkspacePath::new("/tmp/to-delete".into()).unwrap(),
            )
            .unwrap();
            let deleted = WorkspaceService::delete_workspace(ws).unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);
        }

        // ── ID generation uniqueness (batch) ──────────────────────────────────

        #[test]
        fn create_workspace_id_uniqueness_100_workspaces() {
            let ids: std::collections::HashSet<String> = (0..100)
                .map(|i| {
                    WorkspaceService::create_workspace(
                        WorkspaceName::new(format!("ws-{}", i)).unwrap(),
                        WorkspacePath::new(format!("/tmp/ws-{}", i)).unwrap(),
                    )
                    .unwrap()
                    .id
                    .as_str()
                    .to_string()
                })
                .collect();
            assert_eq!(ids.len(), 100);
        }

        // ── Unicode names (allowed by WorkspaceName) ────────────────────────────

        #[test]
        fn create_workspace_accepts_unicode_letters_in_name() {
            let ws = WorkspaceService::create_workspace(
                WorkspaceName::new("ワークスペース".into()).unwrap(),
                WorkspacePath::new("/tmp/unicode".into()).unwrap(),
            )
            .unwrap();
            assert_eq!(ws.name.as_str(), "ワークスペース");
        }
    }
}
