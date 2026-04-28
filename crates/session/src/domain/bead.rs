//! Bead aggregate for atomic units of work.
//!
//! This module provides the Bead aggregate with full lifecycle management:
//! - States: Open → InProgress → Blocked → Deferred → Closed
//! - Invariants enforced via type system and runtime checks

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use super::{
    bead_state::BeadState,
    bead_types::{BeadType, Priority},
    bead_value::{BeadDescription, BeadId, BeadTitle},
};
use crate::{domain::workspace_state::WorkspaceState, error::SessionError};

/// Bead aggregate representing an atomic unit of work.
///
/// # State Machine
/// - Open: Bead is available to be worked on (Q11: initial state after create)
/// - InProgress: Bead is actively being worked on
/// - Blocked: Bead has blockers
/// - Deferred: Bead has been deferred
/// - Closed: Bead is done (terminal state)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bead {
    id: BeadId,
    title: BeadTitle,
    description: Option<BeadDescription>,
    bead_type: BeadType,
    priority: Priority,
    state: BeadState,
    assignee: Option<String>,
    parent: Option<BeadId>,
    depends_on: Vec<BeadId>,
    blocked_by: Vec<BeadId>,
    closed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Bead {
    /// Create a new bead in Open state.
    ///
    /// # Preconditions (P7)
    /// - id must be non-empty, ≤100 chars, alphanumeric/hyphen/underscore only
    /// - title must be non-empty, ≤200 chars
    ///
    /// # Postconditions (Q11)
    /// - state = Open
    /// - created_at = updated_at
    pub fn create(id: BeadId, title: BeadTitle, description: Option<BeadDescription>) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            description,
            bead_type: BeadType::default(),
            priority: Priority::default(),
            state: BeadState::Open,
            assignee: None,
            parent: None,
            depends_on: Vec::new(),
            blocked_by: Vec::new(),
            closed_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the priority of this bead
    #[must_use]
    pub fn with_priority(self, priority: Priority) -> Self {
        Self {
            priority,
            updated_at: Utc::now(),
            ..self
        }
    }

    /// Set the type of this bead
    #[must_use]
    pub fn with_type(self, bead_type: BeadType) -> Self {
        Self {
            bead_type,
            updated_at: Utc::now(),
            ..self
        }
    }

    /// Set the assignee of this bead
    #[must_use]
    pub fn with_assignee(self, assignee: impl Into<String>) -> Self {
        Self {
            assignee: Some(assignee.into()),
            updated_at: Utc::now(),
            ..self
        }
    }

    /// Set the parent bead
    #[must_use]
    pub fn with_parent(self, parent: BeadId) -> Self {
        Self {
            parent: Some(parent),
            updated_at: Utc::now(),
            ..self
        }
    }

    /// Add a dependency (this bead depends on another).
    ///
    /// # Preconditions (P9)
    /// - depends_on must be non-empty
    #[must_use]
    pub fn add_dependency(self, depends_on: BeadId) -> Self {
        // I10: No self-references
        if depends_on != self.id && !self.depends_on.contains(&depends_on) {
            let mut depends_on_new = self.depends_on;
            depends_on_new.push(depends_on);
            return Self {
                depends_on: depends_on_new,
                updated_at: Utc::now(),
                ..self
            };
        }
        self
    }

    /// Add a blocker (this bead is blocked by another).
    ///
    /// # Preconditions (P10)
    /// - blocked_by must be non-empty
    #[must_use]
    pub fn add_blocker(self, blocked_by: BeadId) -> Self {
        // I9: No self-references
        if blocked_by != self.id && !self.blocked_by.contains(&blocked_by) {
            let mut blocked_by_new = self.blocked_by;
            blocked_by_new.push(blocked_by);
            return Self {
                blocked_by: blocked_by_new,
                updated_at: Utc::now(),
                ..self
            };
        }
        self
    }

    /// Transition to a new state.
    ///
    /// # Preconditions (P8)
    /// - transition must be valid according to state machine rules
    ///
    /// # Postconditions (Q12, Q13)
    /// - If transitioning to Closed, closed_at is set
    /// - updated_at is always updated
    pub fn transition(&self, new_state: BeadState) -> Result<Self, SessionError> {
        self.validate_closed_state_transition(new_state)?;
        if let Ok(bead) = self.try_transition_to_closed(new_state) {
            return Ok(bead);
        }
        self.validate_state_transition(new_state)?;

        Ok(Self {
            state: new_state,
            updated_at: Utc::now(),
            ..self.clone()
        })
    }

    fn validate_closed_state_transition(&self, new_state: BeadState) -> Result<(), SessionError> {
        if self.state == BeadState::Closed && new_state != BeadState::Closed {
            Err(SessionError::InvalidTransition {
                from: WorkspaceState::Working,
                to: WorkspaceState::Working,
            })
        } else {
            Ok(())
        }
    }

    fn try_transition_to_closed(&self, new_state: BeadState) -> Result<Self, SessionError> {
        if new_state == BeadState::Closed {
            Ok(Self {
                state: new_state,
                closed_at: Some(Utc::now()),
                updated_at: Utc::now(),
                ..self.clone()
            })
        } else {
            Err(SessionError::InvalidTransition {
                from: WorkspaceState::Working,
                to: WorkspaceState::Working,
            })
        }
    }

    fn validate_state_transition(&self, new_state: BeadState) -> Result<(), SessionError> {
        if self.state.can_transition_to(new_state) {
            Ok(())
        } else {
            Err(SessionError::InvalidTransition {
                from: WorkspaceState::Working,
                to: WorkspaceState::Working,
            })
        }
    }

    /// Check if this bead is blocked.
    ///
    /// # Postconditions (Q14)
    /// - returns true iff blocked_by is non-empty
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        !self.blocked_by.is_empty()
    }

    /// Check if a transition to the given state is possible.
    ///
    /// # Postconditions (Q15, Q16)
    /// - returns false when transitioning from Closed to any other state
    /// - returns true for any transition TO Closed
    #[must_use]
    pub fn can_transition_to(&self, new_state: BeadState) -> bool {
        // Q16: Can always transition to Closed
        if new_state == BeadState::Closed {
            return true;
        }
        // Q15: Cannot transition from Closed
        if self.state == BeadState::Closed {
            return false;
        }
        self.state.can_transition_to(new_state)
    }

    // Getters
    #[must_use]
    pub fn id(&self) -> &BeadId {
        &self.id
    }

    #[must_use]
    pub fn title(&self) -> &BeadTitle {
        &self.title
    }

    #[must_use]
    pub fn description(&self) -> Option<&BeadDescription> {
        self.description.as_ref()
    }

    #[must_use]
    pub fn bead_type(&self) -> BeadType {
        self.bead_type
    }

    #[must_use]
    pub fn priority(&self) -> Priority {
        self.priority
    }

    #[must_use]
    pub fn state(&self) -> BeadState {
        self.state
    }

    #[must_use]
    pub fn assignee(&self) -> Option<&String> {
        self.assignee.as_ref()
    }

    #[must_use]
    pub fn parent(&self) -> Option<&BeadId> {
        self.parent.as_ref()
    }

    #[must_use]
    pub fn depends_on(&self) -> &[BeadId] {
        &self.depends_on
    }

    #[must_use]
    pub fn blocked_by(&self) -> &[BeadId] {
        &self.blocked_by
    }

    #[must_use]
    pub fn closed_at(&self) -> Option<DateTime<Utc>> {
        self.closed_at
    }

    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
