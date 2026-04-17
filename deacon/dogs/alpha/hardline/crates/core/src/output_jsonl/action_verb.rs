//! Action verb enumeration
//!
//! # Design Principles
//!
//! Following Scott Wlaschin's DDD principles:
//! - **Make illegal states unrepresentable** - Enum prevents arbitrary verbs
//! - **Parse at boundaries** - Validate action verbs at construction

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};

use super::OutputLineError;

/// Known action verbs in the system
///
/// These are predefined action verbs that represent operations.
/// Custom verbs can be added via the `Custom` variant for extensibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionVerb {
    /// Run a command or operation
    Run,
    /// Execute a task
    Execute,
    /// Create a new resource
    Create,
    /// Delete a resource
    Delete,
    /// Update a resource
    Update,
    /// Merge resources
    Merge,
    /// Rebase changes
    Rebase,
    /// Sync with remote
    Sync,
    /// Fix an issue
    Fix,
    /// Check status
    Check,
    /// Focus on a target
    Focus,
    /// Attach to a session
    Attach,
    /// Switch tabs
    SwitchTab,
    /// Remove a resource
    Remove,
    /// Discover resources
    Discover,
    /// Would fix (dry run)
    WouldFix,
    /// Custom action verb with string value
    #[serde(untagged)]
    Custom(String),
}

impl ActionVerb {
    /// Create an action verb from known verbs or validate custom format
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::InvalidActionVerb` if custom verb doesn't
    /// follow the pattern: lowercase alphanumeric with hyphens (e.g., "run", "switch-tab")
    pub fn new(verb: impl Into<String>) -> Result<Self, OutputLineError> {
        let verb = verb.into();

        // Match against known verbs (case-insensitive)
        match verb.to_lowercase().as_str() {
            "run" => Ok(Self::Run),
            "execute" => Ok(Self::Execute),
            "create" => Ok(Self::Create),
            "delete" => Ok(Self::Delete),
            "update" => Ok(Self::Update),
            "merge" => Ok(Self::Merge),
            "rebase" => Ok(Self::Rebase),
            "sync" => Ok(Self::Sync),
            "fix" => Ok(Self::Fix),
            "check" => Ok(Self::Check),
            "focus" => Ok(Self::Focus),
            "attach" => Ok(Self::Attach),
            "switch-tab" => Ok(Self::SwitchTab),
            "remove" => Ok(Self::Remove),
            "discovered" => Ok(Self::Discover),
            "would_fix" => Ok(Self::WouldFix),
            custom => {
                // Validate custom verb format
                if custom.trim().is_empty() {
                    return Err(OutputLineError::InvalidActionVerb(
                        "action verb cannot be empty".to_string(),
                    ));
                }

                // Must be lowercase alphanumeric with hyphens
                let lower = custom.to_lowercase();
                if lower != custom {
                    return Err(OutputLineError::InvalidActionVerb(format!(
                        "action verb must be lowercase, got: {custom}"
                    )));
                }

                if !lower
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                {
                    return Err(OutputLineError::InvalidActionVerb(format!(
                        "action verb must be lowercase alphanumeric with hyphens, got: {custom}"
                    )));
                }

                // Must start with a letter
                if !lower.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
                    return Err(OutputLineError::InvalidActionVerb(format!(
                        "action verb must start with a lowercase letter, got: {custom}"
                    )));
                }

                Ok(Self::Custom(lower))
            }
        }
    }

    /// Get the action verb as a string slice
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Run => "run",
            Self::Execute => "execute",
            Self::Create => "create",
            Self::Delete => "delete",
            Self::Update => "update",
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::Sync => "sync",
            Self::Fix => "fix",
            Self::Check => "check",
            Self::Focus => "focus",
            Self::Attach => "attach",
            Self::SwitchTab => "switch-tab",
            Self::Remove => "remove",
            Self::Discover => "discovered",
            Self::WouldFix => "would_fix",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Check if this is a custom action verb
    #[must_use]
    pub const fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }
}

impl fmt::Display for ActionVerb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AsRef<str> for ActionVerb {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
