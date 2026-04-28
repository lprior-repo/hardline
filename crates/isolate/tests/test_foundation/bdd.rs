#![allow(clippy::missing_const_for_fn, clippy::uninlined_format_args)]
//! BDD (Behavior-Driven Development) Test Framework
//!
//! This module provides a simple, zero-dependency BDD framework for writing
//! tests in Given/When/Then format without requiring cucumber-rs.
//!
//! ## Design Principles
//!
//! - Zero panics: All steps return `Result<(), E>`
//! - Clear failure messages: Shows which step failed with context
//! - Functional style: Uses closures for step definitions
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::test_foundation::bdd::{BddContext, BddError, StepType};
//!
//! #[test]
//! fn test_session_creation() -> Result<(), String> {
//!     // Given
//!     let name = "my-session";
//!
//!     // When
//!     let session = create_session(name)?;
//!
//!     // Then
//!     assert!(session.is_active());
//!     Ok(())
//! }
//! ```
//!

use std::fmt::Debug;

/// Error type for BDD step failures.
#[derive(Debug)]
pub struct BddError {
    /// The step type that failed (Given, When, or Then)
    pub step_type: StepType,
    /// The step description
    pub description: String,
    /// The underlying error
    pub cause: String,
}

impl std::fmt::Display for BddError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BDD Step Failed: {:?} '{}' - {}",
            self.step_type, self.description, self.cause
        )
    }
}

impl std::error::Error for BddError {}

/// The type of BDD step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepType {
    /// Given step (setup/preconditions)
    Given,
    /// When step (action/execution)
    When,
    /// Then step (assertion/verification)
    Then,
}

impl std::fmt::Display for StepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Given => write!(f, "Given"),
            Self::When => write!(f, "When"),
            Self::Then => write!(f, "Then"),
        }
    }
}

/// Simple BDD context for storing state between steps.
///
/// This is a simpler alternative to the typestate-based Scenario
/// for cases where you want more flexibility.
#[derive(Debug)]
pub struct BddContext {
    /// Stored state as a type-erased value
    state: Option<String>,
}

impl BddContext {
    /// Create a new empty context.
    #[must_use]
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Store state in the context.
    pub fn set(&mut self, value: impl Into<String>) {
        self.state = Some(value.into());
    }

    /// Get the stored state.
    #[must_use]
    pub fn get(&self) -> Option<&str> {
        self.state.as_deref()
    }

    /// Clear the context.
    pub fn clear(&mut self) {
        self.state = None;
    }
}

impl Default for BddContext {
    fn default() -> Self {
        Self::new()
    }
}

/// A builder for creating BDD-style test scenarios.
///
/// This struct allows building test scenarios with clear Given/When/Then
/// structure while maintaining functional error handling.
#[derive(Debug)]
pub struct ScenarioBuilder {
    /// Name of the scenario
    pub name: String,
    /// Given description
    pub given: Option<String>,
    /// When description
    pub when: Option<String>,
    /// Then description
    pub then: Option<String>,
}

impl ScenarioBuilder {
    /// Create a new scenario builder.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            given: None,
            when: None,
            then: None,
        }
    }

    /// Set the Given step description.
    #[must_use]
    pub fn given(mut self, description: impl Into<String>) -> Self {
        self.given = Some(description.into());
        self
    }

    /// Set the When step description.
    #[must_use]
    pub fn when(mut self, description: impl Into<String>) -> Self {
        self.when = Some(description.into());
        self
    }

    /// Set the Then step description.
    #[must_use]
    pub fn then(mut self, description: impl Into<String>) -> Self {
        self.then = Some(description.into());
        self
    }

    /// Build and return the scenario description.
    #[must_use]
    pub fn build(self) -> String {
        format!(
            "Scenario: {}\n  Given: {}\n  When: {}\n  Then: {}",
            self.name,
            self.given.as_deref().unwrap_or("N/A"),
            self.when.as_deref().unwrap_or("N/A"),
            self.then.as_deref().unwrap_or("N/A")
        )
    }
}

/// Create a BDD error for a Given step failure.
#[must_use]
pub fn given_error(description: impl Into<String>, cause: impl Into<String>) -> BddError {
    BddError {
        step_type: StepType::Given,
        description: description.into(),
        cause: cause.into(),
    }
}

