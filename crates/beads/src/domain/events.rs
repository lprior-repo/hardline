use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{BeadId, BeadState, BeadTitle, Priority};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BeadEvent {
    Created {
        id: BeadId,
        title: BeadTitle,
        created_at: DateTime<Utc>,
    },

    TitleChanged {
        id: BeadId,
        old_title: BeadTitle,
        new_title: BeadTitle,
        changed_at: DateTime<Utc>,
    },

    StateChanged {
        id: BeadId,
        old_state: BeadState,
        new_state: BeadState,
        changed_at: DateTime<Utc>,
    },

    PrioritySet {
        id: BeadId,
        priority: Priority,
        changed_at: DateTime<Utc>,
    },

    AssigneeSet {
        id: BeadId,
        assignee: Option<String>,
        changed_at: DateTime<Utc>,
    },

    DependencyAdded {
        id: BeadId,
        depends_on: BeadId,
        changed_at: DateTime<Utc>,
    },

    BlockerAdded {
        id: BeadId,
        blocked_by: BeadId,
        changed_at: DateTime<Utc>,
    },

    Labeled {
        id: BeadId,
        label: String,
        changed_at: DateTime<Utc>,
    },

    Deleted {
        id: BeadId,
        deleted_at: DateTime<Utc>,
    },
}

impl BeadEvent {
    #[must_use]
    pub fn id(&self) -> &BeadId {
        match self {
            Self::Created { id, .. } => id,
            Self::TitleChanged { id, .. } => id,
            Self::StateChanged { id, .. } => id,
            Self::PrioritySet { id, .. } => id,
            Self::AssigneeSet { id, .. } => id,
            Self::DependencyAdded { id, .. } => id,
            Self::BlockerAdded { id, .. } => id,
            Self::Labeled { id, .. } => id,
            Self::Deleted { id, .. } => id,
        }
    }
}
