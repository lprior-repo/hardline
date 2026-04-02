#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![forbid(unsafe_code)]

//! Domain validation - Railway-Oriented Programming for validation
//!
//! All validation functions return `Result<T, ValidationError>`
//! allowing for easy composition with `.and_then()` chains.

use std::collections::VecDeque;

/// Type alias for validation results - Railway track
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Domain validation errors
///
/// All validation errors are typed and explicit, never string-based.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    /// A required value was empty or only whitespace
    #[error("'{0}' cannot be empty")]
    EmptyValue(String),

    /// Invalid characters were found
    #[error("'{field}' contains invalid character: '{found}'")]
    InvalidCharacters { field: String, found: String },

    /// A value exceeded its maximum allowed value
    #[error("'{field}' exceeds maximum of {max}")]
    ExceedsMaximum { field: String, value: u32, max: u32 },

    /// A value was below its minimum allowed value
    #[error("'{field}' is below minimum of {min}")]
    BelowMinimum { field: String, value: u32, min: u32 },

    /// Invalid state transition
    #[error("invalid state transition from '{from}' to '{to}'")]
    InvalidStateTransition { from: String, to: String },

    /// Value not found in collection
    #[error("'{field}' with value '{value}' not found")]
    NotFound { field: String, value: String },

    /// Value already exists in collection
    #[error("'{field}' with value '{value}' already exists")]
    AlreadyExists { field: String, value: String },

    /// Position out of bounds
    #[error("position {position} is out of bounds (length: {length})")]
    OutOfBounds { position: usize, length: usize },

    /// Multiple validation errors collected
    #[error("multiple validation errors: {count} error(s)", count = .0.len())]
    Multiple(Vec<ValidationError>),
}

impl ValidationError {
    /// Create a Multiple error from a vector of errors.
    #[must_use]
    pub fn multiple(errors: Vec<ValidationError>) -> Self {
        Self::Multiple(errors)
    }

    /// Check if this is a recoverable error.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::EmptyValue(_) | Self::InvalidCharacters { .. } | Self::OutOfBounds { .. }
        )
    }
}

/// Validator builder for collecting multiple validation errors
///
/// Uses Railway-Oriented Programming to collect all errors
/// before failing, rather than failing on the first error.
#[derive(Debug, Clone, Default)]
pub struct Validator {
    errors: VecDeque<ValidationError>,
}

impl Validator {
    /// Create a new validator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a validation that returns a Result.
    ///
    /// If the validation fails, the error is collected.
    /// If it succeeds, the value is returned for chaining.
    pub fn validate<T>(&mut self, result: ValidationResult<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(e) => {
                self.errors.push_back(e);
                None
            }
        }
    }

    /// Validate a predicate, adding an error if it fails.
    pub fn check(&mut self, predicate: bool, error: ValidationError) -> ValidationResult<()> {
        if predicate {
            Ok(())
        } else {
            self.errors.push_back(error);
            Err(ValidationError::EmptyValue("check".to_string()))
        }
    }

    /// Add a custom error.
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push_back(error);
    }

    /// Finalize validation, returning Ok if no errors were collected.
    ///
    /// # Errors
    /// Returns the collected errors, or a single error if only one.
    pub fn finalize(mut self) -> ValidationResult<()> {
        match self.errors.len() {
            0 => Ok(()),
            1 => {
                if let Some(error) = self.errors.pop_front() {
                    Err(error)
                } else {
                    Ok(())
                }
            }
            _ => Err(ValidationError::Multiple(self.errors.into())),
        }
    }

    /// Check if any errors have been collected.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the number of errors collected.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

/// Railway combinator: Validate a value using a predicate
pub fn validate_that<T, F>(value: T, predicate: F, error_msg: &str) -> ValidationResult<T>
where
    F: FnOnce(&T) -> bool,
{
    if predicate(&value) {
        Ok(value)
    } else {
        Err(ValidationError::EmptyValue(error_msg.to_string()))
    }
}

/// Railway combinator: Validate a value is within a range
pub fn validate_range(value: u32, min: u32, max: u32, field: &str) -> ValidationResult<u32> {
    if value < min {
        Err(ValidationError::BelowMinimum {
            field: field.to_string(),
            value,
            min,
        })
    } else if value > max {
        Err(ValidationError::ExceedsMaximum {
            field: field.to_string(),
            value,
            max,
        })
    } else {
        Ok(value)
    }
}

