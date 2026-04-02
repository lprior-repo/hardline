//! Validation helpers for queue domain

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_validate_range_in_bounds() {
        assert_eq!(validate_range(50, 0, 100, "priority").unwrap(), 50);
    }

    #[test]
    fn queue_validate_range_at_min() {
        assert_eq!(validate_range(0, 0, 100, "priority").unwrap(), 0);
    }

    #[test]
    fn queue_validate_range_at_max() {
        assert_eq!(validate_range(100, 0, 100, "priority").unwrap(), 100);
    }

    #[test]
    fn queue_validate_range_below_min() {
        let result = validate_range(0, 1, 100, "priority");
        assert!(matches!(result, Err(ValidationError::BelowMinimum { .. })));
    }

    #[test]
    fn queue_validate_range_above_max() {
        let result = validate_range(101, 0, 100, "priority");
        assert!(matches!(result, Err(ValidationError::ExceedsMaximum { .. })));
    }

    #[test]
    fn queue_validate_range_field_name_included() {
        let result = validate_range(200, 0, 100, "my_field");
        if let Err(ValidationError::ExceedsMaximum { field, .. }) = result {
            assert_eq!(field, "my_field");
        } else {
            panic!("Expected ExceedsMaximum");
        }
    }

    #[test]
    fn queue_validate_range_equal_min_max() {
        assert!(validate_range(5, 5, 5, "test").is_ok());
        assert!(validate_range(4, 5, 5, "test").is_err());
        assert!(validate_range(6, 5, 5, "test").is_err());
    }

    // --- Additional tests ---

    #[test]
    fn queue_validate_range_zero_min_zero_max() {
        assert!(validate_range(0, 0, 0, "test").is_ok());
        assert!(validate_range(1, 0, 0, "test").is_err());
    }

    #[test]
    fn queue_validate_range_just_below_max() {
        assert!(validate_range(99, 0, 100, "priority").is_ok());
    }

    #[test]
    fn queue_validate_range_just_above_min() {
        assert!(validate_range(1, 1, 100, "priority").is_ok());
    }

    #[test]
    fn queue_validate_range_one_above_max() {
        let result = validate_range(101, 0, 100, "priority");
        assert!(matches!(result, Err(ValidationError::ExceedsMaximum { value: 101, .. })));
    }

    #[test]
    fn queue_validate_range_one_below_min() {
        let result = validate_range(0, 1, 100, "priority");
        assert!(matches!(result, Err(ValidationError::BelowMinimum { value: 0, min: 1, .. })));
    }

    #[test]
    fn queue_validate_range_large_values() {
        assert!(validate_range(u32::MAX, 0, u32::MAX, "test").is_ok());
        assert!(validate_range(u32::MAX - 1, 0, u32::MAX, "test").is_ok());
    }

    #[test]
    fn queue_validate_range_field_name_empty() {
        let result = validate_range(200, 0, 100, "");
        assert!(result.is_err());
    }

    #[test]
    fn queue_validate_range_field_name_with_spaces() {
        let result = validate_range(200, 0, 100, "my field name");
        if let Err(ValidationError::ExceedsMaximum { field, .. }) = result {
            assert_eq!(field, "my field name");
        } else {
            panic!("Expected ExceedsMaximum");
        }
    }

    #[test]
    fn queue_validate_range_below_min_field_name_included() {
        let result = validate_range(5, 10, 100, "count_field");
        if let Err(ValidationError::BelowMinimum { field, min, value }) = result {
            assert_eq!(field, "count_field");
            assert_eq!(min, 10);
            assert_eq!(value, 5);
        } else {
            panic!("Expected BelowMinimum");
        }
    }
}
