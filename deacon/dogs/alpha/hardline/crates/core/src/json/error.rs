//! JSON error types and basic error structures

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

    // ── JsonSuccess ──────────────────────────────────────────────────────────

    #[test]
    fn test_json_success_new() {
        let s = JsonSuccess::new("hello");
        assert!(s.success);
        assert_eq!(s.data, "hello");
    }

    #[test]
    fn test_json_success_with_struct() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Payload {
            value: i32,
        }
        let s = JsonSuccess::new(Payload { value: 42 });
        assert!(s.success);
        assert_eq!(s.data.value, 42);
    }

    #[test]
    fn test_json_success_serde_roundtrip() {
        let s = JsonSuccess::new("test data");
        let json = serde_json::to_string(&s).expect("serialize ok");
        let deserialized: JsonSuccess<String> =
            serde_json::from_str(&json).expect("deserialize ok");
        assert!(deserialized.success);
        assert_eq!(deserialized.data, "test data");
    }

    #[test]
    fn test_json_success_with_unit() {
        let s = JsonSuccess::new(());
        assert!(s.success);
    }

    // ── JsonError construction ───────────────────────────────────────────────

    #[test]
    fn test_json_error_new() {
        let err = JsonError::new("TEST_CODE", "Something went wrong");
        assert!(!err.success);
        assert_eq!(err.error.code, "TEST_CODE");
        assert_eq!(err.error.message, "Something went wrong");
        assert_eq!(err.error.exit_code, 4); // default
        assert!(err.error.details.is_none());
        assert!(err.error.suggestion.is_none());
    }

    #[test]
    fn test_json_error_with_details() {
        let err = JsonError::new("ERR", "fail")
            .with_details(serde_json::json!({"key": "value"}));
        assert!(err.error.details.is_some());
        let details = err.error.details.expect("has details");
        assert_eq!(details["key"], "value");
    }

    #[test]
    fn test_json_error_with_suggestion() {
        let err = JsonError::new("ERR", "fail")
            .with_suggestion("Try again");
        assert_eq!(
            err.error.suggestion.as_deref(),
            Some("Try again")
        );
    }

    #[test]
    fn test_json_error_with_exit_code() {
        let err = JsonError::new("ERR", "fail").with_exit_code(1);
        assert_eq!(err.error.exit_code, 1);
    }

    #[test]
    fn test_json_error_chained_builders() {
        let err = JsonError::new("CHAIN", "chained")
            .with_exit_code(2)
            .with_suggestion("fix it")
            .with_details(serde_json::json!({"ctx": 42}));
        assert_eq!(err.error.exit_code, 2);
        assert_eq!(err.error.suggestion.as_deref(), Some("fix it"));
        assert!(err.error.details.is_some());
    }

    // ── JsonError default ────────────────────────────────────────────────────

    #[test]
    fn test_json_error_default() {
        let err = JsonError::default();
        assert!(!err.success);
        assert_eq!(err.error.code, "UNKNOWN");
        assert_eq!(err.error.message, "An unknown error occurred");
        assert_eq!(err.error.exit_code, 4);
    }

    // ── JsonError to_json ────────────────────────────────────────────────────

    #[test]
    fn test_json_error_to_json() {
        let err = JsonError::new("ERR", "test error");
        let json = err.to_json().expect("to_json ok");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse ok");
        assert!(!parsed["success"].as_bool().expect("bool"));
        assert_eq!(parsed["error"]["code"], "ERR");
        assert_eq!(parsed["error"]["message"], "test error");
    }

    // ── JsonError serde roundtrip ────────────────────────────────────────────

    #[test]
    fn test_json_error_serde_roundtrip() {
        let err = JsonError::new("CODE_X", "error msg")
            .with_exit_code(3)
            .with_suggestion("do X");
        let json = serde_json::to_string(&err).expect("serialize ok");
        let deserialized: JsonError =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.error.code, "CODE_X");
        assert_eq!(deserialized.error.message, "error msg");
        assert_eq!(deserialized.error.exit_code, 3);
        assert_eq!(
            deserialized.error.suggestion.as_deref(),
            Some("do X")
        );
    }

    #[test]
    fn test_json_error_serde_skips_none_fields() {
        let err = JsonError::new("ERR", "msg");
        let json_val = serde_json::to_value(&err).expect("serialize ok");
        let error_obj = json_val["error"].as_object().expect("error object");
        assert!(!error_obj.contains_key("details"));
        assert!(!error_obj.contains_key("suggestion"));
    }

    #[test]
    fn test_json_error_serde_includes_details() {
        let err = JsonError::new("ERR", "msg")
            .with_details(serde_json::json!({"field": "val"}));
        let json_val = serde_json::to_value(&err).expect("serialize ok");
        let error_obj = json_val["error"].as_object().expect("error object");
        assert!(error_obj.contains_key("details"));
    }

    // ── JsonError from crate::Error ──────────────────────────────────────────

    #[test]
    fn test_json_error_from_error_ref() {
        let app_err = crate::error::Error::invalid_state("test state error".to_string());
        let json_err = JsonError::from(&app_err);
        assert!(!json_err.success);
        assert!(json_err.error.code.contains("INVALID_STATE") || json_err.error.code.contains("invalid"));
    }

    #[test]
    fn test_json_error_from_error_owned() {
        let app_err = crate::error::Error::not_found("resource not found".to_string());
        let json_err = JsonError::from(app_err);
        assert!(!json_err.success);
    }

    // ── ErrorDetail from_error ───────────────────────────────────────────────

    #[test]
    fn test_error_detail_from_error() {
        let app_err = crate::error::Error::invalid_state("state err".to_string());
        let detail = ErrorDetail::from_error(&app_err);
        assert_eq!(detail.message, "state err");
        // code should be present
        assert!(!detail.code.is_empty());
    }

    // ── Debug ────────────────────────────────────────────────────────────────

    #[test]
    fn test_json_error_debug() {
        let err = JsonError::new("DBG", "debug test");
        let debug = format!("{err:?}");
        assert!(debug.contains("DBG"));
    }

    #[test]
    fn test_error_detail_debug() {
        let detail = ErrorDetail {
            code: "C".to_string(),
            message: "msg".to_string(),
            exit_code: 1,
            details: None,
            suggestion: None,
        };
        let debug = format!("{detail:?}");
        assert!(debug.contains("C"));
    }
}
