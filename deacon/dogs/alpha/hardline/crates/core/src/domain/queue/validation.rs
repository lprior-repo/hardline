//! Queue validation helpers - Pure functions for queue validation
//!
//! Contains the `validate_range` function for validating numeric ranges.

use std::cmp::Ordering;

use crate::domain::validation::{ValidationError, ValidationResult};

/// Railway combinator: Validate a value is within a range
///
/// # Errors
///
/// Returns an error if the value is below the minimum or above the maximum.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::validation::ValidationError;

    #[test]
    fn validate_range_at_minimum() {
        assert_eq!(validate_range(0, 0, 100, "priority"), Ok(0));
    }

    #[test]
    fn validate_range_at_maximum() {
        assert_eq!(validate_range(100, 0, 100, "priority"), Ok(100));
    }

    #[test]
    fn validate_range_at_boundary() {
        assert_eq!(validate_range(50, 0, 100, "priority"), Ok(50));
    }

    #[test]
    fn validate_range_below_minimum() {
        let result = validate_range(0, 1, 100, "priority");
        assert!(matches!(result, Err(ValidationError::BelowMinimum { .. })));
    }

    #[test]
    fn validate_range_above_maximum() {
        let result = validate_range(101, 0, 100, "priority");
        assert!(matches!(
            result,
            Err(ValidationError::ExceedsMaximum { .. })
        ));
    }

    #[test]
    fn validate_range_single_value() {
        assert_eq!(validate_range(42, 42, 42, "value"), Ok(42));
        assert!(validate_range(41, 42, 42, "value").is_err());
        assert!(validate_range(43, 42, 42, "value").is_err());
    }

    #[test]
    fn validate_range_zero_allowed() {
        assert_eq!(validate_range(0, 0, 10, "field"), Ok(0));
    }
}
