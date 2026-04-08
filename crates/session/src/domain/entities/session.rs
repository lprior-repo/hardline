#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::type_complexity)]
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
            Self::OnBranch { name } => write!(f, "{name}"),
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

    #[must_use]
    pub fn generate() -> Self {
        Self(format!("session-{}", uuid::Uuid::new_v4()))
    }

    #[must_use]
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

/// Typestate marker trait to get `SessionState` from the marker type
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

/// Typestate marker trait for active states (where `is_active` returns true)
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
    pub last_synced: Option<DateTime<Utc>>,
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
            last_synced: None,
            created_at: Utc::now(),
            _state: PhantomData,
        })
    }

    /// Create a Session from parsed components (used by repository)
    #[must_use]
    pub fn from_parts(
        id: SessionId,
        name: SessionName,
        workspace: Option<WorkspaceId>,
        bead: Option<BeadId>,
        branch: BranchState,
        last_synced: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            workspace,
            bead,
            branch,
            last_synced,
            created_at,
            _state: PhantomData,
        }
    }

    pub fn activate(self) -> Result<Session<Active>, SessionError> {
        self.transition_impl()
    }

    pub fn fail(self) -> Result<Session<Failed>, SessionError> {
        self.transition_impl()
    }
}

impl<S: StateInfo> Session<S> {
    #[must_use]
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &SessionName {
        &self.name
    }

    #[must_use]
    pub fn workspace(&self) -> Option<&WorkspaceId> {
        self.workspace.as_ref()
    }

    #[must_use]
    pub fn bead(&self) -> Option<&BeadId> {
        self.bead.as_ref()
    }

    #[must_use]
    pub fn branch(&self) -> &BranchState {
        &self.branch
    }

    #[must_use]
    pub fn state(&self) -> SessionState {
        S::state()
    }

    fn transition_impl<T: StateInfo>(self) -> Result<Session<T>, SessionError> {
        Ok(Session {
            id: self.id,
            name: self.name,
            workspace: self.workspace,
            bead: self.bead,
            branch: self.branch,
            last_synced: self.last_synced,
            created_at: self.created_at,
            _state: PhantomData,
        })
    }

    pub fn transition_branch(&self, new_branch: BranchState) -> Result<Self, SessionError> {
        if !self.branch.can_transition_to(&new_branch) {
            return Err(SessionError::InvalidBranchTransition {
                from: format!("{:?}", self.branch),
                to: format!("{new_branch:?}"),
            });
        }

        Ok(Self {
            id: self.id.clone(),
            name: self.name.clone(),
            workspace: self.workspace.clone(),
            bead: self.bead.clone(),
            branch: new_branch,
            last_synced: self.last_synced,
            created_at: self.created_at,
            _state: PhantomData,
        })
    }

    /// Record a sync timestamp.
    ///
    /// # Errors
    ///
    /// Returns `SessionError` if the session data is invalid (unlikely in practice).
    pub fn mark_synced(&self, timestamp: DateTime<Utc>) -> Result<Self, SessionError> {
        Ok(Self {
            id: self.id.clone(),
            name: self.name.clone(),
            workspace: self.workspace.clone(),
            bead: self.bead.clone(),
            branch: self.branch.clone(),
            last_synced: Some(timestamp),
            created_at: self.created_at,
            _state: PhantomData,
        })
    }
}

impl Session<Active> {
    pub fn sync(self) -> Result<Session<Syncing>, SessionError> {
        self.transition_impl()
    }

    pub fn pause(self) -> Result<Session<Paused>, SessionError> {
        self.transition_impl()
    }

    pub fn complete(self) -> Result<Session<Completed>, SessionError> {
        self.transition_impl()
    }

    pub fn fail(self) -> Result<Session<Failed>, SessionError> {
        self.transition_impl()
    }
}

impl Session<Syncing> {
    pub fn sync_complete(self) -> Result<Session<Synced>, SessionError> {
        self.transition_impl()
    }

    pub fn fail(self) -> Result<Session<Failed>, SessionError> {
        self.transition_impl()
    }
}

impl Session<Synced> {
    pub fn reactivate(self) -> Result<Session<Active>, SessionError> {
        self.transition_impl()
    }

    pub fn complete(self) -> Result<Session<Completed>, SessionError> {
        self.transition_impl()
    }

    pub fn pause(self) -> Result<Session<Paused>, SessionError> {
        self.transition_impl()
    }
}

impl Session<Paused> {
    pub fn resume(self) -> Result<Session<Active>, SessionError> {
        self.transition_impl()
    }

    pub fn fail(self) -> Result<Session<Failed>, SessionError> {
        self.transition_impl()
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
            last_synced: self.last_synced,
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
            last_synced: self.last_synced,
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
        assert!(completed.state().is_terminal());
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

    // ==================================================================
    // SessionId Tests
    // ==================================================================

    mod session_id_tests {
        use super::*;

        #[test]
        fn session_id_parse_valid_ascii() {
            let id = SessionId::parse("session-abc123").expect("valid");
            assert_eq!(id.as_str(), "session-abc123");
        }

        #[test]
        fn session_id_parse_empty_rejects() {
            let result = SessionId::parse("");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidIdentifier(_)
            ));
        }

