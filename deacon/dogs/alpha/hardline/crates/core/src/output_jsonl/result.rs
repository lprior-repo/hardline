//! Result output types
//!
//! Provides operation result reporting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::output_jsonl::domain_types::{Message, Outcome};
use crate::output_jsonl::errors::OutputLineError;

/// Result output line for operation results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResultOutput {
    pub kind: ResultKind,
    pub outcome: Outcome,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

/// Kind of result being reported.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    Command,
    Operation,
    Assessment,
    Recovery,
}

impl ResultOutput {
    /// Create a successful result output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if `message` is blank.
    pub fn success(kind: ResultKind, message: Message) -> Result<Self, OutputLineError> {
        Ok(Self {
            kind,
            outcome: Outcome::Success,
            message,
            data: None,
            timestamp: Utc::now(),
        })
    }

    /// Create a failed result output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if `message` is blank.
    pub fn failure(kind: ResultKind, message: Message) -> Result<Self, OutputLineError> {
        Ok(Self {
            kind,
            outcome: Outcome::Failure,
            message,
            data: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_data(self, data: serde_json::Value) -> Self {
        Self {
            data: Some(data),
            ..self
        }
    }
}
