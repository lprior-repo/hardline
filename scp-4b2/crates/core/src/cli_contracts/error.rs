//! Contract error types using thiserror for domain errors.
//!
//! All contract violations return `ContractError` with detailed context.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use thiserror::Error;

/// Contract violation errors.
///
/// These errors represent violations of KIRK contracts:
/// - Preconditions that were not met
/// - Invariants that were violated
/// - Postconditions that were not satisfied
#[derive(Debug, Clone, Error)]
pub enum ContractError {
    /// A precondition was not satisfied before the operation.
    #[error("Precondition '{name}' failed: {description}")]
    PreconditionFailed {
        /// Name of the failed precondition
        name: &'static str,
        /// Human-readable description of the precondition
        description: &'static str,
    },

    /// An invariant was violated during or after the operation.
    #[error("Invariant '{name}' violated: {description}")]
    InvariantViolation {
        /// Name of the violated invariant
        name: &'static str,
        /// Human-readable description of the invariant
        description: &'static str,
    },

    /// A postcondition was not satisfied after the operation.
    #[error("Postcondition '{name}' failed: {description}")]
    PostconditionFailed {
        /// Name of the failed postcondition
        name: &'static str,
        /// Human-readable description of the postcondition
        description: &'static str,
    },

    /// Multiple contract violations occurred.
    #[error("Multiple contract violations: {0}")]
    Multiple(String),

    /// A required input was missing or invalid.
    #[error("Invalid input for '{field}': {reason}")]
    InvalidInput {
        /// The field that was invalid
        field: &'static str,
        /// The reason the input was invalid
        reason: String,
    },

    /// A state transition was invalid.
    #[error("Invalid state transition from '{from}' to '{to}'")]
    InvalidStateTransition {
        /// The source state
        from: String,
        /// The target state
        to: String,
    },

    /// A resource was not found.
    #[error("Resource not found: {resource_type} '{identifier}'")]
    NotFound {
        /// Type of the resource
        resource_type: &'static str,
        /// Identifier of the resource
        identifier: String,
    },

    /// An operation was attempted on a resource in an invalid state.
    #[error("Operation '{operation}' not allowed on {resource_type} in state '{state}'")]
    InvalidOperationForState {
        /// The operation that was attempted
        operation: &'static str,
        /// Type of the resource
        resource_type: &'static str,
        /// Current state of the resource
        state: String,
    },

    /// A concurrent modification was detected.
    #[error("Concurrent modification detected: {description}")]
    ConcurrentModification {
        /// Description of the conflict
        description: String,
    },
}

impl ContractError {
    /// Create a new invalid input error.
    #[must_use]
    pub fn invalid_input(field: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidInput {
            field,
            reason: reason.into(),
        }
    }

    /// Create a new invalid state transition error.
    #[must_use]
    pub fn invalid_transition(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::InvalidStateTransition {
            from: from.into(),
            to: to.into(),
        }
    }

    /// Create a new not found error.
    #[must_use]
    pub fn not_found(resource_type: &'static str, identifier: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type,
            identifier: identifier.into(),
        }
    }

    /// Create a new invalid operation for state error.
    #[must_use]
    pub fn invalid_operation_for_state(
        operation: &'static str,
        resource_type: &'static str,
        state: impl Into<String>,
    ) -> Self {
        Self::InvalidOperationForState {
            operation,
            resource_type,
            state: state.into(),
        }
    }

    /// Combine multiple errors into a single error.
    #[must_use]
    pub fn combine(errors: Vec<Self>) -> Option<Self> {
        if errors.is_empty() {
            return None;
        }
        if errors.len() == 1 {
            return errors.into_iter().next();
        }
        let messages: Vec<String> = errors
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        Some(Self::Multiple(messages.join("; ")))
    }

    /// Check if this is a precondition error.
    #[must_use]
    pub const fn is_precondition(&self) -> bool {
        matches!(self, Self::PreconditionFailed { .. })
    }

    /// Check if this is an invariant error.
    #[must_use]
    pub const fn is_invariant(&self) -> bool {
        matches!(self, Self::InvariantViolation { .. })
    }

    /// Check if this is a postcondition error.
    #[must_use]
    pub const fn is_postcondition(&self) -> bool {
        matches!(self, Self::PostconditionFailed { .. })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precondition_error() {
        let error = ContractError::PreconditionFailed {
            name: "session_exists",
            description: "Session must exist before removal",
        };
        assert!(error.is_precondition());
        assert!(!error.is_invariant());
        assert!(!error.is_postcondition());
        assert!(error.to_string().contains("session_exists"));
    }

    #[test]
    fn test_invariant_error() {
        let error = ContractError::InvariantViolation {
            name: "session_name_unique",
            description: "Session names must be unique",
        };
        assert!(!error.is_precondition());
        assert!(error.is_invariant());
        assert!(!error.is_postcondition());
        assert!(error.to_string().contains("session_name_unique"));
    }

    #[test]
    fn test_postcondition_error() {
        let error = ContractError::PostconditionFailed {
            name: "session_created",
            description: "Session should exist after creation",
        };
        assert!(!error.is_precondition());
        assert!(!error.is_invariant());
        assert!(error.is_postcondition());
        assert!(error.to_string().contains("session_created"));
    }

    #[test]
    fn test_invalid_input_helper() {
        let error = ContractError::invalid_input("name", "cannot be empty");
        assert!(matches!(error, ContractError::InvalidInput { .. }));
        assert!(error.to_string().contains("name"));
        assert!(error.to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_invalid_transition_helper() {
        let error = ContractError::invalid_transition("completed", "active");
        assert!(matches!(
            error,
            ContractError::InvalidStateTransition { .. }
        ));
        assert!(error.to_string().contains("completed"));
        assert!(error.to_string().contains("active"));
    }

    #[test]
    fn test_not_found_helper() {
        let error = ContractError::not_found("Session", "my-session");
        assert!(matches!(error, ContractError::NotFound { .. }));
        assert!(error.to_string().contains("Session"));
        assert!(error.to_string().contains("my-session"));
    }

    #[test]
    fn test_combine_empty() {
        let result = ContractError::combine(vec![]);
        assert!(result.is_none());
    }

    #[test]
    fn test_combine_single() {
        let error = ContractError::not_found("Session", "test");
        let result = ContractError::combine(vec![error]);
        assert!(result.is_some());
        let combined = result;
        assert!(matches!(combined, Some(ContractError::NotFound { .. })));
    }

    #[test]
    fn test_combine_multiple() {
        let errors = vec![
            ContractError::not_found("Session", "test1"),
            ContractError::not_found("Session", "test2"),
        ];
        let result = ContractError::combine(errors);
        assert!(result.is_some());
        if let Some(combined) = result {
            assert!(matches!(combined, ContractError::Multiple(_)));
            assert!(combined.to_string().contains("test1"));
            assert!(combined.to_string().contains("test2"));
        }
    }
}
