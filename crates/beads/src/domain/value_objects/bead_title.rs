use serde::{Deserialize, Serialize};

use crate::error::{BeadError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadTitle(String);

impl BeadTitle {
    pub const MAX_LENGTH: usize = 200;

    pub fn new(title: impl Into<String>) -> Result<Self> {
        let title = title.into();
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(BeadError::InvalidTitle("Title cannot be empty".into()));
        }
        if trimmed.len() > Self::MAX_LENGTH {
            return Err(BeadError::InvalidTitle(format!(
                "Title exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for BeadTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for BeadTitle {
    type Error = BeadError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for BeadTitle {
    type Error = BeadError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Self::new(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_title() {
        let title = BeadTitle::new("A valid title").unwrap();
        assert_eq!(title.as_str(), "A valid title");
    }

    #[test]
    fn empty_title_is_rejected() {
        let result = BeadTitle::new("");
        assert!(result.is_err());
    }

    #[test]
    fn whitespace_only_title_is_rejected() {
        let result = BeadTitle::new("   ");
        assert!(result.is_err());
    }

    #[test]
    fn title_is_trimmed() {
        let title = BeadTitle::new("  padded  ").unwrap();
        assert_eq!(title.as_str(), "padded");
    }

    #[test]
    fn title_exceeding_max_length_is_rejected() {
        let long_title = "x".repeat(BeadTitle::MAX_LENGTH + 1);
        let result = BeadTitle::new(long_title);
        assert!(result.is_err());
    }

    #[test]
    fn title_at_max_length_is_accepted() {
        let title = BeadTitle::new("x".repeat(BeadTitle::MAX_LENGTH)).unwrap();
        assert_eq!(title.as_str().len(), BeadTitle::MAX_LENGTH);
    }

    #[test]
    fn display_returns_inner_value() {
        let title = BeadTitle::new("My Title").unwrap();
        assert_eq!(format!("{title}"), "My Title");
    }

    #[test]
    fn into_inner_returns_owned_string() {
        let title = BeadTitle::new("test").unwrap();
        let inner = title.into_inner();
        assert_eq!(inner, "test");
    }

    #[test]
    fn try_from_string_works() {
        let title: BeadTitle = BeadTitle::try_from("Hello".to_string()).unwrap();
        assert_eq!(title.as_str(), "Hello");
    }

    #[test]
    fn try_from_empty_string_fails() {
        let result = BeadTitle::try_from(String::new());
        assert!(result.is_err());
    }

    #[test]
    fn equality_works() {
        let a = BeadTitle::new("same").unwrap();
        let b = BeadTitle::new("same").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn serde_roundtrip() {
        let title = BeadTitle::new("Serialized Title").unwrap();
        let json = serde_json::to_string(&title).unwrap();
        let parsed: BeadTitle = serde_json::from_str(&json).unwrap();
        assert_eq!(title, parsed);
    }

    #[test]
    fn title_with_single_char_is_accepted() {
        let title = BeadTitle::new("A").unwrap();
        assert_eq!(title.as_str(), "A");
    }

    #[test]
    fn title_with_newline_is_accepted() {
        let title = BeadTitle::new("line\nbreak").unwrap();
        assert!(title.as_str().contains('\n'));
    }

    #[test]
    fn title_with_tab_is_accepted() {
        let title = BeadTitle::new("tab\there").unwrap();
        assert!(title.as_str().contains('\t'));
    }

    #[test]
    fn title_only_whitespace_rejected() {
        let result = BeadTitle::new("\t\n  ");
        assert!(result.is_err());
    }

    #[test]
    fn inequality_works() {
        let a = BeadTitle::new("alpha").unwrap();
        let b = BeadTitle::new("beta").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn single_char_is_min_boundary() {
        let title = BeadTitle::new("X").unwrap();
        assert_eq!(title.as_str(), "X");
        assert_eq!(title.as_str().len(), 1);
    }

    #[test]
    fn max_length_minus_one_accepted() {
        let s = "a".repeat(BeadTitle::MAX_LENGTH - 1);
        let title = BeadTitle::new(&s).unwrap();
        assert_eq!(title.as_str().len(), BeadTitle::MAX_LENGTH - 1);
    }

    #[test]
    fn exactly_max_length_accepted() {
        let s = "a".repeat(BeadTitle::MAX_LENGTH);
        let title = BeadTitle::new(&s).unwrap();
        assert_eq!(title.as_str().len(), BeadTitle::MAX_LENGTH);
    }

    #[test]
    fn max_length_plus_one_rejected() {
        let s = "a".repeat(BeadTitle::MAX_LENGTH + 1);
        let result = BeadTitle::new(&s);
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::InvalidTitle(msg) => {
                assert!(msg.contains("maximum length"));
            }
            other => panic!("expected InvalidTitle, got {other:?}"),
        }
    }

    #[test]
    fn empty_string_rejected() {
        let result = BeadTitle::new("");
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::InvalidTitle(msg) => assert!(msg.contains("empty")),
            other => panic!("expected InvalidTitle, got {other:?}"),
        }
    }

    #[test]
    fn spaces_only_rejected() {
        let result = BeadTitle::new("   ");
        assert!(result.is_err());
    }

    #[test]
    fn tabs_only_rejected() {
        let result = BeadTitle::new("\t\t");
        assert!(result.is_err());
    }

    #[test]
    fn newlines_only_rejected() {
        let result = BeadTitle::new("\n\n");
        assert!(result.is_err());
    }

    #[test]
    fn mixed_whitespace_only_rejected() {
        let result = BeadTitle::new(" \t\n\r ");
        assert!(result.is_err());
    }

    #[test]
    fn leading_whitespace_trimmed() {
        let title = BeadTitle::new("   hello").unwrap();
        assert_eq!(title.as_str(), "hello");
    }

    #[test]
    fn trailing_whitespace_trimmed() {
        let title = BeadTitle::new("hello   ").unwrap();
        assert_eq!(title.as_str(), "hello");
    }

    #[test]
    fn both_sides_trimmed() {
        let title = BeadTitle::new("  hello world  ").unwrap();
        assert_eq!(title.as_str(), "hello world");
    }

    #[test]
    fn internal_whitespace_preserved() {
        let title = BeadTitle::new("hello   world").unwrap();
        assert_eq!(title.as_str(), "hello   world");
    }

    #[test]
    fn internal_tab_preserved() {
        let title = BeadTitle::new("hello\tworld").unwrap();
        assert!(title.as_str().contains('\t'));
    }

    #[test]
    fn internal_newline_preserved() {
        let title = BeadTitle::new("hello\nworld").unwrap();
        assert!(title.as_str().contains('\n'));
    }

    #[test]
    fn title_trimmed_too_long_still_rejected() {
        let content = "a".repeat(BeadTitle::MAX_LENGTH + 1);
        let padded = format!("  {}  ", content);
        let result = BeadTitle::new(&padded);
        assert!(result.is_err());
    }

    #[test]
    fn whitespace_padded_max_length_still_ok() {
        let content = "a".repeat(BeadTitle::MAX_LENGTH);
        let padded = format!("  {}  ", content);
        let title = BeadTitle::new(&padded).unwrap();
        assert_eq!(title.as_str().len(), BeadTitle::MAX_LENGTH);
    }

    #[test]
    fn whitespace_padded_over_max_rejected() {
        let content = "a".repeat(BeadTitle::MAX_LENGTH + 1);
        let padded = format!("  {}  ", content);
        let result = BeadTitle::new(&padded);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_ref_str_whitespace_trimmed() {
        let title = BeadTitle::try_from("  trimmed  ").unwrap();
        assert_eq!(title.as_str(), "trimmed");
    }

    #[test]
    fn try_from_ref_str_empty_fails() {
        let result = BeadTitle::try_from("");
        assert!(result.is_err());
    }

    #[test]
    fn try_from_ref_str_whitespace_only_fails() {
        let result = BeadTitle::try_from("   ");
        assert!(result.is_err());
    }

    #[test]
    fn unicode_content_accepted() {
        let title = BeadTitle::new("日本語タイトル").unwrap();
        assert_eq!(title.as_str(), "日本語タイトル");
    }

    #[test]
    fn emoji_in_title_accepted() {
        let title = BeadTitle::new("Fix 🔥 bug").unwrap();
        assert_eq!(title.as_str(), "Fix 🔥 bug");
    }

    #[test]
    fn unicode_multibyte_over_max_rejected() {
        let s = "é".repeat(101);
        let result = BeadTitle::new(&s);
        assert!(result.is_err());
    }

    #[test]
    fn unicode_multibyte_at_max_accepted() {
        let s = "é".repeat(100);
        let result = BeadTitle::new(&s);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str().len(), BeadTitle::MAX_LENGTH);
    }

    #[test]
    fn clone_works() {
        let a = BeadTitle::new("cloned").unwrap();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn debug_format() {
        let title = BeadTitle::new("debug me").unwrap();
        let debug = format!("{:?}", title);
        assert!(debug.contains("BeadTitle"));
    }

    #[test]
    fn hash_works() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BeadTitle::new("hash-test").unwrap());
        assert!(set.contains(&BeadTitle::new("hash-test").unwrap()));
        assert!(!set.contains(&BeadTitle::new("other").unwrap()));
    }

    #[test]
    fn error_message_empty_title() {
        let result = BeadTitle::new("");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("empty"),
            "error message should mention 'empty': {msg}"
        );
    }

    #[test]
    fn error_message_over_max_length() {
        let s = "a".repeat(BeadTitle::MAX_LENGTH + 1);
        let result = BeadTitle::new(&s);
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("200"),
            "error message should mention max length 200: {msg}"
        );
    }

    #[test]
    fn max_length_constant_is_200() {
        assert_eq!(BeadTitle::MAX_LENGTH, 200);
    }

    #[test]
    fn carriage_return_only_rejected() {
        let result = BeadTitle::new("\r\r");
        assert!(result.is_err());
    }

    #[test]
    fn vertical_whitespace_mixed_rejected() {
        let result = BeadTitle::new(" \n \t \r ");
        assert!(result.is_err());
    }

    #[test]
    fn single_space_rejected() {
        let result = BeadTitle::new(" ");
        assert!(result.is_err());
    }

    #[test]
    fn title_with_special_chars_accepted() {
        let title = BeadTitle::new("Fix: auth@prod (v2.0) [urgent] {blocking}").unwrap();
        assert_eq!(title.as_str(), "Fix: auth@prod (v2.0) [urgent] {blocking}");
    }

    #[test]
    fn title_trim_surrounding_preserves_inner() {
        let title = BeadTitle::new("  hello   world  ").unwrap();
        assert_eq!(title.as_str(), "hello   world");
    }

    mod proptest_bead_title {
        use super::*;
        use proptest::proptest;

        proptest! {
            #[test]
            fn valid_title_roundtrips(ref s in "[a-zA-Z0-9 ]{1,200}") {
                let result = BeadTitle::new(s.as_str());
                match result {
                    Ok(title) => {
                        assert_eq!(title.as_str(), s.trim());
                        assert!(title.as_str().len() <= BeadTitle::MAX_LENGTH);
                    }
                    Err(_) => {
                        assert!(s.trim().is_empty(), "non-whitespace-only title was rejected: {:?}", s);
                    }
                }
            }

            #[test]
            fn title_max_boundary(max_len in 196..=200u32) {
                let s = "x".repeat(max_len as usize);
                let result = BeadTitle::new(s.as_str());
                assert!(result.is_ok(), "title of length {} should be accepted", max_len);
            }

            #[test]
            fn title_over_max_rejected(over_len in 201..=300u32) {
                let s = "x".repeat(over_len as usize);
                let result = BeadTitle::new(s.as_str());
                assert!(result.is_err(), "title of length {} should be rejected", over_len);
            }
        }
    }
}
