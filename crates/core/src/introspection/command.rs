//! Command introspection types
//!
//! This module provides types for documenting command structure,
//! arguments, flags, and other command metadata.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Detailed command introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandIntrospection {
    /// Command name
    pub command: String,
    /// Human-readable description
    pub description: String,
    /// Command aliases
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Positional arguments
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentSpec>,
    /// Optional flags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<FlagSpec>,
    /// Usage examples
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<CommandExample>,
    /// Prerequisites for running this command
    pub prerequisites: Prerequisites,
    /// Side effects this command will produce
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<String>,
    /// Possible error conditions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub error_conditions: Vec<ErrorCondition>,
}

/// Argument specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentSpec {
    /// Argument name
    pub name: String,
    /// Type of argument
    #[serde(rename = "type")]
    pub arg_type: String,
    /// Whether this argument is required
    pub required: bool,
    /// Human-readable description
    pub description: String,
    /// Validation pattern (regex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<String>,
    /// Example values
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
}

/// Flag specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagSpec {
    /// Long flag name (e.g., "no-hooks")
    pub long: String,
    /// Short flag name (e.g., "t")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// Human-readable description
    pub description: String,
    /// Type of flag value
    #[serde(rename = "type")]
    pub flag_type: String,
    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Possible values for enum-like flags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
    /// Category for grouping flags in help output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

impl FlagSpec {
    /// Validate that a category is one of the allowed values.
    ///
    /// Valid categories are: behavior, configuration, filter, output, advanced
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the category is not in the allowed list.
    ///
    /// # Examples
    ///
    /// ```
    /// # use scp_core::introspection::FlagSpec;
    /// assert!(FlagSpec::validate_category("behavior").is_ok());
    /// assert!(FlagSpec::validate_category("invalid").is_err());
    /// ```
    pub fn validate_category(category: &str) -> Result<()> {
        const VALID_CATEGORIES: &[&str] =
            &["behavior", "configuration", "filter", "output", "advanced"];

        if VALID_CATEGORIES.contains(&category) {
            Ok(())
        } else {
            Err(Error::validation_error(format!(
                "Invalid flag category: '{}'. Must be one of: {}",
                category,
                VALID_CATEGORIES.join(", ")
            )))
        }
    }
}

/// Command usage example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExample {
    /// Example command line
    pub command: String,
    /// Description of what this example does
    pub description: String,
}

/// Error condition documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCondition {
    /// Error code
    pub code: String,
    /// Human-readable description
    pub description: String,
    /// How to resolve this error
    pub resolution: String,
}

/// Prerequisites for a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisites {
    /// Must be initialized
    pub initialized: bool,
    /// JJ must be installed
    pub jj_installed: bool,
    /// Additional custom checks
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub custom: Vec<String>,
}

impl Prerequisites {
    /// Check if all prerequisites are met
    ///
    /// # Returns
    ///
    /// Returns `true` if all prerequisites are satisfied. The result should be checked
    /// before proceeding with operations that require these prerequisites.
    #[must_use]
    pub const fn all_met(&self) -> bool {
        self.initialized && self.jj_installed && self.custom.is_empty()
    }

    /// Count how many prerequisites are met
    ///
    /// # Returns
    ///
    /// Returns the count of met prerequisites. The result should be used
    /// for reporting or validation purposes.
    #[must_use]
    pub const fn count_met(&self) -> usize {
        let mut count = 0;
        if self.initialized {
            count += 1;
        }
        if self.jj_installed {
            count += 1;
        }
        count
    }

    /// Total number of prerequisites
    ///
    /// # Returns
    ///
    /// Returns the total count of prerequisites. The result should be used
    /// for reporting or validation purposes.
    #[must_use]
    pub const fn total(&self) -> usize {
        2 + self.custom.len()
    }
}
