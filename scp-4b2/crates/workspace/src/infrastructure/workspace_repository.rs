use crate::domain::entities::{Workspace, WorkspaceId, WorkspaceState};
use crate::error::{Result, WorkspaceError};
use std::collections::HashMap;

pub trait WorkspaceRepository: Send + Sync {
    fn save(&self, workspace: Workspace) -> Result<Workspace>;
    fn get(&self, id: &WorkspaceId) -> Result<Option<Workspace>>;
    fn get_by_name(&self, name: &str) -> Result<Option<Workspace>>;
    fn list(&self) -> Result<Vec<Workspace>>;
    fn list_active(&self) -> Result<Vec<Workspace>>;
    fn delete(&self, id: &WorkspaceId) -> Result<()>;
}

pub struct InMemoryWorkspaceRepository {
    workspaces: HashMap<String, Workspace>,
}

impl InMemoryWorkspaceRepository {
    pub fn new() -> Self {
        Self {
            workspaces: HashMap::new(),
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
        let mut workspaces = self.workspaces.clone();
        let id = workspace.id.as_str().to_string();
        workspaces.insert(id, workspace.clone());
        Ok(workspace)
    }

    fn get(&self, id: &WorkspaceId) -> Result<Option<Workspace>> {
        Ok(self.workspaces.get(id.as_str()).cloned())
    }

    fn get_by_name(&self, name: &str) -> Result<Option<Workspace>> {
        Ok(self
            .workspaces
            .values()
            .find(|w| w.name.as_str() == name)
            .cloned())
    }

    fn list(&self) -> Result<Vec<Workspace>> {
        Ok(self.workspaces.values().cloned().collect())
    }

    fn list_active(&self) -> Result<Vec<Workspace>> {
        Ok(self
            .workspaces
            .values()
            .filter(|w| w.state == WorkspaceState::Active)
            .cloned()
            .collect())
    }

    fn delete(&self, id: &WorkspaceId) -> Result<()> {
        let mut workspaces = self.workspaces.clone();
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
        repo.save(active).unwrap();
        let actives = repo.list_active().unwrap();
        assert_eq!(actives.len(), 1);
    }
}
