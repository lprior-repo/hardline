//! Issue output types
//!
//! Provides issue detection and reporting for problems encountered
//! during AI control plane operations.

use serde::{Deserialize, Serialize};

use crate::output_jsonl::domain_types::{IssueId, IssueScope, IssueTitle, SessionName};
use crate::output_jsonl::errors::OutputLineError;

/// Issue output line for reporting problems or validation errors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    pub id: IssueId,
    pub title: IssueTitle,
    pub kind: IssueKind,
    pub severity: IssueSeverity,
    #[serde(skip_serializing_if = "IssueScope::is_standalone")]
    #[serde(default = "IssueScope::standalone")]
    pub scope: IssueScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Category of issue detected.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    Validation,
    StateConflict,
    ResourceNotFound,
    PermissionDenied,
    Timeout,
    Configuration,
    External,
}

/// Severity level of the issue.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Hint,
    Warning,
    Error,
    Critical,
}

impl Issue {
    /// Create a new issue output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyTitle` if `title` is blank.
    pub const fn new(
        id: IssueId,
        title: IssueTitle,
        kind: IssueKind,
        severity: IssueSeverity,
    ) -> Result<Self, OutputLineError> {
        Ok(Self {
            id,
            title,
            kind,
            severity,
            scope: IssueScope::Standalone,
            suggestion: None,
        })
    }

    #[must_use]
    pub fn with_session(self, session: SessionName) -> Self {
        Self {
            scope: IssueScope::InSession { session },
            ..self
        }
    }

    #[must_use]
    pub fn with_suggestion(self, suggestion: String) -> Self {
        Self {
            suggestion: Some(suggestion),
            ..self
        }
    }
}
