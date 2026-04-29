//! AI-first response envelope per ADR-011.
//!
//! Provides the canonical `Response<T>` wrapper that all commands use
//! for structured JSON output. Each response carries `success`, `data`,
//! `error`, and `metadata` fields so AI agents can parse results reliably.

use serde::{Deserialize, Serialize};

// ── ADR-011 Error Body ─────────────────────────────────────────────────

/// Structured error body embedded in a [`Response`] on failure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseError {
    /// SCREAMING_SNAKE_CASE machine-readable code (e.g. `"WORKSPACE_NOT_FOUND"`).
    pub code: String,
    /// Hierarchical numeric code per ADR-007 (e.g. `1001`).
    pub numeric_code: u16,
    /// Error category string (e.g. `"workspace"`).
    pub category: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional structured details from the error's context map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Optional suggested fix for the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<ResponseFix>,
}

/// A suggested fix for an error, with a command the user can run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseFix {
    /// Shell command the user can run to resolve the error.
    pub command: String,
    /// Human-readable description of what the fix does.
    pub description: String,
    /// Risk level of running this fix.
    pub risk: FixRisk,
}

/// Risk level of a suggested fix command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixRisk {
    /// Read-only or easily reversible.
    Safe,
    /// Modifies state but recoverable.
    Moderate,
    /// Potentially destructive.
    Dangerous,
}

// ── ADR-011 Response Metadata ──────────────────────────────────────────

/// Metadata attached to every response envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMetadata {
    /// Schema / API version string.
    pub version: String,
    /// ISO 8601 timestamp of response generation.
    pub timestamp: String,
    /// Command that generated this response (e.g. `"workspace list"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Execution time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
    /// Optional request ID for tracing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

// ── ADR-011 Response Envelope ──────────────────────────────────────────

/// AI-first response envelope per ADR-011.
///
/// Every command wraps its output in this envelope so that consumers
/// (human or AI) get a uniform structure:
///
/// ```json
/// {
///   "success": true,
///   "data": { ... },
///   "error": null,
///   "metadata": { "version": "1.0.0", "timestamp": "..." }
/// }
/// ```
///
/// # Invariants
///
/// - `success == true` implies `data.is_some() && error.is_none()`
/// - `success == false` implies `data.is_none() && error.is_some()`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<T: Serialize> {
    /// Whether the operation succeeded.
    pub success: bool,
    /// The response payload on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Structured error information on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
    /// Response metadata for debugging and tracing.
    pub metadata: ResponseMetadata,
}

// ── Constructors ───────────────────────────────────────────────────────

impl<T: Serialize> Response<T> {
    /// Create a success response wrapping `data`.
    pub fn success(data: T, command: Option<&str>) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: ResponseMetadata::now(command),
        }
    }

    /// Create an error response from a domain [`crate::error::Error`].
    pub fn from_error(err: &crate::error::Error, command: Option<&str>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ResponseError::from_error(err)),
            metadata: ResponseMetadata::now(command),
        }
    }

    /// Convert a `Result<T, Error>` into a `Response<T>`.
    pub fn from_result(result: Result<T, crate::error::Error>, command: Option<&str>) -> Self {
        match result {
            Ok(data) => Self::success(data, command),
            Err(err) => Self::from_error(&err, command),
        }
    }

    /// Builder-style: set the execution time on the metadata.
    #[must_use]
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.metadata.execution_time_ms = Some(ms);
        self
    }

    /// Builder-style: set a request ID on the metadata.
    #[must_use]
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.metadata.request_id = Some(id.into());
        self
    }
}

// ── ResponseError from domain Error ────────────────────────────────────

impl ResponseError {
    /// Build a `ResponseError` from the unified domain error.
    ///
    /// Maps core's `Error` fields into the ADR-011 envelope shape.
    /// The `numeric_code` is derived from the error code's prefix mapping
    /// per ADR-007 category ranges. The `fix` field is populated from
    /// the error's `suggestion()` when available.
    fn from_error(err: &crate::error::Error) -> Self {
        let (numeric_code, category) = numeric_code_and_category(err.code());
        Self {
            code: err.code().to_string(),
            numeric_code,
            category,
            message: err.to_string(),
            details: err.context_map(),
            fix: err.suggestion().map(|s| ResponseFix {
                command: s.clone(),
                description: s,
                risk: FixRisk::Safe,
            }),
        }
    }
}

