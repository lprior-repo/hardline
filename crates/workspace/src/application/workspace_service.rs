use crate::domain::entities::{Workspace, WorkspaceId, WorkspaceState};
use crate::domain::value_objects::{WorkspaceName, WorkspacePath};
use crate::error::{Result, WorkspaceError};

pub struct WorkspaceService;

impl WorkspaceService {
    pub fn create_workspace(name: WorkspaceName, path: WorkspacePath) -> Result<Workspace> {
        Workspace::create(name, path)
    }

    pub fn initialize_workspace(workspace: &Workspace) -> Result<Workspace> {
        workspace.activate()
    }

    pub fn lock_workspace(workspace: &Workspace, holder: String) -> Result<Workspace> {
        workspace.lock(holder)
    }

    pub fn unlock_workspace(workspace: &Workspace) -> Result<Workspace> {
        workspace.unlock()
    }

    pub fn delete_workspace(workspace: &Workspace) -> Result<Workspace> {
        if workspace.is_locked() {
            return Err(WorkspaceError::WorkspaceLocked(
                workspace.id.as_str().into(),
                workspace.lock_holder.clone().unwrap_or_default(),
            ));
        }
        workspace.delete()
    }

    pub fn recover_workspace(workspace: &Workspace) -> Result<Workspace> {
        if workspace.state == WorkspaceState::Corrupted {
            workspace.unlock()?.activate()
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
        workspaces.iter().filter(|w| w.is_locked()).collect()
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
        let initialized = WorkspaceService::initialize_workspace(&workspace).unwrap();
        assert_eq!(initialized.state, WorkspaceState::Active);
    }

    #[test]
    fn workspace_service_lock_and_unlock() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(&workspace).unwrap();
        let locked = WorkspaceService::lock_workspace(&initialized, "agent-1".into()).unwrap();
        assert!(locked.is_locked());
        let unlocked = WorkspaceService::unlock_workspace(&locked).unwrap();
        assert!(unlocked.is_active());
    }

    #[test]
    fn workspace_service_delete_fails_when_locked() {
        let workspace = WorkspaceService::create_workspace(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let initialized = WorkspaceService::initialize_workspace(&workspace).unwrap();
        let locked = WorkspaceService::lock_workspace(&initialized, "agent-1".into()).unwrap();
        let result = WorkspaceService::delete_workspace(&locked);
        assert!(result.is_err());
    }
}
