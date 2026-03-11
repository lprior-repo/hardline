use crate::domain::value_objects::{WorkspaceName, WorkspacePath};
use crate::error::WorkspaceError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceState {
    Initializing,
    Active,
    Locked,
    Corrupted,
    Deleted,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self::Initializing
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn generate() -> Self {
        Self(format!("ws-{}", uuid::Uuid::new_v4()))
    }

    pub fn parse(id: String) -> Result<Self, WorkspaceError> {
        if id.is_empty() {
            return Err(WorkspaceError::InvalidWorkspaceId("empty id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WorkspaceId {
    fn default() -> Self {
        Self::generate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: WorkspaceName,
    pub path: WorkspacePath,
    pub state: WorkspaceState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lock_holder: Option<String>,
    pub config: Option<WorkspaceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub vcs_type: VcsType,
    pub default_branch: String,
    pub auto_sync: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VcsType {
    Jj,
    Git,
    Both,
}

impl Default for VcsType {
    fn default() -> Self {
        Self::Jj
    }
}

impl Workspace {
    pub fn create(name: WorkspaceName, path: WorkspacePath) -> Result<Self, WorkspaceError> {
        let now = Utc::now();
        Ok(Self {
            id: WorkspaceId::generate(),
            name,
            path,
            state: WorkspaceState::Initializing,
            created_at: now,
            updated_at: now,
            lock_holder: None,
            config: Some(WorkspaceConfig {
                vcs_type: VcsType::default(),
                default_branch: "main".into(),
                auto_sync: true,
            }),
        })
    }

    pub fn activate(&self) -> Result<Self, WorkspaceError> {
        if self.state != WorkspaceState::Initializing {
            return Err(WorkspaceError::InvalidStateTransition {
                from: format!("{:?}", self.state),
                to: "Active".into(),
            });
        }
        Ok(self.transition_to(WorkspaceState::Active))
    }

    pub fn lock(&self, holder: String) -> Result<Self, WorkspaceError> {
        if self.state != WorkspaceState::Active {
            return Err(WorkspaceError::InvalidStateTransition {
                from: format!("{:?}", self.state),
                to: "Locked".into(),
            });
        }
        let mut workspace = self.transition_to(WorkspaceState::Locked);
        workspace.lock_holder = Some(holder);
        Ok(workspace)
    }

    pub fn unlock(&self) -> Result<Self, WorkspaceError> {
        if self.state != WorkspaceState::Locked {
            return Err(WorkspaceError::InvalidStateTransition {
                from: format!("{:?}", self.state),
                to: "Active".into(),
            });
        }
        let mut workspace = self.transition_to(WorkspaceState::Active);
        workspace.lock_holder = None;
        Ok(workspace)
    }

    pub fn mark_corrupted(&self) -> Result<Self, WorkspaceError> {
        let mut workspace = self.transition_to(WorkspaceState::Corrupted);
        workspace.lock_holder = None;
        Ok(workspace)
    }

    pub fn delete(&self) -> Result<Self, WorkspaceError> {
        if matches!(self.state, WorkspaceState::Deleted) {
            return Err(WorkspaceError::InvalidStateTransition {
                from: format!("{:?}", self.state),
                to: "Deleted".into(),
            });
        }
        Ok(self.transition_to(WorkspaceState::Deleted))
    }

    fn transition_to(&self, new_state: WorkspaceState) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            path: self.path.clone(),
            state: new_state,
            created_at: self.created_at,
            updated_at: Utc::now(),
            lock_holder: self.lock_holder.clone(),
            config: self.config.clone(),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.state == WorkspaceState::Locked
    }

    pub fn is_active(&self) -> bool {
        self.state == WorkspaceState::Active
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            WorkspaceState::Deleted | WorkspaceState::Corrupted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_when_created_then_has_initializing_state() {
        let workspace = Workspace::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        assert_eq!(workspace.state, WorkspaceState::Initializing);
    }

    #[test]
    fn workspace_given_initializing_when_activate_then_has_active_state() {
        let workspace = Workspace::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let activated = workspace.activate().unwrap();
        assert_eq!(activated.state, WorkspaceState::Active);
    }

    #[test]
    fn workspace_given_active_when_lock_then_has_locked_state() {
        let workspace = Workspace::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let activated = workspace.activate().unwrap();
        let locked = activated.lock("agent-1".into()).unwrap();
        assert_eq!(locked.state, WorkspaceState::Locked);
        assert_eq!(locked.lock_holder, Some("agent-1".into()));
    }

    #[test]
    fn workspace_given_active_when_activate_then_fails() {
        let workspace = Workspace::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let activated = workspace.activate().unwrap();
        let result = activated.activate();
        assert!(result.is_err());
    }
}
