use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// Value object representing a Git branch name
///
/// Valid branch names must follow Git naming conventions:
/// - Can contain lowercase letters, uppercase letters, numbers
/// - Can contain hyphens, underscores, periods, slashes
/// - Cannot start or end with hyphen
/// - Cannot start or end with period
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

impl BranchName {
    /// Create a new branch name with validation
    pub fn new(name: &str) -> Result<Self, super::WorktreeDomainError> {
        if name.is_empty() {
            return Err(super::WorktreeDomainError::InvalidBranch(
                "Branch name cannot be empty".to_string(),
            ));
        }

        // Check valid characters
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
        {
            return Err(super::WorktreeDomainError::InvalidBranch(format!(
                "Branch name contains invalid characters: {}",
                name
            )));
        }

        // Cannot start or end with hyphen
        if name.starts_with('-') || name.ends_with('-') {
            return Err(super::WorktreeDomainError::InvalidBranch(
                "Branch name cannot start or end with hyphen".to_string(),
            ));
        }

        // Cannot start or end with period
        if name.starts_with('.') || name.ends_with('.') {
            return Err(super::WorktreeDomainError::InvalidBranch(
                "Branch name cannot start or end with period".to_string(),
            ));
        }

        // Cannot contain consecutive periods
        if name.contains("..") {
            return Err(super::WorktreeDomainError::InvalidBranch(
                "Branch name cannot contain consecutive periods".to_string(),
            ));
        }

        Ok(Self(name.to_string()))
    }

    /// Get the branch name as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the branch name as owned string
    pub fn into_string(self) -> String {
        self.0
    }

    /// Check if this is a default branch (main/master)
    pub fn is_default_branch(&self) -> bool {
        let name = self.0.to_lowercase();
        name == "main" || name == "master"
    }

    /// Check if this is a feature branch
    pub fn is_feature_branch(&self) -> bool {
        self.0.starts_with("feature/") || self.0.starts_with("feat/")
    }

    /// Check if this is a release branch
    pub fn is_release_branch(&self) -> bool {
        self.0.starts_with("release/") || self.0.starts_with("rel/")
    }
}

impl Display for BranchName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<BranchName> for String {
    fn from(name: BranchName) -> Self {
        name.0
    }
}

impl<'a> From<&'a BranchName> for &'a str {
    fn from(name: &'a BranchName) -> Self {
        &name.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_name_new_valid_name_returns_instance() {
        let name = BranchName::new("main").unwrap();
        assert_eq!(name.as_str(), "main");
    }

    #[test]
    fn branch_name_new_empty_string_returns_invalid_branch_error() {
        let result = BranchName::new("");
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_new_with_invalid_characters_returns_error() {
        let result = BranchName::new("feature@branch");
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_new_starts_with_hyphen_returns_error() {
        let result = BranchName::new("-feature");
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_new_ends_with_hyphen_returns_error() {
        let result = BranchName::new("feature-");
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_new_starts_with_period_returns_error() {
        let result = BranchName::new(".feature");
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_new_ends_with_period_returns_error() {
        let result = BranchName::new("feature.");
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_new_with_consecutive_periods_returns_error() {
        let result = BranchName::new("feature..test");
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_new_valid_formats_all_return_ok() {
        let valid_names = vec![
            "main",
            "master",
            "feature/new-ui",
            "feat/login",
            "release/1.0.0",
            "bugfix/issue-123",
            "hotfix/critical",
            "feature_branch",
            "feature.branch",
            "Feature-Test",
        ];

        for name in valid_names {
            let result = BranchName::new(name);
            assert!(result.is_ok(), "Branch name '{}' should be valid", name);
        }
    }

    #[test]
    fn branch_name_is_default_branch_returns_true_for_main() {
        assert!(BranchName::new("main").unwrap().is_default_branch());
        assert!(BranchName::new("master").unwrap().is_default_branch());
        assert!(!BranchName::new("develop").unwrap().is_default_branch());
    }

    #[test]
    fn branch_name_is_feature_branch_returns_true_for_feature() {
        assert!(BranchName::new("feature/new-ui")
            .unwrap()
            .is_feature_branch());
        assert!(BranchName::new("feat/login").unwrap().is_feature_branch());
        assert!(!BranchName::new("feature").unwrap().is_feature_branch());
    }

    #[test]
    fn branch_name_is_release_branch_returns_true_for_release() {
        assert!(BranchName::new("release/1.0.0")
            .unwrap()
            .is_release_branch());
        assert!(BranchName::new("rel/2.0").unwrap().is_release_branch());
        assert!(!BranchName::new("release").unwrap().is_release_branch());
    }
}
