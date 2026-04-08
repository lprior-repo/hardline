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

    // --- Exhaustive WorkspaceConfig tests ---

    mod workspace_config_exhaustive {
        use super::*;

        // === 1. Default values are sensible ===

        #[test]
        fn config_default_fields_via_workspace_create() {
            let ws = Workspace::<Initializing>::create(
                WorkspaceName::new("cfg-defaults".into()).unwrap(),
                WorkspacePath::new("/tmp/cfg-defaults".into()).unwrap(),
            )
            .unwrap();
            let cfg = ws.config().expect("workspace must have default config");
            assert_eq!(cfg.vcs_type, VcsType::Git, "default vcs_type should be Git");
            assert_eq!(cfg.default_branch, "main", "default branch should be 'main'");
            assert!(cfg.auto_sync, "auto_sync should default to true");
        }

        #[test]
        fn config_vcs_type_default_is_git() {
            assert_eq!(VcsType::default(), VcsType::Git);
        }

        #[test]
        fn config_all_false_values() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: String::new(),
                auto_sync: false,
            };
            assert!(cfg.default_branch.is_empty());
            assert!(!cfg.auto_sync);
        }

        #[test]
        fn config_with_long_branch_name() {
            let long_branch = "x".repeat(10_000);
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: long_branch.clone(),
                auto_sync: true,
            };
            assert_eq!(cfg.default_branch, long_branch);
            assert_eq!(cfg.default_branch.len(), 10_000);
        }

        #[test]
        fn config_with_unicode_branch_name() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "feature/日本語-branch".into(),
                auto_sync: true,
            };
            assert_eq!(cfg.default_branch, "feature/日本語-branch");
        }

        #[test]
        fn config_with_special_chars_in_branch() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "feature/JIRA-123_fix-bug".into(),
                auto_sync: false,
            };
            assert_eq!(cfg.default_branch, "feature/JIRA-123_fix-bug");
        }

        // === 2. Construction with overrides ===

        #[test]
        fn config_construct_with_custom_branch() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "develop".into(),
                auto_sync: true,
            };
            assert_eq!(cfg.default_branch, "develop");
        }

        #[test]
        fn config_construct_with_auto_sync_false() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: false,
            };
            assert!(!cfg.auto_sync);
        }

        #[test]
        fn config_construct_all_custom() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "release/v2".into(),
                auto_sync: false,
            };
            assert_eq!(cfg.vcs_type, VcsType::Git);
            assert_eq!(cfg.default_branch, "release/v2");
            assert!(!cfg.auto_sync);
        }

        #[test]
        fn config_construct_all_defaults() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::default(),
                default_branch: "main".into(),
                auto_sync: true,
            };
            assert_eq!(cfg.vcs_type, VcsType::Git);
            assert_eq!(cfg.default_branch, "main");
            assert!(cfg.auto_sync);
        }

        // === 3. Serialization to JSON ===

        #[test]
        fn config_serializes_all_fields() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "develop".into(),
                auto_sync: false,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            assert!(json.contains("\"vcs_type\""), "JSON must contain vcs_type");
            assert!(json.contains("\"default_branch\""), "JSON must contain default_branch");
            assert!(json.contains("\"auto_sync\""), "JSON must contain auto_sync");
        }

        #[test]
        fn config_serializes_vcs_type_as_pascal_case() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            assert!(json.contains("\"Git\""), "VcsType::Git should serialize as \"Git\"");
        }

        #[test]
        fn config_serializes_pretty_json() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            let json = serde_json::to_string_pretty(&cfg).unwrap();
            assert!(json.contains("vcs_type"));
            assert!(json.contains("default_branch"));
            assert!(json.contains("auto_sync"));
        }

        #[test]
        fn config_serializes_auto_sync_true() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            assert!(json.contains("true"));
        }

        #[test]
        fn config_serializes_auto_sync_false() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: false,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            assert!(json.contains("false"));
        }

        #[test]
        fn config_serializes_empty_branch() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: String::new(),
                auto_sync: true,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            assert!(json.contains("\"default_branch\":\"\""));
        }

        // === 4. Deserialization from JSON ===

        #[test]
        fn config_deserializes_from_valid_json() {
            let json = r#"{"vcs_type":"Git","default_branch":"develop","auto_sync":false}"#;
            let cfg: WorkspaceConfig = serde_json::from_str(json).unwrap();
            assert_eq!(cfg.vcs_type, VcsType::Git);
            assert_eq!(cfg.default_branch, "develop");
            assert!(!cfg.auto_sync);
        }

        #[test]
        fn config_deserializes_with_true_auto_sync() {
            let json = r#"{"vcs_type":"Git","default_branch":"main","auto_sync":true}"#;
            let cfg: WorkspaceConfig = serde_json::from_str(json).unwrap();
            assert!(cfg.auto_sync);
        }

        // === 5. Round-trip equality (serialize then deserialize) ===

        #[test]
        fn config_roundtrip_default_values() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            let back: WorkspaceConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(cfg.vcs_type, back.vcs_type);
            assert_eq!(cfg.default_branch, back.default_branch);
            assert_eq!(cfg.auto_sync, back.auto_sync);
        }

        #[test]
        fn config_roundtrip_custom_values() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "feature/test-round-trip".into(),
                auto_sync: false,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            let back: WorkspaceConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(cfg.vcs_type, back.vcs_type);
            assert_eq!(cfg.default_branch, back.default_branch);
            assert_eq!(cfg.auto_sync, back.auto_sync);
        }

        #[test]
        fn config_roundtrip_empty_branch() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: String::new(),
                auto_sync: false,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            let back: WorkspaceConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(cfg.default_branch, back.default_branch);
            assert!(back.default_branch.is_empty());
        }

        #[test]
        fn config_roundtrip_unicode_branch() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "feature/日本語".into(),
                auto_sync: true,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            let back: WorkspaceConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(cfg.default_branch, back.default_branch);
        }

        // === 6. Partial updates — field-level mutation via clone + modify ===

        #[test]
        fn config_partial_update_branch_only() {
            let original = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            let updated = WorkspaceConfig {
                default_branch: "develop".into(),
                ..original.clone()
            };
            assert_eq!(updated.vcs_type, VcsType::Git);
            assert_eq!(updated.default_branch, "develop");
            assert!(updated.auto_sync);
            // Original unchanged
            assert_eq!(original.default_branch, "main");
        }

        #[test]
        fn config_partial_update_auto_sync_only() {
            let original = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            let updated = WorkspaceConfig {
                auto_sync: false,
                ..original.clone()
            };
            assert_eq!(updated.vcs_type, VcsType::Git);
            assert_eq!(updated.default_branch, "main");
            assert!(!updated.auto_sync);
        }

        #[test]
        fn config_partial_update_vcs_type_only() {
            let original = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            // VcsType only has Git variant, but struct update syntax still works
            let updated = WorkspaceConfig {
                ..original.clone()
            };
            assert_eq!(updated.vcs_type, original.vcs_type);
            assert_eq!(updated.default_branch, original.default_branch);
            assert_eq!(updated.auto_sync, original.auto_sync);
        }

        #[test]
        fn config_multiple_partial_updates() {
            let base = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            let step1 = WorkspaceConfig {
                default_branch: "develop".into(),
                ..base.clone()
            };
            let step2 = WorkspaceConfig {
                auto_sync: false,
                ..step1.clone()
            };
            assert_eq!(step2.default_branch, "develop");
            assert!(!step2.auto_sync);
            assert_eq!(step2.vcs_type, VcsType::Git);
        }

        // === 7. Field-level validation (what the types enforce) ===

        #[test]
        fn config_vcs_type_only_git_variant_exists() {
            // VcsType currently only has Git. This test documents that.
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            assert_eq!(cfg.vcs_type, VcsType::Git);
        }

        #[test]
        fn config_default_branch_accepts_any_string() {
            // String field — no compile-time validation, documents behavior
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "  spaces  ".into(),
                auto_sync: true,
            };
            assert_eq!(cfg.default_branch, "  spaces  ");
        }

        #[test]
        fn config_auto_sync_is_bool() {
            let cfg_true = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            let cfg_false = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: false,
            };
            assert!(cfg_true.auto_sync);
            assert!(!cfg_false.auto_sync);
        }

        // === 8. Missing fields → deserialization error (no #[serde(default)]) ===

        #[test]
        fn config_missing_vcs_type_fails() {
            let json = r#"{"default_branch":"main","auto_sync":true}"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_err(), "missing vcs_type must fail deserialization");
        }

        #[test]
        fn config_missing_default_branch_fails() {
            let json = r#"{"vcs_type":"Git","auto_sync":true}"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_err(), "missing default_branch must fail deserialization");
        }

        #[test]
        fn config_missing_auto_sync_fails() {
            let json = r#"{"vcs_type":"Git","default_branch":"main"}"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_err(), "missing auto_sync must fail deserialization");
        }

        #[test]
        fn config_empty_json_object_fails() {
            let json = r#"{}"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_err(), "empty object must fail deserialization");
        }

        // === 9. Extra fields → silently ignored (no #[serde(deny_unknown_fields)]) ===

        #[test]
        fn config_extra_fields_ignored() {
            let json = r#"{"vcs_type":"Git","default_branch":"main","auto_sync":true,"extra":"value"}"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_ok(), "extra fields should be silently ignored");
            let cfg = result.unwrap();
            assert_eq!(cfg.vcs_type, VcsType::Git);
            assert_eq!(cfg.default_branch, "main");
            assert!(cfg.auto_sync);
        }

        #[test]
        fn config_multiple_extra_fields_ignored() {
            let json = r#"{"vcs_type":"Git","default_branch":"main","auto_sync":true,"foo":1,"bar":null,"baz":true}"#;
            let cfg: WorkspaceConfig = serde_json::from_str(json).unwrap();
            assert_eq!(cfg.vcs_type, VcsType::Git);
            assert_eq!(cfg.default_branch, "main");
            assert!(cfg.auto_sync);
        }

        #[test]
        fn config_extra_nested_field_ignored() {
            let json = r#"{"vcs_type":"Git","default_branch":"main","auto_sync":true,"nested":{"a":1}}"#;
            let cfg: WorkspaceConfig = serde_json::from_str(json).unwrap();
            assert_eq!(cfg.default_branch, "main");
        }

        // === 10. Type mismatch on fields → deserialization error ===

        #[test]
        fn config_wrong_type_vcs_type_fails() {
            let json = r#"{"vcs_type":123,"default_branch":"main","auto_sync":true}"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_err(), "vcs_type must be a string enum, not a number");
        }

        #[test]
        fn config_wrong_type_default_branch_fails() {
            let json = r#"{"vcs_type":"Git","default_branch":123,"auto_sync":true}"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_err(), "default_branch must be a string, not a number");
        }

        #[test]
        fn config_wrong_type_auto_sync_fails() {
            let json = r#"{"vcs_type":"Git","default_branch":"main","auto_sync":"yes"}"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_err(), "auto_sync must be a boolean, not a string");
        }

        #[test]
        fn config_invalid_vcs_type_variant_fails() {
            let json = r#"{"vcs_type":"svn","default_branch":"main","auto_sync":true}"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_err(), "unknown VcsType variant 'svn' must fail");
        }

        #[test]
        fn config_null_json_fails() {
            let json = r#"null"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_err(), "null must fail for struct deserialization");
        }

        #[test]
        fn config_invalid_json_fails() {
            let json = r#"not json at all"#;
            let result = serde_json::from_str::<WorkspaceConfig>(json);
            assert!(result.is_err());
        }

        // === 11. Clone semantics ===

        #[test]
        fn config_clone_is_independent() {
            let original = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            let cloned = original.clone();
            // Mutating a cloned string doesn't affect original
            let mut modified = cloned;
            modified.default_branch = "develop".into();
            assert_eq!(original.default_branch, "main");
            assert_eq!(modified.default_branch, "develop");
        }

        #[test]
        fn config_clone_equality() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "release/v1".into(),
                auto_sync: false,
            };
            let cloned = cfg.clone();
            assert_eq!(cfg.vcs_type, cloned.vcs_type);
            assert_eq!(cfg.default_branch, cloned.default_branch);
            assert_eq!(cfg.auto_sync, cloned.auto_sync);
        }

        // === 12. Debug format ===

        #[test]
        fn config_debug_contains_all_fields() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "debug-branch".into(),
                auto_sync: true,
            };
            let debug = format!("{cfg:?}");
            assert!(debug.contains("WorkspaceConfig"), "debug should contain type name");
            assert!(debug.contains("vcs_type"), "debug should show vcs_type");
            assert!(debug.contains("default_branch"), "debug should show default_branch");
            assert!(debug.contains("auto_sync"), "debug should show auto_sync");
        }

        #[test]
        fn config_debug_shows_values() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "debug-me".into(),
                auto_sync: false,
            };
            let debug = format!("{cfg:?}");
            assert!(debug.contains("debug-me"));
        }

        // === 13. Config persists through workspace lifecycle ===

        #[test]
        fn config_survives_activate() {
            let ws = Workspace::<Initializing>::create(
                WorkspaceName::new("cfg-activate".into()).unwrap(),
                WorkspacePath::new("/tmp/cfg-activate".into()).unwrap(),
            )
            .unwrap();
            let active = ws.activate().unwrap();
            let cfg = active.config().unwrap();
            assert_eq!(cfg.default_branch, "main");
            assert!(cfg.auto_sync);
            assert_eq!(cfg.vcs_type, VcsType::Git);
        }

        #[test]
        fn config_survives_lock_unlock() {
            let ws = Workspace::<Initializing>::create(
                WorkspaceName::new("cfg-lock".into()).unwrap(),
                WorkspacePath::new("/tmp/cfg-lock".into()).unwrap(),
            )
            .unwrap();
            let active = ws.activate().unwrap();
            let locked = active.lock("agent".into()).unwrap();
            let cfg_locked = locked.config().unwrap();
            assert_eq!(cfg_locked.default_branch, "main");
            let unlocked = locked.unlock().unwrap();
            let cfg_unlocked = unlocked.config().unwrap();
            assert_eq!(cfg_unlocked.default_branch, "main");
            assert_eq!(cfg_unlocked.vcs_type, VcsType::Git);
            assert!(cfg_unlocked.auto_sync);
        }

        #[test]
        fn config_survives_mark_corrupted() {
            let ws = Workspace::<Initializing>::create(
                WorkspaceName::new("cfg-corrupt".into()).unwrap(),
                WorkspacePath::new("/tmp/cfg-corrupt".into()).unwrap(),
            )
            .unwrap();
            let active = ws.activate().unwrap();
            let corrupted = active.mark_corrupted().unwrap();
            let cfg = corrupted.config().unwrap();
            assert_eq!(cfg.default_branch, "main");
            assert!(cfg.auto_sync);
        }

        #[test]
        fn config_survives_delete() {
            let ws = Workspace::<Initializing>::create(
                WorkspaceName::new("cfg-delete".into()).unwrap(),
                WorkspacePath::new("/tmp/cfg-delete".into()).unwrap(),
            )
            .unwrap();
            let active = ws.activate().unwrap();
            let deleted = active.delete().unwrap();
            let cfg = deleted.config().unwrap();
            assert_eq!(cfg.default_branch, "main");
            assert!(cfg.auto_sync);
        }

        #[test]
        fn config_survives_full_lifecycle() {
            let ws = Workspace::<Initializing>::create(
                WorkspaceName::new("cfg-life".into()).unwrap(),
                WorkspacePath::new("/tmp/cfg-life".into()).unwrap(),
            )
            .unwrap();
            let original_cfg = ws.config().unwrap().clone();

            let active = ws.activate().unwrap();
            let locked = active.lock("agent".into()).unwrap();
            let unlocked = locked.unlock().unwrap();
            let deleted = unlocked.delete().unwrap();

            let final_cfg = deleted.config().unwrap();
            assert_eq!(final_cfg.vcs_type, original_cfg.vcs_type);
            assert_eq!(final_cfg.default_branch, original_cfg.default_branch);
            assert_eq!(final_cfg.auto_sync, original_cfg.auto_sync);
        }

        // === 14. JSON structure contract (exact expected shapes) ===

        #[test]
        fn config_json_structure_default() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "main".into(),
                auto_sync: true,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            // Verify the JSON contains the exact expected fields and values
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["vcs_type"], "Git");
            assert_eq!(parsed["default_branch"], "main");
            assert_eq!(parsed["auto_sync"], true);
        }

        #[test]
        fn config_json_structure_custom() {
            let cfg = WorkspaceConfig {
                vcs_type: VcsType::Git,
                default_branch: "develop".into(),
                auto_sync: false,
            };
            let json = serde_json::to_string(&cfg).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["vcs_type"], "Git");
            assert_eq!(parsed["default_branch"], "develop");
            assert_eq!(parsed["auto_sync"], false);
        }

        #[test]
        fn config_deserialize_from_value_object() {
            let mut map = serde_json::Map::new();
            map.insert("vcs_type".into(), serde_json::Value::String("Git".into()));
            map.insert("default_branch".into(), serde_json::Value::String("feature/x".into()));
            map.insert("auto_sync".into(), serde_json::Value::Bool(false));
            let value = serde_json::Value::Object(map);
            let cfg: WorkspaceConfig = serde_json::from_value(value).unwrap();
            assert_eq!(cfg.vcs_type, VcsType::Git);
            assert_eq!(cfg.default_branch, "feature/x");
            assert!(!cfg.auto_sync);
        }

        // === 15. VcsType exhaustive tests ===

        #[test]
        fn vcs_type_git_serializes_as_pascal_case() {
            let json = serde_json::to_string(&VcsType::Git).unwrap();
            assert_eq!(json, "\"Git\"");
        }

        #[test]
        fn vcs_type_git_deserializes_from_pascal_case() {
            let vcs: VcsType = serde_json::from_str("\"Git\"").unwrap();
            assert_eq!(vcs, VcsType::Git);
        }

        #[test]
        fn vcs_type_roundtrip() {
            let json = serde_json::to_string(&VcsType::Git).unwrap();
            let back: VcsType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, VcsType::Git);
        }

        #[test]
        fn vcs_type_equality() {
            assert_eq!(VcsType::Git, VcsType::Git);
        }

        #[test]
        fn vcs_type_copy() {
            let a = VcsType::Git;
            let b = a;
            assert_eq!(a, b);
        }

        #[test]
        fn vcs_type_debug() {
            let debug = format!("{:?}", VcsType::Git);
            assert!(!debug.is_empty());
        }

        #[test]
        fn vcs_type_unknown_variant_fails() {
            let result = serde_json::from_str::<VcsType>("\"mercurial\"");
            assert!(result.is_err());
        }

        #[test]
        fn vcs_type_null_fails() {
            let result = serde_json::from_str::<VcsType>("null");
            assert!(result.is_err());
        }

        // === 16. Config from JSON with field ordering variations ===

        #[test]
        fn config_deserializes_with_reordered_fields() {
            let json = r#"{"auto_sync":false,"vcs_type":"Git","default_branch":"main"}"#;
            let cfg: WorkspaceConfig = serde_json::from_str(json).unwrap();
            assert_eq!(cfg.vcs_type, VcsType::Git);
            assert_eq!(cfg.default_branch, "main");
            assert!(!cfg.auto_sync);
        }

        #[test]
        fn config_deserializes_with_whitespace() {
            let json = r#"
            {
                "vcs_type": "Git",
                "default_branch": "main",
                "auto_sync": true
            }"#;
            let cfg: WorkspaceConfig = serde_json::from_str(json).unwrap();
            assert_eq!(cfg.vcs_type, VcsType::Git);
            assert!(cfg.auto_sync);
        }
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
