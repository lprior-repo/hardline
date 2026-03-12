//! Examples of invariant checking macros in the domain layer.
//!
//! This file demonstrates practical usage of the invariant macros
//! for enforcing business rules and maintaining data consistency.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};

// Example error type
#[derive(Debug, PartialEq, Eq)]
enum ValidationError {
    TimestampOrdering {
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    },
    EmptyState,
    InvalidTransition {
        from: String,
        to: String,
    },
    InvariantViolated(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TimestampOrdering {
                created_at,
                updated_at,
            } => {
                write!(
                    f,
                    "Timestamp ordering violated: updated_at ({updated_at}) < created_at ({created_at})"
                )
            }
            Self::EmptyState => write!(f, "State cannot be empty"),
            Self::InvalidTransition { from, to } => {
                write!(f, "Invalid state transition: {from} -> {to}")
            }
            Self::InvariantViolated(msg) => {
                write!(f, "Invariant violated: {msg}")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

// ============================================================================
// EXAMPLE 1: Basic Invariant Check
// ============================================================================

/// Example: Simple invariant check for timestamp ordering.
fn validate_timestamp_ordering(
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Result<(), ValidationError> {
    crate::invariant!(
        updated_at >= created_at,
        ValidationError::TimestampOrdering {
            created_at,
            updated_at,
        }
    );
    Ok(())
}

#[cfg(test)]
mod test_example_1 {
    use super::*;

    #[test]
    fn test_valid_timestamps() {
        let now = Utc::now();
        assert!(validate_timestamp_ordering(now, now + chrono::Duration::seconds(1)).is_ok());
    }

    #[test]
    fn test_equal_timestamps() {
        let now = Utc::now();
        assert!(validate_timestamp_ordering(now, now).is_ok());
    }

    #[test]
    fn test_invalid_timestamps() {
        let now = Utc::now();
        let result = validate_timestamp_ordering(now, now - chrono::Duration::seconds(1));
        assert!(result.is_err());
    }
}

// ============================================================================
// EXAMPLE 2: Chained Invariants
// ============================================================================

/// Example: Multiple invariant checks in sequence.
fn validate_bead_state(
    title: &str,
    state: &str,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Result<(), ValidationError> {
    // Check title is not empty
    crate::invariant!(
        !title.is_empty(),
        ValidationError::InvariantViolated("title cannot be empty".into())
    );

    // Check state is not empty
    crate::invariant!(!state.is_empty(), ValidationError::EmptyState);

    // Check timestamp ordering
    crate::invariant!(
        updated_at >= created_at,
        ValidationError::TimestampOrdering {
            created_at,
            updated_at,
        }
    );

    Ok(())
}

#[cfg(test)]
mod test_example_2 {
    use super::*;

    #[test]
    fn test_valid_state() {
        let now = Utc::now();
        assert!(validate_bead_state("Test Bead", "open", now, now).is_ok());
    }

    #[test]
    fn test_empty_title() {
        let now = Utc::now();
        let result = validate_bead_state("", "open", now, now);
        assert!(matches!(result, Err(ValidationError::InvariantViolated(_))));
    }

    #[test]
    fn test_empty_state() {
        let now = Utc::now();
        let result = validate_bead_state("Title", "", now, now);
        assert!(matches!(result, Err(ValidationError::EmptyState)));
    }

    #[test]
    fn test_invalid_timestamps() {
        let now = Utc::now();
        let result = validate_bead_state("Title", "open", now, now - chrono::Duration::seconds(1));
        assert!(matches!(
            result,
            Err(ValidationError::TimestampOrdering { .. })
        ));
    }
}

// ============================================================================
// EXAMPLE 3: Debug-Only Invariants
// ============================================================================

/// Example: Expensive validation that only runs in debug builds.
fn validate_deep_consistency(
    items: &[i32],
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Result<(), ValidationError> {
    // Always check timestamp ordering
    crate::invariant!(
        updated_at >= created_at,
        ValidationError::TimestampOrdering {
            created_at,
            updated_at,
        }
    );

    // Expensive O(n) check - only in debug builds
    crate::debug_invariant!(
        items.windows(2).all(|w| w[0] <= w[1]),
        ValidationError::InvariantViolated("items not sorted".into())
    );

    Ok(())
}

#[cfg(test)]
mod test_example_3 {
    use super::*;

    #[test]
    fn test_valid_sorted() {
        let items = vec![1, 2, 3, 4, 5];
        let now = Utc::now();
        assert!(validate_deep_consistency(&items, now, now).is_ok());
    }

    #[test]
    fn test_invalid_unsorted() {
        let items = vec![1, 3, 2, 4, 5];
        let now = Utc::now();

        #[cfg(debug_assertions)]
        {
            let result = validate_deep_consistency(&items, now, now);
            assert!(result.is_err());
        }

        #[cfg(not(debug_assertions))]
        {
            // In release mode, the debug invariant is skipped
            assert!(validate_deep_consistency(&items, now, now).is_ok());
        }
    }
}

// ============================================================================
// EXAMPLE 4: Test-Only Invariants
// ============================================================================

/// Example: Invariants checked only during testing.
fn validate_production_safe(
    value: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Result<(), ValidationError> {
    // Always check timestamp ordering
    crate::invariant!(
        updated_at >= created_at,
        ValidationError::TimestampOrdering {
            created_at,
            updated_at,
        }
    );

    // Expensive validation - only in test builds
    crate::assert_invariant!(
        (0..=1_000_000).contains(&value),
        ValidationError::InvariantViolated(format!("value {value} out of range"))
    );

    Ok(())
}

#[cfg(test)]
mod test_example_4 {
    use super::*;

    #[test]
    fn test_valid_value() {
        let now = Utc::now();
        assert!(validate_production_safe(500, now, now).is_ok());
    }

    #[test]
    fn test_invalid_value() {
        let now = Utc::now();
        #[cfg(test)]
        {
            let result = validate_production_safe(2_000_000, now, now);
            assert!(result.is_err());
        }
    }
}

// ============================================================================
// EXAMPLE 5: Complex Invariant with Computed Error
// ============================================================================

/// Example: Invariant with complex error computation.
fn validate_division(a: i32, b: i32) -> Result<i32, ValidationError> {
    crate::invariant!(
        b != 0,
        ValidationError::InvariantViolated(format!("division by zero: {a} / {b}"))
    );

    crate::invariant!(
        a % b == 0,
        ValidationError::InvariantViolated(format!("non-integer result: {a} / {b}"))
    );

    Ok(a / b)
}

#[cfg(test)]
mod test_example_5 {
    use super::*;

    #[test]
    fn test_valid_division() {
        assert_eq!(validate_division(10, 2).unwrap(), 5);
        assert_eq!(validate_division(20, 5).unwrap(), 4);
    }

    #[test]
    fn test_division_by_zero() {
        let result = validate_division(10, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("division by zero"));
    }

    #[test]
    fn test_non_integer_division() {
        let result = validate_division(10, 3);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("non-integer result"));
    }
}

// ============================================================================
// EXAMPLE 6: State Transition Validation
// ============================================================================

/// Example: Validating state transitions.
fn validate_state_transition(from: &str, to: &str) -> Result<(), ValidationError> {
    // Define valid transitions
    let valid_transitions = [
        ("open", "in_progress"),
        ("open", "blocked"),
        ("in_progress", "blocked"),
        ("in_progress", "closed"),
        ("blocked", "in_progress"),
        ("blocked", "deferred"),
    ];

    crate::invariant!(
        valid_transitions.contains(&(from, to)),
        ValidationError::InvalidTransition {
            from: from.to_string(),
            to: to.to_string(),
        }
    );

    Ok(())
}

#[cfg(test)]
mod test_example_6 {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert!(validate_state_transition("open", "in_progress").is_ok());
        assert!(validate_state_transition("in_progress", "closed").is_ok());
    }

    #[test]
    fn test_invalid_transition() {
        let result = validate_state_transition("closed", "open");
        assert!(matches!(
            result,
            Err(ValidationError::InvalidTransition { .. })
        ));
    }
}
