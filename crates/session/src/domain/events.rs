use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::entities::session::{SessionId, SessionState};
use super::value_objects::SessionName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEvent {
    Activated,
    Paused,
    Resumed,
    Syncing,
    Synced,
    Completed,
    Failed { reason: String },
    BeadClaimed { bead_id: String },
    BeadReleased,
    BranchCreated { branch_name: String },
    BranchDeleted,
}

impl std::fmt::Display for SessionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Activated => write!(f, "Activated"),
            Self::Paused => write!(f, "Paused"),
            Self::Resumed => write!(f, "Resumed"),
            Self::Syncing => write!(f, "Syncing"),
            Self::Synced => write!(f, "Synced"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed { reason } => write!(f, "Failed: {}", reason),
            Self::BeadClaimed { bead_id } => write!(f, "BeadClaimed: {}", bead_id),
            Self::BeadReleased => write!(f, "BeadReleased"),
            Self::BranchCreated { branch_name } => {
                write!(f, "BranchCreated: {}", branch_name)
            }
            Self::BranchDeleted => write!(f, "BranchDeleted"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreatedEvent {
    pub id: SessionId,
    pub name: SessionName,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCompletedEvent {
    pub id: SessionId,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFailedEvent {
    pub id: SessionId,
    pub reason: String,
    pub failed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStateChangedEvent {
    pub id: SessionId,
    pub old_state: SessionState,
    pub new_state: SessionState,
    pub changed_at: DateTime<Utc>,
}
