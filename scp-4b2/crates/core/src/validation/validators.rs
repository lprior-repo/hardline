//! Composable validation patterns and abstractions
//!
//! This module provides **composable validation primitives** following
//! Scott Wlaschin's "Functional Design and Architecture" patterns.
//!
//! # Core Concepts
//!
//! ## 1. Validator Type
//!
//! A `Validator<T>` is a function that takes a value of type `T` and
//! returns `Result<(), ValidationError>`.
//!
//! ## 2. Composition
//!
//! Validators can be combined using combinator functions:
//! - `and`: both validators must pass
//! - `or`: at least one validator must pass
//! - `not`: validator must fail
//! - `map`: transform the error
//!
//! ## 3. Reusability
//!
//! Define validators once, compose them in different ways:
//!
//! ```rust
//! use isolate_core::validation::validators::*;
//!
//! let non_empty = not_empty::<String>();
//! let alphanumeric = is alphanumeric();
//! let valid_id = non_empty.and(alphanumeric);
//! ```
//!
//! # Design Principles
//!
//! 1. **Pure Functions**: All validators are pure (no side effects)
//! 2. **Composable**: Validators combine like Lego bricks
//! 3. **Reusable**: Define once, use everywhere
//! 4. **Type-Safe**: Leverage Rust's type system

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt;
use std::sync::Arc;

// ============================================================================
// VALIDATION ERROR TYPE
// ============================================================================

/// Error type for validation failures.
///
/// This is a simplified error type for validator composition.
/// For domain-specific validation, use `IdentifierError` from the domain layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Human-readable error message
    pub message: String,
    /// Optional field name that failed validation
    pub field: Option<String>,
    /// Optional value that failed validation
    pub value: Option<String>,
}

impl ValidationError {
    /// Create a new validation error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field: None,
            value: None,
        }
    }

    /// Add a field name to the error.
    #[must_use]
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Add a value to the error.
    #[must_use]
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(field) = &self.field {
            write!(f, "Validation error for field '{}': {}", field, self.message)
        } else {
            write!(f, "Validation error: {}", self.message)
        }
    }
}

impl std::error::Error for ValidationError {}

// ============================================================================
// VALIDATOR TYPE
// ============================================================================

/// A validator function that checks a value of type `T`.
///
/// This is a function pointer type for validators.
/// Validators are pure functions that return `Result<(), ValidationError>`.
pub type Validator<T> = fn(&T) -> Result<(), ValidationError>;

/// A boxed validator for dynamic dispatch.
///
/// Use this when you need to store validators in a collection
/// or return them from functions.
pub type BoxedValidator<T> = Box<dyn Fn(&T) -> Result<(), ValidationError> + Send + Sync>;

/// A shared boxed validator using Arc for cheap cloning.
///
/// Use this when validators need to be shared across threads.
pub type SharedValidator<T> = Arc<dyn Fn(&T) -> Result<(), ValidationError> + Send + Sync>;

// ============================================================================
// VALIDATOR TRAIT
// ============================================================================

/// Trait for composable validators.
///
/// This trait provides combinators for combining validators in different ways.
pub trait ValidationRule<T> {
    /// Combine two validators with logical AND: both must pass.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use isolate_core::validation::validators::ValidationRule;
    ///
    /// fn non_empty(s: &str) -> Result<(), ValidationError> {
    ///     if s.is_empty() {
    ///         Err(ValidationError::new("cannot be empty"))
    ///     } else {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// fn min_length(s: &str) -> Result<(), ValidationError> {
    ///     if s.len() < 3 {
    ///         Err(ValidationError::new("must be at least 3 characters"))
    ///     } else {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let validator = non_empty.and(min_length);
    /// assert!(validator("test").is_ok());
    /// assert!(validator("").is_err());
    /// assert!(validator("ab").is_err()); // passes non_empty, fails min_length
    /// ```
    fn and<U>(self, other: U) -> ComposedValidator<T, Self, U>
    where
        Self: Sized,
        U: Fn(&T) -> Result<(), ValidationError>;

