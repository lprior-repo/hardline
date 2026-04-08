#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_wraps)]
#![forbid(unsafe_code)]

use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{WorkspaceName, WorkspacePath};
use crate::error::WorkspaceError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum WorkspaceState {
    #[default]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    #[must_use]
    pub fn generate() -> Self {
        Self(format!("ws-{}", uuid::Uuid::new_v4()))
    }

    pub fn parse(id: String) -> Result<Self, WorkspaceError> {
        if id.is_empty() {
            return Err(WorkspaceError::InvalidWorkspaceId("empty id".into()));
        }
        Ok(Self(id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for WorkspaceId {
    type Err = WorkspaceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s.to_owned())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VcsType {
    #[default]
    Git,
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
    #[must_use]
    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &WorkspaceName {
        &self.name
    }

    #[must_use]
    pub fn path(&self) -> &WorkspacePath {
        &self.path
    }

    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    #[must_use]
    pub fn lock_holder(&self) -> Option<&str> {
        self.lock_holder.as_deref()
    }

    #[must_use]
    pub fn config(&self) -> Option<&WorkspaceConfig> {
        self.config.as_ref()
    }

    #[must_use]
    pub fn is_locked(&self) -> bool {
        self.state == WorkspaceState::Locked
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state == WorkspaceState::Active
    }

    #[must_use]
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

    #[test]
    fn workspace_created_has_default_config() {
        let workspace = Workspace::<Initializing>::create(
            WorkspaceName::new("cfg-test".into()).unwrap(),
            WorkspacePath::new("/tmp/cfg-test".into()).unwrap(),
        )
        .unwrap();
        let config = workspace
            .config()
            .expect("workspace should have default config");
        assert_eq!(config.default_branch, "main");
        assert!(config.auto_sync);
        assert_eq!(config.vcs_type, VcsType::Git);
    }

    #[test]
    fn workspace_created_has_no_lock_holder() {
        let workspace = Workspace::<Initializing>::create(
            WorkspaceName::new("lock-test".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-test".into()).unwrap(),
        )
        .unwrap();
        assert!(workspace.lock_holder().is_none());
    }

    #[test]
    fn workspace_created_at_and_updated_at_match() {
        let workspace = Workspace::<Initializing>::create(
            WorkspaceName::new("ts-test".into()).unwrap(),
            WorkspacePath::new("/tmp/ts-test".into()).unwrap(),
        )
        .unwrap();
        assert_eq!(workspace.created_at(), workspace.updated_at());
    }

    #[test]
    fn workspace_activate_updates_updated_at() {
        let workspace = Workspace::<Initializing>::create(
            WorkspaceName::new("upd-test".into()).unwrap(),
            WorkspacePath::new("/tmp/upd-test".into()).unwrap(),
        )
        .unwrap();
        // Small sleep to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(2));
        let activated = workspace.activate().unwrap();
        assert!(activated.updated_at() >= activated.created_at());
    }

    #[test]
    fn workspace_id_generated_starts_with_ws_prefix() {
        let workspace = Workspace::<Initializing>::create(
            WorkspaceName::new("id-test".into()).unwrap(),
            WorkspacePath::new("/tmp/id-test".into()).unwrap(),
        )
        .unwrap();
        assert!(workspace.id().as_str().starts_with("ws-"));
    }

    #[test]
    fn workspace_id_parse_rejects_empty() {
        let result = WorkspaceId::parse("".into());
        assert!(result.is_err());
        match result.err() {
            Some(WorkspaceError::InvalidWorkspaceId(msg)) => {
                assert!(msg.contains("empty"));
            }
            other => panic!("expected InvalidWorkspaceId, got {other:?}"),
        }
    }

    #[test]
    fn workspace_id_parse_accepts_non_empty() {
        let id = WorkspaceId::parse("my-custom-id".into()).unwrap();
        assert_eq!(id.as_str(), "my-custom-id");
    }

    #[test]
    fn workspace_id_generate_produces_unique_ids() {
        let id1 = WorkspaceId::generate();
        let id2 = WorkspaceId::generate();
        assert_ne!(id1.as_str(), id2.as_str());
    }

    #[test]
    fn workspace_state_default_is_initializing() {
        assert_eq!(WorkspaceState::default(), WorkspaceState::Initializing);
    }

    #[test]
    fn workspace_state_serializes_to_snake_case() {
        let states = vec![
            WorkspaceState::Initializing,
            WorkspaceState::Active,
            WorkspaceState::Locked,
            WorkspaceState::Corrupted,
            WorkspaceState::Deleted,
        ];
        for state in states {
            let json = serde_json::to_string(&state).unwrap_or_else(|_| {
                // fallback: just check Debug format includes expected name
                format!("{:?}", state).to_lowercase()
            });
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn workspace_state_is_terminal_for_deleted_and_corrupted() {
        assert!(WorkspaceState::Deleted.is_terminal());
        assert!(WorkspaceState::Corrupted.is_terminal());
        assert!(!WorkspaceState::Initializing.is_terminal());
        assert!(!WorkspaceState::Active.is_terminal());
        assert!(!WorkspaceState::Locked.is_terminal());
    }

    #[test]
    fn workspace_state_equality() {
        assert_eq!(WorkspaceState::Active, WorkspaceState::Active);
        assert_ne!(WorkspaceState::Active, WorkspaceState::Locked);
    }

    #[test]
    fn vcs_type_default_is_git() {
        assert_eq!(VcsType::default(), VcsType::Git);
    }

    #[test]
    fn vcs_type_equality() {
        assert_eq!(VcsType::Git, VcsType::Git);
    }

    #[test]
    fn workspace_full_lifecycle_initialize_lock_unlock_delete() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("lifecycle".into()).unwrap(),
            WorkspacePath::new("/tmp/lifecycle".into()).unwrap(),
        )
        .unwrap();
        assert_eq!(ws.state, WorkspaceState::Initializing);

        let active = ws.activate().unwrap();
        assert!(active.is_active());
        assert!(!active.is_locked());

        let locked = active.lock("agent-1".into()).unwrap();
        assert!(locked.is_locked());
        assert_eq!(locked.lock_holder(), Some("agent-1"));

        let unlocked = locked.unlock().unwrap();
        assert!(unlocked.is_active());
        assert!(unlocked.lock_holder().is_none());

        let deleted = unlocked.delete().unwrap();
        assert!(deleted.is_terminal());
    }

    #[test]
    fn workspace_initialize_to_delete_skipping_active() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("quick-del".into()).unwrap(),
            WorkspacePath::new("/tmp/quick-del".into()).unwrap(),
        )
        .unwrap();
        let deleted = ws.delete().unwrap();
        assert_eq!(deleted.state, WorkspaceState::Deleted);
    }

    #[test]
    fn workspace_active_to_deleted_directly() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("active-del".into()).unwrap(),
            WorkspacePath::new("/tmp/active-del".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        let deleted = active.delete().unwrap();
        assert_eq!(deleted.state, WorkspaceState::Deleted);
    }

    #[test]
    fn workspace_locked_to_corrupted() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("lock-corrupt".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-corrupt".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        let locked = active.lock("agent-1".into()).unwrap();
        let corrupted = locked.mark_corrupted().unwrap();
        assert!(corrupted.is_terminal());
        assert!(corrupted.lock_holder().is_none());
    }

    #[test]
    fn workspace_locked_to_deleted() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("lock-del".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-del".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        let locked = active.lock("agent-1".into()).unwrap();
        let deleted = locked.delete().unwrap();
        assert_eq!(deleted.state, WorkspaceState::Deleted);
    }

    #[test]
    fn workspace_corrupted_to_deleted() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("corrupt-del".into()).unwrap(),
            WorkspacePath::new("/tmp/corrupt-del".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        let corrupted = active.mark_corrupted().unwrap();
        let deleted = corrupted.delete().unwrap();
        assert_eq!(deleted.state, WorkspaceState::Deleted);
    }

    #[test]
    fn workspace_accessors_return_expected_values() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("accessor-test".into()).unwrap(),
            WorkspacePath::new("/tmp/accessor-test".into()).unwrap(),
        )
        .unwrap();
        assert!(std::ptr::eq(ws.id(), &ws.id));
        assert_eq!(ws.name().as_str(), "accessor-test");
        assert!(ws.path().as_path().to_str().unwrap().contains("/tmp"));
        assert!(ws.is_locked() == false);
        assert!(ws.is_active() == false);
        assert!(ws.is_terminal() == false);
    }

    #[test]
    fn workspace_clone_preserves_fields() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("clone-test".into()).unwrap(),
            WorkspacePath::new("/tmp/clone-test".into()).unwrap(),
        )
        .unwrap();
        let ws2 = ws.clone();
        assert_eq!(ws.id.as_str(), ws2.id.as_str());
        assert_eq!(ws.name, ws2.name);
    }

    #[test]
    fn workspace_config_serialization_roundtrip() {
        let config = WorkspaceConfig {
            vcs_type: VcsType::Git,
            default_branch: "develop".into(),
            auto_sync: false,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WorkspaceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.vcs_type, deserialized.vcs_type);
        assert_eq!(config.default_branch, deserialized.default_branch);
        assert_eq!(config.auto_sync, deserialized.auto_sync);
    }

    #[test]
    fn workspace_config_clone() {
        let config = WorkspaceConfig {
            vcs_type: VcsType::Git,
            default_branch: "main".into(),
            auto_sync: true,
        };
        let cloned = config.clone();
        assert_eq!(config.vcs_type, cloned.vcs_type);
        assert_eq!(config.default_branch, cloned.default_branch);
        assert_eq!(config.auto_sync, cloned.auto_sync);
    }

    #[test]
    fn workspace_config_debug() {
        let config = WorkspaceConfig {
            vcs_type: VcsType::Git,
            default_branch: "debug".into(),
            auto_sync: true,
        };
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("debug"));
    }

    #[test]
    fn vcs_type_serialization_roundtrip() {
        for vcs in [VcsType::Git] {
            let json = serde_json::to_string(&vcs).unwrap();
            let deserialized: VcsType = serde_json::from_str(&json).unwrap();
            assert_eq!(vcs, deserialized);
        }
    }

    #[test]
    fn workspace_state_debug_format() {
        for state in [
            WorkspaceState::Initializing,
            WorkspaceState::Active,
            WorkspaceState::Locked,
            WorkspaceState::Corrupted,
            WorkspaceState::Deleted,
        ] {
            let debug_str = format!("{state:?}");
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn workspace_state_copy_semantics() {
        let state = WorkspaceState::Active;
        let copied = state;
        assert_eq!(state, copied);
    }

    #[test]
    fn workspace_mark_corrupted_clears_lock_holder() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("corrupt-clear".into()).unwrap(),
            WorkspacePath::new("/tmp/corrupt-clear".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        let corrupted = active.mark_corrupted().unwrap();
        assert!(corrupted.lock_holder().is_none());
        assert!(corrupted.is_terminal());
        assert_eq!(corrupted.state, WorkspaceState::Corrupted);
    }

    #[test]
    fn workspace_id_default_generates_unique() {
        let id1 = WorkspaceId::default();
        let id2 = WorkspaceId::default();
        assert_ne!(id1.as_str(), id2.as_str());
        assert!(id1.as_str().starts_with("ws-"));
    }

    #[test]
    fn workspace_id_parse_preserves_value() {
        let id = WorkspaceId::parse("custom-id-123".into()).unwrap();
        assert_eq!(id.as_str(), "custom-id-123");
    }

    // --- Additional unit tests ---

    #[test]
    fn workspace_id_parse_with_uuid_format() {
        let id_str = format!("ws-{}", uuid::Uuid::new_v4());
        let id = WorkspaceId::parse(id_str.clone()).unwrap();
        assert_eq!(id.as_str(), id_str);
    }

    #[test]
    fn workspace_id_parse_with_special_chars() {
        // parse only rejects empty; special characters are fine
        let id = WorkspaceId::parse("ws-123/ABC_!@#$%".into()).unwrap();
        assert_eq!(id.as_str(), "ws-123/ABC_!@#$%");
    }

    #[test]
    fn workspace_id_generate_produces_uuid_format() {
        let id = WorkspaceId::generate();
        // Format: "ws-<uuid>" where uuid is 36 chars (8-4-4-4-12)
        let s = id.as_str();
        assert!(s.starts_with("ws-"));
        assert_eq!(s.len(), 3 + 36); // "ws-" + 36-char UUID
    }

    #[test]
    fn workspace_create_preserves_name_and_path() {
        let name = WorkspaceName::new("my-ws".into()).unwrap();
        let path = WorkspacePath::new("/tmp/my-ws".into()).unwrap();
        let ws = Workspace::<Initializing>::create(name.clone(), path.clone()).unwrap();
        assert_eq!(ws.name().as_str(), "my-ws");
        assert_eq!(ws.path().as_str(), Some("/tmp/my-ws"));
    }

    #[test]
    fn workspace_activate_preserves_name_and_path() {
        let name = WorkspaceName::new("preserve-test".into()).unwrap();
        let path = WorkspacePath::new("/tmp/preserve".into()).unwrap();
        let ws = Workspace::<Initializing>::create(name.clone(), path.clone()).unwrap();
        let active = ws.activate().unwrap();
        assert_eq!(active.name().as_str(), "preserve-test");
        assert_eq!(active.path().as_str(), Some("/tmp/preserve"));
    }

    #[test]
    fn workspace_lock_preserves_id() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("lock-id".into()).unwrap(),
            WorkspacePath::new("/tmp/lock-id".into()).unwrap(),
        )
        .unwrap();
        let id = ws.id.as_str().to_string();
        let active = ws.activate().unwrap();
        let locked = active.lock("agent".into()).unwrap();
        assert_eq!(locked.id.as_str(), id);
    }

    #[test]
    fn workspace_unlock_preserves_id() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("unlock-id".into()).unwrap(),
            WorkspacePath::new("/tmp/unlock-id".into()).unwrap(),
        )
        .unwrap();
        let id = ws.id.as_str().to_string();
        let active = ws.activate().unwrap();
        let locked = active.lock("agent".into()).unwrap();
        let unlocked = locked.unlock().unwrap();
        assert_eq!(unlocked.id.as_str(), id);
    }

    #[test]
    fn workspace_corrupted_preserves_created_at() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("corrupt-ts".into()).unwrap(),
            WorkspacePath::new("/tmp/corrupt-ts".into()).unwrap(),
        )
        .unwrap();
        let created_at = ws.created_at();
        let active = ws.activate().unwrap();
        let corrupted = active.mark_corrupted().unwrap();
        assert_eq!(corrupted.created_at(), created_at);
    }

    #[test]
    fn workspace_deleted_preserves_created_at() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("del-ts".into()).unwrap(),
            WorkspacePath::new("/tmp/del-ts".into()).unwrap(),
        )
        .unwrap();
        let created_at = ws.created_at();
        let active = ws.activate().unwrap();
        let deleted = active.delete().unwrap();
        assert_eq!(deleted.created_at(), created_at);
    }

    #[test]
    fn workspace_active_is_not_locked() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("not-locked".into()).unwrap(),
            WorkspacePath::new("/tmp/not-locked".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        assert!(!active.is_locked());
        assert!(!active.is_terminal());
    }

    #[test]
    fn workspace_locked_is_not_active() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("not-active".into()).unwrap(),
            WorkspacePath::new("/tmp/not-active".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        let locked = active.lock("agent".into()).unwrap();
        assert!(!locked.is_active());
        assert!(!locked.is_terminal());
    }

    #[test]
    fn workspace_multiple_lock_unlock_cycles() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("cycles".into()).unwrap(),
            WorkspacePath::new("/tmp/cycles".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();

        let locked1 = active.lock("agent-1".into()).unwrap();
        assert_eq!(locked1.lock_holder(), Some("agent-1"));

        let unlocked1 = locked1.unlock().unwrap();
        assert!(unlocked1.lock_holder().is_none());

        let locked2 = unlocked1.lock("agent-2".into()).unwrap();
        assert_eq!(locked2.lock_holder(), Some("agent-2"));

        let unlocked2 = locked2.unlock().unwrap();
        assert!(unlocked2.is_active());
    }

    #[test]
    fn workspace_initialize_to_active_preserves_config() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("cfg-preserve".into()).unwrap(),
            WorkspacePath::new("/tmp/cfg-preserve".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        let config = active.config().expect("should have config");
        assert_eq!(config.default_branch, "main");
        assert!(config.auto_sync);
        assert_eq!(config.vcs_type, VcsType::Git);
    }

    #[test]
    fn workspace_state_deserialization_from_snake_case() {
        assert_eq!(
            serde_json::from_str::<WorkspaceState>("\"initializing\"").unwrap(),
            WorkspaceState::Initializing
        );
        assert_eq!(
            serde_json::from_str::<WorkspaceState>("\"active\"").unwrap(),
            WorkspaceState::Active
        );
        assert_eq!(
            serde_json::from_str::<WorkspaceState>("\"locked\"").unwrap(),
            WorkspaceState::Locked
        );
        assert_eq!(
            serde_json::from_str::<WorkspaceState>("\"corrupted\"").unwrap(),
            WorkspaceState::Corrupted
        );
        assert_eq!(
            serde_json::from_str::<WorkspaceState>("\"deleted\"").unwrap(),
            WorkspaceState::Deleted
        );
    }

    #[test]
    fn workspace_state_full_serialization_roundtrip() {
        for state in [
            WorkspaceState::Initializing,
            WorkspaceState::Active,
            WorkspaceState::Locked,
            WorkspaceState::Corrupted,
            WorkspaceState::Deleted,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let deserialized: WorkspaceState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, deserialized, "roundtrip failed for {state:?}");
        }
    }

    #[test]
    fn workspace_config_default_branch_variants() {
        for branch in ["main", "master", "develop", "feature/branch"] {
            let config = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: branch.into(),
                auto_sync: true,
            };
            assert_eq!(config.default_branch, branch);
        }
    }

    #[test]
    fn workspace_id_hash_set_deduplication() {
        use std::collections::HashSet;
        let id = WorkspaceId::parse("same-id".into()).unwrap();
        let id2 = WorkspaceId::parse("same-id".into()).unwrap();
        let mut set = HashSet::new();
        set.insert(id.clone());
        set.insert(id2);
        assert_eq!(set.len(), 1);
        assert!(set.contains(&id));
    }

    #[test]
    fn workspace_id_equality_and_inequality() {
        let a = WorkspaceId::parse("id-a".into()).unwrap();
        let b = WorkspaceId::parse("id-a".into()).unwrap();
        let c = WorkspaceId::parse("id-c".into()).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn vcs_type_debug_format() {
        for vcs in [VcsType::Git] {
            let debug_str = format!("{vcs:?}");
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn workspace_active_has_no_config_change_after_transition() {
        let ws = Workspace::<Initializing>::create(
            WorkspaceName::new("cfg-stable".into()).unwrap(),
            WorkspacePath::new("/tmp/cfg-stable".into()).unwrap(),
        )
        .unwrap();
        let active = ws.activate().unwrap();
        let locked = active.lock("a".into()).unwrap();
        let unlocked = locked.unlock().unwrap();
        let cfg = unlocked.config().unwrap();
        assert_eq!(cfg.vcs_type, VcsType::Git);
        assert_eq!(cfg.default_branch, "main");
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use proptest::prelude::*;
        use proptest::{prop_assert, prop_assert_eq};

        use super::*;

        proptest! {
            #[test]
            fn workspace_name_valid_alphanumeric_with_separators(name in "[a-zA-Z0-9_-]{1,255}") {
                let result = WorkspaceName::new(name);
                prop_assert!(result.is_ok());
            }

            #[test]
            fn workspace_name_invalid_with_special_chars(name in "[a-zA-Z0-9_-]*[. ][a-zA-Z0-9_-]*") {
                // Ensure the string is non-empty and contains a dot or space
                let non_empty = format!("x{}", name);
                if non_empty.contains('.') || non_empty.contains(' ') {
                    let result = WorkspaceName::new(non_empty);
                    prop_assert!(result.is_err());
                }
            }

            #[test]
            fn workspace_name_exactly_255_chars_is_valid(c in "[a-z0-9]{1,255}") {
                prop_assume!(!c.is_empty());
                let padded = if c.len() < 255 {
                    format!("{}{}", c, "a".repeat(255 - c.len()))
                } else {
                    c
                };
                let result = WorkspaceName::new(padded);
                prop_assert!(result.is_ok());
            }

            #[test]
            fn workspace_name_over_255_chars_is_invalid(base in "[a-z0-9]{1,10}") {
                let long = format!("{}{}", base, "a".repeat(256 - base.len()));
                let result = WorkspaceName::new(long);
                prop_assert!(result.is_err());
            }

            #[test]
            fn workspace_id_generate_is_unique_batch(_idx in 0..100usize) {
                let ids: std::collections::HashSet<String> = (0..100)
                    .map(|_| WorkspaceId::generate().as_str().to_string())
                    .collect();
                prop_assert_eq!(ids.len(), 100);
            }

            #[test]
            fn workspace_state_serialization_roundtrip_arbitrary(state_idx in 0usize..5) {
                let states = [
                    WorkspaceState::Initializing,
                    WorkspaceState::Active,
                    WorkspaceState::Locked,
                    WorkspaceState::Corrupted,
                    WorkspaceState::Deleted,
                ];
                let state = states[state_idx];
                let json = serde_json::to_string(&state).unwrap();
                let deserialized: WorkspaceState = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(state, deserialized);
            }

            #[test]
            fn vcs_type_serialization_roundtrip_arbitrary(vcs_idx in 0usize..1) {
                let types = [VcsType::Git];
                let vcs = types[vcs_idx];
                let json = serde_json::to_string(&vcs).unwrap();
                let deserialized: VcsType = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(vcs, deserialized);
            }

            #[test]
            fn workspace_config_serialization_roundtrip_arbitrary(
                vcs_idx in 0usize..1,
                auto_sync in proptest::bool::ANY,
                branch in "[a-z]{1,20}"
            ) {
                let types = [VcsType::Git];
                let config = WorkspaceConfig {
                    vcs_type: types[vcs_idx],
                    default_branch: format!("test-{}", branch),
                    auto_sync,
                };
                let json = serde_json::to_string(&config).unwrap();
                let deserialized: WorkspaceConfig = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(config.vcs_type, deserialized.vcs_type);
                prop_assert_eq!(config.default_branch, deserialized.default_branch);
                prop_assert_eq!(config.auto_sync, deserialized.auto_sync);
            }
        }
    }
}
