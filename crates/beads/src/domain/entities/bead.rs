#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::super::value_objects::{
    BeadDescription, BeadId, BeadState, BeadTitle, BeadType, Labels, Priority,
};

impl Default for BeadState {
    fn default() -> Self {
        Self::Open
    }
}

#[derive(Clone)]
pub struct Open;
#[derive(Clone)]
pub struct InProgress;
#[derive(Clone)]
pub struct Blocked;
#[derive(Clone)]
pub struct Deferred;
#[derive(Clone)]
pub struct Closed;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bead<S = Open> {
    pub id: BeadId,
    pub title: BeadTitle,
    pub description: Option<BeadDescription>,
    pub priority: Option<Priority>,
    pub bead_type: Option<BeadType>,
    pub labels: Labels,
    pub assignee: Option<String>,
    pub parent: Option<BeadId>,
    pub depends_on: Vec<BeadId>,
    pub blocked_by: Vec<BeadId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    bead_state: BeadState,
    _state: PhantomData<S>,
}

impl Bead<Open> {
    pub fn create(id: BeadId, title: BeadTitle, description: Option<BeadDescription>) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            description,
            priority: None,
            bead_type: None,
            labels: Labels::new(),
            assignee: None,
            parent: None,
            depends_on: Vec::new(),
            blocked_by: Vec::new(),
            created_at: now,
            updated_at: now,
            bead_state: BeadState::Open,
            _state: PhantomData,
        }
    }

    pub fn start(self) -> Bead<InProgress> {
        self.transition_impl(BeadState::InProgress)
    }
}

impl<S> Bead<S> {
    pub fn id(&self) -> &BeadId {
        &self.id
    }

    pub fn title(&self) -> &BeadTitle {
        &self.title
    }

    pub fn description(&self) -> Option<&BeadDescription> {
        self.description.as_ref()
    }

    pub fn priority(&self) -> Option<&Priority> {
        self.priority.as_ref()
    }

    pub fn bead_type(&self) -> Option<&BeadType> {
        self.bead_type.as_ref()
    }

    pub fn labels(&self) -> &Labels {
        &self.labels
    }

    pub fn assignee(&self) -> Option<&str> {
        self.assignee.as_deref()
    }

    pub fn parent(&self) -> Option<&BeadId> {
        self.parent.as_ref()
    }

    pub fn depends_on(&self) -> &[BeadId] {
        &self.depends_on
    }

