//! Domain types for CLI contracts.
//!
//! This module contains semantic newtypes that make illegal states unrepresentable.
//! Following Scott Wlaschin's DDD principles:
//! - Parse at boundaries, validate once
//! - Use semantic newtypes instead of primitives
//! - Make illegal states unrepresentable with enums
//!
//! # `SessionName` Consolidation
//!
//! This module now re-exports `domain::SessionName` as the single source of truth.
//! The previous implementation with `new_unchecked()` has been removed to prevent
//! bypassing validation.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{fmt::Display, str::FromStr};

use crate::cli_contracts::ContractError;
// Re-export AgentId from domain layer (single source of truth)
//
// The domain::identifiers module provides the canonical AgentId implementation
// with comprehensive validation (1-128 chars, alphanumeric plus hyphen, underscore,
// dot, and colon). This re-export maintains API compatibility while consolidating
// to a single validated implementation.
pub use crate::domain::AgentId;
// ═══════════════════════════════════════════════════════════════════════════
// IDENTIFIER NEWTYPES
// ═══════════════════════════════════════════════════════════════════════════

// Re-export SessionName from domain layer (single source of truth)
//
// The domain::identifiers module provides the canonical SessionName implementation
// following DDD principles. This re-export maintains API compatibility while
// consolidating to a single validated implementation.
pub use crate::domain::SessionName;

// Provide conversion from ContractError for CLI contract usage
impl AgentId {
    /// Parse with `ContractError` (for CLI contract compatibility)
    ///
    /// # Errors
    ///
    /// Returns `ContractError` if the agent ID is invalid.
    pub fn try_parse_contract(s: impl Into<String>) -> Result<Self, ContractError> {
        let s = s.into();
        Self::parse(s).map_err(|e| ContractError::invalid_input("agent_id", e.to_string()))
    }
}

// Provide conversion from ContractError for CLI contract usage
impl SessionName {
    /// Parse with `ContractError` (for CLI contract compatibility)
    ///
    /// # Errors
    ///
    /// Returns `ContractError` if the name is invalid.
    pub fn try_parse_contract(s: impl Into<String>) -> Result<Self, ContractError> {
        let s = s.into();
        Self::parse(s).map_err(|e| ContractError::invalid_input("name", e.to_string()))
    }
}

// ═══════════════════════════════════════════════════════════════════════════

/// A validated task identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(String);

impl TaskId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for TaskId {
    type Error = ContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(ContractError::invalid_input("task_id", "cannot be empty"));
        }
        Ok(Self(value.to_string()))
    }
}

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════

/// A validated configuration key.
///
/// Must be a dotted path like `session.max_count`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigKey(String);

impl ConfigKey {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate a config key.
    ///
    /// # Errors
    ///
    /// Returns `ContractError::InvalidInput` if:
    /// - The key is empty
    /// - The key is not a dotted path (fewer than 2 segments)
    /// - Any segment is empty
    /// - Any segment contains non-alphanumeric characters (excluding underscore)
    pub fn validate(key: &str) -> Result<(), ContractError> {
        if key.is_empty() {
            return Err(ContractError::invalid_input("key", "cannot be empty"));
        }

        // Key should be a dotted path like "session.max_count"
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() < 2 {
            return Err(ContractError::invalid_input(
                "key",
                "must be a dotted path (e.g., 'section.key')",
            ));
        }

        for part in parts {
            if part.is_empty() {
                return Err(ContractError::invalid_input(
                    "key",
                    "cannot have empty segments",
                ));
            }
            if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(ContractError::invalid_input(
                    "key",
                    "segments must contain only alphanumeric and underscore",
                ));
            }
        }

        Ok(())
    }
}

impl TryFrom<&str> for ConfigKey {
    type Error = ContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value.to_string()))
    }
}

impl Display for ConfigKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════

/// A validated configuration value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValue(String);

impl ConfigValue {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate a config value.
    ///
    /// # Errors
    ///
    /// Returns `ContractError::InvalidInput` if the value is empty.
    pub fn validate(value: &str) -> Result<(), ContractError> {
        if value.is_empty() {
            return Err(ContractError::invalid_input("value", "cannot be empty"));
        }
        Ok(())
    }
}

