#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Conflict resolution types
//!
//! This module provides types for conflict resolution behavior.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error_config::ConfigErrorKind;
use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// CONFLICT MODE ENUM
// ═══════════════════════════════════════════════════════════════════════════

/// Conflict resolution mode
///
/// Defines how conflicts are resolved:
/// - **Auto**: Fully automatic resolution by AI
/// - **Manual**: All conflicts require human intervention
/// - **Hybrid**: AI auto-resolves safe conflicts based on autonomy and keywords
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictMode {
    /// Fully automatic resolution
    ///
    /// All conflicts are resolved by AI without human intervention.
    /// Use with caution - recommended only for CI environments with tests.
    Auto,

    /// Fully manual resolution
    ///
    /// All conflicts require human intervention. AI may suggest resolutions,
    /// but humans must approve them.
    #[default]
    Manual,

    /// Hybrid mode
    ///
    /// AI auto-resolves safe conflicts based on autonomy level and security keywords.
    /// Risky conflicts (those matching security keywords) require human review.
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
            _ => Err(ConfigErrorKind::Invalid(format!(
                "Invalid conflict mode: {s}. Must be one of: auto, manual, hybrid"
            ))
            .into()),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RECOVERY POLICY ENUM
// ═══════════════════════════════════════════════════════════════════════════

/// Recovery policy for database integrity and session cleanup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecoveryPolicy {
    /// Stop immediately on any corruption
    FailFast,
    /// Log warning and attempt to continue
    #[default]
    Warn,
    /// Silently attempt recovery
    Silent,
}

impl std::fmt::Display for RecoveryPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailFast => write!(f, "failfast"),
            Self::Warn => write!(f, "warn"),
            Self::Silent => write!(f, "silent"),
        }
    }
}

impl FromStr for RecoveryPolicy {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "failfast" => Ok(Self::FailFast),
            "warn" => Ok(Self::Warn),
            "silent" => Ok(Self::Silent),
            _ => Err(ConfigErrorKind::Invalid(format!(
                "Invalid recovery policy: {s}. Must be one of: failfast, warn, silent"
            ))
            .into()),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATED BOOL
// ═══════════════════════════════════════════════════════════════════════════

/// A validated boolean newtype that prevents accidental misuse of raw bools.
///
/// Use this for configuration flags where the intent must be explicit.
/// Construct via `ValidatedBool::new(true)` or `ValidatedBool::new(false)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedBool(bool);

impl ValidatedBool {
    /// Create a new validated boolean.
    #[must_use]
    pub const fn new(value: bool) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for ValidatedBool {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ValidatedBool> for bool {
    fn from(vb: ValidatedBool) -> Self {
        vb.0
    }
}

impl std::fmt::Display for ValidatedBool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
