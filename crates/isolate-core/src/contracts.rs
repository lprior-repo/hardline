//! Type contracts and validation system
//!
//! Provides rich type information for AI-first design:
//! - Constraints (min/max, regex patterns)
//! - Contextual hints (examples, suggestions)
//! - Dependencies between fields
//! - Machine-readable schemas

use im::HashMap;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

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

// ═══════════════════════════════════════════════════════════════════════════
// TRAIT FOR TYPES WITH CONTRACTS
// ═══════════════════════════════════════════════════════════════════════════

/// Trait for types that have contracts
pub trait HasContract {
    /// Get the contract for this type
    fn contract() -> TypeContract;

    /// Validate an instance against its contract
    fn validate(&self) -> Result<()>;

    /// Get JSON Schema representation
    ///
    /// # Returns
    ///
    /// Returns the JSON schema for this contract type. The result should be used
    /// as this generates a complete schema definition.
    #[must_use]
    fn json_schema() -> serde_json::Value {
        Self::contract().to_json_schema()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// IMPLEMENTATION HELPERS
// ═══════════════════════════════════════════════════════════════════════════

impl TypeContract {
    /// Convert contract to JSON Schema format
    ///
    /// # Returns
    ///
    /// Returns the JSON schema representation. The result should be used
    /// as this is a transformation operation.
    #[must_use]
    pub fn to_json_schema(&self) -> serde_json::Value {
        let mut schema = serde_json::json!({
            "type": "object",
            "title": self.name,
            "description": self.description,
        });

        if !self.examples.is_empty() {
            if let Some(obj) = schema.as_object_mut() {
                obj.insert("examples".to_string(), serde_json::json!(self.examples));
            }
        }

        // Add field schemas using functional patterns
        let (properties, required) = self.fields.iter().fold(
            (serde_json::Map::new(), Vec::new()),
            |(mut props, mut req), (field_name, field_contract)| {
                props.insert(field_name.clone(), field_contract.to_json_schema());
                if field_contract.required {
                    req.push(field_name.clone());
                }
                (props, req)
            },
        );

        if let Some(obj) = schema.as_object_mut() {
            if !properties.is_empty() {
                obj.insert(
                    "properties".to_string(),
                    serde_json::Value::Object(properties),
                );
            }

            if !required.is_empty() {
                obj.insert("required".to_string(), serde_json::json!(required));
            }
        }

        schema
    }

    /// Create a builder for constructing contracts
    ///
    /// # Returns
    ///
    /// Returns a new builder instance. The result must be used to continue
    /// the builder pattern chain.
    #[must_use]
    pub fn builder(name: impl Into<String>) -> TypeContractBuilder {
        TypeContractBuilder {
            name: name.into(),
            description: String::new(),
            constraints: Vec::new(),
            hints: Vec::new(),
            examples: Vec::new(),
            fields: im::HashMap::new(),
        }
    }
}

impl FieldContract {
    /// Convert field contract to JSON Schema property
    ///
    /// # Returns
    ///
    /// Returns the JSON schema representation. The result should be used
    /// as this is a transformation operation.
    #[must_use]
    pub fn to_json_schema(&self) -> serde_json::Value {
        let mut schema = serde_json::json!({
            "description": self.description,
        });

        // Add type information using safe object mutation
        if let Some(obj) = schema.as_object_mut() {
            obj.insert(
                "type".to_string(),
                match self.field_type.as_str() {
                    "u32" | "u64" | "i32" | "i64" | "usize" => serde_json::json!("integer"),
                    "bool" => serde_json::json!("boolean"),
                    "Vec<String>" => serde_json::json!("array"),
                    _ => serde_json::json!("string"), /* "String" and unknown types default to
                                                       * string */
                },
            );

            // Add constraints using functional patterns
            self.constraints
                .iter()
                .for_each(|constraint| match constraint {
                    Constraint::Regex { pattern, .. } => {
                        obj.insert("pattern".to_string(), serde_json::json!(pattern));
                    }
                    Constraint::Range { min, max, .. } => {
                        if let Some(min_val) = min {
                            obj.insert("minimum".to_string(), serde_json::json!(min_val));
                        }
                        if let Some(max_val) = max {
                            obj.insert("maximum".to_string(), serde_json::json!(max_val));
                        }
                    }
                    Constraint::Length { min, max } => {
                        if let Some(min_len) = min {
                            obj.insert("minLength".to_string(), serde_json::json!(min_len));
                        }
                        if let Some(max_len) = max {
                            obj.insert("maxLength".to_string(), serde_json::json!(max_len));
                        }
                    }
                    Constraint::Enum { values } => {
                        obj.insert("enum".to_string(), serde_json::json!(values));
                    }
                    Constraint::PathExists { .. }
                    | Constraint::PathAbsolute
                    | Constraint::Unique
                    | Constraint::Custom { .. } => {}
                });

            if let Some(default) = &self.default {
                obj.insert("default".to_string(), serde_json::json!(default));
            }

            if !self.examples.is_empty() {
                obj.insert("examples".to_string(), serde_json::json!(self.examples));
            }
        }

        schema
    }

    /// Create a builder for field contracts
    pub fn builder(name: impl Into<String>, field_type: impl Into<String>) -> FieldContractBuilder {
        FieldContractBuilder {
            name: name.into(),
            field_type: field_type.into(),
            required: false,
            description: String::new(),
            constraints: Vec::new(),
            default: None,
            depends_on: Vec::new(),
            examples: Vec::new(),
        }
    }
}

impl Constraint {
    /// Validate a string value against this constraint
    pub fn validate_string(&self, value: &str) -> Result<()> {
        match self {
            Self::Regex {
                pattern,
                description: _,
            } => {
                let re = regex::Regex::new(pattern).map_err(|e| Error::ValidationError {
                    message: format!("Invalid regex pattern: {e}"),
                    field: None,
                    value: None,
                    constraints: Vec::new(),
                })?;

                if !re.is_match(value) {
                    return Err(Error::ValidationError {
                        message: format!("Value does not match regex pattern: {pattern}"),
                        field: None,
                        value: None,
                        constraints: Vec::new(),
                    });
                }
            }
            Self::Length { min, max } => {
                let len = value.len();
                if let Some(min_len) = min {
                    if len < *min_len {
                        return Err(Error::ValidationError {
                            message: format!("Length {len} is less than minimum {min_len}"),
                            field: None,
                            value: None,
                            constraints: Vec::new(),
                        });
                    }
                }
                if let Some(max_len) = max {
                    if len > *max_len {
                        return Err(Error::ValidationError {
                            message: format!("Length {len} exceeds maximum {max_len}"),
                            field: None,
                            value: None,
                            constraints: Vec::new(),
                        });
                    }
                }
            }
            Self::Enum { values } => {
                if !values.contains(&value.to_string()) {
                    return Err(Error::ValidationError {
                        message: format!("Value '{value}' is not in allowed values: {values:?}"),
                        field: None,
                        value: None,
                        constraints: Vec::new(),
                    });
                }
            }
            Self::Range { .. }
            | Self::PathExists { .. }
            | Self::PathAbsolute
            | Self::Unique
            | Self::Custom { .. } => {}
        }
        Ok(())
    }

    /// Validate a numeric value against this constraint
    pub fn validate_number(&self, value: i64) -> Result<()> {
        if let Self::Range {
            min,
            max,
            inclusive,
        } = self
        {
            if let Some(min_val) = min {
                if *inclusive {
                    if value < *min_val {
                        return Err(Error::ValidationError {
                        message: format!("Value {value} is less than minimum {min_val} (inclusive: {inclusive})"),
                        field: None,
                        value: None,
                        constraints: Vec::new(),
                    });
                    }
                } else if value <= *min_val {
                    return Err(Error::ValidationError {
                        message: format!(
                            "Value {value} is less than or equal to minimum {min_val} (exclusive)"
                        ),
                        field: None,
                        value: None,
                        constraints: Vec::new(),
                    });
                }
            }
            if let Some(max_val) = max {
                if *inclusive {
                    if value > *max_val {
                        return Err(Error::ValidationError {
                            message: format!(
                                "Value {value} exceeds maximum {max_val} (inclusive: {inclusive})"
                            ),
                            field: None,
                            value: None,
                            constraints: Vec::new(),
                        });
                    }
                } else if value >= *max_val {
                    return Err(Error::ValidationError {
                        message: format!("Value {value} is greater than or equal to maximum {max_val} (exclusive)"),
                        field: None,
                        value: None,
                        constraints: Vec::new(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Validate a path against this constraint
    pub fn validate_path(&self, path: &std::path::Path) -> Result<()> {
        match self {
            Self::PathAbsolute => {
                if !path.is_absolute() {
                    return Err(Error::ValidationError {
                        message: format!("Path '{}' must be absolute", path.display()),
                        field: None,
                        value: None,
                        constraints: Vec::new(),
                    });
                }
            }
            Self::PathExists { must_be_absolute } => {
                if *must_be_absolute && !path.is_absolute() {
                    return Err(Error::ValidationError {
                        message: format!("Path '{}' must be absolute", path.display()),
                        field: None,
                        value: None,
                        constraints: Vec::new(),
                    });
                }
                match path.try_exists() {
                    Ok(true) => {}
                    _ => {
                        return Err(Error::ValidationError {
                            message: format!("Path '{}' does not exist", path.display()),
                            field: None,
                            value: None,
                            constraints: Vec::new(),
                        });
                    }
                }
            }
            Self::Regex { .. }
            | Self::Range { .. }
            | Self::Length { .. }
            | Self::Enum { .. }
            | Self::Unique
            | Self::Custom { .. } => {}
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BUILDERS
// ═══════════════════════════════════════════════════════════════════════════

pub struct TypeContractBuilder {
    name: String,
    description: String,
    constraints: Vec<Constraint>,
    hints: Vec<ContextualHint>,
    examples: Vec<String>,
    fields: HashMap<String, FieldContract>,
}

impl TypeContractBuilder {
    /// Set the description for the contract.
    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a constraint to the contract.
    #[must_use]
    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Add a contextual hint to the contract.
    #[must_use]
    pub fn hint(mut self, hint: ContextualHint) -> Self {
        self.hints.push(hint);
        self
    }

    /// Add an example to the contract.
    #[must_use]
    pub fn example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Add a field to the contract.
    #[must_use]
    pub fn field(mut self, name: impl Into<String>, field: FieldContract) -> Self {
        self.fields = self.fields.update(name.into(), field);
        self
    }

    /// Build the final `TypeContract`.
    ///
    /// # Returns
    ///
    /// Returns the constructed contract. The result must be used as this
    /// consumes the builder.
    #[must_use]
    pub fn build(self) -> TypeContract {
        TypeContract {
            name: self.name,
            description: self.description,
            constraints: self.constraints,
            hints: self.hints,
            examples: self.examples,
            fields: self.fields,
        }
    }
}

pub struct FieldContractBuilder {
    name: String,
    field_type: String,
    required: bool,
    description: String,
    constraints: Vec<Constraint>,
    default: Option<String>,
    depends_on: Vec<String>,
    examples: Vec<String>,
}

impl FieldContractBuilder {
    /// Mark the field as required.
    #[must_use]
    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set the field description.
    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a constraint to the field.
    #[must_use]
    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Set the default value for the field.
    #[must_use]
    pub fn default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Add a dependency on another field.
    #[must_use]
    pub fn depends_on(mut self, field: impl Into<String>) -> Self {
        self.depends_on.push(field.into());
        self
    }

    pub fn example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Build the final `FieldContract`.
    ///
    /// # Returns
    ///
    /// Returns the constructed contract. The result must be used as this
    /// consumes the builder.
    #[must_use]
    pub fn build(self) -> FieldContract {
        FieldContract {
            name: self.name,
            field_type: self.field_type,
            required: self.required,
            description: self.description,
            constraints: self.constraints,
            default: self.default,
            depends_on: self.depends_on,
            examples: self.examples,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_constraint_valid() {
        let constraint = Constraint::Regex {
            pattern: r"^[a-z0-9_-]+$".to_string(),
            description: "alphanumeric with hyphens and underscores".to_string(),
        };

        assert!(constraint.validate_string("my-session").is_ok());
        assert!(constraint.validate_string("test_123").is_ok());
    }

    #[test]
    fn test_regex_constraint_invalid() {
        let constraint = Constraint::Regex {
            pattern: r"^[a-z0-9_-]+$".to_string(),
            description: "alphanumeric with hyphens and underscores".to_string(),
        };

        assert!(constraint.validate_string("invalid session").is_err());
        assert!(constraint.validate_string("UPPERCASE").is_err());
    }

    #[test]
    fn test_length_constraint_valid() {
        let constraint = Constraint::Length {
            min: Some(1),
            max: Some(64),
        };

        assert!(constraint.validate_string("valid").is_ok());
        assert!(constraint.validate_string("a").is_ok());
        assert!(constraint.validate_string(&"x".repeat(64)).is_ok());
    }

    #[test]
    fn test_length_constraint_too_short() {
        let constraint = Constraint::Length {
            min: Some(1),
            max: Some(64),
        };

        assert!(constraint.validate_string("").is_err());
    }

    #[test]
    fn test_length_constraint_too_long() {
        let constraint = Constraint::Length {
            min: Some(1),
            max: Some(64),
        };

        assert!(constraint.validate_string(&"x".repeat(65)).is_err());
    }

    #[test]
    fn test_range_constraint_valid() {
        let constraint = Constraint::Range {
            min: Some(10),
            max: Some(5000),
            inclusive: true,
        };

        assert!(constraint.validate_number(10).is_ok());
        assert!(constraint.validate_number(100).is_ok());
        assert!(constraint.validate_number(5000).is_ok());
    }

    #[test]
    fn test_range_constraint_too_low() {
        let constraint = Constraint::Range {
            min: Some(10),
            max: Some(5000),
            inclusive: true,
        };

        assert!(constraint.validate_number(9).is_err());
    }

    #[test]
    fn test_range_constraint_too_high() {
        let constraint = Constraint::Range {
            min: Some(10),
            max: Some(5000),
            inclusive: true,
        };

        assert!(constraint.validate_number(5001).is_err());
    }

    #[test]
    fn test_enum_constraint_valid() {
        let constraint = Constraint::Enum {
            values: vec![
                "active".to_string(),
                "paused".to_string(),
                "completed".to_string(),
            ],
        };

        assert!(constraint.validate_string("active").is_ok());
        assert!(constraint.validate_string("paused").is_ok());
        assert!(constraint.validate_string("completed").is_ok());
    }

    #[test]
    fn test_enum_constraint_invalid() {
        let constraint = Constraint::Enum {
            values: vec!["active".to_string(), "paused".to_string()],
        };

        assert!(constraint.validate_string("invalid").is_err());
    }

    #[test]
    fn test_path_absolute_constraint() {
        let constraint = Constraint::PathAbsolute;

        assert!(constraint
            .validate_path(std::path::Path::new("/absolute/path"))
            .is_ok());
        assert!(constraint
            .validate_path(std::path::Path::new("relative/path"))
            .is_err());
    }

    #[test]
    fn test_contract_builder() {
        let contract = TypeContract::builder("TestType")
            .description("A test type")
            .example("example1")
            .build();

        assert_eq!(contract.name, "TestType");
        assert_eq!(contract.description, "A test type");
        assert_eq!(contract.examples.len(), 1);
    }

    #[test]
    fn test_field_contract_builder() {
        let field = FieldContract::builder("name", "String")
            .required()
            .description("The name field")
            .constraint(Constraint::Length {
                min: Some(1),
                max: Some(64),
            })
            .example("my-session")
            .build();

        assert_eq!(field.name, "name");
        assert_eq!(field.field_type, "String");
        assert!(field.required);
        assert_eq!(field.constraints.len(), 1);
        assert_eq!(field.examples.len(), 1);
    }

    #[test]
    fn test_json_schema_generation() {
        let field = FieldContract::builder("name", "String")
            .required()
            .description("Session name")
            .constraint(Constraint::Regex {
                pattern: r"^[a-z0-9_-]+$".to_string(),
                description: "alphanumeric".to_string(),
            })
            .build();

        let contract = TypeContract::builder("Session")
            .description("A session")
            .field("name", field)
            .build();

        let schema = contract.to_json_schema();

        #[allow(clippy::indexing_slicing)]
        {
            assert_eq!(schema["type"], "object");
            assert_eq!(schema["title"], "Session");
            assert!(schema["properties"].is_object());
            assert!(schema["required"].is_array());
        }
    }
}