    pub fn blocked_by(&self) -> &[BeadId] {
        &self.blocked_by
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn transition_impl<T>(self, target_state: BeadState) -> Bead<T> {
        Bead {
            id: self.id,
            title: self.title,
            description: self.description,
            priority: self.priority,
            bead_type: self.bead_type,
            labels: self.labels,
            assignee: self.assignee,
            parent: self.parent,
            depends_on: self.depends_on,
            blocked_by: self.blocked_by,
            created_at: self.created_at,
            updated_at: Utc::now(),
            bead_state: target_state,
            _state: PhantomData,
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

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        !self.blocked_by.is_empty()
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self.bead_state, BeadState::Closed { .. })
    }

    /// Transition to a new state based on the target BeadState.
    /// This method handles the transition internally, reconstructing the bead
    /// with the correct typestate.
    #[must_use]
    pub fn transition_to(&self, target_state: &BeadState) -> Option<Bead> {
        let current_state = &self.bead_state;
        if !self.can_transition_to(target_state) {
            return None;
        }

        // Don't create a new bead if states are the same
        if current_state == target_state {
            return Some(Bead {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority.clone(),
                bead_type: self.bead_type.clone(),
                labels: self.labels.clone(),
                assignee: self.assignee.clone(),
                parent: self.parent.clone(),
                depends_on: self.depends_on.clone(),
                blocked_by: self.blocked_by.clone(),
                created_at: self.created_at,
                updated_at: Utc::now(),
                bead_state: target_state.clone(),
                _state: PhantomData,
            });
        }

        match (current_state, target_state) {
            (BeadState::Open, BeadState::InProgress) => Some(Bead {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority.clone(),
                bead_type: self.bead_type.clone(),
                labels: self.labels.clone(),
                assignee: self.assignee.clone(),
                parent: self.parent.clone(),
                depends_on: self.depends_on.clone(),
                blocked_by: self.blocked_by.clone(),
                created_at: self.created_at,
                updated_at: Utc::now(),
                bead_state: BeadState::InProgress,
                _state: PhantomData,
            }),
            (BeadState::InProgress, BeadState::Blocked) => Some(Bead {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority.clone(),
                bead_type: self.bead_type.clone(),
                labels: self.labels.clone(),
                assignee: self.assignee.clone(),
                parent: self.parent.clone(),
                depends_on: self.depends_on.clone(),
                blocked_by: self.blocked_by.clone(),
                created_at: self.created_at,
                updated_at: Utc::now(),
                bead_state: BeadState::Blocked,
                _state: PhantomData,
            }),
            (BeadState::InProgress, BeadState::Deferred) => Some(Bead {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority.clone(),
                bead_type: self.bead_type.clone(),
                labels: self.labels.clone(),
                assignee: self.assignee.clone(),
                parent: self.parent.clone(),
                depends_on: self.depends_on.clone(),
                blocked_by: self.blocked_by.clone(),
                created_at: self.created_at,
                updated_at: Utc::now(),
                bead_state: BeadState::Deferred,
                _state: PhantomData,
            }),
            (BeadState::InProgress, BeadState::Closed { .. }) => Some(Bead {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority.clone(),
                bead_type: self.bead_type.clone(),
                labels: self.labels.clone(),
                assignee: self.assignee.clone(),
                parent: self.parent.clone(),
                depends_on: self.depends_on.clone(),
                blocked_by: self.blocked_by.clone(),
                created_at: self.created_at,
                updated_at: Utc::now(),
                bead_state: BeadState::Closed {
                    closed_at: Utc::now(),
                },
                _state: PhantomData,
            }),
            (BeadState::Blocked, BeadState::InProgress) => Some(Bead {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority.clone(),
                bead_type: self.bead_type.clone(),
                labels: self.labels.clone(),
                assignee: self.assignee.clone(),
                parent: self.parent.clone(),
                depends_on: self.depends_on.clone(),
                blocked_by: self.blocked_by.clone(),
                created_at: self.created_at,
                updated_at: Utc::now(),
                bead_state: BeadState::InProgress,
                _state: PhantomData,
            }),
            (BeadState::Blocked, BeadState::Deferred) => Some(Bead {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority.clone(),
                bead_type: self.bead_type.clone(),
                labels: self.labels.clone(),
                assignee: self.assignee.clone(),
                parent: self.parent.clone(),
                depends_on: self.depends_on.clone(),
                blocked_by: self.blocked_by.clone(),
                created_at: self.created_at,
                updated_at: Utc::now(),
                bead_state: BeadState::Deferred,
                _state: PhantomData,
            }),
            (BeadState::Blocked, BeadState::Closed { .. }) => Some(Bead {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority.clone(),
                bead_type: self.bead_type.clone(),
                labels: self.labels.clone(),
                assignee: self.assignee.clone(),
                parent: self.parent.clone(),
                depends_on: self.depends_on.clone(),
                blocked_by: self.blocked_by.clone(),
                created_at: self.created_at,
                updated_at: Utc::now(),
                bead_state: BeadState::Closed {
                    closed_at: Utc::now(),
                },
                _state: PhantomData,
            }),
            (BeadState::Deferred, BeadState::InProgress) => Some(Bead {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority.clone(),
                bead_type: self.bead_type.clone(),
                labels: self.labels.clone(),
                assignee: self.assignee.clone(),
                parent: self.parent.clone(),
                depends_on: self.depends_on.clone(),
                blocked_by: self.blocked_by.clone(),
                created_at: self.created_at,
                updated_at: Utc::now(),
                bead_state: BeadState::InProgress,
                _state: PhantomData,
            }),
            (BeadState::Deferred, BeadState::Closed { .. }) => Some(Bead {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority.clone(),
                bead_type: self.bead_type.clone(),
                labels: self.labels.clone(),
                assignee: self.assignee.clone(),
                parent: self.parent.clone(),
                depends_on: self.depends_on.clone(),
                blocked_by: self.blocked_by.clone(),
                created_at: self.created_at,
                updated_at: Utc::now(),
                bead_state: BeadState::Closed {
                    closed_at: Utc::now(),
                },
                _state: PhantomData,
            }),
            _ => None,
        }
    }
}

impl Bead<InProgress> {
    pub fn block(self) -> Bead<Blocked> {
        self.transition_impl(BeadState::Blocked)
    }

