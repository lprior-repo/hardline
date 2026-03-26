#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{WorkspaceName, WorkspacePath};
use crate::error::WorkspaceError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceState {
    Initializing,
    Active,
    Locked,
    Corrupted,
    Deleted,
}

impl WorkspaceState {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Deleted | Self::Corrupted)
    }
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

#[derive(Clone)]
pub struct Initializing;
#[derive(Clone)]
pub struct Active;
#[derive(Clone)]
pub struct Locked;
#[derive(Clone)]
pub struct Corrupted;
#[derive(Clone)]
pub struct Deleted;

#[derive(Debug, Clone)]
pub struct Workspace<S = Initializing> {
    pub id: WorkspaceId,
    pub name: WorkspaceName,
    pub path: WorkspacePath,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lock_holder: Option<String>,
    pub config: Option<WorkspaceConfig>,
    pub state: WorkspaceState,
    pub _state: PhantomData<S>,
}

impl Workspace<Initializing> {
    pub fn create(name: WorkspaceName, path: WorkspacePath) -> Result<Self, WorkspaceError> {
        let now = Utc::now();
        Ok(Self {
            id: WorkspaceId::generate(),
            name,
            path,
            created_at: now,
            updated_at: now,
            lock_holder: None,
            config: Some(WorkspaceConfig {
                vcs_type: VcsType::default(),
                default_branch: "main".into(),
                auto_sync: true,
            }),
            state: WorkspaceState::Initializing,
            _state: PhantomData,
        })
    }

    pub fn activate(self) -> Result<Workspace<Active>, WorkspaceError> {
        self.transition_impl(WorkspaceState::Active)
    }

    pub fn delete(self) -> Result<Workspace<Deleted>, WorkspaceError> {
        self.transition_impl(WorkspaceState::Deleted)
    }
}

impl<S> Workspace<S> {
    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    pub fn name(&self) -> &WorkspaceName {
        &self.name
    }

    pub fn path(&self) -> &WorkspacePath {
        &self.path
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn lock_holder(&self) -> Option<&str> {
        self.lock_holder.as_deref()
    }

    pub fn config(&self) -> Option<&WorkspaceConfig> {
        self.config.as_ref()
    }

    pub fn is_locked(&self) -> bool {
        self.state == WorkspaceState::Locked
    }

    pub fn is_active(&self) -> bool {
        self.state == WorkspaceState::Active
    }

    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    fn transition_impl<T>(self, new_state: WorkspaceState) -> Result<Workspace<T>, WorkspaceError> {
        Ok(Workspace {
            id: self.id,
            name: self.name,
            path: self.path,
            created_at: self.created_at,
            updated_at: Utc::now(),
            lock_holder: self.lock_holder,
            config: self.config,
            state: new_state,
            _state: PhantomData,
        })
    }

    fn transition_with_lock_holder<T>(
        self,
        lock_holder: Option<String>,
        new_state: WorkspaceState,
    ) -> Result<Workspace<T>, WorkspaceError> {
        Ok(Workspace {
            id: self.id,
            name: self.name,
            path: self.path,
            created_at: self.created_at,
            updated_at: Utc::now(),
            lock_holder,
            config: self.config,
            state: new_state,
            _state: PhantomData,
        })
    }
}

impl Workspace<Active> {
    pub fn lock(self, holder: String) -> Result<Workspace<Locked>, WorkspaceError> {
        self.transition_with_lock_holder(Some(holder), WorkspaceState::Locked)
    }

    pub fn mark_corrupted(self) -> Result<Workspace<Corrupted>, WorkspaceError> {
        self.transition_with_lock_holder(None, WorkspaceState::Corrupted)
    }

    pub fn delete(self) -> Result<Workspace<Deleted>, WorkspaceError> {
        self.transition_with_lock_holder(None, WorkspaceState::Deleted)
    }
}

impl Workspace<Locked> {
    pub fn unlock(self) -> Result<Workspace<Active>, WorkspaceError> {
        self.transition_with_lock_holder(None, WorkspaceState::Active)
    }

    pub fn mark_corrupted(self) -> Result<Workspace<Corrupted>, WorkspaceError> {
        self.transition_with_lock_holder(None, WorkspaceState::Corrupted)
    }

    pub fn delete(self) -> Result<Workspace<Deleted>, WorkspaceError> {
        self.transition_with_lock_holder(None, WorkspaceState::Deleted)
    }
}

impl Workspace<Corrupted> {
    pub fn delete(self) -> Result<Workspace<Deleted>, WorkspaceError> {
        self.transition_with_lock_holder(None, WorkspaceState::Deleted)
    }
}

impl Workspace<Deleted> {
    // Deleted is terminal - no further transitions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_when_created_then_has_initializing_state() {
        let workspace = Workspace::<Initializing>::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        assert!(!workspace.is_active());
    }

    #[test]
    fn workspace_given_initializing_when_activate_then_has_active_state() {
        let workspace = Workspace::<Initializing>::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let activated: Workspace<Active> = workspace.activate().unwrap();
        assert!(activated.is_active());
    }

    #[test]
    fn workspace_given_active_when_lock_then_has_locked_state() {
        let workspace = Workspace::<Initializing>::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let activated: Workspace<Active> = workspace.activate().unwrap();
        let locked: Workspace<Locked> = activated.lock("agent-1".into()).unwrap();
        assert!(locked.is_locked());
        assert_eq!(locked.lock_holder(), Some("agent-1"));
    }

    #[test]
    fn workspace_given_active_when_mark_corrupted_then_has_corrupted_state() {
        let workspace = Workspace::<Initializing>::create(
            WorkspaceName::new("test".into()).unwrap(),
            WorkspacePath::new("/tmp/test".into()).unwrap(),
        )
        .unwrap();
        let activated: Workspace<Active> = workspace.activate().unwrap();
        let corrupted: Workspace<Corrupted> = activated.mark_corrupted().unwrap();
        assert!(corrupted.is_terminal());
    }
}