    /// Map the error from this validator to a new error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use isolate_core::validation::validators::ValidationRule;
    ///
    /// fn non_empty(s: &str) -> Result<(), ValidationError> {
    ///     if s.is_empty() {
    ///         Err(ValidationError::new("cannot be empty"))
    ///     } else {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let validator = non_empty.map_err(|e| ValidationError {
    ///     message: format!("Custom: {}", e.message),
    ///     field: Some("username".to_string()),
    ///     value: None,
    /// });
    /// ```
    fn map_err<F>(self, f: F) -> MappedValidator<T, Self, F>
    where
        Self: Sized,
        F: Fn(ValidationError) -> ValidationError;
}

// Implement ValidationRule for function pointers
impl<T, F> ValidationRule<T> for F
where
    F: Fn(&T) -> Result<(), ValidationError>,
{
    fn and<U>(self, other: U) -> ComposedValidator<T, Self, U>
    where
        Self: Sized,
        U: Fn(&T) -> Result<(), ValidationError>,
    {
        ComposedValidator {
            first: self,
            second: other,
            _phantom: std::marker::PhantomData,
        }
    }

    fn map_err<M>(self, mapper: M) -> MappedValidator<T, Self, M>
    where
        Self: Sized,
        M: Fn(ValidationError) -> ValidationError,
    {
        MappedValidator {
            validator: self,
            mapper,
            _phantom: std::marker::PhantomData,
        }
    }
}

// ============================================================================
// COMPOSED VALIDATOR
// ============================================================================