impl TryFrom<&str> for ConfigValue {
    type Error = ContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value.to_string()))
    }
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE ENUMS - Make illegal states unrepresentable
// ═══════════════════════════════════════════════════════════════════════════

/// Session status state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionStatus {
    Creating,
    Active,
    Paused,
    Completed,
    Failed,
}

impl SessionStatus {
    /// Check if a state transition is valid.
    #[must_use]
    pub const fn can_transition_to(self, to: Self) -> bool {
        matches!(
            (self, to),
            (Self::Creating, Self::Active | Self::Failed)
                | (Self::Active, Self::Paused | Self::Completed)
                | (Self::Paused, Self::Active | Self::Completed)
        )
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Creating => "creating",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for SessionStatus {
    type Err = ContractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "creating" => Ok(Self::Creating),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(ContractError::invalid_input(
                "status",
                "must be one of: creating, active, paused, completed, failed",
            )),
        }
    }
}

impl Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════════════════

/// Agent status state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

impl AgentStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Timeout => "timeout",
        }
    }
}

impl FromStr for AgentStatus {
    type Err = ContractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "timeout" => Ok(Self::Timeout),
            _ => Err(ContractError::invalid_input(
                "status",
                "must be one of: pending, running, completed, failed, cancelled, timeout",
            )),
        }
    }
}

impl Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════════════════

/// Task status state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    Open,
    InProgress,
    Blocked,
    Closed,
}

impl TaskStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Closed => "closed",
        }
    }
}

impl FromStr for TaskStatus {
    type Err = ContractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(Self::Open),
            "in_progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "closed" => Ok(Self::Closed),
            _ => Err(ContractError::invalid_input(
                "status",
                "must be one of: open, in_progress, blocked, closed",
            )),
        }
    }
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════════════════

/// Task priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskPriority {
    P0,
    P1,
    P2,
    P3,
    P4,
}

impl TaskPriority {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
            Self::P4 => "P4",
        }
    }
}

impl FromStr for TaskPriority {
    type Err = ContractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "P0" => Ok(Self::P0),
            "P1" => Ok(Self::P1),
            "P2" => Ok(Self::P2),
            "P3" => Ok(Self::P3),
            "P4" => Ok(Self::P4),
            _ => Err(ContractError::invalid_input(
                "priority",
                "must be one of: P0, P1, P2, P3, P4",
            )),
        }
    }
}

impl Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════════════════

/// Configuration scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigScope {
    Local,
    Global,
    System,
}

impl ConfigScope {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Global => "global",
            Self::System => "system",
        }
    }
}

impl FromStr for ConfigScope {
    type Err = ContractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(Self::Local),
            "global" => Ok(Self::Global),
            "system" => Ok(Self::System),
            _ => Err(ContractError::invalid_input(
                "scope",
                "must be one of: local, global, system",
            )),
        }
    }
}

impl Display for ConfigScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// VALUE OBJECTS - Replace repeated primitives with semantic types
// ═══════════════════════════════════════════════════════════════════════════

/// A non-empty string that has been trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate a non-empty string.
    ///
    /// # Errors
    ///
    /// Returns `ContractError::InvalidInput` if the string is empty or contains only whitespace.
    pub fn validate(s: &str) -> Result<(), ContractError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ContractError::invalid_input(
                "string",
                "cannot be empty or whitespace",
            ));
        }
        Ok(())
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = ContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value.trim().to_string()))
    }
}

impl Display for NonEmptyString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════

/// A validated limit value (1..=1000).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Limit(u16);

impl Limit {
    #[must_use]
    pub const fn value(self) -> usize {
        self.0 as usize
    }

    /// Validate a limit value.
    ///
    /// # Errors
    ///
    /// Returns `ContractError::InvalidInput` if:
    /// - The limit is 0
    /// - The limit exceeds 1000
    pub fn validate(limit: usize) -> Result<(), ContractError> {
        if limit == 0 {
            return Err(ContractError::invalid_input("limit", "must be at least 1"));
        }
        if limit > 1000 {
            return Err(ContractError::invalid_input("limit", "cannot exceed 1000"));
        }
        Ok(())
    }
}

