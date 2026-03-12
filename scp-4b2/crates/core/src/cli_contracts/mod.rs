//! KIRK Contracts for CLI Objects
//!
//! This module defines design-by-contract patterns for CLI objects:
//! - `TaskContracts`: Beads task management
//! - `SessionContracts`: Parallel workspace sessions
//! - `AgentContracts`: Agent coordination
//! - `StatusContracts`: Status reporting
//! - `ConfigContracts`: Configuration management
//! - `DoctorContracts`: System diagnostics
//!
//! KIRK stands for:
//! - **K**nown preconditions: What must be true before an operation
//! - **I**nvariants: What must always remain true
//! - **R**eturn guarantees: What the operation guarantees on success
//! - **K**nown postconditions: What will be true after an operation
//!
//! Contracts are checked at runtime in debug builds and can be verified
//! at compile-time where possible.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

mod agent;
mod config;
mod doctor;
mod domain_types;
mod error;
mod macros;
mod session;
mod status;
mod task;

pub use agent::AgentContracts;
pub use config::ConfigContracts;
pub use doctor::{CheckStatus, DoctorStatus};
pub use domain_types::{
    AgentId, AgentStatus, AgentType, ConfigKey, ConfigScope, ConfigValue, FileStatus, Limit,
    NonEmptyString, OutputFormat, Priority, SessionName, SessionStatus, TaskId, TaskPriority,
    TaskStatus, TimeoutSeconds,
};
pub use error::ContractError;
#[allow(unused_imports)]
pub use macros::*;
pub use session::SessionContracts;
pub use status::StatusContracts;
pub use task::TaskContracts;

// ═══════════════════════════════════════════════════════════════════════════
// CORE CONTRACT TRAIT
// ═══════════════════════════════════════════════════════════════════════════

/// Core contract trait defining KIRK pattern for all CLI operations.
///
/// Every CLI command should implement this trait to define:
/// - Preconditions that must be satisfied before execution
/// - Invariants that must hold throughout execution
/// - Return guarantees about the result
/// - Postconditions that will be true after execution
///
/// # Type Parameters
/// - `T`: The input/context type for the operation
/// - `R`: The return type of the operation
///
/// # Example
/// ```ignore
/// struct CreateSessionContracts;
///
/// impl Contract<CreateSessionInput, Session> for CreateSessionContracts {
///     fn preconditions(input: &CreateSessionInput) -> Result<(), ContractError> {
///         // Verify name is valid
///         // Verify workspace is available
///         Ok(())
///     }
///
///     fn invariants(_input: &CreateSessionInput) -> Vec<Invariant> {
///         vec![Invariant::new("session_name_unique", "Session name must be unique")]
///     }
///
///     fn postconditions(input: &CreateSessionInput, result: &Session) -> Result<(), ContractError> {
///         // Verify session was created
///         // Verify name matches
///         Ok(())
///     }
/// }
/// ```
pub trait Contract<T, R: 'static> {
    /// Verify all preconditions before executing the operation.
    ///
    /// # Errors
    /// Returns `ContractError::PreconditionFailed` if any precondition is not met.
    ///
    /// # Arguments
    /// * `input` - The input/context for the operation
    fn preconditions(input: &T) -> Result<(), ContractError>;

    /// Get the list of invariants that must hold throughout the operation.
    ///
    /// Invariants are properties that must be true before, during, and after
    /// the operation completes successfully.
    ///
    /// # Arguments
    /// * `input` - The input/context for the operation
    fn invariants(input: &T) -> Vec<Invariant>;

    /// Verify all postconditions after the operation completes.
    ///
    /// # Errors
    /// Returns `ContractError::PostconditionFailed` if any postcondition is not met.
    ///
    /// # Arguments
    /// * `input` - The input/context for the operation
    /// * `result` - The result of the operation
    fn postconditions(input: &T, result: &R) -> Result<(), ContractError>;

    /// Verify that invariants still hold after the operation.
    ///
    /// # Errors
    /// Returns `ContractError::InvariantViolation` if any invariant is violated.
    ///
    /// # Arguments
    /// * `input` - The input/context for the operation
    /// * `result` - The result of the operation
    fn verify_invariants(input: &T, result: &R) -> Result<(), ContractError> {
        let invariants = Self::invariants(input);
        for invariant in invariants {
            if !invariant.verify(result) {
                return Err(ContractError::InvariantViolation {
                    name: invariant.name,
                    description: invariant.description,
                });
            }
        }
        Ok(())
    }
}

