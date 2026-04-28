//! Implementation helpers for contract types

use super::{
    builders::{FieldContractBuilder, TypeContractBuilder},
    types::{Constraint, FieldContract, TypeContract},
};
use crate::{Error, Result};

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
    #[must_use]
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
                let re = regex::Regex::new(pattern)
                    .map_err(|e| Error::validation_error(format!("Invalid regex pattern: {e}")))?;

                if !re.is_match(value) {
                    return Err(Error::validation_error(format!(
                        "Value does not match regex pattern: {pattern}"
                    )));
                }
            }
            Self::Length { min, max } => {
                let len = value.len();
                if let Some(min_len) = min {
                    if len < *min_len {
                        return Err(Error::validation_error(format!(
                            "Length {len} is less than minimum {min_len}"
                        )));
                    }
                }
                if let Some(max_len) = max {
                    if len > *max_len {
                        return Err(Error::validation_error(format!(
                            "Length {len} exceeds maximum {max_len}"
                        )));
                    }
                }
            }
            Self::Enum { values } => {
                if !values.contains(&value.to_string()) {
                    return Err(Error::validation_error(format!(
                        "Value '{value}' is not in allowed values: {values:?}"
                    )));
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
                        return Err(Error::validation_error(format!(
                            "Value {value} is less than minimum {min_val} (inclusive: {inclusive})"
                        )));
                    }
                } else if value <= *min_val {
                    return Err(Error::validation_error(format!(
                        "Value {value} is less than or equal to minimum {min_val} (exclusive)"
                    )));
                }
            }
            if let Some(max_val) = max {
                if *inclusive {
                    if value > *max_val {
                        return Err(Error::validation_error(format!(
                            "Value {value} exceeds maximum {max_val} (inclusive: {inclusive})"
                        )));
                    }
                } else if value >= *max_val {
                    return Err(Error::validation_error(format!(
                        "Value {value} is greater than or equal to maximum {max_val} (exclusive)"
                    )));
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
                    return Err(Error::validation_error(format!(
                        "Path '{}' must be absolute",
                        path.display()
                    )));
                }
            }
            Self::PathExists { must_be_absolute } => {
                if *must_be_absolute && !path.is_absolute() {
                    return Err(Error::validation_error(format!(
                        "Path '{}' must be absolute",
                        path.display()
                    )));
                }
                match path.try_exists() {
                    Ok(true) => {}
                    _ => {
                        return Err(Error::validation_error(format!(
                            "Path '{}' does not exist",
                            path.display()
                        )));
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