    pub fn defer(self) -> Bead<Deferred> {
        self.transition_impl(BeadState::Deferred)
    }

    pub fn close(self) -> Bead<Closed> {
        self.transition_impl(BeadState::Closed {
            closed_at: Utc::now(),
        })
    }
}

impl Bead<Blocked> {
    pub fn unblock(self) -> Bead<InProgress> {
        self.transition_impl(BeadState::InProgress)
    }

    pub fn defer(self) -> Bead<Deferred> {
        self.transition_impl(BeadState::Deferred)
    }

    pub fn close(self) -> Bead<Closed> {
        self.transition_impl(BeadState::Closed {
            closed_at: Utc::now(),
        })
    }
}

impl Bead<Deferred> {
    pub fn resume(self) -> Bead<InProgress> {
        self.transition_impl(BeadState::InProgress)
    }

    pub fn close(self) -> Bead<Closed> {
        self.transition_impl(BeadState::Closed {
            closed_at: Utc::now(),
        })
    }
}

impl<S> Bead<S> {
    /// Returns the runtime state representation for persistence/storage.
    pub fn state(&self) -> BeadState {
        self.bead_state.clone()
    }

    /// Check if transition to the given state is valid from current typestate.
    #[must_use]
    pub fn can_transition_to(&self, new_state: &BeadState) -> bool {
        match (&self.state(), new_state) {
            // From Open, can go to InProgress
            (BeadState::Open, BeadState::InProgress) => true,
            // From InProgress, can go to Blocked, Deferred, or Closed
            (BeadState::InProgress, BeadState::Blocked) => true,
            (BeadState::InProgress, BeadState::Deferred) => true,
            (BeadState::InProgress, BeadState::Closed { .. }) => true,
            // From Blocked, can go to InProgress, Deferred, or Closed
            (BeadState::Blocked, BeadState::InProgress) => true,
            (BeadState::Blocked, BeadState::Deferred) => true,
            (BeadState::Blocked, BeadState::Closed { .. }) => true,
            // From Deferred, can go to InProgress or Closed
            (BeadState::Deferred, BeadState::InProgress) => true,
            (BeadState::Deferred, BeadState::Closed { .. }) => true,
            // Closed is terminal
            (BeadState::Closed { .. }, _) => false,
            // Same state is allowed (no-op)
            (current, new) => current == new,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bead_when_created_then_has_open_state() {
        let bead = Bead::<Open>::create(
            BeadId::new("test-1").unwrap(),
            BeadTitle::new("Test").unwrap(),
            None,
        );
        assert!(!bead.is_terminal());
    }

    #[test]
    fn bead_given_open_when_start_then_has_in_progress_state() {
        let bead = Bead::<Open>::create(
            BeadId::new("test-1").unwrap(),
            BeadTitle::new("Test").unwrap(),
            None,
        );
        let in_progress: Bead<InProgress> = bead.start();
        assert!(!in_progress.is_terminal());
    }

    #[test]
    fn bead_given_in_progress_when_close_then_has_closed_state() {
        let bead = Bead::<Open>::create(
            BeadId::new("test-1").unwrap(),
            BeadTitle::new("Test").unwrap(),
            None,
        );
        let closed: Bead<Closed> = bead.start().close();
        assert!(closed.is_terminal());
    }

    #[test]
    fn bead_given_blocked_when_unblock_then_has_in_progress_state() {
        let bead = Bead::<Open>::create(
            BeadId::new("test-1").unwrap(),
            BeadTitle::new("Test").unwrap(),
            None,
        );
        let in_progress: Bead<InProgress> = bead.start();
        let blocked: Bead<Blocked> = in_progress.block();
        let unblocked: Bead<InProgress> = blocked.unblock();
        assert!(!unblocked.is_terminal());
    }
}