/// A validator that combines two validators with logical AND.
///
/// Both validators must pass for this validator to succeed.
pub struct ComposedValidator<T, F, G>
where
    F: Fn(&T) -> Result<(), ValidationError>,
    G: Fn(&T) -> Result<(), ValidationError>,
{
    first: F,
    second: G,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F, G> Fn<(&T,)> for ComposedValidator<T, F, G>
where
    F: Fn(&T) -> Result<(), ValidationError>,
    G: Fn(&T) -> Result<(), ValidationError>,
{
    extern "rust-call" fn call(&self, args: (&T,)) -> Result<(), ValidationError> {
        // Run first validator
        if let Err(e) = (self.first)(&args.0) {
            return Err(e);
        }

        // Run second validator
        (self.second)(&args.0)
    }
}

impl<T, F, G> FnMut<(&T,)> for ComposedValidator<T, F, G>
where
    F: Fn(&T) -> Result<(), ValidationError>,
    G: Fn(&T) -> Result<(), ValidationError>,
{
    extern "rust-call" fn call_mut(&mut self, args: (&T,)) -> Result<(), ValidationError> {
        self.call(args)
    }
}

impl<T, F, G> FnOnce<(&T,)> for ComposedValidator<T, F, G>
where
    F: Fn(&T) -> Result<(), ValidationError>,
    G: Fn(&T) -> Result<(), ValidationError>,
{
    extern "rust-call" fn call_once(self, args: (&T,)) -> Result<(), ValidationError> {
        // Run first validator
        if let Err(e) = (self.first)(&args.0) {
            return Err(e);
        }

        // Run second validator
        (self.second)(&args.0)
    }
}

// ============================================================================
// MAPPED VALIDATOR
// ============================================================================

/// A validator that maps errors from the underlying validator.
pub struct MappedValidator<T, F, M>
where
    F: Fn(&T) -> Result<(), ValidationError>,
    M: Fn(ValidationError) -> ValidationError,
{
    validator: F,
    mapper: M,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F, M> Fn<(&T,)> for MappedValidator<T, F, M>
where
    F: Fn(&T) -> Result<(), ValidationError>,
    M: Fn(ValidationError) -> ValidationError,
{
    extern "rust-call" fn call(&self, args: (&T,)) -> Result<(), ValidationError> {
        (self.validator)(&args.0).map_err(&self.mapper)
    }
}

impl<T, F, M> FnMut<(&T,)> for MappedValidator<T, F, M>
where
    F: Fn(&T) -> Result<(), ValidationError>,
    M: Fn(ValidationError) -> ValidationError,
{
    extern "rust-call" fn call_mut(&mut self, args: (&T,)) -> Result<(), ValidationError> {
        self.call(args)
    }
}

impl<T, F, M> FnOnce<(&T,)> for MappedValidator<T, F, M>
where
    F: Fn(&T) -> Result<(), ValidationError>,
    M: Fn(ValidationError) -> ValidationError,
{
    extern "rust-call" fn call_once(self, args: (&T,)) -> Result<(), ValidationError> {
        (self.validator)(&args.0).map_err(self.mapper)
    }
}

// ============================================================================
// COMMON VALIDATORS
// ============================================================================

/// Create a validator that checks if a string is not empty.
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::validators::not_empty;
///
/// let validator = not_empty::<String>();
/// assert!(validator(&"hello".to_string()).is_ok());
/// assert!(validator(&"".to_string()).is_err());
/// ```
#[must_use]
pub fn not_empty<T: AsRef<str>>() -> Validator<T> {
    |value| {
        if value.as_ref().is_empty() {
            Err(ValidationError::new("value cannot be empty"))
        } else {
            Ok(())
        }
    }
}

/// Create a validator that checks if a string is alphanumeric.
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::validators::is_alphanumeric;
///
/// let validator = is_alphanumeric::<String>();
/// assert!(validator(&"abc123".to_string()).is_ok());
/// assert!(validator(&"abc-123".to_string()).is_err());
/// ```
#[must_use]
pub fn is_alphanumeric<T: AsRef<str>>() -> Validator<T> {
    |value| {
        if !value.as_ref().chars().all(|c| c.is_alphanumeric()) {
            Err(ValidationError::new("value must be alphanumeric"))
        } else {
            Ok(())
        }
    }
}

/// Create a validator that checks if a string matches a regex pattern.
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::validators::matches_pattern;
///
/// let validator = matches_pattern::<String>(r"^[a-z]+$");
/// assert!(validator(&"hello".to_string()).is_ok());
/// assert!(validator(&"hello123".to_string()).is_err());
/// ```
#[must_use]
pub fn matches_pattern<T: AsRef<str>>(pattern: &str) -> BoxedValidator<T> {
    let pattern = pattern.to_string();
    Box::new(move |value| {
        let regex = match regex::Regex::new(&pattern) {
            Ok(re) => re,
            Err(_) => {
                return Err(ValidationError::new(format!(
                    "invalid regex pattern: {}",
                    pattern
                )))
            }
        };

        if !regex.is_match(value.as_ref()) {
            Err(ValidationError::new(format!(
                "value does not match pattern: {}",
                pattern
            )))
        } else {
            Ok(())
        }
    })
}

/// Create a validator that checks if a number is within a range.
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::validators::in_range;
///
/// let validator = in_range(1..=100);
/// assert!(validator(&50).is_ok());
/// assert!(validator(&0).is_err());
/// assert!(validator(&101).is_err());
/// ```
#[must_use]
pub fn in_range<T>(range: std::ops::RangeInclusive<T>) -> Validator<T>
where
    T: PartialOrd + std::fmt::Display + Copy,
{
    move |value| {
        if !range.contains(value) {
            Err(ValidationError::new(format!(
                "value must be between {} and {}",
                range.start(),
                range.end()
            )))
        } else {
            Ok(())
        }
    }
}

/// Create a validator that checks if a collection has a minimum length.
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::validators::min_length;
///
/// let validator = min_length(3);
/// assert!(validator(&"hello".to_string()).is_ok());
/// assert!(validator(&"ab".to_string()).is_err());
/// ```
#[must_use]
pub fn min_length<T: AsRef<str>>(min: usize) -> Validator<T> {
    move |value| {
        if value.as_ref().len() < min {
            Err(ValidationError::new(format!(
                "value must be at least {} characters",
                min
            )))
        } else {
            Ok(())
        }
    }
}

/// Create a validator that checks if a collection has a maximum length.
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::validators::max_length;
///
/// let validator = max_length(10);
/// assert!(validator(&"hello".to_string()).is_ok());
/// assert!(validator(&"this is way too long".to_string()).is_err());
/// ```
#[must_use]
pub fn max_length<T: AsRef<str>>(max: usize) -> Validator<T> {
    move |value| {
        if value.as_ref().len() > max {
            Err(ValidationError::new(format!(
                "value must be at most {} characters",
                max
            )))
        } else {
            Ok(())
        }
    }
}

/// Create a validator that checks if a value equals one of the allowed values.
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::validators::one_of;
///
/// let validator = one_of(&["red", "green", "blue"]);
/// assert!(validator(&"red").is_ok());
/// assert!(validator(&"yellow").is_err());
/// ```
#[must_use]
pub fn one_of<'a, T>(allowed: &'a [&'a str]) -> Validator<T>
where
    T: AsRef<str> + 'a,
{
    move |value| {
        if !allowed.contains(&value.as_ref()) {
            Err(ValidationError::new(format!(
                "value must be one of: {}",
                allowed.join(", ")
            )))
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// VALIDATION RESULT HELPERS
// ============================================================================

/// Validate all items in a collection using a validator.
///
/// Returns `Ok(())` if all items pass validation, or the first error encountered.
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::validators::validate_all;
///
/// let items = vec!["hello", "world", "test"];
/// let validator = min_length(3);
/// assert!(validate_all(&items, validator).is_ok());
///
/// let items = vec!["hello", "hi", "test"];
/// assert!(validate_all(&items, validator).is_err()); // "hi" is too short
/// ```
pub fn validate_all<T, F>(items: &[T], validator: F) -> Result<(), ValidationError>
where
    F: Fn(&T) -> Result<(), ValidationError>,
{
    items.iter().try_for_each(|item| validator(item))
}

/// Validate that at least one item in a collection passes validation.
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::validators::validate_any;
///
/// let items = vec!["hello", "world", "test"];
/// let validator = min_length(10);
/// assert!(validate_any(&items, validator).is_err()); // none are long enough
///
/// let items = vec!["hello", "verylongstring", "test"];
/// assert!(validate_any(&items, validator).is_ok()); // "verylongstring" passes
/// ```
pub fn validate_any<T, F>(items: &[T], validator: F) -> Result<(), ValidationError>
where
    F: Fn(&T) -> Result<(), ValidationError>,
{
    items
        .iter()
        .find_map(|item| validator(item).ok())
        .ok_or_else(|| ValidationError::new("no items passed validation"))
}

/// Validate that none of the items in a collection pass validation.
///
/// This is the inverse of `validate_any`.
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::validators::validate_none;
///
/// let items = vec!["hello", "world", "test"];
/// let validator = min_length(10);
/// assert!(validate_none(&items, validator).is_ok()); // none are long enough
///
/// let items = vec!["hello", "verylongstring", "test"];
/// assert!(validate_none(&items, validator).is_err()); // "verylongstring" passes
/// ```
pub fn validate_none<T, F>(items: &[T], validator: F) -> Result<(), ValidationError>
where
    F: Fn(&T) -> Result<(), ValidationError>,
{
    items
        .iter()
        .try_for_each(|item| match validator(item) {
            Ok(_) => Err(ValidationError::new(
                "item passed validation when it should have failed",
            )),
            Err(_) => Ok(()),
        })
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ===== ValidationError Tests =====

    #[test]
    fn test_validation_error_new() {
        let error = ValidationError::new("test error");
        assert_eq!(error.message, "test error");
        assert!(error.field.is_none());
        assert!(error.value.is_none());
    }

    #[test]
    fn test_validation_error_with_field() {
        let error = ValidationError::new("test error").with_field("username");
        assert_eq!(error.field, Some("username".to_string()));
    }

    #[test]
    fn test_validation_error_with_value() {
        let error = ValidationError::new("test error").with_value("test_value");
        assert_eq!(error.value, Some("test_value".to_string()));
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::new("test error");
        assert_eq!(format!("{}", error), "Validation error: test error");

        let error_with_field = error.with_field("username");
        assert_eq!(
            format!("{}", error_with_field),
            "Validation error for field 'username': test error"
        );
    }

    // ===== Common Validators Tests =====

    #[test]
    fn test_not_empty_valid() {
        let validator = not_empty::<String>();
        assert!(validator(&"hello".to_string()).is_ok());
        assert!(validator(&"a".to_string()).is_ok());
    }

    #[test]
    fn test_not_empty_invalid() {
        let validator = not_empty::<String>();
        assert!(validator(&"".to_string()).is_err());
    }

    #[test]
    fn test_is_alphanumeric_valid() {
        let validator = is_alphanumeric::<String>();
        assert!(validator(&"abc123".to_string()).is_ok());
        assert!(validator(&"ABC123".to_string()).is_ok());
    }

    #[test]
    fn test_is_alphanumeric_invalid() {
        let validator = is_alphanumeric::<String>();
        assert!(validator(&"abc-123".to_string()).is_err());
        assert!(validator(&"abc 123".to_string()).is_err());
        assert!(validator(&"abc.123".to_string()).is_err());
    }

    #[test]
    fn test_min_length_valid() {
        let validator = min_length(3);
        assert!(validator(&"hello").is_ok());
        assert!(validator(&"abc").is_ok());
    }

    #[test]
    fn test_min_length_invalid() {
        let validator = min_length(3);
        assert!(validator(&"ab").is_err());
        assert!(validator(&"a").is_err());
        assert!(validator(&"").is_err());
    }

    #[test]
    fn test_max_length_valid() {
        let validator = max_length(5);
        assert!(validator(&"hello").is_ok());
        assert!(validator(&"abc").is_ok());
    }

    #[test]
    fn test_max_length_invalid() {
        let validator = max_length(5);
        assert!(validator(&"hello world").is_err());
    }

    #[test]
    fn test_in_range_valid() {
        let validator = in_range(1..=100);
        assert!(validator(&1).is_ok());
        assert!(validator(&50).is_ok());
        assert!(validator(&100).is_ok());
    }

    #[test]
    fn test_in_range_invalid() {
        let validator = in_range(1..=100);
        assert!(validator(&0).is_err());
        assert!(validator(&101).is_err());
    }

    #[test]
    fn test_one_of_valid() {
        let validator = one_of(&["red", "green", "blue"]);
        assert!(validator(&"red").is_ok());
        assert!(validator(&"green").is_ok());
        assert!(validator(&"blue").is_ok());
    }

    #[test]
    fn test_one_of_invalid() {
        let validator = one_of(&["red", "green", "blue"]);
        assert!(validator(&"yellow").is_err());
        assert!(validator(&"").is_err());
    }

    // ===== Composition Tests =====

    #[test]
    fn test_validator_and_both_pass() {
        let validator = non_empty.and(min_length::<String>(3));
        assert!(validator(&"hello".to_string()).is_ok());
    }

    #[test]
    fn test_validator_and_first_fails() {
        let validator = non_empty.and(min_length::<String>(3));
        let result = validator(&"".to_string());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().message, "value cannot be empty");
    }

    #[test]
    fn test_validator_and_second_fails() {
        let validator = non_empty.and(min_length::<String>(3));
        let result = validator(&"ab".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("at least 3 characters"));
    }

    #[test]
    fn test_validator_map_err() {
        let validator = non_empty.map_err(|e| ValidationError {
            message: format!("Custom: {}", e.message),
            field: Some("username".to_string()),
            value: None,
        });

        let result = validator(&"");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field, Some("username".to_string()));
        assert!(error.message.starts_with("Custom:"));
    }

    // ===== Collection Validation Tests =====

    #[test]
    fn test_validate_all_pass() {
        let items = vec!["hello", "world", "test"];
        let validator = min_length(3);
        assert!(validate_all(&items, validator).is_ok());
    }

    #[test]
    fn test_validate_all_fail() {
        let items = vec!["hello", "hi", "test"];
        let validator = min_length(3);
        assert!(validate_all(&items, validator).is_err());
    }

    #[test]
    fn test_validate_any_pass() {
        let items = vec!["hello", "verylongstring", "test"];
        let validator = min_length(10);
        assert!(validate_any(&items, validator).is_ok());
    }

    #[test]
    fn test_validate_any_fail() {
        let items = vec!["hello", "world", "test"];
        let validator = min_length(10);
        assert!(validate_any(&items, validator).is_err());
    }

    #[test]
    fn test_validate_none_pass() {
        let items = vec!["hello", "world", "test"];
        let validator = min_length(10);
        assert!(validate_none(&items, validator).is_ok());
    }

    #[test]
    fn test_validate_none_fail() {
        let items = vec!["hello", "verylongstring", "test"];
        let validator = min_length(10);
        assert!(validate_none(&items, validator).is_err());
    }
}