/// Create a BDD error for a When step failure.
#[must_use]
pub fn when_error(description: impl Into<String>, cause: impl Into<String>) -> BddError {
    BddError {
        step_type: StepType::When,
        description: description.into(),
        cause: cause.into(),
    }
}

/// Create a BDD error for a Then step failure.
#[must_use]
pub fn then_error(description: impl Into<String>, cause: impl Into<String>) -> BddError {
    BddError {
        step_type: StepType::Then,
        description: description.into(),
        cause: cause.into(),
    }
}

/// Helper macro for creating BDD-style tests.
///
/// # Example
///
/// ```rust,ignore
/// bdd_test! {
///     scenario: "User can create a session",
///     given: "a valid session name" => || Ok("my-session".to_string()),
///     when: "I create the session" => |name: String| Ok(Session::new(name)),
///     then: "the session should be active" => |session: Session| {
///         assert!(session.is_active());
///         Ok(())
///     }
/// }
/// ```
#[macro_export]
macro_rules! bdd_test {
    (
        scenario: $name:expr,
        given: $given_desc:expr => $given_fn:expr,
        when: $when_desc:expr => $when_fn:expr,
        then: $then_desc:expr => $then_fn:expr
    ) => {
        #[test]
        fn bdd_scenario() -> Result<(), Box<dyn std::error::Error>> {
            // Given
            let given_result = $given_fn()
                .map_err(|e: String| format!("Given '{}' failed: {}", $given_desc, e))?;

            // When
            let when_result = $when_fn(given_result)
                .map_err(|e: String| format!("When '{}' failed: {}", $when_desc, e))?;

            // Then
            $then_fn(when_result)
                .map_err(|e: String| format!("Then '{}' failed: {}", $then_desc, e))?;

            Ok(())
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bdd_context() {
        let mut ctx = BddContext::new();
        assert!(ctx.get().is_none());

        ctx.set("test value");
        assert_eq!(ctx.get(), Some("test value"));

        ctx.clear();
        assert!(ctx.get().is_none());
    }

    #[test]
    fn test_step_type_display() {
        assert_eq!(format!("{}", StepType::Given), "Given");
        assert_eq!(format!("{}", StepType::When), "When");
        assert_eq!(format!("{}", StepType::Then), "Then");
    }

    #[test]
    fn test_bdd_error_display() {
        let error = BddError {
            step_type: StepType::Given,
            description: "a valid session".to_string(),
            cause: "session name was empty".to_string(),
        };

        let display = format!("{}", error);
        assert!(display.contains("Given"));
        assert!(display.contains("a valid session"));
        assert!(display.contains("session name was empty"));
    }

    #[test]
    fn test_scenario_builder() {
        let scenario = ScenarioBuilder::new("Test Scenario")
            .given("a valid name")
            .when("I create a session")
            .then("the session should exist")
            .build();

        assert!(scenario.contains("Test Scenario"));
        assert!(scenario.contains("Given: a valid name"));
        assert!(scenario.contains("When: I create a session"));
        assert!(scenario.contains("Then: the session should exist"));
    }

    #[test]
    fn test_error_helpers() {
        let given = given_error("test", "failed");
        assert_eq!(given.step_type, StepType::Given);

        let when = when_error("test", "failed");
        assert_eq!(when.step_type, StepType::When);

        let then = then_error("test", "failed");
        assert_eq!(then.step_type, StepType::Then);
    }

    /// Example BDD test using the simple pattern
    #[test]
    fn test_simple_bdd_pattern() -> Result<(), String> {
        // Given
        let name = "test-session";

        // When
        let validated = validate_session_name_bdd(name)?;

        // Then
        assert_eq!(validated, name);
        Ok(())
    }

    /// Simple string validation for BDD test example
    fn validate_session_name_bdd(name: &str) -> Result<&str, String> {
        if name.is_empty() {
            return Err("session name cannot be empty".to_string());
        }
        if name.len() > 64 {
            return Err("session name too long".to_string());
        }
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err("session name contains invalid characters".to_string());
        }
        Ok(name)
    }
}