/// Derive ADR-007 numeric code and category from a SCREAMING_SNAKE_CASE code.
///
/// Category ranges (ADR-007):
/// - 1xxx: Workspace, 2xxx: Session, 3xxx: Bead, 4xxx: Queue,
/// - 5xxx: VCS, 6xxx: Stack, 7xxx: GitHub, 8xxx: Snapshot, 9xxx: Internal
fn numeric_code_and_category(code: &str) -> (u16, String) {
    let (base, category) = match code {
        c if c.starts_with("WORKSPACE_") => (1000, "workspace"),
        c if c.starts_with("SESSION_") => (2000, "session"),
        c if c.starts_with("BEAD_") || c.starts_with("INVALID_BEAD_") => (3000, "bead"),
        c if c.starts_with("QUEUE_") => (4000, "queue"),
        c if c.starts_with("VCS_")
            || c.starts_with("BRANCH_")
            || c.starts_with("COMMIT_")
            || c.starts_with("WORKING_COPY_") =>
        {
            (5000, "vcs")
        }
        c if c.starts_with("STACK_") => (6000, "stack"),
        c if c.starts_with("GITHUB_") => (7000, "github"),
        c if c.starts_with("SNAPSHOT_") => (8000, "snapshot"),
        _ => (9000, "internal"),
    };
    // Use a simple hash of the code string to produce a unique code within range.
    let hash = code.bytes().fold(0u16, |acc, b| acc.wrapping_add(b));
    let suffix = (hash % 999) + 1; // 001-999
    (base + suffix, category.to_string())
}

// ── ResponseMetadata helpers ───────────────────────────────────────────

