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
        let found = WorkspaceService::find_workspace(&all, &WorkspaceId::parse("nonexistent".into()).unwrap());
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
        let found = WorkspaceService::find_by_name(&all, &WorkspaceName::new("name-search".into()).unwrap());
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
        let found = WorkspaceService::find_by_name(&all, &WorkspaceName::new("ghost".into()).unwrap());
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
        let found = WorkspaceService::find_workspace(
            &[],
            &WorkspaceId::parse("any".into()).unwrap(),
        );
        assert!(found.is_none());
    }

    #[test]
    fn workspace_service_find_by_name_empty() {
        let found = WorkspaceService::find_by_name(
            &[],
            &WorkspaceName::new("any".into()).unwrap(),
        );
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
        ).unwrap();
        assert_eq!(ws.name().as_str(), "name-preserve");
    }

    #[test]
    fn workspace_service_create_preserves_path() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("path-preserve".into()).unwrap(),
            WorkspacePath::new("/tmp/path-preserve".into()).unwrap(),
        ).unwrap();
        assert!(ws.path().as_str().unwrap().contains("/tmp/path-preserve"));
    }

    #[test]
    fn workspace_service_create_generates_unique_ids() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("id-1".into()).unwrap(),
            WorkspacePath::new("/tmp/id-1".into()).unwrap(),
        ).unwrap();
        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("id-2".into()).unwrap(),
            WorkspacePath::new("/tmp/id-2".into()).unwrap(),
        ).unwrap();
        assert_ne!(ws1.id.as_str(), ws2.id.as_str());
    }

    #[test]
    fn workspace_service_initialize_preserves_name() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("init-name".into()).unwrap(),
            WorkspacePath::new("/tmp/init-name".into()).unwrap(),
        ).unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        assert_eq!(initialized.name().as_str(), "init-name");
    }

    #[test]
    fn workspace_service_lock_preserves_id() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("lock-id".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-id".into()).unwrap(),
        ).unwrap();
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
        ).unwrap();
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
        ).unwrap();
        let deleted = WorkspaceService::delete_workspace(ws).unwrap();
        assert_eq!(deleted.state, WorkspaceState::Deleted);
        assert!(deleted.is_terminal());
    }

    #[test]
    fn workspace_service_delete_active_succeeds() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("del-active2".into()).unwrap(),
            WorkspacePath::new("/tmp/del-active2".into()).unwrap(),
        ).unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        let deleted = WorkspaceService::delete_workspace(initialized).unwrap();
        assert_eq!(deleted.state, WorkspaceState::Deleted);
    }

    #[test]
    fn workspace_service_delete_locked_returns_workspace_locked_error() {
        let ws = WorkspaceService::create_workspace(
            WorkspaceName::new("del-locked".into()).unwrap(),
            WorkspacePath::new("/tmp/del-locked".into()).unwrap(),
        ).unwrap();
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
        ).unwrap();
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
        ).unwrap();
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
        ).unwrap();
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
        ).unwrap();
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
        ).unwrap();
        let active_a = WorkspaceService::initialize_workspace(ws1).unwrap();

        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("active-b".into()).unwrap(),
            WorkspacePath::new("/tmp/active-b".into()).unwrap(),
        ).unwrap();
        let active_b = WorkspaceService::initialize_workspace(ws2).unwrap();

        let ws3 = WorkspaceService::create_workspace(
            WorkspaceName::new("init-c".into()).unwrap(),
            WorkspacePath::new("/tmp/init-c".into()).unwrap(),
        ).unwrap();

        let all = vec![active_a.clone(), active_b.clone(), ws3];
        let active_list = WorkspaceService::get_active_workspaces(&all);
        assert_eq!(active_list.len(), 2);
    }

    #[test]
    fn workspace_service_get_locked_workspaces_multiple() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("lock-a".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-a".into()).unwrap(),
        ).unwrap();
        let active1 = WorkspaceService::initialize_workspace(ws1).unwrap();
        let locked1 = WorkspaceService::lock_workspace(active1, "a1".into()).unwrap();

        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("lock-b".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-b".into()).unwrap(),
        ).unwrap();
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
        ).unwrap();
        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("other".into()).unwrap(),
            WorkspacePath::new("/tmp/other".into()).unwrap(),
        ).unwrap();
        let all = vec![ws1.clone(), ws2];
        let found = WorkspaceService::find_by_name(&all, &WorkspaceName::new("dup".into()).unwrap());
        assert!(found.is_some());
        assert_eq!(found.unwrap().id.as_str(), ws1.id.as_str());
    }

    #[test]
    fn workspace_service_get_active_workspaces_no_actives() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("init-a".into()).unwrap(),
            WorkspacePath::new("/tmp/init-a".into()).unwrap(),
        ).unwrap();
        let ws2 = WorkspaceService::create_workspace(
            WorkspaceName::new("init-b".into()).unwrap(),
            WorkspacePath::new("/tmp/init-b".into()).unwrap(),
        ).unwrap();
        let all = vec![ws1, ws2];
        let active_list = WorkspaceService::get_active_workspaces(&all);
        assert!(active_list.is_empty());
    }

    #[test]
    fn workspace_service_get_locked_workspaces_no_locked() {
        let ws1 = WorkspaceService::create_workspace(
            WorkspaceName::new("active-x".into()).unwrap(),
            WorkspacePath::new("/tmp/active-x".into()).unwrap(),
        ).unwrap();
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
        ).unwrap();
        let initialized = WorkspaceService::initialize_workspace(ws).unwrap();
        assert!(initialized.is_active());

        let locked = WorkspaceService::lock_workspace(initialized, "agent-stuck".into()).unwrap();
        assert!(locked.is_locked());

        let recovered = WorkspaceService::recover_workspace(locked).unwrap();
        assert!(recovered.is_active());

        let deleted = WorkspaceService::delete_workspace(recovered).unwrap();
        assert!(deleted.is_terminal());
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use proptest::prelude::*;
        use proptest::{prop_assert, prop_assert_eq};
        use super::*;

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
}
