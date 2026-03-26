#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{BeadId, SessionName, WorkspaceId};
use crate::error::SessionError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchState {
    Detached,
    OnBranch { name: String },
}

impl BranchState {
    #[must_use]
    pub fn branch_name(&self) -> Option<&str> {
        match self {
            Self::Detached => None,
            Self::OnBranch { name } => Some(name),
        }
    }

    #[must_use]
    pub const fn is_detached(&self) -> bool {
        matches!(self, Self::Detached)
    }

    #[must_use]
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

pub struct Created;
pub struct Active;
pub struct Syncing;
pub struct Synced;
pub struct Paused;
pub struct Completed;
pub struct Failed;

/// Typestate marker trait to get SessionState from the marker type
pub trait StateInfo {
    fn state() -> SessionState;
}

impl StateInfo for Created {
    fn state() -> SessionState {
        SessionState::Created
    }
}
impl StateInfo for Active {
    fn state() -> SessionState {
        SessionState::Active
    }
}
impl StateInfo for Syncing {
    fn state() -> SessionState {
        SessionState::Syncing
    }
}
impl StateInfo for Synced {
    fn state() -> SessionState {
        SessionState::Synced
    }
}
impl StateInfo for Paused {
    fn state() -> SessionState {
        SessionState::Paused
    }
}
impl StateInfo for Completed {
    fn state() -> SessionState {
        SessionState::Completed
    }
}
impl StateInfo for Failed {
    fn state() -> SessionState {
        SessionState::Failed
    }
}

/// Typestate marker trait for active states (where is_active returns true)
pub trait SealedActive {}
impl SealedActive for Active {}
impl SealedActive for Syncing {}
impl SealedActive for Synced {}

#[derive(Debug, Clone)]
pub struct Session<S = Created> {
    pub id: SessionId,
    pub name: SessionName,
    pub workspace: Option<WorkspaceId>,
    pub bead: Option<BeadId>,
    pub branch: BranchState,
    pub created_at: DateTime<Utc>,
    _state: PhantomData<S>,
}

impl Session<Created> {
    pub fn create(name: SessionName) -> Result<Self, SessionError> {
        Ok(Self {
            id: SessionId::generate(),
            name,
            workspace: None,
            bead: None,
            branch: BranchState::Detached,
            created_at: Utc::now(),
            _state: PhantomData,
        })
    }

    /// Create a Session from parsed components (used by repository)
    pub fn from_parts(
        id: SessionId,
        name: SessionName,
        workspace: Option<WorkspaceId>,
        bead: Option<BeadId>,
        branch: BranchState,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            workspace,
            bead,
            branch,
            created_at,
            _state: PhantomData,
        }
    }

    pub fn activate(self) -> Result<Session<Active>, SessionError> {
        self.transition_impl(SessionState::Active)
    }

    pub fn fail(self) -> Result<Session<Failed>, SessionError> {
        self.transition_impl(SessionState::Failed)
    }
}

impl<S: StateInfo> Session<S> {
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub fn name(&self) -> &SessionName {
        &self.name
    }

    pub fn workspace(&self) -> Option<&WorkspaceId> {
        self.workspace.as_ref()
    }

    pub fn bead(&self) -> Option<&BeadId> {
        self.bead.as_ref()
    }

    pub fn branch(&self) -> &BranchState {
        &self.branch
    }

    pub fn state(&self) -> SessionState {
        S::state()
    }

    fn transition_impl<T>(self, _new_state: SessionState) -> Result<Session<T>, SessionError> {
        Ok(Session {
            id: self.id,
            name: self.name,
            workspace: self.workspace,
            bead: self.bead,
            branch: self.branch,
            created_at: self.created_at,
            _state: PhantomData,
        })
    }

