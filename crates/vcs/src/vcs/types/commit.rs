//! Commit ID type
//!
//! This module provides `CommitId` - unique identifier for a commit.

use serde::{Deserialize, Serialize};

use crate::vcs::errors::VcsError;

// ============================================================================
// Helper Functions
// ============================================================================

fn is_effectively_empty(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    if s.trim().is_empty() {
        return true;
    }

    s.chars().all(|c| c.is_whitespace() || is_invisible_char(c))
}

fn is_invisible_char(c: char) -> bool {
    matches!(
        c,
        '\u{FEFF}'
            | '\u{200B}'
            | '\u{200C}'
            | '\u{200D}'
            | '\u{2060}'
            | '\u{00AD}'
            | '\u{034F}'
            | '\u{061C}'
            | '\u{180E}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{115F}'
            | '\u{1160}'
    ) || is_in_range(c, '\u{2061}', '\u{2064}')
        || is_in_range(c, '\u{206A}', '\u{206F}')
        || is_in_range(c, '\u{17B4}', '\u{17B5}')
        || is_in_range(c, '\u{202A}', '\u{202E}')
        || is_in_range(c, '\u{2066}', '\u{2069}')
}

fn is_in_range(c: char, start: char, end: char) -> bool {
    c >= start && c <= end
}

// ============================================================================
// CommitId
// ============================================================================

/// Unique identifier for a commit
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitId(String);

impl CommitId {
    /// Create a new commit ID with validation
    ///
    /// # Errors
    /// - `VcsError::InvalidCommitId` if ID is empty, whitespace-only, or contains only invisible characters
    pub fn new(id: impl Into<String>) -> Result<Self, VcsError> {
        let id = id.into();

        if is_effectively_empty(&id) {
            return Err(VcsError::InvalidCommitId(id));
        }

        Ok(Self(id))
    }

    /// Get the commit ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
