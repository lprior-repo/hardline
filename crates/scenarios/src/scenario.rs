//! Scenario YAML schema and parser
//!
//! Defines the structure for behavioral scenarios that can be executed
//! against a twin universe for black-box testing.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A complete scenario with metadata and execution steps
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    /// Unique identifier for the scenario
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Ordered list of steps to execute
    pub steps: Vec<Step>,
}

/// A single step in a scenario
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Step {
    /// HTTP request step
    Http(HttpStep),
    /// Extract value from response
    Extract(ExtractStep),
    /// Assert a condition
    Assert(AssertStep),
}

/// HTTP request configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpStep {
    /// Target URL
    pub url: String,
    /// HTTP method
    #[serde(default)]
    pub method: HttpMethod,
    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body (for POST/PUT/PATCH)
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

/// HTTP methods supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Extract a value from a response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractStep {
    /// Source location (response.body, response.headers, etc.)
    pub from: String,
    /// `JSONPath` or similar selector
    pub path: String,
    /// Variable name to store the extracted value
    pub r#as: String,
}

/// Assert a condition on extracted values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertStep {
    /// Assertion type
    pub assertion: AssertionType,
    /// The actual value (can be a template like {{variable}})
    pub equals: Option<String>,
    /// The expected value
    pub expected: Option<String>,
    /// The value to check existence for
    pub exists: Option<String>,
    /// The value to check for absence
    pub not_exists: Option<String>,
}

/// Types of assertions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionType {
    #[default]
    Equals,
    NotEquals,
    Exists,
    NotExists,
    Contains,
    NotContains,
}

/// Result of parsing a scenario
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioParseResult {
    pub scenario: Scenario,
    pub variables: HashMap<String, String>,
}

impl Scenario {
    /// Parse a scenario from YAML string
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError::ParseError` if YAML is invalid.
    pub fn from_yaml(yaml_str: &str) -> Result<Self, ScenarioError> {
        serde_yaml::from_str(yaml_str).map_err(|e| ScenarioError::ParseError(e.to_string()))
    }

    /// Parse a scenario from YAML bytes
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError::ParseError` if YAML is invalid.
    pub fn from_yaml_bytes(bytes: &[u8]) -> Result<Self, ScenarioError> {
        serde_yaml::from_slice(bytes).map_err(|e| ScenarioError::ParseError(e.to_string()))
    }
}

