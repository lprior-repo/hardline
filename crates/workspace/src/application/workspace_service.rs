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
}
