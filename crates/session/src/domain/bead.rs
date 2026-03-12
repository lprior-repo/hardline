//! Bead aggregate for atomic units of work.
//!
//! This module provides the Bead aggregate with full lifecycle management:
//! - States: Open → InProgress → Blocked → Deferred → Closed
//! - Invariants enforced via type system and runtime checks

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::workspace_state::WorkspaceState;
use crate::error::SessionError;

// Re-export for convenience
use std::result::Result;

/// Bead state enumeration.
///
/// Lifecycle: Open → InProgress → Blocked → Deferred → Closed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BeadState {
    /// Bead is open and available to be worked on
    Open,
    /// Bead is actively being worked on
    InProgress,
    /// Bead is blocked by dependencies
    Blocked,
    /// Bead has been deferred
    Deferred,
    /// Bead is closed/done (terminal state)
    Closed,
}

impl BeadState {
    /// All possible bead states
    pub const fn all() -> [Self; 5] {
        [
            Self::Open,
            Self::InProgress,
            Self::Blocked,
            Self::Deferred,
            Self::Closed,
        ]
    }

    /// Check if this state is terminal (no further transitions possible)
    #[must_use]
    pub fn is_terminal(self) -> bool {
        self == Self::Closed
    }

    /// Check if a transition from this state to target is valid
    /// State machine: Open → InProgress → Blocked/Deferred → Closed
    #[must_use]
    pub const fn can_transition_to(self, target: Self) -> bool {
        match (self, target) {
            // Open can go to InProgress
            (Self::Open, Self::InProgress) => true,
            // InProgress can go to Blocked, Deferred, or Closed
            (Self::InProgress, Self::Blocked) => true,
            (Self::InProgress, Self::Deferred) => true,
            (Self::InProgress, Self::Closed) => true,
            // Blocked can go back to InProgress, Deferred, or Closed
            (Self::Blocked, Self::InProgress) => true,
            (Self::Blocked, Self::Deferred) => true,
            (Self::Blocked, Self::Closed) => true,
            // Deferred can go back to InProgress or Closed
            (Self::Deferred, Self::InProgress) => true,
            (Self::Deferred, Self::Closed) => true,
            // Closed is terminal - cannot transition to any other state
            (Self::Closed, _) => false,
            // Self-loops are not allowed
            _ => false,
        }
    }

    /// Get all valid target states from this state
    #[must_use]
    pub fn valid_transitions(self) -> Vec<Self> {
        Self::all()
            .into_iter()
            .filter(|&target| self.can_transition_to(target))
            .collect()
    }
}

impl Default for BeadState {
    fn default() -> Self {
        Self::Open
    }
}

impl std::fmt::Display for BeadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Blocked => write!(f, "blocked"),
            Self::Deferred => write!(f, "deferred"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

/// Unique bead identifier.
///
/// # Invariants (I6)
/// - Must be non-empty
/// - Must be ≤100 characters
/// - Must contain only alphanumeric characters, hyphens, and underscores
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    pub const MAX_LENGTH: usize = 100;

    pub fn new(id: impl Into<String>) -> Result<Self, SessionError> {
        let id = id.into();
        if id.is_empty() {
            return Err(SessionError::InvalidBeadId("ID cannot be empty".into()));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidBeadId(format!(
                "ID exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(SessionError::InvalidBeadId(
                "ID must contain only alphanumeric characters, hyphens, and underscores".into(),
            ));
        }
        Ok(Self(id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated bead title.
///
/// # Invariants (I7)
/// - Must be non-empty
/// - Must be ≤200 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadTitle(String);

impl BeadTitle {
    pub const MAX_LENGTH: usize = 200;

    pub fn new(title: impl Into<String>) -> Result<Self, SessionError> {
        let title = title.into();
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(SessionError::InvalidBeadTitle(
                "Title cannot be empty".into(),
            ));
        }
        if trimmed.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidBeadTitle(format!(
                "Title exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BeadTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Optional bead description
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadDescription(Option<String>);

impl BeadDescription {
    pub fn new(description: impl Into<String>) -> Result<Self, SessionError> {
        let description = description.into();
        let trimmed = description.trim();
        if trimmed.is_empty() {
            return Ok(Self(None));
        }
        Ok(Self(Some(trimmed.to_string())))
    }

    #[must_use]
    pub fn as_option(&self) -> Option<&String> {
        self.0.as_ref()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.as_ref().map_or(true, |s| s.is_empty())
    }
}

impl std::fmt::Display for BeadDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(s) => write!(f, "{}", s),
            None => write!(f, ""),
        }
    }
}

/// Bead type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BeadType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
}

impl BeadType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Bug => "bug",
            Self::Feature => "feature",
            Self::Task => "task",
            Self::Epic => "epic",
            Self::Chore => "chore",
        }
    }
}

impl Default for BeadType {
    fn default() -> Self {
        Self::Task
    }
}

/// Priority level (0-4)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    pub fn new(priority: u8) -> Result<Self, SessionError> {
        if priority > 4 {
            return Err(SessionError::InvalidPriority(format!(
                "Priority must be 0-4, got {}",
                priority
            )));
        }
        Ok(Self(priority))
    }

    #[must_use]
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self(2) // Medium priority
    }
}