impl ResponseMetadata {
    /// Create metadata with the current timestamp.
    fn now(command: Option<&str>) -> Self {
        Self {
            version: "1.0.0".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            command: command.map(String::from),
            execution_time_ms: None,
            request_id: None,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Response::success ───────────────────────────────────────────────

    #[test]
    fn success_sets_fields_correctly() {
        let resp: Response<String> = Response::success("hello".to_string(), Some("test"));
        assert!(resp.success);
        assert_eq!(resp.data.as_deref(), Some("hello"));
        assert!(resp.error.is_none());
        assert_eq!(resp.metadata.command.as_deref(), Some("test"));
        assert!(resp.metadata.execution_time_ms.is_none());
    }

    #[test]
    fn success_without_command() {
        let resp: Response<i32> = Response::success(42, None);
        assert!(resp.success);
        assert_eq!(resp.data, Some(42));
        assert!(resp.metadata.command.is_none());
    }

    #[test]
    fn success_serializes_to_json() {
        let resp: Response<&str> = Response::success("ok", Some("cmd"));
        let json = serde_json::to_value(&resp).expect("serialize");
        assert_eq!(json["success"], true);
        assert_eq!(json["data"], "ok");
        assert!(json.get("error").is_none() || json["error"].is_null());
    }

    #[test]
    fn success_metadata_has_version_and_timestamp() {
        let resp: Response<()> = Response::success((), Some("x"));
        assert_eq!(resp.metadata.version, "1.0.0");
        assert!(!resp.metadata.timestamp.is_empty());
    }

    // ── Response::from_error ────────────────────────────────────────────

    #[test]
    fn from_error_sets_fields_correctly() {
        let err = crate::error::Error::workspace_not_found("my-ws");
        let resp: Response<String> = Response::from_error(&err, Some("workspace get"));
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert!(resp.error.is_some());
    }

    #[test]
    fn from_error_maps_code_and_numeric_code() {
        let err = crate::error::Error::workspace_not_found("ws");
        let resp: Response<()> = Response::from_error(&err, None);
        let error = resp.error.as_ref().expect("error present");
        assert_eq!(error.code, "WORKSPACE_NOT_FOUND");
        assert!((1000..=1999).contains(&error.numeric_code));
        assert_eq!(error.category, "workspace");
    }

    #[test]
    fn from_error_maps_message() {
        let err = crate::error::Error::queue_empty();
        let resp: Response<()> = Response::from_error(&err, None);
        let error = resp.error.as_ref().expect("error present");
        assert!(error.message.contains("empty"));
    }

    #[test]
    fn from_error_includes_details_for_variants_with_context() {
        let err = crate::error::Error::workspace_not_found("ws-1");
        let resp: Response<()> = Response::from_error(&err, None);
        let error = resp.error.as_ref().expect("error present");
        let details = error.details.as_ref().expect("details present");
        assert_eq!(details["resource_type"], "workspace");
        assert_eq!(details["workspace_name"], "ws-1");
    }

    #[test]
    fn from_error_includes_fix_when_available() {
        let err = crate::error::Error::workspace_not_found("ws");
        let resp: Response<()> = Response::from_error(&err, None);
        let error = resp.error.as_ref().expect("error present");
        let fix = error.fix.as_ref().expect("fix present");
        assert!(fix.command.contains("workspace list"));
        assert_eq!(fix.risk, FixRisk::Safe);
        assert_eq!(fix.command, fix.description);
    }

    #[test]
    fn from_error_no_fix_for_internal_errors() {
        let err = crate::error::Error::internal("oops");
        let resp: Response<()> = Response::from_error(&err, None);
        let error = resp.error.as_ref().expect("error present");
        assert!(error.fix.is_none());
    }

    // ── Response::from_result ───────────────────────────────────────────

    #[test]
    fn from_result_ok_branch() {
        let result: crate::error::Result<i32> = Ok(42);
        let resp = Response::from_result(result, Some("math"));
        assert!(resp.success);
        assert_eq!(resp.data, Some(42));
    }

    #[test]
    fn from_result_err_branch() {
        let result: crate::error::Result<i32> =
            Err(crate::error::Error::session("ghost"));
        let resp = Response::from_result(result, Some("session get"));
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert_eq!(
            resp.error.as_ref().expect("error").code,
            "SESSION_NOT_FOUND"
        );
    }

    // ── Builder methods ─────────────────────────────────────────────────

    #[test]
    fn with_duration_sets_execution_time() {
        let resp: Response<()> = Response::success((), None).with_duration(250);
        assert_eq!(resp.metadata.execution_time_ms, Some(250));
    }

    #[test]
    fn with_request_id_sets_id() {
        let resp: Response<()> = Response::success((), None).with_request_id("req-abc");
        assert_eq!(resp.metadata.request_id.as_deref(), Some("req-abc"));
    }

    #[test]
    fn chained_builders_preserve_all_fields() {
        let resp: Response<i32> = Response::success(99, Some("cmd"))
            .with_duration(100)
            .with_request_id("uuid");
        assert_eq!(resp.data, Some(99));
        assert_eq!(resp.metadata.command.as_deref(), Some("cmd"));
        assert_eq!(resp.metadata.execution_time_ms, Some(100));
        assert_eq!(resp.metadata.request_id.as_deref(), Some("uuid"));
    }

    // ── Serde roundtrip ─────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip_success() {
        let resp = Response::success(serde_json::json!({"key": "val"}), Some("test"));
        let json = serde_json::to_string(&resp).expect("serialize");
        let deserialized: Response<serde_json::Value> =
            serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.success);
        assert_eq!(deserialized.data.as_ref().unwrap()["key"], "val");
    }

    #[test]
    fn serde_roundtrip_error() {
        let err = crate::error::Error::workspace_locked("ws", "alice");
        let resp: Response<()> = Response::from_error(&err, Some("ws delete"));
        let json = serde_json::to_string(&resp).expect("serialize");
        let deserialized: Response<()> =
            serde_json::from_str(&json).expect("deserialize");
        assert!(!deserialized.success);
        let error = deserialized.error.expect("error");
        assert_eq!(error.code, "WORKSPACE_LOCKED");
    }

