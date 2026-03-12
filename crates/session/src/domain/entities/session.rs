use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::events::SessionEvent;
use crate::domain::value_objects::{BeadId, SessionName, WorkspaceId};
use crate::error::SessionError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Created,
    Active,
    Syncing,
    Synced,
    Paused,
    Completed,
    Failed,
}

impl SessionState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    pub fn transition_to(&self, new: Self) -> Result<Self, SessionError> {
        let current = self.clone();
        match (current, new.clone()) {
            (Self::Created, Self::Active) => Ok(new),
            (Self::Created, Self::Failed) => Ok(new),
            (Self::Active, Self::Syncing) => Ok(new),
            (Self::Active, Self::Paused) => Ok(new),
            (Self::Active, Self::Failed) => Ok(new),
            (Self::Syncing, Self::Synced) => Ok(new),
            (Self::Syncing, Self::Failed) => Ok(new),
            (Self::Synced, Self::Active) => Ok(new),
            (Self::Synced, Self::Completed) => Ok(new),
            (Self::Paused, Self::Active) => Ok(new),
            (Self::Paused, Self::Failed) => Ok(new),
            (a, b) if a == b => Ok(b),
            (a, b) => Err(SessionError::InvalidSessionTransition { from: a, to: b }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchState {
    Detached,
    OnBranch { name: String },
}

impl BranchState {
    pub fn branch_name(&self) -> Option<&str> {
        match self {
            Self::Detached => None,
            Self::OnBranch { name } => Some(name),
        }
    }

    pub const fn is_detached(&self) -> bool {
        matches!(self, Self::Detached)
    }

    pub fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Detached, Self::OnBranch { .. })
            | (Self::OnBranch { .. }, Self::Detached)
            | (Self::OnBranch { .. }, Self::OnBranch { .. }) => true,
            (Self::Detached, Self::Detached) => false,
        }
    }
}

impl std::fmt::Display for BranchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Detached => write!(f, "detached"),
            Self::OnBranch { name } => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn parse(s: impl Into<String>) -> Result<Self, SessionError> {
        let s = s.into();
        if s.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "SessionId cannot be empty".into(),
            ));
        }
        if !s.is_ascii() {
            return Err(SessionError::InvalidIdentifier(
                "SessionId must be ASCII".into(),
            ));
        }
        Ok(Self(s))
    }

    pub fn generate() -> Self {
        Self(format!("session-{}", uuid::Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SessionId {
    type Error = SessionError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub name: SessionName,
    pub workspace: Option<WorkspaceId>,
    pub bead: Option<BeadId>,
    pub branch: BranchState,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn create(name: SessionName) -> Result<Self, SessionError> {
        Ok(Self {
            id: SessionId::generate(),
            name,
            workspace: None,
            bead: None,
            branch: BranchState::Detached,
            state: SessionState::Created,
            created_at: Utc::now(),
        })
    }

    pub fn transition(&self, event: SessionEvent) -> Result<Self, SessionError> {
        let current = &self.state;
        let new_state = match (current, event) {
            (SessionState::Created, SessionEvent::Activated) => SessionState::Active,
            (SessionState::Active, SessionEvent::Syncing) => SessionState::Syncing,
            (SessionState::Active, SessionEvent::Paused) => SessionState::Paused,
            (SessionState::Active, SessionEvent::Failed) => SessionState::Failed,
            (SessionState::Syncing, SessionEvent::Synced) => SessionState::Synced,
            (SessionState::Syncing, SessionEvent::Failed) => SessionState::Failed,
            (SessionState::Synced, SessionEvent::Activated) => SessionState::Active,
            (SessionState::Synced, SessionEvent::Completed) => SessionState::Completed,
            (SessionState::Paused, SessionEvent::Activated) => SessionState::Active,
            (SessionState::Paused, SessionEvent::Failed) => SessionState::Failed,
            (a, b) => {
                return Err(SessionError::InvalidSessionTransition {
                    from: a.clone(),
                    to: Self::next_state_for_error(a.clone(), b),
                });
            }
        };

        Ok(Self {
            id: self.id.clone(),
            name: self.name.clone(),
            workspace: self.workspace.clone(),
            bead: self.bead.clone(),
            branch: self.branch.clone(),
            state: new_state,
            created_at: self.created_at,
        })
    }

    /// Helper to determine target state for error reporting
    fn next_state_for_error(current: SessionState, event: SessionEvent) -> SessionState {
        match (current, event) {
            (SessionState::Created, SessionEvent::Activated) => SessionState::Active,
            (SessionState::Active, SessionEvent::Syncing) => SessionState::Syncing,
            (SessionState::Active, SessionEvent::Paused) => SessionState::Paused,
            (SessionState::Active, SessionEvent::Failed) => SessionState::Failed,
            (SessionState::Syncing, SessionEvent::Synced) => SessionState::Synced,
            (SessionState::Syncing, SessionEvent::Failed) => SessionState::Failed,
            (SessionState::Synced, SessionEvent::Activated) => SessionState::Active,
            (SessionState::Synced, SessionEvent::Completed) => SessionState::Completed,
            (SessionState::Paused, SessionEvent::Activated) => SessionState::Active,
            (SessionState::Paused, SessionEvent::Failed) => SessionState::Failed,
            (s, _) => s,
        }
    }

    pub fn transition_branch(&self, new_branch: BranchState) -> Result<Self, SessionError> {
        if !self.branch.can_transition_to(&new_branch) {
            return Err(SessionError::InvalidBranchTransition {
                from: format!("{:?}", self.branch),
                to: format!("{:?}", new_branch),
            });
        }

        Ok(Self {
            branch: new_branch,
            ..self.clone()
        })
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            SessionState::Active | SessionState::Syncing | SessionState::Synced
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_created_has_created_state() {
        let name = SessionName::parse("test-session").expect("valid");
        let session = Session::create(name).expect("created");
        assert_eq!(session.state, SessionState::Created);
    }

    #[test]
    fn test_session_state_transitions() {
        let name = SessionName::parse("test").expect("valid");
        let session = Session::create(name).expect("created");

        let active = session
            .transition(SessionEvent::Activated)
            .expect("valid transition");
        assert_eq!(active.state, SessionState::Active);

        let syncing = active
            .transition(SessionEvent::Syncing)
            .expect("valid transition");
        assert_eq!(syncing.state, SessionState::Syncing);

        let synced = syncing
            .transition(SessionEvent::Synced)
            .expect("valid transition");
        assert_eq!(synced.state, SessionState::Synced);

        let completed = synced
            .transition(SessionEvent::Completed)
            .expect("valid transition");
        assert_eq!(completed.state, SessionState::Completed);
    }

    #[test]
    fn test_branch_transition() {
        let name = SessionName::parse("test").expect("valid");
        let session = Session::create(name).expect("created");

        let on_main = session
            .transition_branch(BranchState::OnBranch {
                name: "main".into(),
            })
            .expect("valid");
        assert_eq!(on_main.branch.branch_name(), Some("main"));

        let detached = on_main
            .transition_branch(BranchState::Detached)
            .expect("valid");
        assert!(detached.branch.is_detached());
    }
}