/// Bead aggregate representing an atomic unit of work.
///
/// # State Machine
/// - Open: Bead is available to be worked on (Q11: initial state after create)
/// - InProgress: Bead is actively being worked on
/// - Blocked: Bead has blockers
/// - Deferred: Bead has been deferred
/// - Closed: Bead is done (terminal state)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self.updated_at = Utc::now();
        self
    }

    /// Set the type of this bead
    pub fn with_type(mut self, bead_type: BeadType) -> Self {
        self.bead_type = bead_type;
        self.updated_at = Utc::now();
        self
    }

    /// Set the assignee of this bead
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self.updated_at = Utc::now();
        self
    }

    /// Set the parent bead
    pub fn with_parent(mut self, parent: BeadId) -> Self {
        self.parent = Some(parent);
        self.updated_at = Utc::now();
        self
    }

    /// Add a dependency (this bead depends on another).
    ///
    /// # Preconditions (P9)
    /// - depends_on must be non-empty
    pub fn add_dependency(mut self, depends_on: BeadId) -> Self {
        // I10: No self-references
        if depends_on != self.id {
            if !self.depends_on.contains(&depends_on) {
                self.depends_on.push(depends_on);
            }
        }
        self.updated_at = Utc::now();
        self
    }

    /// Add a blocker (this bead is blocked by another).
    ///
    /// # Preconditions (P10)
    /// - blocked_by must be non-empty
    pub fn add_blocker(mut self, blocked_by: BeadId) -> Self {
        // I9: No self-references
        if blocked_by != self.id {
            if !self.blocked_by.contains(&blocked_by) {
                self.blocked_by.push(blocked_by);
            }
        }
        self.updated_at = Utc::now();
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
        // Q15: Cannot transition from Closed to any other state
        if self.state == BeadState::Closed && new_state != BeadState::Closed {
            return Err(SessionError::InvalidTransition {
                from: WorkspaceState::Working,
                to: WorkspaceState::Working,
            });
        }

        // Q16: Can always transition to Closed
        if new_state == BeadState::Closed {
            let mut new_bead = self.clone();
            new_bead.state = new_state;
            new_bead.closed_at = Some(Utc::now());
            new_bead.updated_at = Utc::now();
            return Ok(new_bead);
        }

        // Validate the transition
        if !self.state.can_transition_to(new_state) {
            return Err(SessionError::InvalidTransition {
                from: WorkspaceState::Working,
                to: WorkspaceState::Working,
            });
        }

        let mut new_bead = self.clone();
        new_bead.state = new_state;
        new_bead.updated_at = Utc::now();
        Ok(new_bead)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bead_create_sets_open_state() {
        let id = BeadId::new("bd-123").unwrap();
        let title = BeadTitle::new("Test Bead").unwrap();
        let bead = Bead::create(id, title, None);

        assert_eq!(bead.state(), BeadState::Open);
        assert!(!bead.is_blocked());
    }

    #[test]
    fn bead_transition_to_in_progress() {
        let id = BeadId::new("bd-123").unwrap();
        let title = BeadTitle::new("Test Bead").unwrap();
        let bead = Bead::create(id, title, None);
        let in_progress = bead.transition(BeadState::InProgress).unwrap();

        assert_eq!(in_progress.state(), BeadState::InProgress);
    }

    #[test]
    fn bead_transition_to_closed_sets_closed_at() {
        let id = BeadId::new("bd-123").unwrap();
        let title = BeadTitle::new("Test Bead").unwrap();
        let bead = Bead::create(id, title, None);
        let closed = bead.transition(BeadState::Closed).unwrap();

        assert_eq!(closed.state(), BeadState::Closed);
        assert!(closed.closed_at().is_some());
    }

    #[test]
    fn bead_cannot_transition_from_closed() {
        let id = BeadId::new("bd-123").unwrap();
        let title = BeadTitle::new("Test Bead").unwrap();
        let bead = Bead::create(id, title, None);
        let closed = bead.transition(BeadState::Closed).unwrap();
        let result = closed.transition(BeadState::InProgress);

        assert!(result.is_err());
    }

    #[test]
    fn bead_add_dependency() {
        let id = BeadId::new("bd-123").unwrap();
        let title = BeadTitle::new("Test Bead").unwrap();
        let bead = Bead::create(id, title, None);
        let dep_id = BeadId::new("bd-456").unwrap();
        let with_dep = bead.add_dependency(dep_id);

        assert_eq!(with_dep.depends_on().len(), 1);
    }

    #[test]
    fn bead_add_blocker() {
        let id = BeadId::new("bd-123").unwrap();
        let title = BeadTitle::new("Test Bead").unwrap();
        let bead = Bead::create(id, title, None);
        let blocker_id = BeadId::new("bd-456").unwrap();
        let blocked = bead.add_blocker(blocker_id);

        assert!(blocked.is_blocked());
    }

    #[test]
    fn bead_invalid_id_empty() {
        let result = BeadId::new("");
        assert!(result.is_err());
    }

    #[test]
    fn bead_invalid_id_too_long() {
        let long_id = "a".repeat(101);
        let result = BeadId::new(long_id);
        assert!(result.is_err());
    }

    #[test]
    fn bead_invalid_id_invalid_chars() {
        let result = BeadId::new("bd-123!");
        assert!(result.is_err());
    }

    #[test]
    fn bead_title_empty_fails() {
        let result = BeadTitle::new("");
        assert!(result.is_err());
    }

    #[test]
    fn bead_title_too_long_fails() {
        let long_title = "a".repeat(201);
        let result = BeadTitle::new(long_title);
        assert!(result.is_err());
    }
}
