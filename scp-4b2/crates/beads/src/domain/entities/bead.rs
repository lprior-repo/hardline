use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::super::value_objects::{
    BeadDescription, BeadId, BeadState, BeadTitle, BeadType, Labels, Priority,
};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bead {
    pub id: BeadId,
    pub title: BeadTitle,
    pub description: Option<BeadDescription>,
    pub state: BeadState,
    pub priority: Option<Priority>,
    pub bead_type: Option<BeadType>,
    pub labels: Labels,
    pub assignee: Option<String>,
    pub parent: Option<BeadId>,
    pub depends_on: Vec<BeadId>,
    pub blocked_by: Vec<BeadId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Bead {
    pub fn create(id: BeadId, title: BeadTitle, description: Option<BeadDescription>) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            description,
            state: BeadState::Open,
            priority: None,
            bead_type: None,
            labels: Labels::new(),
            assignee: None,
            parent: None,
            depends_on: Vec::new(),
            blocked_by: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_type(mut self, bead_type: BeadType) -> Self {
        self.bead_type = Some(bead_type);
        self
    }

    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    pub fn with_parent(mut self, parent: BeadId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn add_dependency(mut self, depends_on: BeadId) -> Self {
        self.depends_on.push(depends_on);
        self
    }

    pub fn add_blocker(mut self, blocked_by: BeadId) -> Self {
        self.blocked_by.push(blocked_by);
        self
    }

    pub fn with_labels(mut self, labels: Labels) -> Self {
        self.labels = labels;
        self
    }

    pub fn transition(&self, new_state: BeadState) -> Result<Self> {
        let state = self.state.transition_to(new_state)?;
        Ok(Self {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            state,
            priority: self.priority,
            bead_type: self.bead_type.clone(),
            labels: self.labels.clone(),
            assignee: self.assignee.clone(),
            parent: self.parent.clone(),
            depends_on: self.depends_on.clone(),
            blocked_by: self.blocked_by.clone(),
            created_at: self.created_at,
            updated_at: Utc::now(),
        })
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        !self.blocked_by.is_empty()
    }

    #[must_use]
    pub fn can_transition_to(&self, target: &BeadState) -> bool {
        match (&self.state, target) {
            (_, BeadState::Closed { .. }) => true,
            (BeadState::Closed { .. }, _) => false,
            _ => true,
        }
    }
}