/// Represents an invariant that must always hold.
#[derive(Debug, Clone)]
pub struct Invariant {
    /// Name of the invariant (`snake_case`)
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
    /// Verification function (returns true if invariant holds)
    verifier: fn(&dyn std::any::Any) -> bool,
}

impl Invariant {
    /// Create a new invariant with a verification function.
    ///
    /// # Arguments
    /// * `name` - Unique identifier for the invariant
    /// * `description` - Human-readable description
    /// * `verifier` - Function that returns true if the invariant holds
    #[must_use]
    pub const fn new(
        name: &'static str,
        description: &'static str,
        verifier: fn(&dyn std::any::Any) -> bool,
    ) -> Self {
        Self {
            name,
            description,
            verifier,
        }
    }

    /// Create an invariant that always passes (for documentation purposes).
    ///
    /// Use this for invariants that cannot be programmatically verified
    /// but should still be documented.
    #[must_use]
    pub const fn documented(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            description,
            verifier: |_| true,
        }
    }

    /// Verify the invariant against a value.
    ///
    /// # Arguments
    /// * `value` - The value to verify against
    #[must_use]
    pub fn verify(&self, value: &dyn std::any::Any) -> bool {
        (self.verifier)(value)
    }
}

/// A precondition that must be true before an operation.
#[derive(Debug, Clone)]
pub struct Precondition {
    /// Name of the precondition
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
}

impl Precondition {
    /// Create a new precondition.
    #[must_use]
    pub const fn new(name: &'static str, description: &'static str) -> Self {
        Self { name, description }
    }
}

/// A postcondition that will be true after an operation.
#[derive(Debug, Clone)]
pub struct Postcondition {
    /// Name of the postcondition
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
}

impl Postcondition {
    /// Create a new postcondition.
    #[must_use]
    pub const fn new(name: &'static str, description: &'static str) -> Self {
        Self { name, description }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT VERIFICATION
// ═══════════════════════════════════════════════════════════════════════════

/// Verify a precondition and return an error if it fails.
///
/// # Arguments
/// * `condition` - The condition to check
/// * `precondition` - The precondition metadata
///
/// # Errors
/// Returns `ContractError::PreconditionFailed` if condition is false
#[inline]
pub const fn require_precondition(
    condition: bool,
    precondition: &Precondition,
) -> Result<(), ContractError> {
    if condition {
        Ok(())
    } else {
        Err(ContractError::PreconditionFailed {
            name: precondition.name,
            description: precondition.description,
        })
    }
}

/// Verify a postcondition and return an error if it fails.
///
/// # Arguments
/// * `condition` - The condition to check
/// * `postcondition` - The postcondition metadata
///
/// # Errors
/// Returns `ContractError::PostconditionFailed` if condition is false
#[inline]
pub const fn require_postcondition(
    condition: bool,
    postcondition: &Postcondition,
) -> Result<(), ContractError> {
    if condition {
        Ok(())
    } else {
        Err(ContractError::PostconditionFailed {
            name: postcondition.name,
            description: postcondition.description,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precondition_passes() {
        let precondition = Precondition::new("test_precondition", "A test precondition");
        let result = require_precondition(true, &precondition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_precondition_fails() {
        let precondition = Precondition::new("test_precondition", "A test precondition");
        let result = require_precondition(false, &precondition);
        assert!(result.is_err());
        if let Err(ContractError::PreconditionFailed { name, .. }) = result {
            assert_eq!(name, "test_precondition");
        } else {
            panic!("Expected PreconditionFailed error");
        }
    }

    #[test]
    fn test_postcondition_passes() {
        let postcondition = Postcondition::new("test_postcondition", "A test postcondition");
        let result = require_postcondition(true, &postcondition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_postcondition_fails() {
        let postcondition = Postcondition::new("test_postcondition", "A test postcondition");
        let result = require_postcondition(false, &postcondition);
        assert!(result.is_err());
        if let Err(ContractError::PostconditionFailed { name, .. }) = result {
            assert_eq!(name, "test_postcondition");
        } else {
            panic!("Expected PostconditionFailed error");
        }
    }

    #[test]
    fn test_invariant_documented() {
        let invariant = Invariant::documented("test_invariant", "A documented invariant");
        assert_eq!(invariant.name, "test_invariant");
        assert_eq!(invariant.description, "A documented invariant");
        // Documented invariants always verify to true
        assert!(invariant.verify(&() as &dyn std::any::Any));
    }
}