/// Railway combinator: Chain multiple validations, returning the first error
pub fn validate_all<T>(results: Vec<ValidationResult<T>>) -> ValidationResult<Vec<T>> {
    let mut ok_values = Vec::with_capacity(results.len());
    for result in results {
        match result {
            Ok(value) => ok_values.push(value),
            Err(e) => return Err(e),
        }
    }
    Ok(ok_values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_that_success() {
        assert_eq!(validate_that(5, |&n| n > 0, "test"), Ok(5));
    }

    #[test]
    fn test_validate_that_failure() {
        assert!(validate_that(-5, |&n| n > 0, "test").is_err());
    }

    #[test]
    fn test_validate_range_in_bounds() {
        assert_eq!(validate_range(5, 1, 10, "test"), Ok(5));
        assert_eq!(validate_range(1, 1, 10, "test"), Ok(1));
        assert_eq!(validate_range(10, 1, 10, "test"), Ok(10));
    }

    #[test]
    fn test_validate_range_below_minimum() {
        assert!(matches!(
            validate_range(0, 1, 10, "priority"),
            Err(ValidationError::BelowMinimum { .. })
        ));
    }

    #[test]
    fn test_validate_range_exceeds_maximum() {
        assert!(matches!(
            validate_range(11, 1, 10, "priority"),
            Err(ValidationError::ExceedsMaximum { .. })
        ));
    }

    #[test]
    fn test_validator_collects_multiple_errors() {
        let mut validator = Validator::new();
        validator.validate::<()>(Err(ValidationError::EmptyValue("a".to_string())));
        validator.validate::<()>(Err(ValidationError::EmptyValue("b".to_string())));
        validator.validate::<()>(Ok(()));

        let result = validator.finalize();
        assert!(matches!(result, Err(ValidationError::Multiple(_))));

        if let Err(ValidationError::Multiple(errors)) = result {
            assert_eq!(errors.len(), 2);
        }
    }

    #[test]
    fn test_validator_single_error() {
        let mut validator = Validator::new();
        validator.validate::<()>(Err(ValidationError::EmptyValue("a".to_string())));

        let result = validator.finalize();
        assert!(matches!(result, Err(ValidationError::EmptyValue(_))));

        if let Err(ValidationError::EmptyValue(field)) = result {
            assert_eq!(field, "a");
        }
    }

    #[test]
    fn test_validator_no_errors() {
        let mut validator = Validator::new();
        validator.validate::<()>(Ok(()));
        assert!(validator.finalize().is_ok());
    }

    #[test]
    fn test_validator_check() {
        let mut validator = Validator::new();
        let _ = validator.check(true, ValidationError::EmptyValue("a".to_string()));
        let _ = validator.check(false, ValidationError::EmptyValue("b".to_string()));

        assert!(validator.has_errors());
        assert_eq!(validator.error_count(), 1);
    }

    #[test]
    fn test_validate_all_success() {
        let results = vec![Ok(1), Ok(2), Ok(3)];
        assert_eq!(validate_all(results), Ok(vec![1, 2, 3]));
    }

    #[test]
    fn test_validate_all_fails_fast() {
        let results = vec![
            Ok(1),
            Err(ValidationError::EmptyValue("x".to_string())),
            Ok(3),
        ];
        assert!(validate_all(results).is_err());
    }

    #[test]
    fn test_validation_error_display_empty_value() {
        let err = ValidationError::EmptyValue("field_name".into());
        let msg = format!("{err}");
        assert!(msg.contains("field_name") && msg.contains("empty"));
    }

    #[test]
    fn test_validation_error_display_invalid_characters() {
        let err = ValidationError::InvalidCharacters {
            field: "name".into(),
            found: "$".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("name") && msg.contains("$"));
    }

    #[test]
    fn test_validation_error_display_exceeds_maximum() {
        let err = ValidationError::ExceedsMaximum {
            field: "priority".into(),
            value: 200,
            max: 100,
        };
        let msg = format!("{err}");
        assert!(msg.contains("priority") && msg.contains("100"));
    }

    #[test]
    fn test_validation_error_display_below_minimum() {
        let err = ValidationError::BelowMinimum {
            field: "count".into(),
            value: 0,
            min: 1,
        };
        let msg = format!("{err}");
        assert!(msg.contains("count") && msg.contains("1"));
    }

    #[test]
    fn test_validation_error_display_invalid_state_transition() {
        let err = ValidationError::InvalidStateTransition {
            from: "Pending".into(),
            to: "Merged".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Pending") && msg.contains("Merged"));
    }

    #[test]
    fn test_validation_error_display_not_found() {
        let err = ValidationError::NotFound {
            field: "id".into(),
            value: "abc".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("abc") && msg.contains("not found"));
    }

    #[test]
    fn test_validation_error_display_already_exists() {
        let err = ValidationError::AlreadyExists {
            field: "session".into(),
            value: "s1".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("s1") && msg.contains("already exists"));
    }

    #[test]
    fn test_validation_error_display_out_of_bounds() {
        let err = ValidationError::OutOfBounds {
            position: 5,
            length: 3,
        };
        let msg = format!("{err}");
        assert!(msg.contains("5") && msg.contains("3"));
    }

    #[test]
    fn test_validation_error_display_multiple() {
        let errors = vec![
            ValidationError::EmptyValue("a".into()),
            ValidationError::EmptyValue("b".into()),
        ];
        let err = ValidationError::Multiple(errors);
        let msg = format!("{err}");
        assert!(msg.contains("2 error(s)"));
    }

    #[test]
    fn test_validation_error_debug() {
        let err = ValidationError::EmptyValue("x".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("EmptyValue"));
    }

    #[test]
    fn test_validation_error_clone_and_eq() {
        let a = ValidationError::EmptyValue("field".into());
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_validation_error_is_recoverable() {
        assert!(ValidationError::EmptyValue("x".into()).is_recoverable());
        assert!(ValidationError::InvalidCharacters {
            field: "f".into(),
            found: "c".into(),
        }
        .is_recoverable());
        assert!(ValidationError::OutOfBounds {
            position: 0,
            length: 0,
        }
        .is_recoverable());
        assert!(!ValidationError::ExceedsMaximum {
            field: "f".into(),
            value: 0,
            max: 0,
        }
        .is_recoverable());
        assert!(!ValidationError::BelowMinimum {
            field: "f".into(),
            value: 0,
            min: 0,
        }
        .is_recoverable());
        assert!(!ValidationError::InvalidStateTransition {
            from: "A".into(),
            to: "B".into(),
        }
        .is_recoverable());
    }

    #[test]
    fn test_validation_error_multiple_helper() {
        let errors = vec![
            ValidationError::EmptyValue("a".into()),
            ValidationError::EmptyValue("b".into()),
        ];
        let err = ValidationError::multiple(errors);
        assert!(matches!(err, ValidationError::Multiple(_)));
    }

    #[test]
    fn test_validator_default_is_empty() {
        let v = Validator::default();
        assert!(!v.has_errors());
        assert_eq!(v.error_count(), 0);
    }

    #[test]
    fn test_validator_validate_returns_value() {
        let mut v = Validator::new();
        let result = v.validate(Ok(42));
        assert_eq!(result, Some(42));
        assert!(!v.has_errors());
    }

    #[test]
    fn test_validator_validate_collects_error() {
        let mut v = Validator::new();
        let result = v.validate::<()>(Err(ValidationError::EmptyValue("test".into())));
        assert!(result.is_none());
        assert!(v.has_errors());
    }

    #[test]
    fn test_validator_add_error() {
        let mut v = Validator::new();
        v.add_error(ValidationError::EmptyValue("custom".into()));
        assert!(v.has_errors());
        assert_eq!(v.error_count(), 1);
    }

    #[test]
    fn test_validator_finalize_with_two_errors() {
        let mut v = Validator::new();
        v.add_error(ValidationError::EmptyValue("a".into()));
        v.add_error(ValidationError::EmptyValue("b".into()));
        let result = v.finalize();
        assert!(matches!(result, Err(ValidationError::Multiple(_))));
        if let Err(ValidationError::Multiple(errors)) = result {
            assert_eq!(errors.len(), 2);
        }
    }

    #[test]
    fn test_validate_range_at_boundary_min() {
        assert!(validate_range(0, 0, 10, "test").is_ok());
    }

    #[test]
    fn test_validate_range_at_boundary_max() {
        assert!(validate_range(10, 0, 10, "test").is_ok());
    }

    #[test]
    fn test_validate_range_field_name_in_error() {
        let result = validate_range(20, 0, 10, "my_field");
        match result {
            Err(ValidationError::ExceedsMaximum { field, .. }) => {
                assert_eq!(field, "my_field");
            }
            _ => panic!("Expected ExceedsMaximum"),
        }
    }

    #[test]
    fn test_validate_that_returns_original_value() {
        let s = String::from("hello");
        let result = validate_that(s, |v| !v.is_empty(), "test");
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_validate_all_empty_vec() {
        let result: Result<Vec<i32>, ValidationError> = validate_all(vec![]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_validation_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(ValidationError::EmptyValue("x".into()));
        let _ = format!("{err:?}");
    }
}
