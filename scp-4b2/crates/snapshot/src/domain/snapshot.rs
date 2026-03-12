use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub branch_name: String,
    pub commit_hash: String,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
}

impl Snapshot {
    pub fn create(branch_name: String, commit_hash: String, description: Option<String>) -> Self {
        Self {
            id: SnapshotId::generate(),
            branch_name,
            commit_hash,
            created_at: Utc::now(),
            description,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(String);

impl SnapshotId {
    pub fn generate() -> Self {
        Self(format!("snap-{}", uuid::Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
