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
