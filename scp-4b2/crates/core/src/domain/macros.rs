//! Invariant checking macros for the domain layer.
//!
//! This module provides macros for enforcing invariants across the codebase
//! with consistent error messages and zero-panic guarantees.
//!
//! # Design Principles
//!
//! - **Zero panic**: All macros return `Result`, never panic
//! - **Zero unwrap**: No `unwrap()` or `expect()` anywhere
//! - **Consistent error messages**: Standardized format across all invariants
//! - **Conditional compilation**: Test-only and debug-only variants
//! - **Ergonomic**: Simple, readable syntax
//!
//! # Macros
//!
//! - [`invariant!`] - Runtime invariant checks (always enabled)
//! - [`assert_invariant!`] - Test-only invariant checks (test builds only)
//! - [`debug_invariant!`] - Debug-only invariant checks (debug builds only)
//!
//! # Example
//!
//! ```rust,ignore
//! use isolate_core::domain::{BeadError, invariant};
//!
//! fn validate_timestamps(created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Result<(), BeadError> {
//!     invariant!(
//!         updated_at >= created_at,
//!         BeadError::NonMonotonicTimestamps { created_at, updated_at },
//!         "Timestamp ordering violated: updated_at ({updated_at}) < created_at ({created_at})"
//!     );
//!     Ok(())
//! }
//! ```

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

/// Runtime invariant check with custom error.
///
/// This macro checks an invariant condition and returns a `Result` with the
/// provided error if the condition is violated. Use this for invariants that
/// must always be enforced, including in production.
///
/// # Syntax
///
/// ```rust,ignore
/// invariant!(
///     condition,
///     error_expression,
///     "message format {arg1} {arg2}",
///     arg1, arg2
/// );
/// ```
///
/// # Parameters
///
/// - `condition`: Boolean expression to check
/// - `error_expression`: Error value to return if condition is false
/// - `message`: Format string (optional, for documentation)
/// - `args`: Format arguments (optional)
///
/// # Returns
///
/// - `Ok(())` if condition is true
/// - `Err(error_expression)` if condition is false
///
/// # Example
///
/// ```rust,ignore
/// use isolate_core::domain::{BeadError, invariant};
///
/// fn check_bead_state(bead: &Bead) -> Result<(), BeadError> {
///     invariant!(
///         bead.updated_at >= bead.created_at,
///         BeadError::NonMonotonicTimestamps {
///             created_at: bead.created_at,
///             updated_at: bead.updated_at,
///         },
///         "Timestamp ordering violated"
///     );
///     Ok(())
/// }
/// ```
///
/// # Zero Guarantees
///
/// - Never panics
/// - No unwrap/expect
/// - Always returns a Result
#[macro_export]
macro_rules! invariant {
    // Without format message
    ($condition:expr, $error:expr) => {
        if !$condition {
            return Err($error);
        }
    };
    // With format message and arguments
    ($condition:expr, $error:expr, $msg:expr) => {
        if !$condition {
            return Err($error);
        }
    };
    // With format message and arguments (trailing comma for error expr)
    (@internal $condition:expr, $error:expr, $($msg:tt)*) => {
        if !$condition {
            // Message is for documentation only, logged in debug builds
            #[cfg(debug_assertions)]
            {
                log::debug!(concat!("Invariant violated: ", $($msg)*));
            }
            return Err($error);
        }
    };
}

/// Test-only invariant check.
///
/// This macro is identical to [`invariant!`] but only compiles in test builds.
/// Use this for expensive invariants that you want to check in tests but not
/// in production for performance reasons.
///
/// # Syntax
///
/// ```rust,ignore
/// assert_invariant!(
///     condition,
///     error_expression,
///     "message format {arg}",
///     arg
/// );
/// ```
///
/// # Parameters
///
/// Same as [`invariant!`].
///
/// # Returns
///
/// - `Ok(())` if condition is true (in test builds)
/// - `Err(error_expression)` if condition is false (in test builds)
/// - `Ok(())` always (in non-test builds)
///
/// # Example
///
/// ```rust,ignore
/// use isolate_core::domain::{BeadError, assert_invariant};
///
/// fn expensive_validation(bead: &Bead) -> Result<(), BeadError> {
///     // Only check this invariant during tests
///     assert_invariant!(
///         validate_deep_consistency(bead),
///         BeadError::InvalidStateTransition {
///             from: bead.state,
///             to: bead.state,
///         },
///         "Deep consistency check failed"
///     );
///     Ok(())
/// }
/// ```
///
/// # Zero Guarantees
///
/// - Never panics
/// - No unwrap/expect
/// - Compiles to nothing in release builds
#[macro_export]
macro_rules! assert_invariant {
    // Without format message
    (#[cfg(test)] $condition:expr, $error:expr) => {
        if !$condition {
            return Err($error);
        }
    };
    // With format message
    (#[cfg(test)] $condition:expr, $error:expr, $msg:expr) => {
        if !$condition {
            #[cfg(debug_assertions)]
            {
                log::debug!(concat!("Test invariant violated: ", $msg));
            }
            return Err($error);
        }
    };
    // Public interface
    ($condition:expr, $error:expr) => {
        #[cfg(test)]
        if !$condition {
            return Err($error);
        }
    };
    ($condition:expr, $error:expr, $msg:expr) => {
        #[cfg(test)]
        if !$condition {
            #[cfg(debug_assertions)]
            {
                log::debug!(concat!("Test invariant violated: ", $msg));
            }
            return Err($error);
        }
    };
}

