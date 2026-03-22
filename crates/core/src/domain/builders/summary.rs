// //! Summary builder
//!
//! Builder for `Summary` with fluent API.

use chrono::{DateTime, Utc};

use crate::output_jsonl::{domain_types::Message, Summary, SummaryType as OutputSummaryType};

/// Builder for [Summary] with fluent API
///
/// # Required Fields
/// - `type_field`: Summary type
/// - `message`: Summary message
///
/// # Optional Fields
/// - `details`: Additional details
/// - `timestamp`: Timestamp (defaults to now)
#[derive(Debug, Clone)]
pub struct SummaryBuilder {
    // Required fields
    type_field: Option<OutputSummaryType>,
    message: Option<Message>,

    // Optional fields
    details: Option<String>,
    timestamp: Option<DateTime<Utc>>,
}

impl Default for SummaryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SummaryBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            type_field: None,
            message: None,
            details: None,
            timestamp: None,
        }
    }

    /// Set the summary type (required)
    #[must_use]
    pub const fn type_field(mut self, type_field: OutputSummaryType) -> Self {
        self.type_field = Some(type_field);
        self
    }

    /// Set the message (required)
    #[must_use]
    pub fn message(mut self, message: Message) -> Self {
        self.message = Some(message);
        self
    }

    /// Set additional details (optional)
    #[must_use]
    pub fn details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// Set the timestamp (optional)
    #[must_use]
    pub const fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Build the Summary
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<Summary, super::errors::BuilderError> {
        let type_field = self
            .type_field
            .ok_or(super::errors::BuilderError::MissingRequired {
                field: "type_field",
            })?;
        let message = self
            .message
            .ok_or(super::errors::BuilderError::MissingRequired { field: "message" })?;

        Ok(Summary {
            type_field,
            message,
            details: self.details,
            timestamp: self.timestamp.unwrap_or_else(Utc::now),
        })
    }
}