        #[test]
        fn session_id_parse_non_ascii_rejects() {
            let result = SessionId::parse("session-café");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidIdentifier(_)
            ));
        }

        #[test]
        fn session_id_parse_single_char() {
            let id = SessionId::parse("x").expect("valid single char");
            assert_eq!(id.as_str(), "x");
        }

        #[test]
        fn session_id_generate_produces_valid_id() {
            let id = SessionId::generate();
            assert!(id.as_str().starts_with("session-"));
            assert!(!id.as_str().is_empty());
        }

        #[test]
        fn session_id_generate_is_unique() {
            let id1 = SessionId::generate();
            let id2 = SessionId::generate();
            assert_ne!(id1, id2);
        }

        #[test]
        fn session_id_display_shows_inner_value() {
            let id = SessionId::parse("my-session-id").expect("valid");
            assert_eq!(format!("{id}"), "my-session-id");
        }

        #[test]
        fn session_id_try_from_string() {
            let id = SessionId::try_from("test-id".to_string()).expect("valid");
            assert_eq!(id.as_str(), "test-id");
        }

        #[test]
        fn session_id_try_from_empty_string_fails() {
            let result = SessionId::try_from("".to_string());
            assert!(result.is_err());
        }

        #[test]
        fn session_id_equality() {
            let id1 = SessionId::parse("same-id").expect("valid");
            let id2 = SessionId::parse("same-id").expect("valid");
            let id3 = SessionId::parse("other-id").expect("valid");
            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }

        #[test]
        fn session_id_hash_consistency() {
            use std::collections::HashSet;
            let id1 = SessionId::parse("hash-test").expect("valid");
            let id2 = SessionId::parse("hash-test").expect("valid");
            let mut set = HashSet::new();
            set.insert(id1);
            assert!(set.contains(&id2));
        }
    }

    // ==================================================================
    // SessionState Tests
    // ==================================================================

    mod session_state_tests {
        use super::*;

        #[test]
        fn session_state_is_terminal_completed() {
            assert!(SessionState::Completed.is_terminal());
        }

        #[test]
        fn session_state_is_terminal_failed() {
            assert!(SessionState::Failed.is_terminal());
        }

        #[test]
        fn session_state_is_not_terminal_created() {
            assert!(!SessionState::Created.is_terminal());
        }

        #[test]
        fn session_state_is_not_terminal_active() {
            assert!(!SessionState::Active.is_terminal());
        }

        #[test]
        fn session_state_is_not_terminal_syncing() {
            assert!(!SessionState::Syncing.is_terminal());
        }

        #[test]
        fn session_state_is_not_terminal_synced() {
            assert!(!SessionState::Synced.is_terminal());
        }

        #[test]
        fn session_state_is_not_terminal_paused() {
            assert!(!SessionState::Paused.is_terminal());
        }

        #[test]
        fn session_state_serde_serialize_deserialize_roundtrip() {
            let states = [
                SessionState::Created,
                SessionState::Active,
                SessionState::Syncing,
                SessionState::Synced,
                SessionState::Paused,
                SessionState::Completed,
                SessionState::Failed,
            ];
            for state in states {
                let json = serde_json::to_string(&state).expect("serialize");
                let parsed: SessionState = serde_json::from_str(&json).expect("deserialize");
                assert_eq!(state, parsed, "Roundtrip failed for {:?}", state);
            }
        }

        #[test]
        fn session_state_serde_uses_lowercase() {
            let json = serde_json::to_string(&SessionState::Created).expect("serialize");
            assert_eq!(json, "\"created\"");
        }
    }

    // ==================================================================
    // BranchState Tests
    // ==================================================================

    mod branch_state_tests {
        use super::*;

        #[test]
        fn branch_state_detached_branch_name_is_none() {
            let bs = BranchState::Detached;
            assert!(bs.branch_name().is_none());
            assert!(bs.is_detached());
        }

        #[test]
        fn branch_state_on_branch_branch_name_is_some() {
            let bs = BranchState::OnBranch {
                name: "feature-1".to_string(),
            };
            assert_eq!(bs.branch_name(), Some("feature-1"));
            assert!(!bs.is_detached());
        }

        #[test]
        fn branch_state_detached_to_on_branch_is_valid() {
            let from = BranchState::Detached;
            let to = BranchState::OnBranch {
                name: "main".into(),
            };
            assert!(from.can_transition_to(&to));
        }

        #[test]
        fn branch_state_on_branch_to_detached_is_valid() {
            let from = BranchState::OnBranch {
                name: "main".into(),
            };
            let to = BranchState::Detached;
            assert!(from.can_transition_to(&to));
        }

        #[test]
        fn branch_state_on_branch_to_on_branch_is_valid() {
            let from = BranchState::OnBranch { name: "old".into() };
            let to = BranchState::OnBranch { name: "new".into() };
            assert!(from.can_transition_to(&to));
        }

        #[test]
        fn branch_state_detached_to_detached_is_invalid() {
            let from = BranchState::Detached;
            let to = BranchState::Detached;
            assert!(!from.can_transition_to(&to));
        }

        #[test]
        fn branch_state_display_detached() {
            let bs = BranchState::Detached;
            assert_eq!(format!("{bs}"), "detached");
        }

        #[test]
        fn branch_state_display_on_branch() {
            let bs = BranchState::OnBranch {
                name: "my-feature".into(),
            };
            assert_eq!(format!("{bs}"), "my-feature");
        }

        #[test]
        fn branch_state_serde_roundtrip() {
            let states = [
                BranchState::Detached,
                BranchState::OnBranch {
                    name: "main".into(),
                },
            ];
            for state in &states {
                let json = serde_json::to_string(state).expect("serialize");
                let parsed: BranchState = serde_json::from_str(&json).expect("deserialize");
                assert_eq!(state, &parsed);
            }
        }

        #[test]
        fn branch_state_equality() {
            let bs1 = BranchState::OnBranch {
                name: "same".into(),
            };
            let bs2 = BranchState::OnBranch {
                name: "same".into(),
            };
            let bs3 = BranchState::OnBranch {
                name: "different".into(),
            };
            assert_eq!(bs1, bs2);
            assert_ne!(bs1, bs3);
        }
    }

    // ==================================================================
    // Session Typestate Lifecycle Tests
    // ==================================================================

    mod session_lifecycle_tests {
        use super::*;

        #[test]
        fn session_create_starts_with_created_state() {
            let name = SessionName::parse("test-session").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            assert_eq!(session.state(), SessionState::Created);
            assert!(session.workspace().is_none());
            assert!(session.bead().is_none());
            assert!(session.last_synced.is_none());
        }

        #[test]
        fn session_create_generates_non_empty_id() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            assert!(!session.id.as_str().is_empty());
        }

        #[test]
        fn session_from_parts_preserves_values() {
            let id = SessionId::parse("preset-id").expect("valid");
            let name = SessionName::parse("preset-name").expect("valid");
            let ws = WorkspaceId::parse("ws-test").expect("valid");
            let bd = BeadId::parse("bd-abc123").expect("valid");
            let branch = BranchState::OnBranch { name: "dev".into() };
            let created_at = chrono::Utc::now();

            let session = Session::from_parts(
                id.clone(),
                name.clone(),
                Some(ws.clone()),
                Some(bd.clone()),
                branch.clone(),
                None,
                created_at,
            );

            assert_eq!(session.id.as_str(), "preset-id");
            assert_eq!(session.name.as_str(), "preset-name");
            assert_eq!(session.workspace().map(|w| w.as_str()), Some("ws-test"));
            assert_eq!(session.bead().map(|b| b.as_str()), Some("bd-abc123"));
            assert_eq!(session.branch.branch_name(), Some("dev"));
        }

        #[test]
        fn session_activate_transition() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            assert_eq!(active.state(), SessionState::Active);
        }

        #[test]
        fn session_active_is_active() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            assert!(active.is_active());
        }

        #[test]
        fn session_syncing_is_active() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let syncing = active.sync().expect("sync");
            assert!(syncing.is_active());
        }

        #[test]
        fn session_synced_is_active() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync complete");
            assert!(synced.is_active());
        }

        #[test]
        fn session_pause_and_resume_path() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let paused = active.pause().expect("pause");
            assert_eq!(paused.state(), SessionState::Paused);

            let resumed = paused.resume().expect("resume");
            assert_eq!(resumed.state(), SessionState::Active);
        }

        #[test]
        fn session_synced_pause_and_resume() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync complete");
            let paused = synced.pause().expect("pause");
            assert_eq!(paused.state(), SessionState::Paused);
        }

        #[test]
        fn session_complete_from_active() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let completed = active.complete().expect("complete");
            assert_eq!(completed.state(), SessionState::Completed);
            assert!(completed.state().is_terminal());
        }

        #[test]
        fn session_complete_from_synced() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync complete");
            let completed = synced.complete().expect("complete");
            assert_eq!(completed.state(), SessionState::Completed);
        }

        #[test]
        fn session_fail_from_created() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let failed = session.fail().expect("fail");
            assert_eq!(failed.state(), SessionState::Failed);
            assert!(failed.state().is_terminal());
        }

        #[test]
        fn session_fail_from_active() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let failed = active.fail().expect("fail");
            assert_eq!(failed.state(), SessionState::Failed);
        }

        #[test]
        fn session_fail_from_syncing() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let syncing = active.sync().expect("sync");
            let failed = syncing.fail().expect("fail");
            assert_eq!(failed.state(), SessionState::Failed);
        }

        #[test]
        fn session_fail_from_paused() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let paused = active.pause().expect("pause");
            let failed = paused.fail().expect("fail");
            assert_eq!(failed.state(), SessionState::Failed);
        }

        #[test]
        fn session_restart_from_completed() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let completed = active.complete().expect("complete");
            let restarted = completed.restart().expect("restart");
            assert_eq!(restarted.state(), SessionState::Created);
        }

        #[test]
        fn session_retry_from_failed() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let failed = session.fail().expect("fail");
            let retried = failed.retry().expect("retry");
            assert_eq!(retried.state(), SessionState::Created);
        }

        #[test]
        fn session_reactivate_from_synced() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("activate");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync complete");
            let reactivated = synced.reactivate().expect("reactivate");
            assert_eq!(reactivated.state(), SessionState::Active);
        }

        #[test]
        fn session_detached_to_detached_branch_rejects() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let result = session.transition_branch(BranchState::Detached);
            assert!(result.is_err());
            match result {
                Err(SessionError::InvalidBranchTransition { .. }) => {}
                Err(e) => panic!("Expected InvalidBranchTransition, got {:?}", e),
                Ok(_) => panic!("Expected error, got Ok"),
            }
        }

        #[test]
        fn session_transition_branch_on_branch_to_on_branch() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let on_dev = session
                .transition_branch(BranchState::OnBranch { name: "dev".into() })
                .expect("valid");
            let on_main = on_dev
                .transition_branch(BranchState::OnBranch {
                    name: "main".into(),
                })
                .expect("valid");
            assert_eq!(on_main.branch.branch_name(), Some("main"));
        }

        #[test]
        fn session_mark_synced_sets_timestamp() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            assert!(session.last_synced.is_none());

            let timestamp = chrono::Utc::now();
            let synced = session.mark_synced(timestamp).expect("mark_synced");
            assert_eq!(synced.last_synced, Some(timestamp));
        }

        #[test]
        fn session_id_preserved_through_transitions() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let original_id = session.id.as_str().to_string();

            let active = session.activate().expect("activate");
            assert_eq!(active.id.as_str(), original_id);
        }

        #[test]
        fn session_name_preserved_through_transitions() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let original_name = session.name.as_str().to_string();

            let active = session.activate().expect("activate");
            assert_eq!(active.name.as_str(), original_name);
        }

        #[test]
        fn session_transition_impl_changes_state_for_every_single_step() {
            let name = SessionName::parse("test").expect("valid");

            // Created -> Active
            let created = Session::<Created>::create(name.clone()).expect("created");
            assert_eq!(created.state(), SessionState::Created);
            let active = created.activate().expect("activate");
            assert_eq!(active.state(), SessionState::Active);

            // Active -> Syncing
            let syncing = active.sync().expect("sync");
            assert_eq!(syncing.state(), SessionState::Syncing);

            // Syncing -> Synced
            let synced = syncing.sync_complete().expect("sync_complete");
            assert_eq!(synced.state(), SessionState::Synced);

            // Synced -> Active (reactivate)
            let reactivated = synced.reactivate().expect("reactivate");
            assert_eq!(reactivated.state(), SessionState::Active);

            // Active -> Paused
            let paused = reactivated.pause().expect("pause");
            assert_eq!(paused.state(), SessionState::Paused);

            // Paused -> Active (resume)
            let resumed = paused.resume().expect("resume");
            assert_eq!(resumed.state(), SessionState::Active);

            // Active -> Completed
            let completed = resumed.complete().expect("complete");
            assert_eq!(completed.state(), SessionState::Completed);

            // Completed -> Created (restart)
            let restarted = completed.restart().expect("restart");
            assert_eq!(restarted.state(), SessionState::Created);

            // Created -> Failed (direct fail from created)
            let failed = restarted.fail().expect("fail");
            assert_eq!(failed.state(), SessionState::Failed);

            // Failed -> Created (retry)
            let retried = failed.retry().expect("retry");
            assert_eq!(retried.state(), SessionState::Created);
        }

        #[test]
        fn session_all_field_data_preserved_through_full_lifecycle() {
            let name = SessionName::parse("lifecycle-test").expect("valid");
            let created = Session::<Created>::create(name).expect("created");
            let id_before = created.id.as_str().to_string();
            let name_before = created.name.as_str().to_string();
            let ts = chrono::Utc::now();

            // Set branch and mark synced
            let created = created
                .transition_branch(BranchState::OnBranch {
                    name: "feature".into(),
                })
                .expect("branch");
            let created = created.mark_synced(ts).expect("sync");

            let active = created.activate().expect("activate");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync_complete");
            let completed = synced.complete().expect("complete");

            assert_eq!(completed.id.as_str(), id_before);
            assert_eq!(completed.name.as_str(), name_before);
            assert_eq!(completed.branch.branch_name(), Some("feature"));
            assert_eq!(completed.last_synced, Some(ts));
        }

        #[test]
        fn session_fail_from_every_possible_state() {
            let name = SessionName::parse("fail-test").expect("valid");

            // Fail from Created
            let s = Session::<Created>::create(name.clone()).expect("created");
            assert_eq!(s.fail().expect("fail").state(), SessionState::Failed);

            // Fail from Active
            let s = Session::<Created>::create(name.clone()).expect("created");
            let s = s.activate().expect("active");
            assert_eq!(s.fail().expect("fail").state(), SessionState::Failed);

            // Fail from Syncing
            let s = Session::<Created>::create(name.clone()).expect("created");
            let s = s.activate().expect("active").sync().expect("syncing");
            assert_eq!(s.fail().expect("fail").state(), SessionState::Failed);

            // Fail from Paused
            let s = Session::<Created>::create(name).expect("created");
            let s = s.activate().expect("active").pause().expect("paused");
            assert_eq!(s.fail().expect("fail").state(), SessionState::Failed);
        }

        #[test]
        fn session_state_info_trait_provides_correct_state_for_all_markers() {
            assert_eq!(Created::state(), SessionState::Created);
            assert_eq!(Active::state(), SessionState::Active);
            assert_eq!(Syncing::state(), SessionState::Syncing);
            assert_eq!(Synced::state(), SessionState::Synced);
            assert_eq!(Paused::state(), SessionState::Paused);
            assert_eq!(Completed::state(), SessionState::Completed);
            assert_eq!(Failed::state(), SessionState::Failed);
        }
    }

    // ==================================================================
    // State Machine Transition Path Tests (ha-bo9)
    // ==================================================================

    mod state_machine_transition_tests {
        use super::*;
        use crate::domain::events::{SessionEvent, serialize_event};

        /// Path 1: Created → Active → Completed
        /// Verifies state correctness and event correspondence at each step.
        #[test]
        fn transition_path_created_active_completed() {
            let name = SessionName::parse("sm-path-1").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            assert_eq!(session.state(), SessionState::Created);
            assert!(!session.state().is_terminal());

            // Created → Active: state changes, Activated event corresponds
            let active = session.activate().expect("activate");
            assert_eq!(active.state(), SessionState::Active);
            assert!(active.is_active());
            let activated_evt = serialize_event(&SessionEvent::Activated).expect("serialize");
            assert_eq!(activated_evt, "\"Activated\"");

            // Active → Completed: state changes to terminal, Completed event corresponds
            let completed = active.complete().expect("complete");
            assert_eq!(completed.state(), SessionState::Completed);
            assert!(completed.state().is_terminal());
            let completed_evt = serialize_event(&SessionEvent::Completed).expect("serialize");
            assert_eq!(completed_evt, "\"Completed\"");
        }

        /// Path 2: Created → Active → Failed
        /// Verifies state correctness and event correspondence at each step.
        #[test]
        fn transition_path_created_active_failed() {
            let name = SessionName::parse("sm-path-2").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            assert_eq!(session.state(), SessionState::Created);

            // Created → Active
            let active = session.activate().expect("activate");
            assert_eq!(active.state(), SessionState::Active);
            assert!(active.is_active());

            // Active → Failed: state changes to terminal, Failed event corresponds
            let failed = active.fail().expect("fail");
            assert_eq!(failed.state(), SessionState::Failed);
            assert!(failed.state().is_terminal());
            let failed_evt = serialize_event(&SessionEvent::Failed).expect("serialize");
            assert_eq!(failed_evt, "\"Failed\"");
        }

        /// Path 3: Active → Paused (Suspended) → Active
        /// Verifies state correctness and event correspondence at each step.
        #[test]
        fn transition_path_active_paused_active() {
            let name = SessionName::parse("sm-path-3").expect("valid");
            let session = Session::<Created>::create(name).expect("created");

            // Created → Active
            let active = session.activate().expect("activate");
            assert_eq!(active.state(), SessionState::Active);
            assert!(active.is_active());

            // Active → Paused (Suspended)
            let paused = active.pause().expect("pause");
            assert_eq!(paused.state(), SessionState::Paused);
            assert!(!paused.state().is_terminal());
            let paused_evt = serialize_event(&SessionEvent::Paused).expect("serialize");
            assert_eq!(paused_evt, "\"Paused\"");

            // Paused → Active (resume)
            let resumed = paused.resume().expect("resume");
            assert_eq!(resumed.state(), SessionState::Active);
            assert!(resumed.is_active());
            let resumed_evt = serialize_event(&SessionEvent::Activated).expect("serialize");
            assert_eq!(resumed_evt, "\"Activated\"");
        }

        /// Verify identity preservation across all three transition paths.
        #[test]
        fn transition_paths_preserve_session_identity() {
            let name = SessionName::parse("identity-check").expect("valid");
            let original_id = {
                let s = Session::<Created>::create(name.clone()).expect("created");
                s.id.as_str().to_string()
            };

            // Path 1: Created→Active→Completed
            let s = Session::<Created>::create(name.clone()).expect("created");
            let id1 = s.id.as_str().to_string();
            let completed = s.activate().expect("a").complete().expect("c");
            assert_eq!(completed.id.as_str(), id1);
            assert_eq!(completed.name.as_str(), "identity-check");

            // Path 2: Created→Active→Failed
            let s = Session::<Created>::create(name.clone()).expect("created");
            let id2 = s.id.as_str().to_string();
            let failed = s.activate().expect("a").fail().expect("f");
            assert_eq!(failed.id.as_str(), id2);
            assert_eq!(failed.name.as_str(), "identity-check");

            // Path 3: Active→Paused→Active
            let s = Session::<Created>::create(name).expect("created");
            let id3 = s.id.as_str().to_string();
            let resumed = s.activate().expect("a").pause().expect("p").resume().expect("r");
            assert_eq!(resumed.id.as_str(), id3);
            assert_eq!(resumed.name.as_str(), "identity-check");

            // All three sessions have unique IDs
            assert_ne!(id1, original_id);
            assert_ne!(id2, original_id);
            assert_ne!(id1, id2);
            assert_ne!(id1, id3);
        }
    }

    // ==================================================================
    // Session Entity Proptests
    // ==================================================================

    mod session_proptests {
        use super::*;
        use proptest::proptest;
        use proptest::{prop_assert, prop_assert_eq, prop_assert_ne};

        proptest! {
            /// SessionId must reject empty strings
            #[test]
            fn prop_session_id_empty_rejects(s in "[a-z]*") {
                // Only test the empty case within generated strings
                if s.is_empty() {
                    prop_assert!(SessionId::parse(s).is_err());
                }
            }

            /// SessionId generate always starts with "session-" and is ASCII
            #[test]
            fn prop_session_id_generate_invariant(_ in 0u8..1) {
                let id = SessionId::generate();
                prop_assert!(id.as_str().starts_with("session-"));
                prop_assert!(id.as_str().is_ascii());
                prop_assert!(id.as_str().len() > "session-".len());
            }

            /// SessionId generate produces unique values
            #[test]
            fn prop_session_id_generate_unique(_ in 0u8..10) {
                let id1 = SessionId::generate();
                let id2 = SessionId::generate();
                prop_assert_ne!(id1, id2);
            }

            /// Session ID is preserved through every lifecycle transition
            #[test]
            fn prop_session_id_preserved_through_all_transitions(
                name_str in "[a-z][a-z0-9_-]{1,10}"
            ) {
                let name = SessionName::parse(name_str).unwrap();
                let session = Session::<Created>::create(name).unwrap();
                let id = session.id.as_str().to_string();

                let active = session.activate().unwrap();
                prop_assert_eq!(active.id.as_str(), id.as_str());

                let syncing = active.sync().unwrap();
                prop_assert_eq!(syncing.id.as_str(), id.as_str());

                let synced = syncing.sync_complete().unwrap();
                prop_assert_eq!(synced.id.as_str(), id.as_str());

                let completed = synced.complete().unwrap();
                prop_assert_eq!(completed.id.as_str(), id.as_str());

                let restarted = completed.restart().unwrap();
                prop_assert_eq!(restarted.id.as_str(), id.as_str());
            }

            /// Session name is preserved through all transitions
            #[test]
            fn prop_session_name_preserved_through_transitions(
                name_str in "[a-z][a-z0-9_-]{1,10}"
            ) {
                let name = SessionName::parse(name_str.clone()).unwrap();
                let session = Session::<Created>::create(name).unwrap();
                let original_name = session.name.as_str().to_string();

                let active = session.activate().unwrap();
                prop_assert_eq!(active.name.as_str(), original_name.as_str());

                let syncing = active.sync().unwrap();
                prop_assert_eq!(syncing.name.as_str(), original_name.as_str());

                let paused = syncing.fail().unwrap();
                prop_assert_eq!(paused.name.as_str(), original_name.as_str());
            }
        }
    }

    // ==================================================================
    // Session Entity Edge Cases
    // ==================================================================

    mod session_edge_case_tests {
        use super::*;

        #[test]
        fn session_create_with_max_length_name() {
            let max_name = "a".repeat(SessionName::MAX_LENGTH);
            let name = SessionName::parse(&max_name).expect("valid at max length");
            let session = Session::<Created>::create(name).expect("created");
            assert_eq!(session.name.as_str().len(), SessionName::MAX_LENGTH);
        }

        #[test]
        fn session_create_with_min_length_name() {
            let name = SessionName::parse("a").expect("single char valid");
            let session = Session::<Created>::create(name).expect("created");
            assert_eq!(session.name.as_str(), "a");
        }

        #[test]
        fn session_create_name_trimmed() {
            let name = SessionName::parse("  padded  ").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            assert_eq!(session.name.as_str(), "padded");
        }

        #[test]
        fn session_mark_synced_preserves_other_fields() {
            let name = SessionName::parse("sync-test").expect("valid");
            let created = Session::<Created>::create(name).expect("created");
            let branched = created
                .transition_branch(BranchState::OnBranch {
                    name: "main".into(),
                })
                .expect("branch");

            let ts = chrono::Utc::now();
            let synced = branched.mark_synced(ts).expect("mark synced");

            assert_eq!(synced.last_synced, Some(ts));
            assert_eq!(synced.branch.branch_name(), Some("main"));
        }

        #[test]
        fn session_mark_synced_overwrites_previous() {
            let name = SessionName::parse("overwrite").expect("valid");
            let session = Session::<Created>::create(name).expect("created");

            let ts1 = chrono::Utc::now() - chrono::Duration::hours(1);
            let ts2 = chrono::Utc::now();
            let synced1 = session.mark_synced(ts1).expect("first sync");
            let synced2 = synced1.mark_synced(ts2).expect("second sync");

            assert_eq!(synced2.last_synced, Some(ts2));
            assert_ne!(synced2.last_synced, synced1.last_synced);
        }

        #[test]
        fn session_mark_synced_with_past_timestamp() {
            let name = SessionName::parse("past-sync").expect("valid");
            let session = Session::<Created>::create(name).expect("created");

            let past = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00+00:00")
                .unwrap()
                .with_timezone(&chrono::Utc);
            let synced = session.mark_synced(past).expect("sync with past");
            assert_eq!(synced.last_synced, Some(past));
        }

        #[test]
        fn session_mark_synced_with_future_timestamp() {
            let name = SessionName::parse("future-sync").expect("valid");
            let session = Session::<Created>::create(name).expect("created");

            let future = chrono::Utc::now() + chrono::Duration::days(365);
            let synced = session.mark_synced(future).expect("sync with future");
            assert_eq!(synced.last_synced, Some(future));
        }

        #[test]
        fn session_multiple_creates_have_unique_ids() {
            let name = SessionName::parse("unique").expect("valid");
            let s1 = Session::<Created>::create(name.clone()).expect("s1");
            let s2 = Session::<Created>::create(name).expect("s2");
            let s3 = Session::<Created>::create(SessionName::parse("other").expect("valid"))
                .expect("s3");
            assert_ne!(s1.id, s2.id);
            assert_ne!(s2.id, s3.id);
            assert_ne!(s1.id, s3.id);
        }

        #[test]
        fn session_from_parts_with_all_fields() {
            let id = SessionId::parse("preset-id").expect("valid");
            let name = SessionName::parse("preset-name").expect("valid");
            let ws = WorkspaceId::parse("ws-test").expect("valid");
            let bd = BeadId::parse("bd-deadbeef").expect("valid");
            let branch = BranchState::OnBranch { name: "dev".into() };
            let ts = chrono::Utc::now();

            let session = Session::from_parts(
                id.clone(),
                name.clone(),
                Some(ws.clone()),
                Some(bd.clone()),
                branch.clone(),
                Some(ts),
                chrono::Utc::now(),
            );

            assert_eq!(session.id.as_str(), "preset-id");
            assert_eq!(session.name.as_str(), "preset-name");
            assert_eq!(session.workspace().map(|w| w.as_str()), Some("ws-test"));
            assert_eq!(session.bead().map(|b| b.as_str()), Some("bd-deadbeef"));
            assert_eq!(session.last_synced, Some(ts));
        }

        #[test]
        fn session_from_parts_with_no_optional_fields() {
            let session = Session::from_parts(
                SessionId::parse("minimal").expect("valid"),
                SessionName::parse("minimal-name").expect("valid"),
                None,
                None,
                BranchState::Detached,
                None,
                chrono::Utc::now(),
            );

            assert!(session.workspace().is_none());
            assert!(session.bead().is_none());
            assert!(session.last_synced.is_none());
            assert!(session.branch.is_detached());
        }

        #[test]
        fn session_restart_preserves_branch_state() {
            let name = SessionName::parse("restart-branch").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let branched = session
                .transition_branch(BranchState::OnBranch {
                    name: "feature".into(),
                })
                .expect("branch");
            let active = branched.activate().expect("activate");
            let completed = active.complete().expect("complete");
            let restarted = completed.restart().expect("restart");

            assert_eq!(restarted.branch.branch_name(), Some("feature"));
        }

        #[test]
        fn session_retry_preserves_branch_state() {
            let name = SessionName::parse("retry-branch").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let branched = session
                .transition_branch(BranchState::OnBranch {
                    name: "bugfix".into(),
                })
                .expect("branch");
            let failed = branched.fail().expect("fail");
            let retried = failed.retry().expect("retry");

            assert_eq!(retried.branch.branch_name(), Some("bugfix"));
        }

        #[test]
        fn session_created_state_methods() {
            let name = SessionName::parse("state-methods").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            assert!(!session.state().is_terminal());
        }

        #[test]
        fn session_completed_state_is_terminal() {
            let name = SessionName::parse("terminal").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let completed = session.activate().unwrap().complete().unwrap();
            assert!(completed.state().is_terminal());
        }

        #[test]
        fn session_failed_state_is_terminal() {
            let name = SessionName::parse("terminal-fail").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let failed = session.fail().unwrap();
            assert!(failed.state().is_terminal());
        }

        #[test]
        fn session_synced_reactivate_returns_active_state() {
            let name = SessionName::parse("reactivate").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let synced = session
                .activate()
                .unwrap()
                .sync()
                .unwrap()
                .sync_complete()
                .unwrap();
            let reactivated = synced.reactivate().expect("reactivate");
            assert_eq!(reactivated.state(), SessionState::Active);
            assert!(reactivated.is_active());
        }
    }

    // ==================================================================
    // SessionId Serde Tests
    // ==================================================================

    mod session_id_serde_tests {
        use super::*;

        #[test]
        fn session_id_serde_roundtrip() {
            let id = SessionId::parse("test-session-123").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: SessionId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
        }

        #[test]
        fn session_id_serde_json_output() {
            let id = SessionId::parse("my-id").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            assert_eq!(json, "\"my-id\"");
        }

        #[test]
        fn session_id_serde_preserves_equality() {
            let id1 = SessionId::parse("same").expect("valid");
            let json = serde_json::to_string(&id1).expect("serialize");
            let id2: SessionId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id1, id2);
        }
    }

    // ==================================================================
    // BranchState Serde Tests
    // ==================================================================

    mod branch_state_serde_extended_tests {
        use super::*;

        #[test]
        fn branch_state_serde_roundtrip_detached() {
            let bs = BranchState::Detached;
            let json = serde_json::to_string(&bs).expect("serialize");
            let parsed: BranchState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(bs, parsed);
        }

        #[test]
        fn branch_state_serde_roundtrip_on_branch() {
            let bs = BranchState::OnBranch {
                name: "feature-branch".into(),
            };
            let json = serde_json::to_string(&bs).expect("serialize");
            let parsed: BranchState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(bs, parsed);
            assert_eq!(parsed.branch_name(), Some("feature-branch"));
        }

        #[test]
        fn branch_state_serde_json_output_format() {
            let bs = BranchState::OnBranch {
                name: "main".into(),
            };
            let json = serde_json::to_string(&bs).expect("serialize");
            assert!(json.contains("main"));
        }
    }

    // ==================================================================
    // SessionState Proptests
    // ==================================================================

    mod session_state_proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Terminal states are only Completed and Failed
            #[test]
            fn prop_terminal_states_are_only_completed_and_failed(state in any::<u8>()) {
                let states = [
                    SessionState::Created,
                    SessionState::Active,
                    SessionState::Syncing,
                    SessionState::Synced,
                    SessionState::Paused,
                    SessionState::Completed,
                    SessionState::Failed,
                ];
                let idx = (state as usize) % states.len();
                let s = states[idx];
                let is_terminal = s.is_terminal();
                prop_assert_eq!(is_terminal, matches!(s, SessionState::Completed | SessionState::Failed));
            }

            /// SessionState serde roundtrip for all variants
            #[test]
            fn prop_session_state_serde_roundtrip(state_idx in 0u8..7u8) {
                let states = [
                    SessionState::Created,
                    SessionState::Active,
                    SessionState::Syncing,
                    SessionState::Synced,
                    SessionState::Paused,
                    SessionState::Completed,
                    SessionState::Failed,
                ];
                let state = states[state_idx as usize];
                let json = serde_json::to_string(&state).unwrap();
                let parsed: SessionState = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(state, parsed);
            }
        }
    }

    // ==================================================================
    // SessionId Extended Tests — generation, uniqueness, parse round-trip,
    // display formatting (ha-9j6)
    // ==================================================================

    mod session_id_extended_tests {
        use super::*;

        // --- Generation ---

        #[test]
        fn session_id_generate_has_uuid_suffix() {
            let id = SessionId::generate();
            let suffix = &id.as_str()["session-".len()..];
            // UUID v4 format: 8-4-4-4-12 (36 chars with hyphens)
            assert_eq!(suffix.len(), 36);
            assert!(suffix.contains('-'));
        }

        #[test]
        fn session_id_generate_batch_uniqueness() {
            let mut ids = std::collections::HashSet::new();
            for _ in 0..200 {
                ids.insert(SessionId::generate());
            }
            assert_eq!(ids.len(), 200);
        }

        #[test]
        fn session_id_generate_all_ascii() {
            for _ in 0..50 {
                let id = SessionId::generate();
                assert!(id.as_str().is_ascii());
            }
        }

        #[test]
        fn session_id_generate_parse_roundtrip() {
            let id = SessionId::generate();
            let parsed = SessionId::parse(id.as_str()).expect("generated id should parse");
            assert_eq!(id, parsed);
        }

        #[test]
        fn session_id_generate_display_parse_roundtrip() {
            let id = SessionId::generate();
            let displayed = format!("{id}");
            let reparsed = SessionId::parse(&displayed).expect("display output should parse");
            assert_eq!(id, reparsed);
        }

        // --- Parse round-trip ---

        #[test]
        fn session_id_parse_as_str_parse_roundtrip() {
            let original = "session-550e8400-e29b-41d4-a716-446655440000";
            let id = SessionId::parse(original).expect("valid");
            let roundtripped = SessionId::parse(id.as_str()).expect("round-trip");
            assert_eq!(id, roundtripped);
            assert_eq!(roundtripped.as_str(), original);
        }

        #[test]
        fn session_id_display_output_equals_as_str() {
            let id = SessionId::parse("test-display-id").expect("valid");
            assert_eq!(format!("{id}"), id.as_str());
        }

        #[test]
        fn session_id_display_parse_roundtrip_many() {
            for input in &[
                "a",
                "session-abc",
                "x-y-z",
                "UPPER",
                "mixed-Case_123",
                "with spaces",
                "with.dots",
                "with:colons",
                "with/slashes",
            ] {
                let id = SessionId::parse(*input).expect("valid");
                let displayed = format!("{id}");
                let reparsed = SessionId::parse(&displayed).expect("display round-trip");
                assert_eq!(id, reparsed, "round-trip failed for: {input}");
            }
        }

        // --- Parse valid ASCII formats ---

        #[test]
        fn session_id_parse_uuid_format() {
            let uuid = "session-550e8400-e29b-41d4-a716-446655440000";
            let id = SessionId::parse(uuid).expect("valid UUID format");
            assert_eq!(id.as_str(), uuid);
        }

        #[test]
        fn session_id_parse_with_spaces() {
            let id = SessionId::parse("session abc").expect("spaces are ASCII");
            assert_eq!(id.as_str(), "session abc");
        }

        #[test]
        fn session_id_parse_with_dots() {
            let id = SessionId::parse("session.abc.123").expect("dots are ASCII");
            assert_eq!(id.as_str(), "session.abc.123");
        }

        #[test]
        fn session_id_parse_with_colons() {
            let id = SessionId::parse("session:abc:123").expect("colons are ASCII");
            assert_eq!(id.as_str(), "session:abc:123");
        }

        #[test]
        fn session_id_parse_with_slashes() {
            let id = SessionId::parse("session/abc/123").expect("slashes are ASCII");
            assert_eq!(id.as_str(), "session/abc/123");
        }

        #[test]
        fn session_id_parse_with_special_ascii() {
            let id = SessionId::parse("!@#$%^&*()_+-=[]{}|;:',.<>?~`")
                .expect("printable ASCII special chars");
            assert_eq!(id.as_str(), "!@#$%^&*()_+-=[]{}|;:',.<>?~`");
        }

        #[test]
        fn session_id_parse_long_ascii_string() {
            let long = "a".repeat(1000);
            let id = SessionId::parse(&long).expect("long ASCII string accepted");
            assert_eq!(id.as_str().len(), 1000);
        }

        #[test]
        fn session_id_parse_single_digit() {
            let id = SessionId::parse("1").expect("single digit is valid ASCII");
            assert_eq!(id.as_str(), "1");
        }

        #[test]
        fn session_id_parse_all_printable_ascii_range() {
            // All printable ASCII chars (0x20-0x7E)
            let printable: String = (0x20u8..=0x7E).map(char::from).collect();
            let id = SessionId::parse(&printable).expect("all printable ASCII");
            assert_eq!(id.as_str(), printable);
        }

        // --- Parse invalid formats ---

        #[test]
        fn session_id_parse_empty_error_message() {
            let err = SessionId::parse("").unwrap_err();
            let msg = format!("{err}");
            assert!(
                msg.to_lowercase().contains("empty"),
                "error should mention empty: {msg}"
            );
        }

        #[test]
        fn session_id_parse_non_ascii_error_message() {
            let err = SessionId::parse("café").unwrap_err();
            let msg = format!("{err}");
            assert!(
                msg.to_lowercase().contains("ascii"),
                "error should mention ASCII: {msg}"
            );
        }

        #[test]
        fn session_id_parse_emoji_rejects() {
            let result = SessionId::parse("session-🎉");
            assert!(result.is_err());
        }

        #[test]
        fn session_id_parse_cjk_rejects() {
            let result = SessionId::parse("session-日本語");
            assert!(result.is_err());
        }

        #[test]
        fn session_id_parse_accented_chars_rejects() {
            assert!(SessionId::parse("café").is_err());
            assert!(SessionId::parse("naïve").is_err());
        }

        #[test]
        fn session_id_parse_mixed_ascii_and_unicode_rejects() {
            let result = SessionId::parse("abc\u{00E9}def");
            assert!(result.is_err());
        }

        // --- Clone ---

        #[test]
        fn session_id_clone_is_equal() {
            let id = SessionId::parse("clone-test").expect("valid");
            let cloned = id.clone();
            assert_eq!(id, cloned);
            assert_eq!(id.as_str(), cloned.as_str());
        }

        // --- Serde for generated IDs ---

        #[test]
        fn session_id_serde_roundtrip_generated() {
            let id = SessionId::generate();
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: SessionId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
        }

        #[test]
        fn session_id_serde_roundtrip_many() {
            for input in &["a", "session-abc", "with spaces", "x-y_z.1"] {
                let id = SessionId::parse(*input).expect("valid");
                let json = serde_json::to_string(&id).expect("serialize");
                let back: SessionId = serde_json::from_str(&json).expect("deserialize");
                assert_eq!(id, back, "serde roundtrip failed for: {input}");
            }
        }

        // --- Proptests ---

        mod session_id_proptests {
            use super::*;
            use proptest::prelude::*;

            proptest! {
                /// Any non-empty ASCII string should parse successfully
                #[test]
                fn prop_session_id_ascii_parse_ok(s in "[ -~]{1,50}") {
                    let result = SessionId::parse(&s);
                    prop_assert!(result.is_ok(), "ASCII string rejected: {:?}", s);
                    let id = result.unwrap();
                    prop_assert_eq!(id.as_str(), s);
                }

                /// Parse → display → parse round-trip for any valid ASCII string
                #[test]
                fn prop_session_id_display_roundtrip(s in "[ -~]{1,50}") {
                    let id = SessionId::parse(&s).unwrap();
                    let displayed = format!("{id}");
                    let reparsed = SessionId::parse(&displayed).unwrap();
                    prop_assert_eq!(id, reparsed);
                }

                /// Generated IDs are always parseable and unique
                #[test]
                fn prop_session_id_generate_parse_unique(_ in 0u8..20) {
                    let id = SessionId::generate();
                    let parsed = SessionId::parse(id.as_str()).unwrap();
                    prop_assert_eq!(id, parsed);
                }
            }

    // Session State Machine — Invalid Transition Tests (ha-v5x)
    // ==================================================================

    mod invalid_transition_tests {
        use super::*;

        /// The typestate pattern prevents invalid transitions at compile time.
        /// Each state type only exposes its valid transition methods.
        /// See tests/compile_fail/session_*.rs for compile-fail proofs.
        ///
        /// Valid transition matrix:
        ///   Created  → Active, Failed
        ///   Active   → Syncing, Paused, Completed, Failed
        ///   Syncing  → Synced, Failed
        ///   Synced   → Active, Completed, Paused
        ///   Paused   → Active, Failed
        ///   Completed→ Created (restart)
        ///   Failed   → Created (retry)

        // -- Created: only activate() and fail() are available --

        #[test]
        fn created_can_transition_to_active() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let active = session.activate().expect("Created→Active");
            assert_eq!(active.state(), SessionState::Active);
        }

        #[test]
        fn created_can_transition_to_failed() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let failed = session.fail().expect("Created→Failed");
            assert_eq!(failed.state(), SessionState::Failed);
            assert!(failed.state().is_terminal());
        }

        #[test]
        fn created_cannot_complete_must_activate_first() {
            // compile_fail: session_created_cannot_complete.rs
            // Created→Completed is blocked; must go Created→Active→Completed
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let completed = session.activate().expect("activate").complete().expect("complete");
            assert_eq!(completed.state(), SessionState::Completed);
        }

        #[test]
        fn created_cannot_sync_must_activate_first() {
            // compile_fail: session_created_cannot_sync.rs
            // Created→Syncing is blocked; must go Created→Active→Syncing
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let syncing = session.activate().expect("activate").sync().expect("sync");
            assert_eq!(syncing.state(), SessionState::Syncing);
        }

        // -- Completed: only restart() is available --

        #[test]
        fn completed_can_transition_to_created_via_restart() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let completed = session.activate().expect("activate").complete().expect("complete");
            let restarted = completed.restart().expect("Completed→Created");
            assert_eq!(restarted.state(), SessionState::Created);
        }

        #[test]
        fn completed_cannot_activate_must_restart_first() {
            // compile_fail: session_completed_cannot_activate.rs
            // Completed→Active is blocked; must go Completed→Created→Active
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let completed = session.activate().expect("activate").complete().expect("complete");
            let restarted = completed.restart().expect("restart");
            let reactivated = restarted.activate().expect("activate");
            assert_eq!(reactivated.state(), SessionState::Active);
        }

        #[test]
        fn completed_cannot_fail_terminal_state() {
            // compile_fail: session_completed_cannot_fail.rs
            // Completed is terminal; only exit is restart()
        }

        #[test]
        fn completed_is_terminal_and_has_only_restart() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let completed = session.activate().expect("activate").complete().expect("complete");
            assert!(completed.state().is_terminal());
            let restarted = completed.restart().expect("restart");
            assert!(!restarted.state().is_terminal());
        }

        // -- Failed: only retry() is available --

        #[test]
        fn failed_can_transition_to_created_via_retry() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let failed = session.fail().expect("fail");
            let retried = failed.retry().expect("Failed→Created");
            assert_eq!(retried.state(), SessionState::Created);
        }

        #[test]
        fn failed_cannot_pause_must_retry_first() {
            // compile_fail: session_failed_cannot_pause.rs
            // Failed→Paused is blocked; must go Failed→Created→Active→Paused
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let failed = session.fail().expect("fail");
            let retried = failed.retry().expect("retry");
            let activated = retried.activate().expect("activate");
            let paused = activated.pause().expect("pause");
            assert_eq!(paused.state(), SessionState::Paused);
        }

        #[test]
        fn failed_cannot_sync_must_retry_first() {
            // compile_fail: session_failed_cannot_sync.rs
            // Failed→Syncing is blocked; must go Failed→Created→Active→Syncing
        }

        #[test]
        fn failed_is_terminal_and_has_only_retry() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let failed = session.fail().expect("fail");
            assert!(failed.state().is_terminal());
            let retried = failed.retry().expect("retry");
            assert!(!retried.state().is_terminal());
        }

        // -- Paused: only resume() and fail() are available --

        #[test]
        fn paused_can_transition_to_active_via_resume() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let paused = session.activate().expect("activate").pause().expect("pause");
            let resumed = paused.resume().expect("Paused→Active");
            assert_eq!(resumed.state(), SessionState::Active);
        }

        #[test]
        fn paused_can_transition_to_failed() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let paused = session.activate().expect("activate").pause().expect("pause");
            let failed = paused.fail().expect("Paused→Failed");
            assert_eq!(failed.state(), SessionState::Failed);
        }

        #[test]
        fn paused_cannot_complete_must_resume_first() {
            // compile_fail: session_paused_cannot_complete.rs
            // Paused→Completed is blocked; must go Paused→Active→Completed
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let paused = session.activate().expect("activate").pause().expect("pause");
            let resumed = paused.resume().expect("resume");
            let completed = resumed.complete().expect("complete");
            assert_eq!(completed.state(), SessionState::Completed);
        }

        // -- Syncing: only sync_complete() and fail() are available --

        #[test]
        fn syncing_can_transition_to_synced() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let syncing = session.activate().expect("activate").sync().expect("sync");
            let synced = syncing.sync_complete().expect("Syncing→Synced");
            assert_eq!(synced.state(), SessionState::Synced);
        }

        #[test]
        fn syncing_can_transition_to_failed() {
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let syncing = session.activate().expect("activate").sync().expect("sync");
            let failed = syncing.fail().expect("Syncing→Failed");
            assert_eq!(failed.state(), SessionState::Failed);
        }

        #[test]
        fn syncing_cannot_complete_must_finish_sync_first() {
            // compile_fail: session_syncing_cannot_complete.rs
            // Syncing→Completed is blocked; must go Syncing→Synced→Completed
            let name = SessionName::parse("test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let syncing = session.activate().expect("activate").sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync_complete");
            let completed = synced.complete().expect("complete");
            assert_eq!(completed.state(), SessionState::Completed);
        }

        // -- Full valid transition matrix verification --

        #[test]
        fn valid_transition_matrix_created() {
            let name = SessionName::parse("matrix").expect("valid");
            // Created → Active ✓
            let s = Session::<Created>::create(name.clone()).expect("c");
            assert_eq!(s.activate().expect("→Active").state(), SessionState::Active);
            // Created → Failed ✓
            let s = Session::<Created>::create(name).expect("c");
            assert_eq!(s.fail().expect("→Failed").state(), SessionState::Failed);
        }

        #[test]
        fn valid_transition_matrix_active() {
            let name = SessionName::parse("matrix").expect("valid");
            // Active → Syncing ✓
            let s = Session::<Created>::create(name.clone()).expect("c");
            let a = s.activate().expect("a");
            assert_eq!(a.sync().expect("→Syncing").state(), SessionState::Syncing);
            // Active → Paused ✓
            let s = Session::<Created>::create(name.clone()).expect("c");
            let a = s.activate().expect("a");
            assert_eq!(a.pause().expect("→Paused").state(), SessionState::Paused);
            // Active → Completed ✓
            let s = Session::<Created>::create(name.clone()).expect("c");
            let a = s.activate().expect("a");
            assert_eq!(a.complete().expect("→Completed").state(), SessionState::Completed);
            // Active → Failed ✓
            let s = Session::<Created>::create(name).expect("c");
            let a = s.activate().expect("a");
            assert_eq!(a.fail().expect("→Failed").state(), SessionState::Failed);
        }

        #[test]
        fn valid_transition_matrix_syncing() {
            let name = SessionName::parse("matrix").expect("valid");
            // Syncing → Synced ✓
            let s = Session::<Created>::create(name.clone()).expect("c");
            let sy = s.activate().expect("a").sync().expect("sy");
            assert_eq!(sy.sync_complete().expect("→Synced").state(), SessionState::Synced);
            // Syncing → Failed ✓
            let s = Session::<Created>::create(name).expect("c");
            let sy = s.activate().expect("a").sync().expect("sy");
            assert_eq!(sy.fail().expect("→Failed").state(), SessionState::Failed);
        }

        #[test]
        fn valid_transition_matrix_synced() {
            let name = SessionName::parse("matrix").expect("valid");
            // Synced → Active ✓
            let s = Session::<Created>::create(name.clone()).expect("c");
            let sd = s.activate().expect("a").sync().expect("sy").sync_complete().expect("sd");
            assert_eq!(sd.reactivate().expect("→Active").state(), SessionState::Active);
            // Synced → Completed ✓
            let s = Session::<Created>::create(name.clone()).expect("c");
            let sd = s.activate().expect("a").sync().expect("sy").sync_complete().expect("sd");
            assert_eq!(sd.complete().expect("→Completed").state(), SessionState::Completed);
            // Synced → Paused ✓
            let s = Session::<Created>::create(name).expect("c");
            let sd = s.activate().expect("a").sync().expect("sy").sync_complete().expect("sd");
            assert_eq!(sd.pause().expect("→Paused").state(), SessionState::Paused);
        }

        #[test]
        fn valid_transition_matrix_paused() {
            let name = SessionName::parse("matrix").expect("valid");
            // Paused → Active ✓
            let s = Session::<Created>::create(name.clone()).expect("c");
            let p = s.activate().expect("a").pause().expect("p");
            assert_eq!(p.resume().expect("→Active").state(), SessionState::Active);
            // Paused → Failed ✓
            let s = Session::<Created>::create(name).expect("c");
            let p = s.activate().expect("a").pause().expect("p");
            assert_eq!(p.fail().expect("→Failed").state(), SessionState::Failed);
        }

        #[test]
        fn valid_transition_matrix_terminal_states() {
            let name = SessionName::parse("matrix").expect("valid");
            // Completed → Created ✓ (restart)
            let s = Session::<Created>::create(name.clone()).expect("c");
            let co = s.activate().expect("a").complete().expect("co");
            assert_eq!(co.restart().expect("→Created").state(), SessionState::Created);
            // Failed → Created ✓ (retry)
            let s = Session::<Created>::create(name).expect("c");
            let f = s.fail().expect("f");
            assert_eq!(f.retry().expect("→Created").state(), SessionState::Created);
        }

        // -- Branch transition rejection (runtime validation) --

        #[test]
        fn branch_transition_detached_to_detached_rejected_state_unchanged() {
            let name = SessionName::parse("branch-reject").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            // Detached → Detached is invalid
            let result = session.transition_branch(BranchState::Detached);
            assert!(result.is_err());
            match result {
                Err(SessionError::InvalidBranchTransition { from, to }) => {
                    assert!(from.contains("Detached"));
                    assert!(to.contains("Detached"));
                }
                Err(e) => panic!("Expected InvalidBranchTransition, got {:?}", e),
                Ok(_) => panic!("Expected error, got Ok"),
            }

            // Original session state is still accessible (transition_branch takes &self)
            let session = Session::<Created>::create(
                SessionName::parse("branch-reject-2").expect("valid"),
            )
            .expect("created");
            let on_branch = session
                .transition_branch(BranchState::OnBranch {
                    name: "dev".into(),
                })
                .expect("valid");
            assert_eq!(on_branch.branch.branch_name(), Some("dev"));

            // State type is still Created — no state corruption
            assert_eq!(on_branch.state(), SessionState::Created);
        }

        #[test]
        fn branch_transition_preserves_identity_on_success() {
            let name = SessionName::parse("branch-id").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let id_before = session.id.as_str().to_string();

            let branched = session
                .transition_branch(BranchState::OnBranch {
                    name: "main".into(),
                })
                .expect("valid");

            assert_eq!(branched.id.as_str(), id_before);
            assert_eq!(branched.state(), SessionState::Created);
        }

        // -- Identity preserved across multi-step valid transitions --

        #[test]
        fn identity_preserved_created_active_completed_restart_cycle() {
            let name = SessionName::parse("cycle-1").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let id = session.id.as_str().to_string();

            let active = session.activate().expect("activate");
            let completed = active.complete().expect("complete");
            let restarted = completed.restart().expect("restart");

            assert_eq!(restarted.id.as_str(), id);
            assert_eq!(restarted.state(), SessionState::Created);
        }

        #[test]
        fn identity_preserved_created_active_syncing_synced_reactivate() {
            let name = SessionName::parse("cycle-2").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let id = session.id.as_str().to_string();

            let syncing = session.activate().expect("a").sync().expect("sy");
            let synced = syncing.sync_complete().expect("sd");
            let reactivated = synced.reactivate().expect("reactivate");

            assert_eq!(reactivated.id.as_str(), id);
            assert_eq!(reactivated.state(), SessionState::Active);
        }

        #[test]
        fn identity_preserved_failed_retry_cycle() {
            let name = SessionName::parse("cycle-3").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let id = session.id.as_str().to_string();

            let failed = session.fail().expect("fail");
            let retried = failed.retry().expect("retry");

            assert_eq!(retried.id.as_str(), id);
            assert_eq!(retried.state(), SessionState::Created);
        }
    }
    }
    }
}
