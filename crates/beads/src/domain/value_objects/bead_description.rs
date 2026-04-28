use serde::{Deserialize, Serialize};

use crate::error::{BeadError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadDescription(String);

impl BeadDescription {
    pub const MAX_LENGTH: usize = 10_000;

    pub fn new(description: impl Into<String>) -> Result<Self> {
        let description = description.into();
        if description.len() > Self::MAX_LENGTH {
            return Err(BeadError::InvalidDescription(format!(
                "Description exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(description))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for BeadDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for BeadDescription {
    type Error = BeadError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_description() {
        let desc = BeadDescription::new("Some description").unwrap();
        assert_eq!(desc.as_str(), "Some description");
    }

    #[test]
    fn empty_description_is_accepted() {
        let desc = BeadDescription::new("").unwrap();
        assert!(desc.is_empty());
    }

    #[test]
    fn is_empty_returns_true_for_empty() {
        let desc = BeadDescription::new("").unwrap();
        assert!(desc.is_empty());
    }

    #[test]
    fn is_empty_returns_false_for_non_empty() {
        let desc = BeadDescription::new("not empty").unwrap();
        assert!(!desc.is_empty());
    }

    #[test]
    fn description_is_not_trimmed() {
        let desc = BeadDescription::new("  padded  ").unwrap();
        assert_eq!(desc.as_str(), "  padded  ");
    }

    #[test]
    fn description_exceeding_max_length_is_rejected() {
        let long_desc = "x".repeat(BeadDescription::MAX_LENGTH + 1);
        let result = BeadDescription::new(long_desc);
        assert!(result.is_err());
    }

    #[test]
    fn description_at_max_length_is_accepted() {
        let desc = BeadDescription::new("x".repeat(BeadDescription::MAX_LENGTH)).unwrap();
        assert_eq!(desc.as_str().len(), BeadDescription::MAX_LENGTH);
    }

    #[test]
    fn display_returns_inner_value() {
        let desc = BeadDescription::new("test desc").unwrap();
        assert_eq!(format!("{desc}"), "test desc");
    }

    #[test]
    fn into_inner_returns_owned_string() {
        let desc = BeadDescription::new("inner").unwrap();
        let inner = desc.into_inner();
        assert_eq!(inner, "inner");
    }

    #[test]
    fn try_from_string_works() {
        let desc: BeadDescription = BeadDescription::try_from("test".to_string()).unwrap();
        assert_eq!(desc.as_str(), "test");
    }

    #[test]
    fn serde_roundtrip() {
        let desc = BeadDescription::new("A detailed description").unwrap();
        let json = serde_json::to_string(&desc).unwrap();
        let parsed: BeadDescription = serde_json::from_str(&json).unwrap();
        assert_eq!(desc, parsed);
    }

    #[test]
    fn serde_roundtrip_empty() {
        let desc = BeadDescription::new("").unwrap();
        let json = serde_json::to_string(&desc).unwrap();
        let parsed: BeadDescription = serde_json::from_str(&json).unwrap();
        assert_eq!(desc, parsed);
    }

    #[test]
    fn hash_works() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BeadDescription::new("desc-a").unwrap());
        assert!(set.contains(&BeadDescription::new("desc-a").unwrap()));
        assert!(!set.contains(&BeadDescription::new("desc-b").unwrap()));
    }

    #[test]
    fn equality_works() {
        let a = BeadDescription::new("same").unwrap();
        let b = BeadDescription::new("same").unwrap();
        let c = BeadDescription::new("different").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn try_from_empty_string_succeeds() {
        let desc = BeadDescription::try_from(String::new()).unwrap();
        assert!(desc.is_empty());
    }

    #[test]
    fn description_with_unicode_is_accepted() {
        let desc = BeadDescription::new("Hello 世界").unwrap();
        assert!(desc.as_str().contains("世界"));
    }

    #[test]
    fn description_with_newlines_is_accepted() {
        let desc = BeadDescription::new("line1\nline2\nline3").unwrap();
        assert_eq!(desc.as_str().lines().count(), 3);
    }

    #[test]
    fn markdown_headers_preserved() {
        let md = "# Heading 1\n## Heading 2\n### Heading 3";
        let desc = BeadDescription::new(md).unwrap();
        assert_eq!(desc.as_str(), md);
    }

    #[test]
    fn markdown_bold_italic_preserved() {
        let md = "This is **bold** and *italic* and ***both***.";
        let desc = BeadDescription::new(md).unwrap();
        assert_eq!(desc.as_str(), md);
    }

    #[test]
    fn markdown_code_blocks_preserved() {
        let md = "Inline `code` and:\n```\nfenced block\n```";
        let desc = BeadDescription::new(md).unwrap();
        assert!(desc.as_str().contains("`code`"));
        assert!(desc.as_str().contains("```\nfenced block\n```"));
    }

    #[test]
    fn markdown_links_preserved() {
        let md = "[link text](https://example.com)";
        let desc = BeadDescription::new(md).unwrap();
        assert_eq!(desc.as_str(), md);
    }

    #[test]
    fn markdown_lists_preserved() {
        let md = "- item one\n- item two\n  - nested\n1. ordered";
        let desc = BeadDescription::new(md).unwrap();
        assert_eq!(desc.as_str(), md);
    }

    #[test]
    fn markdown_mixed_content() {
        let md = "# Title\n\nParagraph with **bold** and `code`.\n\n- list item\n- [link](url)\n\n> blockquote";
        let desc = BeadDescription::new(md).unwrap();
        assert_eq!(desc.as_str(), md);
    }

    #[test]
    fn empty_string_is_some_not_none() {
        let desc = BeadDescription::new("");
        assert!(desc.is_ok());
        let desc = desc.unwrap();
        assert!(desc.is_empty());
        assert_eq!(desc.as_str(), "");
    }

    #[test]
    fn none_is_not_a_bead_description() {
        let none_desc: Option<BeadDescription> = None;
        let empty_desc: Option<BeadDescription> = Some(BeadDescription::new("").unwrap());
        assert!(none_desc.is_none());
        assert!(empty_desc.is_some());
        assert_ne!(none_desc, empty_desc);
    }

    #[test]
    fn option_some_empty_vs_option_some_content() {
        let none_like: Option<BeadDescription> = Some(BeadDescription::new("").unwrap());
        let some_content: Option<BeadDescription> =
            Some(BeadDescription::new("actual content").unwrap());
        assert!(none_like.is_some());
        assert!(some_content.is_some());
        assert_ne!(none_like, some_content);
        assert!(none_like.as_ref().unwrap().is_empty());
        assert!(!some_content.as_ref().unwrap().is_empty());
    }

    #[test]
    fn clone_produces_equal_value() {
        let desc = BeadDescription::new("clonable").unwrap();
        let cloned = desc.clone();
        assert_eq!(desc, cloned);
    }

    #[test]
    fn debug_output_contains_inner() {
        let desc = BeadDescription::new("visible in debug").unwrap();
        let debug_str = format!("{desc:?}");
        assert!(debug_str.contains("visible in debug"));
    }

    #[test]
    fn try_from_rejects_oversized() {
        let long = "x".repeat(BeadDescription::MAX_LENGTH + 1);
        let result = BeadDescription::try_from(long);
        assert!(result.is_err());
    }

    #[test]
    fn serde_roundtrip_at_max_length() {
        let desc = BeadDescription::new("x".repeat(BeadDescription::MAX_LENGTH)).unwrap();
        let json = serde_json::to_string(&desc).unwrap();
        let parsed: BeadDescription = serde_json::from_str(&json).unwrap();
        assert_eq!(desc, parsed);
    }

    #[test]
    fn serde_roundtrip_with_markdown() {
        let md = "# Title\n**bold** `code`\n- item";
        let desc = BeadDescription::new(md).unwrap();
        let json = serde_json::to_string(&desc).unwrap();
        let parsed: BeadDescription = serde_json::from_str(&json).unwrap();
        assert_eq!(desc, parsed);
        assert_eq!(parsed.as_str(), md);
    }

    #[test]
    fn serde_deserializes_empty_string() {
        let json = r#""""#;
        let desc: BeadDescription = serde_json::from_str(json).unwrap();
        assert!(desc.is_empty());
    }

    mod proptest_bead_description {
        use proptest::proptest;

        use super::*;

        proptest! {
            #[test]
            fn valid_description_roundtrips(len in 0..=10000u32) {
                let s = "a".repeat(len as usize);
                let desc = BeadDescription::new(s.as_str()).unwrap();
                assert_eq!(desc.as_str().len(), len as usize);
                assert_eq!(desc.is_empty(), len == 0);
            }

            #[test]
            fn description_over_max_rejected(over_len in 10001..=10100u32) {
                let s = "a".repeat(over_len as usize);
                let result = BeadDescription::new(s.as_str());
                assert!(result.is_err());
            }
        }
    }
}
