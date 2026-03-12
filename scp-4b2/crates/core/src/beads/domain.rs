#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Domain types for the beads issue tracker.
//!
//! This module implements Domain-Driven Design principles:
//! - Semantic newtypes prevent primitive obsession
//! - Enum-based state makes illegal states unrepresentable
//! - Parse at boundaries, validate once
//! - Pure functional core, side effects at boundaries
//!
//! # Architecture
//!
//! - **Core types**: `IssueId`, `Title`, `Description` - validated newtypes
//! - **State types**: `IssueState` - closed state includes timestamp inline
//! - **Domain errors**: Structured errors with `thiserror`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use thiserror::Error;

// ============================================================================
// Domain Errors
// ============================================================================

/// Errors that can occur in the beads domain.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("ID cannot be empty")]
    EmptyId,

    #[error("ID must match pattern: {0}")]
    InvalidIdPattern(String),

    #[error("Title cannot be empty")]
    EmptyTitle,

    #[error("Title exceeds maximum length of {max} characters (got {got})")]
    TitleTooLong { max: usize, got: usize },

    #[error("Description exceeds maximum length of {max} characters")]
    DescriptionTooLong { max: usize },

    #[error("Invalid datetime format: {0}")]
    InvalidDatetime(String),

    #[error("Issue not found: {0}")]
    NotFound(String),

    #[error("Duplicate issue ID: {0}")]
    DuplicateId(String),

    #[error("Invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition { from: IssueState, to: IssueState },

    #[error("Closed issues must have a closed_at timestamp")]
    ClosedWithoutTimestamp,

    #[error("Invalid filter criteria: {0}")]
    InvalidFilter(String),
}

// ============================================================================
// Semantic Newtypes - Identifiers
// ============================================================================

/// A validated issue identifier.
///
/// Must be non-empty and match typical ID patterns (alphanumeric, hyphens, underscores).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IssueId(String);

impl IssueId {
    /// Maximum length for issue IDs.
    pub const MAX_LENGTH: usize = 100;

    /// Create a new `IssueId`, validating the input.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::EmptyId` if the input is empty.
    /// Returns `DomainError::InvalidIdPattern` if the pattern doesn't match.
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();

        if id.is_empty() {
            return Err(DomainError::EmptyId);
        }