    #[test]
    fn serde_skips_none_fields() {
        let resp: Response<()> = Response::success((), None);
        let json_val = serde_json::to_value(&resp).expect("serialize");
        let obj = json_val.as_object().expect("object");
        assert!(!obj.contains_key("error"));
        assert!(!obj.contains_key("command"));
        assert!(!obj.contains_key("executionTimeMs"));
        assert!(!obj.contains_key("requestId"));
    }

    #[test]
    fn serde_camel_case_metadata() {
        let resp: Response<()> = Response::success((), Some("test cmd"))
            .with_duration(50)
            .with_request_id("req-1");
        let json_val = serde_json::to_value(&resp).expect("serialize");
        let meta = &json_val["metadata"];
        assert!(meta.get("executionTimeMs").is_some());
        assert!(meta.get("requestId").is_some());
        assert!(meta.get("version").is_some());
        assert!(meta.get("timestamp").is_some());
    }

    // ── FixRisk ─────────────────────────────────────────────────────────

    #[test]
    fn fix_risk_serde_roundtrip() {
        for risk in [FixRisk::Safe, FixRisk::Moderate, FixRisk::Dangerous] {
            let json = serde_json::to_string(&risk).expect("serialize");
            let deserialized: FixRisk =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(risk, deserialized);
        }
    }

    #[test]
    fn fix_risk_serializes_lowercase() {
        let json = serde_json::to_value(&FixRisk::Moderate).expect("serialize");
        assert_eq!(json, "moderate");
    }

    // ── Numeric code range validation ───────────────────────────────────

    #[test]
    fn error_numeric_codes_in_valid_range() {
        let errors = vec![
            crate::error::Error::workspace_not_found("x"),
            crate::error::Error::session("s"),
            crate::error::Error::internal("e"),
            crate::error::Error::vcs_conflict("r", "m"),
            crate::error::Error::queue_empty(),
        ];
        for err in &errors {
            let resp: Response<()> = Response::from_error(err, None);
            let error = resp.error.as_ref().expect("error");
            assert!(
                (1000..=9999).contains(&error.numeric_code),
                "Numeric code {} out of range for {:?}",
                error.numeric_code,
                err
            );
        }
    }

    // ── Category consistency ────────────────────────────────────────────

    #[test]
    fn error_category_matches_code_range() {
        let cases: Vec<(crate::error::Error, &str, std::ops::RangeInclusive<u16>)> = vec![
            (crate::error::Error::workspace_not_found("x"), "workspace", 1000..=1999),
            (crate::error::Error::session("s"), "session", 2000..=2999),
            (crate::error::Error::internal("e"), "internal", 9000..=9999),
            (crate::error::Error::vcs_not_initialized(), "vcs", 5000..=5999),
            (crate::error::Error::queue_empty(), "queue", 4000..=4999),
        ];
        for (err, expected_cat, expected_range) in cases {
            let resp: Response<()> = Response::from_error(&err, None);
            let error = resp.error.as_ref().expect("error");
            assert_eq!(error.category, expected_cat);
            assert!(
                expected_range.contains(&error.numeric_code),
                "Numeric code {} outside range for {expected_cat}",
                error.numeric_code
            );
        }
    }

    // ── Clone / Debug ───────────────────────────────────────────────────

    #[test]
    fn response_is_clone() {
        let resp = Response::success(42_i32, Some("cmd"));
        let cloned = resp.clone();
        assert_eq!(cloned.data, resp.data);
    }

    #[test]
    fn response_debug_contains_type_name() {
        let resp: Response<()> = Response::success((), None);
        let debug = format!("{resp:?}");
        assert!(debug.contains("Response"));
    }

    #[test]
    fn response_error_debug() {
        let err = ResponseError {
            code: "TEST".to_string(),
            numeric_code: 1001,
            category: "workspace".to_string(),
            message: "test error".to_string(),
            details: None,
            fix: None,
        };
        let debug = format!("{err:?}");
        assert!(debug.contains("TEST"));
    }
}
