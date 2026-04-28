use chrono::{DateTime, Utc};

use crate::beads::{
    domain::{
        Assignee, BlockedBy, DependsOn, Description, DomainError, IssueId, IssueState, IssueType,
        Labels, ParentId, Priority, Title,
    },
    issue_data::Issue,
};

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
                from: self.state.to_string(),
                to: IssueState::Open.to_string(),
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
}
