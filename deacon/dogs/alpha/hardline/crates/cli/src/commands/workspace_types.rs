//! Workspace types

use scp_core::Error;

/// Sync option for spawn command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOption {
    /// Do not sync with main
    NoSync,
    /// Sync with main after spawning
    WithSync,
}

impl SyncOption {
    /// Convert bool to SyncOption
    pub fn from_bool(sync: bool) -> Self {
        if sync {
            SyncOption::WithSync
        } else {
            SyncOption::NoSync
        }
    }

    /// Returns true if sync is enabled
    pub fn is_sync(&self) -> bool {
        matches!(self, SyncOption::WithSync)
    }
}

/// Validate workspace name (P1)
/// Returns Some(Error) if invalid, None if valid
/// Enforces regex: ^[a-zA-Z][a-zA-Z0-9_-]*$
pub fn validate_workspace_name(name: &str) -> Option<Error> {
    if name.is_empty() {
        return Some(Error::invalid_identifier("workspace name cannot be empty"));
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    // Must start with a letter
    if !first.is_alphabetic() {
        return Some(Error::invalid_identifier(format!(
            "workspace name must start with a letter, got '{}'",
            name
        )));
    }

    // Remaining chars must be alphanumeric, dash, or underscore
    if !chars.all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Some(Error::invalid_identifier(format!(
            "workspace name must be alphanumeric, dash, or underscore only, got '{}'",
            name
        )));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- validate_workspace_name: valid names ----

    #[test]
    fn valid_simple_name() {
        assert!(validate_workspace_name("main").is_none());
    }

    #[test]
    fn valid_name_with_numbers() {
        assert!(validate_workspace_name("workspace-123").is_none());
    }

    #[test]
    fn valid_name_with_underscore() {
        assert!(validate_workspace_name("my_workspace").is_none());
    }

    #[test]
    fn valid_name_with_dash() {
        assert!(validate_workspace_name("feat-auth").is_none());
    }

    #[test]
    fn valid_single_letter() {
        assert!(validate_workspace_name("a").is_none());
    }

    #[test]
    fn valid_uppercase_start() {
        assert!(validate_workspace_name("FeatureBranch").is_none());
    }

    #[test]
    fn valid_mixed_case() {
        assert!(validate_workspace_name("MyFeature-branch_v2").is_none());
    }

    #[test]
    fn valid_all_uppercase() {
        assert!(validate_workspace_name("ABCDEF").is_none());
    }

    #[test]
    fn valid_trailing_digits() {
        assert!(validate_workspace_name("branch123").is_none());
    }

    #[test]
    fn valid_long_name() {
        let long = "a".repeat(256);
        assert!(validate_workspace_name(&long).is_none());
    }

    // ---- validate_workspace_name: invalid first char ----

    #[test]
    fn reject_empty() {
        let result = validate_workspace_name("");
        assert!(result.is_some());
        let err = result.unwrap();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn reject_starts_with_number() {
        let result = validate_workspace_name("123workspace");
        assert!(result.is_some());
        let err = result.unwrap();
        assert!(err.to_string().contains("letter"));
    }

    #[test]
    fn reject_starts_with_dash() {
        assert!(validate_workspace_name("-workspace").is_some());
    }

    #[test]
    fn reject_starts_with_underscore() {
        assert!(validate_workspace_name("_workspace").is_some());
    }

    // ---- validate_workspace_name: invalid interior chars ----

    #[test]
    fn reject_contains_space() {
        assert!(validate_workspace_name("my workspace").is_some());
    }

    #[test]
    fn reject_contains_at_sign() {
        assert!(validate_workspace_name("workspace@v2").is_some());
    }

    #[test]
    fn reject_contains_dot() {
        assert!(validate_workspace_name("feat.branch").is_some());
    }

    #[test]
    fn reject_contains_slash() {
        assert!(validate_workspace_name("feat/branch").is_some());
    }

    #[test]
    fn reject_special_chars() {
        assert!(validate_workspace_name("feat!branch").is_some());
    }

    #[test]
    fn reject_contains_colon() {
        assert!(validate_workspace_name("feat:branch").is_some());
    }

    #[test]
    fn reject_parentheses() {
        assert!(validate_workspace_name("feat(branch)").is_some());
    }

    #[test]
    fn reject_null_byte() {
        assert!(validate_workspace_name("a\x00b").is_some());
    }

    #[test]
    fn reject_newline() {
        assert!(validate_workspace_name("a\nb").is_some());
    }

    #[test]
    fn reject_tab() {
        assert!(validate_workspace_name("a\tb").is_some());
    }

    // ---- validate_workspace_name: error message quality ----

    #[test]
    fn error_message_contains_input_for_bad_start() {
        let err = validate_workspace_name("123").unwrap();
        assert!(err.to_string().contains("123"));
    }

    #[test]
    fn error_message_contains_input_for_bad_chars() {
        let err = validate_workspace_name("abc def").unwrap();
        assert!(err.to_string().contains("abc def"));
    }

    // ---- SyncOption: construction ----

    #[test]
    fn sync_option_from_bool_true() {
        assert_eq!(SyncOption::from_bool(true), SyncOption::WithSync);
    }

    #[test]
    fn sync_option_from_bool_false() {
        assert_eq!(SyncOption::from_bool(false), SyncOption::NoSync);
    }

    // ---- SyncOption: is_sync ----

    #[test]
    fn sync_option_is_sync_with_sync() {
        assert!(SyncOption::WithSync.is_sync());
    }

    #[test]
    fn sync_option_is_sync_without_sync() {
        assert!(!SyncOption::NoSync.is_sync());
    }

    // ---- SyncOption: equality ----

    #[test]
    fn sync_option_equality() {
        assert_eq!(SyncOption::NoSync, SyncOption::NoSync);
        assert_eq!(SyncOption::WithSync, SyncOption::WithSync);
        assert_ne!(SyncOption::NoSync, SyncOption::WithSync);
    }

    // ---- SyncOption: Clone & Copy ----

    #[test]
    fn sync_option_clone() {
        let a = SyncOption::WithSync;
        assert_eq!(a, a.clone());
    }

    #[test]
    fn sync_option_copy() {
        let a = SyncOption::NoSync;
        let b = a;
        assert_eq!(a, b);
    }

    // ---- SyncOption: Debug ----

    #[test]
    fn sync_option_debug_no_sync() {
        assert_eq!(format!("{:?}", SyncOption::NoSync), "NoSync");
    }

    #[test]
    fn sync_option_debug_with_sync() {
        assert_eq!(format!("{:?}", SyncOption::WithSync), "WithSync");
    }

    // ---- SyncOption: roundtrip ----

    #[test]
    fn from_bool_is_sync_roundtrip() {
        assert!(SyncOption::from_bool(true).is_sync());
        assert!(!SyncOption::from_bool(false).is_sync());
    }
}
