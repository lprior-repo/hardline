#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::struct_field_names)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::derivable_impls)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct Open;
#[derive(Clone, Debug, PartialEq)]
pub struct InProgress;
#[derive(Clone, Debug, PartialEq)]
pub struct Blocked;
#[derive(Clone, Debug, PartialEq)]
pub struct Deferred;
#[derive(Clone, Debug, PartialEq)]
pub struct Closed;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[must_use]
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

    #[must_use]
    pub fn start(self) -> Bead<InProgress> {
        self.transition_impl(BeadState::InProgress)
    }
}

impl<S> Bead<S> {
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
    pub fn priority(&self) -> Option<&Priority> {
        self.priority.as_ref()
    }

    #[must_use]
    pub fn bead_type(&self) -> Option<&BeadType> {
        self.bead_type.as_ref()
    }

    #[must_use]
    pub fn labels(&self) -> &Labels {
        &self.labels
    }

    #[must_use]
    pub fn assignee(&self) -> Option<&str> {
        self.assignee.as_deref()
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
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    #[must_use]
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

    #[must_use]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    #[must_use]
    pub fn with_type(mut self, bead_type: BeadType) -> Self {
        self.bead_type = Some(bead_type);
        self
    }

    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    #[must_use]
    pub fn with_parent(mut self, parent: BeadId) -> Self {
        self.parent = Some(parent);
        self
    }

    #[must_use]
    pub fn add_dependency(mut self, depends_on: BeadId) -> Self {
        self.depends_on.push(depends_on);
        self
    }

    #[must_use]
    pub fn add_blocker(mut self, blocked_by: BeadId) -> Self {
        self.blocked_by.push(blocked_by);
        self
    }

    #[must_use]
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

    /// Transition to a new state based on the target `BeadState`.
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
                priority: self.priority,
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
                priority: self.priority,
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
                priority: self.priority,
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
                priority: self.priority,
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
                priority: self.priority,
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
                priority: self.priority,
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
                priority: self.priority,
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
                priority: self.priority,
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
                priority: self.priority,
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
                priority: self.priority,
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
    #[must_use]
    pub fn block(self) -> Bead<Blocked> {
        self.transition_impl(BeadState::Blocked)
    }

    #[must_use]
    pub fn defer(self) -> Bead<Deferred> {
        self.transition_impl(BeadState::Deferred)
    }

    #[must_use]
    pub fn close(self) -> Bead<Closed> {
        self.transition_impl(BeadState::Closed {
            closed_at: Utc::now(),
        })
    }
}

impl Bead<Blocked> {
    #[must_use]
    pub fn unblock(self) -> Bead<InProgress> {
        self.transition_impl(BeadState::InProgress)
    }

    #[must_use]
    pub fn defer(self) -> Bead<Deferred> {
        self.transition_impl(BeadState::Deferred)
    }

    #[must_use]
    pub fn close(self) -> Bead<Closed> {
        self.transition_impl(BeadState::Closed {
            closed_at: Utc::now(),
        })
    }
}

impl Bead<Deferred> {
    #[must_use]
    pub fn resume(self) -> Bead<InProgress> {
        self.transition_impl(BeadState::InProgress)
    }

    #[must_use]
    pub fn close(self) -> Bead<Closed> {
        self.transition_impl(BeadState::Closed {
            closed_at: Utc::now(),
        })
    }
}

impl<S> Bead<S> {
    /// Returns the runtime state representation for persistence/storage.
    #[must_use]
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

    fn make_bead() -> Bead<Open> {
        Bead::<Open>::create(
            BeadId::new("test-1").unwrap(),
            BeadTitle::new("Test Bead").unwrap(),
            Some(BeadDescription::new("A test bead description").unwrap()),
        )
    }

    // ── Creation tests ──────────────────────────────────────────────────────

    #[test]
    fn bead_when_created_then_has_open_state() {
        let bead = make_bead();
        assert_eq!(bead.state(), BeadState::Open);
        assert!(!bead.is_terminal());
    }

    #[test]
    fn bead_when_created_then_preserves_id() {
        let bead = make_bead();
        assert_eq!(bead.id().as_str(), "test-1");
    }

    #[test]
    fn bead_when_created_then_preserves_title() {
        let bead = make_bead();
        assert_eq!(bead.title().as_str(), "Test Bead");
    }

    #[test]
    fn bead_when_created_then_preserves_description() {
        let bead = make_bead();
        assert_eq!(
            bead.description().unwrap().as_str(),
            "A test bead description"
        );
    }

    #[test]
    fn bead_when_created_without_description_then_description_is_none() {
        let bead = Bead::<Open>::create(
            BeadId::new("test-2").unwrap(),
            BeadTitle::new("No Desc").unwrap(),
            None,
        );
        assert!(bead.description().is_none());
    }

    #[test]
    fn bead_when_created_then_has_no_priority() {
        let bead = make_bead();
        assert!(bead.priority().is_none());
    }

    #[test]
    fn bead_when_created_then_has_no_type() {
        let bead = make_bead();
        assert!(bead.bead_type().is_none());
    }

    #[test]
    fn bead_when_created_then_has_empty_labels() {
        let bead = make_bead();
        assert!(bead.labels().as_slice().is_empty());
    }

    #[test]
    fn bead_when_created_then_has_no_assignee() {
        let bead = make_bead();
        assert!(bead.assignee().is_none());
    }

    #[test]
    fn bead_when_created_then_has_no_parent() {
        let bead = make_bead();
        assert!(bead.parent().is_none());
    }

    #[test]
    fn bead_when_created_then_has_empty_dependencies() {
        let bead = make_bead();
        assert!(bead.depends_on().is_empty());
    }

    #[test]
    fn bead_when_created_then_has_empty_blocked_by() {
        let bead = make_bead();
        assert!(bead.blocked_by().is_empty());
        assert!(!bead.is_blocked());
    }

