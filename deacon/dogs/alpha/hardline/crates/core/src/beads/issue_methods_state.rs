//! Issue aggregate root - state methods.
//!
//! This module contains constructors and state transition methods.

use chrono::Utc;

use crate::beads::domain::{DomainError, IssueState};
use crate::beads::issue::Issue;

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
        let id = crate::beads::domain::IssueId::new(id)?;
        let title = crate::beads::domain::Title::new(title)?;
        let now = Utc::now();

        Ok(Self {
            id,
            title,
            state: IssueState::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: crate::beads::domain::Labels::empty(),
            assignee: None,
            parent: None,
            depends_on: crate::beads::domain::DependsOn::empty(),
            blocked_by: crate::beads::domain::BlockedBy::empty(),
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
        created_at: chrono::DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let id = crate::beads::domain::IssueId::new(id)?;
        let title = crate::beads::domain::Title::new(title)?;

        Ok(Self {
            id,
            title,
            state: IssueState::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: crate::beads::domain::Labels::empty(),
            assignee: None,
            parent: None,
            depends_on: crate::beads::domain::DependsOn::empty(),
            blocked_by: crate::beads::domain::BlockedBy::empty(),
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
    pub fn close_with_time(&mut self, closed_at: chrono::DateTime<Utc>) {
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
}
