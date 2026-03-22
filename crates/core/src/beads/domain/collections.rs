//! Collection types for the beads domain.
//!
//! Newtype wrappers around vectors with validation.

use super::errors::DomainError;
use super::ids::IssueId;
use serde::{Deserialize, Serialize};

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
