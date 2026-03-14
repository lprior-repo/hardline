use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    AgentId, BeadDescription, BeadId, BeadState, BeadTitle, BeadType, Labels, Priority,
};
use crate::error::{BeadError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bead {
    pub id: BeadId,
    pub title: BeadTitle,
    pub description: Option<BeadDescription>,
    pub state: BeadState,
    pub priority: Option<Priority>,
    pub bead_type: Option<BeadType>,
    pub labels: Labels,
    pub claimed_by: Option<AgentId>,
    pub assignee: Option<String>,
    pub parent: Option<BeadId>,
    pub depends_on: Vec<BeadId>,
    pub blocked_by: Vec<BeadId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Bead {
    /// Create a new bead in Open state (Q1: Bead::create() returns Bead with state == Open)
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
            claimed_by: None,
            assignee: None,
            parent: None,
            depends_on: Vec::new(),
            blocked_by: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if this bead can transition to the target state
    #[must_use]
    pub fn can_transition_to(&self, target: &BeadState) -> bool {
        self.state.can_transition_to(target)
    }

    /// Transition to a new state (Q2: returns Err on invalid transition, Q3: returns Ok with new state)
    pub fn transition(&self, new_state: BeadState) -> Result<Self> {
        // Q4: No self-loops allowed (I4)
        if self.state == new_state {
            return Err(BeadError::InvalidStateTransition {
                from: self.state.clone(),
                to: new_state,
            });
        }

        let state = self.state.transition_to(new_state)?;
        Ok(Self {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            state,
            priority: self.priority,
            bead_type: self.bead_type.clone(),
            labels: self.labels.clone(),
            claimed_by: self.claimed_by.clone(),
            assignee: self.assignee.clone(),
            parent: self.parent.clone(),
            depends_on: self.depends_on.clone(),
            blocked_by: self.blocked_by.clone(),
            created_at: self.created_at,
            updated_at: Utc::now(),
        })
    }

    /// Claim this bead (P4: requires state == Open)
    /// Returns Err if already claimed (P4 violation)
    pub fn claim(&self, by: AgentId) -> Result<Self> {
        // P4: Claiming a bead requires current state == Open
        // Also check if already claimed
        if !matches!(self.state, BeadState::Open) {
            return Err(BeadError::InvalidStateTransition {
                from: self.state.clone(),
                to: BeadState::Claimed,
            });
        }
        if self.claimed_by.is_some() {
            return Err(BeadError::BeadAlreadyClaimed {
                bead_id: self.id.clone(),
            });
        }

        let state = self.state.transition_to(BeadState::Claimed)?;
        Ok(Self {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            state,
            priority: self.priority,
            bead_type: self.bead_type.clone(),
            labels: self.labels.clone(),
            claimed_by: Some(by),
            assignee: self.assignee.clone(),
            parent: self.parent.clone(),
            depends_on: self.depends_on.clone(),
            blocked_by: self.blocked_by.clone(),
            created_at: self.created_at,
            updated_at: Utc::now(),
        })
    }

    /// Mark this bead as ready (requires state == InProgress)
    pub fn mark_ready(&self) -> Result<Self> {
        let state = self.state.transition_to(BeadState::Ready)?;
        Ok(Self {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            state,
            priority: self.priority,
            bead_type: self.bead_type.clone(),
            labels: self.labels.clone(),
            claimed_by: self.claimed_by.clone(),
            assignee: self.assignee.clone(),
            parent: self.parent.clone(),
            depends_on: self.depends_on.clone(),
            blocked_by: self.blocked_by.clone(),
            created_at: self.created_at,
            updated_at: Utc::now(),
        })
    }

    /// Merge this bead (requires state == Ready)
    /// Q4: Merged is terminal state
    pub fn merge(&self) -> Result<Self> {
        let state = self.state.transition_to(BeadState::Merged)?;
        Ok(Self {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            state,
            priority: self.priority,
            bead_type: self.bead_type.clone(),
            labels: self.labels.clone(),
            claimed_by: self.claimed_by.clone(),
            assignee: self.assignee.clone(),
            parent: self.parent.clone(),
            depends_on: self.depends_on.clone(),
            blocked_by: self.blocked_by.clone(),
            created_at: self.created_at,
            updated_at: Utc::now(),
        })
    }

    /// Abandon this bead (requires state == Ready)
    /// Q4: Abandoned is terminal state
    pub fn abandon(&self) -> Result<Self> {
        let state = self.state.transition_to(BeadState::Abandoned)?;
        Ok(Self {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            state,
            priority: self.priority,
            bead_type: self.bead_type.clone(),
            labels: self.labels.clone(),
            claimed_by: self.claimed_by.clone(),
            assignee: self.assignee.clone(),
            parent: self.parent.clone(),
            depends_on: self.depends_on.clone(),
            blocked_by: self.blocked_by.clone(),
            created_at: self.created_at,
            updated_at: Utc::now(),
        })
    }
}
