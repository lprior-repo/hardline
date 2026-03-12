//! Contract verification macros for runtime checking in debug builds.
//!
//! These macros provide ergonomic ways to verify contracts with zero
//! overhead in release builds.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
// Types are used in macros but linted as unused
#![allow(unused_imports)]

use crate::cli_contracts::{ContractError, Invariant, Postcondition, Precondition};

/// Verify a precondition at runtime.
///
/// In debug builds, this checks the condition and returns an error if false.
/// In release builds, this is a no-op for performance.
///
/// # Example
/// ```ignore
/// fn create_session(name: &str) -> Result<Session, ContractError> {
///     contract_precondition!(!name.is_empty(), "name_not_empty", "Session name must not be empty");
///     // ... create session
///     Ok(session)
/// }
/// ```
#[macro_export]
macro_rules! contract_precondition {
    ($condition:expr, $name:expr, $description:expr) => {{
        #[cfg(debug_assertions)]
        {
            if !($condition) {
                return Err($crate::cli_contracts::ContractError::PreconditionFailed {
                    name: $name,
                    description: $description,
                });
            }
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = ($condition);
        }
    }};
}

/// Verify a postcondition at runtime.
///
/// In debug builds, this checks the condition and returns an error if false.
/// In release builds, this is a no-op for performance.
///
/// # Example
/// ```ignore
/// fn create_session(name: &str) -> Result<Session, ContractError> {
///     let session = Session::new(name);
///     contract_postcondition!(session.name() == name, "name_matches", "Session name must match input");
///     Ok(session)
/// }
/// ```
#[macro_export]
macro_rules! contract_postcondition {
    ($condition:expr, $name:expr, $description:expr) => {{
        #[cfg(debug_assertions)]
        {
            if !($condition) {
                return Err($crate::cli_contracts::ContractError::PostconditionFailed {
                    name: $name,
                    description: $description,
                });
            }
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = ($condition);
        }
    }};
}

/// Assert an invariant at runtime.
///
/// In debug builds, this checks the condition and returns an error if false.
/// In release builds, this is a no-op for performance.
///
/// # Example
/// ```ignore
/// fn process_queue(queue: &mut Queue) -> Result<(), ContractError> {
///     contract_invariant!(queue.len() <= queue.max_capacity(), "queue_capacity", "Queue must not exceed capacity");
///     // ... process
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! contract_invariant {
    ($condition:expr, $name:expr, $description:expr) => {{
        #[cfg(debug_assertions)]
        {
            if !($condition) {
                return Err($crate::cli_contracts::ContractError::InvariantViolation {
                    name: $name,
                    description: $description,
                });
            }
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = ($condition);
        }
    }};
}

/// Verify all preconditions from a slice.
///
/// # Example
/// ```ignore
/// let preconditions = vec![
///     Precondition::new("name_valid", "Name must be valid"),
///     Precondition::new("workspace_exists", "Workspace must exist"),
/// ];
/// verify_preconditions![
///     (is_name_valid(name), &preconditions[0]),
///     (workspace.exists(), &preconditions[1]),
/// ]?;
/// ```
#[macro_export]
macro_rules! verify_preconditions {
    ($(($condition:expr, $precondition:expr)),* $(,)?) => {{
        #[cfg(debug_assertions)]
        {
            let mut errors: Vec<$crate::cli_contracts::ContractError> = Vec::new();
            $(
                if !($condition) {
                    errors.push($crate::cli_contracts::ContractError::PreconditionFailed {
                        name: $precondition.name,
                        description: $precondition.description,
                    });
                }
            )*
            if !errors.is_empty() {
                return Err($crate::cli_contracts::ContractError::combine(errors)
                    .unwrap_or_else(|| $crate::cli_contracts::ContractError::Multiple("Unknown errors".to_string())));
            }
        }
        #[cfg(not(debug_assertions))]
        {
            $(let _ = ($condition);)*
        }
        Ok(())
    }};
}

