//! Branch name type
//!
//! This module provides `BranchName` - named reference to a line of development.

use serde::{Deserialize, Serialize};

use crate::vcs::errors::VcsError;

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

fn has_invalid_branch_syntax(name: &str) -> bool {
    if name == "@" {
        return true;
    }

    if name.starts_with('/') || name.ends_with('/') || name.ends_with('.') {
        return true;
    }

    if name.contains("..")
        || name.contains("@{")
        || std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lock"))
    {
        return true;
    }

    if name.chars().any(|char| {
        char.is_control() || matches!(char, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    }) {
        return true;
    }

    name.split('/').any(str::is_empty)
}

// ============================================================================
// BranchName
// ============================================================================

/// Name of a branch in the VCS
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

impl BranchName {
    /// Create a new branch name with validation
    ///
    /// # Errors
    /// - `VcsError::InvalidBranchName` if name is empty, whitespace-only, or contains only invisible characters
    pub fn new(name: impl Into<String>) -> Result<Self, VcsError> {
        let name = name.into();

        if is_effectively_empty(&name) || has_invalid_branch_syntax(&name) {
            return Err(VcsError::InvalidBranchName(name));
        }

        Ok(Self(name))
    }

    /// Get the branch name as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
