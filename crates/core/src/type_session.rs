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