/// Document a contract for API documentation.
///
/// This macro generates documentation comments that describe the contract.
/// It has no runtime effect.
///
/// # Example
/// ```ignore
/// contract_doc! {
///     "Creates a new session",
///     preconditions: [
///         "name must be non-empty",
///         "name must start with a letter",
///     ],
///     postconditions: [
///         "session exists in database",
///         "session has status 'creating'",
///     ],
///     invariants: [
///         "session name is unique",
///     ],
/// }
/// pub fn create_session(name: &str) -> Result<Session, Error> {
///     // ...
/// }
/// ```
#[macro_export]
macro_rules! contract_doc {
    ($description:expr, preconditions: [$($pre:expr),*], postconditions: [$($post:expr),*], invariants: [$($inv:expr),*]) => {
        // This macro is for documentation only and has no runtime effect
    };
}

/// Run a contract check block that collects all errors.
///
/// # Example
/// ```ignore
/// let result = contract_check! {
///     require(!name.is_empty(), "name_not_empty", "Name must not be empty"),
///     require(name.len() <= 64, "name_length", "Name must be at most 64 chars"),
///     require(workspace.exists(), "workspace_exists", "Workspace must exist"),
/// };
/// result?;
/// ```
#[macro_export]
macro_rules! contract_check {
    ($($require:ident ($condition:expr, $name:expr, $description:expr)),* $(,)?) => {{
        #[cfg(debug_assertions)]
        {
            let mut errors: Vec<$crate::cli_contracts::ContractError> = Vec::new();
            $(
                contract_check!(@check $require, $condition, $name, $description, &mut errors);
            )*
            $crate::cli_contracts::ContractError::combine(errors)
        }
        #[cfg(not(debug_assertions))]
        {
            $(let _ = ($condition);)*
            None
        }
    }};
    (@check require, $condition:expr, $name:expr, $description:expr, $errors:expr) => {
        if !($condition) {
            $errors.push($crate::cli_contracts::ContractError::PreconditionFailed {
                name: $name,
                description: $description,
            });
        }
    };
    (@check invariant, $condition:expr, $name:expr, $description:expr, $errors:expr) => {
        if !($condition) {
            $errors.push($crate::cli_contracts::ContractError::InvariantViolation {
                name: $name,
                description: $description,
            });
        }
    };
    (@check ensure, $condition:expr, $name:expr, $description:expr, $errors:expr) => {
        if !($condition) {
            $errors.push($crate::cli_contracts::ContractError::PostconditionFailed {
                name: $name,
                description: $description,
            });
        }
    };
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precondition_macro_passes() {
        fn test_fn() -> Result<(), ContractError> {
            contract_precondition!(true, "test", "test precondition");
            Ok(())
        }
        assert!(test_fn().is_ok());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_precondition_macro_fails_in_debug() {
        fn test_fn() -> Result<(), ContractError> {
            contract_precondition!(false, "test", "test precondition");
            Ok(())
        }
        assert!(test_fn().is_err());
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn test_precondition_macro_passes_in_release() {
        fn test_fn() -> Result<(), ContractError> {
            contract_precondition!(false, "test", "test precondition");
            Ok(())
        }
        assert!(test_fn().is_ok());
    }

    #[test]
    fn test_postcondition_macro_passes() {
        fn test_fn() -> Result<(), ContractError> {
            contract_postcondition!(true, "test", "test postcondition");
            Ok(())
        }
        assert!(test_fn().is_ok());
    }

    #[test]
    fn test_invariant_macro_passes() {
        fn test_fn() -> Result<(), ContractError> {
            contract_invariant!(true, "test", "test invariant");
            Ok(())
        }
        assert!(test_fn().is_ok());
    }

    #[test]
    fn test_contract_check_no_errors() {
        let result: Option<crate::cli_contracts::ContractError> = contract_check! {
            require(true, "test1", "test 1"),
            require(true, "test2", "test 2"),
        };
        assert!(result.is_none());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_contract_check_with_errors() {
        let result = contract_check! {
            require(true, "test1", "test 1"),
            require(false, "test2", "test 2"),
        };
        assert!(result.is_some());
    }
}
