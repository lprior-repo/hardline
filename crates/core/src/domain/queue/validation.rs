//! Queue validation helpers - Pure functions for queue validation
//!
//! Contains the `validate_range` function for validating numeric ranges.

use std::cmp::Ordering;

use crate::domain::validation::{ValidationError, ValidationResult};

/// Railway combinator: Validate a value is within a range
pub fn validate_range(value: u32, min: u32, max: u32, field: &str) -> ValidationResult<u32> {
    match value.cmp(&min) {
        Ordering::Less => Err(ValidationError::BelowMinimum {
            field: field.to_string(),
            value,
            min,
        }),
        Ordering::Greater if value > max => Err(ValidationError::ExceedsMaximum {
            field: field.to_string(),
            value,
            max,
        }),
        _ => Ok(value),
    }
}