    #[test]
    fn bead_when_created_then_has_valid_timestamps() {
        let before = Utc::now();
        let bead = make_bead();
        let after = Utc::now();
        assert!(bead.created_at() >= before);
        assert!(bead.created_at() <= after);
        assert!(bead.updated_at() >= before);
        assert!(bead.updated_at() <= after);
    }

    // ── Builder method tests ────────────────────────────────────────────────

    #[test]
    fn with_priority_sets_priority() {
        let bead = make_bead().with_priority(Priority::P1);
        assert_eq!(bead.priority(), Some(&Priority::P1));
    }

    #[test]
    fn with_type_sets_bead_type() {
        let bead = make_bead().with_type(BeadType::Bug);
        assert_eq!(bead.bead_type(), Some(&BeadType::Bug));
    }

    #[test]
    fn with_assignee_sets_assignee() {
        let bead = make_bead().with_assignee("alice");
        assert_eq!(bead.assignee(), Some("alice"));
    }

    #[test]
    fn with_parent_sets_parent() {
        let parent_id = BeadId::new("parent-1").unwrap();
        let bead = make_bead().with_parent(parent_id.clone());
        assert_eq!(bead.parent(), Some(&parent_id));
    }

    #[test]
    fn add_dependency_adds_to_list() {
        let dep = BeadId::new("dep-1").unwrap();
        let bead = make_bead().add_dependency(dep.clone());
        assert_eq!(bead.depends_on().len(), 1);
        assert_eq!(bead.depends_on()[0], dep);
    }

    #[test]
    fn add_dependency_accumulates() {
        let dep1 = BeadId::new("dep-1").unwrap();
        let dep2 = BeadId::new("dep-2").unwrap();
        let bead = make_bead()
            .add_dependency(dep1.clone())
            .add_dependency(dep2.clone());
        assert_eq!(bead.depends_on().len(), 2);
        assert_eq!(bead.depends_on()[0], dep1);
        assert_eq!(bead.depends_on()[1], dep2);
    }

    #[test]
    fn add_blocker_adds_to_list() {
        let blocker = BeadId::new("blocker-1").unwrap();
        let bead = make_bead().add_blocker(blocker.clone());
        assert_eq!(bead.blocked_by().len(), 1);
        assert_eq!(bead.blocked_by()[0], blocker);
        assert!(bead.is_blocked());
    }

    #[test]
    fn with_labels_sets_labels() {
        let labels = Labels::new().with("urgent").with("backend");
        let bead = make_bead().with_labels(labels.clone());
        assert_eq!(bead.labels().as_slice(), labels.as_slice());
    }

    // ── Typestate transition tests (compile-time safe) ─────────────────────

    #[test]
    fn bead_given_open_when_start_then_has_in_progress_state() {
        let bead = make_bead();
        let in_progress: Bead<InProgress> = bead.start();
        assert_eq!(in_progress.state(), BeadState::InProgress);
        assert!(!in_progress.is_terminal());
    }

    #[test]
    fn bead_given_in_progress_when_close_then_has_closed_state() {
        let bead = make_bead();
        let closed: Bead<Closed> = bead.start().close();
        assert!(closed.is_terminal());
        assert!(closed.state().is_closed());
        assert!(closed.state().closed_at().is_some());
    }

    #[test]
    fn bead_given_in_progress_when_block_then_has_blocked_state() {
        let bead = make_bead();
        let blocked: Bead<Blocked> = bead.start().block();
        assert_eq!(blocked.state(), BeadState::Blocked);
        assert!(!blocked.is_terminal());
    }

    #[test]
    fn bead_given_in_progress_when_defer_then_has_deferred_state() {
        let bead = make_bead();
        let deferred: Bead<Deferred> = bead.start().defer();
        assert_eq!(deferred.state(), BeadState::Deferred);
        assert!(!deferred.is_terminal());
    }

    #[test]
    fn bead_given_blocked_when_unblock_then_has_in_progress_state() {
        let bead = make_bead();
        let in_progress: Bead<InProgress> = bead.start().block().unblock();
        assert_eq!(in_progress.state(), BeadState::InProgress);
        assert!(!in_progress.is_terminal());
    }

    #[test]
    fn bead_given_blocked_when_defer_then_has_deferred_state() {
        let bead = make_bead();
        let deferred: Bead<Deferred> = bead.start().block().defer();
        assert_eq!(deferred.state(), BeadState::Deferred);
    }

    #[test]
    fn bead_given_blocked_when_close_then_has_closed_state() {
        let bead = make_bead();
        let closed: Bead<Closed> = bead.start().block().close();
        assert!(closed.is_terminal());
    }

    #[test]
    fn bead_given_deferred_when_resume_then_has_in_progress_state() {
        let bead = make_bead();
        let in_progress: Bead<InProgress> = bead.start().defer().resume();
        assert_eq!(in_progress.state(), BeadState::InProgress);
    }

    #[test]
    fn bead_given_deferred_when_close_then_has_closed_state() {
        let bead = make_bead();
        let closed: Bead<Closed> = bead.start().defer().close();
        assert!(closed.is_terminal());
    }

    #[test]
    fn full_lifecycle_open_to_closed() {
        let bead = make_bead();
        let closed: Bead<Closed> = bead.start().block().unblock().defer().resume().close();
        assert!(closed.is_terminal());
        assert!(closed.state().is_closed());
    }

    #[test]
    fn transition_updates_updated_at() {
        let bead = make_bead();
        let original_updated = bead.updated_at();
        // Small delay to ensure timestamp differs
        std::thread::sleep(std::time::Duration::from_millis(2));
        let in_progress = bead.start();
        assert!(in_progress.updated_at() >= original_updated);
    }

    // ── transition_to (dynamic) tests ───────────────────────────────────────

