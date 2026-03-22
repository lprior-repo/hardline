//! Summary output types
//!
//! Provides summary information about the current operation or state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::output_jsonl::domain_types::Message;
use crate::output_jsonl::errors::OutputLineError;

/// Summary output line containing a message with optional details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Summary {
    #[serde(rename = "type")]
    pub type_field: SummaryType,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

/// Type of summary being emitted.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SummaryType {
    Status,
    Count,
    Info,
}

impl Summary {
    /// Create a new summary line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if `message` is blank.
    pub fn new(type_field: SummaryType, message: Message) -> Result<Self, OutputLineError> {
        Ok(Self {
            type_field,
            message,
            details: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_details(self, details: String) -> Self {
        Self {
            details: Some(details),
            ..self
        }
    }
}
