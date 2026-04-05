//! Data types and the branch command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

// ============================================================================
// Constants
// ============================================================================

/// Branches that cannot be deleted or renamed.
const PROTECTED_BRANCHES: &[&str] = &["main", "master", "trunk", "develop"];

// ============================================================================
// Input Types
// ============================================================================

/// Options for the branch create command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct BranchCreateOptions {
    /// Name of the new branch.
    pub name: String,
    /// Dry-run mode.
    pub dry_run: bool,
}

/// Options for the branch delete command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct BranchDeleteOptions {
    /// Name of the branch to delete.
    pub name: String,
    /// Force deletion even if unmerged.
    pub force: bool,
    /// Dry-run mode.
    pub dry_run: bool,
}

/// Options for the branch rename command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct BranchRenameOptions {
    /// Current branch name.
    pub old_name: String,
    /// New branch name.
    pub new_name: String,
    /// Dry-run mode.
    pub dry_run: bool,
}

// ============================================================================
// Output Types
// ============================================================================

/// Output from branch create command.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchCreateOutput {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Name of the branch.
    pub branch_name: String,
    /// Whether this was a dry-run.
    pub dry_run: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

// ============================================================================
// Pure computation functions (Tier 2)
// ============================================================================

/// Check if a branch name is protected.
#[must_use]
pub fn is_protected_branch(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "main" | "master" | "trunk" | "develop"
    )
}

/// Validate a branch name using Git naming conventions.
///
/// # Errors
///
/// Returns error if name is empty, contains invalid characters, or is a reserved name.
pub fn validate_branch_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Branch name cannot be empty".to_string());
    }

    if name.trim().is_empty() {
        return Err("Branch name cannot be whitespace only".to_string());
    }

    if name == "@" {
        return Err("Branch name '@' is reserved".to_string());
    }

    if name.starts_with('/') || name.ends_with('/') {
        return Err("Branch name cannot start or end with '/'".to_string());
    }

    if name.ends_with('.') {
        return Err("Branch name cannot end with '.'".to_string());
    }

    if name.contains("..") {
        return Err("Branch name cannot contain '..'".to_string());
    }

    if name.contains("@{") {
        return Err("Branch name cannot contain '@{'".to_string());
    }

    if name
        .chars()
        .any(|c| c.is_control() || matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\'))
    {
        return Err(format!("Branch name '{}' contains invalid characters", name));
    }

    if name.split('/').any(str::is_empty) {
        return Err("Branch name cannot contain empty path segments".to_string());
    }

    if std::path::Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lock"))
    {
        return Err("Branch name cannot end with '.lock'".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- validate_branch_name ----

    #[test]
    fn validate_branch_name_valid() {
        assert!(validate_branch_name("main").is_ok());
        assert!(validate_branch_name("feature/test").is_ok());
        assert!(validate_branch_name("release/v1.0.0").is_ok());
        assert!(validate_branch_name("my-branch").is_ok());
    }

    #[test]
    fn validate_branch_name_empty() {
        assert!(validate_branch_name("").is_err());
    }

    #[test]
    fn validate_branch_name_whitespace_only() {
        assert!(validate_branch_name("   ").is_err());
    }

    #[test]
    fn validate_branch_name_at_sign() {
        assert!(validate_branch_name("@").is_err());
    }

    #[test]
    fn validate_branch_name_slash() {
        assert!(validate_branch_name("/main").is_err());
        assert!(validate_branch_name("main/").is_err());
    }

    #[test]
    fn validate_branch_name_trailing_dot() {
        assert!(validate_branch_name("main.").is_err());
    }

    #[test]
    fn validate_branch_name_double_dot() {
        assert!(validate_branch_name("feat..ure").is_err());
    }

    #[test]
    fn validate_branch_name_space() {
        assert!(validate_branch_name("feature name").is_err());
    }

    #[test]
    fn validate_branch_name_lock_extension() {
        assert!(validate_branch_name("test.lock").is_err());
    }

    // ---- is_protected_branch ----

    #[test]
    fn is_protected_branch_main() {
        assert!(is_protected_branch("main"));
        assert!(is_protected_branch("Main"));
        assert!(is_protected_branch("MASTER"));
    }

    #[test]
    fn is_protected_branch_feature() {
        assert!(!is_protected_branch("feature"));
        assert!(!is_protected_branch("release/v1.0"));
    }
}
