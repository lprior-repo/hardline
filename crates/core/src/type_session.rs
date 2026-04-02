//! Session aggregate root
//!
//! Combines all session-related types into the main Session struct.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{Error, Result};
use crate::workspace_state::WorkspaceState;

use super::type_branch_state::BranchState;
use super::type_metadata::ValidatedMetadata;
use super::type_session_id::SessionId;
use super::type_session_name::SessionName;
use super::type_session_path::AbsolutePath;
use super::type_session_status::SessionStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub name: SessionName,
    pub status: SessionStatus,
    pub state: WorkspaceState,
    #[serde(serialize_with = "serialize_absolute_path")]
    #[serde(deserialize_with = "deserialize_absolute_path")]
    pub workspace_path: AbsolutePath,
    pub branch: BranchState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: ValidatedMetadata,
}

fn serialize_absolute_path<S>(
    path: &AbsolutePath,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(path.as_str())
}

fn deserialize_absolute_path<'de, D>(deserializer: D) -> std::result::Result<AbsolutePath, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    AbsolutePath::parse(s).map_err(serde::de::Error::custom)
}

impl Session {
    pub fn validate_pure(&self) -> Result<()> {
        if self.updated_at < self.created_at {
            return Err(Error::invalid_state(
                "Updated timestamp cannot be before created timestamp".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_pure()
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type_branch_state::BranchState;
    use crate::type_metadata::ValidatedMetadata;

    fn make_test_session() -> Session {
        Session {
            id: crate::type_session_id::SessionId::parse("test-session-id")
                .expect("valid session id"),
            name: crate::type_session_name::SessionName::parse("test-session")
                .expect("valid session name"),
            status: crate::type_session_status::SessionStatus::Active,
            state: crate::workspace_state::WorkspaceState::Working,
            workspace_path: crate::type_session_path::AbsolutePath::parse("/tmp/test-workspace")
                .expect("valid path"),
            branch: BranchState::OnBranch("main".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_synced: None,
            metadata: ValidatedMetadata::default(),
        }
    }

    #[test]
    fn test_session_validate_pure_valid_timestamps() {
        let session = make_test_session();
        assert!(session.validate_pure().is_ok());
    }

    #[test]
    fn test_session_validate_pure_rejects_updated_before_created() {
        let mut session = make_test_session();
        session.created_at = chrono::Utc::now();
        session.updated_at = chrono::Utc::now() - chrono::Duration::seconds(10);
        let result = session.validate_pure();
        assert!(result.is_err());
    }

    #[test]
    fn test_session_validate_pure_equal_timestamps_is_valid() {
        let mut session = make_test_session();
        let now = chrono::Utc::now();
        session.created_at = now;
        session.updated_at = now;
        assert!(session.validate_pure().is_ok());
    }

    #[test]
    fn test_session_validate_delegates_to_validate_pure() {
        let session = make_test_session();
        assert!(session.validate().is_ok());
    }

    #[test]
    fn test_session_name_accessor() {
        let session = make_test_session();
        assert_eq!(session.name(), "test-session");
    }

    #[test]
    fn test_session_with_synced_timestamp() {
        let mut session = make_test_session();
        session.last_synced = Some(chrono::Utc::now());
        assert!(session.last_synced.is_some());
    }

    #[test]
    fn test_session_default_metadata() {
        let session = make_test_session();
        // Default metadata should be empty
        let _meta = &session.metadata;
    }

    #[test]
    fn test_session_clone() {
        let session = make_test_session();
        let cloned = session.clone();
        assert_eq!(session.id.as_str(), cloned.id.as_str());
        assert_eq!(session.name.as_str(), cloned.name.as_str());
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn test_session_serde_roundtrip() {
        let session = make_test_session();
        let json = serde_json::to_string(&session).expect("serialize ok");
        let deserialized: Session =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(session.id.as_str(), deserialized.id.as_str());
        assert_eq!(session.name.as_str(), deserialized.name.as_str());
        assert_eq!(session.status, deserialized.status);
        assert_eq!(session.state, deserialized.state);
        assert_eq!(session.workspace_path.as_str(), deserialized.workspace_path.as_str());
    }

    #[test]
    fn test_session_serde_roundtrip_with_metadata() {
        let mut session = make_test_session();
        session.metadata.insert("author", "test-agent");
        session.metadata.insert("version", "2.0");

        let json = serde_json::to_string(&session).expect("serialize ok");
        let deserialized: Session =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.metadata.get("author"), Some("test-agent"));
        assert_eq!(deserialized.metadata.get("version"), Some("2.0"));
    }

    #[test]
    fn test_session_serde_roundtrip_with_last_synced() {
        let mut session = make_test_session();
        let sync_time = chrono::Utc::now();
        session.last_synced = Some(sync_time);

        let json = serde_json::to_string(&session).expect("serialize ok");
        let deserialized: Session =
            serde_json::from_str(&json).expect("deserialize ok");
        assert!(deserialized.last_synced.is_some());
    }

    #[test]
    fn test_session_serde_skips_none_last_synced() {
        let session = make_test_session();
        let json_val = serde_json::to_value(&session).expect("serialize ok");
        let obj = json_val.as_object().expect("should be object");
        assert!(!obj.contains_key("last_synced"));
    }

    #[test]
    fn test_session_serde_workspace_path_serialized_as_string() {
        let session = make_test_session();
        let json_val = serde_json::to_value(&session).expect("serialize ok");
        let obj = json_val.as_object().expect("should be object");
        let workspace_path = obj.get("workspace_path").expect("has workspace_path");
        assert!(workspace_path.is_string());
        assert_eq!(
            workspace_path.as_str().expect("string"),
            "/tmp/test-workspace"
        );
    }

    #[test]
    fn test_session_serde_deserialize_invalid_path_fails() {
        let json = r#"{
            "id": "test-session-id",
            "name": "test-session",
            "status": "active",
            "state": "working",
            "workspace_path": "not/absolute",
            "branch": {"OnBranch": "main"},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;
        let result: std::result::Result<Session, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // ── Branch states ────────────────────────────────────────────────────────

    #[test]
    fn test_session_branch_state_variants() {
        let on_branch = Session {
            branch: BranchState::OnBranch("feature".to_string()),
            ..make_test_session()
        };
        assert_eq!(on_branch.branch, BranchState::OnBranch("feature".to_string()));
    }

    // ── Debug ────────────────────────────────────────────────────────────────

    #[test]
    fn test_session_debug() {
        let session = make_test_session();
        let debug = format!("{session:?}");
        assert!(debug.contains("test-session"));
    }

    // ── Status variants ──────────────────────────────────────────────────────

    #[test]
    fn test_session_with_different_statuses() {
        for status in [
            crate::type_session_status::SessionStatus::Active,
            crate::type_session_status::SessionStatus::Paused,
            crate::type_session_status::SessionStatus::Completed,
            crate::type_session_status::SessionStatus::Failed,
        ] {
            let mut session = make_test_session();
            session.status = status;
            assert!(session.validate().is_ok());
        }
    }
}
