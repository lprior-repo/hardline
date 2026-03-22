//! JSON error types and basic error structures

use serde::{Deserialize, Serialize};

use super::error_code::ErrorCode;
use super::error_mapping::{classify_exit_code, map_error_to_parts};

/// Standard JSON success response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSuccess<T> {
    pub success: bool,
    #[serde(flatten)]
    pub data: T,
}

impl<T> JsonSuccess<T> {
    /// Create a new success response
    pub const fn new(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Standard JSON error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonError {
    pub success: bool,
    pub error: ErrorDetail,
}

impl Default for JsonError {
    fn default() -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: "UNKNOWN".to_string(),
                message: "An unknown error occurred".to_string(),
                exit_code: 4,
                details: None,
                suggestion: None,
            },
        }
    }
}

/// Detailed error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Machine-readable error code (`SCREAMING_SNAKE_CASE`)
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Semantic exit code (1-4)
    pub exit_code: i32,
    /// Optional additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Optional suggestion for resolution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl JsonError {
    /// Create a new JSON error with just a code and message
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                exit_code: 4, // Default to unknown/external error
                details: None,
                suggestion: None,
            },
        }
    }

    /// Add details to the error
    #[must_use]
    pub fn with_details(self, details: serde_json::Value) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: self.error.code,
                message: self.error.message,
                exit_code: self.error.exit_code,
                details: Some(details),
                suggestion: self.error.suggestion,
            },
        }
    }

    /// Add a suggestion to the error
    #[must_use]
    pub fn with_suggestion(self, suggestion: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: self.error.code,
                message: self.error.message,
                exit_code: self.error.exit_code,
                details: self.error.details,
                suggestion: Some(suggestion.into()),
            },
        }
    }

    /// Set exit code for this error
    #[must_use]
    pub const fn with_exit_code(self, exit_code: i32) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: self.error.code,
                message: self.error.message,
                exit_code,
                details: self.error.details,
                suggestion: self.error.suggestion,
            },
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> crate::error::Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| crate::error::Error::JsonParse(e))
    }
}

impl ErrorDetail {
    /// Construct an `ErrorDetail` from an Error.
    ///
    /// This is the standard way to convert errors to JSON-serializable format.
    #[must_use]
    pub fn from_error(error: &crate::error::Error) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.to_string(),
            exit_code: classify_exit_code(error),
            details: None,
            suggestion: error.suggestion(),
        }
    }
}

impl From<&crate::error::Error> for JsonError {
    fn from(err: &crate::error::Error) -> Self {
        let (code, message, suggestion) = map_error_to_parts(err);

        let json_error = Self::new(code, message);
        let json_error = match suggestion {
            Some(sugg) => json_error.with_suggestion(sugg),
            None => json_error,
        };
        // Override exit code to match the error classification
        let json_error = json_error.with_exit_code(classify_exit_code(err));
        JsonError {
            success: json_error.success,
            error: ErrorDetail {
                code: json_error.error.code,
                message: json_error.error.message,
                exit_code: classify_exit_code(err),
                details: json_error.error.details,
                suggestion: json_error.error.suggestion,
            },
        }
    }
}

impl From<crate::error::Error> for JsonError {
    fn from(err: crate::error::Error) -> Self {
        Self::from(&err)
    }
}
