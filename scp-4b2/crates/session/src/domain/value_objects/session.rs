use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IdentifierError {
    #[error("identifier cannot be empty")]
    Empty,
    #[error("identifier too long: {0} characters (max {1})")]
    TooLong(usize, usize),
    #[error("identifier contains invalid characters: {0}")]
    InvalidCharacters(String),
    #[error("identifier must start with a letter")]
    InvalidStart,
    #[error("identifier must be ASCII only")]
    NotAscii,
}

pub type SessionNameError = IdentifierError;
pub type WorkspaceIdError = IdentifierError;
pub type BeadIdError = IdentifierError;

fn validate_session_name(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::Empty);
    }
    if s.len() > 63 {
        return Err(IdentifierError::TooLong(s.len(), 63));
    }
    if !s.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return Err(IdentifierError::InvalidStart);
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(IdentifierError::InvalidCharacters(
            "must contain only letters, numbers, hyphens, or underscores".into(),
        ));
    }
    Ok(())
}

fn validate_hex_id(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::Empty);
    }
    if !s.starts_with("bd-") {
        return Err(IdentifierError::InvalidCharacters(
            "must start with 'bd-'".into(),
        ));
    }
    let hex_part = &s[3..];
    if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(IdentifierError::InvalidCharacters(
            "must be valid hex after 'bd-'".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionName(String);

impl SessionName {
    pub const MAX_LENGTH: usize = 63;

    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        let trimmed = s.trim();
        validate_session_name(trimmed)?;
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SessionName {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        if s.is_empty() {
            return Err(IdentifierError::Empty);
        }
        Ok(Self(s))
    }

    pub fn generate() -> Self {
        Self(format!("ws-{}", uuid::Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_hex_id(&s)?;
        Ok(Self(s))
    }

    pub fn generate() -> Self {
        let hex = format!("{:x}", uuid::Uuid::new_v4());
        Self(format!("bd-{}", &hex[..12]))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_session_value_object_name_adversarial(s in ".*") {
            let res = SessionName::parse(s.clone());
            let trimmed = s.trim();

            if let Ok(name) = res {
                let name_str = name.as_str();
                prop_assert!(!name_str.is_empty(), "Empty string allowed");
                prop_assert!(name_str.len() <= SessionName::MAX_LENGTH, "Max length exceeded: {} > {}", name_str.len(), SessionName::MAX_LENGTH);

                let first_char = name_str.chars().next().unwrap();
                prop_assert!(first_char.is_ascii_alphabetic(), "First char not ascii alphabetic: {}", first_char);

                let valid_chars = name_str.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
                prop_assert!(valid_chars, "Contains invalid chars: {}", name_str);
            } else {
                // If it failed, it must violate one of the rules.
                let violates_rules = trimmed.is_empty()
                    || trimmed.len() > SessionName::MAX_LENGTH
                    || !trimmed.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
                    || !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

                prop_assert!(violates_rules, "Valid string was rejected: {:?}", s);
            }
        }
    }

    #[test]
    fn test_valid_session_name() {
        assert!(SessionName::parse("my-session").is_ok());
        assert!(SessionName::parse("test_123").is_ok());
    }

    #[test]
    fn test_invalid_session_name_empty() {
        assert!(SessionName::parse("").is_err());
    }

    #[test]
    fn test_valid_bead_id() {
        assert!(BeadId::parse("bd-abc123").is_ok());
    }

    #[test]
    fn test_invalid_bead_id_no_prefix() {
        assert!(BeadId::parse("abc123").is_err());
    }
}