    #[test]
    fn transition_to_open_from_open_returns_bead() {
        let bead = make_bead();
        let result = bead.transition_to(&BeadState::Open);
        assert!(result.is_some());
        assert_eq!(result.unwrap().state(), BeadState::Open);
    }

    #[test]
    fn transition_to_in_progress_from_open() {
        let bead = make_bead();
        let result = bead.transition_to(&BeadState::InProgress);
        assert!(result.is_some());
        assert_eq!(result.unwrap().state(), BeadState::InProgress);
    }

    #[test]
    fn transition_to_closed_from_in_progress() {
        let bead = make_bead().start();
        let result = bead.transition_to(&BeadState::Closed {
            closed_at: Utc::now(),
        });
        assert!(result.is_some());
        let transitioned = result.unwrap();
        assert!(transitioned.state().is_closed());
        assert!(transitioned.state().closed_at().is_some());
    }

    #[test]
    fn transition_to_blocked_from_in_progress() {
        let bead = make_bead().start();
        let result = bead.transition_to(&BeadState::Blocked);
        assert!(result.is_some());
        assert_eq!(result.unwrap().state(), BeadState::Blocked);
    }

    #[test]
    fn transition_to_deferred_from_in_progress() {
        let bead = make_bead().start();
        let result = bead.transition_to(&BeadState::Deferred);
        assert!(result.is_some());
        assert_eq!(result.unwrap().state(), BeadState::Deferred);
    }

    #[test]
    fn transition_to_in_progress_from_blocked() {
        // transition_to works on any Bead<S>, using dynamic transition
        let open_bead = make_bead();
        let in_progress = open_bead.transition_to(&BeadState::InProgress).unwrap();
        let blocked = in_progress.transition_to(&BeadState::Blocked).unwrap();
        let back = blocked.transition_to(&BeadState::InProgress).unwrap();
        assert_eq!(back.state(), BeadState::InProgress);
    }

    #[test]
    fn transition_to_deferred_from_blocked() {
        let bead = make_bead();
        let in_progress = bead.transition_to(&BeadState::InProgress).unwrap();
        let blocked = in_progress.transition_to(&BeadState::Blocked).unwrap();
        let deferred = blocked.transition_to(&BeadState::Deferred).unwrap();
        assert_eq!(deferred.state(), BeadState::Deferred);
    }

    #[test]
    fn transition_to_closed_from_blocked() {
        let bead = make_bead();
        let in_progress = bead.transition_to(&BeadState::InProgress).unwrap();
        let blocked = in_progress.transition_to(&BeadState::Blocked).unwrap();
        let closed = blocked
            .transition_to(&BeadState::Closed {
                closed_at: Utc::now(),
            })
            .unwrap();
        assert!(closed.state().is_closed());
    }

    #[test]
    fn transition_to_in_progress_from_deferred() {
        let bead = make_bead();
        let in_progress = bead.transition_to(&BeadState::InProgress).unwrap();
        let deferred = in_progress.transition_to(&BeadState::Deferred).unwrap();
        let resumed = deferred.transition_to(&BeadState::InProgress).unwrap();
        assert_eq!(resumed.state(), BeadState::InProgress);
    }

    #[test]
    fn transition_to_closed_from_deferred() {
        let bead = make_bead();
        let in_progress = bead.transition_to(&BeadState::InProgress).unwrap();
        let deferred = in_progress.transition_to(&BeadState::Deferred).unwrap();
        let closed = deferred
            .transition_to(&BeadState::Closed {
                closed_at: Utc::now(),
            })
            .unwrap();
        assert!(closed.state().is_closed());
    }

    #[test]
    fn transition_from_closed_returns_none() {
        let bead = make_bead();
        let closed = bead.transition_to(&BeadState::InProgress).unwrap();
        let closed = closed
            .transition_to(&BeadState::Closed {
                closed_at: Utc::now(),
            })
            .unwrap();
        // Cannot transition from Closed to anything
        let result = closed.transition_to(&BeadState::Open);
        assert!(result.is_none());
        let result = closed.transition_to(&BeadState::InProgress);
        assert!(result.is_none());
    }

    #[test]
    fn transition_to_invalid_from_open_returns_none() {
        let bead = make_bead();
        // Cannot go from Open directly to Blocked
        let result = bead.transition_to(&BeadState::Blocked);
        assert!(result.is_none());
    }

    #[test]
    fn transition_to_invalid_from_open_to_closed_returns_none() {
        let bead = make_bead();
        // Cannot go from Open directly to Closed
        let result = bead.transition_to(&BeadState::Closed {
            closed_at: Utc::now(),
        });
        assert!(result.is_none());
    }

    // ── can_transition_to tests ─────────────────────────────────────────────

    #[test]
    fn can_transition_open_to_in_progress() {
        let bead = make_bead();
        assert!(bead.can_transition_to(&BeadState::InProgress));
    }

    #[test]
    fn cannot_transition_open_to_blocked() {
        let bead = make_bead();
        assert!(!bead.can_transition_to(&BeadState::Blocked));
    }

    #[test]
    fn cannot_transition_open_to_deferred() {
        let bead = make_bead();
        assert!(!bead.can_transition_to(&BeadState::Deferred));
    }

    #[test]
    fn cannot_transition_open_to_closed() {
        let bead = make_bead();
        assert!(!bead.can_transition_to(&BeadState::Closed {
            closed_at: Utc::now(),
        }));
    }

    #[test]
    fn can_transition_open_to_open() {
        let bead = make_bead();
        assert!(bead.can_transition_to(&BeadState::Open));
    }

    #[test]
    fn can_transition_in_progress_to_blocked() {
        let bead = make_bead().start();
        assert!(bead.can_transition_to(&BeadState::Blocked));
    }

    #[test]
    fn can_transition_in_progress_to_deferred() {
        let bead = make_bead().start();
        assert!(bead.can_transition_to(&BeadState::Deferred));
    }