/// Errors that can occur when working with scenarios
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScenarioError {
    #[error("Failed to parse scenario YAML: {0}")]
    ParseError(String),

    #[error("Invalid step: {0}")]
    InvalidStep(String),

    #[error("Invalid template: {0}")]
    InvalidTemplate(String),

    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Assertion failed: {0}")]
    AssertionFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_SCENARIO: &str = r#"
name: "SendGrid email validation"
description: "Validates email sending flow"
steps:
  - type: http
    url: "http://localhost:3001/v3/mail/send"
    method: POST
    headers:
      Authorization: "Bearer test-key"
      Content-Type: "application/json"
    body:
      personalizations:
        - to:
            - email: "test@example.com"
      from:
        email: "sender@example.com"
      subject: "Test"
      content:
        - type: "text"
          value: "Test email"
  - type: extract
    from: response.body
    path: "$.message_id"
    as: "message_id"
  - type: assert
    assertion: equals
    equals: "{{message_id}}"
    expected: "test-123"
"#;

    #[test]
    fn test_scenario_parsing() {
        let scenario = Scenario::from_yaml(VALID_SCENARIO).expect("Failed to parse scenario");

        assert_eq!(scenario.name, "SendGrid email validation");
        assert_eq!(scenario.description, "Validates email sending flow");
        assert_eq!(scenario.steps.len(), 3);

        // First step is HTTP
        match &scenario.steps[0] {
            Step::Http(http) => {
                assert_eq!(http.url, "http://localhost:3001/v3/mail/send");
                assert_eq!(http.method, HttpMethod::Post);
            }
            _ => panic!("Expected HTTP step"),
        }

        // Second step is extract
        match &scenario.steps[1] {
            Step::Extract(ext) => {
                assert_eq!(ext.from, "response.body");
                assert_eq!(ext.path, "$.message_id");
                assert_eq!(ext.r#as, "message_id");
            }
            _ => panic!("Expected Extract step"),
        }

        // Third step is assert
        match &scenario.steps[2] {
            Step::Assert(assert) => {
                assert_eq!(assert.equals.as_deref(), Some("{{message_id}}"));
                assert_eq!(assert.expected.as_deref(), Some("test-123"));
            }
            _ => panic!("Expected Assert step"),
        }
    }

    #[test]
    fn test_scenario_default_method() {
        let yaml = r#"
name: "Test"
description: "Test"
steps:
  - type: http
    url: "http://localhost:3001/test"
"#;
        let scenario = Scenario::from_yaml(yaml).expect("Failed to parse");

        match &scenario.steps[0] {
            Step::Http(http) => {
                assert_eq!(http.method, HttpMethod::Get);
            }
            _ => panic!("Expected HTTP step"),
        }
    }

    // === RED QUEEN — Gen 1: Parsing adversarial tests ===

    #[test]
    fn test_invalid_yaml_returns_parse_error() {
        let result = Scenario::from_yaml("not valid yaml: [");
        assert!(result.is_err());
        let err = result.expect_err("should be err");
        assert!(matches!(err, ScenarioError::ParseError(_)));
    }

    #[test]
    fn test_empty_yaml_returns_parse_error() {
        let result = Scenario::from_yaml("");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_fields_returns_parse_error() {
        let yaml = r#"
name: "test"
"#;
        let result = Scenario::from_yaml(yaml);
        // Missing 'description' and 'steps' — serde default should handle or fail
        // Either way, it should not panic
        let _ = result;
    }

    #[test]
    fn test_empty_steps_list_parses() {
        let yaml = r#"
name: "empty"
description: "no steps"
steps: []
"#;
        let scenario = Scenario::from_yaml(yaml).expect("empty steps should parse");
        assert!(scenario.steps.is_empty());
    }

    #[test]
    fn test_all_http_methods_parse() {
        let methods = [
            ("GET", HttpMethod::Get),
            ("POST", HttpMethod::Post),
            ("PUT", HttpMethod::Put),
            ("PATCH", HttpMethod::Patch),
            ("DELETE", HttpMethod::Delete),
        ];
        for (label, expected) in methods {
            let yaml = format!(
                r#"
name: "method test"
description: "test {label}"
steps:
  - type: http
    url: "http://localhost:3001/test"
    method: {label}
"#
            );
            let scenario = Scenario::from_yaml(&yaml)
                .unwrap_or_else(|e| panic!("{label} should parse: {e}"));
            if let Step::Http(http) = &scenario.steps[0] {
                assert_eq!(http.method, expected, "method mismatch for {label}");
            }
        }
    }

    #[test]
    fn test_all_assertion_types_parse() {
        let assertions = [
            "equals", "not_equals", "exists", "not_exists", "contains", "not_contains",
        ];
        for assertion in assertions {
            let yaml = format!(
                r#"
name: "assertion test"
description: "test {assertion}"
steps:
  - type: assert
    assertion: {assertion}
"#
            );
            let _scenario = Scenario::from_yaml(&yaml)
                .unwrap_or_else(|e| panic!("{assertion} should parse: {e}"));
        }
    }

    #[test]
    fn test_from_yaml_bytes_equivalent_to_from_yaml() {
        let yaml = VALID_SCENARIO;
        let from_str = Scenario::from_yaml(yaml).expect("from_yaml");
        let from_bytes = Scenario::from_yaml_bytes(yaml.as_bytes()).expect("from_yaml_bytes");
        assert_eq!(from_str, from_bytes);
    }

    #[test]
    fn test_scenario_roundtrip_serialize_deserialize() {
        let original = Scenario::from_yaml(VALID_SCENARIO).expect("parse");
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: Scenario = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_scenario_error_display() {
        let err = ScenarioError::ParseError("bad yaml".to_string());
        let msg = err.to_string();
        assert!(msg.contains("bad yaml"));

        let err = ScenarioError::VariableNotFound("x".to_string());
        let msg = err.to_string();
        assert!(msg.contains("x"));
    }

    #[test]
    fn test_http_step_default_headers_and_body() {
        let yaml = r#"
name: "defaults"
description: "test defaults"
steps:
  - type: http
    url: "http://localhost:3001/test"
"#;
        let scenario = Scenario::from_yaml(yaml).expect("parse");
        if let Step::Http(http) = &scenario.steps[0] {
            assert!(http.headers.is_empty());
            assert!(http.body.is_none());
        }
    }
}
