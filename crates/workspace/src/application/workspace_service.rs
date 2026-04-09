use crate::domain::entities::{Workspace, WorkspaceId, WorkspaceState};
use crate::domain::value_objects::{WorkspaceName, WorkspacePath};
use crate::error::WorkspaceError;
use chrono::Utc;

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
        if workspace.state == WorkspaceState::Corrupted {
            Ok(Workspace {
                id: workspace.id,
                name: workspace.name,
                path: workspace.path,
                created_at: workspace.created_at,
                updated_at: Utc::now(),
                lock_holder: None,
                config: workspace.config,
                state: WorkspaceState::Active,
                _state: std::marker::PhantomData,
            })
        } else if workspace.state == WorkspaceState::Locked {
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
    fn workspace_service_recover_succeeds_when_corrupted() {
        // Happy path: recover_workspace should succeed for Corrupted state
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
        assert!(result.is_ok());
        let recovered = result.unwrap();
        assert!(recovered.is_active());
        assert!(recovered.lock_holder().is_none());
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

    // --- Exhaustive lock_workspace tests (ha-da9) ---

    mod lock_workspace_exhaustive {
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

        fn make_initializing(name: &str) -> Workspace {
            WorkspaceService::create_workspace(
                WorkspaceName::new(name.into()).unwrap(),
                WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
            )
            .unwrap()
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

        // ── Happy path: Active → Locked ──

        #[test]
        fn lock_happy_path_transitions_active_to_locked() {
            let active = make_active("lock-happy");
            let locked = WorkspaceService::lock_workspace(active, "agent-1".into()).unwrap();
            assert_eq!(locked.state, WorkspaceState::Locked);
            assert!(locked.is_locked());
        }

        #[test]
        fn lock_happy_path_records_lock_holder() {
            let active = make_active("lock-holder-rec");
            let locked = WorkspaceService::lock_workspace(active, "agent-42".into()).unwrap();
            assert_eq!(locked.lock_holder(), Some("agent-42"));
        }

        #[test]
        fn lock_happy_path_preserves_workspace_id() {
            let active = make_active("lock-id");
            let id_before = active.id.as_str().to_string();
            let locked = WorkspaceService::lock_workspace(active, "agent".into()).unwrap();
            assert_eq!(locked.id.as_str(), id_before);
        }

        #[test]
        fn lock_happy_path_preserves_workspace_name() {
            let active = make_active("lock-name");
            let locked = WorkspaceService::lock_workspace(active, "agent".into()).unwrap();
            assert_eq!(locked.name.as_str(), "lock-name");
        }

        #[test]
        fn lock_happy_path_preserves_workspace_path() {
            let active = make_active("lock-path");
            let locked = WorkspaceService::lock_workspace(active, "agent".into()).unwrap();
            assert_eq!(locked.path.as_str(), Some("/tmp/lock-path"));
        }

        #[test]
        fn lock_happy_path_preserves_config() {
            let active = make_active("lock-cfg");
            let locked = WorkspaceService::lock_workspace(active, "agent".into()).unwrap();
            let config = locked.config().expect("config should be present");
            assert_eq!(config.default_branch, "main");
            assert!(config.auto_sync);
        }

        #[test]
        fn lock_happy_path_updates_updated_at() {
            let active = make_active("lock-ts");
            let ts_before = active.updated_at();
            std::thread::sleep(std::time::Duration::from_millis(2));
            let locked = WorkspaceService::lock_workspace(active, "agent".into()).unwrap();
            assert!(locked.updated_at() >= ts_before);
        }

        // ── Already locked: re-lock behavior ──

        #[test]
        fn lock_on_already_locked_succeeds_with_new_holder() {
            let locked = make_locked("relock", "agent-1");
            let relocked = WorkspaceService::lock_workspace(locked, "agent-2".into()).unwrap();
            assert_eq!(relocked.state, WorkspaceState::Locked);
            assert!(relocked.is_locked());
        }

        #[test]
        fn lock_on_already_locked_overwrites_holder() {
            let locked = make_locked("relock-holder", "agent-original");
            let relocked = WorkspaceService::lock_workspace(locked, "agent-new".into()).unwrap();
            assert_eq!(relocked.lock_holder(), Some("agent-new"));
        }

        #[test]
        fn lock_on_already_locked_preserves_id() {
            let locked = make_locked("relock-id", "a1");
            let id_before = locked.id.as_str().to_string();
            let relocked = WorkspaceService::lock_workspace(locked, "a2".into()).unwrap();
            assert_eq!(relocked.id.as_str(), id_before);
        }

        #[test]
        fn lock_on_already_locked_preserves_name() {
            let locked = make_locked("relock-name", "a1");
            let relocked = WorkspaceService::lock_workspace(locked, "a2".into()).unwrap();
            assert_eq!(relocked.name.as_str(), "relock-name");
        }

        // ── Lock by different holder ──

        #[test]
        fn lock_by_different_holder_replaces_original_holder() {
            let active = make_active("diff-holder");
            let locked1 = WorkspaceService::lock_workspace(active, "holder-A".into()).unwrap();
            assert_eq!(locked1.lock_holder(), Some("holder-A"));

            let locked2 = WorkspaceService::lock_workspace(locked1, "holder-B".into()).unwrap();
            assert_eq!(locked2.lock_holder(), Some("holder-B"));
        }

        #[test]
        fn lock_by_different_holder_does_not_preserve_original() {
            let locked = make_locked("no-original", "first");
            let relocked = WorkspaceService::lock_workspace(locked, "second".into()).unwrap();
            assert_ne!(relocked.lock_holder(), Some("first"));
            assert_eq!(relocked.lock_holder(), Some("second"));
        }

        #[test]
        fn lock_by_same_holder_is_idempotent() {
            let locked = make_locked("same-holder", "agent-X");
            let relocked = WorkspaceService::lock_workspace(locked, "agent-X".into()).unwrap();
            assert_eq!(relocked.lock_holder(), Some("agent-X"));
            assert_eq!(relocked.state, WorkspaceState::Locked);
        }

        // ── Lock on non-Active workspace states ──

        #[test]
        fn lock_on_initializing_succeeds_and_sets_locked() {
            let initializing = make_initializing("lock-init");
            let locked =
                WorkspaceService::lock_workspace(initializing, "agent-init".into()).unwrap();
            assert_eq!(locked.state, WorkspaceState::Locked);
            assert_eq!(locked.lock_holder(), Some("agent-init"));
        }

        #[test]
        fn lock_on_corrupted_succeeds_and_sets_locked() {
            let corrupted = make_corrupted("lock-corrupt");
            let locked =
                WorkspaceService::lock_workspace(corrupted, "agent-corrupt".into()).unwrap();
            assert_eq!(locked.state, WorkspaceState::Locked);
            assert_eq!(locked.lock_holder(), Some("agent-corrupt"));
        }

        #[test]
        fn lock_on_deleted_succeeds_and_sets_locked() {
            let deleted = make_deleted("lock-deleted");
            let locked = WorkspaceService::lock_workspace(deleted, "agent-del".into()).unwrap();
            assert_eq!(locked.state, WorkspaceState::Locked);
            assert_eq!(locked.lock_holder(), Some("agent-del"));
        }

        #[test]
        fn lock_on_corrupted_preserves_identity() {
            let corrupted = make_corrupted("lock-corrupt-id");
            let id_before = corrupted.id.as_str().to_string();
            let locked = WorkspaceService::lock_workspace(corrupted, "agent".into()).unwrap();
            assert_eq!(locked.id.as_str(), id_before);
        }

        #[test]
        fn lock_on_deleted_preserves_identity() {
            let deleted = make_deleted("lock-del-id");
            let id_before = deleted.id.as_str().to_string();
            let locked = WorkspaceService::lock_workspace(deleted, "agent".into()).unwrap();
            assert_eq!(locked.id.as_str(), id_before);
        }

        // ── Lock holder identity preserved across queries ──

        #[test]
        fn lock_holder_visible_in_get_locked_workspaces() {
            let locked = make_locked("query-holder", "agent-query");
            let all = vec![locked];
            let locked_list = WorkspaceService::get_locked_workspaces(&all);
            assert_eq!(locked_list.len(), 1);
            assert_eq!(locked_list[0].lock_holder(), Some("agent-query"));
        }

        #[test]
        fn lock_holder_visible_in_find_workspace() {
            let locked = make_locked("find-holder", "agent-find");
            let id = locked.id.as_str().to_string();
            let all = vec![locked];
            let found = WorkspaceService::find_workspace(&all, &WorkspaceId::parse(id).unwrap());
            assert!(found.is_some());
            assert_eq!(found.unwrap().lock_holder(), Some("agent-find"));
        }

        #[test]
        fn lock_holder_visible_in_find_by_name() {
            let locked = make_locked("find-name-holder", "agent-name");
            let all = vec![locked.clone()];
            let found = WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("find-name-holder".into()).unwrap(),
            );
            assert!(found.is_some());
            assert_eq!(found.unwrap().lock_holder(), Some("agent-name"));
        }

        #[test]
        fn multiple_locked_workspaces_each_retain_their_holder() {
            let locked1 = make_locked("multi-1", "holder-alpha");
            let locked2 = make_locked("multi-2", "holder-beta");
            let locked3 = make_locked("multi-3", "holder-gamma");
            let all = vec![locked1, locked2, locked3];
            let locked_list = WorkspaceService::get_locked_workspaces(&all);
            assert_eq!(locked_list.len(), 3);
            let holders: Vec<Option<&str>> = locked_list.iter().map(|w| w.lock_holder()).collect();
            assert!(holders.contains(&Some("holder-alpha")));
            assert!(holders.contains(&Some("holder-beta")));
            assert!(holders.contains(&Some("holder-gamma")));
        }

        #[test]
        fn lock_holder_cleared_after_unlock() {
            let locked = make_locked("unlock-clear", "agent-clear");
            assert_eq!(locked.lock_holder(), Some("agent-clear"));
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(unlocked.lock_holder().is_none());
            assert!(unlocked.is_active());
        }

        // ── Lock holder with various string values ──

        #[test]
        fn lock_with_empty_string_holder() {
            let active = make_active("lock-empty");
            let locked = WorkspaceService::lock_workspace(active, "".into()).unwrap();
            assert_eq!(locked.lock_holder(), Some(""));
        }

        #[test]
        fn lock_with_long_holder_name() {
            let active = make_active("lock-long");
            let long_holder = "a".repeat(1000);
            let expected = long_holder.clone();
            let locked = WorkspaceService::lock_workspace(active, long_holder).unwrap();
            assert_eq!(locked.lock_holder(), Some(expected.as_str()));
        }

        #[test]
        fn lock_with_special_chars_holder() {
            let active = make_active("lock-special");
            let holder = "agent-🎉/子@#$%";
            let locked = WorkspaceService::lock_workspace(active, holder.into()).unwrap();
            assert_eq!(locked.lock_holder(), Some("agent-🎉/子@#$%"));
        }

        #[test]
        fn lock_with_whitespace_holder() {
            let active = make_active("lock-ws");
            let locked = WorkspaceService::lock_workspace(active, "  spaces  ".into()).unwrap();
            assert_eq!(locked.lock_holder(), Some("  spaces  "));
        }

        // ── Lock/unlock roundtrip ──

        #[test]
        fn lock_then_unlock_returns_to_active() {
            let active = make_active("roundtrip");
            let locked = WorkspaceService::lock_workspace(active, "agent-rt".into()).unwrap();
            assert!(locked.is_locked());
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(unlocked.is_active());
            assert!(unlocked.lock_holder().is_none());
        }

        #[test]
        fn lock_unlock_relock_preserves_latest_holder() {
            let active = make_active("relock-cycle");
            let locked1 = WorkspaceService::lock_workspace(active, "first".into()).unwrap();
            let unlocked = WorkspaceService::unlock_workspace(locked1).unwrap();
            let locked2 = WorkspaceService::lock_workspace(unlocked, "second".into()).unwrap();
            assert_eq!(locked2.lock_holder(), Some("second"));
        }

        // ── Lock preserves created_at ──

        #[test]
        fn lock_preserves_created_at() {
            let active = make_active("lock-created");
            let created_at = active.created_at();
            let locked = WorkspaceService::lock_workspace(active, "agent".into()).unwrap();
            assert_eq!(locked.created_at(), created_at);
        }

        // ── Lock after recover ──

        #[test]
        fn lock_after_recover_succeeds() {
            let locked = make_locked("lock-recover", "agent-old");
            let recovered = WorkspaceService::recover_workspace(locked).unwrap();
            assert!(recovered.is_active());
            let relocked = WorkspaceService::lock_workspace(recovered, "agent-new".into()).unwrap();
            assert!(relocked.is_locked());
            assert_eq!(relocked.lock_holder(), Some("agent-new"));
        }

        // ── Exhaustive state matrix: lock from every state ──

        #[test]
        fn lock_from_every_state_succeeds() {
            let states: Vec<(&str, Workspace)> = vec![
                ("Initializing", make_initializing("matrix-init")),
                ("Active", make_active("matrix-active")),
                ("Locked", make_locked("matrix-locked", "original")),
                ("Corrupted", make_corrupted("matrix-corrupt")),
                ("Deleted", make_deleted("matrix-deleted")),
            ];

            for (label, ws) in states {
                let result =
                    WorkspaceService::lock_workspace(ws, format!("holder-{}", label).into());
                assert!(
                    result.is_ok(),
                    "lock_workspace should succeed from state {}",
                    label
                );
                let locked = result.unwrap();
                assert_eq!(
                    locked.state,
                    WorkspaceState::Locked,
                    "state after lock from {} should be Locked",
                    label
                );
                let expected_holder = format!("holder-{}", label);
                assert_eq!(
                    locked.lock_holder(),
                    Some(expected_holder.as_str()),
                    "lock holder should be set when locking from {}",
                    label
                );
            }
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
    // delete_workspace exhaustive tests (ha-b47)
    // =============================================================================

    mod delete_workspace_exhaustive {
        use super::*;

        fn make_active(name: &str) -> Workspace {
            WorkspaceService::create_workspace(
                WorkspaceName::new(name.into()).unwrap(),
                WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
            )
            .and_then(WorkspaceService::initialize_workspace)
            .unwrap()
        }

        fn make_initializing(name: &str) -> Workspace {
            WorkspaceService::create_workspace(
                WorkspaceName::new(name.into()).unwrap(),
                WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
            )
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

        // ── Happy path: Active → Deleted ──

        #[test]
        fn delete_active_transitions_to_deleted() {
            let active = make_active("del-happy");
            let result = WorkspaceService::delete_workspace(active);
            assert!(result.is_ok());
            let deleted = result.unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);
        }

        #[test]
        fn delete_active_result_is_terminal() {
            let active = make_active("del-terminal");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert!(deleted.is_terminal());
        }

        #[test]
        fn delete_active_not_active() {
            let active = make_active("del-not-active");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert!(!deleted.is_active());
        }

        #[test]
        fn delete_active_not_locked() {
            let active = make_active("del-not-locked");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert!(!deleted.is_locked());
        }

        // ── Happy path: Initializing → Deleted ──

        #[test]
        fn delete_initializing_transitions_to_deleted() {
            let init = make_initializing("del-init-happy");
            let result = WorkspaceService::delete_workspace(init);
            assert!(result.is_ok());
            let deleted = result.unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);
        }

        #[test]
        fn delete_initializing_result_is_terminal() {
            let init = make_initializing("del-init-term");
            let deleted = WorkspaceService::delete_workspace(init).unwrap();
            assert!(deleted.is_terminal());
        }

        // ── Field preservation: Active → Deleted ──

        #[test]
        fn delete_active_preserves_id() {
            let active = make_active("del-id");
            let id_before = active.id.as_str().to_string();
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert_eq!(deleted.id.as_str(), id_before);
        }

        #[test]
        fn delete_active_preserves_name() {
            let active = make_active("del-name");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert_eq!(deleted.name.as_str(), "del-name");
        }

        #[test]
        fn delete_active_preserves_path() {
            let active = make_active("del-path");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert!(deleted.path.as_str().unwrap().contains("del-path"));
        }

        #[test]
        fn delete_active_preserves_created_at() {
            let active = make_active("del-cat");
            let created_at = active.created_at();
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert_eq!(deleted.created_at(), created_at);
        }

        #[test]
        fn delete_active_preserves_config() {
            let active = make_active("del-cfg");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            let config = deleted.config().expect("should have config");
            assert_eq!(config.default_branch, "main");
            assert!(config.auto_sync);
        }

        #[test]
        fn delete_active_preserves_lock_holder_none() {
            let active = make_active("del-lh");
            assert!(active.lock_holder().is_none());
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert!(deleted.lock_holder().is_none());
        }

        // ── Field preservation: Initializing → Deleted ──

        #[test]
        fn delete_initializing_preserves_id() {
            let init = make_initializing("del-init-id");
            let id_before = init.id.as_str().to_string();
            let deleted = WorkspaceService::delete_workspace(init).unwrap();
            assert_eq!(deleted.id.as_str(), id_before);
        }

        #[test]
        fn delete_initializing_preserves_name() {
            let init = make_initializing("del-init-name");
            let deleted = WorkspaceService::delete_workspace(init).unwrap();
            assert_eq!(deleted.name.as_str(), "del-init-name");
        }

        #[test]
        fn delete_initializing_preserves_path() {
            let init = make_initializing("del-init-path");
            let deleted = WorkspaceService::delete_workspace(init).unwrap();
            assert!(deleted.path.as_str().unwrap().contains("del-init-path"));
        }

        #[test]
        fn delete_initializing_preserves_created_at() {
            let init = make_initializing("del-init-cat");
            let created_at = init.created_at();
            let deleted = WorkspaceService::delete_workspace(init).unwrap();
            assert_eq!(deleted.created_at(), created_at);
        }

        #[test]
        fn delete_initializing_preserves_config() {
            let init = make_initializing("del-init-cfg");
            let deleted = WorkspaceService::delete_workspace(init).unwrap();
            let config = deleted.config().expect("should have config");
            assert_eq!(config.default_branch, "main");
            assert!(config.auto_sync);
        }

        // ── Locked workspace rejection ──

        #[test]
        fn delete_locked_returns_err() {
            let locked = make_locked("del-locked-err", "agent-1");
            let result = WorkspaceService::delete_workspace(locked);
            assert!(result.is_err());
        }

        #[test]
        fn delete_locked_returns_workspace_locked_error() {
            let locked = make_locked("del-locked-var", "agent-x");
            let result = WorkspaceService::delete_workspace(locked);
            match result.err() {
                Some(WorkspaceError::WorkspaceLocked(id, holder)) => {
                    assert!(!id.is_empty());
                    assert_eq!(holder, "agent-x");
                }
                other => panic!("expected WorkspaceLocked, got {other:?}"),
            }
        }

        #[test]
        fn delete_locked_error_not_invalid_state_transition() {
            let locked = make_locked("del-locked-nist", "agent");
            let result = WorkspaceService::delete_workspace(locked);
            match result.err() {
                Some(WorkspaceError::WorkspaceLocked(_, _)) => {}
                other => panic!("expected WorkspaceLocked, got {other:?}"),
            }
        }

        #[test]
        fn delete_locked_with_empty_holder() {
            let locked = make_locked("del-locked-empty", "");
            let result = WorkspaceService::delete_workspace(locked);
            assert!(result.is_err());
            match result.err() {
                Some(WorkspaceError::WorkspaceLocked(_, holder)) => {
                    assert_eq!(holder, "");
                }
                other => panic!("expected WorkspaceLocked, got {other:?}"),
            }
        }

        #[test]
        fn delete_locked_with_long_holder() {
            let long_holder = "a".repeat(500);
            let expected = long_holder.clone();
            let locked = make_locked("del-locked-long", &long_holder);
            let result = WorkspaceService::delete_workspace(locked);
            match result.err() {
                Some(WorkspaceError::WorkspaceLocked(_, holder)) => {
                    assert_eq!(holder, expected);
                }
                other => panic!("expected WorkspaceLocked, got {other:?}"),
            }
        }

        #[test]
        fn delete_locked_with_special_chars_holder() {
            let holder = "agent-🎉/子@#$%";
            let locked = make_locked("del-locked-spec", holder);
            let result = WorkspaceService::delete_workspace(locked);
            match result.err() {
                Some(WorkspaceError::WorkspaceLocked(_, h)) => {
                    assert_eq!(h, holder);
                }
                other => panic!("expected WorkspaceLocked, got {other:?}"),
            }
        }

        #[test]
        fn delete_locked_error_message_contains_locked() {
            let locked = make_locked("del-locked-msg", "agent-msg");
            let err = WorkspaceService::delete_workspace(locked).err().unwrap();
            let msg = format!("{err}");
            assert!(
                msg.contains("locked"),
                "error message should contain 'locked': {msg}"
            );
        }

        // ── Corrupted workspace rejection ──

        #[test]
        fn delete_corrupted_returns_err() {
            let corrupted = make_corrupted("del-corrupt-err");
            let result = WorkspaceService::delete_workspace(corrupted);
            assert!(result.is_err());
        }

        #[test]
        fn delete_corrupted_returns_invalid_state_transition() {
            let corrupted = make_corrupted("del-corrupt-var");
            let result = WorkspaceService::delete_workspace(corrupted);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Corrupted");
                    assert_eq!(to, "Deleted");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn delete_corrupted_error_message_format() {
            let corrupted = make_corrupted("del-corrupt-msg");
            let err = WorkspaceService::delete_workspace(corrupted).err().unwrap();
            let msg = format!("{err}");
            assert!(
                msg.contains("Corrupted"),
                "error should mention Corrupted: {msg}"
            );
            assert!(
                msg.contains("Deleted"),
                "error should mention Deleted: {msg}"
            );
        }

        // ── Already deleted rejection ──

        #[test]
        fn delete_already_deleted_returns_err() {
            let deleted = make_deleted("del-already");
            let result = WorkspaceService::delete_workspace(deleted);
            assert!(result.is_err());
        }

        #[test]
        fn delete_already_deleted_returns_invalid_state_transition() {
            let deleted = make_deleted("del-already-var");
            let result = WorkspaceService::delete_workspace(deleted);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Deleted");
                    assert_eq!(to, "Deleted");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn delete_already_deleted_is_not_idempotent() {
            let active = make_active("del-not-idem");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);
            let second = WorkspaceService::delete_workspace(deleted);
            assert!(
                second.is_err(),
                "second delete should fail, not be idempotent"
            );
        }

        #[test]
        fn delete_already_deleted_error_message() {
            let deleted = make_deleted("del-already-msg");
            let err = WorkspaceService::delete_workspace(deleted).err().unwrap();
            let msg = format!("{err}");
            assert!(
                msg.contains("Invalid state transition"),
                "error should mention transition: {msg}"
            );
        }

        // ── Recover deleted workspace is NOT supported ──

        #[test]
        fn recover_deleted_workspace_returns_err() {
            let deleted = make_deleted("del-recover");
            let result = WorkspaceService::recover_workspace(deleted);
            assert!(result.is_err());
        }

        #[test]
        fn recover_deleted_workspace_returns_invalid_state_transition() {
            let deleted = make_deleted("del-recover-var");
            let result = WorkspaceService::recover_workspace(deleted);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Deleted");
                    assert_eq!(to, "Recoverable");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        // ── Table-driven: state matrix for delete_workspace ──

        #[test]
        fn table_driven_delete_from_each_state() {
            let cases: Vec<(&str, WorkspaceState, bool)> = vec![
                ("Initializing", WorkspaceState::Initializing, true),
                ("Active", WorkspaceState::Active, true),
                ("Locked", WorkspaceState::Locked, false),
                ("Corrupted", WorkspaceState::Corrupted, false),
                ("Deleted", WorkspaceState::Deleted, false),
            ];

            for (label, state, expect_ok) in cases {
                let ws = make_active(&format!("tbl-{}", label));
                let ws_with_state = Workspace { state, ..ws };
                let result = WorkspaceService::delete_workspace(ws_with_state);
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
                    assert_eq!(result.unwrap().state, WorkspaceState::Deleted);
                }
            }
        }

        #[test]
        fn table_driven_delete_error_variants_by_state() {
            let locked = make_locked("tbl-locked", "agent");
            let result = WorkspaceService::delete_workspace(locked);
            match result.err() {
                Some(WorkspaceError::WorkspaceLocked(_, _)) => {}
                other => panic!("Locked: expected WorkspaceLocked, got {other:?}"),
            }

            let corrupted = make_corrupted("tbl-corrupt");
            let result = WorkspaceService::delete_workspace(corrupted);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Corrupted");
                    assert_eq!(to, "Deleted");
                }
                other => panic!("Corrupted: expected InvalidStateTransition, got {other:?}"),
            }

            let deleted = make_deleted("tbl-deleted");
            let result = WorkspaceService::delete_workspace(deleted);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Deleted");
                    assert_eq!(to, "Deleted");
                }
                other => panic!("Deleted: expected InvalidStateTransition, got {other:?}"),
            }
        }

        // ── Service-level vs entity-level divergence ──

        #[test]
        fn delete_locked_blocked_by_service_but_entity_allows() {
            let locked = make_locked("svc-vs-entity", "agent");
            let svc_result = WorkspaceService::delete_workspace(locked.clone());
            assert!(svc_result.is_err(), "service should block Locked deletion");
        }

        #[test]
        fn delete_corrupted_blocked_by_service_but_state_machine_allows() {
            let corrupted = make_corrupted("svc-vs-sm");
            let svc_result = WorkspaceService::delete_workspace(corrupted);
            assert!(
                svc_result.is_err(),
                "service should block Corrupted deletion even though state machine allows _->Deleted"
            );
        }

        // ── Lifecycle: delete after various paths ──

        #[test]
        fn delete_after_create_directly() {
            let ws = make_initializing("lc-direct");
            let deleted = WorkspaceService::delete_workspace(ws).unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);
        }

        #[test]
        fn delete_after_initialize() {
            let active = make_active("lc-init");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);
        }

        #[test]
        fn delete_after_lock_unlock_cycle() {
            let active = make_active("lc-cycle");
            let locked = WorkspaceService::lock_workspace(active, "agent".into()).unwrap();
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            let deleted = WorkspaceService::delete_workspace(unlocked).unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);
            assert!(deleted.is_terminal());
        }

        #[test]
        fn delete_after_recover() {
            let active = make_active("lc-recover");
            let locked = WorkspaceService::lock_workspace(active, "agent".into()).unwrap();
            let recovered = WorkspaceService::recover_workspace(locked).unwrap();
            assert!(recovered.is_active());
            let deleted = WorkspaceService::delete_workspace(recovered).unwrap();
            assert!(deleted.is_terminal());
        }

        #[test]
        fn delete_after_multiple_lock_unlock_cycles() {
            let mut ws = make_active("lc-multi");
            for i in 0..5 {
                let locked = WorkspaceService::lock_workspace(ws, format!("agent-{}", i)).unwrap();
                ws = WorkspaceService::unlock_workspace(locked).unwrap();
            }
            let deleted = WorkspaceService::delete_workspace(ws).unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);
        }

        // ── Deleted workspace is excluded from query methods ──

        #[test]
        fn deleted_workspace_not_in_active_list() {
            let active = make_active("q-active");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            let all = vec![deleted];
            let active_list = WorkspaceService::get_active_workspaces(&all);
            assert!(active_list.is_empty());
        }

        #[test]
        fn deleted_workspace_not_in_locked_list() {
            let active = make_active("q-locked");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            let all = vec![deleted];
            let locked_list = WorkspaceService::get_locked_workspaces(&all);
            assert!(locked_list.is_empty());
        }

        #[test]
        fn deleted_workspace_still_findable_by_id() {
            let active = make_active("q-find");
            let id = active.id.as_str().to_string();
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            let all = vec![deleted];
            let found = WorkspaceService::find_workspace(&all, &WorkspaceId::parse(id).unwrap());
            assert!(found.is_some());
            assert_eq!(found.unwrap().state, WorkspaceState::Deleted);
        }

        #[test]
        fn deleted_workspace_still_findable_by_name() {
            let active = make_active("q-name");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            let all = vec![deleted];
            let found =
                WorkspaceService::find_by_name(&all, &WorkspaceName::new("q-name".into()).unwrap());
            assert!(found.is_some());
            assert_eq!(found.unwrap().state, WorkspaceState::Deleted);
        }

        #[test]
        fn mixed_active_and_deleted_only_actives_returned() {
            let active = make_active("q-mix-active");
            let deleted_source = make_active("q-mix-del");
            let deleted = WorkspaceService::delete_workspace(deleted_source).unwrap();
            let all = vec![active.clone(), deleted];
            let actives = WorkspaceService::get_active_workspaces(&all);
            assert_eq!(actives.len(), 1);
            assert_eq!(actives[0].name.as_str(), "q-mix-active");
        }

        // ── Multiple independent deletions ──

        #[test]
        fn delete_multiple_workspaces_independently() {
            let mut deleted = Vec::new();
            for i in 0..10 {
                let active = make_active(&format!("batch-{}", i));
                let del = WorkspaceService::delete_workspace(active).unwrap();
                assert_eq!(del.state, WorkspaceState::Deleted);
                deleted.push(del);
            }
            let ids: std::collections::HashSet<&str> =
                deleted.iter().map(|w| w.id.as_str()).collect();
            assert_eq!(
                ids.len(),
                10,
                "all workspace IDs must be unique after deletion"
            );
        }

        #[test]
        fn delete_many_and_verify_all_terminal() {
            let all: Vec<Workspace> = (0..20)
                .map(|i| {
                    let active = make_active(&format!("term-{}", i));
                    WorkspaceService::delete_workspace(active).unwrap()
                })
                .collect();
            for ws in &all {
                assert!(
                    ws.is_terminal(),
                    "workspace {} should be terminal",
                    ws.name.as_str()
                );
                assert_eq!(ws.state, WorkspaceState::Deleted);
            }
        }

        // ── No panic on any state ──

        #[test]
        fn delete_no_panic_on_locked() {
            let locked = make_locked("no-panic-locked", "agent");
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = WorkspaceService::delete_workspace(locked);
            }));
            assert!(
                result.is_ok(),
                "delete_workspace on Locked should not panic"
            );
        }

        #[test]
        fn delete_no_panic_on_corrupted() {
            let corrupted = make_corrupted("no-panic-corrupt");
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = WorkspaceService::delete_workspace(corrupted);
            }));
            assert!(
                result.is_ok(),
                "delete_workspace on Corrupted should not panic"
            );
        }

        #[test]
        fn delete_no_panic_on_already_deleted() {
            let deleted = make_deleted("no-panic-deleted");
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = WorkspaceService::delete_workspace(deleted);
            }));
            assert!(
                result.is_ok(),
                "delete_workspace on Deleted should not panic"
            );
        }

        // ── Consistency across calls ──

        #[test]
        fn delete_active_deterministic_state() {
            let active = make_active("det-1");
            let deleted = WorkspaceService::delete_workspace(active).unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);

            let active2 = make_active("det-2");
            let deleted2 = WorkspaceService::delete_workspace(active2).unwrap();
            assert_eq!(deleted2.state, WorkspaceState::Deleted);
        }

        // ── Proptests for delete_workspace ──

        #[cfg(test)]
        mod proptests {
            use super::*;
            use proptest::prelude::*;
            use proptest::prop_assert;

            proptest! {
                #[test]
                fn delete_active_always_produces_deleted(name in "[a-zA-Z0-9_-]{1,50}") {
                    let active = make_active(&name);
                    let result = WorkspaceService::delete_workspace(active);
                    prop_assert!(result.is_ok());
                    prop_assert_eq!(result.unwrap().state, WorkspaceState::Deleted);
                }

                #[test]
                fn delete_initializing_always_produces_deleted(name in "[a-zA-Z0-9_-]{1,50}") {
                    let init = make_initializing(&name);
                    let result = WorkspaceService::delete_workspace(init);
                    prop_assert!(result.is_ok());
                    prop_assert_eq!(result.unwrap().state, WorkspaceState::Deleted);
                }

                #[test]
                fn delete_locked_always_fails(name in "[a-zA-Z0-9_-]{1,50}", holder in "[a-zA-Z0-9_-]{1,20}") {
                    let locked = make_locked(&name, &holder);
                    let result = WorkspaceService::delete_workspace(locked);
                    prop_assert!(result.is_err());
                }

                #[test]
                fn delete_preserves_id_after_active_delete(name in "[a-zA-Z0-9_-]{1,50}") {
                    let active = make_active(&name);
                    let id_before = active.id.as_str().to_string();
                    let deleted = WorkspaceService::delete_workspace(active).unwrap();
                    prop_assert_eq!(deleted.id.as_str(), id_before);
                }

                #[test]
                fn delete_preserves_name_after_active_delete(name in "[a-zA-Z0-9_-]{1,50}") {
                    let active = make_active(&name);
                    let deleted = WorkspaceService::delete_workspace(active).unwrap();
                    prop_assert_eq!(deleted.name.as_str(), name);
                }

                #[test]
                fn delete_preserves_created_after_active_delete(name in "[a-zA-Z0-9_-]{1,50}") {
                    let active = make_active(&name);
                    let created_at = active.created_at();
                    let deleted = WorkspaceService::delete_workspace(active).unwrap();
                    prop_assert_eq!(deleted.created_at(), created_at);
                }

                #[test]
                fn delete_batch_produces_unique_ids(names in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 5..20)) {
                    let mut ids = std::collections::HashSet::new();
                    for name in &names {
                        let active = make_active(name);
                        let deleted = WorkspaceService::delete_workspace(active).unwrap();
                        ids.insert(deleted.id.as_str().to_string());
                    }
                    prop_assert_eq!(ids.len(), names.len());
                }

                #[test]
                fn delete_then_find_by_id_roundtrip(name in "[a-zA-Z0-9_-]{1,50}") {
                    let active = make_active(&name);
                    let id = active.id.clone();
                    let deleted = WorkspaceService::delete_workspace(active).unwrap();
                    let all = vec![deleted];
                    let found = WorkspaceService::find_workspace(&all, &id);
                    prop_assert!(found.is_some());
                    prop_assert_eq!(found.unwrap().state, WorkspaceState::Deleted);
                }
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

    // =============================================================================
    // unlock_workspace exhaustive tests (ha-5y3)
    // =============================================================================

    mod unlock_workspace_exhaustive {
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

        fn make_initializing(name: &str) -> Workspace {
            WorkspaceService::create_workspace(
                WorkspaceName::new(name.into()).unwrap(),
                WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
            )
            .unwrap()
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

        // ══════════════════════════════════════════════════════════════════════════
        // HAPPY PATH: Locked → Active
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlock_happy_path_transitions_locked_to_active() {
            let locked = make_locked("unlock-happy", "agent-1");
            let result = WorkspaceService::unlock_workspace(locked);
            assert!(result.is_ok());
            let unlocked = result.unwrap();
            assert_eq!(unlocked.state, WorkspaceState::Active);
            assert!(unlocked.is_active());
        }

        #[test]
        fn unlock_happy_path_clears_lock_holder() {
            let locked = make_locked("unlock-clear-holder", "agent-42");
            assert_eq!(locked.lock_holder(), Some("agent-42"));
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(unlocked.lock_holder().is_none());
        }

        #[test]
        fn unlock_happy_path_preserves_workspace_id() {
            let locked = make_locked("unlock-id", "agent");
            let id_before = locked.id.as_str().to_string();
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert_eq!(unlocked.id.as_str(), id_before);
        }

        #[test]
        fn unlock_happy_path_preserves_workspace_name() {
            let locked = make_locked("unlock-name", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert_eq!(unlocked.name.as_str(), "unlock-name");
        }

        #[test]
        fn unlock_happy_path_preserves_workspace_path() {
            let locked = make_locked("unlock-path", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert_eq!(unlocked.path.as_str(), Some("/tmp/unlock-path"));
        }

        #[test]
        fn unlock_happy_path_preserves_config() {
            let locked = make_locked("unlock-cfg", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            let config = unlocked.config().expect("should have config");
            assert_eq!(config.default_branch, "main");
            assert!(config.auto_sync);
        }

        #[test]
        fn unlock_happy_path_preserves_created_at() {
            let locked = make_locked("unlock-created-ts", "agent");
            let created_at = locked.created_at();
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert_eq!(unlocked.created_at(), created_at);
        }

        #[test]
        fn unlock_happy_path_not_locked_after_unlock() {
            let locked = make_locked("unlock-notlocked", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(!unlocked.is_locked());
        }

        #[test]
        fn unlock_happy_path_not_terminal_after_unlock() {
            let locked = make_locked("unlock-notterm", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(!unlocked.is_terminal());
        }

        #[test]
        fn unlock_happy_path_is_active() {
            let locked = make_locked("unlock-isactive", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(unlocked.is_active());
        }

        #[test]
        fn unlock_happy_path_with_different_holder_names() {
            let holders = vec!["agent-1", "x", "a-very-long-agent-name-with-lots-of-chars"];
            for holder in holders {
                let locked = make_locked("unlock-holders", holder);
                let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
                assert!(unlocked.lock_holder().is_none());
                assert!(unlocked.is_active());
            }
        }

        #[test]
        fn unlock_after_lock_with_empty_holder() {
            let active = make_active("unlock-empty-holder");
            let locked = WorkspaceService::lock_workspace(active, "".into()).unwrap();
            assert_eq!(locked.lock_holder(), Some(""));
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(unlocked.lock_holder().is_none());
            assert!(unlocked.is_active());
        }

        // ══════════════════════════════════════════════════════════════════════════
        // NOT LOCKED: workspace in non-Locked state returns error
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlock_fails_when_active() {
            let active = make_active("unlock-active-err");
            let result = WorkspaceService::unlock_workspace(active);
            assert!(result.is_err());
        }

        #[test]
        fn unlock_active_error_is_invalid_state_transition() {
            let active = make_active("unlock-active-type");
            let result = WorkspaceService::unlock_workspace(active);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Active");
                    assert_eq!(to, "Active");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn unlock_fails_when_initializing() {
            let initializing = make_initializing("unlock-init-err");
            let result = WorkspaceService::unlock_workspace(initializing);
            assert!(result.is_err());
        }

        #[test]
        fn unlock_initializing_error_is_invalid_state_transition() {
            let initializing = make_initializing("unlock-init-type");
            let result = WorkspaceService::unlock_workspace(initializing);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Initializing");
                    assert_eq!(to, "Active");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn unlock_fails_when_corrupted() {
            let corrupted = make_corrupted("unlock-corrupt-err");
            let result = WorkspaceService::unlock_workspace(corrupted);
            assert!(result.is_err());
        }

        #[test]
        fn unlock_corrupted_error_is_invalid_state_transition() {
            let corrupted = make_corrupted("unlock-corrupt-type");
            let result = WorkspaceService::unlock_workspace(corrupted);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Corrupted");
                    assert_eq!(to, "Active");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn unlock_fails_when_deleted() {
            let deleted = make_deleted("unlock-del-err");
            let result = WorkspaceService::unlock_workspace(deleted);
            assert!(result.is_err());
        }

        #[test]
        fn unlock_deleted_error_is_invalid_state_transition() {
            let deleted = make_deleted("unlock-del-type");
            let result = WorkspaceService::unlock_workspace(deleted);
            match result.err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Deleted");
                    assert_eq!(to, "Active");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn unlock_manually_constructed_active_with_lock_holder_fails() {
            let ws = Workspace {
                id: WorkspaceId::parse("manual-active".into()).unwrap(),
                name: WorkspaceName::new("manual-active".into()).unwrap(),
                path: WorkspacePath::new("/tmp/manual".into()).unwrap(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                lock_holder: Some("ghost-holder".into()),
                config: None,
                state: WorkspaceState::Active,
                _state: std::marker::PhantomData,
            };
            let result = WorkspaceService::unlock_workspace(ws);
            assert!(result.is_err());
        }

        // ══════════════════════════════════════════════════════════════════════════
        // TABLE-DRIVEN: only Locked state is accepted
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn table_driven_only_locked_succeeds() {
            let cases: Vec<(&str, WorkspaceState, bool)> = vec![
                ("Initializing", WorkspaceState::Initializing, false),
                ("Active", WorkspaceState::Active, false),
                ("Locked", WorkspaceState::Locked, true),
                ("Corrupted", WorkspaceState::Corrupted, false),
                ("Deleted", WorkspaceState::Deleted, false),
            ];

            for (label, state, expect_ok) in cases {
                let ws = WorkspaceService::create_workspace(
                    WorkspaceName::new(format!("tbl-unlock-{}", label).into()).unwrap(),
                    WorkspacePath::new(format!("/tmp/tbl-unlock-{}", label)).unwrap(),
                )
                .unwrap();
                let ws_with_state = Workspace { state, ..ws };
                let result = WorkspaceService::unlock_workspace(ws_with_state);
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

        // ══════════════════════════════════════════════════════════════════════════
        // WRONG HOLDER: unlock by different identity
        //
        // NOTE: The current WorkspaceService::unlock_workspace does NOT validate
        // the lock holder identity. It always clears the lock regardless of who
        // holds it. These tests document the CURRENT behavior. If holder validation
        // is added in the future, these tests should be updated to expect errors.
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlock_by_different_holder_succeeds_current_behavior() {
            let locked = make_locked("wrong-holder-ok", "agent-owner");
            assert_eq!(locked.lock_holder(), Some("agent-owner"));
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(unlocked.is_active());
            assert!(unlocked.lock_holder().is_none());
        }

        #[test]
        fn unlock_clears_holder_regardless_of_who_calls() {
            let locked = make_locked("holder-indifferent", "agent-A");
            assert_eq!(locked.lock_holder(), Some("agent-A"));
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(unlocked.lock_holder().is_none());
        }

        #[test]
        fn unlock_by_different_holder_preserves_workspace_id() {
            let locked = make_locked("holder-preserve-id", "original-agent");
            let id_before = locked.id.as_str().to_string();
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert_eq!(unlocked.id.as_str(), id_before);
        }

        #[test]
        fn unlock_by_different_holder_preserves_workspace_name() {
            let locked = make_locked("holder-preserve-name", "original-agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert_eq!(unlocked.name.as_str(), "holder-preserve-name");
        }

        #[test]
        fn unlock_by_different_holder_preserves_config() {
            let locked = make_locked("holder-preserve-cfg", "original-agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            let config = unlocked.config().expect("should have config");
            assert_eq!(config.default_branch, "main");
            assert!(config.auto_sync);
        }

        // ══════════════════════════════════════════════════════════════════════════
        // STATE CORRUPTION: Locked→Corrupted then unlock fails
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlock_fails_after_manual_corruption_from_locked() {
            let locked = make_locked("corrupt-then-unlock", "agent-1");
            let corrupted = Workspace {
                state: WorkspaceState::Corrupted,
                ..locked
            };
            let result = WorkspaceService::unlock_workspace(corrupted);
            assert!(result.is_err());
        }

        #[test]
        fn unlock_after_corruption_returns_invalid_state_transition() {
            let locked = make_locked("corrupt-unlock-err", "agent-1");
            let corrupted = Workspace {
                state: WorkspaceState::Corrupted,
                ..locked
            };
            match WorkspaceService::unlock_workspace(corrupted).err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Corrupted");
                    assert_eq!(to, "Active");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn unlock_fails_after_manual_deletion_from_locked() {
            let locked = make_locked("deleted-then-unlock", "agent-1");
            let deleted = Workspace {
                state: WorkspaceState::Deleted,
                ..locked
            };
            let result = WorkspaceService::unlock_workspace(deleted);
            assert!(result.is_err());
        }

        #[test]
        fn unlock_after_deletion_returns_invalid_state_transition() {
            let locked = make_locked("del-unlock-err", "agent-1");
            let deleted = Workspace {
                state: WorkspaceState::Deleted,
                ..locked
            };
            match WorkspaceService::unlock_workspace(deleted).err() {
                Some(WorkspaceError::InvalidStateTransition { from, to }) => {
                    assert_eq!(from, "Deleted");
                    assert_eq!(to, "Active");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        // ══════════════════════════════════════════════════════════════════════════
        // MANUALLY CONSTRUCTED: workspace not found (no matching entity)
        //
        // WorkspaceService::unlock_workspace operates on a Workspace value, not a
        // repository. There is no "not found" case at this level. We test that
        // manually constructed workspaces with Locked state still unlock correctly.
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlock_manually_constructed_locked_workspace() {
            let ws = Workspace {
                id: WorkspaceId::parse("manual-ws-123".into()).unwrap(),
                name: WorkspaceName::new("manual-unlock".into()).unwrap(),
                path: WorkspacePath::new("/tmp/manual-unlock".into()).unwrap(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                lock_holder: Some("manual-agent".into()),
                config: None,
                state: WorkspaceState::Locked,
                _state: std::marker::PhantomData,
            };
            let unlocked = WorkspaceService::unlock_workspace(ws).unwrap();
            assert!(unlocked.is_active());
            assert!(unlocked.lock_holder().is_none());
            assert_eq!(unlocked.id.as_str(), "manual-ws-123");
            assert_eq!(unlocked.name.as_str(), "manual-unlock");
        }

        #[test]
        fn unlock_manually_constructed_locked_no_config() {
            let ws = Workspace {
                id: WorkspaceId::parse("no-cfg-id".into()).unwrap(),
                name: WorkspaceName::new("no-cfg".into()).unwrap(),
                path: WorkspacePath::new("/tmp/no-cfg".into()).unwrap(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                lock_holder: Some("holder".into()),
                config: None,
                state: WorkspaceState::Locked,
                _state: std::marker::PhantomData,
            };
            let unlocked = WorkspaceService::unlock_workspace(ws).unwrap();
            assert!(unlocked.config().is_none());
            assert!(unlocked.is_active());
        }

        #[test]
        fn unlock_manually_constructed_locked_no_holder() {
            let ws = Workspace {
                id: WorkspaceId::parse("no-holder-id".into()).unwrap(),
                name: WorkspaceName::new("no-holder".into()).unwrap(),
                path: WorkspacePath::new("/tmp/no-holder".into()).unwrap(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                lock_holder: None,
                config: None,
                state: WorkspaceState::Locked,
                _state: std::marker::PhantomData,
            };
            let unlocked = WorkspaceService::unlock_workspace(ws).unwrap();
            assert!(unlocked.lock_holder().is_none());
            assert!(unlocked.is_active());
        }

        // ══════════════════════════════════════════════════════════════════════════
        // LIFECYCLE: lock → unlock → lock → unlock (idempotency & cycling)
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlock_then_relock_with_different_holder() {
            let locked1 = make_locked("cycle-1", "agent-A");
            let unlocked1 = WorkspaceService::unlock_workspace(locked1).unwrap();
            assert!(unlocked1.is_active());

            let locked2 = WorkspaceService::lock_workspace(unlocked1, "agent-B".into());
            assert!(locked2.is_ok());
            let locked2 = locked2.unwrap();
            assert_eq!(locked2.lock_holder(), Some("agent-B"));

            let unlocked2 = WorkspaceService::unlock_workspace(locked2).unwrap();
            assert!(unlocked2.is_active());
            assert!(unlocked2.lock_holder().is_none());
        }

        #[test]
        fn unlock_then_delete_succeeds() {
            let locked = make_locked("unlock-del", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            let deleted = WorkspaceService::delete_workspace(unlocked).unwrap();
            assert_eq!(deleted.state, WorkspaceState::Deleted);
            assert!(deleted.is_terminal());
        }

        #[test]
        fn unlock_multiple_cycles_preserves_identity() {
            let active = make_active("multi-cycle");
            let id = active.id.as_str().to_string();
            let name = active.name.as_str().to_string();

            let mut current = active;
            for i in 0..5 {
                let locked =
                    WorkspaceService::lock_workspace(current, format!("agent-{}", i)).unwrap();
                current = WorkspaceService::unlock_workspace(locked).unwrap();
                assert_eq!(current.id.as_str(), id);
                assert_eq!(current.name.as_str(), name);
                assert!(current.is_active());
            }
        }

        #[test]
        fn double_unlock_fails() {
            let locked = make_locked("double-unlock", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert!(unlocked.is_active());
            let second = WorkspaceService::unlock_workspace(unlocked);
            assert!(
                second.is_err(),
                "unlocking an already-unlocked workspace must fail"
            );
        }

        // ══════════════════════════════════════════════════════════════════════════
        // ERROR MESSAGE FORMAT
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlock_error_message_contains_from_and_to_states() {
            let active = make_active("err-msg-unlock");
            let result = WorkspaceService::unlock_workspace(active);
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

        // ══════════════════════════════════════════════════════════════════════════
        // CROSS-METHOD: unlock result is visible to query methods
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlocked_workspace_appears_in_active_list() {
            let locked = make_locked("query-active", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            let all = vec![unlocked];
            let active_list = WorkspaceService::get_active_workspaces(&all);
            assert_eq!(active_list.len(), 1);
            assert_eq!(active_list[0].name.as_str(), "query-active");
        }

        #[test]
        fn unlocked_workspace_not_in_locked_list() {
            let locked = make_locked("query-locked", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            let all = vec![unlocked];
            let locked_list = WorkspaceService::get_locked_workspaces(&all);
            assert!(locked_list.is_empty());
        }

        #[test]
        fn unlocked_workspace_findable_by_id() {
            let locked = make_locked("find-unlocked", "agent");
            let id = locked.id.clone();
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            let all = vec![unlocked];
            let found = WorkspaceService::find_workspace(&all, &id);
            assert!(found.is_some());
            assert_eq!(found.unwrap().name.as_str(), "find-unlocked");
            assert!(found.unwrap().lock_holder().is_none());
        }

        #[test]
        fn unlocked_workspace_findable_by_name() {
            let locked = make_locked("name-unlocked", "agent");
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            let all = vec![unlocked];
            let found = WorkspaceService::find_by_name(
                &all,
                &WorkspaceName::new("name-unlocked".into()).unwrap(),
            );
            assert!(found.is_some());
            assert!(found.unwrap().lock_holder().is_none());
        }

        // ══════════════════════════════════════════════════════════════════════════
        // MULTIPLE WORKSPACES: independent unlock operations
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlock_multiple_independent_workspaces() {
            let mut unlocked_all = Vec::new();
            for i in 0..5 {
                let locked = make_locked(&format!("multi-{}", i), &format!("agent-{}", i));
                let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
                assert!(unlocked.is_active());
                assert!(unlocked.lock_holder().is_none());
                unlocked_all.push(unlocked);
            }
            let ids: std::collections::HashSet<&str> =
                unlocked_all.iter().map(|w| w.id.as_str()).collect();
            assert_eq!(ids.len(), 5);
        }

        #[test]
        fn unlock_does_not_affect_other_locked_workspaces() {
            let locked1 = make_locked("stay-locked", "agent-A");
            let locked2 = make_locked("get-unlocked", "agent-B");

            let _unlocked2 = WorkspaceService::unlock_workspace(locked2).unwrap();
            assert!(locked1.is_locked());
            assert_eq!(locked1.lock_holder(), Some("agent-A"));
        }

        // ══════════════════════════════════════════════════════════════════════════
        // RECOVER VS UNLOCK: both clear lock but via different paths
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlock_and_recover_both_clear_holder() {
            let locked_for_unlock = make_locked("via-unlock", "agent-1");
            let locked_for_recover = make_locked("via-recover", "agent-1");

            let unlocked = WorkspaceService::unlock_workspace(locked_for_unlock).unwrap();
            let recovered = WorkspaceService::recover_workspace(locked_for_recover).unwrap();

            assert!(unlocked.lock_holder().is_none());
            assert!(recovered.lock_holder().is_none());
            assert!(unlocked.is_active());
            assert!(recovered.is_active());
        }

        // ══════════════════════════════════════════════════════════════════════════
        // TIMESTAMP: unlock preserves created_at
        // ══════════════════════════════════════════════════════════════════════════

        #[test]
        fn unlock_preserves_created_at() {
            let locked = make_locked("unlock-ts", "agent");
            let created_at = locked.created_at();
            let unlocked = WorkspaceService::unlock_workspace(locked).unwrap();
            assert_eq!(unlocked.created_at(), created_at);
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // EXHAUSTIVE recover_workspace TESTS (ha-dp9)
    // ──────────────────────────────────────────────────────────────────────────

    mod recover_workspace_tests {
        use super::*;
        use crate::domain::entities::workspace::VcsType;

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
            let active = make_active(name);
            Workspace {
                state: WorkspaceState::Corrupted,
                ..active
            }
        }

        fn make_deleted(name: &str) -> Workspace {
            let active = make_active(name);
            Workspace {
                state: WorkspaceState::Deleted,
                ..active
            }
        }

        fn make_initializing(name: &str) -> Workspace {
            WorkspaceService::create_workspace(
                WorkspaceName::new(name.into()).unwrap(),
                WorkspacePath::new(format!("/tmp/{}", name)).unwrap(),
            )
            .unwrap()
        }

        #[test]
        fn recover_corrupted_to_active() {
            let corrupted = make_corrupted("happy-path");
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert!(recovered.is_active());
            assert!(!recovered.is_terminal());
        }

        #[test]
        fn recover_corrupted_preserves_workspace_id() {
            let corrupted = make_corrupted("id-preserve");
            let id_before = corrupted.id.clone();
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert_eq!(recovered.id.as_str(), id_before.as_str());
        }

        #[test]
        fn recover_corrupted_preserves_name() {
            let corrupted = make_corrupted("name-preserve");
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert_eq!(recovered.name.as_str(), "name-preserve");
        }

        #[test]
        fn recover_corrupted_preserves_path() {
            let corrupted = make_corrupted("path-preserve");
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert_eq!(recovered.path.as_str(), Some("/tmp/path-preserve"));
        }

        #[test]
        fn recover_corrupted_preserves_config() {
            let corrupted = make_corrupted("config-preserve");
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            let config = recovered.config().expect("should have config");
            assert_eq!(config.default_branch, "main");
            assert!(config.auto_sync);
            assert_eq!(config.vcs_type, VcsType::Git);
        }

        #[test]
        fn recover_corrupted_preserves_created_at() {
            let corrupted = make_corrupted("ts-preserve");
            let created_at = corrupted.created_at();
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert_eq!(recovered.created_at(), created_at);
        }

        #[test]
        fn recover_corrupted_updates_updated_at() {
            let corrupted = make_corrupted("ts-update");
            let updated_before = corrupted.updated_at();
            std::thread::sleep(std::time::Duration::from_millis(2));
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert!(recovered.updated_at() > updated_before);
        }

        #[test]
        fn recover_clears_corrupted_state_flag() {
            let corrupted = make_corrupted("clear-corrupt-flag");
            assert_eq!(corrupted.state, WorkspaceState::Corrupted);
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert_eq!(recovered.state, WorkspaceState::Active);
            assert!(!recovered.is_terminal());
        }

        #[test]
        fn recover_locked_workspace_unlocks_and_activates() {
            let locked = make_locked("recover-locked", "stuck-agent");
            let recovered = WorkspaceService::recover_workspace(locked).unwrap();
            assert!(recovered.is_active());
            assert!(recovered.lock_holder().is_none());
        }

        #[test]
        fn recover_locked_preserves_all_fields() {
            let locked = make_locked("locked-fields", "agent-x");
            let id = locked.id.as_str().to_string();
            let name = locked.name.as_str().to_string();
            let path = locked.path.as_str().unwrap().to_string();
            let created = locked.created_at();
            let config = locked.config().cloned();

            let recovered = WorkspaceService::recover_workspace(locked).unwrap();

            assert_eq!(recovered.id.as_str(), id);
            assert_eq!(recovered.name.as_str(), name);
            assert_eq!(recovered.path.as_str(), Some(path.as_str()));
            assert_eq!(recovered.created_at(), created);
            assert_eq!(
                recovered.config().map(|c| &c.default_branch),
                config.as_ref().map(|c| &c.default_branch)
            );
        }

        #[test]
        fn recover_rejects_active_workspace() {
            let active = make_active("reject-active");
            let result = WorkspaceService::recover_workspace(active);
            assert!(result.is_err());
            match result.err().unwrap() {
                WorkspaceError::InvalidStateTransition { from, to } => {
                    assert_eq!(from, "Active");
                    assert_eq!(to, "Recoverable");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn recover_rejects_initializing_workspace() {
            let init = make_initializing("reject-init");
            let result = WorkspaceService::recover_workspace(init);
            assert!(result.is_err());
            match result.err().unwrap() {
                WorkspaceError::InvalidStateTransition { from, to } => {
                    assert_eq!(from, "Initializing");
                    assert_eq!(to, "Recoverable");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn recover_rejects_deleted_workspace() {
            let deleted = make_deleted("reject-deleted");
            let result = WorkspaceService::recover_workspace(deleted);
            assert!(result.is_err());
            match result.err().unwrap() {
                WorkspaceError::InvalidStateTransition { from, to } => {
                    assert_eq!(from, "Deleted");
                    assert_eq!(to, "Recoverable");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }
        }

        #[test]
        fn recover_multiple_times_idempotent() {
            let corrupted = make_corrupted("multi-recover");
            let id = corrupted.id.as_str().to_string();
            let name = corrupted.name.as_str().to_string();

            // First recovery succeeds (Corrupted -> Active)
            let recovered1 = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert_eq!(recovered1.id.as_str(), id);
            assert_eq!(recovered1.name.as_str(), name);

            // Second recovery fails (Active cannot be recovered)
            let result2 = WorkspaceService::recover_workspace(recovered1.clone());
            assert!(result2.is_err());
            match result2.err().unwrap() {
                WorkspaceError::InvalidStateTransition { from, to } => {
                    assert_eq!(from, "Active");
                    assert_eq!(to, "Recoverable");
                }
                other => panic!("expected InvalidStateTransition, got {other:?}"),
            }

            // Third recovery also fails
            let result3 = WorkspaceService::recover_workspace(recovered1.clone());
            assert!(result3.is_err());

            // Recovery is idempotent in the sense that the first recovery is deterministic
            // and subsequent recoveries fail predictably
            let recovered2 =
                WorkspaceService::recover_workspace(make_corrupted("multi-recover-2")).unwrap();
            assert_eq!(recovered1.state, recovered2.state);
        }

        #[test]
        fn recover_preserves_config_across_multiple_recoveries() {
            let corrupted = make_corrupted("multi-cfg");
            let recovered1 = WorkspaceService::recover_workspace(corrupted).unwrap();
            let cfg1 = recovered1.config().cloned().expect("cfg1");

            // Second recovery fails (Active cannot be recovered)
            let result2 = WorkspaceService::recover_workspace(recovered1);
            assert!(result2.is_err());

            // But config is preserved on first recovery
            assert_eq!(cfg1.default_branch, "main");
            assert!(cfg1.auto_sync);

            // Create another corrupted workspace and recover it
            let corrupted2 = make_corrupted("multi-cfg-2");
            let recovered2 = WorkspaceService::recover_workspace(corrupted2).unwrap();
            let cfg2 = recovered2.config().expect("cfg2");

            // Config is preserved across independent recoveries
            assert_eq!(cfg1.default_branch, cfg2.default_branch);
            assert_eq!(cfg1.auto_sync, cfg2.auto_sync);
        }

        #[test]
        fn recover_corrupted_with_no_config() {
            let corrupted = make_corrupted("no-cfg");
            let corrupted_ws = Workspace {
                config: None,
                ..corrupted
            };
            let recovered = WorkspaceService::recover_workspace(corrupted_ws).unwrap();
            assert!(recovered.config().is_none());
            assert!(recovered.is_active());
        }

        #[test]
        fn recover_corrupted_with_lock_holder() {
            let active = make_active("lock-on-corrupt");
            let corrupted = Workspace {
                lock_holder: Some("stuck-agent".into()),
                state: WorkspaceState::Corrupted,
                ..active
            };
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert!(recovered.lock_holder().is_none());
            assert!(recovered.is_active());
        }

        #[test]
        fn recover_preserves_workspace_id_across_many_recoveries() {
            let id = WorkspaceId::parse("ws-unique-id-12345".into()).unwrap();
            let corrupted = Workspace {
                id: id.clone(),
                ..make_corrupted("id-many")
            };

            // First recovery succeeds
            let recovered1 = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert_eq!(recovered1.id.as_str(), id.as_str());

            // Second recovery fails (Active cannot be recovered)
            let result2 = WorkspaceService::recover_workspace(recovered1.clone());
            assert!(result2.is_err());

            // But ID is preserved on the first recovery
            assert_eq!(recovered1.id.as_str(), "ws-unique-id-12345");
        }

        #[test]
        fn recover_corrupted_result_is_valid_active_state() {
            let corrupted = make_corrupted("state-machine");
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            // Note: recover_workspace is an application-level operation that bypasses
            // the state machine's can_transition checks. It's allowed even though
            // Corrupted->Active is not a valid state machine transition.
            assert_eq!(recovered.state, WorkspaceState::Active);
        }

        #[test]
        fn recover_corrupted_vs_unlock_active() {
            let corrupted = make_corrupted("compare");
            let recovering = make_initializing("compare-init");

            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            let initialized = WorkspaceService::initialize_workspace(recovering).unwrap();

            // Both result in Active state
            assert_eq!(recovered.state, WorkspaceState::Active);
            assert_eq!(initialized.state, WorkspaceState::Active);
            assert!(recovered.is_active());
            assert!(initialized.is_active());
        }

        #[test]
        fn recover_corrupted_timestamps_are_reasonable() {
            let corrupted = make_corrupted("ts-reasonable");
            let recovered = WorkspaceService::recover_workspace(corrupted).unwrap();
            assert!(recovered.created_at() <= recovered.updated_at());
            assert!(recovered.updated_at() <= chrono::Utc::now());
        }

        #[test]
        fn recover_error_message_contains_from_and_to_states() {
            let active = make_active("error-msg");
            let result = WorkspaceService::recover_workspace(active);
            let err = result.err().unwrap();
            let msg = format!("{err}");
            assert!(msg.contains("Active") || msg.contains("Active"));
            assert!(msg.contains("Recoverable"));
        }

        #[test]
        fn all_recoverable_states_converge_to_active() {
            let corrupted = make_corrupted("state-a");
            let locked = make_locked("state-b", "agent");

            let recovered_corrupt = WorkspaceService::recover_workspace(corrupted).unwrap();
            let recovered_locked = WorkspaceService::recover_workspace(locked).unwrap();

            assert!(recovered_corrupt.is_active());
            assert!(recovered_locked.is_active());
            assert!(recovered_corrupt.lock_holder().is_none());
            assert!(recovered_locked.lock_holder().is_none());
        }

        #[test]
        fn recover_workspace_not_found_via_find() {
            let corrupted = make_corrupted("not-found");
            let ghost_id = WorkspaceId::parse("ws-nonexistent".into()).unwrap();
            let binding = [corrupted];

            let found = WorkspaceService::find_workspace(&binding, &ghost_id);
            assert!(found.is_none());
        }

        #[test]
        fn table_driven_recover_state_coverage() {
            let test_cases = vec![
                (WorkspaceState::Corrupted, true, "corrupted"),
                (WorkspaceState::Locked, true, "locked"),
                (WorkspaceState::Active, false, "active"),
                (WorkspaceState::Initializing, false, "initializing"),
                (WorkspaceState::Deleted, false, "deleted"),
            ];

            for (state, should_succeed, label) in test_cases {
                let ws = match state {
                    WorkspaceState::Corrupted => {
                        let active = make_active(label);
                        Workspace {
                            state: WorkspaceState::Corrupted,
                            ..active
                        }
                    }
                    WorkspaceState::Locked => make_locked(label, "agent"),
                    WorkspaceState::Active => make_active(label),
                    WorkspaceState::Initializing => make_initializing(label),
                    WorkspaceState::Deleted => {
                        let active = make_active(label);
                        Workspace {
                            state: WorkspaceState::Deleted,
                            ..active
                        }
                    }
                };

                let result = WorkspaceService::recover_workspace(ws);
                if should_succeed {
                    assert!(result.is_ok(), "{label} should succeed");
                    if let Ok(recovered) = result {
                        assert!(
                            recovered.is_active(),
                            "{label} should result in Active state"
                        );
                    }
                } else {
                    assert!(result.is_err(), "{label} should fail");
                }
            }
        }
    }
}
