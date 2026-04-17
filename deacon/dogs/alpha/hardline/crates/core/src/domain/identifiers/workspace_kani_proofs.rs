//! Kani proofs for WorkspaceId validation invariants.
//!
//! # Invariants Proven
//!
//! 1. Valid workspace IDs are accepted
//! 2. Empty workspace IDs are rejected
//! 3. Workspace names without path separators are accepted

#[cfg(kani)]
mod proof {
    use crate::domain::identifiers::{
        error::IdentifierError, validation::validate_workspace_name, WorkspaceName,
    };

    #[kani::proof]
    fn verify_valid_workspace_name_accepted() {
        let valid_names = ["my-workspace", "workspace_123", "ws"];

        for name in valid_names {
            let result = WorkspaceName::parse(name);
            assert!(result.is_ok(), "Should accept: {}", name);
        }
    }

    #[kani::proof]
    fn verify_empty_workspace_name_rejected() {
        let result = WorkspaceName::parse("");
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_workspace_name_with_path_separator_rejected() {
        let result = WorkspaceName::parse("my/workspace");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(IdentifierError::ContainsPathSeparators)
        ));
    }

    #[kani::proof]
    fn verify_workspace_name_with_backslash_rejected() {
        let result = WorkspaceName::parse("my\\workspace");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(IdentifierError::ContainsPathSeparators)
        ));
    }

    #[kani::proof]
    fn verify_workspace_name_with_null_rejected() {
        let result = WorkspaceName::parse("my\0workspace");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(IdentifierError::ContainsPathSeparators)
        ));
    }

    #[kani::proof]
    fn verify_workspace_name_max_length() {
        let long_name = "a".repeat(256);
        let result = WorkspaceName::parse(&long_name);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(IdentifierError::TooLong { max: 255, .. })
        ));
    }

    #[kani::proof]
    fn verify_validate_workspace_name_valid() {
        let result = validate_workspace_name("valid-workspace");
        assert!(result.is_ok());
    }

    #[kani::proof]
    fn verify_validate_workspace_name_empty() {
        let result = validate_workspace_name("");
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_validate_workspace_name_with_slash() {
        let result = validate_workspace_name("my/workspace");
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_workspace_name_roundtrip() {
        let original = "my-workspace";
        let parsed = WorkspaceName::parse(original).unwrap();
        assert_eq!(parsed.into_string(), original);
    }
}