    pub fn transition_branch(&self, new_branch: BranchState) -> Result<Self, SessionError> {
        if !self.branch.can_transition_to(&new_branch) {
            return Err(SessionError::InvalidBranchTransition {
                from: format!("{:?}", self.branch),
                to: format!("{:?}", new_branch),
            });
        }

        Ok(Self {
            id: self.id.clone(),
            name: self.name.clone(),
            workspace: self.workspace.clone(),
            bead: self.bead.clone(),
            branch: new_branch,
            created_at: self.created_at,
            _state: PhantomData,
        })
    }
}

impl Session<Active> {
    pub fn sync(self) -> Result<Session<Syncing>, SessionError> {
        self.transition_impl(SessionState::Syncing)
    }

    pub fn pause(self) -> Result<Session<Paused>, SessionError> {
        self.transition_impl(SessionState::Paused)
    }

    pub fn complete(self) -> Result<Session<Completed>, SessionError> {
        self.transition_impl(SessionState::Completed)
    }

    pub fn fail(self) -> Result<Session<Failed>, SessionError> {
        self.transition_impl(SessionState::Failed)
    }
}

impl Session<Syncing> {
    pub fn sync_complete(self) -> Result<Session<Synced>, SessionError> {
        self.transition_impl(SessionState::Synced)
    }

    pub fn fail(self) -> Result<Session<Failed>, SessionError> {
        self.transition_impl(SessionState::Failed)
    }
}

impl Session<Synced> {
    pub fn reactivate(self) -> Result<Session<Active>, SessionError> {
        self.transition_impl(SessionState::Active)
    }

    pub fn complete(self) -> Result<Session<Completed>, SessionError> {
        self.transition_impl(SessionState::Completed)
    }

    pub fn pause(self) -> Result<Session<Paused>, SessionError> {
        self.transition_impl(SessionState::Paused)
    }
}

impl Session<Paused> {
    pub fn resume(self) -> Result<Session<Active>, SessionError> {
        self.transition_impl(SessionState::Active)
    }

    pub fn fail(self) -> Result<Session<Failed>, SessionError> {
        self.transition_impl(SessionState::Failed)
    }
}

impl Session<Completed> {
    pub fn restart(self) -> Result<Session<Created>, SessionError> {
        Ok(Session {
            id: self.id,
            name: self.name,
            workspace: self.workspace,
            bead: self.bead,
            branch: self.branch,
            created_at: self.created_at,
            _state: PhantomData,
        })
    }
}

impl Session<Failed> {
    pub fn retry(self) -> Result<Session<Created>, SessionError> {
        Ok(Session {
            id: self.id,
            name: self.name,
            workspace: self.workspace,
            bead: self.bead,
            branch: self.branch,
            created_at: self.created_at,
            _state: PhantomData,
        })
    }
}

impl<S: SealedActive> Session<S> {
    #[must_use]
    pub fn is_active(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_created_has_created_state() {
        let name = SessionName::parse("test-session").expect("valid");
        let session = Session::<Created>::create(name).expect("created");
        assert!(matches!(session.branch, BranchState::Detached));
    }

    #[test]
    fn test_session_state_transitions() {
        let name = SessionName::parse("test").expect("valid");
        let session = Session::<Created>::create(name).expect("created");

        let active: Session<Active> = session.activate().expect("valid transition");
        let syncing: Session<Syncing> = active.sync().expect("valid transition");
        let synced: Session<Synced> = syncing.sync_complete().expect("valid transition");
        let completed: Session<Completed> = synced.complete().expect("valid transition");
        assert!(completed.is_active() == false);
    }

    #[test]
    fn test_branch_transition() {
        let name = SessionName::parse("test").expect("valid");
        let session = Session::<Created>::create(name).expect("created");

        let on_main: Session<Created> = session
            .transition_branch(BranchState::OnBranch {
                name: "main".into(),
            })
            .expect("valid");
        assert_eq!(on_main.branch.branch_name(), Some("main"));

        let detached: Session<Created> = on_main
            .transition_branch(BranchState::Detached)
            .expect("valid");
        assert!(detached.branch.is_detached());
    }
}