impl TryFrom<usize> for Limit {
    type Error = ContractError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(u16::try_from(value).map_err(|_| {
            ContractError::invalid_input("limit", "value too large for internal representation")
        })?))
    }
}

// ═══════════════════════════════════════════════════════════════════════════

/// A validated priority value (0..=1000, where 0 is highest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Priority(u16);

impl Priority {
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0 as u32
    }

    /// Validate a priority value.
    ///
    /// # Errors
    ///
    /// Returns `ContractError::InvalidInput` if the priority exceeds 1000.
    pub fn validate(priority: u32) -> Result<(), ContractError> {
        if priority > 1000 {
            return Err(ContractError::invalid_input(
                "priority",
                "must be between 0 and 1000",
            ));
        }
        Ok(())
    }
}

impl TryFrom<u32> for Priority {
    type Error = ContractError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(u16::try_from(value).map_err(|_| {
            ContractError::invalid_input("priority", "value too large for internal representation")
        })?))
    }
}

// ═══════════════════════════════════════════════════════════════════════════

/// A validated timeout value in seconds (1..=86400, i.e., 24 hours).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeoutSeconds(u64);

impl TimeoutSeconds {
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    const MIN: u64 = 1;
    const MAX: u64 = 24 * 60 * 60; // 24 hours

    /// Validate a timeout value.
    ///
    /// # Errors
    ///
    /// Returns `ContractError::InvalidInput` if:
    /// - The timeout is less than 1 second
    /// - The timeout exceeds 24 hours (86400 seconds)
    pub fn validate(timeout: u64) -> Result<(), ContractError> {
        if timeout < Self::MIN {
            return Err(ContractError::invalid_input(
                "timeout",
                "must be at least 1 second",
            ));
        }
        if timeout > Self::MAX {
            return Err(ContractError::invalid_input(
                "timeout",
                "cannot exceed 24 hours",
            ));
        }
        Ok(())
    }
}

impl TryFrom<u64> for TimeoutSeconds {
    type Error = ContractError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_name_valid() {
        assert!(SessionName::try_from("valid-name").is_ok());
        assert!(SessionName::try_from("Feature_Auth").is_ok());
        assert!(SessionName::try_from("a").is_ok());
        assert!(SessionName::try_from("test123").is_ok());
    }

    #[test]
    fn test_session_name_trims_whitespace() {
        // Consolidated version trims whitespace
        let name = SessionName::parse("  valid-name  ").expect("valid");
        assert_eq!(name.as_str(), "valid-name");
    }

    #[test]
    fn test_session_name_invalid() {
        assert!(SessionName::try_from("").is_err());
        assert!(SessionName::try_from("1invalid").is_err());
        assert!(SessionName::try_from("-invalid").is_err());
        assert!(SessionName::try_from("_invalid").is_err());
        assert!(SessionName::try_from("invalid name").is_err());
        assert!(SessionName::try_from("invalid@name").is_err());
    }

    #[test]
    fn test_session_status_transitions() {
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));
        assert!(!SessionStatus::Completed.can_transition_to(SessionStatus::Active));
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::P0 < TaskPriority::P1);
        assert!(TaskPriority::P1 < TaskPriority::P2);
        assert!(TaskPriority::P2 < TaskPriority::P3);
        assert!(TaskPriority::P3 < TaskPriority::P4);
    }

    #[test]
    fn test_limit_validation() {
        assert!(Limit::try_from(1).is_ok());
        assert!(Limit::try_from(1000).is_ok());
        assert!(Limit::try_from(0).is_err());
        assert!(Limit::try_from(1001).is_err());
    }

    #[test]
    fn test_timeout_validation() {
        assert!(TimeoutSeconds::try_from(1).is_ok());
        assert!(TimeoutSeconds::try_from(3600).is_ok());
        assert!(TimeoutSeconds::try_from(86400).is_ok());
        assert!(TimeoutSeconds::try_from(0).is_err());
        assert!(TimeoutSeconds::try_from(86401).is_err());
    }
}

// Additional integration tests are in domain_tests.rs
#[cfg(test)]
mod integration_tests {
    // Tests are in a separate file to avoid cluttering this module
}