    #[test]
    fn can_transition_in_progress_to_closed() {
        let bead = make_bead().start();
        assert!(bead.can_transition_to(&BeadState::Closed {
            closed_at: Utc::now(),
        }));
    }

    #[test]
    fn can_transition_blocked_to_in_progress() {
        let _bead = make_bead().start().block();
        assert!(_bead.can_transition_to(&BeadState::InProgress));
    }

    #[test]
    fn can_transition_blocked_to_deferred() {
        let bead = make_bead().start().block();
        assert!(bead.can_transition_to(&BeadState::Deferred));
    }

    #[test]
    fn can_transition_blocked_to_closed() {
        let bead = make_bead().start().block();
        assert!(bead.can_transition_to(&BeadState::Closed {
            closed_at: Utc::now(),
        }));
    }

    #[test]
    fn can_transition_deferred_to_in_progress() {
        let bead = make_bead().start().defer();
        assert!(bead.can_transition_to(&BeadState::InProgress));
    }

    #[test]
    fn can_transition_deferred_to_closed() {
        let bead = make_bead().start().defer();
        assert!(bead.can_transition_to(&BeadState::Closed {
            closed_at: Utc::now(),
        }));
    }

    #[test]
    fn cannot_transition_deferred_to_blocked() {
        let bead = make_bead().start().defer();
        assert!(!bead.can_transition_to(&BeadState::Blocked));
    }

    #[test]
    fn cannot_transition_closed_to_anything() {
        let bead = make_bead().start().close();
        assert!(!bead.can_transition_to(&BeadState::Open));
        assert!(!bead.can_transition_to(&BeadState::InProgress));
        assert!(!bead.can_transition_to(&BeadState::Blocked));
        assert!(!bead.can_transition_to(&BeadState::Deferred));
        assert!(!bead.can_transition_to(&BeadState::Closed {
            closed_at: Utc::now(),
        }));
    }

    // ── is_blocked / is_terminal tests ──────────────────────────────────────

    #[test]
    fn is_blocked_true_when_blockers_present() {
        let bead = make_bead().add_blocker(BeadId::new("b1").unwrap());
        assert!(bead.is_blocked());
    }

    #[test]
    fn is_blocked_false_when_no_blockers() {
        assert!(!make_bead().is_blocked());
    }

    #[test]
    fn is_terminal_true_for_closed() {
        let bead = make_bead().start().close();
        assert!(bead.is_terminal());
    }

    #[test]
    fn is_terminal_false_for_open() {
        assert!(!make_bead().is_terminal());
    }

    #[test]
    fn is_terminal_false_for_in_progress() {
        assert!(!make_bead().start().is_terminal());
    }

    #[test]
    fn is_terminal_false_for_blocked() {
        assert!(!make_bead().start().block().is_terminal());
    }

    #[test]
    fn is_terminal_false_for_deferred() {
        assert!(!make_bead().start().defer().is_terminal());
    }

    // ── Cloning tests ───────────────────────────────────────────────────────

    #[test]
    fn bead_can_be_cloned() {
        let bead = make_bead()
            .with_priority(Priority::P2)
            .with_type(BeadType::Feature)
            .with_assignee("bob");
        let cloned = bead.clone();
        assert_eq!(cloned.id().as_str(), bead.id().as_str());
        assert_eq!(cloned.title().as_str(), bead.title().as_str());
        assert_eq!(cloned.priority(), bead.priority());
        assert_eq!(cloned.bead_type(), bead.bead_type());
        assert_eq!(cloned.assignee(), bead.assignee());
    }

    // ── Serde tests ─────────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let bead = make_bead()
            .with_priority(Priority::P0)
            .with_type(BeadType::Task)
            .with_labels(Labels::new().with("core"));
        let json = serde_json::to_string(&bead).unwrap();
        let parsed: Bead = serde_json::from_str(&json).unwrap();
        assert_eq!(bead, parsed);
    }

