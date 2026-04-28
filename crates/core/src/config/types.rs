#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Configuration types for VCS, Auth, and Conflict Resolution

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{error_config::ConfigErrorKind, Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ForgeType {
    #[default]
    Github,
    Gitlab,
    Gitea,
}

impl ForgeType {
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::Gitea => "gitea",
        }
    }
}

impl FromStr for ForgeType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "github" => Ok(Self::Github),
            "gitlab" => Ok(Self::Gitlab),
            "gitea" => Ok(Self::Gitea),
            _ => Err(ConfigErrorKind::Invalid(format!("Invalid forge type: {s}")).into()),
        }
    }
}

impl std::fmt::Display for ForgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthSourceType {
    StaxGithubTokenEnv,
    CredentialsFile,
    #[default]
    GhCli,
    GithubTokenEnv,
}

impl AuthSourceType {
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::StaxGithubTokenEnv => "STAX_GITHUB_TOKEN",
            Self::CredentialsFile => "credentials_file",
            Self::GhCli => "gh_cli",
            Self::GithubTokenEnv => "GITHUB_TOKEN",
        }
    }
}

impl FromStr for AuthSourceType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "stax_github_token_env" | "stax" => Ok(Self::StaxGithubTokenEnv),
            "credentials_file" | "credentials" => Ok(Self::CredentialsFile),
            "gh_cli" | "gh" => Ok(Self::GhCli),
            "github_token_env" | "github_token" => Ok(Self::GithubTokenEnv),
            _ => Err(ConfigErrorKind::Invalid(format!("Invalid auth source: {s}")).into()),
        }
    }
}

impl std::fmt::Display for AuthSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictMode {
    Auto,
    #[default]
    Manual,
    Hybrid,
}

impl std::fmt::Display for ConflictMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Manual => write!(f, "manual"),
            Self::Hybrid => write!(f, "hybrid"),
        }
    }
}

impl FromStr for ConflictMode {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            "hybrid" => Ok(Self::Hybrid),
            _ => Err(ConfigErrorKind::Invalid(format!("Invalid conflict mode: {s}")).into()),
        }
    }
}

/// A boolean that strictly rejects non-boolean types during deserialization.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ValidatedBool(bool);

impl ValidatedBool {
    #[must_use]
    pub const fn new(value: bool) -> Self {
        Self(value)
    }
    #[must_use]
    pub const fn value(self) -> bool {
        self.0
    }
}

impl From<bool> for ValidatedBool {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<ValidatedBool> for bool {
    fn from(value: ValidatedBool) -> Self {
        value.0
    }
}

impl std::ops::Deref for ValidatedBool {
    type Target = bool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Not for ValidatedBool {
    type Output = bool;
    fn not(self) -> Self::Output {
        !self.0
    }
}

impl std::fmt::Display for ValidatedBool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for ValidatedBool {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bool(self.0)
    }
}

impl<'de> Deserialize<'de> for ValidatedBool {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BoolVisitor;
        impl serde::de::Visitor<'_> for BoolVisitor {
            type Value = ValidatedBool;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a boolean value")
            }
            fn visit_bool<E>(self, value: bool) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(ValidatedBool(value))
            }
        }
        deserializer.deserialize_bool(BoolVisitor)
    }
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*;

    #[test]
    fn test_validated_bool_success() {
        let val: ValidatedBool = serde_json::from_str("true").expect("should parse");
        assert!(val.value());
    }

    #[test]
    fn test_validated_bool_rejections() {
        let res: std::result::Result<ValidatedBool, _> = serde_json::from_str("\"true\"");
        assert!(res.is_err());
    }
}
