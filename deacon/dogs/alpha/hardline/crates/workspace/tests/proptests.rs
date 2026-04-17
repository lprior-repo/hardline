//! Proptest invariants for workspace crate value objects

use proptest::prelude::*;
use scp_workspace::domain::value_objects::branch_name::BranchName;
use scp_workspace::domain::value_objects::workspace_name::WorkspaceName;

proptest! {
    #[test]
    fn proptest_workspace_name_roundtrip(
        input in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,62}",
    ) {
        let name = WorkspaceName::new(input.clone())?;
        prop_assert_eq!(name.as_str(), &input);
    }

    #[test]
    fn proptest_workspace_name_reflexive(
        input in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,62}",
    ) {
        let name = WorkspaceName::new(input)?;
        prop_assert_eq!(&name, &name.clone());
    }

    #[test]
    fn proptest_workspace_name_rejects_invalid_chars(
        prefix in "[a-zA-Z0-9_-]{0,10}",
        bad_char in "[^a-zA-Z0-9_-]",
        suffix in "[a-zA-Z0-9_-]{0,10}",
    ) {
        let input = format!("{}{}{}", prefix, bad_char, suffix);
        if input.is_empty() || input.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Ok(());
        }
        prop_assert!(WorkspaceName::new(input.clone()).is_err(), "should reject: {:?}", input);
    }

    #[test]
    fn proptest_workspace_name_too_long_rejected(
        input in "[a-zA-Z0-9_-]{256,500}",
    ) {
        prop_assert!(WorkspaceName::new(input).is_err());
    }
}

#[test]
fn proptest_workspace_name_empty_rejected() {
    assert!(WorkspaceName::new("".into()).is_err());
}

proptest! {
    #[test]
    fn proptest_workspace_branch_name_roundtrip(
        input in "[a-zA-Z0-9/_.-]{1,100}",
    ) {
        let name = BranchName::new(input.clone())?;
        prop_assert_eq!(name.as_str(), &input);
    }

    #[test]
    fn proptest_workspace_branch_name_reflexive(
        input in "[a-zA-Z0-9/_.-]{1,100}",
    ) {
        let name = BranchName::new(input)?;
        prop_assert_eq!(&name, &name.clone());
    }

    #[test]
    fn proptest_workspace_branch_name_null_rejected(
        prefix in "[a-zA-Z0-9/_.-]{0,10}",
        suffix in "[a-zA-Z0-9/_.-]{0,10}",
    ) {
        let input = format!("{}{}{}", prefix, '\0', suffix);
        prop_assert!(BranchName::new(input).is_err());
    }
}

#[test]
fn proptest_workspace_branch_name_empty_rejected() {
    assert!(BranchName::new("".into()).is_err());
}