    #[test]
    fn serde_preserves_state() {
        let bead = make_bead().start();
        let json = serde_json::to_string(&bead).unwrap();
        let parsed: Bead = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.state(), BeadState::InProgress);
    }

    // ── Accessor tests ──────────────────────────────────────────────────────

    #[test]
    fn id_accessor() {
        let bead = make_bead();
        assert_eq!(bead.id().as_str(), "test-1");
    }

    #[test]
    fn title_accessor() {
        let bead = make_bead();
        assert_eq!(bead.title().as_str(), "Test Bead");
    }

    #[test]
    fn description_accessor_some() {
        let bead = make_bead();
        assert!(bead.description().is_some());
    }

    #[test]
    fn description_accessor_none() {
        let bead = Bead::<Open>::create(
            BeadId::new("t").unwrap(),
            BeadTitle::new("T").unwrap(),
            None,
        );
        assert!(bead.description().is_none());
    }

    #[test]
    fn created_at_and_updated_at_are_reasonable() {
        let now = Utc::now();
        let bead = make_bead();
        // created_at and updated_at should be close to "now"
        let diff = (bead.created_at() - now).num_seconds().abs();
        assert!(diff < 2, "created_at differs from now by {diff}s");
    }

    // ── Serde with non-Open states ──────────────────────────────────────────

    #[test]
    fn serde_roundtrip_blocked_state() {
        let bead = make_bead().start().block();
        let json = serde_json::to_string(&bead).unwrap();
        let parsed: Bead = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.state(), BeadState::Blocked);
    }

    #[test]
    fn serde_roundtrip_deferred_state() {
        let bead = make_bead().start().defer();
        let json = serde_json::to_string(&bead).unwrap();
        let parsed: Bead = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.state(), BeadState::Deferred);
    }

    #[test]
    fn serde_roundtrip_closed_state() {
        let bead = make_bead().start().close();
        let json = serde_json::to_string(&bead).unwrap();
        let parsed: Bead = serde_json::from_str(&json).unwrap();
        assert!(parsed.state().is_closed());
        assert!(parsed.state().closed_at().is_some());
    }

    #[test]
    fn serde_roundtrip_all_fields_populated() {
        let dep_id = BeadId::new("dep-x").unwrap();
        let blocker_id = BeadId::new("blk-x").unwrap();
        let parent_id = BeadId::new("par-x").unwrap();
        let bead = make_bead()
            .with_priority(Priority::P0)
            .with_type(BeadType::Feature)
            .with_assignee("charlie")
            .with_parent(parent_id)
            .add_dependency(dep_id)
            .add_blocker(blocker_id)
            .with_labels(Labels::new().with("core").with("urgent"));
        let json = serde_json::to_string(&bead).unwrap();
        let parsed: Bead = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id().as_str(), "test-1");
        assert_eq!(parsed.title().as_str(), "Test Bead");
        assert_eq!(
            parsed.description().unwrap().as_str(),
            "A test bead description"
        );
        assert_eq!(parsed.priority(), Some(&Priority::P0));
        assert_eq!(parsed.bead_type(), Some(&BeadType::Feature));
        assert_eq!(parsed.assignee(), Some("charlie"));
        assert_eq!(parsed.parent().unwrap().as_str(), "par-x");
        assert_eq!(parsed.depends_on().len(), 1);
        assert_eq!(parsed.depends_on()[0].as_str(), "dep-x");
        assert_eq!(parsed.blocked_by().len(), 1);
        assert_eq!(parsed.blocked_by()[0].as_str(), "blk-x");
        assert!(parsed.is_blocked());
        assert_eq!(parsed.labels().as_slice().len(), 2);
    }

    // ── transition_to same-state tests ──────────────────────────────────────

    #[test]
    fn transition_to_same_state_in_progress() {
        let bead = make_bead().start();
        let result = bead.transition_to(&BeadState::InProgress);
        assert!(result.is_some());
        assert_eq!(result.unwrap().state(), BeadState::InProgress);
    }

    #[test]
    fn transition_to_same_state_blocked() {
        let bead = make_bead();
        let blocked = bead.transition_to(&BeadState::InProgress).unwrap();
        let blocked = blocked.transition_to(&BeadState::Blocked).unwrap();
        let result = blocked.transition_to(&BeadState::Blocked);
        assert!(result.is_some());
        assert_eq!(result.unwrap().state(), BeadState::Blocked);
    }

    #[test]
    fn transition_to_same_state_deferred() {
        let bead = make_bead();
        let deferred = bead.transition_to(&BeadState::InProgress).unwrap();
        let deferred = deferred.transition_to(&BeadState::Deferred).unwrap();
        let result = deferred.transition_to(&BeadState::Deferred);
        assert!(result.is_some());
        assert_eq!(result.unwrap().state(), BeadState::Deferred);
    }

    // ── Transition chain tests (ha-dl99) ──────────────────────────────────

    #[test]
    fn entity_chain_open_to_in_progress_to_closed() {
        let bead = make_bead();
        assert_eq!(bead.state(), BeadState::Open);
        assert!(!bead.is_terminal());

        // Open → InProgress via typestate API
        let in_progress: Bead<InProgress> = bead.start();
        assert_eq!(in_progress.state(), BeadState::InProgress);
        assert!(!in_progress.is_terminal());

        // InProgress → Closed via typestate API
        let closed: Bead<Closed> = in_progress.close();
        assert!(closed.is_terminal());
        assert!(closed.state().is_closed());
        assert!(closed.state().closed_at().is_some());
    }

    #[test]
    fn entity_chain_dynamic_open_to_in_progress_to_closed() {
        let bead = make_bead();
        assert_eq!(bead.state(), BeadState::Open);

        // Open → InProgress via dynamic transition_to
        let in_progress = bead
            .transition_to(&BeadState::InProgress)
            .expect("Open→InProgress should succeed");
        assert_eq!(in_progress.state(), BeadState::InProgress);

        // InProgress → Closed via dynamic transition_to
        let closed = in_progress
            .transition_to(&BeadState::Closed {
                closed_at: Utc::now(),
            })
            .expect("InProgress→Closed should succeed");
        assert!(closed.state().is_closed());
        assert!(closed.state().closed_at().is_some());
    }

    #[test]
    fn entity_open_cannot_transition_to_blocked_or_deferred() {
        let bead = make_bead();
        // Entity enforces Open can only go to InProgress
        assert!(!bead.can_transition_to(&BeadState::Blocked));
        assert!(!bead.can_transition_to(&BeadState::Deferred));
        assert!(!bead.can_transition_to(&BeadState::Closed {
            closed_at: Utc::now(),
        }));
        assert!(bead.can_transition_to(&BeadState::InProgress));
    }

    #[test]
    fn entity_blocked_can_return_to_in_progress_not_open() {
        let blocked = make_bead().start().block();
        assert!(blocked.can_transition_to(&BeadState::InProgress));
        assert!(!blocked.can_transition_to(&BeadState::Open));
    }

    #[test]
    fn entity_deferred_can_return_to_in_progress_not_open() {
        let deferred = make_bead().start().defer();
        assert!(deferred.can_transition_to(&BeadState::InProgress));
        assert!(!deferred.can_transition_to(&BeadState::Open));
    }

    // ── transition_to invalid paths ─────────────────────────────────────────

    #[test]
    fn transition_in_progress_to_open_returns_none() {
        let bead = make_bead().start();
        let result = bead.transition_to(&BeadState::Open);
        assert!(result.is_none());
    }

    #[test]
    fn transition_blocked_to_open_returns_none() {
        let bead = make_bead();
        let blocked = bead.transition_to(&BeadState::InProgress).unwrap();
        let blocked = blocked.transition_to(&BeadState::Blocked).unwrap();
        let result = blocked.transition_to(&BeadState::Open);
        assert!(result.is_none());
    }

    #[test]
    fn transition_deferred_to_blocked_returns_none() {
        let bead = make_bead();
        let deferred = bead.transition_to(&BeadState::InProgress).unwrap();
        let deferred = deferred.transition_to(&BeadState::Deferred).unwrap();
        let result = deferred.transition_to(&BeadState::Blocked);
        assert!(result.is_none());
    }

    #[test]
    fn transition_deferred_to_open_returns_none() {
        let bead = make_bead();
        let deferred = bead.transition_to(&BeadState::InProgress).unwrap();
        let deferred = deferred.transition_to(&BeadState::Deferred).unwrap();
        let result = deferred.transition_to(&BeadState::Open);
        assert!(result.is_none());
    }

    // ── can_transition_to same-state ────────────────────────────────────────

    #[test]
    fn can_transition_in_progress_to_in_progress() {
        let bead = make_bead().start();
        assert!(bead.can_transition_to(&BeadState::InProgress));
    }

    #[test]
    fn can_transition_blocked_to_blocked() {
        let bead = make_bead().start().block();
        assert!(bead.can_transition_to(&BeadState::Blocked));
    }

    #[test]
    fn can_transition_deferred_to_deferred() {
        let bead = make_bead().start().defer();
        assert!(bead.can_transition_to(&BeadState::Deferred));
    }

    #[test]
    fn cannot_transition_blocked_to_open() {
        let bead = make_bead().start().block();
        assert!(!bead.can_transition_to(&BeadState::Open));
    }

    #[test]
    fn cannot_transition_in_progress_to_open() {
        let bead = make_bead().start();
        assert!(!bead.can_transition_to(&BeadState::Open));
    }

    // ── Builder chaining ────────────────────────────────────────────────────

    #[test]
    fn builder_chain_all_setters() {
        let bead = make_bead()
            .with_priority(Priority::P2)
            .with_type(BeadType::Task)
            .with_assignee("dave")
            .with_parent(BeadId::new("parent-x").unwrap())
            .add_dependency(BeadId::new("dep-a").unwrap())
            .add_dependency(BeadId::new("dep-b").unwrap())
            .add_blocker(BeadId::new("blk-a").unwrap())
            .with_labels(Labels::new().with("tag1"));
        assert_eq!(bead.priority(), Some(&Priority::P2));
        assert_eq!(bead.bead_type(), Some(&BeadType::Task));
        assert_eq!(bead.assignee(), Some("dave"));
        assert!(bead.parent().is_some());
        assert_eq!(bead.depends_on().len(), 2);
        assert_eq!(bead.blocked_by().len(), 1);
        assert!(bead.is_blocked());
        assert!(bead.labels().contains("tag1"));
    }

    // ── Debug formatting ────────────────────────────────────────────────────

    #[test]
    fn bead_is_debug() {
        let bead = make_bead();
        let debug = format!("{bead:?}");
        assert!(debug.contains("test-1"));
        assert!(debug.contains("Test Bead"));
    }

    // ── Proptests ────────────────────────────────────────────────────────────

    use proptest::proptest;

    proptest! {
        #[test]
        fn bead_created_preserves_id(ref id in "[a-zA-Z0-9_-]{1,50}") {
            let bead = Bead::<Open>::create(
                BeadId::new(id.as_str()).unwrap(),
                BeadTitle::new("Test").unwrap(),
                None,
            );
            assert_eq!(bead.id().as_str(), id.as_str());
        }

        #[test]
        fn bead_created_preserves_title(ref title in "[a-zA-Z]{1,100}") {
            let bead = Bead::<Open>::create(
                BeadId::new("proptest-bead").unwrap(),
                BeadTitle::new(title.as_str()).unwrap(),
                None,
            );
            assert_eq!(bead.title().as_str(), title.as_str());
        }

        #[test]
        fn priority_ordering_is_consistent(a_val in 0u8..=4, b_val in 0u8..=4) {
            let pa = Priority::from_value(a_val);
            let pb = Priority::from_value(b_val);
            let bead_a = make_bead().with_priority(pa);
            let bead_b = make_bead().with_priority(pb);
            // Priority on bead preserves ordering
            assert_eq!(bead_a.priority().cmp(&bead_b.priority()), pa.cmp(&pb));
        }

        #[test]
        fn state_transitions_preserve_data(ref title in "[a-zA-Z]{1,50}") {
            let bead = Bead::<Open>::create(
                BeadId::new("state-proptest").unwrap(),
                BeadTitle::new(title.as_str()).unwrap(),
                Some(BeadDescription::new("desc").unwrap()),
            )
            .with_priority(Priority::P2)
            .with_type(BeadType::Feature)
            .with_assignee("tester");

            let in_progress: Bead<InProgress> = bead.start();
            assert_eq!(in_progress.title().as_str(), title.as_str());
            assert_eq!(in_progress.priority(), Some(&Priority::P2));
            assert_eq!(in_progress.bead_type(), Some(&BeadType::Feature));
            assert_eq!(in_progress.assignee(), Some("tester"));
        }

        #[test]
        fn serde_roundtrip_with_various_priorities(prio_val in 0u8..=4) {
            let prio = Priority::from_value(prio_val);
            let bead = make_bead().with_priority(prio);
            let json = serde_json::to_string(&bead).unwrap();
            let parsed: Bead = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.priority(), Some(&prio));
        }

        #[test]
        fn serde_roundtrip_with_various_types(type_seed in 0u8..=4) {
            let bt = match type_seed {
                0 => BeadType::Bug,
                1 => BeadType::Feature,
                2 => BeadType::Task,
                3 => BeadType::Epic,
                _ => BeadType::Chore,
            };
            let bead = make_bead().with_type(bt.clone());
            let json = serde_json::to_string(&bead).unwrap();
            let parsed: Bead = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.bead_type(), Some(&bt));
        }

        #[test]
        fn cannot_transition_from_closed_any_target(target_seed in 0u8..=4) {
            let closed = make_bead().start().close();
            let target = match target_seed {
                0 => BeadState::Open,
                1 => BeadState::InProgress,
                2 => BeadState::Blocked,
                3 => BeadState::Deferred,
                _ => BeadState::Closed { closed_at: Utc::now() },
            };
            assert!(!closed.can_transition_to(&target));
            assert!(closed.transition_to(&target).is_none());
        }
    }

    // ── Exhaustive invalid transition tests (ha-29t2) ─────────────────────────

    /// Helper: build a bead in the given state via transition_to chaining.
    /// Returns Bead<Open> at the type level, but with the correct runtime bead_state.
    fn bead_in_state(state: &BeadState) -> Bead {
        let bead = make_bead();
        match state {
            BeadState::Open => bead,
            BeadState::InProgress => bead.transition_to(&BeadState::InProgress).unwrap(),
            BeadState::Blocked => {
                let ip = bead.transition_to(&BeadState::InProgress).unwrap();
                ip.transition_to(&BeadState::Blocked).unwrap()
            }
            BeadState::Deferred => {
                let ip = bead.transition_to(&BeadState::InProgress).unwrap();
                ip.transition_to(&BeadState::Deferred).unwrap()
            }
            BeadState::Closed { .. } => {
                let ip = bead.transition_to(&BeadState::InProgress).unwrap();
                ip.transition_to(&BeadState::Closed {
                    closed_at: Utc::now(),
                })
                .unwrap()
            }
        }
    }

    /// All five BeadState variants for iterating.
    fn all_states() -> Vec<BeadState> {
        vec![
            BeadState::Open,
            BeadState::InProgress,
            BeadState::Blocked,
            BeadState::Deferred,
            BeadState::Closed {
                closed_at: Utc::now(),
            },
        ]
    }

    /// Returns true if the (from, to) transition is valid per the state machine.
    fn is_valid_transition(from: &BeadState, to: &BeadState) -> bool {
        // Same-state is always valid (no-op), EXCEPT Closed which is terminal
        if from == to {
            return !matches!(from, BeadState::Closed { .. });
        }
        match (from, to) {
            // Open → InProgress only
            (BeadState::Open, BeadState::InProgress) => true,
            // InProgress → Blocked | Deferred | Closed
            (BeadState::InProgress, BeadState::Blocked) => true,
            (BeadState::InProgress, BeadState::Deferred) => true,
            (BeadState::InProgress, BeadState::Closed { .. }) => true,
            // Blocked → InProgress | Deferred | Closed
            (BeadState::Blocked, BeadState::InProgress) => true,
            (BeadState::Blocked, BeadState::Deferred) => true,
            (BeadState::Blocked, BeadState::Closed { .. }) => true,
            // Deferred → InProgress | Closed
            (BeadState::Deferred, BeadState::InProgress) => true,
            (BeadState::Deferred, BeadState::Closed { .. }) => true,
            // Everything else is invalid
            _ => false,
        }
    }

    #[test]
    fn exhaustive_transition_matrix_can_transition_to() {
        for from in &all_states() {
            for to in &all_states() {
                let bead = bead_in_state(from);
                let expected = is_valid_transition(from, to);
                let actual = bead.can_transition_to(to);
                assert_eq!(
                    actual, expected,
                    "can_transition_to({from:?}, {to:?}): expected {expected}, got {actual}"
                );
            }
        }
    }

    #[test]
    fn exhaustive_transition_matrix_transition_to_returns_none_for_invalid() {
        for from in &all_states() {
            for to in &all_states() {
                if is_valid_transition(from, to) {
                    continue;
                }
                let bead = bead_in_state(from);
                let result = bead.transition_to(to);
                assert!(
                    result.is_none(),
                    "transition_to({from:?}, {to:?}) should return None, got Some"
                );
            }
        }
    }

    #[test]
    fn invalid_transition_preserves_original_state() {
        // After an invalid transition attempt, the original bead's state must be unchanged.
        for from in &all_states() {
            for to in &all_states() {
                if is_valid_transition(from, to) {
                    continue;
                }
                let bead = bead_in_state(from);
                let original_state = bead.state().clone();
                let _ = bead.transition_to(to);
                assert_eq!(
                    bead.state(),
                    original_state,
                    "state changed after invalid transition {from:?} → {to:?}"
                );
            }
        }
    }

    // ── Specific invalid paths (each gets its own named test) ─────────────────

    #[test]
    fn invalid_open_to_blocked_returns_none() {
        let bead = make_bead();
        assert!(!bead.can_transition_to(&BeadState::Blocked));
        assert!(bead.transition_to(&BeadState::Blocked).is_none());
        assert_eq!(bead.state(), BeadState::Open);
    }

    #[test]
    fn invalid_open_to_deferred_returns_none() {
        let bead = make_bead();
        assert!(!bead.can_transition_to(&BeadState::Deferred));
        assert!(bead.transition_to(&BeadState::Deferred).is_none());
        assert_eq!(bead.state(), BeadState::Open);
    }

    #[test]
    fn invalid_open_to_closed_returns_none() {
        let bead = make_bead();
        let closed_target = BeadState::Closed {
            closed_at: Utc::now(),
        };
        assert!(!bead.can_transition_to(&closed_target));
        assert!(bead.transition_to(&closed_target).is_none());
        assert_eq!(bead.state(), BeadState::Open);
    }

    #[test]
    fn invalid_in_progress_to_open_returns_none() {
        let bead = make_bead().start();
        assert!(!bead.can_transition_to(&BeadState::Open));
        assert!(bead.transition_to(&BeadState::Open).is_none());
        assert_eq!(bead.state(), BeadState::InProgress);
    }

    #[test]
    fn invalid_blocked_to_open_returns_none() {
        let bead = make_bead().start().block();
        assert!(!bead.can_transition_to(&BeadState::Open));
        assert!(bead.transition_to(&BeadState::Open).is_none());
        assert_eq!(bead.state(), BeadState::Blocked);
    }

    #[test]
    fn invalid_deferred_to_open_returns_none() {
        let bead = make_bead().start().defer();
        assert!(!bead.can_transition_to(&BeadState::Open));
        assert!(bead.transition_to(&BeadState::Open).is_none());
        assert_eq!(bead.state(), BeadState::Deferred);
    }

    #[test]
    fn invalid_deferred_to_blocked_returns_none() {
        let bead = make_bead().start().defer();
        assert!(!bead.can_transition_to(&BeadState::Blocked));
        assert!(bead.transition_to(&BeadState::Blocked).is_none());
        assert_eq!(bead.state(), BeadState::Deferred);
    }

    #[test]
    fn invalid_closed_to_open_returns_none() {
        let bead = make_bead().start().close();
        assert!(!bead.can_transition_to(&BeadState::Open));
        assert!(bead.transition_to(&BeadState::Open).is_none());
        assert!(bead.is_terminal());
    }

    #[test]
    fn invalid_closed_to_in_progress_returns_none() {
        let bead = make_bead().start().close();
        assert!(!bead.can_transition_to(&BeadState::InProgress));
        assert!(bead.transition_to(&BeadState::InProgress).is_none());
        assert!(bead.is_terminal());
    }

    #[test]
    fn invalid_closed_to_blocked_returns_none() {
        let bead = make_bead().start().close();
        assert!(!bead.can_transition_to(&BeadState::Blocked));
        assert!(bead.transition_to(&BeadState::Blocked).is_none());
        assert!(bead.is_terminal());
    }

    #[test]
    fn invalid_closed_to_deferred_returns_none() {
        let bead = make_bead().start().close();
        assert!(!bead.can_transition_to(&BeadState::Deferred));
        assert!(bead.transition_to(&BeadState::Deferred).is_none());
        assert!(bead.is_terminal());
    }

    #[test]
    fn invalid_closed_to_closed_returns_none() {
        // Closed is terminal — even re-closing is rejected
        let bead = make_bead().start().close();
        let closed_target = BeadState::Closed {
            closed_at: Utc::now(),
        };
        assert!(!bead.can_transition_to(&closed_target));
        assert!(bead.transition_to(&closed_target).is_none());
        assert!(bead.is_terminal());
    }

    #[test]
    fn closed_state_is_absolutely_terminal() {
        // Closed cannot transition to ANY state — exhaustively verify all five
        let bead = make_bead().start().close();
        for target in &all_states() {
            assert!(
                !bead.can_transition_to(target),
                "Closed should not be able to transition to {target:?}"
            );
            assert!(
                bead.transition_to(target).is_none(),
                "Closed transition_to({target:?}) should return None"
            );
        }
        // Terminal flag stays true
        assert!(bead.is_terminal());
    }

    // ── Data integrity after invalid transitions ──────────────────────────────

    #[test]
    fn invalid_transition_preserves_all_fields() {
        let bead = make_bead()
            .with_priority(Priority::P1)
            .with_type(BeadType::Bug)
            .with_assignee("alice")
            .with_labels(Labels::new().with("urgent"));
        // Attempt invalid Open → Blocked
        assert!(bead.transition_to(&BeadState::Blocked).is_none());
        // All fields unchanged
        assert_eq!(bead.id().as_str(), "test-1");
        assert_eq!(bead.title().as_str(), "Test Bead");
        assert_eq!(bead.priority(), Some(&Priority::P1));
        assert_eq!(bead.bead_type(), Some(&BeadType::Bug));
        assert_eq!(bead.assignee(), Some("alice"));
        assert!(bead.labels().contains("urgent"));
        assert_eq!(bead.state(), BeadState::Open);
    }

    #[test]
    fn invalid_transition_from_in_progress_preserves_all_fields() {
        let bead = make_bead()
            .with_priority(Priority::P2)
            .with_type(BeadType::Feature)
            .with_assignee("bob")
            .start();
        // Attempt invalid InProgress → Open
        assert!(bead.transition_to(&BeadState::Open).is_none());
        assert_eq!(bead.priority(), Some(&Priority::P2));
        assert_eq!(bead.bead_type(), Some(&BeadType::Feature));
        assert_eq!(bead.assignee(), Some("bob"));
        assert_eq!(bead.state(), BeadState::InProgress);
    }

    #[test]
    fn invalid_transition_from_blocked_preserves_all_fields() {
        let bead = make_bead()
            .with_priority(Priority::P3)
            .with_type(BeadType::Task)
            .with_assignee("carol")
            .start()
            .block();
        // Attempt invalid Blocked → Open
        assert!(bead.transition_to(&BeadState::Open).is_none());
        assert_eq!(bead.priority(), Some(&Priority::P3));
        assert_eq!(bead.bead_type(), Some(&BeadType::Task));
        assert_eq!(bead.assignee(), Some("carol"));
        assert_eq!(bead.state(), BeadState::Blocked);
    }

    #[test]
    fn invalid_transition_from_deferred_preserves_all_fields() {
        let bead = make_bead()
            .with_priority(Priority::P4)
            .with_type(BeadType::Chore)
            .with_assignee("dave")
            .start()
            .defer();
        // Attempt invalid Deferred → Blocked
        assert!(bead.transition_to(&BeadState::Blocked).is_none());
        assert_eq!(bead.priority(), Some(&Priority::P4));
        assert_eq!(bead.bead_type(), Some(&BeadType::Chore));
        assert_eq!(bead.assignee(), Some("dave"));
        assert_eq!(bead.state(), BeadState::Deferred);
    }
}
