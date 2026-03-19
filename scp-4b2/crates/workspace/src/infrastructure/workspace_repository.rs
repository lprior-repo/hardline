use crate::domain::entities::{Workspace, WorkspaceId};
use crate::error::{Result, WorkspaceError};
use std::collections::HashMap;
use std::sync::Mutex;

pub trait WorkspaceRepository: Send + Sync {
    fn save(&self, workspace: Workspace) -> Result<Workspace>;
    fn get(&self, id: &WorkspaceId) -> Result<Option<Workspace>>;
    fn get_by_name(&self, name: &str) -> Result<Option<Workspace>>;
    fn list(&self) -> Result<Vec<Workspace>>;
    fn list_active(&self) -> Result<Vec<Workspace>>;
    fn delete(&self, id: &WorkspaceId) -> Result<()>;
}

pub struct InMemoryWorkspaceRepository {
    workspaces: Mutex<HashMap<String, Workspace>>,
}

impl InMemoryWorkspaceRepository {
    pub fn new() -> Self {
        Self {
            workspaces: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryWorkspaceRepository {
    fn default() -> Self {
        Self::new()
    }
}

// Pure calculation helpers
fn find_workspace_by_name<'a>(
    map: &'a HashMap<String, Workspace>,
    name: &str,
) -> Option<&'a Workspace> {
    map.values().find(|w| w.name.as_str() == name)
}

fn collect_all_workspaces(map: &HashMap<String, Workspace>) -> Vec<Workspace> {
    map.values().cloned().collect()
}

fn filter_active_workspaces(map: &HashMap<String, Workspace>) -> Vec<Workspace> {
    map.values()
        .filter(|w| w.state == crate::domain::entities::WorkspaceState::Active)
        .cloned()
        .collect()
}

// Helper to safely acquire mutex lock, treating poison as unrecoverable
fn lock_workspace_map(
    workspaces: &Mutex<HashMap<String, Workspace>>,
) -> std::sync::MutexGuard<'_, HashMap<String, Workspace>> {
    // In normal operation, the lock will not be poisoned since we own the mutex
    // If poisoned, we treat it as unrecoverable and panic
    match workspaces.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl WorkspaceRepository for InMemoryWorkspaceRepository {
    fn save(&self, workspace: Workspace) -> Result<Workspace> {
        let workspace_clone = workspace.clone();
        let id_str = workspace_clone.id.as_str().to_string();
        let _old = lock_workspace_map(&self.workspaces).insert(id_str, workspace_clone.clone());
        Ok(workspace)
    }

    fn get(&self, id: &WorkspaceId) -> Result<Option<Workspace>> {
        let map = lock_workspace_map(&self.workspaces);
        Ok(map.get(id.as_str()).cloned())
    }

    fn get_by_name(&self, name: &str) -> Result<Option<Workspace>> {
        let map = lock_workspace_map(&self.workspaces);
        Ok(find_workspace_by_name(&map, name).cloned())
    }

    fn list(&self) -> Result<Vec<Workspace>> {
        let map = lock_workspace_map(&self.workspaces);
        Ok(collect_all_workspaces(&map))
    }

    fn list_active(&self) -> Result<Vec<Workspace>> {
        let map = lock_workspace_map(&self.workspaces);
        Ok(filter_active_workspaces(&map))
    }

    fn delete(&self, id: &WorkspaceId) -> Result<()> {
        let key_str = id.as_str().to_string();
        lock_workspace_map(&self.workspaces)
            .remove(&key_str)
            .map(|_| ())
            .ok_or_else(|| WorkspaceError::WorkspaceNotFound(id.as_str().into()))
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
        repo.save(active).unwrap();
        let actives = repo.list_active().unwrap();
        assert_eq!(actives.len(), 1);
    }
}
