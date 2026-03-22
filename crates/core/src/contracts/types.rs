//! Type contracts and validation system
//!
//! Provides rich type information for AI-first design:
//! - Constraints (min/max, regex patterns)
//! - Contextual hints (examples, suggestions)
//! - Dependencies between fields
//! - Machine-readable schemas

use im::HashMap;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// CORE TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// A contract describes constraints and metadata for a type or field
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeContract {
    /// Human-readable name of the type
    pub name: String,

    /// Description of what this type represents
    pub description: String,

    /// Constraints that must be satisfied
    pub constraints: Vec<Constraint>,

    /// Contextual hints for AI/users
    pub hints: Vec<ContextualHint>,

    /// Examples of valid values
    pub examples: Vec<String>,

    /// Field-level contracts for composite types
    #[serde(default)]
    pub fields: HashMap<String, FieldContract>,
}

/// A contract for a specific field
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldContract {
    /// Field name
    pub name: String,

    /// Field type (e.g., "String", "u32", "`PathBuf`")
    pub field_type: String,

    /// Is this field required?
    pub required: bool,

    /// Description of this field
    pub description: String,

    /// Constraints for this field
    pub constraints: Vec<Constraint>,

    /// Default value (if any)
    pub default: Option<String>,

    /// Dependencies on other fields
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Examples for this field
    pub examples: Vec<String>,
}

/// Validation constraints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Constraint {
    /// String must match regex pattern
    Regex {
        pattern: String,
        description: String,
    },

    /// Numeric range constraint
    Range {
        min: Option<i64>,
        max: Option<i64>,
        inclusive: bool,
    },

    /// Length constraint (for strings, arrays, etc.)
    Length {
        min: Option<usize>,
        max: Option<usize>,
    },

    /// Must be one of these values
    Enum { values: Vec<String> },

    /// Path must exist
    PathExists { must_be_absolute: bool },

    /// Path must be absolute
    PathAbsolute,

    /// Value must be unique across all instances
    Unique,

    /// Custom validation with description
    Custom { rule: String, description: String },
}

/// Contextual hints for AI agents and users
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextualHint {
    /// Type of hint
    #[serde(rename = "type")]
    pub hint_type: HintType,

    /// The hint message
    pub message: String,

    /// When this hint applies (optional condition)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,

    /// Related field or operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_to: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HintType {
    /// Best practice suggestion
    BestPractice,

    /// Common pitfall warning
    Warning,

    /// Usage example
    Example,

    /// Performance consideration
    Performance,

    /// Security consideration
    Security,

    /// Compatibility note
    Compatibility,
}
