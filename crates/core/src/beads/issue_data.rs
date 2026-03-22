//! Issue aggregate root.
//!
//! This module defines the `Issue` aggregate root which encapsulates
//! the domain logic for issue management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::beads::domain::{
    Assignee, BlockedBy, DependsOn, Description, IssueId, IssueState, IssueType,
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
