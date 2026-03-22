//! Change ID type
//!
//! This module provides `ChangeId` - unique identifier for a VCS change/commit (Git SHA or JJ ID).

use serde::{Deserialize, Serialize};

use crate::vcs::errors::ParseError;
use crate::vcs::types::backend_type::BackendType;

// ============================================================================
// Helper Functions
// ============================================================================

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

fn is_effectively_empty(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    if s.trim().is_empty() {
        return true;
    }

    s.chars().all(|c| c.is_whitespace() || is_invisible_char(c))
}

// ============================================================================
// ChangeIdInner
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum ChangeIdInner {
    /// Git commit SHA (7-40 lowercase hex chars)
    Git { sha: String },
    /// JJ change ID (lowercase base36)
    Jj { id: String },
}

// ============================================================================
// ChangeId
// ============================================================================

/// Unique identifier for a VCS change/commit
///
/// # Invariants
/// - Always contains a non-empty, trimmed ID string
/// - Git SHAs are lowercase hex
/// - JJ change IDs are lowercase base36
/// - Backend type is encoded to prevent cross-backend comparison
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeId {
    inner: ChangeIdInner,
}

impl ChangeId {
    /// Create a Git `ChangeId` from a SHA string
    ///
    /// # Preconditions
    /// - P1: `sha` is not empty
    /// - P2: `sha` contains only hex characters (0-9, a-f, A-F)
    /// - P4: `sha` length is 7-40 characters
    ///
    /// # Postconditions
    /// - Q4: SHA is normalized to lowercase
    ///
    /// # Errors
    /// - `ParseError::Empty` if input is empty/whitespace
    /// - `ParseError::InvalidCharacters` if non-hex chars present
    /// - `ParseError::InvalidGitShaLength` if length invalid
    pub fn from_git_sha(sha: impl AsRef<str>) -> Result<Self, ParseError> {
        let sha = sha.as_ref().trim();

        if is_effectively_empty(sha) {
            return Err(ParseError::Empty);
        }

        let len = sha.len();
        if !(7..=40).contains(&len) {
            return Err(ParseError::InvalidGitShaLength(len));
        }

        if !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ParseError::InvalidCharacters(sha.to_string()));
        }

        Ok(Self {
            inner: ChangeIdInner::Git {
                sha: sha.to_lowercase(),
            },
        })
    }

    /// Create a JJ `ChangeId` from a change ID string
    ///
    /// # Preconditions
    /// - P1: `id` is not empty
    /// - P3: `id` contains only base36 characters (0-9, a-z)
    /// - P5: `id` length is >= 1
    ///
    /// # Errors
    /// - `ParseError::Empty` if input is empty/whitespace
    /// - `ParseError::InvalidCharacters` if non-base36 chars present
    /// - `ParseError::InvalidJjLength` if length is 0
    pub fn from_jj_id(id: impl AsRef<str>) -> Result<Self, ParseError> {
        let id = id.as_ref().trim();

        if is_effectively_empty(id) {
            return Err(ParseError::Empty);
        }

        let len = id.len();
        if len == 0 {
            return Err(ParseError::InvalidJjLength(len));
        }

        let normalized = id.to_lowercase();
        if !normalized
            .chars()
            .all(|c: char| c.is_ascii_digit() || c.is_ascii_lowercase())
        {
            return Err(ParseError::InvalidCharacters(id.to_string()));
        }

        Ok(Self {
            inner: ChangeIdInner::Jj { id: normalized },
        })
    }

    /// Get the backend type for this `ChangeId`
    ///
    /// # Postconditions
    /// - Q3: Returns correct `BackendType`
    #[must_use]
    pub fn backend_type(&self) -> BackendType {
        match &self.inner {
            ChangeIdInner::Git { .. } => BackendType::Git,
            ChangeIdInner::Jj { .. } => BackendType::Jj,
        }
    }

    /// Get the ID as a string slice (without backend prefix)
    ///
    /// # Postconditions
    /// - Q2: Returns inner ID only
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.inner {
            ChangeIdInner::Git { sha } => sha,
            ChangeIdInner::Jj { id } => id,
        }
    }
}

impl std::str::FromStr for ChangeId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();

        if is_effectively_empty(trimmed) {
            return Err(ParseError::Empty);
        }

        let is_hex = trimmed.chars().all(|c| c.is_ascii_hexdigit());

        if is_hex {
            Self::from_git_sha(trimmed)
        } else {
            Self::from_jj_id(trimmed)
        }
    }
}

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            ChangeIdInner::Git { sha } => write!(f, "git:{sha}"),
            ChangeIdInner::Jj { id } => write!(f, "jj:{id}"),
        }
    }
}
