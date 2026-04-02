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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vcs::ChangeId;

    // -- Change validation tests --

    #[test]
    fn change_new_valid() {
        let id = ChangeId::from_git_sha("abc123def456").expect("valid sha");
        let change = Change::new(
            id.clone(),
            "Implement feature",
            "Alice <alice@example.com>",
            Utc::now(),
        ).expect("valid change");
        assert_eq!(change.id(), &id);
        assert_eq!(change.message(), "Implement feature");
        assert_eq!(change.author(), "Alice <alice@example.com>");
    }

    #[test]
    fn change_new_trims_message() {
        let id = ChangeId::from_git_sha("abc123def456").expect("valid sha");
        let change = Change::new(
            id,
            "  Spaced message  ",
            "Bob",
            Utc::now(),
        ).expect("valid change");
        assert_eq!(change.message(), "Spaced message");
    }

    #[test]
    fn change_new_trims_author() {
        let id = ChangeId::from_git_sha("abc123def456").expect("valid sha");
        let change = Change::new(
            id,
            "A commit",
            "  Charlie <c@e.com>  ",
            Utc::now(),
        ).expect("valid change");
        assert_eq!(change.author(), "Charlie <c@e.com>");
    }

    #[test]
    fn change_new_empty_message_rejects() {
        let id = ChangeId::from_git_sha("abc123def456").expect("valid sha");
        let result = Change::new(id, "", "Author", Utc::now());
        assert_eq!(result, Err(ChangeError::EmptyMessage));
    }

    #[test]
    fn change_new_whitespace_message_rejects() {
        let id = ChangeId::from_git_sha("abc123def456").expect("valid sha");
        let result = Change::new(id, "   ", "Author", Utc::now());
        assert_eq!(result, Err(ChangeError::EmptyMessage));
    }

    #[test]
    fn change_new_empty_author_rejects() {
        let id = ChangeId::from_git_sha("abc123def456").expect("valid sha");
        let result = Change::new(id, "Message", "", Utc::now());
        assert_eq!(result, Err(ChangeError::EmptyAuthor));
    }

    #[test]
    fn change_new_whitespace_author_rejects() {
        let id = ChangeId::from_git_sha("abc123def456").expect("valid sha");
        let result = Change::new(id, "Message", "   ", Utc::now());
        assert_eq!(result, Err(ChangeError::EmptyAuthor));
    }

    #[test]
    fn change_timestamp_returns_same_value() {
        let ts = Utc::now();
        let id = ChangeId::from_git_sha("abc123def456").expect("valid sha");
        let change = Change::new(id, "msg", "auth", ts).expect("valid");
        assert_eq!(change.timestamp(), ts);
    }

    #[test]
    fn change_clone() {
        let id = ChangeId::from_git_sha("abc123def456").expect("valid sha");
        let change = Change::new(id, "msg", "auth", Utc::now()).expect("valid");
        let cloned = change.clone();
        assert_eq!(change.id().as_str(), cloned.id().as_str());
        assert_eq!(change.message(), cloned.message());
    }

    #[test]
    fn change_serde_roundtrip() {
        let id = ChangeId::from_git_sha("deadbeef1234567").expect("valid sha");
        let change = Change::new(
            id,
            "Serde test commit",
            "Test Author <test@test.com>",
            Utc::now(),
        ).expect("valid");
        let json = serde_json::to_string(&change).expect("serialize");
        let deserialized: Change = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(change.message(), deserialized.message());
        assert_eq!(change.author(), deserialized.author());
        assert_eq!(change.id().as_str(), deserialized.id().as_str());
    }

    #[test]
    fn change_eq_same_values() {
        let ts = chrono::DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z").expect("parse");
        let ts_utc = ts.with_timezone(&Utc);
        let id = ChangeId::from_git_sha("abc1234").expect("valid");
        let a = Change::new(id.clone(), "msg", "auth", ts_utc).expect("valid");
        let b = Change::new(id, "msg", "auth", ts_utc).expect("valid");
        assert_eq!(a, b);
    }

    // -- RepoStatus tests --

    #[test]
    fn repo_status_default_is_clean() {
        let status = RepoStatus::default();
        assert!(!status.has_changes);
        assert_eq!(status.added, 0);
        assert_eq!(status.modified, 0);
        assert_eq!(status.deleted, 0);
        assert!(status.current_branch.is_none());
    }

    #[test]
    fn repo_status_with_all_changes() {
        let branch = BranchName::new("test").expect("valid");
        let status = RepoStatus {
            has_changes: true,
            added: 5,
            modified: 3,
            deleted: 2,
            current_branch: Some(branch),
        };
        assert!(status.has_changes);
        assert_eq!(status.added, 5);
        assert_eq!(status.modified, 3);
        assert_eq!(status.deleted, 2);
        assert_eq!(status.current_branch.as_ref().map(BranchName::as_str), Some("test"));
    }

    #[test]
    fn repo_status_serde_roundtrip_clean() {
        let status = RepoStatus::default();
        let json = serde_json::to_string(&status).expect("serialize");
        let deserialized: RepoStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, deserialized);
    }

    #[test]
    fn repo_status_serde_roundtrip_dirty() {
        let branch = BranchName::new("develop").expect("valid");
        let status = RepoStatus {
            has_changes: true,
            added: 1,
            modified: 2,
            deleted: 0,
            current_branch: Some(branch),
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let deserialized: RepoStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, deserialized);
    }

    #[test]
    fn repo_status_clone() {
        let status = RepoStatus {
            has_changes: true,
            added: 10,
            modified: 20,
            deleted: 5,
            current_branch: None,
        };
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }
}
