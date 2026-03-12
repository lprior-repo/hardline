#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use thiserror::Error;

/// Errors that can occur when working with beads.
#[derive(Debug, Error)]
pub enum BeadsError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Issue not found: {0}")]
    NotFound(String),

    #[error("Invalid filter: {0}")]
    InvalidFilter(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("Path error: {0}")]
    PathError(String),

    #[error("Insert failed: {0}")]
    InsertFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Duplicate ID: {0}")]
    DuplicateId(String),
}

/// Status of an issue in the beads tracker.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Serialize,
    Deserialize,
    Hash,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    #[strum(to_string = "open")]
    Open,

    #[strum(to_string = "in_progress")]
    InProgress,

    #[strum(to_string = "blocked")]
    Blocked,

    #[strum(to_string = "deferred")]
    Deferred,

    #[strum(to_string = "closed")]
    Closed,
}

/// Type classification for issues.
#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueType {
    #[strum(to_string = "bug")]
    Bug,

    #[strum(to_string = "feature")]
    Feature,

    #[strum(to_string = "task")]
    Task,

    #[strum(to_string = "epic")]
    Epic,

    #[strum(to_string = "chore")]
    Chore,

    #[strum(to_string = "merge-request")]
    MergeRequest,
}

/// Priority level for issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
    P4,
}

impl Priority {
    #[must_use]
    pub const fn from_u32(n: u32) -> Option<Self> {
        match n {
            0 => Some(Self::P0),
            1 => Some(Self::P1),
            2 => Some(Self::P2),
            3 => Some(Self::P3),
            4 => Some(Self::P4),
            _ => None,
        }
    }

    #[must_use]
    pub const fn to_u32(self) -> u32 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
            Self::P4 => 4,
        }
    }
}

/// An issue in the beads tracker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadIssue {
    pub id: String,
    pub title: String,
    pub status: IssueStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<IssueType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_by: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,
}

impl BeadIssue {
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        matches!(self.status, IssueStatus::Blocked)
            || self.blocked_by.as_ref().is_some_and(|v| !v.is_empty())
    }

    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.status, IssueStatus::Open | IssueStatus::InProgress)
    }
}

/// Summary statistics for a collection of issues.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeadsSummary {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub deferred: usize,
    pub closed: usize,
}

impl BeadsSummary {
    #[must_use]
    #[allow(clippy::arithmetic_side_effects)]
    pub fn from_issues(issues: &[BeadIssue]) -> Self {
        issues.iter().fold(Self::default(), |mut acc, issue| {
            acc.total += 1;
            match issue.status {
                IssueStatus::Open => acc.open += 1,
                IssueStatus::InProgress => acc.in_progress += 1,
                IssueStatus::Blocked => acc.blocked += 1,
                IssueStatus::Deferred => acc.deferred += 1,
                IssueStatus::Closed => acc.closed += 1,
            }
            acc
        })
    }

    #[must_use]
    #[allow(clippy::arithmetic_side_effects)]
    pub const fn active(&self) -> usize {
        self.open + self.in_progress
    }

    #[must_use]
    pub const fn has_blockers(&self) -> bool {
        self.blocked > 0
    }
}

/// Filter criteria for querying issues.
#[derive(Debug, Clone, Default)]
pub struct BeadFilter {
    pub status: Vec<IssueStatus>,
    pub issue_type: Vec<IssueType>,
    pub priority_min: Option<Priority>,
    pub priority_max: Option<Priority>,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub parent: Option<String>,
    pub has_parent: bool,
    pub blocked_only: bool,
    pub search_text: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl BeadFilter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_status(mut self, status: IssueStatus) -> Self {
        self.status.push(status);
        self
    }

    #[must_use]
    pub fn with_statuses(mut self, statuses: impl IntoIterator<Item = IssueStatus>) -> Self {
        self.status.extend(statuses);
        self
    }

    #[must_use]
    pub fn with_type(mut self, issue_type: IssueType) -> Self {
        self.issue_type.push(issue_type);
        self
    }

    #[must_use]
    pub const fn with_priority_range(mut self, min: Priority, max: Priority) -> Self {
        self.priority_min = Some(min);
        self.priority_max = Some(max);
        self
    }

    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    #[must_use]
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    #[must_use]
    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    #[must_use]
    pub const fn blocked_only(mut self) -> Self {
        self.blocked_only = true;
        self
    }

    #[must_use]
    pub fn with_search(mut self, text: impl Into<String>) -> Self {
        self.search_text = Some(text.into());
        self
    }

    #[must_use]
    pub const fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    #[must_use]
    pub const fn offset(mut self, n: usize) -> Self {
        self.offset = Some(n);
        self
    }
}

/// Sort field for issues.
#[derive(Debug, Clone, Copy, EnumString, Display, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum BeadSort {
    #[strum(to_string = "priority")]
    Priority,

    #[strum(to_string = "created")]
    Created,

    #[strum(to_string = "updated")]
    Updated,

    #[strum(to_string = "closed")]
    Closed,

    #[strum(to_string = "status")]
    Status,

    #[strum(to_string = "title")]
    Title,

    #[strum(to_string = "id")]
    Id,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, EnumString, Display, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum SortDirection {
    #[strum(to_string = "asc")]
    Asc,

    #[strum(to_string = "desc")]
    Desc,
}

/// Complete query parameters for issues.
#[derive(Debug, Clone)]
pub struct BeadQuery {
    pub filter: BeadFilter,
    pub sort: BeadSort,
    pub direction: SortDirection,
    pub include_closed: bool,
}

impl Default for BeadQuery {
    fn default() -> Self {
        Self {
            filter: BeadFilter::new(),
            sort: BeadSort::Priority,
            direction: SortDirection::Desc,
            include_closed: false,
        }
    }
}

impl BeadQuery {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn filter(mut self, filter: BeadFilter) -> Self {
        self.filter = filter;
        self
    }

    #[must_use]
    pub const fn sort_by(mut self, sort: BeadSort) -> Self {
        self.sort = sort;
        self
    }

    #[must_use]
    pub const fn direction(mut self, direction: SortDirection) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub const fn include_closed(mut self, include: bool) -> Self {
        self.include_closed = include;
        self
    }
}
