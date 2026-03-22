//! Warning output types
//!
//! Provides warning reporting for non-critical issues.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::output_jsonl::domain_types::{Message, WarningCode};
use crate::output_jsonl::errors::OutputLineError;

/// Warning output line for non-critical issues.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Warning {
    pub code: WarningCode,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Context>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

/// Context for a warning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Context {
    pub session: String,
    pub workspace: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional: Option<serde_json::Value>,
}

impl Warning {
    /// Create a new warning output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if `message` is blank.
    pub fn new(code: WarningCode, message: Message) -> Result<Self, OutputLineError> {
        Ok(Self {
            code,
            message,
            context: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_context(self, session: String, workspace: PathBuf) -> Self {
        Self {
            context: Some(Context {
                session,
                workspace,
                additional: None,
            }),
            ..self
        }
    }
}
