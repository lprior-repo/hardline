//! Change and status types for VCS operations
//!
//! This module provides:
//! - `Change` - A single atomic modification in VCS history
//! - `RepoStatus` - Current state of the working directory

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::errors::ChangeError;
use super::types::{BranchName, ChangeId};

// ============================================================================
// Change
// ============================================================================

/// A single atomic change in VCS history
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    /// Unique identifier for this change
    id: ChangeId,
    /// Commit message (first line / summary)
    message: String,
    /// Author of the change (e.g., "Alice <alice@example.com>")
    author: String,
    /// Timestamp when the change was created
    timestamp: DateTime<Utc>,
}

impl Change {
    /// Create a new Change with validation
    ///
    /// # Preconditions
    /// - P6: `message` is not empty (after trimming)
    /// - P7: `author` is not empty (after trimming)
    ///
    /// # Postconditions
    /// - Q5: All fields populated
    /// - I5: Message is trimmed
    ///
    /// # Errors
    /// - `ChangeError::EmptyMessage` if message is empty
    /// - `ChangeError::EmptyAuthor` if author is empty
    pub fn new(
        id: ChangeId,
        message: impl Into<String>,
        author: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> Result<Self, ChangeError> {
        let message = message.into();
        let author = author.into();

        if is_effectively_empty_for_change(&message) {
            return Err(ChangeError::EmptyMessage);
        }

        if is_effectively_empty_for_change(&author) {
            return Err(ChangeError::EmptyAuthor);
        }

        Ok(Self {
            id,
            message: message.trim().to_string(),
            author: author.trim().to_string(),
            timestamp,
        })
    }

    /// Get a reference to the change ID
    #[must_use]
    pub fn id(&self) -> &ChangeId {
        &self.id
    }

    /// Get the commit message
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the author string
    #[must_use]
    pub fn author(&self) -> &str {
        &self.author
    }

    /// Get the timestamp
    #[must_use]
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

/// Check if a string is effectively empty (helper for Change validation)
fn is_effectively_empty_for_change(s: &str) -> bool {
    s.trim().is_empty()
}

// ============================================================================
// RepoStatus
// ============================================================================

/// Status of a repository working directory
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoStatus {
    /// Whether the working directory has uncommitted changes
    pub has_changes: bool,
    /// Number of added files
    pub added: u32,
    /// Number of modified files
    pub modified: u32,
    /// Number of deleted files
    pub deleted: u32,
    /// Current branch name (if any)
    pub current_branch: Option<BranchName>,
}
