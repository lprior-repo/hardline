// //! Issue builder
//! Builder for `Issue` with fluent API.

use crate::output_jsonl::{
    domain_types::{IssueId, IssueScope, IssueTitle},
    Issue, IssueKind as OutputIssueKind, IssueSeverity,
};

/// Builder for [Issue] with fluent API
///
/// # Required Fields
/// - `id`: Issue identifier
/// - `title`: Issue title
/// - `kind`: Issue kind
/// - `severity`: Issue severity
///
/// # Optional Fields
/// - `scope`: Issue scope (defaults to Standalone)
/// - `suggestion`: Suggested fix
#[derive(Debug, Clone)]
pub struct IssueBuilder {
    // Required fields
    id: Option<IssueId>,
    title: Option<IssueTitle>,
    kind: Option<IssueKind>,
    severity: Option<IssueSeverity>,

    // Optional fields
    scope: Option<IssueScope>,
    suggestion: Option<String>,
}

/// Issue kind classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    Validation,
    StateConflict,
    ResourceNotFound,
    PermissionDenied,
    Timeout,
    Configuration,
    External,
}

impl Default for IssueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IssueBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            id: None,
            title: None,
            kind: None,
            severity: None,
            scope: None,
            suggestion: None,
        }
    }

    /// Set the issue ID (required)
    #[must_use]
    pub fn id(mut self, id: IssueId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the issue title (required)
    #[must_use]
    pub fn title(mut self, title: IssueTitle) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the issue kind (required)
    #[must_use]
    pub const fn kind(mut self, kind: IssueKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Set the issue severity (required)
    #[must_use]
    pub const fn severity(mut self, severity: IssueSeverity) -> Self {
        self.severity = Some(severity);
        self
    }

    /// Set the issue scope (optional)
    #[must_use]
    pub fn scope(mut self, scope: IssueScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Set the suggestion (optional)
    #[must_use]
    pub fn suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// Build the Issue
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<Issue, super::errors::BuilderError> {
        let id = self
            .id
            .ok_or(super::errors::BuilderError::MissingRequired { field: "id" })?;
        let title = self
            .title
            .ok_or(super::errors::BuilderError::MissingRequired { field: "title" })?;
        let kind = self
            .kind
            .ok_or(super::errors::BuilderError::MissingRequired { field: "kind" })?;
        let severity = self
            .severity
            .ok_or(super::errors::BuilderError::MissingRequired { field: "severity" })?;

        Ok(Issue {
            id,
            title,
            kind: convert_issue_kind(kind),
            severity,
            scope: self.scope.unwrap_or(IssueScope::Standalone),
            suggestion: self.suggestion,
        })
    }
}

const fn convert_issue_kind(kind: IssueKind) -> OutputIssueKind {
    match kind {
        IssueKind::Validation => OutputIssueKind::Validation,
        IssueKind::StateConflict => OutputIssueKind::StateConflict,
        IssueKind::ResourceNotFound => OutputIssueKind::ResourceNotFound,
        IssueKind::PermissionDenied => OutputIssueKind::PermissionDenied,
        IssueKind::Timeout => OutputIssueKind::Timeout,
        IssueKind::Configuration => OutputIssueKind::Configuration,
        IssueKind::External => OutputIssueKind::External,
    }
}
