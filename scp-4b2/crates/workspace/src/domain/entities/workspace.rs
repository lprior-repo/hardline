use crate::domain::value_objects::{WorkspaceName, WorkspacePath};
use crate::error::WorkspaceError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorkspaceState {
    #[default]
    Initializing,
    Active,
    Locked,
    Corrupted,
    Deleted,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VcsType {
    #[default]
    Jj,
    Git,
    Both,
}

/// Constructs a default workspace configuration with standard settings.
fn build_default_workspace_config() -> Option<WorkspaceConfig> {
    Some(WorkspaceConfig {
        vcs_type: VcsType::default(),
        default_branch: "main".into(),
        auto_sync: true,
    })
}

impl Workspace {
    pub fn create(name: WorkspaceName, path: WorkspacePath) -> Result<Self, WorkspaceError> {
        let now = Utc::now();
        let config = build_default_workspace_config();
        Ok(Self::with_updated_state(
            &WorkspaceId::generate(),
            &name,
            &path,
            WorkspaceState::Initializing,
            now,
            None,
            &config,
        ))
    }

    pub fn activate(&self) -> Result<Self, WorkspaceError> {
        Self::validate_state_transition(
            self.state,
            WorkspaceState::Initializing,
            format!("{:?}", WorkspaceState::Active),
        )
        .map(|()| self.transition_to(WorkspaceState::Active))
    }

    pub fn lock(&self, holder: String) -> Result<Self, WorkspaceError> {
        Self::validate_state_transition(
            self.state,
            WorkspaceState::Active,
            format!("{:?}", WorkspaceState::Locked),
        )
        .map(|()| self.transition_to_with_lock(WorkspaceState::Locked, Some(holder)))
    }

    pub fn unlock(&self) -> Result<Self, WorkspaceError> {
        Self::validate_state_transition(
            self.state,
            WorkspaceState::Locked,
            format!("{:?}", WorkspaceState::Active),
        )
        .map(|()| self.transition_to_with_lock(WorkspaceState::Active, None))
    }

    pub fn mark_corrupted(&self) -> Result<Self, WorkspaceError> {
        Self::validate_state_transition(
            self.state,
            WorkspaceState::Active,
            format!("{:?}", WorkspaceState::Corrupted),
        )
        .map(|()| self.transition_to_with_lock(WorkspaceState::Corrupted, None))
    }

    pub fn delete(&self) -> Result<Self, WorkspaceError> {
        Self::validate_not_in_state(self.state, WorkspaceState::Deleted, "Deleted")
            .map(|()| self.transition_to(WorkspaceState::Deleted))
    }

    /// Formats a state transition error from the current state to a target state.
    fn format_state_transition_error(
        from: WorkspaceState,
        to: impl Into<String>,
    ) -> WorkspaceError {
        WorkspaceError::InvalidStateTransition {
            from: format!("{:?}", from),
            to: to.into(),
        }
    }

    fn validate_state_transition(
        current_state: WorkspaceState,
        expected_from: WorkspaceState,
        target_to: impl Into<String>,
    ) -> Result<(), WorkspaceError> {
        if current_state != expected_from {
            return Err(Self::format_state_transition_error(
                current_state,
                target_to,
            ));
        }
        Ok(())
    }

    fn validate_not_in_state(
        current_state: WorkspaceState,
        forbidden_state: WorkspaceState,
        target_to: &'static str,
    ) -> Result<(), WorkspaceError> {
        if current_state == forbidden_state {
            return Err(Self::format_state_transition_error(
                current_state,
                target_to,
            ));
        }
        Ok(())
    }

    fn transition_to(&self, new_state: WorkspaceState) -> Self {
        self.transition_to_with_lock(new_state, self.lock_holder.clone())
    }

    fn transition_to_with_lock(
        &self,
        new_state: WorkspaceState,
        lock_holder: Option<String>,
    ) -> Self {
        Self::with_updated_state(
            &self.id,
            &self.name,
            &self.path,
            new_state,
            self.created_at,
            lock_holder,
            &self.config,
        )
    }

    fn with_updated_state(
        id: &WorkspaceId,
        name: &WorkspaceName,
        path: &WorkspacePath,
        state: WorkspaceState,
        created_at: DateTime<Utc>,
        lock_holder: Option<String>,
        config: &Option<WorkspaceConfig>,
    ) -> Self {
        Self {
            id: id.clone(),
            name: name.clone(),
            path: path.clone(),
            state,
            created_at,
            updated_at: Utc::now(),
            lock_holder,
            config: config.clone(),
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
