use chrono::{DateTime, Utc};

use crate::beads::domain::{
    Assignee, BlockedBy, DependsOn, Description, DomainError, IssueId, IssueState, IssueType,
    Labels, ParentId, Priority, Title,
};

use crate::beads::issue_data::Issue;

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
