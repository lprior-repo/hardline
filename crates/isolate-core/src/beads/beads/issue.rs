//! Issue aggregate root.
//!
//! This module defines the `Issue` aggregate root which encapsulates
//! the domain logic for issue management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::domain::{
    Assignee, BlockedBy, DependsOn, Description, DomainError, IssueId, IssueState, IssueType,
    Labels, ParentId, Priority, Title,
};

// ============================================================================
// Issue Aggregate Root
// ============================================================================

/// An issue in the beads tracker.
///
/// This is the aggregate root for the Issue aggregate. All invariants
/// are enforced through the type system and constructor validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Unique identifier for this issue.
    pub id: IssueId,
    /// Title of the issue.
    pub title: Title,
    /// Current state of the issue.
    pub state: IssueState,
    /// Priority level.
    pub priority: Option<Priority>,
    /// Type classification.
    pub issue_type: Option<IssueType>,
    /// Detailed description.
    pub description: Option<Description>,
    /// Labels attached to this issue.
    pub labels: Labels,
    /// Assignee responsible for this issue.
    pub assignee: Option<Assignee>,
    /// Parent issue if this is a sub-issue.
    pub parent: Option<ParentId>,
    /// Issues that this issue depends on.
    pub depends_on: DependsOn,
    /// Issues that are blocking this issue.
    pub blocked_by: BlockedBy,
    /// When the issue was created.
    pub created_at: DateTime<Utc>,
    /// When the issue was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Issue {
    // ========================================================================
    // Constructors
    // ========================================================================

    /// Create a new issue with the given ID and title.
    ///
    /// The issue will be created in the `Open` state with the current timestamp.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if ID or title validation fails.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Result<Self, DomainError> {
        let id = IssueId::new(id)?;
        let title = Title::new(title)?;
        let now = Utc::now();

        Ok(Self {
            id,
            title,
            state: IssueState::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: Labels::empty(),
            assignee: None,
            parent: None,
            depends_on: DependsOn::empty(),
            blocked_by: BlockedBy::empty(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Create a new issue with a specific creation time (for testing/import).
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if ID or title validation fails.
    pub fn new_with_time(
        id: impl Into<String>,
        title: impl Into<String>,
        created_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let id = IssueId::new(id)?;
        let title = Title::new(title)?;

        Ok(Self {
            id,
            title,
            state: IssueState::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: Labels::empty(),
            assignee: None,
            parent: None,
            depends_on: DependsOn::empty(),
            blocked_by: BlockedBy::empty(),
            created_at,
            updated_at: created_at,
        })
    }

    // ========================================================================
    // State Transitions
    // ========================================================================

    /// Transition the issue to a new state.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidStateTransition` if the transition is invalid.
    pub fn transition_to(&mut self, new_state: IssueState) -> Result<(), DomainError> {
        self.state = self.state.transition_to(new_state)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Close the issue with the current timestamp.
    pub fn close(&mut self) {
        self.state = IssueState::Closed {
            closed_at: Utc::now(),
        };
        self.updated_at = Utc::now();
    }

    /// Close the issue with a specific timestamp.
    pub fn close_with_time(&mut self, closed_at: DateTime<Utc>) {
        self.state = IssueState::Closed { closed_at };
        self.updated_at = Utc::now();
    }

    /// Reopen a closed issue.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if the issue is not closed.
    pub fn reopen(&mut self) -> Result<(), DomainError> {
        if !self.state.is_closed() {
            return Err(DomainError::InvalidStateTransition {
                from: self.state,
                to: IssueState::Open,
            });
        }
        self.state = IssueState::Open;
        self.updated_at = Utc::now();
        Ok(())
    }

    // ========================================================================
    // Field Updates
    // ========================================================================

    /// Update the title.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if title validation fails.
    pub fn update_title(&mut self, title: impl Into<String>) -> Result<(), DomainError> {
        self.title = Title::new(title)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update the description.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if description validation fails.
    pub fn update_description(
        &mut self,
        description: impl Into<String>,
    ) -> Result<(), DomainError> {
        self.description = Some(Description::new(description)?);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Clear the description.
    pub fn clear_description(&mut self) {
        self.description = None;
        self.updated_at = Utc::now();
    }

    /// Set the priority.
    pub fn set_priority(&mut self, priority: Priority) {
        self.priority = Some(priority);
        self.updated_at = Utc::now();
    }

    /// Clear the priority.
    pub fn clear_priority(&mut self) {
        self.priority = None;
        self.updated_at = Utc::now();
    }

    /// Set the issue type.
    pub fn set_issue_type(&mut self, issue_type: IssueType) {
        self.issue_type = Some(issue_type);
        self.updated_at = Utc::now();
    }

    /// Clear the issue type.
    pub fn clear_issue_type(&mut self) {
        self.issue_type = None;
        self.updated_at = Utc::now();
    }

    /// Set the assignee.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if assignee validation fails.
    pub fn set_assignee(&mut self, assignee: impl Into<String>) -> Result<(), DomainError> {
        self.assignee = Some(Assignee::new(assignee)?);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Clear the assignee.
    pub fn clear_assignee(&mut self) {
        self.assignee = None;
        self.updated_at = Utc::now();
    }

    /// Set the parent issue.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if parent ID validation fails.
    pub fn set_parent(&mut self, parent: impl Into<String>) -> Result<(), DomainError> {
        self.parent = Some(ParentId::new(parent)?);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Clear the parent.
    pub fn clear_parent(&mut self) {
        self.parent = None;
        self.updated_at = Utc::now();
    }

    /// Set the labels.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if label validation fails.
    pub fn set_labels(&mut self, labels: Vec<String>) -> Result<(), DomainError> {
        self.labels = Labels::new(labels)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Clear all labels.
    pub fn clear_labels(&mut self) {
        self.labels = Labels::empty();
        self.updated_at = Utc::now();
    }

    /// Add a single label.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if adding the label would exceed limits.
    pub fn add_label(&mut self, label: String) -> Result<(), DomainError> {
        self.labels = self.labels.add(label)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Remove a label if it exists.
    pub fn remove_label(&mut self, label: &str) {
        self.labels = self.labels.remove(label);
        self.updated_at = Utc::now();
    }

    /// Set the dependencies.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if dependency validation fails.
    pub fn set_depends_on(&mut self, dependencies: Vec<String>) -> Result<(), DomainError> {
        self.depends_on = DependsOn::new(dependencies)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Clear all dependencies.
    pub fn clear_depends_on(&mut self) {
        self.depends_on = DependsOn::empty();
        self.updated_at = Utc::now();
    }

    /// Set the blockers.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if blocker validation fails.
    pub fn set_blocked_by(&mut self, blockers: Vec<String>) -> Result<(), DomainError> {
        self.blocked_by = BlockedBy::new(blockers)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Clear all blockers.
    pub fn clear_blocked_by(&mut self) {
        self.blocked_by = BlockedBy::empty();
        self.updated_at = Utc::now();
    }

    // ========================================================================
    // Query Methods
    // ========================================================================

    /// Check if the issue is currently blocked.
    #[must_use]
    pub const fn is_blocked(&self) -> bool {
        self.state.is_blocked() || !self.blocked_by.is_empty()
    }

    /// Check if the issue is active (open or in progress).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Check if the issue is closed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.state.is_closed()
    }

    /// Check if the issue has a parent.
    #[must_use]
    pub const fn has_parent(&self) -> bool {
        self.parent.is_some()
    }

    /// Get the closed timestamp if closed.
    #[must_use]
    pub const fn closed_at(&self) -> Option<DateTime<Utc>> {
        self.state.closed_at()
    }
}

// ============================================================================
// Builder Pattern for Issue Construction
// ============================================================================

/// Builder for creating or updating issues.
#[derive(Debug, Clone)]
pub struct IssueBuilder {
    id: Option<String>,
    title: Option<String>,
    state: Option<IssueState>,
    priority: Option<Priority>,
    issue_type: Option<IssueType>,
    description: Option<String>,
    labels: Option<Vec<String>>,
    assignee: Option<String>,
    parent: Option<String>,
    depends_on: Option<Vec<String>>,
    blocked_by: Option<Vec<String>>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

impl Default for IssueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IssueBuilder {
    /// Create a new builder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            id: None,
            title: None,
            state: None,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Set the issue ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the state.
    #[must_use]
    pub const fn state(mut self, state: IssueState) -> Self {
        self.state = Some(state);
        self
    }

    /// Set the priority.
    #[must_use]
    pub const fn priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set the issue type.
    #[must_use]
    pub const fn issue_type(mut self, issue_type: IssueType) -> Self {
        self.issue_type = Some(issue_type);
        self
    }

    /// Set the description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the labels.
    #[must_use]
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Set the assignee.
    #[must_use]
    pub fn assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    /// Set the parent.
    #[must_use]
    pub fn parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    /// Set the dependencies.
    #[must_use]
    pub fn depends_on(mut self, depends_on: Vec<String>) -> Self {
        self.depends_on = Some(depends_on);
        self
    }

    /// Set the blockers.
    #[must_use]
    pub fn blocked_by(mut self, blocked_by: Vec<String>) -> Self {
        self.blocked_by = Some(blocked_by);
        self
    }

    /// Set the creation time.
    #[must_use]
    pub const fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the update time.
    #[must_use]
    pub const fn updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Build the issue.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if validation fails.
    ///
    /// # Panics
    ///
    /// Panics if ID or title are not set (useful for testing with known-good data).
    pub fn build(self) -> Result<Issue, DomainError> {
        let id = self.id.ok_or(DomainError::EmptyId)?;
        let title = self.title.ok_or(DomainError::EmptyTitle)?;
        let now = self.created_at.unwrap_or_else(Utc::now);
        let updated = self.updated_at.unwrap_or(now);

        let issue = Issue {
            id: IssueId::new(id)?,
            title: Title::new(title)?,
            state: self.state.unwrap_or(IssueState::Open),
            priority: self.priority,
            issue_type: self.issue_type,
            description: self.description.map(Description::new).transpose()?,
            labels: self.labels.map_or(Ok(Labels::empty()), Labels::new)?,
            assignee: self.assignee.map(Assignee::new).transpose()?,
            parent: self.parent.map(ParentId::new).transpose()?,
            depends_on: self
                .depends_on
                .map_or(Ok(DependsOn::empty()), DependsOn::new)?,
            blocked_by: self
                .blocked_by
                .map_or(Ok(BlockedBy::empty()), BlockedBy::new)?,
            created_at: now,
            updated_at: updated,
        };

        Ok(issue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_creation() {
        let issue = Issue::new("test-1", "Test Issue").unwrap();
        assert_eq!(issue.id.as_str(), "test-1");
        assert_eq!(issue.title.as_str(), "Test Issue");
        assert!(issue.is_active());
        assert!(!issue.is_closed());
    }

    #[test]
    fn test_issue_close() {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        assert!(!issue.is_closed());

        issue.close();
        assert!(issue.is_closed());
        assert!(issue.closed_at().is_some());
    }

    #[test]
    fn test_issue_reopen() {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.close();
        assert!(issue.is_closed());

        issue.reopen().unwrap();
        assert!(!issue.is_closed());
        assert!(issue.is_active());
    }

    #[test]
    fn test_issue_blocked() {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.set_blocked_by(vec!["blocker-1".to_string()]).unwrap();
        assert!(issue.is_blocked());
    }

    #[test]
    fn test_invalid_id() {
        let result = Issue::new("", "Test Issue");
        assert!(matches!(result, Err(DomainError::EmptyId)));
    }

    #[test]
    fn test_invalid_title() {
        let result = Issue::new("test-1", "");
        assert!(matches!(result, Err(DomainError::EmptyTitle)));
    }
}
