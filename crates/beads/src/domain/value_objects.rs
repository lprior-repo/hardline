use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::error::{BeadError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    pub const MAX_LENGTH: usize = 100;

    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.is_empty() {
            return Err(BeadError::InvalidId("ID cannot be empty".into()));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(BeadError::InvalidId(format!(
                "ID exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        if !id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(BeadError::InvalidId(
                "ID must contain only alphanumeric characters, hyphens, and underscores".into(),
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

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for BeadId {
    type Error = BeadError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for BeadId {
    type Error = BeadError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadDescription(String);

impl BeadDescription {
    pub const MAX_LENGTH: usize = 10_000;

    pub fn new(description: impl Into<String>) -> Result<Self> {
        let description = description.into();
        if description.len() > Self::MAX_LENGTH {
            return Err(BeadError::InvalidTitle(format!(
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

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BeadState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed { closed_at: DateTime<Utc> },
}

impl BeadState {
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Open | Self::InProgress)
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }

    #[must_use]
    pub fn closed_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Closed { closed_at } => Some(*closed_at),
            _ => None,
        }
    }

    pub fn transition_to(&self, new_state: Self) -> Result<Self> {
        if matches!(new_state, Self::Closed { .. }) && !matches!(self, Self::Closed { .. }) {
            return Ok(Self::Closed {
                closed_at: Utc::now(),
            });
        }
        Ok(new_state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
    P4,
}

impl Priority {
    #[must_use]
    pub fn value(&self) -> u8 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
            Self::P4 => 4,
        }
    }

    pub fn from_value(value: u8) -> Self {
        match value {
            0 => Self::P0,
            1 => Self::P1,
            2 => Self::P2,
            3 => Self::P3,
            _ => Self::P4,
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.value())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BeadType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Labels(pub Vec<String>);

impl Labels {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with(mut self, label: impl Into<String>) -> Self {
        self.0.push(label.into());
        self
    }

    #[must_use]
    pub fn contains(&self, label: &str) -> bool {
        self.0.iter().any(|l| l == label)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

impl Default for Labels {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── BeadId tests ──────────────────────────────────────────────────────────

    mod bead_id {
        use super::*;

        #[test]
        fn valid_alphanumeric_id() {
            let id = BeadId::new("abc123").unwrap();
            assert_eq!(id.as_str(), "abc123");
        }

        #[test]
        fn valid_id_with_hyphens() {
            let id = BeadId::new("abc-123").unwrap();
            assert_eq!(id.as_str(), "abc-123");
        }

        #[test]
        fn valid_id_with_underscores() {
            let id = BeadId::new("abc_123").unwrap();
            assert_eq!(id.as_str(), "abc_123");
        }

        #[test]
        fn empty_id_is_rejected() {
            let result = BeadId::new("");
            assert!(result.is_err());
            match result.unwrap_err() {
                BeadError::InvalidId(msg) => assert!(msg.contains("empty")),
                other => panic!("expected InvalidId, got {other:?}"),
            }
        }

        #[test]
        fn whitespace_only_id_is_rejected() {
            // Spaces are not alphanumeric or hyphen/underscore, so this fails
            let result = BeadId::new("   ");
            assert!(result.is_err());
        }

        #[test]
        fn id_exceeding_max_length_is_rejected() {
            let long_id = "a".repeat(BeadId::MAX_LENGTH + 1);
            let result = BeadId::new(long_id);
            assert!(result.is_err());
        }

        #[test]
        fn id_at_max_length_is_accepted() {
            let id = BeadId::new("a".repeat(BeadId::MAX_LENGTH)).unwrap();
            assert_eq!(id.as_str().len(), BeadId::MAX_LENGTH);
        }

        #[test]
        fn id_with_spaces_is_rejected() {
            let result = BeadId::new("has spaces");
            assert!(result.is_err());
        }

        #[test]
        fn id_with_special_chars_is_rejected() {
            let result = BeadId::new("has@special#chars");
            assert!(result.is_err());
        }

        #[test]
        fn display_returns_inner_value() {
            let id = BeadId::new("test-id").unwrap();
            assert_eq!(format!("{id}"), "test-id");
        }

        #[test]
        fn into_inner_returns_owned_string() {
            let id = BeadId::new("my-id").unwrap();
            let inner = id.into_inner();
            assert_eq!(inner, "my-id");
        }

        #[test]
        fn try_from_string_works() {
            let id: BeadId = "valid_id".try_into().unwrap();
            assert_eq!(id.as_str(), "valid_id");
        }

        #[test]
        fn try_from_ref_str_works() {
            let id = BeadId::try_from("valid_id").unwrap();
            assert_eq!(id.as_str(), "valid_id");
        }

        #[test]
        fn try_from_invalid_string_fails() {
            let result = BeadId::try_from("bad id!");
            assert!(result.is_err());
        }

        #[test]
        fn equality_works() {
            let a = BeadId::new("same").unwrap();
            let b = BeadId::new("same").unwrap();
            let c = BeadId::new("different").unwrap();
            assert_eq!(a, b);
            assert_ne!(a, c);
        }

        #[test]
        fn hash_works() {
            use std::collections::HashSet;
            let mut set = HashSet::new();
            set.insert(BeadId::new("x").unwrap());
            assert!(set.contains(&BeadId::new("x").unwrap()));
            assert!(!set.contains(&BeadId::new("y").unwrap()));
        }

        #[test]
        fn serde_roundtrip() {
            let id = BeadId::new("serde-test").unwrap();
            let json = serde_json::to_string(&id).unwrap();
            let parsed: BeadId = serde_json::from_str(&json).unwrap();
            assert_eq!(id, parsed);
        }

        #[test]
        fn id_with_single_char_is_accepted() {
            let id = BeadId::new("a").unwrap();
            assert_eq!(id.as_str(), "a");
        }

        #[test]
        fn id_exactly_max_length_plus_one_is_rejected() {
            let long_id: String = "a".repeat(BeadId::MAX_LENGTH);
            let too_long: String = "b".repeat(BeadId::MAX_LENGTH + 1);
            assert!(BeadId::new(long_id).is_ok());
            assert!(BeadId::new(too_long).is_err());
        }

        #[test]
        fn id_with_only_hyphens_is_accepted() {
            let id = BeadId::new("---").unwrap();
            assert_eq!(id.as_str(), "---");
        }

        #[test]
        fn id_with_only_underscores_is_accepted() {
            let id = BeadId::new("___").unwrap();
            assert_eq!(id.as_str(), "___");
        }

        #[test]
        fn id_with_newline_is_rejected() {
            let result = BeadId::new("has\nnewline");
            assert!(result.is_err());
        }

        #[test]
        fn id_with_tab_is_rejected() {
            let result = BeadId::new("has\ttab");
            assert!(result.is_err());
        }

        #[test]
        fn id_with_slash_is_rejected() {
            let result = BeadId::new("a/b/c");
            assert!(result.is_err());
        }

        #[test]
        fn id_with_dot_is_rejected() {
            let result = BeadId::new("a.b.c");
            assert!(result.is_err());
        }
    }

    // ── BeadTitle tests ──────────────────────────────────────────────────────

    mod bead_title {
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
    }

    // ── BeadDescription tests ────────────────────────────────────────────────

    mod bead_description {
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
    }

    // ── BeadState tests ──────────────────────────────────────────────────────

    mod bead_state {
        use super::*;

        #[test]
        fn default_is_open() {
            assert_eq!(BeadState::default(), BeadState::Open);
        }

        #[test]
        fn open_is_active() {
            assert!(BeadState::Open.is_active());
        }

        #[test]
        fn in_progress_is_active() {
            assert!(BeadState::InProgress.is_active());
        }

        #[test]
        fn blocked_is_not_active() {
            assert!(!BeadState::Blocked.is_active());
        }

        #[test]
        fn deferred_is_not_active() {
            assert!(!BeadState::Deferred.is_active());
        }

        #[test]
        fn closed_is_not_active() {
            let closed = BeadState::Closed {
                closed_at: Utc::now(),
            };
            assert!(!closed.is_active());
        }

        #[test]
        fn is_blocked_only_for_blocked() {
            assert!(BeadState::Blocked.is_blocked());
            assert!(!BeadState::Open.is_blocked());
            assert!(!BeadState::InProgress.is_blocked());
            assert!(!BeadState::Deferred.is_blocked());
            let closed = BeadState::Closed {
                closed_at: Utc::now(),
            };
            assert!(!closed.is_blocked());
        }

        #[test]
        fn is_closed_only_for_closed_variant() {
            assert!(BeadState::Closed {
                closed_at: Utc::now()
            }
            .is_closed());
            assert!(!BeadState::Open.is_closed());
            assert!(!BeadState::InProgress.is_closed());
            assert!(!BeadState::Blocked.is_closed());
            assert!(!BeadState::Deferred.is_closed());
        }

        #[test]
        fn closed_at_returns_some_for_closed() {
            let now = Utc::now();
            let state = BeadState::Closed {
                closed_at: now,
            };
            assert_eq!(state.closed_at(), Some(now));
        }

        #[test]
        fn closed_at_returns_none_for_non_closed() {
            assert_eq!(BeadState::Open.closed_at(), None);
            assert_eq!(BeadState::InProgress.closed_at(), None);
            assert_eq!(BeadState::Blocked.closed_at(), None);
            assert_eq!(BeadState::Deferred.closed_at(), None);
        }

        #[test]
        fn transition_to_closed_from_open_succeeds() {
            let result = BeadState::Open.transition_to(BeadState::Closed {
                closed_at: Utc::now(),
            });
            assert!(result.is_ok());
            assert!(result.unwrap().is_closed());
        }

        #[test]
        fn transition_to_same_state_succeeds() {
            let result = BeadState::Open.transition_to(BeadState::Open);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), BeadState::Open);
        }

        #[test]
        fn display_open() {
            assert_eq!(format!("{}", BeadState::Open), "open");
        }

        #[test]
        fn display_in_progress() {
            assert_eq!(format!("{}", BeadState::InProgress), "inprogress");
        }

        #[test]
        fn display_blocked() {
            assert_eq!(format!("{}", BeadState::Blocked), "blocked");
        }

        #[test]
        fn display_deferred() {
            assert_eq!(format!("{}", BeadState::Deferred), "deferred");
        }

        #[test]
        fn display_closed() {
            let state = BeadState::Closed {
                closed_at: Utc::now(),
            };
            assert_eq!(format!("{state}"), "closed");
        }

        #[test]
        fn serde_roundtrip_open() {
            let json = serde_json::to_string(&BeadState::Open).unwrap();
            let parsed: BeadState = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, BeadState::Open);
        }

        #[test]
        fn serde_roundtrip_in_progress() {
            let json = serde_json::to_string(&BeadState::InProgress).unwrap();
            let parsed: BeadState = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, BeadState::InProgress);
        }

        #[test]
        fn serde_roundtrip_closed() {
            let state = BeadState::Closed {
                closed_at: Utc::now(),
            };
            let json = serde_json::to_string(&state).unwrap();
            let parsed: BeadState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, parsed);
        }

        #[test]
        fn from_str_parses_open() {
            let state: BeadState = "open".parse().unwrap();
            assert_eq!(state, BeadState::Open);
        }

        #[test]
        fn from_str_parses_inprogress() {
            let state: BeadState = "inprogress".parse().unwrap();
            assert_eq!(state, BeadState::InProgress);
        }

        #[test]
        fn from_str_parses_blocked() {
            let state: BeadState = "blocked".parse().unwrap();
            assert_eq!(state, BeadState::Blocked);
        }

        #[test]
        fn from_str_parses_deferred() {
            let state: BeadState = "deferred".parse().unwrap();
            assert_eq!(state, BeadState::Deferred);
        }

        #[test]
        fn from_str_parses_closed() {
            let state: BeadState = "closed".parse().unwrap();
            assert!(state.is_closed());
        }

        #[test]
        fn from_str_rejects_invalid() {
            let result: std::result::Result<BeadState, _> = "invalid_state".parse();
            assert!(result.is_err());
        }

        #[test]
        fn serde_roundtrip_blocked() {
            let json = serde_json::to_string(&BeadState::Blocked).unwrap();
            let parsed: BeadState = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, BeadState::Blocked);
        }

        #[test]
        fn serde_roundtrip_deferred() {
            let json = serde_json::to_string(&BeadState::Deferred).unwrap();
            let parsed: BeadState = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, BeadState::Deferred);
        }

        #[test]
        fn closed_state_equality_with_different_timestamps() {
            let now = Utc::now();
            let state1 = BeadState::Closed { closed_at: now };
            let state2 = BeadState::Closed { closed_at: now };
            assert_eq!(state1, state2);
        }

        #[test]
        fn closed_state_inequality_with_different_timestamps() {
            let state1 = BeadState::Closed {
                closed_at: Utc::now(),
            };
            let state2 = BeadState::Closed {
                closed_at: Utc::now() + chrono::Duration::seconds(60),
            };
            assert_ne!(state1, state2);
        }

        #[test]
        fn hash_works() {
            use std::collections::HashSet;
            let mut set = HashSet::new();
            set.insert(BeadState::Open);
            assert!(set.contains(&BeadState::Open));
            assert!(!set.contains(&BeadState::Blocked));
        }

        #[test]
        fn transition_to_non_closed_preserves_state() {
            let result = BeadState::Open.transition_to(BeadState::Blocked);
            // Open -> Blocked is handled by the general Ok(new_state) path
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), BeadState::Blocked);
        }

        #[test]
        fn transition_to_closed_from_in_progress_succeeds() {
            let result = BeadState::InProgress.transition_to(BeadState::Closed {
                closed_at: Utc::now(),
            });
            assert!(result.is_ok());
            let state = result.unwrap();
            assert!(state.is_closed());
            assert!(state.closed_at().is_some());
        }

        #[test]
        fn transition_to_closed_from_blocked_succeeds() {
            let result = BeadState::Blocked.transition_to(BeadState::Closed {
                closed_at: Utc::now(),
            });
            assert!(result.is_ok());
            assert!(result.unwrap().is_closed());
        }

        #[test]
        fn transition_to_closed_from_deferred_succeeds() {
            let result = BeadState::Deferred.transition_to(BeadState::Closed {
                closed_at: Utc::now(),
            });
            assert!(result.is_ok());
            assert!(result.unwrap().is_closed());
        }

        #[test]
        fn transition_from_closed_to_closed_returns_current_time() {
            let past = Utc::now() - chrono::Duration::days(1);
            let result = BeadState::Closed { closed_at: past }
                .transition_to(BeadState::Closed {
                    closed_at: Utc::now() + chrono::Duration::days(365),
                });
            assert!(result.is_ok());
            // Closed -> Closed: doesn't match the special pattern, goes to Ok(new_state)
            // which would be the passed-in Closed state
        }
    }

    // ── Priority tests ───────────────────────────────────────────────────────

    mod priority {
        use super::*;

        #[test]
        fn p0_value_is_zero() {
            assert_eq!(Priority::P0.value(), 0);
        }

        #[test]
        fn p1_value_is_one() {
            assert_eq!(Priority::P1.value(), 1);
        }

        #[test]
        fn p2_value_is_two() {
            assert_eq!(Priority::P2.value(), 2);
        }

        #[test]
        fn p3_value_is_three() {
            assert_eq!(Priority::P3.value(), 3);
        }

        #[test]
        fn p4_value_is_four() {
            assert_eq!(Priority::P4.value(), 4);
        }

        #[test]
        fn from_value_maps_correctly() {
            assert_eq!(Priority::from_value(0), Priority::P0);
            assert_eq!(Priority::from_value(1), Priority::P1);
            assert_eq!(Priority::from_value(2), Priority::P2);
            assert_eq!(Priority::from_value(3), Priority::P3);
        }

        #[test]
        fn from_value_defaults_to_p4_for_unknown() {
            assert_eq!(Priority::from_value(99), Priority::P4);
            assert_eq!(Priority::from_value(255), Priority::P4);
        }

        #[test]
        fn ordering_is_correct() {
            assert!(Priority::P0 < Priority::P1);
            assert!(Priority::P1 < Priority::P2);
            assert!(Priority::P2 < Priority::P3);
            assert!(Priority::P3 < Priority::P4);
        }

        #[test]
        fn display_p0() {
            assert_eq!(format!("{}", Priority::P0), "P0");
        }

        #[test]
        fn display_p4() {
            assert_eq!(format!("{}", Priority::P4), "P4");
        }

        #[test]
        fn equality_works() {
            assert_eq!(Priority::P0, Priority::P0);
            assert_ne!(Priority::P0, Priority::P1);
        }

        #[test]
        fn serde_roundtrip() {
            for p in [Priority::P0, Priority::P1, Priority::P2, Priority::P3, Priority::P4] {
                let json = serde_json::to_string(&p).unwrap();
                let parsed: Priority = serde_json::from_str(&json).unwrap();
                assert_eq!(p, parsed);
            }
        }

        #[test]
        fn serde_serializes_lowercase() {
            let json = serde_json::to_string(&Priority::P0).unwrap();
            assert_eq!(json, "\"p0\"");
        }

        #[test]
        fn serde_deserializes_lowercase() {
            let parsed: Priority = serde_json::from_str("\"p3\"").unwrap();
            assert_eq!(parsed, Priority::P3);
        }

        #[test]
        fn hash_works() {
            use std::collections::HashSet;
            let mut set = HashSet::new();
            set.insert(Priority::P0);
            assert!(set.contains(&Priority::P0));
            assert!(!set.contains(&Priority::P1));
        }

        #[test]
        fn from_value_four_maps_to_p4() {
            assert_eq!(Priority::from_value(4), Priority::P4);
        }

        #[test]
        fn display_p1_through_p3() {
            assert_eq!(format!("{}", Priority::P1), "P1");
            assert_eq!(format!("{}", Priority::P2), "P2");
            assert_eq!(format!("{}", Priority::P3), "P3");
        }

        #[test]
        fn total_ordering() {
            assert!(Priority::P0 < Priority::P1);
            assert!(Priority::P1 < Priority::P2);
            assert!(Priority::P2 < Priority::P3);
            assert!(Priority::P3 < Priority::P4);
            assert!(Priority::P0 <= Priority::P0);
            assert!(Priority::P4 >= Priority::P3);
        }
    }

    // ── BeadType tests ───────────────────────────────────────────────────────

    mod bead_type {
        use super::*;

        #[test]
        fn all_variants_exist() {
            let _ = BeadType::Bug;
            let _ = BeadType::Feature;
            let _ = BeadType::Task;
            let _ = BeadType::Epic;
            let _ = BeadType::Chore;
        }

        #[test]
        fn display_bug() {
            assert_eq!(format!("{}", BeadType::Bug), "bug");
        }

        #[test]
        fn display_feature() {
            assert_eq!(format!("{}", BeadType::Feature), "feature");
        }

        #[test]
        fn display_task() {
            assert_eq!(format!("{}", BeadType::Task), "task");
        }

        #[test]
        fn display_epic() {
            assert_eq!(format!("{}", BeadType::Epic), "epic");
        }

        #[test]
        fn display_chore() {
            assert_eq!(format!("{}", BeadType::Chore), "chore");
        }

        #[test]
        fn from_str_parses_all_variants() {
            assert_eq!("bug".parse::<BeadType>().unwrap(), BeadType::Bug);
            assert_eq!(
                "feature".parse::<BeadType>().unwrap(),
                BeadType::Feature
            );
            assert_eq!("task".parse::<BeadType>().unwrap(), BeadType::Task);
            assert_eq!("epic".parse::<BeadType>().unwrap(), BeadType::Epic);
            assert_eq!("chore".parse::<BeadType>().unwrap(), BeadType::Chore);
        }

        #[test]
        fn from_str_rejects_invalid() {
            let result: std::result::Result<BeadType, _> = "nonexistent".parse();
            assert!(result.is_err());
        }

        #[test]
        fn serde_roundtrip() {
            for bt in [
                BeadType::Bug,
                BeadType::Feature,
                BeadType::Task,
                BeadType::Epic,
                BeadType::Chore,
            ] {
                let json = serde_json::to_string(&bt).unwrap();
                let parsed: BeadType = serde_json::from_str(&json).unwrap();
                assert_eq!(bt, parsed);
            }
        }

        #[test]
        fn serde_serializes_lowercase() {
            let json = serde_json::to_string(&BeadType::Feature).unwrap();
            assert_eq!(json, "\"feature\"");
        }

        #[test]
        fn equality_works() {
            assert_eq!(BeadType::Bug, BeadType::Bug);
            assert_ne!(BeadType::Bug, BeadType::Feature);
        }

        #[test]
        fn hash_works() {
            use std::collections::HashSet;
            let mut set = HashSet::new();
            set.insert(BeadType::Bug);
            assert!(set.contains(&BeadType::Bug));
            assert!(!set.contains(&BeadType::Feature));
        }

        #[test]
        fn debug_format() {
            let debug = format!("{:?}", BeadType::Feature);
            assert!(debug.contains("Feature"));
        }

        #[test]
        fn all_five_variants_are_distinct() {
            let variants = [
                BeadType::Bug,
                BeadType::Feature,
                BeadType::Task,
                BeadType::Epic,
                BeadType::Chore,
            ];
            for i in 0..variants.len() {
                for j in (i + 1)..variants.len() {
                    assert_ne!(variants[i], variants[j]);
                }
            }
        }
    }

    // ── Labels tests ─────────────────────────────────────────────────────────

    mod labels {
        use proptest::proptest;
        use super::*;

        #[test]
        fn new_is_empty() {
            let labels = Labels::new();
            assert!(labels.as_slice().is_empty());
        }

        #[test]
        fn default_is_empty() {
            let labels = Labels::default();
            assert!(labels.as_slice().is_empty());
        }

        #[test]
        fn with_adds_label() {
            let labels = Labels::new().with("bug").with("urgent");
            assert_eq!(labels.as_slice(), &["bug".to_string(), "urgent".to_string()]);
        }

        #[test]
        fn contains_returns_true_for_existing() {
            let labels = Labels::new().with("rust");
            assert!(labels.contains("rust"));
        }

        #[test]
        fn contains_returns_false_for_missing() {
            let labels = Labels::new().with("rust");
            assert!(!labels.contains("go"));
        }

        #[test]
        fn contains_returns_false_for_empty() {
            let labels = Labels::new();
            assert!(!labels.contains("anything"));
        }

        #[test]
        fn equality_works() {
            let a = Labels::new().with("x").with("y");
            let b = Labels::new().with("x").with("y");
            assert_eq!(a, b);
        }

        #[test]
        fn inequality_works() {
            let a = Labels::new().with("x");
            let b = Labels::new().with("y");
            assert_ne!(a, b);
        }

        #[test]
        fn serde_roundtrip() {
            let labels = Labels::new().with("a").with("b").with("c");
            let json = serde_json::to_string(&labels).unwrap();
            let parsed: Labels = serde_json::from_str(&json).unwrap();
            assert_eq!(labels, parsed);
        }

        #[test]
        fn serde_roundtrip_empty() {
            let labels = Labels::new();
            let json = serde_json::to_string(&labels).unwrap();
            let parsed: Labels = serde_json::from_str(&json).unwrap();
            assert_eq!(labels, parsed);
        }

        #[test]
        fn hash_works() {
            use std::collections::HashSet;
            let mut set = HashSet::new();
            set.insert(Labels::new().with("tag-1"));
            assert!(set.contains(&Labels::new().with("tag-1")));
            assert!(!set.contains(&Labels::new().with("tag-2")));
        }

        #[test]
        fn with_accepts_string_ref() {
            let labels = Labels::new().with("ref-label");
            assert!(labels.contains("ref-label"));
        }

        #[test]
        fn with_accepts_string_owned() {
            let label = String::from("owned-label");
            let labels = Labels::new().with(label);
            assert!(labels.contains("owned-label"));
        }

        #[test]
        fn as_slice_length_matches() {
            let labels = Labels::new().with("a").with("b").with("c");
            assert_eq!(labels.as_slice().len(), 3);
        }

        #[test]
        fn contains_empty_label() {
            let labels = Labels::new().with("");
            assert!(labels.contains(""));
        }

        #[test]
        fn with_duplicate_labels() {
            let labels = Labels::new().with("dup").with("dup");
            assert_eq!(labels.as_slice().len(), 2);
        }

        proptest! {
            #[test]
            fn labels_contain_added_string(ref s in ".{1,50}") {
                let labels = Labels::new().with(s.clone());
                assert!(labels.contains(s));
            }

            #[test]
            fn labels_length_matches_additions(ref parts in proptest::collection::vec(".{1,20}", 0..10)) {
                let mut labels = Labels::new();
                for p in parts {
                    labels = labels.with(p);
                }
                assert_eq!(labels.as_slice().len(), parts.len());
            }

            #[test]
            fn labels_empty_when_created(ref s in ".{0}") {
                let labels = Labels::new();
                assert!(labels.as_slice().is_empty());
                assert!(!labels.contains(s));
            }
        }
    }

    // ── Proptests for BeadId ─────────────────────────────────────────────────

    mod proptest_bead_id {
        use proptest::proptest;
        use super::*;

        proptest! {
            #[test]
            fn valid_id_roundtrips(ref s in "[a-zA-Z0-9_-]{1,100}") {
                let id = BeadId::new(s.as_str()).unwrap();
                assert_eq!(id.as_str(), s.as_str());
            }

            #[test]
            fn id_exceeding_max_is_rejected(ref s in ".{101,200}") {
                let result = BeadId::new(s.as_str());
                assert!(result.is_err());
            }

            #[test]
            fn id_with_invalid_chars_rejected(ref s in "[a-zA-Z0-9_-]{0,10}[ @!#.][a-zA-Z0-9_-]{0,10}") {
                if !s.is_empty() && s.chars().any(|c| !c.is_alphanumeric() && c != '-' && c != '_') {
                    let result = BeadId::new(s.as_str());
                    assert!(result.is_err(), "expected rejection for: {:?}", s);
                }
            }
        }
    }

    // ── Proptests for BeadTitle ──────────────────────────────────────────────

    mod proptest_bead_title {
        use proptest::proptest;
        use super::*;

        proptest! {
            #[test]
            fn valid_title_roundtrips(ref s in "[a-zA-Z0-9 ]{1,200}") {
                let result = BeadTitle::new(s.as_str());
                match result {
                    Ok(title) => {
                        // Trimmed version should match
                        assert_eq!(title.as_str(), s.trim());
                        assert!(title.as_str().len() <= BeadTitle::MAX_LENGTH);
                    }
                    Err(_) => {
                        // Only valid if whitespace-only
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

    // ── Proptests for Priority ───────────────────────────────────────────────

    mod proptest_priority {
        use proptest::proptest;
        use super::*;

        proptest! {
            #[test]
            fn from_value_roundtrips(val in 0u8..=4) {
                let p = Priority::from_value(val);
                assert_eq!(p.value(), val);
            }

            #[test]
            fn values_greater_than_four_default_to_p4(val in 5u8..=255) {
                let p = Priority::from_value(val);
                assert_eq!(p, Priority::P4);
            }

            #[test]
            fn priority_ordering_is_total(a in 0u8..=4, b in 0u8..=4) {
                let pa = Priority::from_value(a);
                let pb = Priority::from_value(b);
                // Total ordering: exactly one of <, ==, >
                assert_eq!(pa < pb || pa == pb || pa > pb, true);
            }

            #[test]
            fn priority_ordering_consistent_with_value(a in 0u8..=4, b in 0u8..=4) {
                let pa = Priority::from_value(a);
                let pb = Priority::from_value(b);
                assert_eq!(pa.cmp(&pb), a.cmp(&b));
            }

            #[test]
            fn serde_roundtrip_any_priority(val in 0u8..=4) {
                let p = Priority::from_value(val);
                let json = serde_json::to_string(&p).unwrap();
                let parsed: Priority = serde_json::from_str(&json).unwrap();
                assert_eq!(p, parsed);
            }
        }
    }

    // ── Proptests for BeadState ──────────────────────────────────────────────

    mod proptest_bead_state {
        use proptest::proptest;
        use super::*;

        proptest! {
            #[test]
            fn active_implies_not_closed(state_seed in 0u8..=4) {
                let state = match state_seed {
                    0 => BeadState::Open,
                    1 => BeadState::InProgress,
                    2 => BeadState::Blocked,
                    3 => BeadState::Deferred,
                    _ => BeadState::Closed { closed_at: Utc::now() },
                };
                // If active, not closed (and vice versa)
                if state.is_active() {
                    assert!(!state.is_closed());
                }
                if state.is_closed() {
                    assert!(!state.is_active());
                }
            }

            #[test]
            fn closed_state_always_has_timestamp(state_seed in 0u8..=4) {
                let state = match state_seed {
                    0 => BeadState::Open,
                    1 => BeadState::InProgress,
                    2 => BeadState::Blocked,
                    3 => BeadState::Deferred,
                    _ => BeadState::Closed { closed_at: Utc::now() },
                };
                assert_eq!(state.is_closed(), state.closed_at().is_some());
            }
        }
    }

    // ── Proptests for BeadDescription ────────────────────────────────────────

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

    // ── Proptests for BeadType ───────────────────────────────────────────────

    mod proptest_bead_type {
        use proptest::proptest;
        use super::*;

        proptest! {
            #[test]
            fn serde_roundtrip_any_variant(seed in 0u8..=4) {
                let bt = match seed {
                    0 => BeadType::Bug,
                    1 => BeadType::Feature,
                    2 => BeadType::Task,
                    3 => BeadType::Epic,
                    _ => BeadType::Chore,
                };
                let json = serde_json::to_string(&bt).unwrap();
                let parsed: BeadType = serde_json::from_str(&json).unwrap();
                assert_eq!(bt, parsed);
            }
        }
    }
}