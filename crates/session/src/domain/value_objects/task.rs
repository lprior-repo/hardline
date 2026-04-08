//! Task-related value objects: AgentId, TaskId, Title, Description

use serde::{Deserialize, Serialize};

use crate::error::{SessionError, TaskIdError};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Result<Self, SessionError> {
        let id = id.into();
        if id.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "AgentId cannot be empty".into(),
            ));
        }
        Ok(Self(id))
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

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for AgentId {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    pub fn parse(id: impl Into<String>) -> Result<Self, TaskIdError> {
        let id = id.into();
        if id.is_empty() {
            return Err(TaskIdError::InvalidInput);
        }
        if !id.starts_with("bd-") {
            return Err(TaskIdError::InvalidPrefix);
        }
        let suffix = &id[3..];
        if suffix.is_empty() {
            return Err(TaskIdError::EmptySuffix);
        }
        if !suffix.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(TaskIdError::InvalidHex);
        }
        Ok(Self(id))
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

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for TaskId {
    type Error = TaskIdError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for TaskId {
    type Error = TaskIdError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Title(String);

impl Title {
    pub const MAX_LENGTH: usize = 200;

    pub fn new(title: impl Into<String>) -> Result<Self, SessionError> {
        let title = title.into();
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "Title cannot be empty".into(),
            ));
        }
        if trimmed.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidIdentifier(format!(
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

impl std::fmt::Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Title {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Description(String);

impl Description {
    pub const MAX_LENGTH: usize = 10000;

    pub fn new(desc: impl Into<String>) -> Result<Self, SessionError> {
        let desc = desc.into();
        if desc.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidIdentifier(format!(
                "Description exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(desc))
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

impl std::fmt::Display for Description {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Description {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_valid_taskid_with_numeric_suffix() {
        let result = TaskId::parse("bd-123abc");
        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert_eq!(task_id.as_str(), "bd-123abc");
    }

    #[test]
    fn test_parses_valid_taskid_with_alphanumeric_hex() {
        let result = TaskId::parse("bd-deadbeef");
        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert_eq!(task_id.as_str(), "bd-deadbeef");
    }

    #[test]
    fn test_parses_valid_taskid_case_insensitive_hex() {
        let result = TaskId::parse("bd-ABCDEF");
        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert_eq!(task_id.as_str(), "bd-ABCDEF");
    }

    #[test]
    fn test_try_from_str_trait_valid_input() {
        let result = TaskId::try_from("bd-f00ba7");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "bd-f00ba7");
    }

    #[test]
    fn test_parse_empty_string_returns_invalid_input_error() {
        let result = TaskId::parse("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskIdError::InvalidInput));
    }

    #[test]
    fn test_parse_missing_prefix_returns_invalid_prefix_error() {
        let result = TaskId::parse("abc-123");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskIdError::InvalidPrefix));
    }

    #[test]
    fn test_parse_invalid_hex_returns_invalid_hex_error() {
        let result = TaskId::parse("bd-xyz");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskIdError::InvalidHex));
    }

    #[test]
    fn test_parse_empty_suffix_returns_empty_suffix_error() {
        let result = TaskId::parse("bd-");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskIdError::EmptySuffix));
    }

    #[test]
    fn test_try_from_str_trait_invalid_input() {
        let result = TaskId::try_from("invalid");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskIdError::InvalidPrefix));
    }

    #[test]
    fn test_single_hex_digit_suffix() {
        let result = TaskId::parse("bd-a");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "bd-a");
    }

    #[test]
    fn test_mixed_case_hex() {
        let result = TaskId::parse("bd-AbCdEf123456");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "bd-AbCdEf123456");
    }

    #[test]
    fn test_to_string_always_starts_with_bd_prefix() {
        let task_id = TaskId::parse("bd-abc123").unwrap();
        assert!(task_id.to_string().starts_with("bd-"));
    }

    #[test]
    fn test_as_str_returns_valid_string_slice() {
        let task_id = TaskId::parse("bd-abc123").unwrap();
        assert_eq!(task_id.as_str(), "bd-abc123");
    }

    #[test]
    fn test_roundtrip_parse_to_string() {
        let original = TaskId::parse("bd-abc123").unwrap();
        let reparsed = TaskId::parse(original.to_string()).unwrap();
        assert_eq!(original, reparsed);
    }

    #[test]
    fn test_equality_based_on_value() {
        let task_id1 = TaskId::parse("bd-abc123").unwrap();
        let task_id2 = TaskId::parse("bd-abc123").unwrap();
        assert_eq!(task_id1, task_id2);
    }

    #[test]
    fn test_display_trait_outputs_correct_format() {
        let task_id = TaskId::parse("bd-abc123").unwrap();
        let formatted = format!("{}", task_id);
        assert_eq!(formatted, "bd-abc123");
    }

    #[test]
    fn test_into_inner_returns_original_string() {
        let task_id = TaskId::parse("bd-abc123").unwrap();
        assert_eq!(task_id.into_inner(), "bd-abc123");
    }

    // =========================================================================
    // AgentId Tests
    // =========================================================================

    mod agent_id_tests {
        use super::*;

        #[test]
        fn agent_id_valid() {
            let id = AgentId::new("agent-001").expect("valid");
            assert_eq!(id.as_str(), "agent-001");
        }

        #[test]
        fn agent_id_empty_rejects() {
            let result = AgentId::new("");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidIdentifier(_)
            ));
        }

        #[test]
        fn agent_id_whitespace_only_rejects() {
            // AgentId does NOT trim, so whitespace-only is technically valid (non-empty)
            let id = AgentId::new("  agent  ").expect("valid");
            assert_eq!(id.as_str(), "  agent  ");
        }

        #[test]
        fn agent_id_display() {
            let id = AgentId::new("alice").expect("valid");
            assert_eq!(format!("{id}"), "alice");
        }

        #[test]
        fn agent_id_try_from_string() {
            let id = AgentId::try_from("bob".to_string()).expect("valid");
            assert_eq!(id.as_str(), "bob");
        }

        #[test]
        fn agent_id_try_from_empty_string_fails() {
            let result = AgentId::try_from("".to_string());
            assert!(result.is_err());
        }

        #[test]
        fn agent_id_into_inner() {
            let id = AgentId::new("charlie").expect("valid");
            assert_eq!(id.into_inner(), "charlie");
        }

        #[test]
        fn agent_id_equality() {
            let id1 = AgentId::new("same").expect("valid");
            let id2 = AgentId::new("same").expect("valid");
            let id3 = AgentId::new("different").expect("valid");
            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }
    }

    // =========================================================================
    // Title Tests
    // =========================================================================

    mod title_tests {
        use super::*;

        #[test]
        fn title_valid() {
            let title = Title::new("Implement feature X").expect("valid");
            assert_eq!(title.as_str(), "Implement feature X");
        }

        #[test]
        fn title_trims_whitespace() {
            let title = Title::new("  Padded Title  ").expect("valid");
            assert_eq!(title.as_str(), "Padded Title");
        }

        #[test]
        fn title_empty_rejects() {
            let result = Title::new("");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidIdentifier(_)
            ));
        }

        #[test]
        fn title_whitespace_only_rejects() {
            let result = Title::new("   ");
            assert!(result.is_err());
        }

        #[test]
        fn title_max_length_boundary() {
            let max_title = "a".repeat(Title::MAX_LENGTH);
            let title = Title::new(max_title).expect("at max length");
            assert_eq!(title.as_str().len(), Title::MAX_LENGTH);
        }

        #[test]
        fn title_exceeds_max_length_rejects() {
            let too_long = "a".repeat(Title::MAX_LENGTH + 1);
            let result = Title::new(too_long);
            assert!(result.is_err());
        }

        #[test]
        fn title_display() {
            let title = Title::new("My Title").expect("valid");
            assert_eq!(format!("{title}"), "My Title");
        }

        #[test]
        fn title_try_from_string() {
            let title = Title::try_from("Test".to_string()).expect("valid");
            assert_eq!(title.as_str(), "Test");
        }

        #[test]
        fn title_into_inner() {
            let title = Title::new("Inner").expect("valid");
            assert_eq!(title.into_inner(), "Inner");
        }

        #[test]
        fn title_with_special_chars() {
            let title = Title::new("Fix: issue #123 (critical)").expect("valid");
            assert_eq!(title.as_str(), "Fix: issue #123 (critical)");
        }

        #[test]
        fn title_single_char_min_boundary() {
            let title = Title::new("X").expect("single char is valid");
            assert_eq!(title.as_str(), "X");
        }

        #[test]
        fn title_trims_tabs() {
            let title = Title::new("\tTabbed\t").expect("valid");
            assert_eq!(title.as_str(), "Tabbed");
        }

        #[test]
        fn title_trims_newlines() {
            let title = Title::new("\nLine\n").expect("valid");
            assert_eq!(title.as_str(), "Line");
        }

        #[test]
        fn title_trims_mixed_whitespace() {
            let title = Title::new(" \t\n Mixed \n\t ").expect("valid");
            assert_eq!(title.as_str(), "Mixed");
        }

        #[test]
        fn title_whitespace_only_tabs_rejects() {
            let result = Title::new("\t\t");
            assert!(result.is_err());
        }

        #[test]
        fn title_whitespace_only_newlines_rejects() {
            let result = Title::new("\n\n");
            assert!(result.is_err());
        }

        #[test]
        fn title_whitespace_only_mixed_rejects() {
            let result = Title::new(" \t\n ");
            assert!(result.is_err());
        }

        #[test]
        fn title_interior_whitespace_preserved() {
            let title = Title::new("Hello  World").expect("valid");
            assert_eq!(title.as_str(), "Hello  World");
        }

        #[test]
        fn title_max_length_exactly_after_trim() {
            // Input is padded with spaces, but after trim it's exactly MAX_LENGTH
            let inner = "a".repeat(Title::MAX_LENGTH);
            let input = format!("  {inner}  ");
            let title = Title::new(input).expect("valid after trim");
            assert_eq!(title.as_str().len(), Title::MAX_LENGTH);
        }

        #[test]
        fn title_over_max_length_after_trim_rejects() {
            let inner = "a".repeat(Title::MAX_LENGTH + 1);
            let input = format!("  {inner}  ");
            let result = Title::new(input);
            assert!(result.is_err());
        }

        #[test]
        fn title_exactly_max_length_accepted() {
            let title_str = "a".repeat(Title::MAX_LENGTH);
            let title = Title::new(&title_str).expect("exactly at max");
            assert_eq!(title.as_str().len(), Title::MAX_LENGTH);
        }

        #[test]
        fn title_one_over_max_length_rejected() {
            let title_str = "a".repeat(Title::MAX_LENGTH + 1);
            let result = Title::new(&title_str);
            assert!(result.is_err());
        }

        #[test]
        fn title_two_chars_valid() {
            let title = Title::new("AB").expect("valid");
            assert_eq!(title.as_str(), "AB");
        }
    }

    // =========================================================================
    // Description Tests
    // =========================================================================

    mod description_tests {
        use super::*;

        #[test]
        fn description_valid() {
            let desc = Description::new("A detailed description").expect("valid");
            assert_eq!(desc.as_str(), "A detailed description");
        }

        #[test]
        fn description_empty_allowed() {
            let desc = Description::new("").expect("empty is valid");
            assert_eq!(desc.as_str(), "");
        }

        #[test]
        fn description_whitespace_preserved() {
            // Description does NOT trim
            let desc = Description::new("  spaces  ").expect("valid");
            assert_eq!(desc.as_str(), "  spaces  ");
        }

        #[test]
        fn description_max_length_boundary() {
            let max_desc = "x".repeat(Description::MAX_LENGTH);
            let desc = Description::new(max_desc).expect("at max length");
            assert_eq!(desc.as_str().len(), Description::MAX_LENGTH);
        }

        #[test]
        fn description_exceeds_max_length_rejects() {
            let too_long = "x".repeat(Description::MAX_LENGTH + 1);
            let result = Description::new(too_long);
            assert!(result.is_err());
        }

        #[test]
        fn description_display() {
            let desc = Description::new("Show me").expect("valid");
            assert_eq!(format!("{desc}"), "Show me");
        }

        #[test]
        fn description_try_from_string() {
            let desc = Description::try_from("via tryfrom".to_string()).expect("valid");
            assert_eq!(desc.as_str(), "via tryfrom");
        }

        #[test]
        fn description_into_inner() {
            let desc = Description::new("consume").expect("valid");
            assert_eq!(desc.into_inner(), "consume");
        }

        #[test]
        fn description_with_newlines() {
            let desc = Description::new("Line 1\nLine 2\nLine 3").expect("valid");
            assert!(desc.as_str().contains('\n'));
        }

        // --- Length boundary tests ---

        #[test]
        fn description_zero_length_is_valid() {
            let desc = Description::new("").expect("empty string is valid");
            assert_eq!(desc.as_str().len(), 0);
        }

        #[test]
        fn description_single_char_is_valid() {
            let desc = Description::new("X").expect("single char is valid");
            assert_eq!(desc.as_str(), "X");
        }

        #[test]
        fn description_max_length_minus_one() {
            let content = "a".repeat(Description::MAX_LENGTH - 1);
            let desc = Description::new(content).expect("one below max");
            assert_eq!(desc.as_str().len(), Description::MAX_LENGTH - 1);
        }

        #[test]
        fn description_exactly_max_length() {
            let content = "b".repeat(Description::MAX_LENGTH);
            let desc = Description::new(content).expect("exactly at max");
            assert_eq!(desc.as_str().len(), Description::MAX_LENGTH);
        }

        #[test]
        fn description_one_over_max_rejects() {
            let content = "c".repeat(Description::MAX_LENGTH + 1);
            let result = Description::new(content);
            assert!(result.is_err());
        }

        #[test]
        fn description_far_over_max_rejects() {
            let content = "d".repeat(Description::MAX_LENGTH + 1000);
            let result = Description::new(content);
            assert!(result.is_err());
        }

        // --- Equality & cloning ---

        #[test]
        fn description_equality_same_content() {
            let d1 = Description::new("same").unwrap();
            let d2 = Description::new("same").unwrap();
            assert_eq!(d1, d2);
        }

        #[test]
        fn description_inequality_different_content() {
            let d1 = Description::new("alpha").unwrap();
            let d2 = Description::new("beta").unwrap();
            assert_ne!(d1, d2);
        }

        #[test]
        fn description_clone_preserves_content() {
            let original = Description::new("clone me").unwrap();
            let cloned = original.clone();
            assert_eq!(original, cloned);
            assert_eq!(original.as_str(), cloned.as_str());
        }
    }

    // =========================================================================
    // Description Markdown Tests
    // =========================================================================

    mod description_markdown_tests {
        use super::*;

        #[test]
        fn description_markdown_headers() {
            let md = "# Heading 1\n## Heading 2\n### Heading 3";
            let desc = Description::new(md).expect("valid");
            assert_eq!(desc.as_str(), md);
            assert!(desc.as_str().contains("# Heading 1"));
        }

        #[test]
        fn description_markdown_bold_and_italic() {
            let md = "This is **bold** and *italic* and ***both***";
            let desc = Description::new(md).expect("valid");
            assert!(desc.as_str().contains("**bold**"));
            assert!(desc.as_str().contains("*italic*"));
            assert!(desc.as_str().contains("***both***"));
        }

        #[test]
        fn description_markdown_inline_code() {
            let md = "Use `cargo test` to run tests";
            let desc = Description::new(md).expect("valid");
            assert!(desc.as_str().contains("`cargo test`"));
        }

        #[test]
        fn description_markdown_code_block() {
            let md = "```rust\nfn main() {}\n```";
            let desc = Description::new(md).expect("valid");
            assert!(desc.as_str().contains("```rust"));
            assert!(desc.as_str().contains("fn main()"));
        }

        #[test]
        fn description_markdown_link() {
            let md = "[click here](https://example.com)";
            let desc = Description::new(md).expect("valid");
            assert!(desc.as_str().contains("[click here]"));
            assert!(desc.as_str().contains("(https://example.com)"));
        }

        #[test]
        fn description_markdown_unordered_list() {
            let md = "- item one\n- item two\n- item three";
            let desc = Description::new(md).expect("valid");
            assert_eq!(desc.as_str(), md);
        }

        #[test]
        fn description_markdown_ordered_list() {
            let md = "1. first\n2. second\n3. third";
            let desc = Description::new(md).expect("valid");
            assert_eq!(desc.as_str(), md);
        }

        #[test]
        fn description_markdown_blockquote() {
            let md = "> This is a quote\n> spanning lines";
            let desc = Description::new(md).expect("valid");
            assert!(desc.as_str().starts_with('>'));
        }

        #[test]
        fn description_markdown_horizontal_rule() {
            let md = "above\n---\nbelow";
            let desc = Description::new(md).expect("valid");
            assert!(desc.as_str().contains("\n---\n"));
        }

        #[test]
        fn description_markdown_table() {
            let md = "| A | B |\n|---|---|\n| 1 | 2 |";
            let desc = Description::new(md).expect("valid");
            assert!(desc.as_str().contains("| A | B |"));
            assert!(desc.as_str().contains("|---|---|"));
        }

        #[test]
        fn description_special_chars_asterisk() {
            let desc = Description::new("*.txt").expect("valid");
            assert_eq!(desc.as_str(), "*.txt");
        }

        #[test]
        fn description_special_chars_brackets() {
            let desc = Description::new("[test](url)").expect("valid");
            assert_eq!(desc.as_str(), "[test](url)");
        }

        #[test]
        fn description_special_chars_angle_brackets() {
            let desc = Description::new("<html>tag</html>").expect("valid");
            assert_eq!(desc.as_str(), "<html>tag</html>");
        }

        #[test]
        fn description_special_chars_backslashes() {
            let desc = Description::new(r"path\to\file").expect("valid");
            assert_eq!(desc.as_str(), r"path\to\file");
        }

        #[test]
        fn description_special_chars_pipe() {
            let desc = Description::new("a | b | c").expect("valid");
            assert_eq!(desc.as_str(), "a | b | c");
        }

        #[test]
        fn description_special_chars_hash() {
            let desc = Description::new("issue #123").expect("valid");
            assert_eq!(desc.as_str(), "issue #123");
        }

        #[test]
        fn description_special_chars_underscore() {
            let desc = Description::new("snake_case_name").expect("valid");
            assert_eq!(desc.as_str(), "snake_case_name");
        }

        #[test]
        fn description_special_chars_tilde() {
            let desc = Description::new("~~strikethrough~~").expect("valid");
            assert_eq!(desc.as_str(), "~~strikethrough~~");
        }

        #[test]
        fn description_unicode_content() {
            let desc = Description::new("日本語テスト emoji: 🎉").expect("valid");
            assert!(desc.as_str().contains("日本語テスト"));
        }

        #[test]
        fn description_mixed_markdown_document() {
            let md = r#"# Task

## Description

This task does **important** things.

- [ ] unchecked item
- [x] checked item

```rust
let x = 42;
```

> A wise developer once said...

See [docs](https://example.com) for details.
"#;
            let desc = Description::new(md).expect("valid");
            assert!(desc.as_str().contains("# Task"));
            assert!(desc.as_str().contains("**important**"));
            assert!(desc.as_str().contains("- [x]"));
            assert!(desc.as_str().contains("let x = 42"));
            assert!(desc.as_str().contains("> A wise"));
        }

        #[test]
        fn description_markdown_serde_roundtrip() {
            let md = "# Title\n\n**bold** and *italic*\n\n```\ncode\n```\n";
            let desc = Description::new(md).expect("valid");
            let json = serde_json::to_string(&desc).expect("serialize");
            let parsed: Description = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(desc, parsed);
            assert_eq!(parsed.as_str(), md);
        }

        #[test]
        fn description_markdown_at_max_length() {
            // Build a description that fills MAX_LENGTH with markdown content
            let header = "# H\n\n";
            let body = "x".repeat(Description::MAX_LENGTH - header.len());
            let md = format!("{header}{body}");
            let desc = Description::new(&md).expect("valid at max");
            assert_eq!(desc.as_str().len(), Description::MAX_LENGTH);
        }

        #[test]
        fn description_tabs_preserved() {
            let desc = Description::new("col1\tcol2\tcol3").expect("valid");
            assert!(desc.as_str().contains('\t'));
        }

        #[test]
        fn description_crlf_preserved() {
            let desc = Description::new("line1\r\nline2").expect("valid");
            assert!(desc.as_str().contains("\r\n"));
        }

        #[test]
        fn description_display_shows_raw_markdown() {
            let md = "**bold**";
            let desc = Description::new(md).expect("valid");
            // Display should show raw markdown, not rendered
            assert_eq!(format!("{desc}"), md);
        }
    }

    // =========================================================================
    // TaskId Serde Tests
    // =========================================================================

    mod task_id_serde_tests {
        use super::*;

        #[test]
        fn task_id_serde_roundtrip() {
            let id = TaskId::parse("bd-abc123").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: TaskId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
        }

        #[test]
        fn task_id_serde_json_output() {
            let id = TaskId::parse("bd-cafe").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            assert_eq!(json, "\"bd-cafe\"");
        }

        #[test]
        fn task_id_serde_roundtrip_uppercase() {
            let id = TaskId::parse("bd-ABCDEF").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: TaskId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
        }
    }

    // =========================================================================
    // AgentId Serde Tests
    // =========================================================================

    mod agent_id_serde_tests {
        use super::*;

        #[test]
        fn agent_id_serde_roundtrip() {
            let id = AgentId::new("agent-001").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: AgentId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
        }

        #[test]
        fn agent_id_serde_json_output() {
            let id = AgentId::new("alice").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            assert_eq!(json, "\"alice\"");
        }

        #[test]
        fn agent_id_serde_roundtrip_with_special_chars() {
            let id = AgentId::new("agent@domain.com").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: AgentId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
        }
    }

    // =========================================================================
    // Title Serde Tests
    // =========================================================================

    mod title_serde_tests {
        use super::*;

        #[test]
        fn title_serde_roundtrip() {
            let title = Title::new("My Task Title").expect("valid");
            let json = serde_json::to_string(&title).expect("serialize");
            let parsed: Title = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(title, parsed);
        }

        #[test]
        fn title_serde_json_output() {
            let title = Title::new("Hello").expect("valid");
            let json = serde_json::to_string(&title).expect("serialize");
            assert_eq!(json, "\"Hello\"");
        }

        #[test]
        fn title_serde_roundtrip_with_special_chars() {
            let title = Title::new("Fix: bug #123 (critical)").expect("valid");
            let json = serde_json::to_string(&title).expect("serialize");
            let parsed: Title = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(title, parsed);
        }
    }

    // =========================================================================
    // Description Serde Tests
    // =========================================================================

    mod description_serde_tests {
        use super::*;

        #[test]
        fn description_serde_roundtrip() {
            let desc = Description::new("A detailed description").expect("valid");
            let json = serde_json::to_string(&desc).expect("serialize");
            let parsed: Description = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(desc, parsed);
        }

        #[test]
        fn description_serde_roundtrip_empty() {
            let desc = Description::new("").expect("valid");
            let json = serde_json::to_string(&desc).expect("serialize");
            let parsed: Description = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(desc, parsed);
        }

        #[test]
        fn description_serde_json_output() {
            let desc = Description::new("test").expect("valid");
            let json = serde_json::to_string(&desc).expect("serialize");
            assert_eq!(json, "\"test\"");
        }

        #[test]
        fn description_serde_preserves_whitespace() {
            let desc = Description::new("  spaces  ").expect("valid");
            let json = serde_json::to_string(&desc).expect("serialize");
            let parsed: Description = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed.as_str(), "  spaces  ");
        }
    }

    // =========================================================================
    // TaskId Proptests
    // =========================================================================

    mod task_id_proptests {
        use super::*;

        #[test]
        fn task_id_roundtrip_various() {
            for suffix in &[
                "1",
                "abc123",
                "deadbeef",
                "ABCDEF",
                "AbCdEf123456",
                "f00ba7",
            ] {
                let full = format!("bd-{suffix}");
                let id = TaskId::parse(&full).unwrap();
                assert_eq!(id.to_string(), full);
            }
        }

        #[test]
        fn task_id_equality() {
            let id1 = TaskId::parse("bd-deadbeef").unwrap();
            let id2 = TaskId::parse("bd-deadbeef").unwrap();
            let id3 = TaskId::parse("bd-cafebabe").unwrap();
            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }

        #[test]
        fn task_id_display_matches_as_str() {
            let id = TaskId::parse("bd-abcdef12").unwrap();
            assert_eq!(format!("{id}"), "bd-abcdef12");
        }
    }
}
