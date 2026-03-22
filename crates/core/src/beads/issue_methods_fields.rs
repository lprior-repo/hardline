//! Issue aggregate root - field methods.
//!
//! This module contains field update and query methods.

use chrono::Utc;

use crate::beads::domain::DomainError;
use crate::beads::issue::Issue;

impl Issue {
    // ========================================================================
    // Field Updates
    // ========================================================================

    /// Update the title.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if title validation fails.
    pub fn update_title(&mut self, title: impl Into<String>) -> Result<(), DomainError> {
        self.title = crate::beads::domain::Title::new(title)?;
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
        self.description = Some(crate::beads::domain::Description::new(description)?);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Clear the description.
    pub fn clear_description(&mut self) {
        self.description = None;
        self.updated_at = Utc::now();
    }

    /// Set the priority.
    pub fn set_priority(&mut self, priority: crate::beads::domain::Priority) {
        self.priority = Some(priority);
        self.updated_at = Utc::now();
    }

    /// Clear the priority.
    pub fn clear_priority(&mut self) {
        self.priority = None;
        self.updated_at = Utc::now();
    }

    /// Set the issue type.
    pub fn set_issue_type(&mut self, issue_type: crate::beads::domain::IssueType) {
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
        self.assignee = Some(crate::beads::domain::Assignee::new(assignee)?);
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
        self.parent = Some(crate::beads::domain::ParentId::new(parent)?);
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
        self.labels = crate::beads::domain::Labels::new(labels)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Clear all labels.
    pub fn clear_labels(&mut self) {
        self.labels = crate::beads::domain::Labels::empty();
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
        self.depends_on = crate::beads::domain::DependsOn::new(dependencies)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Clear all dependencies.
    pub fn clear_depends_on(&mut self) {
        self.depends_on = crate::beads::domain::DependsOn::empty();
        self.updated_at = Utc::now();
    }

    /// Set the blockers.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if blocker validation fails.
    pub fn set_blocked_by(&mut self, blockers: Vec<String>) -> Result<(), DomainError> {
        self.blocked_by = crate::beads::domain::BlockedBy::new(blockers)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Clear all blockers.
    pub fn clear_blocked_by(&mut self) {
        self.blocked_by = crate::beads::domain::BlockedBy::empty();
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
    pub const fn closed_at(&self) -> Option<chrono::DateTime<Utc>> {
        self.state.closed_at()
    }
}