        if id.len() > Self::MAX_LENGTH {
            return Err(DomainError::InvalidIdPattern(format!(
                "ID exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }

        // Validate pattern: alphanumeric, hyphens, underscores only
        if !id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(DomainError::InvalidIdPattern(
                "ID must contain only alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }

        Ok(Self(id))
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for IssueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for IssueId {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for IssueId {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// ============================================================================
// Semantic Newtypes - Text Fields
// ============================================================================

/// A validated issue title.
///
/// Must be non-empty and within length limits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Title(String);

impl Title {
    /// Maximum length for titles.
    pub const MAX_LENGTH: usize = 200;

    /// Create a new `Title`, validating the input.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::EmptyTitle` if the input is empty.
    /// Returns `DomainError::TitleTooLong` if the input exceeds max length.
    pub fn new(title: impl Into<String>) -> Result<Self, DomainError> {
        let title = title.into();
        let trimmed = title.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyTitle);
        }

        if trimmed.len() > Self::MAX_LENGTH {
            return Err(DomainError::TitleTooLong {
                max: Self::MAX_LENGTH,
                got: trimmed.len(),
            });
        }

        Ok(Self(trimmed.to_string()))
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Title {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Title {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// A validated issue description.
///
/// Optional field with length limits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Description(String);

impl Description {
    /// Maximum length for descriptions.
    pub const MAX_LENGTH: usize = 10_000;

    /// Create a new `Description`, validating the input.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::DescriptionTooLong` if the input exceeds max length.
    pub fn new(description: impl Into<String>) -> Result<Self, DomainError> {
        let description = description.into();

        if description.len() > Self::MAX_LENGTH {
            return Err(DomainError::DescriptionTooLong {
                max: Self::MAX_LENGTH,
            });
        }

        Ok(Self(description))
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Description {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Description {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Description {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// ============================================================================
// Assignee and Parent Identifiers
// ============================================================================

/// An assignee identifier (username or email).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Assignee(String);

impl Assignee {
    /// Maximum length for assignee.
    pub const MAX_LENGTH: usize = 100;

    /// Create a new `Assignee`, validating the input.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidIdPattern` if the pattern doesn't match.
    pub fn new(assignee: impl Into<String>) -> Result<Self, DomainError> {
        let assignee = assignee.into();

        if assignee.is_empty() {
            return Err(DomainError::InvalidIdPattern(
                "Assignee cannot be empty".to_string(),
            ));
        }

        if assignee.len() > Self::MAX_LENGTH {
            return Err(DomainError::InvalidIdPattern(format!(
                "Assignee exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }

        Ok(Self(assignee))
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Assignee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Assignee {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// A parent issue identifier.
///
/// Type alias for semantic clarity - references another issue.
pub type ParentId = IssueId;

// ============================================================================
// State Types - Making Illegal States Unrepresentable
// ============================================================================

/// The complete state of an issue.
///
/// This enum makes the closed timestamp requirement unrepresentable:
/// - `Closed` variant *must* include a timestamp
/// - No other variant can be closed
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
pub enum IssueState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    #[strum(to_string = "closed")]
    Closed {
        closed_at: DateTime<Utc>,
    },
}

impl IssueState {
    /// Check if the issue is in an active state (open or in progress).
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Open | Self::InProgress)
    }

    /// Check if the issue is blocked.
    #[must_use]
    pub const fn is_blocked(self) -> bool {
        matches!(self, Self::Blocked)
    }

    /// Check if the issue is closed.
    #[must_use]
    pub const fn is_closed(self) -> bool {
        matches!(self, Self::Closed { .. })
    }

    /// Get the closed timestamp if the issue is closed.
    #[must_use]
    pub const fn closed_at(self) -> Option<DateTime<Utc>> {
        match self {
            Self::Closed { closed_at } => Some(closed_at),
            _ => None,
        }
    }

    /// Transition to a new state with validation.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidStateTransition` if the transition is invalid.
    pub const fn transition_to(self, new_state: Self) -> Result<Self, DomainError> {
        // Can transition from any state to any state (flexible workflow)
        // But Closed MUST have a timestamp (enforced by type)
        Ok(new_state)
    }
}

// ============================================================================
// Other Domain Types
// ============================================================================

/// Type classification for issues.
#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
    #[strum(to_string = "merge-request")]
    MergeRequest,
}

/// Priority level for issues.
///
/// Lower number = higher priority.
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
    pub const fn to_u32(&self) -> u32 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
            Self::P4 => 4,
        }
    }
}

// ============================================================================
// Labels Collection
// ============================================================================

/// A collection of validated labels.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Labels(Vec<String>);

impl Labels {
    /// Maximum number of labels per issue.
    pub const MAX_COUNT: usize = 20;
    /// Maximum length per label.
    pub const MAX_LABEL_LENGTH: usize = 50;

    /// Create new labels from a vector.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidFilter` if validation fails.
    pub fn new(labels: Vec<String>) -> Result<Self, DomainError> {
        if labels.len() > Self::MAX_COUNT {
            return Err(DomainError::InvalidFilter(format!(
                "Cannot have more than {} labels",
                Self::MAX_COUNT
            )));
        }

        for label in &labels {
            if label.len() > Self::MAX_LABEL_LENGTH {
                return Err(DomainError::InvalidFilter(format!(
                    "Label exceeds maximum length of {}",
                    Self::MAX_LABEL_LENGTH
                )));
            }
        }

        Ok(Self(labels))
    }

    /// Create empty labels.
    #[must_use]
    pub const fn empty() -> Self {
        Self(Vec::new())
    }

    /// Get iterator over labels.
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }

    /// Check if contains a label.
    #[must_use]
    pub fn contains(&self, label: &str) -> bool {
        self.0.iter().any(|l| l == label)
    }

    /// Get number of labels.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Add a label, returning a new Labels instance.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if adding the label would exceed limits.
    pub fn add(&self, label: String) -> Result<Self, DomainError> {
        let mut new_labels = self.0.clone();
        new_labels.push(label);
        Self::new(new_labels)
    }

    /// Remove a label if it exists, returning a new Labels instance.
    #[must_use]
    pub fn remove(&self, label: &str) -> Self {
        let new_labels: Vec<String> = self.0.iter().filter(|l| l != &label).cloned().collect();
        // Note: We don't use new() here since we're removing, not adding
        // and the resulting labels are guaranteed to be valid
        Self(new_labels)
    }

    /// Get the inner vector as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    /// Clone the inner vector.
    #[must_use]
    pub fn to_vec(&self) -> Vec<String> {
        self.0.clone()
    }
}

impl Default for Labels {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Dependencies Collection
// ============================================================================

/// A collection of issue IDs that this issue depends on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DependsOn(Vec<IssueId>);

impl DependsOn {
    /// Maximum number of dependencies per issue.
    pub const MAX_COUNT: usize = 50;

    /// Create new dependencies from a vector of issue IDs.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if any ID is invalid or count exceeds limit.
    pub fn new(ids: Vec<String>) -> Result<Self, DomainError> {
        if ids.len() > Self::MAX_COUNT {
            return Err(DomainError::InvalidFilter(format!(
                "Cannot have more than {} dependencies",
                Self::MAX_COUNT
            )));
        }

        let validated = ids
            .into_iter()
            .map(IssueId::new)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self(validated))
    }

    /// Create empty dependencies.
    #[must_use]
    pub const fn empty() -> Self {
        Self(Vec::new())
    }

    /// Get iterator over dependency IDs.
    pub fn iter(&self) -> impl Iterator<Item = &IssueId> {
        self.0.iter()
    }

    /// Check if depends on a specific issue.
    #[must_use]
    pub fn contains(&self, id: &IssueId) -> bool {
        self.0.iter().any(|d| d == id)
    }

    /// Get number of dependencies.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for DependsOn {
    fn default() -> Self {
        Self::empty()
    }
}

/// A collection of issue IDs that are blocking this issue.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockedBy(Vec<IssueId>);

impl BlockedBy {
    /// Maximum number of blockers per issue.
    pub const MAX_COUNT: usize = 50;

    /// Create new blockers from a vector of issue IDs.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if any ID is invalid or count exceeds limit.
    pub fn new(ids: Vec<String>) -> Result<Self, DomainError> {
        if ids.len() > Self::MAX_COUNT {
            return Err(DomainError::InvalidFilter(format!(
                "Cannot have more than {} blockers",
                Self::MAX_COUNT
            )));
        }

        let validated = ids
            .into_iter()
            .map(IssueId::new)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self(validated))
    }

    /// Create empty blockers.
    #[must_use]
    pub const fn empty() -> Self {
        Self(Vec::new())
    }

    /// Get iterator over blocker IDs.
    pub fn iter(&self) -> impl Iterator<Item = &IssueId> {
        self.0.iter()
    }

    /// Check if blocked by a specific issue.
    #[must_use]
    pub fn contains(&self, id: &IssueId) -> bool {
        self.0.iter().any(|b| b == id)
    }

    /// Get number of blockers.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for BlockedBy {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_id_valid() {
        assert!(IssueId::new("valid-id-123").is_ok());
        assert!(IssueId::new("valid_id_456").is_ok());
    }

    #[test]
    fn test_issue_id_invalid() {
        assert!(matches!(IssueId::new(""), Err(DomainError::EmptyId)));
        assert!(IssueId::new("invalid id").is_err());
        assert!(IssueId::new("invalid.id").is_err());
    }

    #[test]
    fn test_title_valid() {
        assert!(Title::new("Valid Title").is_ok());
        assert!(Title::new("  Trimmed  ").is_ok());
    }

    #[test]
    fn test_title_invalid() {
        assert!(matches!(Title::new(""), Err(DomainError::EmptyTitle)));
        assert!(Title::new("  ").is_err()); // Trimmed to empty
    }

    #[test]
    fn test_issue_state_closed_has_timestamp() {
        let state = IssueState::Closed {
            closed_at: Utc::now(),
        };
        assert!(state.is_closed());
        assert!(state.closed_at().is_some());
    }

    #[test]
    fn test_issue_state_open_no_timestamp() {
        let state = IssueState::Open;
        assert!(!state.is_closed());
        assert!(state.closed_at().is_none());
        assert!(state.is_active());
    }

    #[test]
    fn test_labels_validation() {
        assert!(Labels::new(vec!["label1".to_string(), "label2".to_string()]).is_ok());

        // Test exceeding max count
        let too_many_labels: Vec<String> = (0..=Labels::MAX_COUNT)
            .map(|i| format!("label{i}"))
            .collect();
        assert!(Labels::new(too_many_labels).is_err());
    }
}
