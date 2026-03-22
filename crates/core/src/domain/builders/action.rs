// //! Action builder
//!
//! Builder for `Action` with fluent API.

use chrono::{DateTime, Utc};

use crate::output_jsonl::{
    Action, ActionResult as OutputActionResult, ActionStatus, ActionTarget,
    ActionVerb as OutputActionVerb,
};

/// Builder for [Action] with fluent API
///
/// # Required Fields
/// - `verb`: Action verb
/// - `target`: Action target
/// - `status`: Action status
///
/// # Optional Fields
/// - `result`: Action result (defaults to Pending)
/// - `timestamp`: Timestamp (defaults to now)
#[derive(Debug, Clone)]
pub struct ActionBuilder {
    // Required fields
    verb: Option<OutputActionVerb>,
    target: Option<ActionTarget>,
    status: Option<ActionStatus>,

    // Optional fields
    result: Option<OutputActionResult>,
    timestamp: Option<DateTime<Utc>>,
}

impl Default for ActionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            verb: None,
            target: None,
            status: None,
            result: None,
            timestamp: None,
        }
    }

    /// Set the action verb (required)
    #[must_use]
    pub fn verb(mut self, verb: OutputActionVerb) -> Self {
        self.verb = Some(verb);
        self
    }

    /// Set the action target (required)
    #[must_use]
    pub fn target(mut self, target: ActionTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// Set the action status (required)
    #[must_use]
    pub const fn status(mut self, status: ActionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the action result (optional)
    #[must_use]
    pub fn result(mut self, result: OutputActionResult) -> Self {
        self.result = Some(result);
        self
    }

    /// Set a completed result with a message (optional)
    #[must_use]
    pub fn with_completed_result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(OutputActionResult::Completed {
            result: result.into(),
        });
        self
    }

    /// Set the timestamp (optional)
    #[must_use]
    pub const fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Build the Action
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<Action, super::errors::BuilderError> {
        let verb = self
            .verb
            .ok_or(super::errors::BuilderError::MissingRequired { field: "verb" })?;
        let target = self
            .target
            .ok_or(super::errors::BuilderError::MissingRequired { field: "target" })?;
        let status = self
            .status
            .ok_or(super::errors::BuilderError::MissingRequired { field: "status" })?;

        Ok(Action {
            verb,
            target,
            status,
            result: self.result.unwrap_or(OutputActionResult::Pending),
            timestamp: self.timestamp.unwrap_or_else(Utc::now),
        })
    }
}
