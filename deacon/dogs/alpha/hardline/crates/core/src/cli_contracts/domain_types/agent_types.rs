//! Agent and output type definitions for CLI contracts.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt::Display;
use std::str::FromStr;

use crate::cli_contracts::ContractError;

/// Agent type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentType {
    Claude,
    Cursor,
    Aider,
    Copilot,
}

impl AgentType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::Aider => "aider",
            Self::Copilot => "copilot",
        }
    }
}

impl FromStr for AgentType {
    type Err = ContractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "claude" => Ok(Self::Claude),
            "cursor" => Ok(Self::Cursor),
            "aider" => Ok(Self::Aider),
            "copilot" => Ok(Self::Copilot),
            _ => Err(ContractError::invalid_input(
                "agent_type",
                "must be one of: claude, cursor, aider, copilot",
            )),
        }
    }
}

impl Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    Text,
    Json,
    Yaml,
}

impl OutputFormat {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Yaml => "yaml",
        }
    }
}

impl FromStr for OutputFormat {
    type Err = ContractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            "yaml" => Ok(Self::Yaml),
            _ => Err(ContractError::invalid_input(
                "format",
                "must be one of: text, json, yaml",
            )),
        }
    }
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// File status in diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

impl FileStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Modified => "M",
            Self::Added => "A",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Untracked => "?",
        }
    }
}

impl FromStr for FileStatus {
    type Err = ContractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "M" => Ok(Self::Modified),
            "A" => Ok(Self::Added),
            "D" => Ok(Self::Deleted),
            "R" => Ok(Self::Renamed),
            "?" => Ok(Self::Untracked),
            _ => Err(ContractError::invalid_input(
                "status",
                "must be one of: M, A, D, R, ?",
            )),
        }
    }
}

impl Display for FileStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