/// Debug-only invariant check.
///
/// This macro is identical to [`invariant!`] but only compiles in debug builds.
/// Use this for invariants that help during development but shouldn't impact
/// production performance.
///
/// # Syntax
///
/// ```rust,ignore
/// debug_invariant!(
///     condition,
///     error_expression,
///     "message format {arg}",
///     arg
/// );
/// ```
///
/// # Parameters
///
/// Same as [`invariant!`].
///
/// # Returns
///
/// - `Ok(())` if condition is true (in debug builds)
/// - `Err(error_expression)` if condition is false (in debug builds)
/// - `Ok(())` always (in release builds)
///
/// # Example
///
/// ```rust,ignore
/// use isolate_core::domain::{BeadError, debug_invariant};
///
/// fn update_with_debug_check(bead: &Bead) -> Result<Bead, BeadError> {
///     let updated = bead.update_title("New Title")?;
///
///     // Only check in debug builds
///     debug_invariant!(
///         updated.validate().is_ok(),
///         BeadError::InvalidStateTransition {
///             from: bead.state,
///             to: updated.state,
///         },
///         "Post-update validation failed"
///     );
///
///     Ok(updated)
/// }
/// ```
///
/// # Zero Guarantees
///
/// - Never panics
/// - No unwrap/expect
/// - Compiles to nothing in release builds
#[macro_export]
macro_rules! debug_invariant {
    // Without format message
    ($condition:expr, $error:expr) => {
        #[cfg(debug_assertions)]
        if !$condition {
            return Err($error);
        }
    };
    // With format message
    ($condition:expr, $error:expr, $msg:expr) => {
        #[cfg(debug_assertions)]
        if !$condition {
            log::debug!(concat!("Debug invariant violated: ", $msg));
            return Err($error);
        }
    };
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    // We can't directly test macros in this module since they expand at the call site.
    // Tests are located in the domain module tests that use these macros.
}

// ============================================================================
// USAGE EXAMPLES (DOC TESTS)
// ============================================================================

#[cfg(test)]
mod example_tests {
    // Example error type for testing

    // Example error type for testing
    #[derive(Debug, PartialEq, Eq)]
    enum TestError {
        InvariantViolated(String),
    }

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::InvariantViolated(msg) => write!(f, "Invariant violated: {msg}"),
            }
        }
    }

    impl std::error::Error for TestError {}

    #[test]
    fn test_invariant_macro_pass() {
        fn check_positive(x: i32) -> Result<(), TestError> {
            invariant!(
                x > 0,
                TestError::InvariantViolated("x must be positive".into())
            );
            Ok(())
        }

        assert!(check_positive(42).is_ok());
    }

    #[test]
    fn test_invariant_macro_fail() {
        fn check_positive(x: i32) -> Result<(), TestError> {
            invariant!(
                x > 0,
                TestError::InvariantViolated(format!("x={x} must be positive"))
            );
            Ok(())
        }

        let result = check_positive(-5);
        assert!(result.is_err());
        assert_eq!(
            result,
            Err(TestError::InvariantViolated("x=-5 must be positive".into()))
        );
    }

    #[test]
    fn test_invariant_with_complex_condition() {
        fn check_order(a: i32, b: i32) -> Result<(), TestError> {
            invariant!(
                a <= b,
                TestError::InvariantViolated(format!("a={a} > b={b}"))
            );
            Ok(())
        }

        assert!(check_order(1, 2).is_ok());
        assert!(check_order(5, 5).is_ok());
        assert!(check_order(10, 5).is_err());
    }

    #[test]
    fn test_chained_invariants() {
        fn validate_range(x: i32) -> Result<(), TestError> {
            invariant!(
                x >= 0,
                TestError::InvariantViolated("negative value".into())
            );
            invariant!(
                x <= 100,
                TestError::InvariantViolated("value too large".into())
            );
            Ok(())
        }

        assert!(validate_range(50).is_ok());
        assert!(validate_range(-1).is_err());
        assert!(validate_range(101).is_err());
    }

    #[test]
    fn test_invariant_with_computed_error() {
        fn check_division(a: i32, b: i32) -> Result<(), TestError> {
            invariant!(
                b != 0,
                TestError::InvariantViolated(format!("division by zero: {a} / {b}"))
            );
            Ok(())
        }

        assert!(check_division(10, 2).is_ok());
        assert!(check_division(10, 0).is_err());
    }

    // Test that assert_invariant only compiles in test mode
    #[test]
    fn test_assert_invariant() {
        fn check_test_only(x: i32) -> Result<(), TestError> {
            assert_invariant!(
                x > 0,
                TestError::InvariantViolated("test-only check failed".into())
            );
            Ok(())
        }

        // Should pass in test mode
        assert!(check_test_only(42).is_ok());

        // This would fail in test mode but not in release
        #[cfg(test)]
        assert!(check_test_only(-5).is_err());
    }

    // Test that debug_invariant works correctly
    #[test]
    fn test_debug_invariant() {
        fn check_debug(x: i32) -> Result<(), TestError> {
            debug_invariant!(
                x > 0,
                TestError::InvariantViolated("debug check failed".into())
            );
            Ok(())
        }

        // Should pass
        assert!(check_debug(42).is_ok());

        // In debug mode, this would fail
        #[cfg(debug_assertions)]
        assert!(check_debug(-5).is_err());

        // In release mode, it passes
        #[cfg(not(debug_assertions))]
        assert!(check_debug(-5).is_ok());
    }
}
