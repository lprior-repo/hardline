use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkspaceEvent {
    WorkspaceCreated {
        workspace_id: String,
        name: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceActivated {
        workspace_id: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceLocked {
        workspace_id: String,
        holder: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceUnlocked {
        workspace_id: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceCorrupted {
        workspace_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceDeleted {
        workspace_id: String,
        timestamp: DateTime<Utc>,
    },
    WorkspaceConfigUpdated {
        workspace_id: String,
        timestamp: DateTime<Utc>,
    },
}

impl WorkspaceEvent {
    pub fn workspace_created(workspace_id: String, name: String) -> Self {
        Self::WorkspaceCreated {
            workspace_id,
            name,
            timestamp: Utc::now(),
        }
    }

    pub fn workspace_locked(workspace_id: String, holder: String) -> Self {
        Self::WorkspaceLocked {
            workspace_id,
            holder,
            timestamp: Utc::now(),
        }
    }
}
