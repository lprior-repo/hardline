use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// Value object representing a valid worktree name
///
/// Valid names must:
/// - Not be empty
/// - Not exceed 128 characters
/// - Not contain '/' characters
/// - Not start with '.'
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorktreeName(String);

impl WorktreeName {
    const MAX_LENGTH: usize = 128;

    /// Create a new worktree name with validation
    pub fn new(name: &str) -> Result<Self, super::WorktreeDomainError> {
        if name.is_empty() {
            return Err(super::WorktreeDomainError::InvalidName(
                "Name cannot be empty".to_string(),
            ));
        }

        if name.len() > Self::MAX_LENGTH {
            return Err(super::WorktreeDomainError::InvalidName(format!(
                "Name exceeds maximum length of {} characters",
                Self::MAX_LENGTH
            )));
        }

        if name.contains('/') {
            return Err(super::WorktreeDomainError::InvalidName(
                "Name cannot contain '/'".to_string(),
            ));
        }

        if name.starts_with('.') {
            return Err(super::WorktreeDomainError::InvalidName(
                "Name cannot start with '.'".to_string(),
            ));
        }

        Ok(Self(name.to_string()))
    }

    /// Create a worktree name without validation (unsafe, use with caution)
    pub fn new_unchecked(name: String) -> Self {
        Self(name)
    }

    /// Get the name as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the name as owned string
    pub fn into_string(self) -> String {
        self.0
    }

    /// Check if this name matches another
    pub fn matches(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl Display for WorktreeName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<WorktreeName> for String {
    fn from(name: WorktreeName) -> Self {
        name.0
    }
}

impl<'a> From<&'a WorktreeName> for &'a str {
    fn from(name: &'a WorktreeName) -> Self {
        &name.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::errors::WorktreeDomainError;

    #[test]
    fn worktree_name_new_valid_name_returns_instance() {
        let name = WorktreeName::new("feature-branch").unwrap();
        assert_eq!(name.as_str(), "feature-branch");
    }

    #[test]
    fn worktree_name_new_empty_string_returns_invalid_name_error() {
        let result = WorktreeName::new("");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            WorktreeDomainError::InvalidName("Name cannot be empty".to_string())
        );
    }

    #[test]
    fn worktree_name_new_with_slash_returns_invalid_name_error() {
        let result = WorktreeName::new("feature/sub");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            WorktreeDomainError::InvalidName("Name cannot contain '/'".to_string())
        );
    }

    #[test]
    fn worktree_name_new_starts_with_dot_returns_invalid_name_error() {
        let result = WorktreeName::new(".hidden");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            WorktreeDomainError::InvalidName("Name cannot start with '.'".to_string())
        );
    }

    #[test]
    fn worktree_name_display_impl_returns_string() {
        let name = WorktreeName::new("my-worktree").unwrap();
        assert_eq!(format!("{}", name), "my-worktree");
    }

    #[test]
    fn worktree_name_conversion_to_string_and_str_preserves_value() {
        let name = WorktreeName::new("test").unwrap();
        let owned: String = name.clone().into();
        assert_eq!(owned, "test");

        let slice: &str = (&name).into();
        assert_eq!(slice, "test");
    }

    #[test]
    fn worktree_name_matches_returns_true_for_same_string() {
        let name = WorktreeName::new("my-worktree").unwrap();
        assert!(name.matches("my-worktree"));
        assert!(!name.matches("other-worktree"));
    }

    #[test]
    fn worktree_name_new_exceeds_max_length_returns_error() {
        let long_name = "a".repeat(129);
        let result = WorktreeName::new(&long_name);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("maximum length"));
    }

    #[test]
    fn worktree_name_new_at_max_length_succeeds() {
        let name = "a".repeat(128);
        let result = WorktreeName::new(&name);
        assert!(result.is_ok());
    }
}
