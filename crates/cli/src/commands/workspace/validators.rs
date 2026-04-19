//! Workspace name validators

use scp_core::Error;

/// Validate workspace name (P1)
/// Returns Some(Error) if invalid, None if valid
/// Enforces regex: `^[a-zA-Z][a-zA-Z0-9_-]*$`
#[must_use]
pub fn validate_workspace_name(name: &str) -> Option<Error> {
    if name.is_empty() {
        return Some(Error::invalid_identifier("workspace name cannot be empty"));
    }

    let mut chars = name.chars();
    let first = chars.next()?;

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
    use proptest::prelude::*;

    // ---- Valid names ----

    #[test]
    fn valid_simple_name() {
        assert!(validate_workspace_name("abc").is_none());
    }

    #[test]
    fn valid_single_letter() {
        assert!(validate_workspace_name("a").is_none());
    }

    #[test]
    fn valid_uppercase_start() {
        assert!(validate_workspace_name("Zebra").is_none());
    }

    #[test]
    fn valid_with_numbers() {
        assert!(validate_workspace_name("abc123").is_none());
    }

    #[test]
    fn valid_with_dash() {
        assert!(validate_workspace_name("abc-def").is_none());
    }

    #[test]
    fn valid_with_underscore() {
        assert!(validate_workspace_name("abc_def").is_none());
    }

    #[test]
    fn valid_mixed_all_allowed_chars() {
        assert!(validate_workspace_name("abc-def_123").is_none());
    }

    #[test]
    fn valid_long_name() {
        let long = "a".repeat(256);
        assert!(validate_workspace_name(&long).is_none());
    }

    #[test]
    fn valid_all_uppercase() {
        assert!(validate_workspace_name("ABCDEF").is_none());
    }

    #[test]
    fn valid_mixed_case() {
        assert!(validate_workspace_name("MyFeature-branch_v2").is_none());
    }

    #[test]
    fn valid_trailing_digits() {
        assert!(validate_workspace_name("branch123").is_none());
    }

    #[test]
    fn valid_only_dashes_after_first() {
        assert!(validate_workspace_name("a-b-c").is_none());
    }

    #[test]
    fn valid_only_underscores_after_first() {
        assert!(validate_workspace_name("a_b_c").is_none());
    }

    // ---- Invalid: empty ----

    #[test]
    fn reject_empty() {
        let result = validate_workspace_name("");
        assert!(result.is_some());
        assert!(result.unwrap().to_string().contains("empty"));
    }

    // ---- Invalid: first character ----

    #[test]
    fn reject_starts_with_digit() {
        let result = validate_workspace_name("123invalid");
        assert!(result.is_some());
        assert!(result.unwrap().to_string().contains("letter"));
    }

    #[test]
    fn reject_starts_with_dash() {
        assert!(validate_workspace_name("-workspace").is_some());
    }

    #[test]
    fn reject_starts_with_underscore() {
        assert!(validate_workspace_name("_workspace").is_some());
    }

    #[test]
    fn reject_starts_with_special() {
        assert!(validate_workspace_name("@invalid").is_some());
    }

    #[test]
    fn reject_starts_with_space() {
        assert!(validate_workspace_name(" workspace").is_some());
    }

    #[test]
    fn reject_starts_with_dot() {
        assert!(validate_workspace_name(".hidden").is_some());
    }

    #[test]
    fn reject_starts_with_slash() {
        assert!(validate_workspace_name("/path").is_some());
    }

    #[test]
    fn reject_starts_with_colon() {
        assert!(validate_workspace_name(":name").is_some());
    }

    // ---- Invalid: interior characters ----

    #[test]
    fn reject_space_in_middle() {
        assert!(validate_workspace_name("abc def").is_some());
    }

    #[test]
    fn reject_at_sign() {
        assert!(validate_workspace_name("abc@def").is_some());
    }

    #[test]
    fn reject_exclamation() {
        assert!(validate_workspace_name("valid-name!").is_some());
    }

    #[test]
    fn reject_dot() {
        assert!(validate_workspace_name("feat.branch").is_some());
    }

    #[test]
    fn reject_slash() {
        assert!(validate_workspace_name("feat/branch").is_some());
    }

    #[test]
    fn reject_colon() {
        assert!(validate_workspace_name("feat:branch").is_some());
    }

    #[test]
    fn reject_hash() {
        assert!(validate_workspace_name("abc@#$%").is_some());
    }

    #[test]
    fn reject_parentheses() {
        assert!(validate_workspace_name("feat(branch)").is_some());
    }

    #[test]
    fn reject_equals() {
        assert!(validate_workspace_name("a=b").is_some());
    }

    #[test]
    fn reject_plus() {
        assert!(validate_workspace_name("a+b").is_some());
    }

    #[test]
    fn reject_tab() {
        assert!(validate_workspace_name("a\tb").is_some());
    }

    #[test]
    fn reject_newline() {
        assert!(validate_workspace_name("a\nb").is_some());
    }

    #[test]
    fn reject_null_byte() {
        assert!(validate_workspace_name("a\x00b").is_some());
    }

    #[test]
    fn reject_trailing_special() {
        assert!(validate_workspace_name("valid!").is_some());
    }

    // ---- Invalid: Unicode ----

    #[test]
    fn reject_emoji() {
        // Emoji are not alphanumeric
        assert!(validate_workspace_name("branch🎉").is_some());
    }

    #[test]
    fn accept_unicode_alphanumeric() {
        // Rust's is_alphabetic/is_alphanumeric accepts Unicode — this is valid
        assert!(validate_workspace_name("branch名").is_none());
    }

    #[test]
    fn accept_unicode_first_char() {
        // Unicode letters pass is_alphabetic
        assert!(validate_workspace_name("über").is_none());
    }

    // ---- Error message quality ----

    #[test]
    fn error_message_contains_input_for_bad_start() {
        let err = validate_workspace_name("123").unwrap();
        let msg = err.to_string();
        assert!(msg.contains("123"), "Error should mention the input: {msg}");
    }

    #[test]
    fn error_message_contains_input_for_bad_chars() {
        let err = validate_workspace_name("abc def").unwrap();
        let msg = err.to_string();
        assert!(
            msg.contains("abc def"),
            "Error should mention the input: {msg}"
        );
    }

    // ---- Proptests ----

    proptest! {
        #[test]
        fn proptest_valid_lowercase_start_names(name in "[a-z][a-zA-Z0-9_-]{0,20}") {
            prop_assert!(validate_workspace_name(&name).is_none(),
                "Valid name '{}' was rejected", name);
        }

        #[test]
        fn proptest_valid_uppercase_start_names(name in "[A-Z][a-zA-Z0-9_-]{0,20}") {
            prop_assert!(validate_workspace_name(&name).is_none(),
                "Valid name '{}' was rejected", name);
        }

        #[test]
        fn proptest_digit_start_always_rejected(rest in "[a-zA-Z0-9_-]{0,20}") {
            let name = format!("1{}", rest);
            prop_assert!(validate_workspace_name(&name).is_some(),
                "Name starting with digit should be rejected: '{}'", name);
        }

        #[test]
        fn proptest_dash_start_always_rejected(rest in "[a-zA-Z0-9_-]{0,20}") {
            let name = format!("-{}", rest);
            prop_assert!(validate_workspace_name(&name).is_some(),
                "Name starting with dash should be rejected: '{}'", name);
        }

        #[test]
        fn proptest_underscore_start_always_rejected(rest in "[a-zA-Z0-9_-]{0,20}") {
            let name = format!("_{}", rest);
            prop_assert!(validate_workspace_name(&name).is_some(),
                "Name starting with underscore should be rejected: '{}'", name);
        }

        #[test]
        fn proptest_space_in_name_rejected(prefix in "[a-zA-Z][a-zA-Z0-9_-]{0,10}", suffix in "[a-zA-Z0-9_-]{0,10}") {
            let name = format!("{} {}", prefix, suffix);
            prop_assert!(validate_workspace_name(&name).is_some(),
                "Name with space should be rejected: '{}'", name);
        }

        #[test]
        fn proptest_valid_name_is_deterministic(name in "[a-z][a-zA-Z0-9_-]{0,20}") {
            let r1 = validate_workspace_name(&name).is_some();
            let r2 = validate_workspace_name(&name).is_some();
            prop_assert_eq!(r1, r2);
        }

        #[test]
        fn proptest_any_nonempty_string_has_consistent_validation(s in ".{1,50}") {
            let r1 = validate_workspace_name(&s).is_some();
            let r2 = validate_workspace_name(&s).is_some();
            prop_assert_eq!(r1, r2);
        }
    }
}
